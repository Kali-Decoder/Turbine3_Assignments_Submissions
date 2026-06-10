use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{constants::OFFER_SEED, state::Offer};

/// Cancels an open offer, refunding the escrowed SOL (and the PDA rent) back to
/// the buyer and closing the Offer account.
#[derive(Accounts)]
pub struct CancelOffer<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    pub maker_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        close = buyer,
        has_one = buyer,
        has_one = maker_mint,
        seeds = [OFFER_SEED, maker_mint.key().as_ref(), buyer.key().as_ref()],
        bump = offer.bump,
    )]
    pub offer: Account<'info, Offer>,

    pub system_program: Program<'info, System>,
}

pub fn cancel_offer_handler(_ctx: Context<CancelOffer>) -> Result<()> {
    // `close = buyer` returns all lamports held by the Offer PDA (rent + the
    // escrowed offer amount) to the buyer.
    Ok(())
}
