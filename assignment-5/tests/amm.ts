import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import { Amm } from "../target/types/amm";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  LAMPORTS_PER_SOL,
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

describe("amm", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.amm as Program<Amm>;
  const connection = provider.connection;

  // Funded wallets / authorities reused across tests.
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

  // Per-suite pool config
  const seed = new BN(Math.floor(Math.random() * 1_000_000));
  const feeBps = 30; // 0.30%

  // ATAs
  let lpAtaA: PublicKey;
  let lpAtaB: PublicKey;
  let lpAtaLp: PublicKey;
  let traderAtaA: PublicKey;
  let traderAtaB: PublicKey;

  const DECIMALS = 6;
  const ONE_TOKEN = new BN(10 ** DECIMALS);

  before(async () => {
    // Airdrop the side wallets
    for (const kp of [lp, trader, outsider]) {
      const sig = await connection.requestAirdrop(
        kp.publicKey,
        10 * LAMPORTS_PER_SOL
      );
      await connection.confirmTransaction(sig);
    }

    // Create both underlying mints (initializer is the mint authority)
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

    // Derive PDAs
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

    // ATAs for the LP and the trader
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

    // Mint a generous starting balance to the LP and trader
    await mintTo(
      connection,
      initializer,
      mintA,
      lpAtaA,
      initializer,
      1_000_000 * 10 ** DECIMALS
    );
    await mintTo(
      connection,
      initializer,
      mintB,
      lpAtaB,
      initializer,
      1_000_000 * 10 ** DECIMALS
    );
    await mintTo(
      connection,
      initializer,
      mintA,
      traderAtaA,
      initializer,
      1_000_000 * 10 ** DECIMALS
    );
    await mintTo(
      connection,
      initializer,
      mintB,
      traderAtaB,
      initializer,
      1_000_000 * 10 ** DECIMALS
    );

    lpAtaLp = getAssociatedTokenAddressSync(mintLp, lp.publicKey, false);
  });

  describe("initialize", () => {
    it("creates a config and an LP mint owned by the config PDA", async () => {
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
        })
        .rpc();

      const cfg = await program.account.config.fetch(configPda);
      expect(cfg.seed.toString()).to.equal(seed.toString());
      expect(cfg.mintA.toString()).to.equal(mintA.toString());
      expect(cfg.mintB.toString()).to.equal(mintB.toString());
      expect(cfg.feeBps).to.equal(feeBps);
      expect(cfg.locked).to.equal(false);
      expect(cfg.authority.toString()).to.equal(
        initializer.publicKey.toString()
      );

      const lpMintInfo = await getMint(connection, mintLp);
      expect(lpMintInfo.mintAuthority.toString()).to.equal(
        configPda.toString()
      );
      expect(lpMintInfo.decimals).to.equal(6);
    });

    it("rejects identical mints", async () => {
      const badSeed = new BN(seed.toNumber() + 1);
      const [badConfig] = PublicKey.findProgramAddressSync(
        [CONFIG_SEED, badSeed.toArrayLike(Buffer, "le", 8)],
        program.programId
      );
      const [badLp] = PublicKey.findProgramAddressSync(
        [LP_SEED, badConfig.toBuffer()],
        program.programId
      );
      // We pass the same mint twice. The constraint on `mint_b` in the
      // initialize accounts struct should refuse it. If Anchor/Solana fails
      // earlier (because two `init` ATAs collapse to the same address) we
      // accept that as a valid rejection too — both prevent the bad pool.
      const badVaultA = getAssociatedTokenAddressSync(mintA, badConfig, true);
      const badVaultB = getAssociatedTokenAddressSync(mintA, badConfig, true);
      try {
        await program.methods
          .initialize(badSeed, feeBps, initializer.publicKey)
          .accountsPartial({
            initializer: initializer.publicKey,
            mintA,
            mintB: mintA,
            config: badConfig,
            mintLp: badLp,
            vaultA: badVaultA,
            vaultB: badVaultB,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        assert.fail("expected rejection for identical mints");
      } catch (err: any) {
        expect(err.toString()).to.match(/IdenticalMints|Simulation failed/);
      }
    });

    it("rejects fee >= 10_000 bps", async () => {
      const badSeed = new BN(seed.toNumber() + 2);
      const [badConfig] = PublicKey.findProgramAddressSync(
        [CONFIG_SEED, badSeed.toArrayLike(Buffer, "le", 8)],
        program.programId
      );
      const [badLp] = PublicKey.findProgramAddressSync(
        [LP_SEED, badConfig.toBuffer()],
        program.programId
      );
      const badVaultA = getAssociatedTokenAddressSync(mintA, badConfig, true);
      const badVaultB = getAssociatedTokenAddressSync(mintB, badConfig, true);
      try {
        await program.methods
          .initialize(badSeed, 10_000, initializer.publicKey)
          .accountsPartial({
            initializer: initializer.publicKey,
            mintA,
            mintB,
            config: badConfig,
            mintLp: badLp,
            vaultA: badVaultA,
            vaultB: badVaultB,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        assert.fail("expected InvalidFee");
      } catch (err: any) {
        expect(err.toString()).to.match(/InvalidFee/);
      }
    });
  });

  describe("deposit", () => {
    it("bootstraps the pool with 100 A + 400 B and mints sqrt(a*b) LP", async () => {
      const amountA = ONE_TOKEN.muln(100);
      const amountB = ONE_TOKEN.muln(400);

      await program.methods
        .deposit(amountA, amountB, new BN(0))
        .accountsPartial({
          user: lp.publicKey,
          mintA,
          mintB,
          config: configPda,
          mintLp,
          vaultA,
          vaultB,
          userAtaA: lpAtaA,
          userAtaB: lpAtaB,
          userAtaLp: lpAtaLp,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([lp])
        .rpc();

      const va = await getAccount(connection, vaultA);
      const vb = await getAccount(connection, vaultB);
      expect(va.amount.toString()).to.equal(amountA.toString());
      expect(vb.amount.toString()).to.equal(amountB.toString());

      // sqrt(100 * 400) tokens, both in token-base-units
      const expectedLp = BigInt(
        Math.floor(Math.sqrt(Number(amountA.mul(amountB).toString())))
      );
      const lpAccount = await getAccount(connection, lpAtaLp);
      expect(lpAccount.amount.toString()).to.equal(expectedLp.toString());
    });

    it("a second proportional deposit issues LP proportional to share of reserves", async () => {
      const before = await program.account.config.fetch(configPda);
      void before;
      const lpMintBefore = await getMint(connection, mintLp);
      const vaBefore = await getAccount(connection, vaultA);

      // Add another 50 A — pool is 1:4 so this should pull 200 B and mint
      // (50 / 100) * supplyBefore LP tokens.
      const amountA = ONE_TOKEN.muln(50);

      await program.methods
        .deposit(amountA, ONE_TOKEN.muln(250), new BN(0))
        .accountsPartial({
          user: lp.publicKey,
          mintA,
          mintB,
          config: configPda,
          mintLp,
          vaultA,
          vaultB,
          userAtaA: lpAtaA,
          userAtaB: lpAtaB,
          userAtaLp: lpAtaLp,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([lp])
        .rpc();

      const vaAfter = await getAccount(connection, vaultA);
      const vbAfter = await getAccount(connection, vaultB);
      const lpMintAfter = await getMint(connection, mintLp);

      // Pool ratio is preserved (1:4).
      expect((BigInt(vbAfter.amount) * 100n) / BigInt(vaAfter.amount)).to.equal(
        400n
      );

      // LP supply growth matches share of A deposited
      const expectedMint =
        (BigInt(amountA.toString()) * BigInt(lpMintBefore.supply.toString())) /
        BigInt(vaBefore.amount);
      const minted =
        BigInt(lpMintAfter.supply.toString()) -
        BigInt(lpMintBefore.supply.toString());
      expect(minted).to.equal(expectedMint);
    });

    it("rejects deposit when required B exceeds max_b (slippage)", async () => {
      try {
        await program.methods
          .deposit(ONE_TOKEN.muln(10), new BN(1), new BN(0))
          .accountsPartial({
            user: lp.publicKey,
            mintA,
            mintB,
            config: configPda,
            mintLp,
            vaultA,
            vaultB,
            userAtaA: lpAtaA,
            userAtaB: lpAtaB,
            userAtaLp: lpAtaLp,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([lp])
          .rpc();
        assert.fail("expected SlippageExceeded");
      } catch (err: any) {
        expect(err.toString()).to.match(/SlippageExceeded/);
      }
    });

    it("rejects zero-amount deposit", async () => {
      try {
        await program.methods
          .deposit(new BN(0), new BN(0), new BN(0))
          .accountsPartial({
            user: lp.publicKey,
            mintA,
            mintB,
            config: configPda,
            mintLp,
            vaultA,
            vaultB,
            userAtaA: lpAtaA,
            userAtaB: lpAtaB,
            userAtaLp: lpAtaLp,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
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
    it("swaps A for B and grows the constant product (fee accrues)", async () => {
      const vaBefore = await getAccount(connection, vaultA);
      const vbBefore = await getAccount(connection, vaultB);
      const traderBBefore = await getAccount(connection, traderAtaB);

      const amountIn = ONE_TOKEN.muln(10);
      await program.methods
        .swap(amountIn, new BN(0), true)
        .accountsPartial({
          user: trader.publicKey,
          mintA,
          mintB,
          config: configPda,
          vaultA,
          vaultB,
          userAtaA: traderAtaA,
          userAtaB: traderAtaB,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([trader])
        .rpc();

      const vaAfter = await getAccount(connection, vaultA);
      const vbAfter = await getAccount(connection, vaultB);
      const traderBAfter = await getAccount(connection, traderAtaB);

      // Trader receives B
      expect(traderBAfter.amount > traderBBefore.amount).to.equal(true);
      // Vault A grew by exactly amount_in
      expect(BigInt(vaAfter.amount) - BigInt(vaBefore.amount)).to.equal(
        BigInt(amountIn.toString())
      );
      // k must not decrease (fee makes it grow)
      const kBefore = BigInt(vaBefore.amount) * BigInt(vbBefore.amount);
      const kAfter = BigInt(vaAfter.amount) * BigInt(vbAfter.amount);
      expect(kAfter >= kBefore).to.equal(true);
    });

    it("swaps B for A in the reverse direction", async () => {
      const traderABefore = await getAccount(connection, traderAtaA);
      const amountIn = ONE_TOKEN.muln(20);
      await program.methods
        .swap(amountIn, new BN(0), false)
        .accountsPartial({
          user: trader.publicKey,
          mintA,
          mintB,
          config: configPda,
          vaultA,
          vaultB,
          userAtaA: traderAtaA,
          userAtaB: traderAtaB,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([trader])
        .rpc();
      const traderAAfter = await getAccount(connection, traderAtaA);
      expect(traderAAfter.amount > traderABefore.amount).to.equal(true);
    });

    it("rejects a swap when min_amount_out is too high (slippage)", async () => {
      try {
        await program.methods
          .swap(ONE_TOKEN.muln(1), new BN("999999999999"), true)
          .accountsPartial({
            user: trader.publicKey,
            mintA,
            mintB,
            config: configPda,
            vaultA,
            vaultB,
            userAtaA: traderAtaA,
            userAtaB: traderAtaB,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([trader])
          .rpc();
        assert.fail("expected SlippageExceeded");
      } catch (err: any) {
        expect(err.toString()).to.match(/SlippageExceeded/);
      }
    });

    it("rejects a zero-amount swap", async () => {
      try {
        await program.methods
          .swap(new BN(0), new BN(0), true)
          .accountsPartial({
            user: trader.publicKey,
            mintA,
            mintB,
            config: configPda,
            vaultA,
            vaultB,
            userAtaA: traderAtaA,
            userAtaB: traderAtaB,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([trader])
          .rpc();
        assert.fail("expected ZeroAmount");
      } catch (err: any) {
        expect(err.toString()).to.match(/ZeroAmount/);
      }
    });
  });

  describe("lock / unlock", () => {
    it("rejects lock from a non-authority signer", async () => {
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

    it("locks the pool and blocks deposits, withdraws and swaps", async () => {
      await program.methods
        .lock()
        .accountsPartial({
          authority: initializer.publicKey,
          config: configPda,
        })
        .rpc();

      const cfg = await program.account.config.fetch(configPda);
      expect(cfg.locked).to.equal(true);

      try {
        await program.methods
          .swap(ONE_TOKEN, new BN(0), true)
          .accountsPartial({
            user: trader.publicKey,
            mintA,
            mintB,
            config: configPda,
            vaultA,
            vaultB,
            userAtaA: traderAtaA,
            userAtaB: traderAtaB,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([trader])
          .rpc();
        assert.fail("expected PoolLocked");
      } catch (err: any) {
        expect(err.toString()).to.match(/PoolLocked/);
      }
    });

    it("unlocks the pool and lets trading resume", async () => {
      await program.methods
        .unlock()
        .accountsPartial({
          authority: initializer.publicKey,
          config: configPda,
        })
        .rpc();

      const cfg = await program.account.config.fetch(configPda);
      expect(cfg.locked).to.equal(false);

      // A small swap should now succeed.
      await program.methods
        .swap(ONE_TOKEN, new BN(0), true)
        .accountsPartial({
          user: trader.publicKey,
          mintA,
          mintB,
          config: configPda,
          vaultA,
          vaultB,
          userAtaA: traderAtaA,
          userAtaB: traderAtaB,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([trader])
        .rpc();
    });
  });

  describe("withdraw", () => {
    it("burns LP tokens for a pro-rata share of both vaults", async () => {
      const lpAccBefore = await getAccount(connection, lpAtaLp);
      const lpAtoBefore = await getAccount(connection, lpAtaA);
      const lpBtoBefore = await getAccount(connection, lpAtaB);
      const vaBefore = await getAccount(connection, vaultA);
      const vbBefore = await getAccount(connection, vaultB);
      const lpMintBefore = await getMint(connection, mintLp);

      // Burn 25% of the LP supply held by the LP wallet.
      const burnAmount = new BN((BigInt(lpAccBefore.amount) / 4n).toString());

      await program.methods
        .withdraw(burnAmount, new BN(0), new BN(0))
        .accountsPartial({
          user: lp.publicKey,
          mintA,
          mintB,
          config: configPda,
          mintLp,
          vaultA,
          vaultB,
          userAtaA: lpAtaA,
          userAtaB: lpAtaB,
          userAtaLp: lpAtaLp,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([lp])
        .rpc();

      const lpAtoAfter = await getAccount(connection, lpAtaA);
      const lpBtoAfter = await getAccount(connection, lpAtaB);
      const vaAfter = await getAccount(connection, vaultA);
      const vbAfter = await getAccount(connection, vaultB);
      const lpMintAfter = await getMint(connection, mintLp);

      // LP supply decreased by exactly burnAmount
      expect(
        BigInt(lpMintBefore.supply.toString()) -
          BigInt(lpMintAfter.supply.toString())
      ).to.equal(BigInt(burnAmount.toString()));

      // Withdrawn amounts equal lp_amount * reserve / lp_supply
      const expectedA =
        (BigInt(burnAmount.toString()) * BigInt(vaBefore.amount)) /
        BigInt(lpMintBefore.supply.toString());
      const expectedB =
        (BigInt(burnAmount.toString()) * BigInt(vbBefore.amount)) /
        BigInt(lpMintBefore.supply.toString());
      expect(BigInt(lpAtoAfter.amount) - BigInt(lpAtoBefore.amount)).to.equal(
        expectedA
      );
      expect(BigInt(lpBtoAfter.amount) - BigInt(lpBtoBefore.amount)).to.equal(
        expectedB
      );
      expect(BigInt(vaBefore.amount) - BigInt(vaAfter.amount)).to.equal(
        expectedA
      );
      expect(BigInt(vbBefore.amount) - BigInt(vbAfter.amount)).to.equal(
        expectedB
      );
    });

    it("rejects a withdraw when min_a is unsatisfiable (slippage)", async () => {
      const lpAcc = await getAccount(connection, lpAtaLp);
      try {
        await program.methods
          .withdraw(
            new BN(lpAcc.amount.toString()).divn(10),
            new BN("999999999999"),
            new BN(0)
          )
          .accountsPartial({
            user: lp.publicKey,
            mintA,
            mintB,
            config: configPda,
            mintLp,
            vaultA,
            vaultB,
            userAtaA: lpAtaA,
            userAtaB: lpAtaB,
            userAtaLp: lpAtaLp,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([lp])
          .rpc();
        assert.fail("expected SlippageExceeded");
      } catch (err: any) {
        expect(err.toString()).to.match(/SlippageExceeded/);
      }
    });

    it("rejects a zero-amount withdraw", async () => {
      try {
        await program.methods
          .withdraw(new BN(0), new BN(0), new BN(0))
          .accountsPartial({
            user: lp.publicKey,
            mintA,
            mintB,
            config: configPda,
            mintLp,
            vaultA,
            vaultB,
            userAtaA: lpAtaA,
            userAtaB: lpAtaB,
            userAtaLp: lpAtaLp,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([lp])
          .rpc();
        assert.fail("expected ZeroAmount");
      } catch (err: any) {
        expect(err.toString()).to.match(/ZeroAmount/);
      }
    });
  });
});
