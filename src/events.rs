use anchor_lang::prelude::*;

#[event]
pub struct TokenCreatedEvent {
    /// The public key of the creator of the token
    pub creator: Pubkey,
    /// The public key of the mint
    pub mint: Pubkey,
    /// The timestamp of when the token was created
    pub create_time: i64,
    /// The sell lock period
    pub sell_lock_period: i64,
}

#[event]
pub struct BuyTokenEvent {
    /// The public key of the user
    pub user: Pubkey,
    /// The public key of the token
    pub mint: Pubkey,
    /// The amount of sol in
    pub sol_in: u64,
    /// The amount of token out
    pub token_out: u64,
    /// The timestamp of when the token was bought
    pub buy_time: i64,
}

#[event]
pub struct SellTokenEvent {
    /// The public key of the user
    pub user: Pubkey,
    /// The public key of the token
    pub mint: Pubkey,
    /// The amount of token in
    pub token_in: u64,
    /// The amount of sol out
    pub sol_out: u64,
    /// The timestamp of when the token was sold
    pub sell_time: i64,
}

#[event]
pub struct DrainPoolEvent {
    /// The public key of the liquidity pool
    pub pool: Pubkey,
    /// The public key of the mint
    pub mint: Pubkey,
    /// The public key of the creator wallet
    pub creator_wallet: Pubkey,
    /// The public key of the company wallet
    pub company_wallet: Pubkey,
    /// The amount sent to creator (rounded down)
    pub creator_amount: u64,
    /// The amount sent to company (remainder)
    pub company_amount: u64,
    /// The total amount drained
    pub total_drained: u64,
    /// The timestamp of when the pool was drained
    pub drain_time: i64,
}
