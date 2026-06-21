use anchor_lang::prelude::*;

#[error_code]
pub enum AmmError {
    #[msg("Pool is locked")]
    PoolLocked,
    #[msg("Fee must be less than 10000 bps")]
    InvalidFee,
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Slippage limit exceeded")]
    SlippageExceeded,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Pool reserves are empty")]
    EmptyReserves,
    #[msg("Only the pool authority may perform this action")]
    Unauthorized,
    #[msg("Mint A and Mint B must be different")]
    IdenticalMints,
    #[msg("No previous instruction in transaction")]
    MissingPreviousInstruction,
    #[msg("Previous instruction is not from this program")]
    InvalidProgram,
    #[msg("Previous instruction is not burn_lp")]
    InvalidInstruction,
    #[msg("Previous instruction data does not match")]
    InvalidInstructionData,
    #[msg("Previous instruction accounts do not match")]
    InvalidInstructionAccounts,
    #[msg("payout must immediately follow burn_lp in the same transaction")]
    InvalidInstructionOrder,
}
