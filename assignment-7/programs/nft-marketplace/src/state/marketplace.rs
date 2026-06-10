use anchor_lang::prelude::*;

/// Global configuration for a single marketplace instance.
///
/// One marketplace owns a SOL `treasury` (for fees on SOL sales), a `rewards`
/// mint (buyers are minted reward tokens on every successful purchase) and is
/// identified by a human-readable `name` which is part of its PDA seeds.
#[account]
#[derive(InitSpace)]
pub struct Marketplace {
    /// Authority allowed to administer the marketplace.
    pub admin: Pubkey,
    /// Marketplace fee charged on every sale, in basis points.
    pub fee: u16,
    /// Bump for the marketplace PDA.
    pub bump: u8,
    /// Bump for the SOL treasury PDA.
    pub treasury_bump: u8,
    /// Bump for the rewards mint PDA.
    pub rewards_bump: u8,
    /// Human-readable name; part of the PDA seeds.
    #[max_len(32)]
    pub name: String,
}
