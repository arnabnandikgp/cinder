/**
 * Filmable walkthrough on mb-stack + mocked Phoenix residual (dummy ack).
 *
 *   ./scripts/stack-local.sh          # other terminal
 *   ./scripts/cinder-demo.sh
 *
 * Prints QFS isolation, then Alice / Bob / Book before and after a close with PnL.
 */
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { spawnSync } from "child_process";
import * as anchor from "@anchor-lang/core";
import { BN, Program } from "@anchor-lang/core";
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
  Transaction,
} from "@solana/web3.js";
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
const BASE = process.env.PROVIDER_ENDPOINT || "http://127.0.0.1:8899";
const BASE_WS = process.env.WS_ENDPOINT || "ws://127.0.0.1:8900";
const ER = process.env.EPHEMERAL_PROVIDER_ENDPOINT || "http://127.0.0.1:7799";
const ER_WS = process.env.EPHEMERAL_WS_ENDPOINT || "ws://127.0.0.1:7800";
const QFS = process.env.TEE_PROVIDER_ENDPOINT || "http://127.0.0.1:6699";
const QFS_WS = process.env.TEE_WS_ENDPOINT || "ws://127.0.0.1:6700";
const RPC_OPTS = { skipPreflight: true, commitment: "confirmed" as const };
const ASSET_SOL = 1;
const CREDIT = 100_000_000;
const LOTS = 10;
const IM_TEN_LOTS = 1_250_000;
const OPEN_VWAP = 10_000_000;
const CLOSE_VWAP = -11_000_000;
const STUB_MM = 625_000; // stub_cinder_mm(10)

async function ensurePrograms(
  conn: Connection,
  programs: [string, PublicKey][]
) {
  const missing = [];
  for (const [label, id] of programs) {
    const info = await conn.getAccountInfo(id);
    if (!info) missing.push(`${label} ${id.toBase58()}`);
  }
  const ours = missing.filter(
    (m) => m.startsWith("cinder_vault") || m.startsWith("cinder_ledger")
  );
  if (ours.length) {
    console.log("Cinder programs not on :8899 (stack was reset?). Deploying...");
    const r = spawnSync(
      "anchor",
      ["deploy", "--provider.cluster", "localnet"],
      { stdio: "inherit", env: process.env }
    );
    if (r.status !== 0) {
      throw new Error(
        "anchor deploy failed. From repo root: ./scripts/cinder-demo.sh"
      );
    }
    await sleep(2000);
    for (const [label, id] of programs) {
      if (!label.startsWith("cinder_")) continue;
      if (!(await conn.getAccountInfo(id))) {
        throw new Error(`${label} still missing after deploy: ${id.toBase58()}`);
      }
    }
  }
  const still = [];
  for (const [label, id] of programs) {
    if (!(await conn.getAccountInfo(id))) {
      still.push(`${label} ${id.toBase58()}`);
    }
  }
  if (still.length) {
    throw new Error(
      "ProgramAccountNotFound — missing on " +
        BASE +
        ":\n  " +
        still.join("\n  ") +
        "\nRestart mb-stack and run ./scripts/cinder-demo.sh"
    );
  }
}

function loadProgram<T extends anchor.Idl>(
  name: string,
  provider: anchor.AnchorProvider
): Program<T> {
  const idl = JSON.parse(
    fs.readFileSync(`target/idl/${name}.json`, "utf8")
  ) as T;
  return new Program(idl, provider);
}

async function sendTx(
  connection: Connection,
  payer: Keypair,
  tx: Transaction,
  extraSigners: Keypair[] = []
) {
  const latest = await connection.getLatestBlockhash("confirmed");
  tx.feePayer = payer.publicKey;
  tx.recentBlockhash = latest.blockhash;
  tx.sign(payer, ...extraSigners);
  const sig = await connection.sendRawTransaction(tx.serialize(), {
    skipPreflight: true,
  });
  const conf = await connection.confirmTransaction(
    {
      signature: sig,
      blockhash: latest.blockhash,
      lastValidBlockHeight: latest.lastValidBlockHeight,
    },
    "confirmed"
  );
  if (conf.value.err) {
    const got = await connection.getTransaction(sig, {
      commitment: "confirmed",
      maxSupportedTransactionVersion: 0,
    });
    const logs = got?.meta?.logMessages?.join("\n") ?? "(no logs available)";
    throw new Error(
      `transaction ${sig} failed: ${JSON.stringify(conf.value.err)}\n${logs}`
    );
  }
  return sig;
}

async function sendIx(
  connection: Connection,
  payer: Keypair,
  builder: { instruction: () => Promise<anchor.web3.TransactionInstruction> },
  extraSigners: Keypair[] = []
) {
  const ix = await builder.instruction();
  return sendTx(connection, payer, new Transaction().add(ix), extraSigners);
}

function pda(programId: PublicKey, seeds: (Buffer | Uint8Array)[]): PublicKey {
  return PublicKey.findProgramAddressSync(seeds, programId)[0];
}

function oid(tag: number): number[] {
  return Array.from({ length: 16 }, (_, i) => (i + tag) % 256);
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

function assertLocalOrTls(url: string, kind: "http" | "ws") {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    throw new Error(`invalid ${kind} endpoint ${url}`);
  }
  const host = parsed.hostname;
  const loopback = host === "127.0.0.1" || host === "localhost";
  const ok = loopback
    ? parsed.protocol === `${kind}:`
    : parsed.protocol === `${kind}s:`;
  if (!ok) {
    throw new Error(
      `${kind} endpoint must be loopback ${kind}:// or remote ${kind}s:// (got ${url})`
    );
  }
}

function qfsConnection(token: string) {
  assertLocalOrTls(QFS, "http");
  assertLocalOrTls(QFS_WS, "ws");
  return new Connection(`${QFS}?token=${token}`, {
    wsEndpoint: `${QFS_WS}?token=${token}`,
    commitment: "confirmed",
  });
}

async function authToken(payer: Keypair) {
  return getAuthToken(QFS, payer.publicKey, (message: Uint8Array) =>
    Promise.resolve(nacl.sign.detached(message, payer.secretKey))
  );
}

function usdc(n: number | string) {
  const v = typeof n === "string" ? Number(n) : n;
  return (v / 1_000_000).toFixed(2) + " USDC";
}

function pane(title: string, lines: string[]) {
  console.log("");
  console.log("┌─ " + title);
  for (const line of lines) console.log("│  " + line);
  console.log("└");
}

async function main() {
  if (
    !(await rpcUp(QFS)) ||
    !(await rpcUp(ER)) ||
    !(await rpcUp(BASE))
  ) {
    console.error(
      "mb-stack not up. In another terminal: ./scripts/stack-local.sh"
    );
    console.error("Then: ./scripts/cinder-demo.sh");
    process.exit(1);
  }

  const base = new Connection(BASE, {
    commitment: "confirmed",
    wsEndpoint: BASE_WS,
  });
  const erConn = new Connection(ER, {
    commitment: "confirmed",
    wsEndpoint: ER_WS,
  });
  process.env.ANCHOR_WALLET =
    process.env.ANCHOR_WALLET ||
    path.join(os.homedir(), ".config", "solana", "id.json");
  const wallet = anchor.Wallet.local();
  const provider = new anchor.AnchorProvider(base, wallet, RPC_OPTS);
  anchor.setProvider(provider);
  const vault = loadProgram<CinderVault>("cinder_vault", provider);
  const ledger = loadProgram<CinderLedger>("cinder_ledger", provider);
  const payer = wallet.payer;

  await ensurePrograms(base, [
    ["cinder_vault", vault.programId],
    ["cinder_ledger", ledger.programId],
    ["spl-token", TOKEN_PROGRAM_ID],
    ["spl-associated-token", ASSOCIATED_TOKEN_PROGRAM_ID],
  ]);

  const adapter = Keypair.generate();
  const alice = Keypair.generate();
  const bob = Keypair.generate();
  const phoenixTrader = Keypair.generate().publicKey;

  await airdrop(base, adapter.publicKey);
  await airdrop(base, alice.publicKey);
  await airdrop(base, bob.publicKey);

  const configPda = pda(vault.programId, [Buffer.from("config")]);
  const vaultAuth = pda(vault.programId, [Buffer.from("vault-authority")]);
  const reservePda = pda(vault.programId, [Buffer.from("reserve")]);
  const bookPda = pda(ledger.programId, [Buffer.from("book")]);
  const feesPda = pda(ledger.programId, [Buffer.from("fees")]);
  const ledgerA = pda(ledger.programId, [
    Buffer.from("user"),
    alice.publicKey.toBuffer(),
  ]);
  const ledgerB = pda(ledger.programId, [
    Buffer.from("user"),
    bob.publicKey.toBuffer(),
  ]);
  const usdcMint = await createMint(base, payer, payer.publicKey, null, 6);
  const vaultAta = getAssociatedTokenAddressSync(usdcMint, vaultAuth, true);

  if (!(await base.getAccountInfo(configPda))) {
    console.log("initialize vault...");
    await sendIx(
      base,
      payer,
      vault.methods
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
    );
    await sendIx(
      base,
      payer,
      vault.methods
        .setAllowlist([ASSET_SOL])
        .accountsPartial({ admin: payer.publicKey, config: configPda })
    );
  } else {
    throw new Error(
      "Cinder state already exists. Restart with ./scripts/stack-local.sh before running the demo."
    );
  }

  await sendIx(
    base,
    payer,
    ledger.methods.initialize().accountsPartial({
      adapter: adapter.publicKey,
      config: configPda,
      book: bookPda,
      feeAccrual: feesPda,
      systemProgram: SystemProgram.programId,
    }),
    [adapter]
  );

  for (const [user, userLedger] of [
    [alice, ledgerA],
    [bob, ledgerB],
  ] as const) {
    await sendIx(
      base,
      payer,
      ledger.methods.initUser().accountsPartial({
        adapter: adapter.publicKey,
        user: user.publicKey,
        config: configPda,
        userLedger,
        book: bookPda,
        systemProgram: SystemProgram.programId,
      }),
      [adapter, user]
    );
  }

  const sendBase = async (builder: any, signers: Keypair[]) => {
    const tx = await builder.transaction();
    return sendTx(base, payer, tx, signers);
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
      user: alice.publicKey,
      config: configPda,
      userLedger: ledgerA,
      validator: ER_VALIDATOR,
    }),
    [adapter, alice]
  );
  await sendBase(
    ledger.methods.delegateUser().accountsPartial({
      adapter: adapter.publicKey,
      user: bob.publicKey,
      config: configPda,
      userLedger: ledgerB,
      validator: ER_VALIDATOR,
    }),
    [adapter, bob]
  );

  async function waitEr(address: PublicKey, label: string) {
    for (let i = 0; i < 40; i++) {
      const info = await erConn.getAccountInfo(address);
      if (info && info.owner.equals(ledger.programId)) return;
      await sleep(500);
    }
    throw new Error(`${label} did not appear on ER`);
  }
  await waitEr(bookPda, "book");
  await waitEr(ledgerA, "Alice");
  await waitEr(ledgerB, "Bob");

  const adapterWallet = new anchor.Wallet(adapter);
  const erProvider = new anchor.AnchorProvider(erConn, adapterWallet, {
    commitment: "confirmed",
  });
  const erProgram = new Program(
    ledger.idl as CinderLedger,
    erProvider
  ) as Program<CinderLedger>;

  const sendEr = async (builder: any, extra: Keypair[] = []) => {
    const tx = await builder.transaction();
    return sendTx(erConn, adapter, tx, extra);
  };
  const perm = (account: PublicKey) => ({
    adapter: adapter.publicKey,
    config: configPda,
    permission: permissionPdaFromAccount(account),
    magicProgram: MAGIC_PROGRAM_ID,
    permissionProgram: PERMISSION_PROGRAM_ID,
    ephemeralVault: EPHEMERAL_VAULT_ID,
  });
  await sendEr(
    erProgram.methods.initBookPermission().accountsPartial({
      ...perm(bookPda),
      book: bookPda,
    })
  );
  await sendEr(
    erProgram.methods.initFeesPermission().accountsPartial({
      ...perm(feesPda),
      feeAccrual: feesPda,
    })
  );
  await sendEr(
    erProgram.methods.initUserPermission().accountsPartial({
      ...perm(ledgerA),
      userLedger: ledgerA,
    })
  );
  await sendEr(
    erProgram.methods.initUserPermission().accountsPartial({
      ...perm(ledgerB),
      userLedger: ledgerB,
    })
  );

  // Pre-fund user cash on the ER (no G-BUF engine; mock residual does not bounce).
  await sendEr(
    erProgram.methods.creditDeposit(new BN(CREDIT)).accountsPartial({
      adapter: adapter.publicKey,
      config: configPda,
      book: bookPda,
      userLedger: ledgerA,
    })
  );
  await sendEr(
    erProgram.methods.creditDeposit(new BN(CREDIT)).accountsPartial({
      adapter: adapter.publicKey,
      config: configPda,
      book: bookPda,
      userLedger: ledgerB,
    })
  );

  const tokA = await authToken(alice);
  const tokB = await authToken(bob);
  const qfsA = qfsConnection(tokA.token);
  const qfsB = qfsConnection(tokB.token);

  const aliceOnQfs = (await qfsA.getAccountInfo(ledgerA)) !== null;
  let bobDenied = false;
  try {
    bobDenied = (await qfsB.getAccountInfo(ledgerA)) === null;
  } catch {
    bobDenied = true;
  }
  const aliceOnEr = (await erConn.getAccountInfo(ledgerA)) !== null;
  pane("1. QFS isolation", [
    `Alice token reads Alice on :6699     ${aliceOnQfs ? "yes" : "NO"}`,
    `Bob token reads Alice on :6699       ${bobDenied ? "DENIED" : "LEAK"}`,
    `Raw ER :7799 reads Alice (unfiltered) ${aliceOnEr ? "yes" : "NO"}`,
  ]);
  if (!aliceOnQfs || !bobDenied || !aliceOnEr) {
    throw new Error("isolation check failed");
  }

  const place = async (
    user: Keypair,
    userLedger: PublicKey,
    lots: number,
    tag: number,
    reduceOnly: boolean,
    nonce: number
  ) => {
    const uw = new anchor.Wallet(user);
    const erUser = new Program(
      ledger.idl as CinderLedger,
      new anchor.AnchorProvider(erConn, uw, { commitment: "confirmed" })
    ) as Program<CinderLedger>;
    const tx = await erUser.methods
      .placeOrder(
        ASSET_SOL,
        new BN(lots),
        oid(tag),
        50,
        reduceOnly,
        new BN(nonce)
      )
      .accountsPartial({
        user: user.publicKey,
        config: configPda,
        book: bookPda,
        userLedger,
      })
      .transaction();
    await sendTx(erConn, user, tx);
  };

  const ack = async (
    userLedger: PublicKey,
    tag: number,
    lots: number,
    vwap: number,
    postPositionIm: number,
    fee = 0
  ) => {
    const tx = await erProgram.methods
      .ackPhoenixFill(
        oid(tag),
        new BN(lots),
        new BN(fee),
        new BN(vwap),
        new BN(postPositionIm)
      )
      .accountsPartial({
        adapter: adapter.publicKey,
        config: configPda,
        userLedger,
        book: bookPda,
        feeAccrual: feesPda,
      })
      .transaction();
    await sendTx(erConn, adapter, tx);
  };

  await place(alice, ledgerA, LOTS, 1, false, 0);
  await ack(ledgerA, 1, LOTS, OPEN_VWAP, IM_TEN_LOTS);
  await place(bob, ledgerB, -LOTS, 2, false, 0);
  await ack(ledgerB, 2, -LOTS, -OPEN_VWAP, IM_TEN_LOTS);

  const snapshot = async (title: string) => {
    const a = await erProgram.account.userLedger.fetch(ledgerA);
    const b = await erProgram.account.userLedger.fetch(ledgerB);
    const book = await erProgram.account.book.fetch(bookPda);
    const lotsA = a.positionsLen ? a.positions[0].lots.toNumber() : 0;
    const lotsB = b.positionsLen ? b.positions[0].lots.toNumber() : 0;
    const bookLots =
      book.residualLen > 0 ? book.residuals[0].lots.toNumber() : 0;
    pane(title, [
      `Alice  lots=${lotsA}  free=${usdc(a.free.toNumber())}  reserved=${usdc(a.reserved.toNumber())}  Cinder MM (stub)=${usdc(STUB_MM)}`,
      `Bob    lots=${lotsB}  free=${usdc(b.free.toNumber())}  reserved=${usdc(b.reserved.toNumber())}`,
      `Book   public net=${bookLots}  (Alice+Bob=${lotsA + lotsB})`,
    ]);
    return { lotsA, lotsB, bookLots, freeA: a.free.toNumber() };
  };

  const mid = await snapshot("2. After offsetting fills (mocked Phoenix residual)");
  if (mid.bookLots !== 0 || mid.lotsA + mid.lotsB !== 0) {
    throw new Error("expected net Book 0 after offsetting fills");
  }

  await place(alice, ledgerA, -LOTS, 3, true, 1);
  await ack(ledgerA, 3, -LOTS, CLOSE_VWAP, 0);
  const end = await snapshot("3. After Alice closes (realized PnL into free)");
  if (end.lotsA !== 0) throw new Error("Alice should be flat");
  if (end.freeA !== CREDIT + 1_000_000) {
    throw new Error(
      `Alice free expected ${CREDIT + 1_000_000}, got ${end.freeA}`
    );
  }

  pane("Notes (not in this tape)", [
    "No live liq/funding clocks. Stub MM on localnet. escape_withdraw unsupported. Market/IOC only.",
  ]);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
