import * as anchor from "@anchor-lang/core";
import { BN, Program } from "@anchor-lang/core";
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
import { createHash } from "crypto";
import { CinderVault } from "../target/types/cinder_vault";
import { CinderLedger } from "../target/types/cinder_ledger";

const ER_VALIDATOR = new PublicKey(
  "mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev"
);
const ASSET_SOL = 1;
const ASSET_BTC = 2;
const HALT_ENTRIES = 1 << 0;
const VENUE_BREACH = 1 << 7;
const CREDIT = 100_000_000; // 100 USDC
const LOTS = 10;
// stub IM: 10 lots * 1e6 * 12500 / (10 * 10000) = 1_250_000
const IM_TEN_LOTS = 1_250_000;
const IM_PER_LOT = IM_TEN_LOTS / LOTS;
const LIMIT_TICKS = new BN(1_000_000);
const LAST_VALID_SLOT = new BN("18446744073709551615");

function pda(programId: PublicKey, seeds: (Buffer | Uint8Array)[]): PublicKey {
  return PublicKey.findProgramAddressSync(seeds, programId)[0];
}

function oid(tag: number): number[] {
  return Array.from({ length: 16 }, (_, i) => (i + tag) % 256);
}

describe("ledger accounts and order machine", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const vault = (anchor.workspace as any).cinderVault as Program<CinderVault>;
  const ledger = (anchor.workspace as any).cinderLedger as Program<CinderLedger>;
  const payer = (provider.wallet as anchor.Wallet).payer;
  const connection = provider.connection;

  const adapter = Keypair.generate();
  const unauthorizedAdapter = Keypair.generate();
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

  it("initializes Config, UserLedger, Book, and FeeAccrual with stable seeds", async () => {
    await vault.methods
      .initialize(adapter.publicKey, vaultAuth, phoenixTrader, ER_VALIDATOR)
      .accountsPartial({
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
    expect(root.schemaVersion).to.equal(1);
    expect(root.epoch.toNumber()).to.equal(0);
    expect(root.totalBadDebt.toNumber()).to.equal(0);
    expect((await connection.getAccountInfo(reservePda))?.data.length).to.equal(117);

    await ledger.methods
      .initialize()
      .accountsPartial({
        adapter: adapter.publicKey,
        config: configPda,
        book: bookPda,
        feeAccrual: feesPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([adapter])
      .rpc();

    const book = await ledger.account.book.fetch(bookPda);
    expect(book.schemaVersion).to.equal(1);
    expect(book.residualLen).to.equal(0);
    expect(book.invariantOk).to.equal(1);
    expect(book.halt).to.equal(0);
    expect(book.phoenixCollateral.toNumber()).to.equal(0);
    expect((await connection.getAccountInfo(bookPda))?.data.length).to.equal(365);

    const fees = await ledger.account.feeAccrual.fetch(feesPda);
    expect(fees.phoenixFeesPaid.toNumber()).to.equal(0);
    expect(fees.cinderFeesAccrued.toNumber()).to.equal(0);

    await vault.methods
      .setAllowlist([ASSET_SOL])
      .accountsPartial({ admin: payer.publicKey, config: configPda })
      .rpc();

    await ledger.methods
      .initUser()
      .accountsPartial({
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
    expect(ul.schemaVersion).to.equal(1);
    expect(ul.user.toBase58()).to.equal(user.publicKey.toBase58());
    expect(ul.free.toNumber()).to.equal(0);
    expect(ul.reserved.toNumber()).to.equal(0);
    expect(ul.badDebtUsdc.toNumber()).to.equal(0);
    expect(ul.nonce.toNumber()).to.equal(0);
    expect(ul.positionsLen).to.equal(0);
    expect(ul.pendingOidCount).to.equal(0);
    expect((await connection.getAccountInfo(userLedgerPda))?.data.length).to.equal(980);

    await ledger.methods
      .creditDeposit(new BN(CREDIT))
      .accountsPartial({
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

  it("rejects a second migration of current accounts", async () => {
    const expectAlreadyMigrated = async (request: Promise<string>) => {
      try {
        await request;
        expect.fail("migration should be one-shot");
      } catch (e: any) {
        const code = e.error?.errorCode?.code ?? e.toString();
        expect(code).to.match(/AccountAlreadyMigrated/);
      }
    };

    await expectAlreadyMigrated(
      ledger.methods
        .migrateBook()
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
        })
        .signers([adapter])
        .rpc()
    );
    await expectAlreadyMigrated(
      ledger.methods
        .migrateUserLedger()
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: userLedgerPda,
        })
        .signers([adapter])
        .rpc()
    );
    await expectAlreadyMigrated(
      vault.methods
        .migrateReserveRoot()
        .accountsPartial({
          admin: payer.publicKey,
          config: configPda,
          reserveRoot: reservePda,
          systemProgram: SystemProgram.programId,
        })
        .rpc()
    );
  });

  describe("guarded operator recovery", () => {
    let recoveryUser: Keypair;
    let recoveryLedger: PublicKey;
    const clientOid = oid(210);
    const accounts = () => ({ adapter: adapter.publicKey, config: configPda,
      book: bookPda, userLedger: recoveryLedger });

    before(async () => {
      // Also support selecting only this suite/scenario on a fresh validator.
      if (await connection.getAccountInfo(configPda) === null) {
        await vault.methods.initialize(adapter.publicKey, vaultAuth, phoenixTrader, ER_VALIDATOR)
          .accountsPartial({ admin: payer.publicKey, config: configPda, vaultAuthority: vaultAuth,
            reserveRoot: reservePda, usdcMint, vaultUsdcAta: vaultAta,
            tokenProgram: TOKEN_PROGRAM_ID, associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId }).rpc();
      }
      if (await connection.getAccountInfo(bookPda) === null) {
        await ledger.methods.initialize().accountsPartial({ adapter: adapter.publicKey,
          config: configPda, book: bookPda, feeAccrual: feesPda,
          systemProgram: SystemProgram.programId }).signers([adapter]).rpc();
      }
      await vault.methods.setAllowlist([ASSET_SOL])
        .accountsPartial({ admin: payer.publicKey, config: configPda }).rpc();
    });

    beforeEach(async () => {
      recoveryUser = Keypair.generate();
      recoveryLedger = pda(ledger.programId, [Buffer.from("user"), recoveryUser.publicKey.toBuffer()]);
      await ledger.methods.initUser().accountsPartial({ ...accounts(), user: recoveryUser.publicKey,
        systemProgram: SystemProgram.programId }).signers([adapter, recoveryUser]).rpc();
      await ledger.methods.creditDeposit(new BN(CREDIT)).accountsPartial(accounts()).signers([adapter]).rpc();
    });

    async function expectRejected(request: Promise<unknown>, pattern: RegExp) {
      let error: any;
      try { await request; } catch (e) { error = e; }
      expect(error, "transaction should be rejected").to.exist;
      expect(error.error?.errorCode?.code ?? error.toString()).to.match(pattern);
    }

    async function guard(placementNonce: number) {
      const state = await ledger.account.userLedger.fetch(recoveryLedger);
      const encoded = await ledger.coder.accounts.encode("userLedger", state);
      return { clientOid, placementNonce: new BN(placementNonce),
        observedLedgerNonce: state.nonce,
        observedLedgerHash: Array.from(createHash("sha256").update(encoded.subarray(8)).digest()),
        kind: 1, assetId: ASSET_SOL, requestedLots: new BN(LOTS),
        limitPriceTicks: LIMIT_TICKS, lastValidSlot: LAST_VALID_SLOT };
    }

    async function place(nonce: number) {
      await ledger.methods.placeOrder(ASSET_SOL, new BN(LOTS), clientOid,
        LIMIT_TICKS, LAST_VALID_SLOT, new BN(IM_TEN_LOTS), new BN(10_000_000), false, new BN(nonce))
        .accountsPartial({ ...accounts(), user: recoveryUser.publicKey })
        .signers([adapter, recoveryUser]).rpc();
    }

    it("changes only OPERATOR_DOWN and rejects a foreign operator", async () => {
      const flags = HALT_ENTRIES | (1 << 6) | VENUE_BREACH;
      const configBefore = (await vault.account.config.fetch(configPda)).paused;
      const bookBefore = (await ledger.account.book.fetch(bookPda)).halt;
      try {
        await vault.methods.setHalt(flags).accountsPartial({ admin: payer.publicKey, config: configPda }).rpc();
        await ledger.methods.setBookHalt(flags).accountsPartial({ adapter: adapter.publicKey, config: configPda, book: bookPda }).signers([adapter]).rpc();
        await expectRejected(vault.methods.setOperatorDown(false).accountsPartial({ adapter: unauthorizedAdapter.publicKey, config: configPda }).signers([unauthorizedAdapter]).rpc(), /Unauthorized/);
        await expectRejected(ledger.methods.setOperatorDown(false).accountsPartial({ adapter: unauthorizedAdapter.publicKey, config: configPda, book: bookPda }).signers([unauthorizedAdapter]).rpc(), /Unauthorized/);
        for (const down of [true, false]) {
          await vault.methods.setOperatorDown(down).accountsPartial({ adapter: adapter.publicKey, config: configPda }).signers([adapter]).rpc();
          await ledger.methods.setOperatorDown(down).accountsPartial({ adapter: adapter.publicKey, config: configPda, book: bookPda }).signers([adapter]).rpc();
          const expected = flags | (down ? 1 << 5 : 0);
          expect((await vault.account.config.fetch(configPda)).paused).to.equal(expected);
          expect((await ledger.account.book.fetch(bookPda)).halt).to.equal(expected);
        }
        await expectRejected(vault.methods.addOperatorHalt(32).accountsPartial({ adapter: unauthorizedAdapter.publicKey, config: configPda }).signers([unauthorizedAdapter]).rpc(), /Unauthorized/);
        await expectRejected(ledger.methods.addOperatorHalt(32).accountsPartial({ adapter: unauthorizedAdapter.publicKey, config: configPda, book: bookPda }).signers([unauthorizedAdapter]).rpc(), /Unauthorized/);
        for (const addition of [32, 0, 32]) {
          await vault.methods.addOperatorHalt(addition).accountsPartial({ adapter: adapter.publicKey, config: configPda }).signers([adapter]).rpc();
          await ledger.methods.addOperatorHalt(addition).accountsPartial({ adapter: adapter.publicKey, config: configPda, book: bookPda }).signers([adapter]).rpc();
          expect((await vault.account.config.fetch(configPda)).paused).to.equal(flags | 32);
          expect((await ledger.account.book.fetch(bookPda)).halt).to.equal(flags | 32);
        }
      } finally {
        await vault.methods.setHalt(configBefore).accountsPartial({ admin: payer.publicKey, config: configPda }).rpc();
        await ledger.methods.setBookHalt(bookBefore).accountsPartial({ adapter: adapter.publicKey, config: configPda, book: bookPda }).signers([adapter]).rpc();
      }
    });

    it("requires the exact guarded intent and applies failure only once", async () => {
      await place(0);
      const observed = await guard(0);
      for (const altered of [
        { ...observed, limitPriceTicks: LIMIT_TICKS.addn(1) },
        { ...observed, requestedLots: new BN(-LOTS) },
        { ...observed, placementNonce: new BN(1) },
        { ...observed, kind: 2 },
      ]) {
        await expectRejected(ledger.methods.ackPhoenixFailGuarded(altered, new BN(0))
          .accountsPartial(accounts()).signers([adapter]).rpc(), /OidNotFound|BadNonce/);
      }
      await expectRejected(ledger.methods.ackPhoenixFailGuarded(observed, new BN(0))
        .accountsPartial({ ...accounts(), adapter: unauthorizedAdapter.publicKey })
        .signers([unauthorizedAdapter]).rpc(), /Unauthorized/);
      await ledger.methods.ackPhoenixFailGuarded(observed, new BN(0)).accountsPartial(accounts()).signers([adapter]).rpc();
      await expectRejected(ledger.methods.ackPhoenixFailGuarded(observed, new BN(0))
        .accountsPartial(accounts()).signers([adapter]).rpc(), /BadNonce|OidNotFound/);
      expect((await ledger.account.userLedger.fetch(recoveryLedger)).free.toNumber()).to.equal(CREDIT);
    });

    it("rejects reused-OID and non-nonce cash-write races without losing the pending order", async () => {
      await place(0);
      await ledger.methods.ackPhoenixFailGuarded(await guard(0), new BN(0))
        .accountsPartial(accounts()).signers([adapter]).rpc();
      const old = await guard(0);
      await place(1);
      await expectRejected(ledger.methods.ackPhoenixFailGuarded(old, new BN(0))
        .accountsPartial(accounts()).signers([adapter]).rpc(), /BadNonce/);
      const observed = await guard(1);
      await ledger.methods.creditDeposit(new BN(1)).accountsPartial(accounts()).signers([adapter]).rpc();
      const changed = await ledger.account.userLedger.fetch(recoveryLedger);
      expect(changed.nonce.toString()).to.equal(observed.observedLedgerNonce.toString());
      await expectRejected(ledger.methods.ackPhoenixFailGuarded(observed, new BN(0))
        .accountsPartial(accounts()).signers([adapter]).rpc(), /BadNonce/);
      expect((await ledger.account.userLedger.fetch(recoveryLedger)).pendingOidCount).to.equal(1);
      await ledger.methods.ackPhoenixFailGuarded(await guard(1), new BN(0))
        .accountsPartial(accounts()).signers([adapter]).rpc();
      const restored = await ledger.account.userLedger.fetch(recoveryLedger);
      expect(restored.pendingOidCount).to.equal(0);
      expect(restored.free.toNumber()).to.equal(CREDIT + 1);
    });
  });

  it("requires the configured adapter and nonzero Phoenix order bounds", async () => {
    const expectFailure = async (request: Promise<string>, pattern: RegExp) => {
      try {
        await request;
        expect.fail("place_order should have failed");
      } catch (e: any) {
        const code = e.error?.errorCode?.code ?? e.toString();
        expect(code).to.match(pattern);
      }
    };
    const place = (
      adapterKey: PublicKey,
      signers: Keypair[],
      limit: BN,
      deadline: BN
    ) =>
      ledger.methods
        .placeOrder(
          ASSET_SOL,
          new BN(1),
          oid(200),
          limit,
          deadline,
          new BN(IM_PER_LOT),
          new BN(1_000_000),
          false,
          new BN(0)
        )
        .accountsPartial({
          user: user.publicKey,
          adapter: adapterKey,
          config: configPda,
          book: bookPda,
          userLedger: userLedgerPda,
        })
        .signers(signers)
        .rpc();

    await expectFailure(
      place(unauthorizedAdapter.publicKey, [user, unauthorizedAdapter], LIMIT_TICKS, LAST_VALID_SLOT),
      /Unauthorized/
    );
    await expectFailure(
      place(adapter.publicKey, [user, adapter], new BN(0), LAST_VALID_SLOT),
      /ZeroLimitPrice/
    );
    await expectFailure(
      place(adapter.publicKey, [user, adapter], LIMIT_TICKS, new BN(0)),
      /ZeroDeadline/
    );

    const state = await ledger.account.userLedger.fetch(userLedgerPda);
    expect(state.nonce.toNumber()).to.equal(0);
    expect(state.pendingOidCount).to.equal(0);
  });

  it("place +10 lots then fail-ack restores free and lots", async () => {
    await ledger.methods
      .placeOrder(
        ASSET_SOL,
        new BN(LOTS),
        oid(1),
        LIMIT_TICKS,
        LAST_VALID_SLOT,
        new BN(IM_TEN_LOTS),
        new BN(LOTS * 1_000_000),
        false,
        new BN(0)
      )
      .accountsPartial({
        user: user.publicKey,
        adapter: adapter.publicKey,
        config: configPda,
        book: bookPda,
        userLedger: userLedgerPda,
      })
      .signers([user, adapter])
      .rpc();

    let ul = await ledger.account.userLedger.fetch(userLedgerPda);
    expect(ul.free.toNumber()).to.equal(CREDIT - IM_TEN_LOTS);
    expect(ul.reserved.toNumber()).to.equal(IM_TEN_LOTS);
    expect(ul.positionsLen).to.equal(1);
    expect(ul.positions[0].lots.toNumber()).to.equal(LOTS);
    expect(ul.positions[0].reservedIm.toNumber()).to.equal(IM_TEN_LOTS);
    expect(ul.pendingOidCount).to.equal(1);
    expect(ul.nonce.toNumber()).to.equal(1);
    expect(ul.openOids[0].limitPriceTicks.toNumber()).to.equal(LIMIT_TICKS.toNumber());
    expect(ul.openOids[0].lastValidSlot.toString()).to.equal(LAST_VALID_SLOT.toString());

    const bookBefore = await ledger.account.book.fetch(bookPda);
    expect(bookBefore.residualLen).to.equal(0);

    await ledger.methods
      .ackPhoenixFail(oid(1), new BN(0))
      .accountsPartial({
        adapter: adapter.publicKey,
        config: configPda,
        book: bookPda,
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
      .placeOrder(
        ASSET_SOL,
        new BN(LOTS),
        oid(2),
        LIMIT_TICKS,
        LAST_VALID_SLOT,
        new BN(IM_TEN_LOTS),
        new BN(LOTS * 1_000_000),
        false,
        new BN(1)
      )
      .accountsPartial({
        user: user.publicKey,
        adapter: adapter.publicKey,
        config: configPda,
        book: bookPda,
        userLedger: userLedgerPda,
      })
      .signers([user, adapter])
      .rpc();

    await ledger.methods
      .ackPhoenixFill(
        oid(2),
        new BN(LOTS),
        new BN(0),
        new BN(0),
        new BN(LIMIT_TICKS.toNumber() + 1),
        new BN(IM_TEN_LOTS)
      )
      .accountsPartial({
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
    expect(book.halt & (VENUE_BREACH | HALT_ENTRIES)).to.equal(
      VENUE_BREACH | HALT_ENTRIES
    );

    await ledger.methods
      .setBookHalt(0)
      .accountsPartial({
        adapter: adapter.publicKey,
        config: configPda,
        book: bookPda,
      })
      .signers([adapter])
      .rpc();
  });

  it("HALT_ENTRIES blocks place_order", async () => {
    await ledger.methods
      .setBookHalt(HALT_ENTRIES)
      .accountsPartial({
        adapter: adapter.publicKey,
        config: configPda,
        book: bookPda,
      })
      .signers([adapter])
      .rpc();

    try {
      await ledger.methods
        .placeOrder(
          ASSET_SOL,
          new BN(LOTS),
          oid(3),
          LIMIT_TICKS,
          LAST_VALID_SLOT,
          new BN(IM_TEN_LOTS),
          new BN(LOTS * 1_000_000),
          false,
          new BN(2)
        )
        .accountsPartial({
          user: user.publicKey,
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: userLedgerPda,
        })
        .signers([user, adapter])
        .rpc();
      expect.fail("place_order should fail when HALT_ENTRIES is set");
    } catch (e: any) {
      const code = e.error?.errorCode?.code ?? e.toString();
      expect(code).to.match(/Halted/);
    }

    await ledger.methods
      .setBookHalt(0)
      .accountsPartial({
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
        .accountsPartial({ user: user.publicKey })
        .signers([user])
        .rpc();
      expect.fail("escape_withdraw should be unsupported");
    } catch (e: any) {
      const code = e.error?.errorCode?.code ?? e.toString();
      expect(code).to.match(/Unsupported/);
    }
  });

  describe("order machine", () => {
    const placeAccounts = () => ({
      user: user.publicKey,
      adapter: adapter.publicKey,
      config: configPda,
      book: bookPda,
      userLedger: userLedgerPda,
    });

    it("replay nonce fails", async () => {
      try {
        await ledger.methods
          .placeOrder(
            ASSET_SOL,
            new BN(1),
            oid(4),
            LIMIT_TICKS,
            LAST_VALID_SLOT,
            new BN(IM_PER_LOT),
            new BN(1_000_000),
            false,
            new BN(0)
          )
          .accountsPartial(placeAccounts())
          .signers([user, adapter])
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
          .placeOrder(
            ASSET_SOL,
            new BN(1),
            oid(5),
            LIMIT_TICKS,
            LAST_VALID_SLOT,
            new BN(IM_PER_LOT),
            new BN(1_000_000),
            true,
            new BN(2)
          )
          .accountsPartial(placeAccounts())
          .signers([user, adapter])
          .rpc();
        expect.fail("reduce-only increase should fail");
      } catch (e: any) {
        const code = e.error?.errorCode?.code ?? e.toString();
        expect(code).to.match(/ReduceOnlyIncrease/);
      }
    });

    it("bounded liquidation cannot flip or cancel an unresolved intent and restores on failure", async () => {
      const before = await ledger.account.userLedger.fetch(userLedgerPda);
      const beforeBook = await ledger.account.book.fetch(bookPda);
      const liquidationAccounts = { adapter: adapter.publicKey, config: configPda, userLedger: userLedgerPda, book: bookPda };
      for (const quantity of [0, LOTS + 1]) {
        try {
          await ledger.methods.liquidateUserBounded(ASSET_SOL, oid(88), LIMIT_TICKS, LAST_VALID_SLOT, new BN(quantity), new BN(0))
            .accountsPartial(liquidationAccounts).signers([adapter]).rpc();
          expect.fail("invalid bounded liquidation must reject");
        } catch (e: any) { expect(e.error?.errorCode?.code ?? e.toString()).to.match(/Overflow/); }
      }
      const close = 4;
      await ledger.methods.liquidateUserBounded(ASSET_SOL, oid(89), LIMIT_TICKS, LAST_VALID_SLOT, new BN(close), new BN((LOTS - close) * IM_PER_LOT))
        .accountsPartial(liquidationAccounts).signers([adapter]).rpc();
      const tentative = await ledger.account.userLedger.fetch(userLedgerPda);
      expect(tentative.positions[0].lots.toNumber()).to.equal(LOTS - close);
      expect(tentative.positions[0].entryQuoteLots.toString()).to.equal(before.positions[0].entryQuoteLots.toString());
      expect(tentative.nonce.toString()).to.equal(before.nonce.toString());
      expect((await ledger.account.book.fetch(bookPda)).residuals).to.deep.equal(beforeBook.residuals);
      try {
        await ledger.methods.liquidateUserBounded(ASSET_SOL, oid(88), LIMIT_TICKS, LAST_VALID_SLOT, new BN(1), new BN(0))
          .accountsPartial(liquidationAccounts).signers([adapter]).rpc();
        expect.fail("must not cancel the first unresolved liquidation");
      } catch (e: any) { expect(e.error?.errorCode?.code ?? e.toString()).to.match(/OidCap/); }
      await ledger.methods.ackPhoenixFail(oid(89), new BN(LOTS * IM_PER_LOT)).accountsPartial(liquidationAccounts).signers([adapter]).rpc();
      const restored = await ledger.account.userLedger.fetch(userLedgerPda);
      expect(restored.pendingOidCount).to.equal(0);
      expect(restored.positions[0].lots.toString()).to.equal(before.positions[0].lots.toString());
      expect(restored.positions[0].entryQuoteLots.toString()).to.equal(before.positions[0].entryQuoteLots.toString());
      expect(restored.free.add(restored.reserved).toString()).to.equal(before.free.add(before.reserved).toString());
    });

    it("liquidate_user is adapter-signed, tentative, Book moves on ack", async () => {
      const liqOid = oid(90);
      try {
        await ledger.methods
          .liquidateUser(ASSET_SOL, liqOid, LIMIT_TICKS, LAST_VALID_SLOT)
          .accountsPartial({
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
        .liquidateUser(ASSET_SOL, liqOid, LIMIT_TICKS, LAST_VALID_SLOT)
        .accountsPartial({
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
        .ackPhoenixFill(
          liqOid,
          new BN(-LOTS),
          new BN(0),
          new BN(0),
          LIMIT_TICKS,
          new BN(0)
        )
        .accountsPartial({
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
        .accountsPartial({
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
            LIMIT_TICKS,
            LAST_VALID_SLOT,
            new BN(IM_PER_LOT),
            new BN(1_000_000),
            false,
            new BN(startNonce + i)
          )
          .accountsPartial(placeAccounts())
          .signers([user, adapter])
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
            LIMIT_TICKS,
            LAST_VALID_SLOT,
            new BN(IM_PER_LOT),
            new BN(1_000_000),
            false,
            new BN(startNonce + 8)
          )
          .accountsPartial(placeAccounts())
          .signers([user, adapter])
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
        .accountsPartial({
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
        .accountsPartial({
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
        .placeOrder(
          ASSET_SOL,
          new BN(LOTS),
          oid(70),
          LIMIT_TICKS,
          LAST_VALID_SLOT,
          new BN(IM_TEN_LOTS),
          new BN(LOTS * 1_000_000),
          false,
          new BN(0)
        )
        .accountsPartial({
          user: pnlUser.publicKey,
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: pnlLedger,
        })
        .signers([pnlUser, adapter])
        .rpc();
      await ledger.methods
        .ackPhoenixFill(
          oid(70),
          new BN(LOTS),
          new BN(0),
          new BN(OPEN_VWAP),
          LIMIT_TICKS,
          new BN(IM_TEN_LOTS)
        )
        .accountsPartial({
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
        .placeOrder(
          ASSET_SOL,
          new BN(-LOTS),
          oid(71),
          LIMIT_TICKS,
          LAST_VALID_SLOT,
          new BN(0),
          new BN(0),
          true,
          new BN(1)
        )
        .accountsPartial({
          user: pnlUser.publicKey,
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: pnlLedger,
        })
        .signers([pnlUser, adapter])
        .rpc();
      const mid = await ledger.account.userLedger.fetch(pnlLedger);
      const freeWhilePending = mid.free.toNumber();

      await ledger.methods
        .ackPhoenixFail(oid(71), new BN(IM_TEN_LOTS))
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: pnlLedger,
        })
        .signers([adapter])
        .rpc();
      const afterFail = await ledger.account.userLedger.fetch(pnlLedger);
      expect(afterFail.positions[0].lots.toNumber()).to.equal(LOTS);
      expect(afterFail.free.toNumber()).to.equal(CREDIT - IM_TEN_LOTS);
      expect(afterFail.free.toNumber()).to.not.equal(freeWhilePending + PROFIT);

      await ledger.methods
        .placeOrder(
          ASSET_SOL,
          new BN(-LOTS),
          oid(72),
          LIMIT_TICKS,
          LAST_VALID_SLOT,
          new BN(0),
          new BN(0),
          true,
          new BN(2)
        )
        .accountsPartial({
          user: pnlUser.publicKey,
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: pnlLedger,
        })
        .signers([pnlUser, adapter])
        .rpc();
      await ledger.methods
        .ackPhoenixFill(
          oid(72),
          new BN(-LOTS),
          new BN(0),
          new BN(CLOSE_VWAP),
          LIMIT_TICKS,
          new BN(0)
        )
        .accountsPartial({
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
        .placeOrder(
          ASSET_SOL,
          new BN(LOTS),
          oid(80),
          LIMIT_TICKS,
          LAST_VALID_SLOT,
          new BN(IM_TEN_LOTS),
          new BN(LOTS * 1_000_000),
          false,
          new BN(3)
        )
        .accountsPartial({
          user: pnlUser.publicKey,
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: pnlLedger,
        })
        .signers([pnlUser, adapter])
        .rpc();
      await ledger.methods
        .ackPhoenixFill(
          oid(80),
          new BN(LOTS),
          new BN(0),
          new BN(OPEN_VWAP),
          LIMIT_TICKS,
          new BN(IM_TEN_LOTS)
        )
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: pnlLedger,
          book: bookPda,
          feeAccrual: feesPda,
        })
        .signers([adapter])
        .rpc();
      await ledger.methods
        .placeOrder(
          ASSET_SOL,
          new BN(5),
          oid(81),
          LIMIT_TICKS,
          LAST_VALID_SLOT,
          new BN(15 * IM_PER_LOT),
          new BN(15_000_000),
          false,
          new BN(4)
        )
        .accountsPartial({
          user: pnlUser.publicKey,
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: pnlLedger,
        })
        .signers([pnlUser, adapter])
        .rpc();
      await ledger.methods
        .placeOrder(
          ASSET_SOL,
          new BN(-4),
          oid(82),
          LIMIT_TICKS,
          LAST_VALID_SLOT,
          new BN(11 * IM_PER_LOT),
          new BN(11_000_000),
          false,
          new BN(5)
        )
        .accountsPartial({
          user: pnlUser.publicKey,
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: pnlLedger,
        })
        .signers([pnlUser, adapter])
        .rpc();
      const freeBefore = (
        await ledger.account.userLedger.fetch(pnlLedger)
      ).free.toNumber();
      await ledger.methods
        .ackPhoenixFill(
          oid(82),
          new BN(-4),
          new BN(0),
          new BN(-4_400_000),
          LIMIT_TICKS,
          new BN(11 * IM_PER_LOT)
        )
        .accountsPartial({
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

  describe("withdrawals and reserve root", () => {
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
        .accountsPartial({
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
        .accountsPartial({
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
          .accountsPartial({
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

    it("operator-down on either side rejects a new flat withdrawal without debiting cash", async () => {
      const configBefore = (await vault.account.config.fetch(configPda)).paused;
      const bookBefore = (await ledger.account.book.fetch(bookPda)).halt;
      const before = await ledger.account.userLedger.fetch(withdrawLedger);
      try {
        for (const l1Down of [true, false]) {
          await vault.methods.setOperatorDown(l1Down)
            .accountsPartial({ adapter: adapter.publicKey, config: configPda })
            .signers([adapter]).rpc();
          await ledger.methods.setOperatorDown(!l1Down)
            .accountsPartial({ adapter: adapter.publicKey, config: configPda, book: bookPda })
            .signers([adapter]).rpc();
          let rejected: any;
          try {
            await ledger.methods.requestWithdraw(new BN(WITHDRAW))
              .accountsPartial({ user: withdrawUser.publicKey, config: configPda,
                book: bookPda, userLedger: withdrawLedger })
              .signers([withdrawUser]).rpc();
          } catch (error) { rejected = error; }
          expect(rejected, "an unresolved operator gate must block withdrawal").to.exist;
          expect(rejected.error?.errorCode?.code ?? rejected.toString()).to.match(/Halted/);
          const after = await ledger.account.userLedger.fetch(withdrawLedger);
          expect(after.free.toString()).to.equal(before.free.toString());
          expect(after.withdrawable.toString()).to.equal(before.withdrawable.toString());
        }
      } finally {
        await vault.methods.setHalt(configBefore)
          .accountsPartial({ admin: payer.publicKey, config: configPda }).rpc();
        await ledger.methods.setBookHalt(bookBefore)
          .accountsPartial({ adapter: adapter.publicKey, config: configPda, book: bookPda })
          .signers([adapter]).rpc();
      }
    });

    it("flat withdraw credits user ATA and zeros withdrawable", async () => {
      await ledger.methods
        .requestWithdraw(new BN(WITHDRAW))
        .accountsPartial({
          user: withdrawUser.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: withdrawLedger,
        })
        .signers([withdrawUser])
        .rpc();

      await vault.methods
        .userWithdrawL1(new BN(WITHDRAW))
        .accountsPartial({
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
        .accountsPartial({
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
        .writeReserveRootGuarded(
          before.epoch,
          Array.from({ length: 32 }, (_, i) => i),
          1,
          new BN(CREDIT),
          new BN(0),
          new BN(0),
          Array.from({ length: 32 }, () => 1)
        )
        .accountsPartial({
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
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: withdrawLedger,
        })
        .signers([adapter])
        .rpc();

      await vault.methods
        .writeReserveRootGuarded(
          mid.epoch,
          Array.from({ length: 32 }, (_, i) => 32 - i),
          1,
          new BN(CREDIT + 1_000_000 - WITHDRAW),
          new BN(0),
          new BN(0),
          Array.from({ length: 32 }, () => 2)
        )
        .accountsPartial({
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

  it("guarded reserve publication commits debt and rejects an old epoch without mutation", async () => {
    const before = await vault.account.reserveRoot.fetch(reservePda);
    const accounts = { adapter: adapter.publicKey, config: configPda, reserveRoot: reservePda };
    const root = Array(32).fill(43), hash = Array(32).fill(44);
    await vault.methods.writeReserveRootGuarded(before.epoch, root, 2, new BN(12), new BN(13), new BN(14), hash)
      .accountsPartial(accounts).signers([adapter]).rpc();
    const published = await vault.account.reserveRoot.fetch(reservePda);
    expect(published.epoch.toString()).to.equal(before.epoch.add(new BN(1)).toString());
    expect(published.totalBadDebt.toNumber()).to.equal(14);
    try {
      await vault.methods.writeReserveRootGuarded(before.epoch, Array(32).fill(45), 0, new BN(0), new BN(0), new BN(0), hash)
        .accountsPartial(accounts).signers([adapter]).rpc();
      expect.fail("old expected epoch must reject");
    } catch (e: any) { expect(e.error?.errorCode?.code ?? e.toString()).to.match(/ExecutionGuardFailed/); }
    expect(await vault.account.reserveRoot.fetch(reservePda)).to.deep.equal(published);
    try {
      await vault.methods.writeReserveRoot(Array(32).fill(46), 0, new BN(0), new BN(0), hash)
        .accountsPartial(accounts).signers([adapter]).rpc();
      expect.fail("legacy publication must reject even an authorized adapter");
    } catch (e: any) { expect(e.error?.errorCode?.code ?? e.toString()).to.match(/ExecutionGuardFailed/); }
    expect(await vault.account.reserveRoot.fetch(reservePda)).to.deep.equal(published);
  });

  describe("vault PDA and settlement action", () => {
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
        .accountsPartial({
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
        .accountsPartial({
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
        .accountsPartial({
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
          .accountsPartial({
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
        .accountsPartial({
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
        .accountsPartial({
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
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: fundLedger,
        })
        .signers([adapter])
        .rpc();
      await ledger.methods
        .placeOrder(
          ASSET_SOL,
          new BN(LOTS),
          oid(80),
          LIMIT_TICKS,
          LAST_VALID_SLOT,
          new BN(IM_TEN_LOTS),
          new BN(LOTS * 1_000_000),
          false,
          new BN(0)
        )
        .accountsPartial({
          user: fundUser.publicKey,
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: fundLedger,
        })
        .signers([fundUser, adapter])
        .rpc();
      await ledger.methods
        .ackPhoenixFill(
          oid(80),
          new BN(LOTS),
          new BN(0),
          new BN(0),
          LIMIT_TICKS,
          new BN(IM_TEN_LOTS)
        )
        .accountsPartial({
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
      const reservedBefore = before.reserved.toNumber();

      await ledger.methods
        .bumpFundingEpoch(new BN(1))
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
        })
        .signers([adapter])
        .rpc();

      await ledger.methods
        .allocateFunding(new BN(1), false, [
          { assetId: ASSET_SOL, deltaUsdc: new BN(DELTA), postPositionImUsdc: new BN(0) },
        ])
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: fundLedger,
          book: bookPda,
        })
        .signers([adapter])
        .rpc();

      const accrued = await ledger.account.userLedger.fetch(fundLedger);
      expect(accrued.free.toNumber()).to.equal(freeBefore);
      expect(accrued.reserved.toNumber()).to.equal(reservedBefore);
      expect(accrued.positions[0].reservedIm.toString()).to.equal(before.positions[0].reservedIm.toString());
      expect(accrued.positions[0].unsettledFunding.toNumber()).to.equal(DELTA);
      expect(accrued.lastFundingEpoch.toNumber()).to.equal(1);

      try {
        await ledger.methods
          .allocateFunding(new BN(1), false, [])
          .accountsPartial({
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
          .accountsPartial({
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
          .accountsPartial({
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
        .allocateFunding(new BN(1), true, [
          {
            assetId: ASSET_SOL,
            deltaUsdc: new BN(0),
            postPositionImUsdc: new BN(2_000_000),
          },
        ])
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: fundLedger,
          book: bookPda,
        })
        .signers([adapter])
        .rpc();

      const folded = await ledger.account.userLedger.fetch(fundLedger);
      expect(folded.free.toNumber()).to.equal(
        freeBefore + reservedBefore + DELTA - 2_000_000
      );
      expect(folded.reserved.toNumber()).to.equal(2_000_000);
      expect(folded.positions[0].unsettledFunding.toNumber()).to.equal(0);
    });
  });

  describe("total fill acknowledgement", () => {
    const marginUser = Keypair.generate();
    const debtUser = Keypair.generate();
    const replayUser = Keypair.generate();
    let marginLedger: PublicKey;
    let debtLedger: PublicKey;
    let replayLedger: PublicKey;
    const STARTING_CASH = 3_000_000;
    const OPEN_VWAP = 10_000_000;
    const LOSS_CLOSE_VWAP = -1_000_000;
    const FEE = 500_000;
    const EXPECTED_DEBT = 6_500_000;

    before(async () => {
      await airdrop(marginUser.publicKey);
      await airdrop(debtUser.publicKey);
      await airdrop(replayUser.publicKey);
      marginLedger = pda(ledger.programId, [
        Buffer.from("user"),
        marginUser.publicKey.toBuffer(),
      ]);
      debtLedger = pda(ledger.programId, [
        Buffer.from("user"),
        debtUser.publicKey.toBuffer(),
      ]);
      replayLedger = pda(ledger.programId, [
        Buffer.from("user"),
        replayUser.publicKey.toBuffer(),
      ]);
      await ledger.methods
        .initUser()
        .accountsPartial({
          adapter: adapter.publicKey,
          user: marginUser.publicKey,
          config: configPda,
          userLedger: marginLedger,
          book: bookPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([adapter, marginUser])
        .rpc();
      await ledger.methods
        .initUser()
        .accountsPartial({
          adapter: adapter.publicKey,
          user: debtUser.publicKey,
          config: configPda,
          userLedger: debtLedger,
          book: bookPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([adapter, debtUser])
        .rpc();
      await ledger.methods
        .initUser()
        .accountsPartial({
          adapter: adapter.publicKey,
          user: replayUser.publicKey,
          config: configPda,
          userLedger: replayLedger,
          book: bookPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([adapter, replayUser])
        .rpc();
      await ledger.methods
        .creditDeposit(new BN(STARTING_CASH))
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: debtLedger,
        })
        .signers([adapter])
        .rpc();
      await ledger.methods
        .creditDeposit(new BN(10_000_000))
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: replayLedger,
        })
        .signers([adapter])
        .rpc();
    });

    it("acknowledges a fill when the post-fill margin target cannot be met", async () => {
      await ledger.methods
        .creditDeposit(new BN(2_000_000))
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: marginLedger,
        })
        .signers([adapter])
        .rpc();
      await ledger.methods
        .placeOrder(
          ASSET_SOL,
          new BN(LOTS),
          oid(118),
          LIMIT_TICKS,
          LAST_VALID_SLOT,
          new BN(IM_TEN_LOTS),
          new BN(LOTS * 1_000_000),
          false,
          new BN(0)
        )
        .accountsPartial({
          user: marginUser.publicKey,
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: marginLedger,
        })
        .signers([marginUser, adapter])
        .rpc();
      await ledger.methods
        .ackPhoenixFill(
          oid(118),
          new BN(LOTS),
          new BN(1_000_000),
          new BN(10_000_000),
          LIMIT_TICKS,
          new BN(IM_TEN_LOTS)
        )
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: marginLedger,
          book: bookPda,
          feeAccrual: feesPda,
        })
        .signers([adapter])
        .rpc();

      const state = await ledger.account.userLedger.fetch(marginLedger);
      const book = await ledger.account.book.fetch(bookPda);
      expect(state.badDebtUsdc.toNumber()).to.equal(0);
      expect(state.free.toNumber()).to.equal(0);
      expect(state.reserved.toNumber()).to.equal(1_000_000);
      expect(state.positions[0].reservedIm.toNumber()).to.equal(IM_TEN_LOTS);
      expect(book.halt & HALT_ENTRIES).to.equal(HALT_ENTRIES);

      await ledger.methods
        .setBookHalt(0)
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
        })
        .signers([adapter])
        .rpc();
    });

    it("halts entries when a failed close restores an unfunded margin target", async () => {
      const user = Keypair.generate();
      await airdrop(user.publicKey);
      const userLedger = pda(ledger.programId, [
        Buffer.from("user"),
        user.publicKey.toBuffer(),
      ]);
      const accounts = {
        adapter: adapter.publicKey,
        config: configPda,
        book: bookPda,
        userLedger,
      };
      await ledger.methods
        .initUser()
        .accountsPartial({ ...accounts, user: user.publicKey, systemProgram: SystemProgram.programId })
        .signers([adapter, user])
        .rpc();
      await ledger.methods
        .creditDeposit(new BN(2_000_000))
        .accountsPartial(accounts)
        .signers([adapter])
        .rpc();
      const place = (lots: number, tag: number, postIm: number, nonce: number) =>
        ledger.methods
          .placeOrder(
            ASSET_SOL,
            new BN(lots),
            oid(tag),
            LIMIT_TICKS,
            LAST_VALID_SLOT,
            new BN(postIm),
            new BN(lots > 0 ? OPEN_VWAP : 0),
            lots < 0,
            new BN(nonce)
          )
          .accountsPartial({ ...accounts, user: user.publicKey })
          .signers([adapter, user])
          .rpc();
      await place(LOTS, 140, IM_TEN_LOTS, 0);
      await ledger.methods
        .ackPhoenixFill(
          oid(140), new BN(LOTS), new BN(0), new BN(OPEN_VWAP),
          LIMIT_TICKS, new BN(IM_TEN_LOTS)
        )
        .accountsPartial({ ...accounts, feeAccrual: feesPda })
        .signers([adapter])
        .rpc();
      await place(-LOTS, 141, 0, 1);
      const bookBefore = await ledger.account.book.fetch(bookPda);

      // Fresh venue risk can require more margin than the remaining cash.
      await ledger.methods
        .ackPhoenixFail(oid(141), new BN(3_000_000))
        .accountsPartial(accounts)
        .signers([adapter])
        .rpc();

      const state = await ledger.account.userLedger.fetch(userLedger);
      const bookAfter = await ledger.account.book.fetch(bookPda);
      expect(state.positions[0].lots.toNumber()).to.equal(LOTS);
      expect(state.positions[0].reservedIm.toNumber()).to.equal(3_000_000);
      expect(state.reserved.toNumber()).to.equal(2_000_000);
      expect(state.free.toNumber()).to.equal(0);
      expect(state.badDebtUsdc.toNumber()).to.equal(0);
      expect(state.pendingOidCount).to.equal(0);
      expect(state.openOids.find((row: { clientOid: number[] }) =>
        row.clientOid.every((byte: number, i: number) => byte === oid(141)[i])
      )?.state).to.equal(2); // OID_FAILED
      expect(bookAfter.halt & HALT_ENTRIES).to.equal(HALT_ENTRIES);
      expect(bookAfter.residualLen).to.equal(bookBefore.residualLen);
      expect(bookAfter.residuals.map((row: { lots: BN }) => row.lots.toString()))
        .to.deep.equal(bookBefore.residuals.map((row: { lots: BN }) => row.lots.toString()));

      await ledger.methods
        .setBookHalt(0)
        .accountsPartial({ adapter: adapter.publicKey, config: configPda, book: bookPda })
        .signers([adapter])
        .rpc();
    });

    it("prioritizes a live reused OID over an older acknowledged row", async () => {
      const blockerOid = oid(130);
      const reusedOid = oid(131);
      const place = (clientOid: number[], nonce: number) =>
        ledger.methods
          .placeOrder(
            ASSET_SOL,
            new BN(1),
            clientOid,
            LIMIT_TICKS,
            LAST_VALID_SLOT,
            new BN(IM_PER_LOT),
            new BN(1_000_000),
            false,
            new BN(nonce)
          )
          .accountsPartial({
            user: replayUser.publicKey,
            adapter: adapter.publicKey,
            config: configPda,
            book: bookPda,
            userLedger: replayLedger,
          })
          .signers([replayUser, adapter])
          .rpc();
      const acknowledge = (clientOid: number[], postIm: number) =>
        ledger.methods
          .ackPhoenixFill(
            clientOid,
            new BN(1),
            new BN(0),
            new BN(1_000_000),
            LIMIT_TICKS,
            new BN(postIm)
          )
          .accountsPartial({
            adapter: adapter.publicKey,
            config: configPda,
            userLedger: replayLedger,
            book: bookPda,
            feeAccrual: feesPda,
          })
          .signers([adapter])
          .rpc();

      await place(blockerOid, 0);
      await place(reusedOid, 1);
      await acknowledge(reusedOid, 2 * IM_PER_LOT);
      await ledger.methods
        .ackPhoenixFail(blockerOid, new BN(0))
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: replayLedger,
        })
        .signers([adapter])
        .rpc();

      await place(reusedOid, 2);
      const bookBefore = await ledger.account.book.fetch(bookPda);
      const lotsBefore = bookBefore.residuals
        .slice(0, bookBefore.residualLen)
        .find((row: { assetId: number }) => row.assetId === ASSET_SOL)
        ?.lots.toNumber() ?? 0;
      await acknowledge(reusedOid, 2 * IM_PER_LOT);

      const state = await ledger.account.userLedger.fetch(replayLedger);
      const bookAfter = await ledger.account.book.fetch(bookPda);
      const lotsAfter = bookAfter.residuals
        .slice(0, bookAfter.residualLen)
        .find((row: { assetId: number }) => row.assetId === ASSET_SOL)
        ?.lots.toNumber() ?? 0;
      expect(state.pendingOidCount).to.equal(0);
      expect(state.positions[0].lots.toNumber()).to.equal(2);
      expect(lotsAfter).to.equal(lotsBefore + 1);
    });

    it("records an unaffordable confirmed loss and fee exactly once", async () => {
      const bookBefore = await ledger.account.book.fetch(bookPda);
      const lotsBefore = bookBefore.residuals
        .slice(0, bookBefore.residualLen)
        .find((row: { assetId: number }) => row.assetId === ASSET_SOL)
        ?.lots.toNumber() ?? 0;
      const feesBefore = await ledger.account.feeAccrual.fetch(feesPda);

      await ledger.methods
        .placeOrder(
          ASSET_SOL,
          new BN(LOTS),
          oid(120),
          LIMIT_TICKS,
          LAST_VALID_SLOT,
          new BN(IM_TEN_LOTS),
          new BN(LOTS * 1_000_000),
          false,
          new BN(0)
        )
        .accountsPartial({
          user: debtUser.publicKey,
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: debtLedger,
        })
        .signers([debtUser, adapter])
        .rpc();
      await ledger.methods
        .ackPhoenixFill(
          oid(120),
          new BN(LOTS),
          new BN(0),
          new BN(OPEN_VWAP),
          LIMIT_TICKS,
          new BN(IM_TEN_LOTS)
        )
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: debtLedger,
          book: bookPda,
          feeAccrual: feesPda,
        })
        .signers([adapter])
        .rpc();
      await ledger.methods
        .placeOrder(
          ASSET_SOL,
          new BN(-LOTS),
          oid(121),
          LIMIT_TICKS,
          LAST_VALID_SLOT,
          new BN(0),
          new BN(0),
          true,
          new BN(1)
        )
        .accountsPartial({
          user: debtUser.publicKey,
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: debtLedger,
        })
        .signers([debtUser, adapter])
        .rpc();

      const acknowledgeLoss = () =>
        ledger.methods
          .ackPhoenixFill(
            oid(121),
            new BN(-LOTS),
            new BN(FEE),
            new BN(LOSS_CLOSE_VWAP),
            LIMIT_TICKS,
            new BN(0)
          )
          .accountsPartial({
            adapter: adapter.publicKey,
            config: configPda,
            userLedger: debtLedger,
            book: bookPda,
            feeAccrual: feesPda,
          })
          .signers([adapter])
          .rpc();

      await acknowledgeLoss();
      const after = await ledger.account.userLedger.fetch(debtLedger);
      const bookAfter = await ledger.account.book.fetch(bookPda);
      const feesAfter = await ledger.account.feeAccrual.fetch(feesPda);
      const lotsAfter = bookAfter.residuals
        .slice(0, bookAfter.residualLen)
        .find((row: { assetId: number }) => row.assetId === ASSET_SOL)
        ?.lots.toNumber() ?? 0;

      expect(after.positionsLen).to.equal(0);
      expect(after.free.toNumber()).to.equal(0);
      expect(after.reserved.toNumber()).to.equal(0);
      expect(after.badDebtUsdc.toNumber()).to.equal(EXPECTED_DEBT);
      expect(after.pendingOidCount).to.equal(0);
      expect(lotsAfter).to.equal(lotsBefore);
      expect(bookAfter.invariantOk).to.equal(1);
      expect(bookAfter.halt & (1 << 6)).to.equal(1 << 6);
      expect(bookAfter.halt & HALT_ENTRIES).to.equal(HALT_ENTRIES);
      expect(bookAfter.halt & (1 << 1)).to.equal(1 << 1);
      expect(feesAfter.phoenixFeesPaid.toNumber()).to.equal(
        feesBefore.phoenixFeesPaid.toNumber() + FEE
      );

      await acknowledgeLoss();
      const replayed = await ledger.account.userLedger.fetch(debtLedger);
      const replayedBook = await ledger.account.book.fetch(bookPda);
      const replayedFees = await ledger.account.feeAccrual.fetch(feesPda);
      expect(replayed.badDebtUsdc.toNumber()).to.equal(EXPECTED_DEBT);
      expect(replayedBook.residuals).to.deep.equal(bookAfter.residuals);
      expect(replayedFees.phoenixFeesPaid.toNumber()).to.equal(
        feesAfter.phoenixFeesPaid.toNumber()
      );
    });

    it("uses later deposits to retire debt before restoring free cash", async () => {
      await ledger.methods
        .creditDeposit(new BN(6_000_000))
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: debtLedger,
        })
        .signers([adapter])
        .rpc();
      let state = await ledger.account.userLedger.fetch(debtLedger);
      expect(state.badDebtUsdc.toNumber()).to.equal(500_000);
      expect(state.free.toNumber()).to.equal(0);

      await ledger.methods
        .creditDeposit(new BN(1_000_000))
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: debtLedger,
        })
        .signers([adapter])
        .rpc();
      state = await ledger.account.userLedger.fetch(debtLedger);
      expect(state.badDebtUsdc.toNumber()).to.equal(0);
      expect(state.free.toNumber()).to.equal(500_000);
    });

    it("signals debt created while liquidating an already-flat position", async () => {
      await vault.methods
        .setAllowlist([ASSET_SOL, ASSET_BTC])
        .accountsPartial({ admin: payer.publicKey, config: configPda })
        .rpc();
      await ledger.methods
        .setBookHalt(0)
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
        })
        .signers([adapter])
        .rpc();

      const secondAssetOid = oid(132);
      await ledger.methods
        .placeOrder(
          ASSET_BTC,
          new BN(LOTS),
          secondAssetOid,
          LIMIT_TICKS,
          LAST_VALID_SLOT,
          new BN(IM_TEN_LOTS),
          new BN(LOTS * 1_000_000),
          false,
          new BN(3)
        )
        .accountsPartial({
          user: replayUser.publicKey,
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: replayLedger,
        })
        .signers([replayUser, adapter])
        .rpc();
      await ledger.methods
        .ackPhoenixFill(
          secondAssetOid,
          new BN(LOTS),
          new BN(0),
          new BN(10_000_000),
          LIMIT_TICKS,
          new BN(IM_TEN_LOTS)
        )
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: replayLedger,
          book: bookPda,
          feeAccrual: feesPda,
        })
        .signers([adapter])
        .rpc();

      const bookBefore = await ledger.account.book.fetch(bookPda);
      const nextEpoch = bookBefore.fundingEpoch.toNumber() + 1;
      await ledger.methods
        .bumpFundingEpoch(new BN(nextEpoch))
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
        })
        .signers([adapter])
        .rpc();
      await ledger.methods
        .allocateFunding(new BN(nextEpoch), false, [
          { assetId: ASSET_SOL, deltaUsdc: new BN(-11_000_000), postPositionImUsdc: new BN(0) },
        ])
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: replayLedger,
          book: bookPda,
        })
        .signers([adapter])
        .rpc();

      const closeOid = oid(133);
      await ledger.methods
        .placeOrder(
          ASSET_SOL,
          new BN(-2),
          closeOid,
          LIMIT_TICKS,
          LAST_VALID_SLOT,
          new BN(0),
          new BN(0),
          true,
          new BN(4)
        )
        .accountsPartial({
          user: replayUser.publicKey,
          adapter: adapter.publicKey,
          config: configPda,
          book: bookPda,
          userLedger: replayLedger,
        })
        .signers([replayUser, adapter])
        .rpc();
      await ledger.methods
        .ackPhoenixFill(
          closeOid,
          new BN(-2),
          new BN(0),
          new BN(-2_000_000),
          LIMIT_TICKS,
          new BN(0)
        )
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: replayLedger,
          book: bookPda,
          feeAccrual: feesPda,
        })
        .signers([adapter])
        .rpc();

      await ledger.methods
        .liquidateUser(ASSET_SOL, oid(134), LIMIT_TICKS, LAST_VALID_SLOT)
        .accountsPartial({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger: replayLedger,
          book: bookPda,
        })
        .signers([adapter])
        .rpc();

      const state = await ledger.account.userLedger.fetch(replayLedger);
      const bookAfter = await ledger.account.book.fetch(bookPda);
      expect(state.positionsLen).to.equal(1);
      const remaining = state.positions
        .slice(0, state.positionsLen)
        .find((position: { assetId: number }) => position.assetId === ASSET_BTC);
      expect(remaining).to.exist;
      expect(remaining.lots.toNumber()).to.equal(LOTS);
      expect(remaining.reservedIm.toNumber()).to.equal(IM_TEN_LOTS);
      expect(state.badDebtUsdc.toNumber()).to.equal(1_000_000);
      expect(bookAfter.halt & (1 << 6)).to.equal(1 << 6);
      expect(bookAfter.halt & HALT_ENTRIES).to.equal(HALT_ENTRIES);
      expect(bookAfter.halt & (1 << 1)).to.equal(1 << 1);
    });
  });
});
