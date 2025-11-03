use anchor_lang::prelude::error_code;
#[error_code]
pub enum DumpError {
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Insufficient output amount")]
    InsufficientOutput,
    #[msg("Invalid sell lock period")]
    InvalidSellLockPeriod,
    #[msg("Ramp limit exceeded")]
    RampLimitExceeded,
    #[msg("Excessive input amount")]
    ExcessiveInputAmount,
    #[msg("Sell not unlocked")]
    SellNotUnlocked,
    #[msg("Too many ramping limits (max 4 allowed)")]
    TooManyRampingLimits,
    #[msg("Invalid ramping limit time (start_sec must be >= 0 and end_sec > start_sec)")]
    InvalidRampingLimitTime,
    #[msg("Invalid ramping limit bps (must be > 0 and <= 10000)")]
    InvalidRampingLimitBps,
    #[msg("Overlapping ramping limit periods")]
    OverlappingRampingLimits,
    #[msg("Excessive ramping limit time (total time must be <= 1/5 of sell unlock time)")]
    ExcessiveRampingLimitTime,
    #[msg("Ramping limit exceeds unlock time")]
    RampingLimitExceedsUnlockTime,
    #[msg("Invalid virtual SOL reserve")]
    InvalidVirtualSolReserve,
    #[msg("Invalid fee rate")]
    InvalidFeeRate,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Invalid token account")]
    InvalidTokenAccount,
    #[msg("Reentrancy")]
    Reentrancy,
    #[msg("Liquidity pool closed")]
    PoolClosed,
}
