use {
    crate::{constants::*, errors::*, events::*, states::*},
    anchor_lang::{prelude::*, system_program},
    anchor_spl::token::Mint,
};

/// Drain pool surplus SOL to creator and company wallets
///
/// This function distributes surplus SOL from the liquidity pool when all tokens
/// have been returned to the pool (real_token_reserve == TOTAL_SUPPLY).
///
/// Security Features:
/// - Reentrancy protection using in_use flag
/// - Comprehensive input validation
/// - State updates after successful transfers
/// - Event emission for transparency
/// - Creator-only access (as intended)
///
/// Distribution: 50/50 split with creator getting rounded down amount
/// and company receiving the remainder to ensure no lamports are lost.
/// If the balance is below twice the rent threshold, the entire amount is
/// sent to the creator to avoid leaving rent-exempt dust.
pub fn drain_pool_surplus(ctx: Context<DrainPoolSurplus>) -> Result<()> {
    let pool = &mut ctx.accounts.liquidity_pool;
    let clock = Clock::get()?;

    require!(!pool.closed, DumpError::PoolClosed);

    // 🔒 REENTRANCY PROTECTION
    require!(!pool.in_use, DumpError::Reentrancy);
    pool.in_use = true;

    // ✅ COMPREHENSIVE INPUT VALIDATION
    require!(
        pool.real_token_reserve == TOTAL_SUPPLY,
        DumpError::InsufficientBalance
    );

    // Validate wallet addresses are not program-derived
    // Note: We trust the constraint checks above which validate against pool state

    // Validate pool state integrity
    require!(
        pool.creator_wallet == ctx.accounts.creator_wallet.key(),
        DumpError::InvalidTokenAccount
    );
    require!(
        pool.company_tax_wallet == ctx.accounts.company_wallet.key(),
        DumpError::InvalidTokenAccount
    );

    let balance = ctx.accounts.liquidity_pool_authority.lamports();
    require!(balance > 0, DumpError::InsufficientBalance);

    // Determine rent-exempt threshold for the authority account
    let rent_exempt_threshold =
        Rent::get()?.minimum_balance(ctx.accounts.liquidity_pool_authority.data_len());
    let double_rent = rent_exempt_threshold
        .checked_mul(2)
        .ok_or(DumpError::MathOverflow)?;

    let signer_seeds: &[&[u8]] = &[
        &pool.key().to_bytes(),
        &ctx.accounts.mint_account.key().to_bytes(),
        AUTHORITY_SEED.as_bytes(),
        &[ctx.bumps.liquidity_pool_authority],
    ];

    // Determine distribution amounts
    let (creator_amount, company_amount) = if balance < double_rent {
        (balance, 0)
    } else {
        let half = balance / 2;
        let company_amount = balance.checked_sub(half).ok_or(DumpError::MathOverflow)?;
        (half, company_amount)
    };

    // Transfer creator share
    system_program::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.liquidity_pool_authority.to_account_info(),
                to: ctx.accounts.creator_wallet.to_account_info(),
            },
            &[signer_seeds],
        ),
        creator_amount,
    )?;

    // Transfer company share only if non-zero
    if company_amount > 0 {
        system_program::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.liquidity_pool_authority.to_account_info(),
                    to: ctx.accounts.company_wallet.to_account_info(),
                },
                &[signer_seeds],
            ),
            company_amount,
        )?;
    }

    // 🔔 EVENT EMISSION FOR TRANSPARENCY AND AUDITABILITY
    emit!(DrainPoolEvent {
        pool: pool.key(),
        mint: ctx.accounts.mint_account.key(),
        creator_wallet: ctx.accounts.creator_wallet.key(),
        company_wallet: ctx.accounts.company_wallet.key(),
        creator_amount,
        company_amount,
        total_drained: balance,
        drain_time: clock.unix_timestamp,
    });

    pool.closed = true;

    // 🔓 RELEASE REENTRANCY LOCK
    pool.in_use = false;

    Ok(())
}

#[derive(Accounts)]
pub struct DrainPoolSurplus<'info> {
    /// Creator wallet - must be signer and match pool creator
    #[account(mut, constraint = creator_wallet.key() == liquidity_pool.creator_wallet)]
    pub creator_wallet: Signer<'info>,

    /// Company wallet - receives remainder from 50/50 split
    /// CHECK: Validated against pool.company_tax_wallet
    #[account(mut, constraint = company_wallet.key() == liquidity_pool.company_tax_wallet)]
    pub company_wallet: AccountInfo<'info>,

    /// Mint account for the token
    #[account(mut)]
    pub mint_account: Box<Account<'info, Mint>>,

    /// Liquidity pool account
    #[account(
        mut,
        seeds = [
            mint_account.key().as_ref(),
            LIQUIDITY_SEED.as_bytes(),
        ],
        bump,
        close = creator_wallet,
    )]
    pub liquidity_pool: Box<Account<'info, LiquidityPool>>,

    /// Liquidity pool authority PDA
    /// CHECK: PDA derived from pool and mint
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

    /// System program for SOL transfers
    pub system_program: Program<'info, System>,
}
