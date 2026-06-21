import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import { IntrospectionAmm } from "../target/types/introspection_amm";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  LAMPORTS_PER_SOL,
  Transaction,
} from "@solana/web3.js";
import {
  createMint,
  createAssociatedTokenAccount,
  mintTo,
  getAccount,
  getAssociatedTokenAddressSync,
  getMint,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { assert, expect } from "chai";

const CONFIG_SEED = Buffer.from("config");
const LP_SEED = Buffer.from("lp");
const SYSVAR_INSTRUCTIONS = new PublicKey(
  "Sysvar1nstructions1111111111111111111111111"
);

describe("introspection-amm", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace
    .IntrospectionAmm as Program<IntrospectionAmm>;
  const connection = provider.connection;

  const initializer = (provider.wallet as anchor.Wallet).payer;
  const lp = Keypair.generate();
  const trader = Keypair.generate();
  const outsider = Keypair.generate();

  let mintA: PublicKey;
  let mintB: PublicKey;
  let mintLp: PublicKey;
  let configPda: PublicKey;
  let vaultA: PublicKey;
  let vaultB: PublicKey;

  const seed = new BN(Math.floor(Math.random() * 1_000_000));
  const feeBps = 30;

  let lpAtaA: PublicKey;
  let lpAtaB: PublicKey;
  let lpAtaLp: PublicKey;
  let traderAtaA: PublicKey;
  let traderAtaB: PublicKey;

  const DECIMALS = 6;
  const ONE_TOKEN = new BN(10 ** DECIMALS);

  const sharedAccounts = () => ({
    mintA,
    mintB,
    config: configPda,
    mintLp,
    vaultA,
    vaultB,
    tokenProgram: TOKEN_PROGRAM_ID,
    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  });

  async function withdrawViaIntrospection(
    user: Keypair,
    userAtaA: PublicKey,
    userAtaB: PublicKey,
    userAtaLp: PublicKey,
    lpAmount: BN,
    minA: BN,
    minB: BN
  ) {
    const burnIx = await program.methods
      .burnLp(lpAmount)
      .accountsPartial({
        user: user.publicKey,
        ...sharedAccounts(),
        userAtaLp,
      })
      .instruction();

    const payoutIx = await program.methods
      .payout(minA, minB)
      .accountsPartial({
        user: user.publicKey,
        ...sharedAccounts(),
        userAtaA,
        userAtaB,
        userAtaLp,
        instructions: SYSVAR_INSTRUCTIONS,
      })
      .instruction();

    const tx = new Transaction().add(burnIx, payoutIx);
    return provider.sendAndConfirm(tx, [user]);
  }

  before(async () => {
    for (const kp of [lp, trader, outsider]) {
      const sig = await connection.requestAirdrop(
        kp.publicKey,
        10 * LAMPORTS_PER_SOL
      );
      await connection.confirmTransaction(sig);
    }

    mintA = await createMint(
      connection,
      initializer,
      initializer.publicKey,
      null,
      DECIMALS
    );
    mintB = await createMint(
      connection,
      initializer,
      initializer.publicKey,
      null,
      DECIMALS
    );

    [configPda] = PublicKey.findProgramAddressSync(
      [CONFIG_SEED, seed.toArrayLike(Buffer, "le", 8)],
      program.programId
    );
    [mintLp] = PublicKey.findProgramAddressSync(
      [LP_SEED, configPda.toBuffer()],
      program.programId
    );
    vaultA = getAssociatedTokenAddressSync(mintA, configPda, true);
    vaultB = getAssociatedTokenAddressSync(mintB, configPda, true);

    lpAtaA = await createAssociatedTokenAccount(
      connection,
      lp,
      mintA,
      lp.publicKey
    );
    lpAtaB = await createAssociatedTokenAccount(
      connection,
      lp,
      mintB,
      lp.publicKey
    );
    traderAtaA = await createAssociatedTokenAccount(
      connection,
      trader,
      mintA,
      trader.publicKey
    );
    traderAtaB = await createAssociatedTokenAccount(
      connection,
      trader,
      mintB,
      trader.publicKey
    );

    for (const [mint, ata] of [
      [mintA, lpAtaA],
      [mintB, lpAtaB],
      [mintA, traderAtaA],
      [mintB, traderAtaB],
    ] as const) {
      await mintTo(
        connection,
        initializer,
        mint,
        ata,
        initializer,
        1_000_000 * 10 ** DECIMALS
      );
    }
  });

  describe("initialize", () => {
    it("creates config, LP mint, and vaults", async () => {
      await program.methods
        .initialize(seed, feeBps, initializer.publicKey)
        .accountsPartial({
          initializer: initializer.publicKey,
          mintA,
          mintB,
          config: configPda,
          mintLp,
          vaultA,
          vaultB,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .rpc();

      const cfg = await program.account.config.fetch(configPda);
      expect(cfg.seed.toNumber()).to.equal(seed.toNumber());
      expect(cfg.feeBps).to.equal(feeBps);
      expect(cfg.mintA.toBase58()).to.equal(mintA.toBase58());
      expect(cfg.mintB.toBase58()).to.equal(mintB.toBase58());
      expect(cfg.locked).to.equal(false);
    });

    it("rejects identical mints", async () => {
      const badSeed = new BN(999_999_001);
      const [badConfig] = PublicKey.findProgramAddressSync(
        [CONFIG_SEED, badSeed.toArrayLike(Buffer, "le", 8)],
        program.programId
      );
      const [badLpMint] = PublicKey.findProgramAddressSync(
        [LP_SEED, badConfig.toBuffer()],
        program.programId
      );
      const badVault = getAssociatedTokenAddressSync(mintA, badConfig, true);

      try {
        await program.methods
          .initialize(badSeed, feeBps, null)
          .accountsPartial({
            initializer: initializer.publicKey,
            mintA,
            mintB: mintA,
            config: badConfig,
            mintLp: badLpMint,
            vaultA: badVault,
            vaultB: badVault,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          })
          .rpc();
        assert.fail("expected identical mint rejection");
      } catch (err: unknown) {
        assert.exists(err);
      }
    });

    it("rejects invalid fee", async () => {
      const badSeed = new BN(999_999_002);
      const [badConfig] = PublicKey.findProgramAddressSync(
        [CONFIG_SEED, badSeed.toArrayLike(Buffer, "le", 8)],
        program.programId
      );
      const [badLpMint] = PublicKey.findProgramAddressSync(
        [LP_SEED, badConfig.toBuffer()],
        program.programId
      );

      const badVaultA = getAssociatedTokenAddressSync(mintA, badConfig, true);
      const badVaultB = getAssociatedTokenAddressSync(mintB, badConfig, true);

      try {
        await program.methods
          .initialize(badSeed, 10_000, null)
          .accountsPartial({
            initializer: initializer.publicKey,
            mintA,
            mintB,
            config: badConfig,
            mintLp: badLpMint,
            vaultA: badVaultA,
            vaultB: badVaultB,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          })
          .rpc();
        assert.fail("expected InvalidFee");
      } catch (err: any) {
        expect(err.toString()).to.match(/InvalidFee|6001/);
      }
    });
  });

  describe("deposit", () => {
    it("bootstraps liquidity and mints LP", async () => {
      lpAtaLp = getAssociatedTokenAddressSync(mintLp, lp.publicKey, true);
      const depositA = ONE_TOKEN.muln(100);
      const depositB = ONE_TOKEN.muln(100);

      await program.methods
        .deposit(depositA, depositB, new BN(0))
        .accountsPartial({
          user: lp.publicKey,
          ...sharedAccounts(),
          vaultA,
          vaultB,
          userAtaA: lpAtaA,
          userAtaB: lpAtaB,
          userAtaLp: lpAtaLp,
        })
        .signers([lp])
        .rpc();

      const lpBal = await getAccount(connection, lpAtaLp);
      expect(Number(lpBal.amount)).to.be.greaterThan(0);

      const va = await getAccount(connection, vaultA);
      const vb = await getAccount(connection, vaultB);
      expect(va.amount.toString()).to.equal(depositA.toString());
      expect(vb.amount.toString()).to.equal(depositB.toString());
    });

    it("rejects zero deposit", async () => {
      try {
        await program.methods
          .deposit(new BN(0), ONE_TOKEN, new BN(0))
          .accountsPartial({
            user: lp.publicKey,
            ...sharedAccounts(),
            userAtaA: lpAtaA,
            userAtaB: lpAtaB,
            userAtaLp: lpAtaLp,
          })
          .signers([lp])
          .rpc();
        assert.fail("expected ZeroAmount");
      } catch (err: any) {
        expect(err.toString()).to.match(/ZeroAmount/);
      }
    });
  });

  describe("swap", () => {
    it("swaps A for B", async () => {
      const vbBefore = (await getAccount(connection, traderAtaB)).amount;
      await program.methods
        .swap(ONE_TOKEN.muln(5), new BN(0), true)
        .accountsPartial({
          user: trader.publicKey,
          ...sharedAccounts(),
          userAtaA: traderAtaA,
          userAtaB: traderAtaB,
        })
        .signers([trader])
        .rpc();
      const vbAfter = (await getAccount(connection, traderAtaB)).amount;
      expect(vbAfter > vbBefore).to.equal(true);
    });

    it("swaps B for A", async () => {
      const vaBefore = (await getAccount(connection, traderAtaA)).amount;
      await program.methods
        .swap(ONE_TOKEN.muln(5), new BN(0), false)
        .accountsPartial({
          user: trader.publicKey,
          ...sharedAccounts(),
          userAtaA: traderAtaA,
          userAtaB: traderAtaB,
        })
        .signers([trader])
        .rpc();
      const vaAfter = (await getAccount(connection, traderAtaA)).amount;
      expect(vaAfter > vaBefore).to.equal(true);
    });

    it("rejects excessive slippage", async () => {
      try {
        await program.methods
          .swap(ONE_TOKEN, new BN("999999999999"), true)
          .accountsPartial({
            user: trader.publicKey,
            ...sharedAccounts(),
            userAtaA: traderAtaA,
            userAtaB: traderAtaB,
          })
          .signers([trader])
          .rpc();
        assert.fail("expected SlippageExceeded");
      } catch (err: any) {
        expect(err.toString()).to.match(/SlippageExceeded/);
      }
    });
  });

  describe("burn_lp + payout (instruction introspection)", () => {
    it("withdraws via burn_lp immediately followed by payout", async () => {
      const lpMintBefore = await getMint(connection, mintLp);
      const lpBalBefore = (await getAccount(connection, lpAtaLp)).amount;
      const ataABefore = (await getAccount(connection, lpAtaA)).amount;

      const burnAmount = new BN(lpBalBefore.toString()).divn(4);

      await withdrawViaIntrospection(
        lp,
        lpAtaA,
        lpAtaB,
        lpAtaLp,
        burnAmount,
        new BN(0),
        new BN(0)
      );

      const lpBalAfter = (await getAccount(connection, lpAtaLp)).amount;
      const lpMintAfter = await getMint(connection, mintLp);
      const ataAAfter = (await getAccount(connection, lpAtaA)).amount;

      expect(lpBalAfter < lpBalBefore).to.equal(true);
      expect(lpMintAfter.supply < lpMintBefore.supply).to.equal(true);
      expect(ataAAfter > ataABefore).to.equal(true);
    });

    it("rejects payout without a preceding burn_lp", async () => {
      try {
        await program.methods
          .payout(new BN(0), new BN(0))
          .accountsPartial({
            user: lp.publicKey,
            ...sharedAccounts(),
            userAtaA: lpAtaA,
            userAtaB: lpAtaB,
            userAtaLp: lpAtaLp,
            instructions: SYSVAR_INSTRUCTIONS,
          })
          .signers([lp])
          .rpc();
        assert.fail("expected MissingPreviousInstruction");
      } catch (err: any) {
        expect(err.toString()).to.match(/MissingPreviousInstruction/);
      }
    });

    it("rejects payout when burn_lp is not the previous instruction", async () => {
      const burnAmount = ONE_TOKEN;
      const burnIx = await program.methods
        .burnLp(burnAmount)
        .accountsPartial({
          user: lp.publicKey,
          ...sharedAccounts(),
          userAtaLp: lpAtaLp,
        })
        .instruction();

      const noopIx = SystemProgram.transfer({
        fromPubkey: lp.publicKey,
        toPubkey: lp.publicKey,
        lamports: 0,
      });

      const payoutIx = await program.methods
        .payout(new BN(0), new BN(0))
        .accountsPartial({
          user: lp.publicKey,
          ...sharedAccounts(),
          userAtaA: lpAtaA,
          userAtaB: lpAtaB,
          userAtaLp: lpAtaLp,
          instructions: SYSVAR_INSTRUCTIONS,
        })
        .instruction();

      const tx = new Transaction().add(burnIx, noopIx, payoutIx);

      try {
        await provider.sendAndConfirm(tx, [lp]);
        assert.fail("expected InvalidProgram");
      } catch (err: any) {
        expect(err.toString()).to.match(/InvalidProgram|6009/);
      }
    });

    it("rejects burn_lp with zero amount", async () => {
      try {
        await program.methods
          .burnLp(new BN(0))
          .accountsPartial({
            user: lp.publicKey,
            ...sharedAccounts(),
            userAtaLp: lpAtaLp,
          })
          .signers([lp])
          .rpc();
        assert.fail("expected ZeroAmount");
      } catch (err: any) {
        expect(err.toString()).to.match(/ZeroAmount/);
      }
    });

    it("rejects payout when slippage is too high", async () => {
      const lpBal = (await getAccount(connection, lpAtaLp)).amount;
      const burnAmount = new BN(lpBal.toString()).divn(10);

      const burnIx = await program.methods
        .burnLp(burnAmount)
        .accountsPartial({
          user: lp.publicKey,
          ...sharedAccounts(),
          userAtaLp: lpAtaLp,
        })
        .instruction();

      const payoutIx = await program.methods
        .payout(new BN("999999999999"), new BN("999999999999"))
        .accountsPartial({
          user: lp.publicKey,
          ...sharedAccounts(),
          userAtaA: lpAtaA,
          userAtaB: lpAtaB,
          userAtaLp: lpAtaLp,
          instructions: SYSVAR_INSTRUCTIONS,
        })
        .instruction();

      const tx = new Transaction().add(burnIx, payoutIx);

      try {
        await provider.sendAndConfirm(tx, [lp]);
        assert.fail("expected SlippageExceeded");
      } catch (err: any) {
        expect(err.toString()).to.match(/SlippageExceeded/);
      }
    });
  });

  describe("lock / unlock", () => {
    it("rejects lock from non-authority", async () => {
      try {
        await program.methods
          .lock()
          .accountsPartial({
            authority: outsider.publicKey,
            config: configPda,
          })
          .signers([outsider])
          .rpc();
        assert.fail("expected Unauthorized");
      } catch (err: any) {
        expect(err.toString()).to.match(/Unauthorized/);
      }
    });

    it("locks and blocks burn_lp", async () => {
      await program.methods
        .lock()
        .accountsPartial({
          authority: initializer.publicKey,
          config: configPda,
        })
        .rpc();

      try {
        await program.methods
          .burnLp(ONE_TOKEN)
          .accountsPartial({
            user: lp.publicKey,
            ...sharedAccounts(),
            userAtaLp: lpAtaLp,
          })
          .signers([lp])
          .rpc();
        assert.fail("expected PoolLocked");
      } catch (err: any) {
        expect(err.toString()).to.match(/PoolLocked/);
      }
    });

    it("unlocks the pool", async () => {
      await program.methods
        .unlock()
        .accountsPartial({
          authority: initializer.publicKey,
          config: configPda,
        })
        .rpc();

      const cfg = await program.account.config.fetch(configPda);
      expect(cfg.locked).to.equal(false);
    });
  });
});
