/**
 * Filmable walkthrough on mb-stack + mocked Phoenix residual (dummy ack).
 *
 *   ./scripts/stack-c.sh          # other terminal
 *   ./scripts/cinder-demo.sh
 *
 * Prints QFS isolation, then Alice / Bob / Book before and after a close with PnL.
 */
import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
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
const ER = process.env.EPHEMERAL_PROVIDER_ENDPOINT || "http://127.0.0.1:7799";
const QFS = process.env.TEE_PROVIDER_ENDPOINT || "http://127.0.0.1:6699";
const QFS_WS = process.env.TEE_WS_ENDPOINT || "ws://127.0.0.1:6700";
const ASSET_SOL = 1;
const CREDIT = 100_000_000;
const LOTS = 10;
const OPEN_VWAP = 10_000_000;
const CLOSE_VWAP = -11_000_000;
const STUB_MM = 625_000; // stub_cinder_mm(10)

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
      "mb-stack not up. In another terminal: ./scripts/stack-c.sh"
    );
    console.error("Then: ./scripts/cinder-demo.sh");
    process.exit(1);
  }

  const base = new Connection(BASE, "confirmed");
  const erConn = new Connection(ER, "confirmed");
  const wallet = anchor.Wallet.local();
  const provider = new anchor.AnchorProvider(base, wallet, {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);
  const vault = (anchor.workspace as any).cinderVault as Program<CinderVault>;
  const ledger = (anchor.workspace as any).cinderLedger as Program<CinderLedger>;
  const payer = wallet.payer;

  const adapter = Keypair.generate();
  const alice = Keypair.generate();
  const bob = Keypair.generate();
  const phoenixTrader = Keypair.generate().publicKey;

  await airdrop(base, adapter.publicKey);
  await airdrop(base, alice.publicKey);
  await airdrop(base, bob.publicKey);

  const usdcMint = await createMint(base, payer, payer.publicKey, null, 6);
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
  const vaultAta = getAssociatedTokenAddressSync(usdcMint, vaultAuth, true);

  if (!(await base.getAccountInfo(configPda))) {
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
    await vault.methods
      .setAllowlist([ASSET_SOL])
      .accounts({ admin: payer.publicKey, config: configPda })
      .rpc();
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
  } else {
    console.error(
      "Config already exists on this validator. Restart: ./scripts/stack-c.sh"
    );
    process.exit(1);
  }

  for (const [user, userLedger] of [
    [alice, ledgerA],
    [bob, ledgerB],
  ] as const) {
    await ledger.methods
      .initUser()
      .accounts({
        adapter: adapter.publicKey,
        user: user.publicKey,
        config: configPda,
        userLedger,
        book: bookPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([adapter, user])
      .rpc();
  }

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

  const sendEr = async (builder: any, extra?: Keypair[]) => {
    let tx = await builder.transaction();
    tx.feePayer = adapter.publicKey;
    tx.recentBlockhash = (await erConn.getLatestBlockhash()).blockhash;
    tx = await adapterWallet.signTransaction(tx);
    if (extra) for (const s of extra) tx.partialSign(s);
    return erProvider.sendAndConfirm(tx, extra ?? [], { skipPreflight: true });
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
    erProgram.methods.creditDeposit(new BN(CREDIT)).accounts({
      adapter: adapter.publicKey,
      config: configPda,
      book: bookPda,
      userLedger: ledgerA,
    })
  );
  await sendEr(
    erProgram.methods.creditDeposit(new BN(CREDIT)).accounts({
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
    let tx = await erUser.methods
      .placeOrder(
        ASSET_SOL,
        new BN(lots),
        oid(tag),
        50,
        reduceOnly,
        new BN(nonce)
      )
      .accounts({
        user: user.publicKey,
        config: configPda,
        book: bookPda,
        userLedger,
      })
      .transaction();
    tx.feePayer = user.publicKey;
    tx.recentBlockhash = (await erConn.getLatestBlockhash()).blockhash;
    tx = await uw.signTransaction(tx);
    const sig = await erConn.sendRawTransaction(tx.serialize(), {
      skipPreflight: true,
    });
    await erConn.confirmTransaction(sig, "confirmed");
  };

  const ack = async (
    userLedger: PublicKey,
    tag: number,
    lots: number,
    vwap: number,
    fee = 0
  ) => {
    let tx = await erProgram.methods
      .ackPhoenixFill(oid(tag), new BN(lots), new BN(fee), new BN(vwap))
      .accounts({
        adapter: adapter.publicKey,
        config: configPda,
        userLedger,
        book: bookPda,
        feeAccrual: feesPda,
      })
      .transaction();
    tx.feePayer = adapter.publicKey;
    tx.recentBlockhash = (await erConn.getLatestBlockhash()).blockhash;
    tx = await adapterWallet.signTransaction(tx);
    const sig = await erConn.sendRawTransaction(tx.serialize(), {
      skipPreflight: true,
    });
    await erConn.confirmTransaction(sig, "confirmed");
  };

  await place(alice, ledgerA, LOTS, 1, false, 0);
  await ack(ledgerA, 1, LOTS, OPEN_VWAP);
  await place(bob, ledgerB, -LOTS, 2, false, 0);
  await ack(ledgerB, 2, -LOTS, -OPEN_VWAP);

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
  await ack(ledgerA, 3, -LOTS, CLOSE_VWAP);
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
