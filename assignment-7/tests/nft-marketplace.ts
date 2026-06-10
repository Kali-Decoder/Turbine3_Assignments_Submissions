import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { NftMarketplace } from "../target/types/nft_marketplace";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  getOrCreateAssociatedTokenAccount,
  getAssociatedTokenAddressSync,
  getAccount,
  mintTo,
} from "@solana/spl-token";
import { Keypair, PublicKey, LAMPORTS_PER_SOL, SystemProgram } from "@solana/web3.js";
import { assert } from "chai";

describe("nft-marketplace", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.nftMarketplace as Program<NftMarketplace>;
  const connection = provider.connection;
  const payer = (provider.wallet as anchor.Wallet).payer;

  const NAME = "janhavi-mart";
  const FEE_BPS = 200; // 2%
  const REWARD_AMOUNT = 1_000_000; // 1 reward token (6 decimals)

  // PDAs shared across the whole marketplace.
  const marketplace = PublicKey.findProgramAddressSync(
    [Buffer.from("marketplace"), Buffer.from(NAME)],
    program.programId
  )[0];
  const treasury = PublicKey.findProgramAddressSync(
    [Buffer.from("treasury"), marketplace.toBuffer()],
    program.programId
  )[0];
  const rewardsMint = PublicKey.findProgramAddressSync(
    [Buffer.from("rewards"), marketplace.toBuffer()],
    program.programId
  )[0];

  const listingPda = (mint: PublicKey) =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("listing"), marketplace.toBuffer(), mint.toBuffer()],
      program.programId
    )[0];

  const offerPda = (mint: PublicKey, buyer: PublicKey) =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("offer"), mint.toBuffer(), buyer.toBuffer()],
      program.programId
    )[0];

  const ata = (mint: PublicKey, owner: PublicKey) =>
    getAssociatedTokenAddressSync(mint, owner, true);

  /** Fund a fresh keypair with SOL. */
  async function fundedKeypair(sol = 10): Promise<Keypair> {
    const kp = Keypair.generate();
    const sig = await connection.requestAirdrop(kp.publicKey, sol * LAMPORTS_PER_SOL);
    await connection.confirmTransaction(sig, "confirmed");
    return kp;
  }

  /** Mint a brand-new NFT (decimals 0, supply 1) into `owner`'s ATA. */
  async function mintNft(owner: Keypair): Promise<{ mint: PublicKey; ownerAta: PublicKey }> {
    const mint = await createMint(connection, payer, payer.publicKey, null, 0);
    const ownerAta = (
      await getOrCreateAssociatedTokenAccount(connection, payer, mint, owner.publicKey)
    ).address;
    await mintTo(connection, payer, mint, ownerAta, payer, 1);
    return { mint, ownerAta };
  }

  it("initializes the marketplace", async () => {
    await program.methods
      .initialize(NAME, FEE_BPS)
      .accountsPartial({
        admin: payer.publicKey,
        marketplace,
        treasury,
        rewardsMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const mp = await program.account.marketplace.fetch(marketplace);
    assert.strictEqual(mp.name, NAME);
    assert.strictEqual(mp.fee, FEE_BPS);
    assert.ok(mp.admin.equals(payer.publicKey));
  });

  it("lists an NFT and lets a buyer purchase it with SOL", async () => {
    const maker = await fundedKeypair();
    const buyer = await fundedKeypair();
    const { mint, ownerAta: makerAta } = await mintNft(maker);
    const listing = listingPda(mint);
    const price = new BN(LAMPORTS_PER_SOL); // 1 SOL

    // List (SOL-denominated => payment_mint = null).
    await program.methods
      .list(price, null)
      .accountsPartial({
        maker: maker.publicKey,
        marketplace,
        makerMint: mint,
        makerAta,
        listing,
        vault: ata(mint, listing),
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker])
      .rpc();

    const listed = await program.account.listing.fetch(listing);
    assert.ok(listed.price.eq(price));
    assert.strictEqual(listed.paymentMint, null);
    const vaultAcc = await getAccount(connection, ata(mint, listing));
    assert.strictEqual(vaultAcc.amount.toString(), "1");

    const treasuryBefore = await connection.getBalance(treasury);
    const buyerNftAta = ata(mint, buyer.publicKey);
    const buyerRewardsAta = ata(rewardsMint, buyer.publicKey);

    // Purchase.
    await program.methods
      .purchase()
      .accountsPartial({
        buyer: buyer.publicKey,
        maker: maker.publicKey,
        marketplace,
        treasury,
        makerMint: mint,
        buyerAta: buyerNftAta,
        vault: ata(mint, listing),
        rewardsMint,
        buyerRewardsAta,
        listing,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([buyer])
      .rpc();

    // Buyer owns the NFT and earned rewards.
    const nft = await getAccount(connection, buyerNftAta);
    assert.strictEqual(nft.amount.toString(), "1");
    const rewards = await getAccount(connection, buyerRewardsAta);
    assert.strictEqual(rewards.amount.toString(), REWARD_AMOUNT.toString());

    // Treasury collected the 2% fee.
    const fee = price.muln(FEE_BPS).divn(10_000).toNumber();
    const treasuryAfter = await connection.getBalance(treasury);
    assert.strictEqual(treasuryAfter - treasuryBefore, fee);

    // Listing is closed.
    const closed = await program.account.listing.fetchNullable(listing);
    assert.isNull(closed);
  });

  it("lets the maker delist and reclaim the NFT", async () => {
    const maker = await fundedKeypair();
    const { mint, ownerAta: makerAta } = await mintNft(maker);
    const listing = listingPda(mint);

    await program.methods
      .list(new BN(LAMPORTS_PER_SOL), null)
      .accountsPartial({
        maker: maker.publicKey,
        marketplace,
        makerMint: mint,
        makerAta,
        listing,
        vault: ata(mint, listing),
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker])
      .rpc();

    await program.methods
      .delist()
      .accountsPartial({
        maker: maker.publicKey,
        marketplace,
        makerMint: mint,
        makerAta,
        listing,
        vault: ata(mint, listing),
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker])
      .rpc();

    // NFT is back with the maker and the listing is gone.
    const back = await getAccount(connection, makerAta);
    assert.strictEqual(back.amount.toString(), "1");
    const closed = await program.account.listing.fetchNullable(listing);
    assert.isNull(closed);
  });

  it("lists priced in an SPL token and buys with that token (buy_with_token)", async () => {
    const maker = await fundedKeypair();
    const buyer = await fundedKeypair();
    const { mint, ownerAta: makerAta } = await mintNft(maker);
    const listing = listingPda(mint);

    // A USDC-like payment token (6 decimals).
    const paymentMint = await createMint(connection, payer, payer.publicKey, null, 6);
    const price = new BN(100_000_000); // 100 tokens

    // Fund the buyer with payment tokens.
    const buyerPaymentAta = (
      await getOrCreateAssociatedTokenAccount(connection, payer, paymentMint, buyer.publicKey)
    ).address;
    await mintTo(connection, payer, paymentMint, buyerPaymentAta, payer, 1_000_000_000);

    // List priced in the payment token.
    await program.methods
      .list(price, paymentMint)
      .accountsPartial({
        maker: maker.publicKey,
        marketplace,
        makerMint: mint,
        makerAta,
        listing,
        vault: ata(mint, listing),
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker])
      .rpc();

    const listed = await program.account.listing.fetch(listing);
    assert.ok(listed.paymentMint!.equals(paymentMint));

    const makerPaymentAta = ata(paymentMint, maker.publicKey);
    const treasuryPaymentAta = ata(paymentMint, marketplace);
    const buyerNftAta = ata(mint, buyer.publicKey);
    const buyerRewardsAta = ata(rewardsMint, buyer.publicKey);

    await program.methods
      .buyWithToken()
      .accountsPartial({
        buyer: buyer.publicKey,
        maker: maker.publicKey,
        marketplace,
        paymentMint,
        makerMint: mint,
        buyerPaymentAta,
        makerPaymentAta,
        treasuryPaymentAta,
        buyerNftAta,
        vault: ata(mint, listing),
        rewardsMint,
        buyerRewardsAta,
        listing,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([buyer])
      .rpc();

    const fee = price.muln(FEE_BPS).divn(10_000);
    const sellerAmount = price.sub(fee);

    // Maker received proceeds, treasury received the fee, buyer received the NFT.
    const makerTokens = await getAccount(connection, makerPaymentAta);
    assert.strictEqual(makerTokens.amount.toString(), sellerAmount.toString());
    const treasuryTokens = await getAccount(connection, treasuryPaymentAta);
    assert.strictEqual(treasuryTokens.amount.toString(), fee.toString());
    const nft = await getAccount(connection, buyerNftAta);
    assert.strictEqual(nft.amount.toString(), "1");
    const closed = await program.account.listing.fetchNullable(listing);
    assert.isNull(closed);
  });

  it("supports make_offer then accept_offer at the offered price", async () => {
    const maker = await fundedKeypair();
    const buyer = await fundedKeypair();
    const { mint, ownerAta: makerAta } = await mintNft(maker);
    const listing = listingPda(mint);
    const offer = offerPda(mint, buyer.publicKey);
    const offerAmount = new BN(LAMPORTS_PER_SOL / 2); // half the listed price

    // List at 1 SOL.
    await program.methods
      .list(new BN(LAMPORTS_PER_SOL), null)
      .accountsPartial({
        maker: maker.publicKey,
        marketplace,
        makerMint: mint,
        makerAta,
        listing,
        vault: ata(mint, listing),
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker])
      .rpc();

    // Buyer makes a lower offer.
    await program.methods
      .makeOffer(offerAmount)
      .accountsPartial({
        buyer: buyer.publicKey,
        marketplace,
        makerMint: mint,
        listing,
        offer,
        systemProgram: SystemProgram.programId,
      })
      .signers([buyer])
      .rpc();

    const offerAcc = await program.account.offer.fetch(offer);
    assert.ok(offerAcc.amount.eq(offerAmount));

    const makerBefore = await connection.getBalance(maker.publicKey);
    const treasuryBefore = await connection.getBalance(treasury);
    const buyerNftAta = ata(mint, buyer.publicKey);

    // Maker accepts the offer.
    await program.methods
      .acceptOffer()
      .accountsPartial({
        maker: maker.publicKey,
        buyer: buyer.publicKey,
        marketplace,
        treasury,
        makerMint: mint,
        buyerNftAta,
        vault: ata(mint, listing),
        rewardsMint,
        buyerRewardsAta: ata(rewardsMint, buyer.publicKey),
        listing,
        offer,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker])
      .rpc();

    const fee = offerAmount.muln(FEE_BPS).divn(10_000).toNumber();
    const sellerAmount = offerAmount.toNumber() - fee;

    // Buyer got the NFT; maker & treasury got the split of the offer amount.
    const nft = await getAccount(connection, buyerNftAta);
    assert.strictEqual(nft.amount.toString(), "1");
    const treasuryAfter = await connection.getBalance(treasury);
    assert.strictEqual(treasuryAfter - treasuryBefore, fee);
    const makerAfter = await connection.getBalance(maker.publicKey);
    // The seller proceeds drawn from the escrow are exactly `sellerAmount`. The
    // maker also signs this transaction, paying the network fee and the rent for
    // the buyer's freshly created NFT/rewards ATAs, so the net delta is
    // `sellerAmount` minus those small costs. Allow a 0.01 SOL buffer for them.
    assert.isAbove(makerAfter - makerBefore, sellerAmount - 10_000_000);

    const listingClosed = await program.account.listing.fetchNullable(listing);
    assert.isNull(listingClosed);
    const offerClosed = await program.account.offer.fetchNullable(offer);
    assert.isNull(offerClosed);
  });

  it("supports make_offer then cancel_offer with a SOL refund", async () => {
    const maker = await fundedKeypair();
    const buyer = await fundedKeypair();
    const { mint, ownerAta: makerAta } = await mintNft(maker);
    const listing = listingPda(mint);
    const offer = offerPda(mint, buyer.publicKey);
    const offerAmount = new BN(LAMPORTS_PER_SOL / 4);

    await program.methods
      .list(new BN(LAMPORTS_PER_SOL), null)
      .accountsPartial({
        maker: maker.publicKey,
        marketplace,
        makerMint: mint,
        makerAta,
        listing,
        vault: ata(mint, listing),
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker])
      .rpc();

    await program.methods
      .makeOffer(offerAmount)
      .accountsPartial({
        buyer: buyer.publicKey,
        marketplace,
        makerMint: mint,
        listing,
        offer,
        systemProgram: SystemProgram.programId,
      })
      .signers([buyer])
      .rpc();

    const buyerBefore = await connection.getBalance(buyer.publicKey);

    await program.methods
      .cancelOffer()
      .accountsPartial({
        buyer: buyer.publicKey,
        makerMint: mint,
        offer,
        systemProgram: SystemProgram.programId,
      })
      .signers([buyer])
      .rpc();

    // Offer is closed and the buyer was refunded the escrow plus rent.
    const offerClosed = await program.account.offer.fetchNullable(offer);
    assert.isNull(offerClosed);
    const buyerAfter = await connection.getBalance(buyer.publicKey);
    assert.isAtLeast(buyerAfter - buyerBefore, offerAmount.toNumber());
  });
});
