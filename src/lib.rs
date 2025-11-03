use anchor_lang::prelude::*;

mod constants;
mod errors;
mod events;
mod instructions;
mod states;
mod utils;

use instructions::*;

// Re-export RampingLimit for use by other programs
pub use states::RampingLimit;

declare_id!("4g98mzoFU5txWcZrgp2CUwGh3yMAtGgyofFsKrGhR8jP");

#[program]
pub mod dump_contract {
    use super::*;

    pub fn mint(
        ctx: Context<MintToken>,
        token_name: String,
        token_symbol: String,
        token_uri: String,
        sell_lock_period: i64,
        virtual_sol_reserve: u64,
        ramping_limits: Vec<RampingLimit>,
    ) -> Result<()> {
        mint_token(
            ctx,
            token_name,
            token_symbol,
            token_uri,
            sell_lock_period,
            virtual_sol_reserve,
            ramping_limits,
        )
    }

    pub fn buy_exact_tokens(
        ctx: Context<BuyTokens>,
        token_out: u64,
        max_sol_in: u64,
    ) -> Result<()> {
        instructions::buy_exact_tokens(ctx, token_out, max_sol_in)
    }

    pub fn buy_tokens_with_exact_sol(
        ctx: Context<BuyTokens>,
        sol_in: u64,
        min_token_out: u64,
    ) -> Result<()> {
        instructions::buy_tokens_with_exact_sol(ctx, sol_in, min_token_out)
    }

    pub fn sell_exact_tokens(
        ctx: Context<SellTokens>,
        token_in: u64,
        min_sol_out: u64,
    ) -> Result<()> {
        instructions::sell_exact_tokens(ctx, token_in, min_sol_out)
    }

    pub fn drain_pool_surplus(ctx: Context<DrainPoolSurplus>) -> Result<()> {
        instructions::drain_pool_surplus(ctx)
    }
}
