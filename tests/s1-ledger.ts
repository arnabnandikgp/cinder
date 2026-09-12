import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccount,
  createMint,
  getAccount,
  getAssociatedTokenAddressSync,
  mintTo,
} from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { expect } from "chai";
import { CinderVault } from "../target/types/cinder_vault";
import { CinderLedger } from "../target/types/cinder_ledger";

const ER_VALIDATOR = new PublicKey(
  "mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev"
);
const ASSET_SOL = 1;
const HALT_ENTRIES = 1 << 0;
const CREDIT = 100_000_000; // 100 USDC
const LOTS = 10;
// stub IM: 10 lots * 1e6 * 12500 / (10 * 10000) = 1_250_000
const IM_TEN_LOTS = 1_250_000;

function pda(programId: PublicKey, seeds: (Buffer | Uint8Array)[]): PublicKey {
  return PublicKey.findProgramAddressSync(seeds, programId)[0];
}

function oid(tag: number): number[] {
  return Array.from({ length: 16 }, (_, i) => (i + tag) % 256);
}

describe("S1 accounts and order machine", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const vault = (anchor.workspace as any).cinderVault as Program<CinderVault>;
  const ledger = (anchor.workspace as any).cinderLedger as Program<CinderLedger>;
  const payer = (provider.wallet as anchor.Wallet).payer;
  const connection = provider.connection;

  const adapter = Keypair.generate();
  const user = Keypair.generate();
  const phoenixTrader = Keypair.generate().publicKey;

  let usdcMint: PublicKey;
  let configPda: PublicKey;
  let vaultAuth: PublicKey;
  let reservePda: PublicKey;
  let vaultAta: PublicKey;
  let bookPda: PublicKey;
  let feesPda: PublicKey;
  let userLedgerPda: PublicKey;

  async function airdrop(pk: PublicKey) {
    const sig = await connection.requestAirdrop(pk, 2 * anchor.web3.LAMPORTS_PER_SOL);
    await connection.confirmTransaction(sig, "confirmed");
  }

  before(async () => {
    await airdrop(adapter.publicKey);
    await airdrop(user.publicKey);

    usdcMint = await createMint(connection, payer, payer.publicKey, null, 6);

    configPda = pda(vault.programId, [Buffer.from("config")]);
    vaultAuth = pda(vault.programId, [Buffer.from("vault-authority")]);
    reservePda = pda(vault.programId, [Buffer.from("reserve")]);
    bookPda = pda(ledger.programId, [Buffer.from("book")]);
    feesPda = pda(ledger.programId, [Buffer.from("fees")]);
    userLedgerPda = pda(ledger.programId, [
      Buffer.from("user"),
      user.publicKey.toBuffer(),
    ]);
    vaultAta = getAssociatedTokenAddressSync(usdcMint, vaultAuth, true);
  });

  it("inits Config, UserLedger, Book, FeeAccrual; seeds match freeze", async () => {
    await vault.methods
      .initialize(adapter.publicKey, vaultAuth, phoenixTrader, ER_VALIDATOR)
      .accounts({
        admin: payer.publicKey,
        config: configPda,
        vaultAuthority: vaultAuth,
        reserveRoot: reservePda,
        usdcMint,
        vaultUsdcAta: vaultAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const config = await vault.account.config.fetch(configPda);
    expect(config.admin.toBase58()).to.equal(payer.publicKey.toBase58());
    expect(config.adapter.toBase58()).to.equal(adapter.publicKey.toBase58());
    expect(config.vaultAuthority.toBase58()).to.equal(vaultAuth.toBase58());
    expect(config.phoenixTrader.toBase58()).to.equal(phoenixTrader.toBase58());
    expect(config.usdcMint.toBase58()).to.equal(usdcMint.toBase58());
    expect(config.vaultUsdcAta.toBase58()).to.equal(vaultAta.toBase58());
    expect(config.erValidator.toBase58()).to.equal(ER_VALIDATOR.toBase58());
    expect(config.paused).to.equal(0);
    expect(config.userImMultBps).to.equal(12_500);
    expect(config.userMmMultBps).to.equal(12_500);
    expect(config.maxUserLeverage).to.equal(10);
    expect(config.bufferMinBps).to.equal(2_000);
    expect(config.bufferFloorUsdc.toNumber()).to.equal(50_000_000);
    expect(config.allowlistLen).to.equal(0);

    const root = await vault.account.reserveRoot.fetch(reservePda);
    expect(root.epoch.toNumber()).to.equal(0);

    await ledger.methods
      .initialize()
      .accounts({
        adapter: adapter.publicKey,
        config: configPda,
        book: bookPda,
        feeAccrual: feesPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([adapter])
      .rpc();

    const book = await ledger.account.book.fetch(bookPda);
    expect(book.residualLen).to.equal(0);
    expect(book.invariantOk).to.equal(1);
    expect(book.halt).to.equal(0);
    expect(book.phoenixCollateral.toNumber()).to.equal(0);

    const fees = await ledger.account.feeAccrual.fetch(feesPda);
    expect(fees.phoenixFeesPaid.toNumber()).to.equal(0);
    expect(fees.cinderFeesAccrued.toNumber()).to.equal(0);

    await vault.methods
      .setAllowlist([ASSET_SOL])
      .accounts({ admin: payer.publicKey, config: configPda })
      .rpc();

    await ledger.methods
      .initUser()
      .accounts({
        adapter: adapter.publicKey,
        user: user.publicKey,
        config: configPda,
        userLedger: userLedgerPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([adapter, user])
      .rpc();

    const ul = await ledger.account.userLedger.fetch(userLedgerPda);
    expect(ul.user.toBase58()).to.equal(user.publicKey.toBase58());
    expect(ul.free.toNumber()).to.equal(0);
    expect(ul.reserved.toNumber()).to.equal(0);
    expect(ul.nonce.toNumber()).to.equal(0);
    expect(ul.positionsLen).to.equal(0);
    expect(ul.pendingOidCount).to.equal(0);

    await ledger.methods
      .creditDeposit(new BN(CREDIT))
      .accounts({
        adapter: adapter.publicKey,
        config: configPda,
        book: bookPda,
        userLedger: userLedgerPda,
      })
      .signers([adapter])
      .rpc();

    const credited = await ledger.account.userLedger.fetch(userLedgerPda);
    expect(credited.free.toNumber()).to.equal(CREDIT);
  });

  it("place +10 lots then fail-ack restores free and lots", async () => {
    await ledger.methods
      .placeOrder(ASSET_SOL, new BN(LOTS), oid(1), 50, false, new BN(0))
      .accounts({
        user: user.publicKey,
        config: configPda,
        book: bookPda,
        userLedger: userLedgerPda,
      })
      .signers([user])
      .rpc();

    let ul = await ledger.account.userLedger.fetch(userLedgerPda);
    expect(ul.free.toNumber()).to.equal(CREDIT - IM_TEN_LOTS);
    expect(ul.reserved.toNumber()).to.equal(IM_TEN_LOTS);
    expect(ul.positionsLen).to.equal(1);
    expect(ul.positions[0].lots.toNumber()).to.equal(LOTS);
    expect(ul.positions[0].reservedIm.toNumber()).to.equal(IM_TEN_LOTS);
    expect(ul.pendingOidCount).to.equal(1);
    expect(ul.nonce.toNumber()).to.equal(1);

    const bookBefore = await ledger.account.book.fetch(bookPda);
    expect(bookBefore.residualLen).to.equal(0);

    await ledger.methods
      .ackPhoenixFail(oid(1))
      .accounts({
        adapter: adapter.publicKey,
        config: configPda,
        userLedger: userLedgerPda,
      })
      .signers([adapter])
      .rpc();

    ul = await ledger.account.userLedger.fetch(userLedgerPda);
    expect(ul.free.toNumber()).to.equal(CREDIT);
    expect(ul.reserved.toNumber()).to.equal(0);
    expect(ul.positionsLen).to.equal(0);
    expect(ul.pendingOidCount).to.equal(0);

    const bookAfter = await ledger.account.book.fetch(bookPda);
    expect(bookAfter.residualLen).to.equal(0);
  });

  it("place then fill-ack updates position, reserved IM, and Book", async () => {
    await ledger.methods
      .placeOrder(ASSET_SOL, new BN(LOTS), oid(2), 50, false, new BN(1))
      .accounts({
        user: user.publicKey,
        config: configPda,
        book: bookPda,
        userLedger: userLedgerPda,
      })
      .signers([user])
      .rpc();

    await ledger.methods
      .ackPhoenixFill(oid(2), new BN(LOTS), new BN(0), new BN(0))
      .accounts({
        adapter: adapter.publicKey,
        config: configPda,
        userLedger: userLedgerPda,
        book: bookPda,
        feeAccrual: feesPda,
      })
      .signers([adapter])
      .rpc();

    const ul = await ledger.account.userLedger.fetch(userLedgerPda);
    expect(ul.free.toNumber()).to.equal(CREDIT - IM_TEN_LOTS);
    expect(ul.reserved.toNumber()).to.equal(IM_TEN_LOTS);
    expect(ul.positionsLen).to.equal(1);
    expect(ul.positions[0].assetId).to.equal(ASSET_SOL);
    expect(ul.positions[0].lots.toNumber()).to.equal(LOTS);
    expect(ul.positions[0].reservedIm.toNumber()).to.equal(IM_TEN_LOTS);
    expect(ul.pendingOidCount).to.equal(0);
    expect(ul.openOids[0].state).to.equal(1); // acked

    const book = await ledger.account.book.fetch(bookPda);
    expect(book.residualLen).to.equal(1);
    expect(book.residuals[0].assetId).to.equal(ASSET_SOL);
    expect(book.residuals[0].lots.toNumber()).to.equal(LOTS);
  });

  it("HALT_ENTRIES blocks place_order", async () => {
    await ledger.methods
      .setBookHalt(HALT_ENTRIES)
      .accounts({
        adapter: adapter.publicKey,
        config: configPda,
        book: bookPda,
      })
      .signers([adapter])
      .rpc();

    try {
      await ledger.methods
        .placeOrder(ASSET_SOL, new BN(LOTS), oid(3), 50, false, new BN(2))
        .accounts({
          user: user.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: userLedgerPda,
        })
        .signers([user])
        .rpc();
      expect.fail("place_order should fail when HALT_ENTRIES is set");
    } catch (e: any) {
      const code = e.error?.errorCode?.code ?? e.toString();
      expect(code).to.match(/Halted/);
    }

    await ledger.methods
      .setBookHalt(0)
      .accounts({
        adapter: adapter.publicKey,
        config: configPda,
        book: bookPda,
      })
      .signers([adapter])
      .rpc();
  });

  it("escape_withdraw returns unsupported", async () => {
    try {
      await vault.methods
        .escapeWithdraw()
        .accounts({ user: user.publicKey })
        .signers([user])
        .rpc();
      expect.fail("escape_withdraw should be unsupported");
    } catch (e: any) {
      const code = e.error?.errorCode?.code ?? e.toString();
      expect(code).to.match(/Unsupported/);
    }
  });

  describe("S3 order machine", () => {
    const placeAccounts = () => ({
      user: user.publicKey,
      config: configPda,
      book: bookPda,
      userLedger: userLedgerPda,
    });

    it("replay nonce fails", async () => {
      try {
        await ledger.methods
          .placeOrder(ASSET_SOL, new BN(1), oid(4), 50, false, new BN(0))
          .accounts(placeAccounts())
          .signers([user])
          .rpc();
        expect.fail("replayed nonce should fail");
      } catch (e: any) {
        const code = e.error?.errorCode?.code ?? e.toString();
        expect(code).to.match(/BadNonce/);
      }
    });

    it("reduce-only that would increase exposure fails", async () => {
      try {
        await ledger.methods
          .placeOrder(ASSET_SOL, new BN(1), oid(5), 50, true, new BN(2))
          .accounts(placeAccounts())
          .signers([user])
          .rpc();
        expect.fail("reduce-only increase should fail");
      } catch (e: any) {
        const code = e.error?.errorCode?.code ?? e.toString();
        expect(code).to.match(/ReduceOnlyIncrease/);
      }
    });

    it("liquidate_user is adapter-signed and flattens one asset", async () => {
      try {
        await ledger.methods
          .liquidateUser(ASSET_SOL)
          .accounts({
            adapter: user.publicKey,
            config: configPda,
            userLedger: userLedgerPda,
            book: bookPda,
          })
          .signers([user])
          .rpc();
        expect.fail("user must not be able to liquidate");
      } catch (e: any) {
        const code = e.error?.errorCode?.code ?? e.toString();
        expect(code).to.match(/Unauthorized/);
      }

      await ledger.methods
        .liquidateUser(ASSET_SOL)
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: userLedgerPda,
          book: bookPda,
        })
        .signers([adapter])
        .rpc();

      const ul = await ledger.account.userLedger.fetch(userLedgerPda);
      expect(ul.positionsLen).to.equal(0);
      expect(ul.reserved.toNumber()).to.equal(0);
      expect(ul.free.toNumber()).to.equal(CREDIT);
      expect(ul.pendingOidCount).to.equal(0);

      const book = await ledger.account.book.fetch(bookPda);
      expect(book.residualLen).to.equal(0);
    });

    it("ninth concurrent oid fails", async () => {
      const startNonce = (
        await ledger.account.userLedger.fetch(userLedgerPda)
      ).nonce.toNumber();
      for (let i = 0; i < 8; i++) {
        await ledger.methods
          .placeOrder(
            ASSET_SOL,
            new BN(1),
            oid(20 + i),
            50,
            false,
            new BN(startNonce + i)
          )
          .accounts(placeAccounts())
          .signers([user])
          .rpc();
      }
      const mid = await ledger.account.userLedger.fetch(userLedgerPda);
      expect(mid.pendingOidCount).to.equal(8);

      try {
        await ledger.methods
          .placeOrder(
            ASSET_SOL,
            new BN(1),
            oid(40),
            50,
            false,
            new BN(startNonce + 8)
          )
          .accounts(placeAccounts())
          .signers([user])
          .rpc();
        expect.fail("ninth concurrent oid should fail");
      } catch (e: any) {
        const code = e.error?.errorCode?.code ?? e.toString();
        expect(code).to.match(/OidCap/);
      }
    });
  });

  describe("S8 withdraw and reserve root", () => {
    const withdrawUser = Keypair.generate();
    let withdrawLedger: PublicKey;
    let userAta: PublicKey;
    const WITHDRAW = 10_000_000;

    before(async () => {
      await airdrop(withdrawUser.publicKey);
      withdrawLedger = pda(ledger.programId, [
        Buffer.from("user"),
        withdrawUser.publicKey.toBuffer(),
      ]);
      userAta = getAssociatedTokenAddressSync(usdcMint, withdrawUser.publicKey);
      await ledger.methods
        .initUser()
        .accounts({
          adapter: adapter.publicKey,
          user: withdrawUser.publicKey,
          config: configPda,
          userLedger: withdrawLedger,
          systemProgram: SystemProgram.programId,
        })
        .signers([adapter, withdrawUser])
        .rpc();
      await ledger.methods
        .creditDeposit(new BN(CREDIT))
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: withdrawLedger,
        })
        .signers([adapter])
        .rpc();
      await createAssociatedTokenAccount(
        connection,
        payer,
        usdcMint,
        withdrawUser.publicKey
      );
      await mintTo(connection, payer, usdcMint, vaultAta, payer, CREDIT);
    });

    it("open position withdraw is rejected", async () => {
      try {
        await ledger.methods
          .requestWithdraw(new BN(1))
          .accounts({
            user: user.publicKey,
            config: configPda,
            book: bookPda,
            userLedger: userLedgerPda,
          })
          .signers([user])
          .rpc();
        expect.fail("pending oids should block withdraw");
      } catch (e: any) {
        const code = e.error?.errorCode?.code ?? e.toString();
        expect(code).to.match(/NotFlat/);
      }
    });

    it("flat withdraw credits user ATA and zeros withdrawable", async () => {
      await ledger.methods
        .requestWithdraw(new BN(WITHDRAW))
        .accounts({
          user: withdrawUser.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: withdrawLedger,
        })
        .signers([withdrawUser])
        .rpc();

      await vault.methods
        .userWithdrawL1(new BN(WITHDRAW))
        .accounts({
          adapter: adapter.publicKey,
          user: withdrawUser.publicKey,
          config: configPda,
          vaultAuthority: vaultAuth,
          vaultUsdcAta: vaultAta,
          userUsdcAta: userAta,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([adapter])
        .rpc();

      await ledger.methods
        .completeWithdraw(new BN(WITHDRAW))
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: withdrawLedger,
        })
        .signers([adapter])
        .rpc();

      const ul = await ledger.account.userLedger.fetch(withdrawLedger);
      expect(ul.withdrawable.toNumber()).to.equal(0);
      expect(ul.free.toNumber()).to.equal(CREDIT - WITHDRAW);
      const ata = await getAccount(connection, userAta);
      expect(Number(ata.amount)).to.equal(WITHDRAW);
    });

    it("root epoch bumps and hash changes after a credit", async () => {
      const before = await vault.account.reserveRoot.fetch(reservePda);
      await vault.methods
        .writeReserveRoot(
          Array.from({ length: 32 }, (_, i) => i),
          1,
          new BN(CREDIT),
          new BN(0),
          Array.from({ length: 32 }, () => 1)
        )
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          reserveRoot: reservePda,
        })
        .signers([adapter])
        .rpc();
      const mid = await vault.account.reserveRoot.fetch(reservePda);
      expect(mid.epoch.toNumber()).to.equal(before.epoch.toNumber() + 1);

      await ledger.methods
        .creditDeposit(new BN(1_000_000))
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: withdrawLedger,
        })
        .signers([adapter])
        .rpc();

      await vault.methods
        .writeReserveRoot(
          Array.from({ length: 32 }, (_, i) => 32 - i),
          1,
          new BN(CREDIT + 1_000_000 - WITHDRAW),
          new BN(0),
          Array.from({ length: 32 }, () => 2)
        )
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          reserveRoot: reservePda,
        })
        .signers([adapter])
        .rpc();
      const after = await vault.account.reserveRoot.fetch(reservePda);
      expect(after.epoch.toNumber()).to.equal(mid.epoch.toNumber() + 1);
      expect(Buffer.from(after.root).toString("hex")).to.not.equal(
        Buffer.from(mid.root).toString("hex")
      );
    });
  });
});
