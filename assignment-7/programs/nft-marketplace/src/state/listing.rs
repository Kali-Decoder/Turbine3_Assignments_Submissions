use anchor_lang::prelude::*;

/// An active listing for a single NFT (a mint with supply 1 / 0 decimals).
///
/// The listed NFT is escrowed in a vault ATA owned by this PDA for the lifetime
/// of the listing. `payment_mint` decides how the listing is paid for:
///   * `None`        -> denominated in SOL, settled via `purchase`.
///   * `Some(mint)`  -> denominated in that SPL token, settled via `buy_with_token`.
#[account]
#[derive(InitSpace)]
pub struct Listing {
    /// The seller who created the listing and escrowed the NFT.
    pub maker: Pubkey,
    /// The NFT mint being sold.
    pub maker_mint: Pubkey,
    /// Asking price, expressed in lamports (SOL) or token base units.
    pub price: u64,
    /// Payment mint. `None` means the listing is priced in SOL.
    pub payment_mint: Option<Pubkey>,
    /// Bump for the listing PDA.
    pub bump: u8,
}
