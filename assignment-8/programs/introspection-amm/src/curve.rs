use anchor_lang::prelude::*;

use crate::constants::FEE_DENOMINATOR;
use crate::error::AmmError;

pub fn sqrt_u128(value: u128) -> u128 {
    if value < 2 {
        return value;
    }
    let mut x = value;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + value / x) / 2;
    }
    x
}

pub fn quote_b_for_a(amount_a: u64, reserve_a: u64, reserve_b: u64) -> Result<u64> {
    require!(reserve_a > 0 && reserve_b > 0, AmmError::EmptyReserves);
    let out = (amount_a as u128)
        .checked_mul(reserve_b as u128)
        .ok_or(AmmError::Overflow)?
        .checked_div(reserve_a as u128)
        .ok_or(AmmError::Overflow)?;
    u64::try_from(out).map_err(|_| AmmError::Overflow.into())
}

pub fn initial_lp_amount(amount_a: u64, amount_b: u64) -> Result<u64> {
    let product = (amount_a as u128)
        .checked_mul(amount_b as u128)
        .ok_or(AmmError::Overflow)?;
    u64::try_from(sqrt_u128(product)).map_err(|_| AmmError::Overflow.into())
}

pub fn lp_from_deposit(amount_a: u64, reserve_a: u64, lp_supply: u64) -> Result<u64> {
    require!(reserve_a > 0 && lp_supply > 0, AmmError::EmptyReserves);
    let out = (amount_a as u128)
        .checked_mul(lp_supply as u128)
        .ok_or(AmmError::Overflow)?
        .checked_div(reserve_a as u128)
        .ok_or(AmmError::Overflow)?;
    u64::try_from(out).map_err(|_| AmmError::Overflow.into())
}

pub fn withdraw_amount(lp_amount: u64, reserve: u64, lp_supply: u64) -> Result<u64> {
    require!(lp_supply > 0, AmmError::EmptyReserves);
    let out = (lp_amount as u128)
        .checked_mul(reserve as u128)
        .ok_or(AmmError::Overflow)?
        .checked_div(lp_supply as u128)
        .ok_or(AmmError::Overflow)?;
    u64::try_from(out).map_err(|_| AmmError::Overflow.into())
}

pub fn swap_output(
    amount_in: u64,
    reserve_in: u64,
    reserve_out: u64,
    fee_bps: u16,
) -> Result<u64> {
    require!(reserve_in > 0 && reserve_out > 0, AmmError::EmptyReserves);
    require!(amount_in > 0, AmmError::ZeroAmount);

    let fee_denom = FEE_DENOMINATOR as u128;
    let fee_num = fee_denom
        .checked_sub(fee_bps as u128)
        .ok_or(AmmError::InvalidFee)?;

    let amount_in_with_fee = (amount_in as u128)
        .checked_mul(fee_num)
        .ok_or(AmmError::Overflow)?;

    let numerator = amount_in_with_fee
        .checked_mul(reserve_out as u128)
        .ok_or(AmmError::Overflow)?;
    let denominator = (reserve_in as u128)
        .checked_mul(fee_denom)
        .ok_or(AmmError::Overflow)?
        .checked_add(amount_in_with_fee)
        .ok_or(AmmError::Overflow)?;

    u64::try_from(
        numerator
            .checked_div(denominator)
            .ok_or(AmmError::Overflow)?,
    )
    .map_err(|_| AmmError::Overflow.into())
}
