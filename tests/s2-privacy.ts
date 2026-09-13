import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  createMint,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import { expect } from "chai";
import nacl from "tweetnacl";
import {
  EPHEMERAL_VAULT_ID,
  getAuthToken,
  MAGIC_PROGRAM_ID,
  PERMISSION_PROGRAM_ID,
  permissionPdaFromAccount,
} from "@magicblock-labs/ephemeral-rollups-sdk";
import { CinderVault } from "../target/types/cinder_vault";
import { CinderLedger } from "../target/types/cinder_ledger";

const ER_VALIDATOR = new PublicKey(
  "mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev"
);
const EPHEMERAL_VAULT = EPHEMERAL_VAULT_ID;
const BASE = process.env.PROVIDER_ENDPOINT || "http://127.0.0.1:8899";
const ER = process.env.EPHEMERAL_PROVIDER_ENDPOINT || "http://127.0.0.1:7799";
const QFS = process.env.TEE_PROVIDER_ENDPOINT || "http://127.0.0.1:6699";
const QFS_WS = process.env.TEE_WS_ENDPOINT || "ws://127.0.0.1:6700";

function pda(programId: PublicKey, seeds: (Buffer | Uint8Array)[]): PublicKey {
  return PublicKey.findProgramAddressSync(seeds, programId)[0];
}

function sleep(ms: number) {
  return new Promise((r) => setTimeout(r, ms));
}

async function rpcUp(url: string): Promise<boolean> {
  try {
    await new Connection(url, "confirmed").getVersion();
    return true;
  } catch {
    return false;
  }
}

async function airdrop(connection: Connection, pk: PublicKey) {
  const sig = await connection.requestAirdrop(
    pk,
    2 * anchor.web3.LAMPORTS_PER_SOL
  );
  await connection.confirmTransaction(sig, "confirmed");
}

async function authToken(payer: Keypair) {
  return getAuthToken(QFS, payer.publicKey, (message: Uint8Array) =>
    Promise.resolve(nacl.sign.detached(message, payer.secretKey))
  );
}

function qfsConnection(token: string) {
  return new Connection(`${QFS}?token=${token}`, {
    wsEndpoint: `${QFS_WS}?token=${token}`,
    commitment: "confirmed",
  });
}

async function waitErOwner(
  er: Connection,
  address: PublicKey,
  owner: PublicKey,
  label: string
) {
  for (let i = 0; i < 40; i++) {
    const info = await er.getAccountInfo(address);
    if (info && info.owner.equals(owner)) return info;
    await sleep(500);
  }
  throw new Error(`${label} did not appear on ER owned by ${owner.toBase58()}`);
}

describe("S2 PER / QFS isolation", function () {
  this.timeout(120_000);

  let skip = false;
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const vault = (anchor.workspace as any).cinderVault as Program<CinderVault>;
  const ledger = (anchor.workspace as any).cinderLedger as Program<CinderLedger>;
  const payer = (provider.wallet as anchor.Wallet).payer;
  const base = provider.connection;
  const erConn = new Connection(ER, "confirmed");

  const adapter = Keypair.generate();
  const userA = Keypair.generate();
  const userB = Keypair.generate();
  const phoenixTrader = Keypair.generate().publicKey;

  let usdcMint: PublicKey;
  let configPda: PublicKey;
  let vaultAuth: PublicKey;
  let reservePda: PublicKey;
  let vaultAta: PublicKey;
  let bookPda: PublicKey;
  let feesPda: PublicKey;
  let ledgerA: PublicKey;

  before(async function () {
    if (!(await rpcUp(QFS)) || !(await rpcUp(ER)) || !(await rpcUp(BASE))) {
      if (process.env.CINDER_REQUIRE_STACK === "1") {
        throw new Error(
          `required mb-stack not up (QFS ${QFS}, ER ${ER}, base ${BASE})`
        );
      }
      skip = true;
      this.skip();
    }

    await airdrop(base, adapter.publicKey);
    await airdrop(base, userA.publicKey);
    await airdrop(base, userB.publicKey);

    usdcMint = await createMint(base, payer, payer.publicKey, null, 6);
    configPda = pda(vault.programId, [Buffer.from("config")]);
    vaultAuth = pda(vault.programId, [Buffer.from("vault-authority")]);
    reservePda = pda(vault.programId, [Buffer.from("reserve")]);
    bookPda = pda(ledger.programId, [Buffer.from("book")]);
    feesPda = pda(ledger.programId, [Buffer.from("fees")]);
    ledgerA = pda(ledger.programId, [
      Buffer.from("user"),
      userA.publicKey.toBuffer(),
    ]);
    vaultAta = getAssociatedTokenAddressSync(usdcMint, vaultAuth, true);
  });

  it("inits, delegates, and sets ER permissions", async function () {
    if (skip) this.skip();

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

    await ledger.methods
      .initUser()
      .accountsPartial({
        adapter: adapter.publicKey,
        user: userA.publicKey,
        config: configPda,
        userLedger: ledgerA,
        book: bookPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([adapter, userA])
      .rpc();

    const sendBase = async (builder: any, signers: Keypair[]) => {
      const tx = await builder.transaction();
      return provider.sendAndConfirm(tx, signers, {
        skipPreflight: true,
        commitment: "confirmed",
      });
    };

    await sendBase(
      ledger.methods.delegateBook().accountsPartial({
        adapter: adapter.publicKey,
        config: configPda,
        book: bookPda,
        validator: ER_VALIDATOR,
      }),
      [adapter]
    );
    await sendBase(
      ledger.methods.delegateFees().accountsPartial({
        adapter: adapter.publicKey,
        config: configPda,
        feeAccrual: feesPda,
        validator: ER_VALIDATOR,
      }),
      [adapter]
    );
    await sendBase(
      ledger.methods.delegateUser().accountsPartial({
        adapter: adapter.publicKey,
        user: userA.publicKey,
        config: configPda,
        userLedger: ledgerA,
        validator: ER_VALIDATOR,
      }),
      [adapter, userA]
    );

    await waitErOwner(erConn, bookPda, ledger.programId, "book");
    await waitErOwner(erConn, feesPda, ledger.programId, "fees");
    await waitErOwner(erConn, ledgerA, ledger.programId, "user A ledger");

    const adapterWallet = new anchor.Wallet(adapter);
    const adapterToken = await authToken(adapter);
    const qfsAdapter = qfsConnection(adapterToken.token);
    const qfsProvider = new anchor.AnchorProvider(qfsAdapter, adapterWallet, {
      commitment: "confirmed",
    });
    const erProgram = new Program(
      ledger.idl as CinderLedger,
      qfsProvider
    ) as Program<CinderLedger>;

    const sendEr = async (builder: any) => {
      let tx = await builder.transaction();
      tx.feePayer = adapter.publicKey;
      tx.recentBlockhash = (await qfsAdapter.getLatestBlockhash()).blockhash;
      tx = await adapterWallet.signTransaction(tx);
      return qfsProvider.sendAndConfirm(tx, [], { skipPreflight: true });
    };

    const permAccounts = (account: PublicKey) => ({
      adapter: adapter.publicKey,
      config: configPda,
      permission: permissionPdaFromAccount(account),
      magicProgram: MAGIC_PROGRAM_ID,
      permissionProgram: PERMISSION_PROGRAM_ID,
      ephemeralVault: EPHEMERAL_VAULT,
    });

    await sendEr(
      erProgram.methods.initBookPermission().accountsPartial({
        ...permAccounts(bookPda),
        book: bookPda,
      })
    );
    await sendEr(
      erProgram.methods.initFeesPermission().accountsPartial({
        ...permAccounts(feesPda),
        feeAccrual: feesPda,
      })
    );
    await sendEr(
      erProgram.methods.initUserPermission().accountsPartial({
        ...permAccounts(ledgerA),
        userLedger: ledgerA,
      })
    );
  });

  it("A token reads A on QFS :6699", async function () {
    if (skip) this.skip();
    const tok = await authToken(userA);
    const conn = qfsConnection(tok.token);
    const info = await conn.getAccountInfo(ledgerA);
    expect(info, "A should read A's ledger on QFS").to.not.equal(null);
    expect(info!.data.length).to.be.greaterThan(0);
  });

  it("B token cannot getAccountInfo A's ledger on QFS", async function () {
    if (skip) this.skip();
    const tok = await authToken(userB);
    const conn = qfsConnection(tok.token);
    let denied = false;
    try {
      const info = await conn.getAccountInfo(ledgerA);
      denied = info === null;
    } catch {
      denied = true;
    }
    expect(denied, "B must not read A's ledger on QFS").to.equal(true);
  });

  it("raw ER :7799 serves A's ledger without a QFS token", async function () {
    if (skip) this.skip();
    const info = await erConn.getAccountInfo(ledgerA);
    expect(info, "raw ER must still expose the account (filter is QFS)").to.not
      .equal(null);
    expect(info!.owner.equals(ledger.programId)).to.equal(true);
  });

  it("adapter token reads A and Book on QFS", async function () {
    if (skip) this.skip();
    const tok = await authToken(adapter);
    const conn = qfsConnection(tok.token);
    const a = await conn.getAccountInfo(ledgerA);
    const book = await conn.getAccountInfo(bookPda);
    expect(a, "adapter should read A's ledger").to.not.equal(null);
    expect(book, "adapter should read Book").to.not.equal(null);
  });
});
