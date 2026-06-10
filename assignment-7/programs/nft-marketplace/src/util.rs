use anchor_lang::prelude::*;

use crate::{constants::BPS_DENOMINATOR, error::MarketplaceError};

/// Split `price` into `(amount_to_seller, marketplace_fee)` using `fee_bps`
/// basis points. The buyer always pays exactly `price`; the fee is taken out
/// of the seller's proceeds.
pub fn split_price(price: u64, fee_bps: u16) -> Result<(u64, u64)> {
    let fee = (price as u128)
        .checked_mul(fee_bps as u128)
        .ok_or(MarketplaceError::Overflow)?
        .checked_div(BPS_DENOMINATOR as u128)
        .ok_or(MarketplaceError::Overflow)? as u64;
    let seller = price.checked_sub(fee).ok_or(MarketplaceError::Overflow)?;
    Ok((seller, fee))
}
