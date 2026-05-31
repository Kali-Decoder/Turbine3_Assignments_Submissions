import anchor from "@coral-xyz/anchor";
const { BN } = anchor;
import { expect } from "chai";
import { 
  createMint, 
  getOrCreateAssociatedTokenAccount, 
  TOKEN_PROGRAM_ID 
} from "@solana/spl-token";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import { generateSigner, publicKey } from "@metaplex-foundation/umi";
import { createSignerFromWalletAdapter } from "@metaplex-foundation/umi-signer-wallet-adapters";
import { createCollection, create, mplCore } from "@metaplex-foundation/mpl-core";

describe("nft-staking", () => {
  // 1. Configure provider contexts
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.NftStaking;
  const connection = provider.connection;
  const wallet = provider.wallet;

  // 2. Setup Umi connected to Devnet with the Core plugin
  const umi = createUmi(connection.rpcEndpoint).use(mplCore());
  
  // Wrap Anchor's wallet to be compatible with Umi's signer interfaces
  const umiUserSigner = createSignerFromWalletAdapter(wallet);
  umi.identity = umiUserSigner;
  umi.payer = umiUserSigner;

  // 3. Setup Test Account Tracking States
  let poolPda;
  let userStakePda;
  let rewardMint;
  let userRewardAccount;
  
  let collectionAddress;
  let assetAddress;

  const rewardPerSec = new BN(1000000); // 1 token base unit per second

  before(async () => {
    console.log("--- Initializing Test Setup States On Devnet ---");
    
    // Derive our Program PDA account locations
    [poolPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("pool"), wallet.publicKey.toBuffer()],
      program.programId
    );

    [userStakePda] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("user_stake"), 
        poolPda.toBuffer(), 
        wallet.publicKey.toBuffer()
      ],
      program.programId
    );

    // Create a new reward token mint on devnet
    rewardMint = await createMint(
      connection,
      wallet.payer,
      poolPda, // Pool PDA acts as the mint authority
      null,
      6
    );

    // Initialize/Fetch the staker's ATA reward account
    userRewardAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      wallet.payer,
      rewardMint,
      wallet.publicKey
    );

    // Mint a Core Collection asset on Devnet using Umi
    const collectionSigner = generateSigner(umi);
    collectionAddress = new anchor.web3.PublicKey(collectionSigner.publicKey);
    
    await createCollection(umi, {
      collection: collectionSigner,
      name: "Staking Collection",
      uri: "https://example.com/collection.json",
    }).sendAndConfirm(umi);
    console.log(`Collection Created successfully: ${collectionAddress.toBase58()}`);

    // Create an individual Core NFT asset inside that Collection
    const assetSigner = generateSigner(umi);
    assetAddress = new anchor.web3.PublicKey(assetSigner.publicKey);

    await create(umi, {
      asset: assetSigner,
      collection: publicKey(collectionAddress.toBase58()), // Pass the collection address raw key
      authority: umi.identity,                            // Explicitly set your wallet as the collection authority
      name: "Staked Core NFT #1",
      uri: "https://example.com/nft.json",
    }).sendAndConfirm(umi);
    console.log(`NFT Asset Created successfully: ${assetAddress.toBase58()}`);
  });

  // --- ACTIONS & INSTRUCTION TEST ASSERTIONS ---

  it("Initializes the staking platform configuration pool", async () => {
    await program.methods
      .initializePool(rewardPerSec)
      .accounts({
        authority: wallet.publicKey,
        pool: poolPda,
        rewardMint: rewardMint,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const poolState = await program.account.stakingPool.fetch(poolPda);
    expect(poolState.authority.toBase58()).to.equal(wallet.publicKey.toBase58());
    expect(poolState.rewardPerSec.toString()).to.equal(rewardPerSec.toString());
  });

  it("Stakes a Metaplex Core NFT & increments collection attribute plugin", async () => {
    await program.methods
      .stake()
      .accounts({
        user: wallet.publicKey,
        pool: poolPda,
        userStake: userStakePda,
        collection: collectionAddress,
        collectionAuthority: wallet.publicKey,
        asset: assetAddress,
        mplCoreProgram: new anchor.web3.PublicKey("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d"),
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const stakeState = await program.account.userStakeAccount.fetch(userStakePda);
    expect(stakeState.stakedCount.toNumber()).to.equal(1);
    expect(stakeState.lastUpdateTimestamp.toNumber()).to.be.greaterThan(0);
  });

  it("Challenge 1a: Allows claiming rewards standalone without unstaking", async () => {
    console.log("Simulating block processing intervals to collect yield distribution...");
    await new Promise((resolve) => setTimeout(resolve, 3000));

    await program.methods
      .claimRewards()
      .accounts({
        user: wallet.publicKey,
        pool: poolPda,
        userStake: userStakePda,
        rewardMint: rewardMint,
        userRewardAccount: userRewardAccount.address,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const stakeState = await program.account.userStakeAccount.fetch(userStakePda);
    const tokenBalance = await connection.getTokenAccountBalance(userRewardAccount.address);

    // Confirm rewards were distributed successfully while the NFT remains locked/staked
    expect(Number(tokenBalance.value.amount)).to.be.greaterThan(0);
    expect(stakeState.stakedCount.toNumber()).to.equal(1);
    console.log(`Standalone rewards claim successful. Staker balance: ${tokenBalance.value.uiAmount} Tokens.`);
  });

  it("Challenge 1b: Allows user to unstake instantly directly after claiming rewards", async () => {
    // Execute unstaking immediately following the claim method
    await program.methods
      .unstake()
      .accounts({
        user: wallet.publicKey,
        pool: poolPda,
        userStake: userStakePda,
        collection: collectionAddress,
        collectionAuthority: wallet.publicKey,
        asset: assetAddress,
        mplCoreProgram: new anchor.web3.PublicKey("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d"),
      })
      .rpc();

    const stakeState = await program.account.userStakeAccount.fetch(userStakePda);
    expect(stakeState.stakedCount.toNumber()).to.equal(0);
    console.log("Successfully unstaked the asset immediately following a claim instruction!");
  });
});