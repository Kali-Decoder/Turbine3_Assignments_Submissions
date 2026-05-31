# NFT Staking Program (Solana + Anchor)

A decentralized NFT staking protocol built on **Solana** using the **Anchor Framework** and **Metaplex Core**. This project enables users to stake Metaplex Core NFTs, earn SPL token rewards over time, claim rewards without unstaking, and unstake at any time while preserving earned rewards.

## Features

### NFT Staking

* Stake Metaplex Core NFTs into the staking protocol.
* Track individual user staking positions.
* Maintain collection-level staking statistics.

### Reward Distribution

* Earn rewards continuously based on staking duration.
* Configurable reward emission rate.
* Accumulate rewards while NFTs remain staked.

### Standalone Reward Claims

* Claim earned rewards without removing NFTs from staking.
* Rewards are minted directly to the user's token account.
* Staking position remains active after claiming.

### Flexible Unstaking

* Unstake NFTs at any time.
* Automatically calculate and preserve pending rewards.
* Update staking statistics upon unstaking.

### SPL Token Integration

* Uses SPL Tokens as reward assets.
* PDA-controlled mint authority.
* Secure reward distribution via CPI.

---

## Architecture

### Staking Pool

The staking pool stores global protocol configuration:

| Field          | Description           |
| -------------- | --------------------- |
| authority      | Pool administrator    |
| reward_mint    | SPL reward token mint |
| reward_per_sec | Reward emission rate  |
| bump           | PDA bump seed         |

### User Stake Account

Each user maintains a staking state:

| Field                 | Description              |
| --------------------- | ------------------------ |
| staked_count          | Number of NFTs staked    |
| last_update_timestamp | Last reward update       |
| accumulated_rewards   | Stored unclaimed rewards |

---

## PDA Structure

### Pool PDA

```rust
seeds = [
    b"pool",
    authority.key().as_ref()
]
```

Stores staking configuration and acts as reward mint authority.

### User Stake PDA

```rust
seeds = [
    b"user_stake",
    pool.key().as_ref(),
    user.key().as_ref()
]
```

Stores user staking data and reward information.

---

## Program Instructions

### Initialize Pool

Creates the staking pool configuration.

#### Accounts

* Authority (Signer)
* Pool PDA
* Reward Mint
* System Program

#### Parameters

```rust
reward_per_sec: u64
```

---

### Stake

Allows a user to stake an NFT.

#### Functionality

* Creates or updates user stake account.
* Updates pending rewards.
* Increments staked NFT count.
* Updates collection staking metadata.

---

### Claim Rewards

Allows users to claim accumulated rewards without unstaking.

#### Functionality

* Calculates pending rewards.
* Mints reward tokens to user account.
* Resets accumulated rewards.
* Keeps NFT actively staked.

---

### Unstake

Allows users to remove NFTs from staking.

#### Functionality

* Calculates final rewards.
* Updates staking statistics.
* Decrements staked NFT count.
* Preserves earned rewards.

---

## Reward Formula

Rewards are calculated using:

```text
Rewards =
(Time Elapsed)
× Reward Rate
× Number of NFTs Staked
```

Example:

```text
Reward Rate: 1 token/sec
NFTs Staked: 2
Time Staked: 100 seconds

Rewards = 100 × 1 × 2
Rewards = 200 Tokens
```

---

## Project Structure

```text
nft-staking/
│
├── programs/
│   └── nft-staking/
│       └── src/
│           └── lib.rs
│
├── tests/
│   └── nft-staking.ts
│
├── migrations/
│
├── Anchor.toml
├── Cargo.toml
└── package.json
```

---

## Technology Stack

### Solana

High-performance blockchain for low-cost and fast transactions.

### Anchor Framework

Provides a secure framework for Solana smart contract development.

### Metaplex Core

Used for NFT collections, assets, and plugin management.

### SPL Token Program

Handles reward token minting and transfers.

### TypeScript

Used for integration and testing.

---

## Testing

The project includes integration tests covering:

### Pool Initialization

* Creates staking pool.
* Verifies configuration parameters.

### NFT Staking

* Stakes Metaplex Core NFTs.
* Updates collection metadata.

### Reward Claiming

* Claims rewards independently.
* Ensures staking position remains active.

### Unstaking

* Unstakes immediately after claiming rewards.
* Verifies staking count updates correctly.

Run tests:

```bash
anchor test --provider.cluster devnet
```

Run without redeployment:

```bash
anchor test --provider.cluster devnet --skip-deploy --skip-build
```

---

## Deployment

### Build Program

```bash
anchor build
```

### Deploy to Devnet

```bash
anchor deploy --provider.cluster devnet
```

### Verify Deployment

```bash
solana program show <PROGRAM_ID> --url devnet
```

---

## Security Considerations

* PDA-based authority management.
* Deterministic account derivation.
* Secure reward minting via CPI.
* Controlled token mint authority.
* Account ownership validation through Anchor constraints.

---

## Future Improvements

* Multi-NFT staking support.
* Reward multipliers.
* NFT rarity-based rewards.
* Lock-up periods.
* DAO-controlled reward parameters.
* Dynamic emission schedules.
* Leaderboards and staking analytics.

---

## Learning Objectives

This project demonstrates:

* Solana Program Development
* Anchor Framework Fundamentals
* PDA Derivation and Signing
* CPI (Cross Program Invocation)
* SPL Token Minting
* Metaplex Core Integration
* NFT Staking Mechanics
* Reward Distribution Systems

---

## Author

**Neeraj Choubisa (Nikku.dev)**

Full Stack Blockchain Developer | Solana Builder | Web3 Educator

* 30+ Hackathon Wins
* Turbin3 Builder
* Midnight Ecosystem Contributor
* Blockchain Developer & Community Builder

Built as part of the Turbin3 Solana Development Program.


## Deployment Details

The NFT Staking program has been successfully deployed and verified on Solana Devnet.

### Program Information

| Field                | Value                                                                                     |
| -------------------- | ----------------------------------------------------------------------------------------- |
| Program Name         | nft_staking                                                                               |
| Network              | Solana Devnet                                                                             |
| Program ID           | `6YVayRULP5LeU7QR573Z2QFK4aw5W9oYnBkvnDaWDCg`                                             |
| Deployment Signature | `v9zH4tRsBDnqvLfxtDnsnZkCBrczE878h6Yk9Qf2JRTVyaZS6PGXNhyubWe7zUUMA51BamyuP6wt4QdaDCobwyx` |
| IDL Account          | `9atvLUi9zYDym76bNn158gZrWkmjnSxL16vscnFrDGve`                                            |

### Deployment Output

```bash
Deploying cluster: https://api.devnet.solana.com

Program Id:
6YVayRULP5LeU7QR573Z2QFK4aw5W9oYnBkvnDaWDCg

Signature:
v9zH4tRsBDnqvLfxtDnsnZkCBrczE878h6Yk9Qf2JRTVyaZS6PGXNhyubWe7zUUMA51BamyuP6wt4QdaDCobwyx

Program confirmed on-chain

IDL Account:
9atvLUi9zYDym76bNn158gZrWkmjnSxL16vscnFrDGve

Deploy success
```

### Verification

Verify the deployed program:

```bash
solana program show 6YVayRULP5LeU7QR573Z2QFK4aw5W9oYnBkvnDaWDCg --url devnet
```

View transaction on Solana Explorer:

https://explorer.solana.com/tx/v9zH4tRsBDnqvLfxtDnsnZkCBrczE878h6Yk9Qf2JRTVyaZS6PGXNhyubWe7zUUMA51BamyuP6wt4QdaDCobwyx?cluster=devnet

View Program on Solana Explorer:

https://explorer.solana.com/address/6YVayRULP5LeU7QR573Z2QFK4aw5W9oYnBkvnDaWDCg?cluster=devnet


## Test Results

### Assignment Challenges Completed

✅ Pool Initialization

✅ NFT Staking with Metaplex Core

✅ Standalone Reward Claiming (Without Unstaking)

✅ Instant Unstaking After Reward Claim

### Sample Test Output

```bash
✔ Initializes the staking platform configuration pool

✔ Stakes a Metaplex Core NFT & increments collection attribute plugin

✔ Challenge 1a: Allows claiming rewards standalone without unstaking

✔ Challenge 1b: Allows user to unstake instantly directly after claiming rewards
```

This confirms:

* Users can stake NFTs.
* Rewards accrue over time.
* Rewards can be claimed independently.
* NFTs remain staked after reward claims.
* Users can unstake immediately after claiming rewards.
