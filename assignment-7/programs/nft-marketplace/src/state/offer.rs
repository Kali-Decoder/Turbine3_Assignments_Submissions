use anchor_lang::prelude::*;

/// A standing SOL offer made by a buyer against a listed NFT.
///
/// The offered lamports are escrowed directly in this PDA. The maker may
/// `accept_offer` to sell at `amount` instead of the listed price, or the
/// buyer may `cancel_offer` to reclaim their escrowed SOL.
#[account]
#[derive(InitSpace)]
pub struct Offer {
    /// The buyer who made (and funded) the offer.
    pub buyer: Pubkey,
    /// The NFT mint the offer targets.
    pub maker_mint: Pubkey,
    /// Lamports escrowed in this PDA and offered for the NFT.
    pub amount: u64,
    /// Bump for the offer PDA.
    pub bump: u8,
}
