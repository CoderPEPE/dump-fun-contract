use {
    crate::{constants::*, errors::*, events::*, states::*, utils::*},
    anchor_lang::{prelude::*, system_program},
    anchor_spl::{
        associated_token::AssociatedToken,
        token::{self, Mint, Token, TokenAccount, Transfer},
    },
    fixed::types::I128F0,
};

pub fn sell_exact_tokens(ctx: Context<SellTokens>, token_in: u64, min_sol_out: u64) -> Result<()> {
    let pool = &mut ctx.accounts.liquidity_pool;
    let clock = Clock::get()?;

    require!(!pool.closed, DumpError::PoolClosed);

    // Reentrancy guard
    require!(!pool.in_use, DumpError::Reentrancy);
    pool.in_use = true;

    let elapsed = clock.unix_timestamp - pool.create_time;
    require!(elapsed >= pool.sell_lock_period, DumpError::SellNotUnlocked);

    require!(
        SOL_FEE + SELL_FEE_TO_COMPANY + SELL_FEE_TO_CREATOR < 10_000,
        DumpError::InvalidFeeRate
    );

    require!(
        ctx.accounts.user_token_account.amount >= token_in,
        DumpError::InsufficientBalance
    );


    let token_in_fp = I128F0::from_num(token_in);
    let virtual_sol_reserve = I128F0::from_num(pool.virtual_sol_reserve);
    let real_token_reserve = I128F0::from_num(pool.real_token_reserve);

    let denominator = real_token_reserve
        .checked_add(token_in_fp)
        .ok_or(DumpError::MathOverflow)?;

    let amount_out = token_in_fp
        .checked_mul(virtual_sol_reserve)
        .and_then(|n| n.checked_div(denominator))
        .ok_or(DumpError::MathOverflow)?
        .to_num::<u64>();

    let sol_fee = mul_div_round_up(amount_out, SOL_FEE as u64, 10_000);
    let fee_creator = mul_div_round_up(amount_out, SELL_FEE_TO_CREATOR as u64, 10_000);
    let fee_company = mul_div_round_up(amount_out, SELL_FEE_TO_COMPANY as u64, 10_000);

    let amount_out_with_fee = amount_out
        .checked_sub(sol_fee)
        .and_then(|v| v.checked_sub(fee_creator))
        .and_then(|v| v.checked_sub(fee_company))
        .ok_or(DumpError::MathOverflow)?;

    require!(amount_out_with_fee >= min_sol_out, DumpError::InsufficientOutput);

    let pool_sol_balance = ctx.accounts.liquidity_pool_authority.lamports();
    require!(amount_out <= pool_sol_balance, DumpError::InsufficientBalance);

    pool.virtual_sol_reserve = pool
        .virtual_sol_reserve
        .checked_sub(amount_out)
        .ok_or(DumpError::MathOverflow)?;

    pool.real_token_reserve = pool
        .real_token_reserve
        .checked_add(token_in)
        .ok_or(DumpError::MathOverflow)?;

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.liquidity_pool_token_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        token_in,
    )?;

    let signer_seeds: &[&[u8]] = &[
        &pool.key().to_bytes(),
        &ctx.accounts.mint_account.key().to_bytes(),
        AUTHORITY_SEED.as_bytes(),
        &[ctx.bumps.liquidity_pool_authority],
    ];
    let signer: &[&[&[u8]]] = &[signer_seeds];

    system_program::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.liquidity_pool_authority.to_account_info(),
                to: ctx.accounts.user.to_account_info(),
            },
            signer,
        ),
        amount_out_with_fee,
    )?;

    system_program::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.liquidity_pool_authority.to_account_info(),
                to: ctx.accounts.platform_fee_wallet.to_account_info(),
            },
            signer,
        ),
        sol_fee,
    )?;

    system_program::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.liquidity_pool_authority.to_account_info(),
                to: ctx.accounts.company_tax_wallet.to_account_info(),
            },
            signer,
        ),
        fee_company,
    )?;

    system_program::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.liquidity_pool_authority.to_account_info(),
                to: ctx.accounts.creator_wallet.to_account_info(),
            },
            signer,
        ),
        fee_creator,
    )?;

    emit!(SellTokenEvent {
        user: ctx.accounts.user.key(),
        mint: ctx.accounts.mint_account.key(),
        token_in,
        sol_out: amount_out_with_fee,
        sell_time: clock.unix_timestamp,
    });

    // Release reentrancy guard
    pool.in_use = false;

    Ok(())
}


#[derive(Accounts)]
pub struct SellTokens<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: This is the platform fee wallet
    #[account(
        mut,
        constraint = platform_fee_wallet.key() == liquidity_pool.platform_fee_wallet
    )]
    pub platform_fee_wallet: AccountInfo<'info>,

    /// CHECK: This is the company tax wallet
    #[account(
        mut,
        constraint = company_tax_wallet.key() == liquidity_pool.company_tax_wallet
    )]
    pub company_tax_wallet: AccountInfo<'info>,

    /// CHECK: This is the creator wallet
    #[account(
        mut,
        constraint = creator_wallet.key() == liquidity_pool.creator_wallet
    )]
    pub creator_wallet: AccountInfo<'info>,

    #[account(mut)]
    pub mint_account: Box<Account<'info, Mint>>,

    #[account(
        mut,
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
        mut,
        associated_token::mint = mint_account,
        associated_token::authority = liquidity_pool_authority,
        associated_token::token_program = token_program,
    )]
    pub liquidity_pool_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint_account,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_token_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}
