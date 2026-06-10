use anchor_lang::prelude::*;

/// PDA seed prefixes used across the program.
#[constant]
pub const MARKETPLACE_SEED: &[u8] = b"marketplace";
#[constant]
pub const TREASURY_SEED: &[u8] = b"treasury";
#[constant]
pub const REWARDS_SEED: &[u8] = b"rewards";
#[constant]
pub const LISTING_SEED: &[u8] = b"listing";
#[constant]
pub const OFFER_SEED: &[u8] = b"offer";

/// Maximum length, in bytes, of a marketplace name.
pub const MAX_NAME_LEN: usize = 32;

/// Fee denominator: fees are expressed in basis points (1 bps = 0.01%).
pub const BPS_DENOMINATOR: u64 = 10_000;

/// Reward tokens (6 decimals) minted to a buyer on each successful purchase.
pub const REWARD_AMOUNT: u64 = 1_000_000;
