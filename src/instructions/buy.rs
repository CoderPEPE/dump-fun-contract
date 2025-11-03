use {
    crate::{constants::*, errors::*, events::*, states::*, utils::*},
    anchor_lang::{prelude::*, system_program},
    anchor_spl::{
        associated_token::AssociatedToken,
        token::{self, Mint, Token, TokenAccount, Transfer},
    },
    fixed::types::I128F0,
};

pub fn buy_exact_tokens(ctx: Context<BuyTokens>, token_out: u64, max_sol_in: u64) -> Result<()> {
    let liquidity_pool = &mut ctx.accounts.liquidity_pool;
    let clock = Clock::get()?;

    require!(!liquidity_pool.closed, DumpError::PoolClosed);

    // Reentrancy guard
    require!(!liquidity_pool.in_use, DumpError::Reentrancy);
    liquidity_pool.in_use = true;

    require!(
        token_out < liquidity_pool.real_token_reserve,
        DumpError::ExcessiveInputAmount
    );

    let denominator = liquidity_pool
        .real_token_reserve
        .checked_sub(token_out)
        .ok_or(DumpError::MathOverflow)?;

    let amount_in = mul_div_round_up(
        token_out,
        liquidity_pool.virtual_sol_reserve,
        denominator,
    );

    require!(SOL_FEE < 10_000, DumpError::InvalidFeeRate);
    let amount_in_with_fee = mul_div_round_up(amount_in, 10_000, 10_000 - SOL_FEE as u64);

    require!(amount_in_with_fee <= max_sol_in, DumpError::ExcessiveInputAmount);
    require!(ctx.accounts.user.lamports() >= amount_in_with_fee, DumpError::InsufficientBalance);

    let elapsed = clock.unix_timestamp - liquidity_pool.create_time;
    if let Some(limit_bps) = liquidity_pool
        .ramping_limits
        .iter()
        .find(|limit| elapsed >= limit.start_sec && elapsed < limit.end_sec)
        .map(|l| l.limit_bps)
    {
        let max_buy = (liquidity_pool.real_token_reserve * limit_bps as u64) / 10_000;
        require!(
            ctx.accounts.user_token_account.amount + token_out <= max_buy,
            DumpError::RampLimitExceeded
        );
    }

    liquidity_pool.virtual_sol_reserve = liquidity_pool
        .virtual_sol_reserve
        .checked_add(amount_in)
        .ok_or(DumpError::MathOverflow)?;

    liquidity_pool.real_token_reserve = liquidity_pool
        .real_token_reserve
        .checked_sub(token_out)
        .ok_or(DumpError::MathOverflow)?;

    liquidity_pool.total_sol_volume = liquidity_pool
        .total_sol_volume
        .checked_add(amount_in)
        .ok_or(DumpError::MathOverflow)?;

    system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.liquidity_pool_authority.to_account_info(),
            },
        ),
        amount_in,
    )?;

    system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.platform_fee_wallet.to_account_info(),
            },
        ),
        amount_in_with_fee.checked_sub(amount_in).ok_or(DumpError::MathOverflow)?,
    )?;

    let signer_seeds = &[
        &liquidity_pool.key().to_bytes(),
        &ctx.accounts.mint_account.key().to_bytes(),
        AUTHORITY_SEED.as_bytes(),
        &[ctx.bumps.liquidity_pool_authority],
    ];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.liquidity_pool_token_account.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.liquidity_pool_authority.to_account_info(),
            },
            &[signer_seeds],
        ),
        token_out,
    )?;

    emit!(BuyTokenEvent {
        user: ctx.accounts.user.key(),
        mint: ctx.accounts.mint_account.key(),
        sol_in: amount_in_with_fee,
        token_out,
        buy_time: clock.unix_timestamp,
    });

    liquidity_pool.in_use = false;

    Ok(())
}

pub fn buy_tokens_with_exact_sol(ctx: Context<BuyTokens>, sol_in: u64, min_token_out: u64) -> Result<()> {
    let liquidity_pool = &mut ctx.accounts.liquidity_pool;
    let clock = Clock::get()?;

    require!(!liquidity_pool.closed, DumpError::PoolClosed);

    // Reentrancy guard
    require!(!liquidity_pool.in_use, DumpError::Reentrancy);
    liquidity_pool.in_use = true;

    require!(ctx.accounts.user.lamports() >= sol_in, DumpError::InsufficientBalance);
    require!(SOL_FEE < 10_000, DumpError::InvalidFeeRate);

    let sol_fee = mul_div_round_up(sol_in, SOL_FEE as u64, 10_000);
    let sol_after_fee = sol_in.checked_sub(sol_fee).ok_or(DumpError::MathOverflow)?;

    let sol_after_fee_fp = I128F0::from_num(sol_after_fee);
    let virtual_sol_reserve = I128F0::from_num(liquidity_pool.virtual_sol_reserve);
    let real_token_reserve = I128F0::from_num(liquidity_pool.real_token_reserve);

    let token_out = sol_after_fee_fp
        .checked_mul(real_token_reserve)
        .and_then(|n| n.checked_div(virtual_sol_reserve.checked_add(sol_after_fee_fp)?))
        .ok_or(DumpError::MathOverflow)?
        .to_num::<u64>();

    require!(token_out >= min_token_out, DumpError::InsufficientOutput);

    let elapsed = clock.unix_timestamp - liquidity_pool.create_time;
    if let Some(limit_bps) = liquidity_pool
        .ramping_limits
        .iter()
        .find(|limit| elapsed >= limit.start_sec && elapsed < limit.end_sec)
        .map(|l| l.limit_bps)
    {
        let max_buy = (liquidity_pool.real_token_reserve * limit_bps as u64) / 10_000;
        require!(
            ctx.accounts.user_token_account.amount + token_out <= max_buy,
            DumpError::RampLimitExceeded
        );
    }

    liquidity_pool.virtual_sol_reserve = liquidity_pool
        .virtual_sol_reserve
        .checked_add(sol_after_fee)
        .ok_or(DumpError::MathOverflow)?;

    liquidity_pool.real_token_reserve = liquidity_pool
        .real_token_reserve
        .checked_sub(token_out)
        .ok_or(DumpError::MathOverflow)?;

    liquidity_pool.total_sol_volume = liquidity_pool
        .total_sol_volume
        .checked_add(sol_after_fee)
        .ok_or(DumpError::MathOverflow)?;
    
    system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.liquidity_pool_authority.to_account_info(),
            },
        ),
        sol_after_fee,
    )?;

    system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.platform_fee_wallet.to_account_info(),
            },
        ),
        sol_fee,
    )?;

    let signer_seeds = &[
        &liquidity_pool.key().to_bytes(),
        &ctx.accounts.mint_account.key().to_bytes(),
        AUTHORITY_SEED.as_bytes(),
        &[ctx.bumps.liquidity_pool_authority],
    ];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.liquidity_pool_token_account.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.liquidity_pool_authority.to_account_info(),
            },
            &[signer_seeds],
        ),
        token_out,
    )?;

    emit!(BuyTokenEvent {
        user: ctx.accounts.user.key(),
        mint: ctx.accounts.mint_account.key(),
        sol_in,
        token_out,
        buy_time: clock.unix_timestamp,
    });

    liquidity_pool.in_use = false;

    Ok(())
}


#[derive(Accounts)]
pub struct BuyTokens<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: This is the platform fee wallet
    #[account(
        mut,
        constraint = platform_fee_wallet.key() == liquidity_pool.platform_fee_wallet
    )]
    pub platform_fee_wallet: AccountInfo<'info>,

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
