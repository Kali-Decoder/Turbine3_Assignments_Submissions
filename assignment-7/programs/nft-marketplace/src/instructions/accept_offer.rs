use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, mint_to, transfer_checked, CloseAccount, Mint, MintTo, TokenAccount,
        TokenInterface, TransferChecked,
    },
};

use crate::{
    constants::{LISTING_SEED, MARKETPLACE_SEED, OFFER_SEED, REWARDS_SEED, REWARD_AMOUNT, TREASURY_SEED},
    state::{Listing, Marketplace, Offer},
    util::split_price,
};

/// Accepts a standing offer: the maker sells the NFT at the offer's amount
/// instead of the listed price. The escrowed SOL is released from the Offer PDA
/// (split between maker and treasury), the NFT goes to the buyer, and both the
/// listing and offer are closed.
#[derive(Accounts)]
pub struct AcceptOffer<'info> {
    /// The seller accepting the offer.
    #[account(mut)]
    pub maker: Signer<'info>,

    /// The buyer who made the offer; receives the NFT and the offer rent refund.
    #[account(mut)]
    pub buyer: SystemAccount<'info>,

    #[account(
        seeds = [MARKETPLACE_SEED, marketplace.name.as_bytes()],
        bump = marketplace.bump,
    )]
    pub marketplace: Box<Account<'info, Marketplace>>,

    #[account(
        mut,
        seeds = [TREASURY_SEED, marketplace.key().as_ref()],
        bump = marketplace.treasury_bump,
    )]
    pub treasury: SystemAccount<'info>,

    pub maker_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = maker_mint,
        associated_token::authority = buyer,
        associated_token::token_program = token_program,
    )]
    pub buyer_nft_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = maker_mint,
        associated_token::authority = listing,
        associated_token::token_program = token_program,
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [REWARDS_SEED, marketplace.key().as_ref()],
        bump = marketplace.rewards_bump,
        mint::token_program = token_program,
    )]
    pub rewards_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = rewards_mint,
        associated_token::authority = buyer,
        associated_token::token_program = token_program,
    )]
    pub buyer_rewards_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        close = maker,
        has_one = maker,
        has_one = maker_mint,
        seeds = [LISTING_SEED, marketplace.key().as_ref(), maker_mint.key().as_ref()],
        bump = listing.bump,
    )]
    pub listing: Box<Account<'info, Listing>>,

    #[account(
        mut,
        close = buyer,
        has_one = buyer,
        has_one = maker_mint,
        seeds = [OFFER_SEED, maker_mint.key().as_ref(), buyer.key().as_ref()],
        bump = offer.bump,
    )]
    pub offer: Box<Account<'info, Offer>>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn accept_offer_handler(ctx: Context<AcceptOffer>) -> Result<()> {
    let amount = ctx.accounts.offer.amount;
    let (seller_amount, fee) = split_price(amount, ctx.accounts.marketplace.fee)?;

    // Settle the NFT side: release to buyer, mint rewards, close the vault.
    let marketplace_key = ctx.accounts.marketplace.key();
    let maker_mint_key = ctx.accounts.maker_mint.key();
    let listing_seeds: &[&[&[u8]]] = &[&[
        LISTING_SEED,
        marketplace_key.as_ref(),
        maker_mint_key.as_ref(),
        &[ctx.accounts.listing.bump],
    ]];

    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.maker_mint.to_account_info(),
                to: ctx.accounts.buyer_nft_ata.to_account_info(),
                authority: ctx.accounts.listing.to_account_info(),
            },
            listing_seeds,
        ),
        1,
        ctx.accounts.maker_mint.decimals,
    )?;

    let name = ctx.accounts.marketplace.name.clone();
    let marketplace_seeds: &[&[&[u8]]] =
        &[&[MARKETPLACE_SEED, name.as_bytes(), &[ctx.accounts.marketplace.bump]]];
    mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.rewards_mint.to_account_info(),
                to: ctx.accounts.buyer_rewards_ata.to_account_info(),
                authority: ctx.accounts.marketplace.to_account_info(),
            },
            marketplace_seeds,
        ),
        REWARD_AMOUNT,
    )?;

    close_account(CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        CloseAccount {
            account: ctx.accounts.vault.to_account_info(),
            destination: ctx.accounts.maker.to_account_info(),
            authority: ctx.accounts.listing.to_account_info(),
        },
        listing_seeds,
    ))?;

    // Release the escrowed SOL from the Offer PDA. The PDA is program-owned and
    // carries data, so lamports are moved by direct balance manipulation rather
    // than a system-program CPI. This runs after the token CPIs so that `maker`
    // and `treasury` are not credited while also being passed into a CPI. The
    // remaining rent is returned to the buyer by the `close = buyer` constraint.
    **ctx.accounts.offer.to_account_info().try_borrow_mut_lamports()? -= amount;
    **ctx.accounts.maker.to_account_info().try_borrow_mut_lamports()? += seller_amount;
    **ctx.accounts.treasury.to_account_info().try_borrow_mut_lamports()? += fee;

    Ok(())
}
