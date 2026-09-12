import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  ACCOUNT_SIZE,
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccount,
  createInitializeAccountInstruction,
  createMint,
  getAccount,
  getAssociatedTokenAddressSync,
  getMinimumBalanceForRentExemptAccount,
  mintTo,
} from "@solana/spl-token";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import { DELEGATION_PROGRAM_ID } from "@magicblock-labs/ephemeral-rollups-sdk";
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
        book: bookPda,
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

    it("liquidate_user is adapter-signed, tentative, Book moves on ack", async () => {
      const liqOid = oid(90);
      try {
        await ledger.methods
          .liquidateUser(ASSET_SOL, liqOid)
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
        .liquidateUser(ASSET_SOL, liqOid)
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: userLedgerPda,
          book: bookPda,
        })
        .signers([adapter])
        .rpc();

      let ul = await ledger.account.userLedger.fetch(userLedgerPda);
      expect(ul.positionsLen).to.equal(0);
      expect(ul.reserved.toNumber()).to.equal(0);
      expect(ul.free.toNumber()).to.equal(CREDIT);
      expect(ul.pendingOidCount).to.equal(1);
      const liqRow = ul.openOids.find((o: { state: number }) => o.state === 3);
      expect(liqRow).to.exist;
      expect(liqRow.lotsDelta.toNumber()).to.equal(-LOTS);

      let book = await ledger.account.book.fetch(bookPda);
      expect(book.residualLen).to.equal(1);
      expect(book.residuals[0].lots.toNumber()).to.equal(LOTS);

      await ledger.methods
        .ackPhoenixFill(liqOid, new BN(-LOTS), new BN(0), new BN(0))
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: userLedgerPda,
          book: bookPda,
          feeAccrual: feesPda,
        })
        .signers([adapter])
        .rpc();

      ul = await ledger.account.userLedger.fetch(userLedgerPda);
      expect(ul.pendingOidCount).to.equal(0);
      expect(ul.positionsLen).to.equal(0);

      book = await ledger.account.book.fetch(bookPda);
      expect(book.residualLen).to.equal(0);
    });

    it("heartbeat_scan writes last_scan_ms", async () => {
      await ledger.methods
        .heartbeatScan(new BN(1_500))
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
        })
        .signers([adapter])
        .rpc();
      const book = await ledger.account.book.fetch(bookPda);
      expect(book.lastScanMs.toNumber()).to.equal(1_500);
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

  describe("G-PNL reducing ack", () => {
    const pnlUser = Keypair.generate();
    let pnlLedger: PublicKey;
    const OPEN_VWAP = 10_000_000; // 10 lots * 1 USDC
    const CLOSE_VWAP = -11_000_000; // sell 10 at 1.1 USDC
    const PROFIT = 1_000_000;

    before(async () => {
      await airdrop(pnlUser.publicKey);
      pnlLedger = pda(ledger.programId, [
        Buffer.from("user"),
        pnlUser.publicKey.toBuffer(),
      ]);
      await ledger.methods
        .initUser()
        .accounts({
          adapter: adapter.publicKey,
          user: pnlUser.publicKey,
          config: configPda,
          userLedger: pnlLedger,
          book: bookPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([adapter, pnlUser])
        .rpc();
      await ledger.methods
        .creditDeposit(new BN(CREDIT))
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: pnlLedger,
        })
        .signers([adapter])
        .rpc();
    });

    it("open then full close credits realized into free; fail-ack does not", async () => {
      await ledger.methods
        .placeOrder(ASSET_SOL, new BN(LOTS), oid(70), 50, false, new BN(0))
        .accounts({
          user: pnlUser.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: pnlLedger,
        })
        .signers([pnlUser])
        .rpc();
      await ledger.methods
        .ackPhoenixFill(oid(70), new BN(LOTS), new BN(0), new BN(OPEN_VWAP))
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: pnlLedger,
          book: bookPda,
          feeAccrual: feesPda,
        })
        .signers([adapter])
        .rpc();

      const opened = await ledger.account.userLedger.fetch(pnlLedger);
      expect(opened.positions[0].entryQuoteLots.toNumber()).to.equal(OPEN_VWAP);
      expect(opened.free.toNumber()).to.equal(CREDIT - IM_TEN_LOTS);

      await ledger.methods
        .placeOrder(ASSET_SOL, new BN(-LOTS), oid(71), 50, true, new BN(1))
        .accounts({
          user: pnlUser.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: pnlLedger,
        })
        .signers([pnlUser])
        .rpc();
      const mid = await ledger.account.userLedger.fetch(pnlLedger);
      const freeWhilePending = mid.free.toNumber();

      await ledger.methods
        .ackPhoenixFail(oid(71))
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: pnlLedger,
          book: bookPda,
        })
        .signers([adapter])
        .rpc();
      const afterFail = await ledger.account.userLedger.fetch(pnlLedger);
      expect(afterFail.positions[0].lots.toNumber()).to.equal(LOTS);
      expect(afterFail.free.toNumber()).to.equal(CREDIT - IM_TEN_LOTS);
      expect(afterFail.free.toNumber()).to.not.equal(freeWhilePending + PROFIT);

      await ledger.methods
        .placeOrder(ASSET_SOL, new BN(-LOTS), oid(72), 50, true, new BN(2))
        .accounts({
          user: pnlUser.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: pnlLedger,
        })
        .signers([pnlUser])
        .rpc();
      await ledger.methods
        .ackPhoenixFill(oid(72), new BN(-LOTS), new BN(0), new BN(CLOSE_VWAP))
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: pnlLedger,
          book: bookPda,
          feeAccrual: feesPda,
        })
        .signers([adapter])
        .rpc();

      const closed = await ledger.account.userLedger.fetch(pnlLedger);
      expect(closed.positionsLen).to.equal(0);
      expect(closed.reserved.toNumber()).to.equal(0);
      expect(closed.free.toNumber()).to.equal(CREDIT + PROFIT);
    });

    it("out-of-order ack uses confirmed lots not tentative", async () => {
      await ledger.methods
        .placeOrder(ASSET_SOL, new BN(LOTS), oid(80), 50, false, new BN(3))
        .accounts({
          user: pnlUser.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: pnlLedger,
        })
        .signers([pnlUser])
        .rpc();
      await ledger.methods
        .ackPhoenixFill(oid(80), new BN(LOTS), new BN(0), new BN(OPEN_VWAP))
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: pnlLedger,
          book: bookPda,
          feeAccrual: feesPda,
        })
        .signers([adapter])
        .rpc();
      await ledger.methods
        .placeOrder(ASSET_SOL, new BN(5), oid(81), 50, false, new BN(4))
        .accounts({
          user: pnlUser.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: pnlLedger,
        })
        .signers([pnlUser])
        .rpc();
      await ledger.methods
        .placeOrder(ASSET_SOL, new BN(-4), oid(82), 50, false, new BN(5))
        .accounts({
          user: pnlUser.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: pnlLedger,
        })
        .signers([pnlUser])
        .rpc();
      const freeBefore = (
        await ledger.account.userLedger.fetch(pnlLedger)
      ).free.toNumber();
      await ledger.methods
        .ackPhoenixFill(oid(82), new BN(-4), new BN(0), new BN(-4_400_000))
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: pnlLedger,
          book: bookPda,
          feeAccrual: feesPda,
        })
        .signers([adapter])
        .rpc();
      const after = await ledger.account.userLedger.fetch(pnlLedger);
      // Confirmed was +10; closing 4 @ 1.1 vs 1.0 → +0.4 USDC. Tentative leftover +5 still pending.
      expect(after.positions[0].lots.toNumber()).to.equal(11);
      expect(after.positions[0].entryQuoteLots.toNumber()).to.equal(6_000_000);
      expect(after.free.toNumber()).to.equal(freeBefore + 400_000);
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
          book: bookPda,
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

  describe("S9 vault PDA and settle action", () => {
    const standIn = Keypair.generate();
    const phoenixSide = Keypair.generate();
    const SETTLE = 5_000_000;

    before(async () => {
      await airdrop(standIn.publicKey);
      const rent = await getMinimumBalanceForRentExemptAccount(connection);
      const tx = new Transaction().add(
        SystemProgram.createAccount({
          fromPubkey: payer.publicKey,
          newAccountPubkey: phoenixSide.publicKey,
          space: ACCOUNT_SIZE,
          lamports: rent,
          programId: TOKEN_PROGRAM_ID,
        }),
        createInitializeAccountInstruction(
          phoenixSide.publicKey,
          usdcMint,
          vaultAuth
        )
      );
      await provider.sendAndConfirm(tx, [phoenixSide]);
      await mintTo(connection, payer, usdcMint, vaultAta, payer, SETTLE * 2);
    });

    it("retire_stand_in sets Config.vault_authority to the PDA", async () => {
      await vault.methods
        .retireStandIn()
        .accounts({
          admin: payer.publicKey,
          config: configPda,
          vaultAuthority: vaultAuth,
        })
        .rpc();
      const cfg = await vault.account.config.fetch(configPda);
      expect(cfg.vaultAuthority.toBase58()).to.equal(vaultAuth.toBase58());
    });

    it("PDA-signed post/pull move USDC without the stand-in key", async () => {
      await vault.methods
        .postCollateral(new BN(SETTLE))
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          vaultAuthority: vaultAuth,
          vaultUsdcAta: vaultAta,
          destUsdcAta: phoenixSide.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([adapter])
        .rpc();
      expect(Number((await getAccount(connection, phoenixSide.publicKey)).amount)).to.equal(
        SETTLE
      );

      await vault.methods
        .pullCollateralPda(new BN(SETTLE))
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          vaultAuthority: vaultAuth,
          vaultUsdcAta: vaultAta,
          sourceUsdcAta: phoenixSide.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([adapter])
        .rpc();
      expect(Number((await getAccount(connection, phoenixSide.publicKey)).amount)).to.equal(
        0
      );
    });

    it("stand-in key cannot pull after retire", async () => {
      const standInAta = await createAssociatedTokenAccount(
        connection,
        payer,
        usdcMint,
        standIn.publicKey
      );
      try {
        await vault.methods
          .pullCollateral(new BN(1))
          .accounts({
            adapter: adapter.publicKey,
            standIn: standIn.publicKey,
            config: configPda,
            vaultUsdcAta: vaultAta,
            sourceUsdcAta: standInAta,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([adapter, standIn])
          .rpc();
        expect.fail("stand-in must not pull after retire");
      } catch (e: any) {
        const code = e.error?.errorCode?.code ?? e.toString();
        expect(code).to.match(/BadTransitOwner|Unauthorized/);
      }
    });

    it("settle_user_withdraw pays via PDA with escrow_auth bound to the PDA", async () => {
      const settleAta = await createAssociatedTokenAccount(
        connection,
        payer,
        usdcMint,
        user.publicKey
      );
      const escrow = PublicKey.findProgramAddressSync(
        [Buffer.from("balance"), vaultAuth.toBuffer(), Buffer.from([255])],
        DELEGATION_PROGRAM_ID
      )[0];
      await vault.methods
        .settleUserWithdraw(new BN(SETTLE))
        .accounts({
          adapter: adapter.publicKey,
          user: user.publicKey,
          config: configPda,
          vaultAuthority: vaultAuth,
          escrowAuth: vaultAuth,
          escrow,
          vaultUsdcAta: vaultAta,
          userUsdcAta: settleAta,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([adapter])
        .rpc();
      expect(Number((await getAccount(connection, settleAta)).amount)).to.equal(
        SETTLE
      );
    });
  });

  describe("Funding allocation", () => {
    const fundUser = Keypair.generate();
    let fundLedger: PublicKey;
    const DELTA = -1_000_000;

    before(async () => {
      await airdrop(fundUser.publicKey);
      fundLedger = pda(ledger.programId, [
        Buffer.from("user"),
        fundUser.publicKey.toBuffer(),
      ]);
      await ledger.methods
        .initUser()
        .accounts({
          adapter: adapter.publicKey,
          user: fundUser.publicKey,
          config: configPda,
          userLedger: fundLedger,
          book: bookPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([adapter, fundUser])
        .rpc();
      await ledger.methods
        .creditDeposit(new BN(CREDIT))
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: fundLedger,
        })
        .signers([adapter])
        .rpc();
      await ledger.methods
        .placeOrder(ASSET_SOL, new BN(LOTS), oid(80), 50, false, new BN(0))
        .accounts({
          user: fundUser.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: fundLedger,
        })
        .signers([fundUser])
        .rpc();
      await ledger.methods
        .ackPhoenixFill(oid(80), new BN(LOTS), new BN(0), new BN(0))
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: fundLedger,
          book: bookPda,
          feeAccrual: feesPda,
        })
        .signers([adapter])
        .rpc();
    });

    it("accrue writes unsettled only; fold moves cash; epoch gates", async () => {
      const before = await ledger.account.userLedger.fetch(fundLedger);
      const freeBefore = before.free.toNumber();

      await ledger.methods
        .bumpFundingEpoch(new BN(1))
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
        })
        .signers([adapter])
        .rpc();

      await ledger.methods
        .allocateFunding(new BN(1), false, [
          { assetId: ASSET_SOL, deltaUsdc: new BN(DELTA) },
        ])
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: fundLedger,
          book: bookPda,
        })
        .signers([adapter])
        .rpc();

      const accrued = await ledger.account.userLedger.fetch(fundLedger);
      expect(accrued.free.toNumber()).to.equal(freeBefore);
      expect(accrued.positions[0].unsettledFunding.toNumber()).to.equal(DELTA);
      expect(accrued.lastFundingEpoch.toNumber()).to.equal(1);

      try {
        await ledger.methods
          .allocateFunding(new BN(1), false, [])
          .accounts({
            adapter: adapter.publicKey,
            config: configPda,
            userLedger: fundLedger,
            book: bookPda,
          })
          .signers([adapter])
          .rpc();
        expect.fail("replay epoch should fail");
      } catch (e: any) {
        expect((e.error?.errorCode?.code ?? e.toString()).toString()).to.match(
          /BadFundingEpoch/
        );
      }

      try {
        await ledger.methods
          .allocateFunding(new BN(3), false, [])
          .accounts({
            adapter: adapter.publicKey,
            config: configPda,
            userLedger: fundLedger,
            book: bookPda,
          })
          .signers([adapter])
          .rpc();
        expect.fail("gap epoch should fail");
      } catch (e: any) {
        expect((e.error?.errorCode?.code ?? e.toString()).toString()).to.match(
          /BadFundingEpoch/
        );
      }

      try {
        await ledger.methods
          .requestWithdraw(new BN(1))
          .accounts({
            user: fundUser.publicKey,
            config: configPda,
            book: bookPda,
            userLedger: fundLedger,
          })
          .signers([fundUser])
          .rpc();
        expect.fail("open position should fail first");
      } catch (e: any) {
        const code = (e.error?.errorCode?.code ?? e.toString()).toString();
        expect(code).to.match(/NotFlat|UnsettledFunding/);
      }

      await ledger.methods
        .allocateFunding(new BN(1), true, [])
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: fundLedger,
          book: bookPda,
        })
        .signers([adapter])
        .rpc();

      const folded = await ledger.account.userLedger.fetch(fundLedger);
      expect(folded.free.toNumber()).to.equal(freeBefore + DELTA);
      expect(folded.positions[0].unsettledFunding.toNumber()).to.equal(0);
    });
  });
});
