use {
    crate::{constants::*, errors::*, events::*, states::*},
    anchor_lang::{prelude::*, system_program},
    anchor_spl::{
        associated_token::AssociatedToken,
        metadata::{
            create_metadata_accounts_v3, mpl_token_metadata::types::DataV2,
            update_metadata_accounts_v2, CreateMetadataAccountsV3, Metadata,
            UpdateMetadataAccountsV2,
        },
        token::{
            mint_to, set_authority, spl_token::instruction::AuthorityType, Mint, MintTo,
            SetAuthority, Token, TokenAccount,
        },
    },
};

pub fn mint_token(
    ctx: Context<MintToken>,
    token_name: String,
    token_symbol: String,
    token_uri: String,
    sell_lock_period: i64,
    virtual_sol_reserve: u64,
    ramping_limits: Vec<RampingLimit>,
) -> Result<()> {
    require!(
        sell_lock_period >= 300 && sell_lock_period <= MAX_SELL_LOCK_PERIOD,
        DumpError::InvalidSellLockPeriod
    );

    require!(
        virtual_sol_reserve >= MIN_VIRTUAL_SOL_RESERVE
            && virtual_sol_reserve <= MAX_VIRTUAL_SOL_RESERVE,
        DumpError::InvalidVirtualSolReserve
    );

    // Validate ramping limits
    validate_ramping_limits(&ramping_limits, sell_lock_period)?;

    create_metadata_accounts_v3(
        CpiContext::new(
            ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                metadata: ctx.accounts.metadata_account.to_account_info(),
                mint: ctx.accounts.mint_account.to_account_info(),
                mint_authority: ctx.accounts.payer.to_account_info(),
                update_authority: ctx.accounts.payer.to_account_info(),
                payer: ctx.accounts.payer.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
        ),
        DataV2 {
            name: token_name,
            symbol: token_symbol,
            uri: token_uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        },
        false, // Is mutable
        true,  // Update authority is signer
        None,  // Collection details
    )?;

    // Initialize the mint account with the total supply
    mint_to(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.mint_account.to_account_info(),
                to: ctx.accounts.liquidity_pool_token_account.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            },
        ),
        TOTAL_SUPPLY,
    )?;

    // Renounce the update authority by setting it to Some(Pubkey::default())
    update_metadata_accounts_v2(
        CpiContext::new(
            ctx.accounts.token_metadata_program.to_account_info(),
            UpdateMetadataAccountsV2 {
                metadata: ctx.accounts.metadata_account.to_account_info(),
                update_authority: ctx.accounts.payer.to_account_info(),
            },
        ),
        Some(Pubkey::default()), // New update authority (renounced via default pubkey)
        None,                    // Data (None to leave unchanged)
        None,                    // Primary sale happened (None to leave unchanged)
        None,                    // Is mutable (None to leave unchanged)
    )?;

    set_authority(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            SetAuthority {
                current_authority: ctx.accounts.payer.to_account_info(),
                account_or_mint: ctx.accounts.mint_account.to_account_info(),
            },
        ),
        AuthorityType::MintTokens,
        None,
    )?;

    // --------------- Create liquidity pool ---------------
    let liquidity_pool = &mut ctx.accounts.liquidity_pool;
    liquidity_pool.creator_wallet = ctx.accounts.payer.key();
    liquidity_pool.platform_fee_wallet = PLATFORM_FEE_WALLET;
    liquidity_pool.company_tax_wallet = COMPANY_TAX_WALLET;
    liquidity_pool.mint_account = ctx.accounts.mint_account.key();
    liquidity_pool.sell_lock_period = sell_lock_period;
    liquidity_pool.virtual_sol_reserve = virtual_sol_reserve;
    liquidity_pool.total_sol_volume = 0;
    liquidity_pool.real_token_reserve = TOTAL_SUPPLY;
    liquidity_pool.create_time = Clock::get()?.unix_timestamp;
    liquidity_pool.ramping_limits = ramping_limits;
    liquidity_pool.closed = false;

    // Initialize the liquidity pool authority PDA as a system account
    let signer_seeds = &[
        &liquidity_pool.key().to_bytes(),
        &ctx.accounts.mint_account.key().to_bytes(),
        AUTHORITY_SEED.as_bytes(),
        &[ctx.bumps.liquidity_pool_authority],
    ];

    // Fund the authority PDA with rent-exempt lamports
    let rent_exempt_lamports = Rent::get()?.minimum_balance(0);

    system_program::create_account(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            system_program::CreateAccount {
                from: ctx.accounts.payer.to_account_info(),
                to: ctx.accounts.liquidity_pool_authority.to_account_info(),
            },
            &[signer_seeds],
        ),
        rent_exempt_lamports,
        0, // space
        &ctx.accounts.system_program.key(),
    )?;

    emit!(TokenCreatedEvent {
        creator: ctx.accounts.payer.key(),
        mint: ctx.accounts.mint_account.key(),
        create_time: liquidity_pool.create_time,
        sell_lock_period,
    });

    Ok(())
}

fn validate_ramping_limits(
    ramping_limits: &Vec<RampingLimit>,
    sell_lock_period: i64,
) -> Result<()> {
    // Rule 1: Can be empty (no limits)
    if ramping_limits.is_empty() {
        return Ok(());
    }

    // Rule 2: Maximum 4 items
    require!(ramping_limits.len() <= 4, DumpError::TooManyRampingLimits);

    // Rule 3: Check individual limit validity
    for limit in ramping_limits {
        require!(limit.start_sec >= 0, DumpError::InvalidRampingLimitTime);
        require!(
            limit.end_sec > limit.start_sec,
            DumpError::InvalidRampingLimitTime
        );
        require!(
            limit.limit_bps > 0 && limit.limit_bps <= 10000,
            DumpError::InvalidRampingLimitBps
        );
    }

    // Rule 4: Check for overlapping periods
    for i in 0..ramping_limits.len() {
        for j in (i + 1)..ramping_limits.len() {
            let limit1 = &ramping_limits[i];
            let limit2 = &ramping_limits[j];

            // Check if periods overlap
            require!(
                limit1.end_sec <= limit2.start_sec || limit2.end_sec <= limit1.start_sec,
                DumpError::OverlappingRampingLimits
            );
        }
    }

    // Rule 5: Total limit time should be <= 1/5 of sell unlock time
    let max_allowed_limit_time = sell_lock_period / 5;
    let mut total_limit_time = 0i64;

    for limit in ramping_limits {
        total_limit_time += limit.end_sec - limit.start_sec;
    }

    require!(
        total_limit_time <= max_allowed_limit_time,
        DumpError::ExcessiveRampingLimitTime
    );

    // Rule 6: All limits should end before sell unlock time
    for limit in ramping_limits {
        require!(
            limit.end_sec <= sell_lock_period,
            DumpError::RampingLimitExceedsUnlockTime
        );
    }

    Ok(())
}

#[derive(Accounts)]
pub struct MintToken<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Validate address by deriving pda
    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), mint_account.key().as_ref()],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    pub metadata_account: AccountInfo<'info>,
    // Create new mint account
    #[account(
        init,
        payer = payer,
        mint::decimals = TOKEN_DECIMAL,
        mint::authority = payer.key(),
    )]
    pub mint_account: Account<'info, Mint>,


    #[account(
        init,
        payer = payer,
        space = LiquidityPool::LEN,
        seeds = [
            mint_account.key().as_ref(),
            LIQUIDITY_SEED.as_bytes(),
        ],
        bump,
    )]
    pub liquidity_pool: Box<Account<'info, LiquidityPool>>,

    /// CHECK: Read only authority for the liquidity pool
    #[account(
        mut,
        seeds = [
            liquidity_pool.key().as_ref(),
            mint_account.key().as_ref(),
            AUTHORITY_SEED.as_bytes(),
        ],
        bump,
    )]
    pub liquidity_pool_authority: AccountInfo<'info>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint_account,
        associated_token::authority = liquidity_pool_authority,
        associated_token::token_program = token_program
    )]
    pub liquidity_pool_token_account: Box<Account<'info, TokenAccount>>,

    pub token_metadata_program: Program<'info, Metadata>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}
