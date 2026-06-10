# NFT Marketplace

A Solana program (Anchor) for listing and trading NFTs. It supports two ways to
price a sale — in SOL or in an SPL token — and a make-offer / accept-offer flow
where buyers escrow SOL and bid below the asking price.

Built for Assignment 2 (Week 5). Written from scratch; the three optional
challenges are all implemented and tested.

![All tests passing](docs/tests-passing.png)

## What it does

- **Initialize** a marketplace with a fee (in basis points), a SOL treasury, and
  a rewards mint.
- **List** an NFT for sale, priced either in SOL or in any SPL token.
- **Delist** to cancel a listing and get the NFT back.
- **Purchase** a SOL-priced listing.
- **Buy with token** — settle a token-priced listing by paying the listing's
  payment mint (e.g. USDC).
- **Make / accept / cancel offer** — bid a custom amount of SOL on a listed NFT.

Every successful sale also mints reward tokens to the buyer.

## Accounts

| Account       | Seeds                                    | Purpose |
|---------------|------------------------------------------|---------|
| `Marketplace` | `["marketplace", name]`                  | Config: admin, fee, name, bumps. |
| `Listing`     | `["listing", marketplace, maker_mint]`   | An NFT escrowed for sale, with its price and optional `payment_mint`. |
| `Offer`       | `["offer", maker_mint, buyer]`           | A buyer's standing SOL offer; escrows the offered lamports. |

The marketplace also owns two PDAs derived from it:

- `treasury` — a `SystemAccount` (`["treasury", marketplace]`) that collects fees
  on **SOL** sales.
- `rewards_mint` — a mint (`["rewards", marketplace]`) whose authority is the
  marketplace PDA, used to mint buyer rewards.

The listed NFT is held in a **vault** — an associated token account owned by the
`Listing` PDA — for the lifetime of the listing.

## Instructions

| Instruction     | Signer | Summary |
|-----------------|--------|---------|
| `initialize`    | admin  | Create the marketplace, treasury PDA and rewards mint. |
| `list`          | maker  | Escrow the NFT into the vault; record `price` and `payment_mint` (`None` = SOL). |
| `delist`        | maker  | Return the NFT, close the vault and listing. |
| `purchase`      | buyer  | Pay a SOL listing; split SOL between maker and treasury. |
| `buy_with_token`| buyer  | Pay a token listing via `transfer_checked`; split tokens between maker and treasury. |
| `make_offer`    | buyer  | Escrow SOL into an `Offer` PDA at a self-chosen amount. |
| `accept_offer`  | maker  | Sell at the offered amount; release escrow and the NFT. |
| `cancel_offer`  | buyer  | Refund the escrowed SOL and close the offer. |

### Fee model

The buyer always pays exactly `price`. The marketplace fee is taken out of the
seller's proceeds: `fee = price * fee_bps / 10_000`, the maker receives
`price - fee`, and the treasury receives `fee`. The same split is applied to an
accepted offer's amount.

## The three challenges

**1. Delist** — `delist` transfers the NFT from the vault back to the maker,
then closes the vault (via a `close_account` CPI signed by the listing PDA) and
the listing account, refunding rent to the maker.

**2. SPL token payments** — `Listing.payment_mint: Option<Pubkey>` denominates a
listing. `list(price, Some(mint))` prices it in an SPL token; `buy_with_token`
settles it with `token_interface::transfer_checked`, splitting the payment
between the maker's ATA and a treasury ATA owned by the marketplace PDA. Mints
use `Interface<TokenInterface>` and `InterfaceAccount<Mint>` throughout, so the
program is Token-2022 ready, exactly like the rewards mint. (For SOL listings the
treasury stays a `SystemAccount`; for token listings the treasury is an ATA.)

**3. Make-offer / accept-offer** — `make_offer` opens an `Offer` PDA at
`["offer", asset, buyer]` and escrows the offered lamports into it via a
system-program transfer. `accept_offer` lets the maker sell at the offer amount
instead of the listed price: it releases the escrowed SOL from the PDA (split
maker/treasury), transfers the NFT to the buyer, and closes the listing and
offer. `cancel_offer` refunds the buyer and closes the offer.

Because the `Offer` PDA is program-owned and carries data, its escrowed lamports
are moved out by direct balance manipulation rather than a system CPI. In
`accept_offer` that release is done *after* the token CPIs, so the maker and
treasury accounts aren't credited while also being passed into a CPI.

## Design notes

- **NFTs are modeled as SPL mints with 0 decimals and supply 1.** The program
  deliberately doesn't depend on Metaplex Token Metadata, which keeps the tests
  fully self-contained and deterministic. The marketplace mechanics (escrow,
  token CPIs, ATAs, PDAs, offers) are identical regardless of metadata.
- **Heavy account structs are `Box`ed.** Instructions like `buy_with_token`
  carry many `InterfaceAccount`s and `init_if_needed` ATAs; boxing them moves the
  accounts to the heap and keeps the instruction within Solana's per-frame stack
  limit.

## Project layout

```
programs/nft-marketplace/src
├── lib.rs              # program entrypoints
├── constants.rs        # PDA seeds, fee denominator, reward amount
├── error.rs            # custom errors
├── util.rs             # fee/price split helper
├── state/              # Marketplace, Listing, Offer
└── instructions/       # one file per instruction
tests/nft-marketplace.ts  # end-to-end tests for every instruction
```

## Build and test

Requires the Solana and Anchor toolchains.

```bash
# versions this was built with
solana-cli 3.1.14
anchor-cli 0.32.1

# install JS deps
yarn install

# compile the program
anchor build

# run the full test suite on a local validator
anchor test
```

`anchor test` spins up a local validator, deploys the program, and runs the
TypeScript suite in `tests/`, which exercises all eight instructions:
initialize, list + purchase (SOL), list + delist, list + buy_with_token,
make_offer + accept_offer, and make_offer + cancel_offer.
