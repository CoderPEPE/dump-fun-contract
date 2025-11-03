use anchor_lang::{prelude::*, solana_program::native_token::LAMPORTS_PER_SOL};

#[constant]
pub const TOKEN_DECIMAL: u8 = 6;
pub const TOTAL_SUPPLY: u64 = 1000000000 * 10u64.pow(TOKEN_DECIMAL as u32);
pub const MIN_VIRTUAL_SOL_RESERVE: u64 = 5 * LAMPORTS_PER_SOL;
pub const MAX_VIRTUAL_SOL_RESERVE: u64 = 500 * LAMPORTS_PER_SOL;
/// Maximum duration tokens can be locked from selling (5 years)
pub const MAX_SELL_LOCK_PERIOD: i64 = 5 * 365 * 24 * 60 * 60; // seconds
pub const SOL_FEE: u16 = 100; // 1%
pub const SELL_FEE_TO_CREATOR: u16 = 250; // 2.5%
pub const SELL_FEE_TO_COMPANY: u16 = 250; // 2.5%

pub const LIQUIDITY_SEED: &str = "liquidity";
pub const AUTHORITY_SEED: &str = "authority";

#[constant]
pub const PLATFORM_FEE_WALLET: Pubkey = pubkey!("nCfLEJSfAHMdXWH7B4xWRqDHvaq7B3k5Ly9RZcPUxDm");
pub const COMPANY_TAX_WALLET: Pubkey = pubkey!("ENBRyYi5PMybdNhU6HcN7u2HsjN4vXaeB3y3S3Kzwyue");
