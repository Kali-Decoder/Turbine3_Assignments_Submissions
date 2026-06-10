use anchor_lang::prelude::*;

#[error_code]
pub enum MarketplaceError {
    #[msg("Marketplace name must be between 1 and 32 bytes")]
    InvalidNameLength,
    #[msg("Fee in basis points cannot exceed 10000")]
    InvalidFee,
    #[msg("This listing is denominated in SOL; use `purchase`")]
    NotTokenListing,
    #[msg("This listing is denominated in an SPL token; use `buy_with_token`")]
    NotSolListing,
    #[msg("The supplied payment mint does not match the listing")]
    PaymentMintMismatch,
    #[msg("Offer amount must be greater than zero")]
    ZeroOffer,
    #[msg("Arithmetic overflow")]
    Overflow,
}
