pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
pub mod util;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("Jz389nQRM6HGu8jbhK2xaKQz3F9HViUsmBhVPumSpLA");

#[program]
pub mod nft_marketplace {
    use super::*;

    /// Create a marketplace, its SOL treasury and its rewards mint.
    pub fn initialize(ctx: Context<Initialize>, name: String, fee: u16) -> Result<()> {
        instructions::initialize::initialize_handler(ctx, name, fee)
    }

    /// List an NFT for sale, priced in SOL (`payment_mint = None`) or in an SPL
    /// token (`payment_mint = Some(mint)`).
    pub fn list(ctx: Context<List>, price: u64, payment_mint: Option<Pubkey>) -> Result<()> {
        instructions::list::list_handler(ctx, price, payment_mint)
    }

    /// Cancel a listing and return the NFT to the maker.
    pub fn delist(ctx: Context<Delist>) -> Result<()> {
        instructions::delist::delist_handler(ctx)
    }

    /// Buy a SOL-denominated listing.
    pub fn purchase(ctx: Context<Purchase>) -> Result<()> {
        instructions::purchase::purchase_handler(ctx)
    }

    /// Buy a token-denominated listing, paying in the listing's payment mint.
    pub fn buy_with_token(ctx: Context<BuyWithToken>) -> Result<()> {
        instructions::buy_with_token::buy_with_token_handler(ctx)
    }

    /// Make a SOL offer on a listed NFT at a self-chosen price.
    pub fn make_offer(ctx: Context<MakeOffer>, amount: u64) -> Result<()> {
        instructions::make_offer::make_offer_handler(ctx, amount)
    }

    /// Accept a standing offer, selling the NFT at the offered amount.
    pub fn accept_offer(ctx: Context<AcceptOffer>) -> Result<()> {
        instructions::accept_offer::accept_offer_handler(ctx)
    }

    /// Cancel an open offer and refund the escrowed SOL to the buyer.
    pub fn cancel_offer(ctx: Context<CancelOffer>) -> Result<()> {
        instructions::cancel_offer::cancel_offer_handler(ctx)
    }
}
