use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
pub struct LiquidityPool {
    /// Creator wallet of token
    pub creator_wallet: Pubkey,
    /// Platform fee wallet
    pub platform_fee_wallet: Pubkey,
    /// Company tax wallet
    pub company_tax_wallet: Pubkey,
    /// Mint account of token A
    pub mint_account: Pubkey,
    /// Sell lock period (duration in seconds from pool creation)
    pub sell_lock_period: i64,
    /// Virtual sol reserve
    pub virtual_sol_reserve: u64,
    /// Total SOL volume
    pub total_sol_volume: u64,
    /// Virtual token reserve
    pub real_token_reserve: u64,
    /// Pool creation time
    pub create_time: i64,
    /// Dynamic ramping limit entries
    pub ramping_limits: Vec<RampingLimit>,
    /// Reentrancy lock
    pub in_use: bool,
    /// Pool closed flag
    pub closed: bool,
}

impl LiquidityPool {
    // Account discriminator: 8 bytes
    // Fixed fields: 4 Pubkeys (32*4) + 2 i64 (8*2) + 3 u64 (8*3) = 128 + 16 + 24 = 168 bytes
    // Vec ramping_limits: 4 bytes (length) + max 4 items * 18 bytes each = 4 + 72 = 76 bytes
    // Boolean fields: 2 bytes
    // Total: 8 + 168 + 76 + 2 = 254 bytes
    pub const LEN: usize = 254;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct RampingLimit {
    pub start_sec: i64,
    pub end_sec: i64,
    pub limit_bps: u16,
}

impl RampingLimit {
    pub const LEN: usize = 8 + 8 + 2;
}
