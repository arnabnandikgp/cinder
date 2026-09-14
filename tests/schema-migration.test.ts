import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import { readFileSync } from "fs";
import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import {
  EPHEMERAL_VAULT_ID,
  getAuthToken,
  MAGIC_PROGRAM_ID,
  PERMISSION_PROGRAM_ID,
  permissionPdaFromAccount,
} from "@magicblock-labs/ephemeral-rollups-sdk";
import { expect } from "chai";
import nacl from "tweetnacl";
import { CinderLedger } from "../target/types/cinder_ledger";
import { CinderVault } from "../target/types/cinder_vault";

const ER_VALIDATOR = new PublicKey(
  "mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev"
);
const ER = process.env.EPHEMERAL_PROVIDER_ENDPOINT || "http://127.0.0.1:7799";
const QFS = process.env.TEE_PROVIDER_ENDPOINT || "http://127.0.0.1:6699";
const QFS_WS = process.env.TEE_WS_ENDPOINT || "ws://127.0.0.1:6700";

type Manifest = {
  adapter: string;
  user: string;
  userKeypairPath: string;
  config: string;
  reserveRoot: string;
  book: string;
  userLedger: string;
};

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitForAccount(
  connection: Connection,
  address: PublicKey,
  owner: PublicKey
) {
  for (let attempt = 0; attempt < 40; attempt++) {
    const info = await connection.getAccountInfo(address);
    if (info?.owner.equals(owner)) return info;
    await sleep(500);
  }
  throw new Error(`account ${address.toBase58()} did not appear on ER`);
}

async function tokenFor(connection: string, signer: Keypair) {
  return getAuthToken(connection, signer.publicKey, (message: Uint8Array) =>
    Promise.resolve(nacl.sign.detached(message, signer.secretKey))
  );
}

function qfsConnection(token: string) {
  return new Connection(`${QFS}?token=${token}`, {
    wsEndpoint: `${QFS_WS}?token=${token}`,
    commitment: "confirmed",
  });
}

async function privateRead(connection: Connection, address: PublicKey) {
  return connection.getAccountInfo(address);
}

describe("v0 schema migration on delegated accounts", function () {
  this.timeout(180_000);

  const manifestPath = process.env.CINDER_MIGRATION_MANIFEST;
  if (!manifestPath) {
    it("requires the migration harness", function () {
      this.skip();
    });
    return;
  }

  const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as Manifest;
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const vault = (anchor.workspace as any).cinderVault as Program<CinderVault>;
  const ledger = (anchor.workspace as any).cinderLedger as Program<CinderLedger>;
  const adapter = (provider.wallet as anchor.Wallet).payer;
  const user = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(readFileSync(manifest.userKeypairPath, "utf8")))
  );
  const erConnection = new Connection(ER, "confirmed");

  const config = new PublicKey(manifest.config);
  const reserveRoot = new PublicKey(manifest.reserveRoot);
  const book = new PublicKey(manifest.book);
  const userLedger = new PublicKey(manifest.userLedger);

  it("preserves state, rejects replay, and restores private reads", async () => {
    expect(adapter.publicKey.toBase58()).to.equal(manifest.adapter);
    expect(user.publicKey.toBase58()).to.equal(manifest.user);

    const sendBase = async (builder: any, signers: Keypair[] = []) => {
      const transaction = await builder.transaction();
      return provider.sendAndConfirm(transaction, signers, {
        commitment: "confirmed",
        skipPreflight: true,
      });
    };

    await sendBase(
      ledger.methods.delegateBook().accountsPartial({
        adapter: adapter.publicKey,
        config,
        book,
        validator: ER_VALIDATOR,
      })
    );
    await sendBase(
      ledger.methods.delegateUser().accountsPartial({
        adapter: adapter.publicKey,
        user: user.publicKey,
        config,
        userLedger,
        validator: ER_VALIDATOR,
      }),
      [user]
    );

    const legacyBook = await waitForAccount(erConnection, book, ledger.programId);
    const legacyUser = await waitForAccount(erConnection, userLedger, ledger.programId);
    expect(legacyBook.data.length).to.equal(364);
    expect(legacyUser.data.length).to.equal(843);

    const erProvider = new anchor.AnchorProvider(
      erConnection,
      new anchor.Wallet(adapter),
      { commitment: "confirmed" }
    );
    const erLedger = new Program(
      ledger.idl as CinderLedger,
      erProvider
    ) as Program<CinderLedger>;

    await erLedger.methods
      .migrateBook()
      .accountsPartial({ adapter: adapter.publicKey, config, book })
      .rpc();
    await erLedger.methods
      .migrateUserLedger()
      .accountsPartial({
        adapter: adapter.publicKey,
        config,
        userLedger,
      })
      .rpc();
    await vault.methods
      .migrateReserveRoot()
      .accountsPartial({
        admin: adapter.publicKey,
        config,
        reserveRoot,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const migratedBook = await erLedger.account.book.fetch(book);
    const migratedUser = await erLedger.account.userLedger.fetch(userLedger);
    const migratedRoot = await vault.account.reserveRoot.fetch(reserveRoot);
    expect(migratedBook.schemaVersion).to.equal(1);
    expect(migratedBook.residuals[0].lots.toNumber()).to.equal(10);
    expect(migratedUser.schemaVersion).to.equal(1);
    expect(migratedUser.free.toNumber()).to.equal(98_750_000);
    expect(migratedUser.reserved.toNumber()).to.equal(1_250_000);
    expect(migratedUser.positions[0].lots.toNumber()).to.equal(10);
    expect(migratedUser.badDebtUsdc.toNumber()).to.equal(0);
    expect(migratedRoot.schemaVersion).to.equal(1);
    expect(migratedRoot.totalFree.toNumber()).to.equal(98_750_000);
    expect(migratedRoot.totalReserved.toNumber()).to.equal(1_250_000);
    expect(migratedRoot.totalBadDebt.toNumber()).to.equal(0);
    expect((await erConnection.getAccountInfo(book))?.data.length).to.equal(365);
    expect((await erConnection.getAccountInfo(userLedger))?.data.length).to.equal(980);
    expect((await provider.connection.getAccountInfo(reserveRoot))?.data.length).to.equal(117);

    try {
      await erLedger.methods
        .migrateUserLedger()
        .accountsPartial({ adapter: adapter.publicKey, config, userLedger })
        .rpc();
      expect.fail("second migration should fail");
    } catch (error: any) {
      const code = error.error?.errorCode?.code ?? error.toString();
      expect(code).to.match(/AccountAlreadyMigrated|already been migrated/i);
    }

    const permissionAccounts = (account: PublicKey) => ({
      adapter: adapter.publicKey,
      config,
      permission: permissionPdaFromAccount(account),
      magicProgram: MAGIC_PROGRAM_ID,
      permissionProgram: PERMISSION_PROGRAM_ID,
      ephemeralVault: EPHEMERAL_VAULT_ID,
    });
    await erLedger.methods
      .initBookPermission()
      .accountsPartial({ ...permissionAccounts(book), book })
      .rpc();
    await erLedger.methods
      .initUserPermission()
      .accountsPartial({ ...permissionAccounts(userLedger), userLedger })
      .rpc();

    const outsider = Keypair.generate();
    const [adapterToken, userToken, outsiderToken] = await Promise.all([
      tokenFor(QFS, adapter),
      tokenFor(QFS, user),
      tokenFor(QFS, outsider),
    ]);
    const adapterQfs = qfsConnection(adapterToken.token);
    const userQfs = qfsConnection(userToken.token);
    const outsiderQfs = qfsConnection(outsiderToken.token);

    expect(await privateRead(adapterQfs, userLedger)).to.not.equal(null);
    expect(await privateRead(userQfs, userLedger)).to.not.equal(null);
    expect(await privateRead(outsiderQfs, userLedger)).to.equal(null);
    expect(await privateRead(adapterQfs, book)).to.not.equal(null);
    expect(await privateRead(userQfs, book)).to.equal(null);
  });
});
