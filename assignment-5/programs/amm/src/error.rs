use anchor_lang::prelude::*;

#[error_code]
pub enum AmmError {
    #[msg("The pool is locked")]
    PoolLocked,
    #[msg("Fee basis points must be less than the fee denominator")]
    InvalidFee,
    #[msg("Provided amount is zero")]
    ZeroAmount,
    #[msg("Slippage tolerance exceeded")]
    SlippageExceeded,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Pool reserves are empty")]
    EmptyReserves,
    #[msg("Only the configured authority can call this instruction")]
    Unauthorized,
    #[msg("Mint A and Mint B must be different")]
    IdenticalMints,
}
