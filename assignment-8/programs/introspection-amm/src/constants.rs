use anchor_lang::prelude::*;

#[constant]
pub const CONFIG_SEED: &[u8] = b"config";
#[constant]
pub const LP_SEED: &[u8] = b"lp";

pub const FEE_DENOMINATOR: u64 = 10_000;
