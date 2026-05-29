# Assignment 2 — SPL Token + NFT Minting

This folder contains my **Turbin3 Assignment 2** submission for **Kali-Decoder**.

The scripts here walk through two flows on **Solana devnet**:

- creating and managing an SPL token
- uploading NFT assets to Irys and minting the NFT on-chain

## Project setup

Before running anything, make sure these files are in place at the project root:

- `devnet-wallet.json` — your devnet keypair
- `image.jpeg` — the image you want to mint as an NFT
- `.env` — copied from `.env.example`

Install dependencies and create the local environment file:

```bash
npm install
cp .env.example .env
```

The default `.env` values point to Solana devnet. If devnet is unavailable, you can switch to a local validator later.

## What the scripts do

### SPL token flow

| Command | Purpose |
|---|---|
| `npm run spl:init` | Creates the SPL mint account |
| `npm run spl:metadata` | Writes token metadata to the mint |
| `npm run spl:mint` | Creates the ATA and mints tokens |
| `npm run spl:transfer` | Transfers tokens to another wallet |

### NFT flow

| Command | Purpose |
|---|---|
| `npm run nft:image` | Uploads `image.jpeg` to Irys and prints the image URI |
| `npm run nft:metadata` | Uploads NFT metadata JSON to Irys and prints the metadata URI |
| `npm run nft:mint` | Mints the NFT on-chain using the metadata URI |

## How to mint the NFT

1. Update the NFT fields in `src/nft/nft_metadata.ts` and `src/nft/nft_mint.ts` if you want to change the name or description.
2. Run `npm run nft:image` and copy the printed image URI.
3. Paste that URI into `src/nft/nft_metadata.ts`, then run `npm run nft:metadata`.
4. Copy the printed metadata URI into `src/nft/nft_mint.ts`.
5. Run `npm run nft:mint` to mint the NFT on devnet.

## How to run the SPL token flow

1. Run `npm run spl:init` and copy the mint address it prints.
2. Paste that mint address into:
   - `src/spl/spl_metadata.ts`
   - `src/spl/spl_mint.ts`
   - `src/spl/spl_transfer.ts`
3. If you want to transfer tokens, also paste the recipient wallet into `src/spl/spl_transfer.ts`.
4. Run the remaining SPL commands in order.

## Kali-Decoder branding

The metadata in this assignment is now aligned with **Kali-Decoder** instead of Janhavi. The NFT metadata, NFT mint name, and SPL token metadata should all reflect the correct submission identity.

## Submission checklist

After you mint everything, keep these details for your submission:

- SPL mint address
- NFT asset address
- image URI
- metadata URI
- transaction signatures

## My completed NFT run

These are the values from my successful Kali-Decoder NFT mint on devnet:

- Image URI: `https://gateway.irys.xyz/HDbnDD5vAJgkik6fdxqZhtm44ojKohjJGtb5DZWdGRFA`
- Metadata URI: `https://gateway.irys.xyz/By4WDd6m9UEMmLgygYgpkUQMtEqnCKjGhqbusb5NB1ta`
- NFT asset address: `4bkSuiFT4upKYrL5Ww6KAj2VmqAGE1pkCi21BDtcMNmE`
- Mint signature: `58vo56Umh1k3KTzTd51zpot1KFjEx32eJ4TLFskJ6QpeoFjFs4MtEif3MJus7cYn3LtoU6CArE8fDvWnD1WAjruT`

Explorer links:

- [NFT asset on Solana Explorer](https://explorer.solana.com/address/4bkSuiFT4upKYrL5Ww6KAj2VmqAGE1pkCi21BDtcMNmE?cluster=devnet)
- [NFT mint transaction on Solana Explorer](https://explorer.solana.com/tx/58vo56Umh1k3KTzTd51zpot1KFjEx32eJ4TLFskJ6QpeoFjFs4MtEif3MJus7cYn3LtoU6CArE8fDvWnD1WAjruT?cluster=devnet)
- [Image URI on Irys](https://gateway.irys.xyz/HDbnDD5vAJgkik6fdxqZhtm44ojKohjJGtb5DZWdGRFA)
- [Metadata URI on Irys](https://gateway.irys.xyz/By4WDd6m9UEMmLgygYgpkUQMtEqnCKjGhqbusb5NB1ta)

## Notes

- Use a funded devnet wallet before minting.
- If a script still has a placeholder mint or URI, replace it with the value printed by the previous step.
- Phantom should be switched to **Devnet** if you want to view the SPL token balance.
