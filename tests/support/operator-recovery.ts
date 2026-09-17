/** Real QFS/Anchor recovery with deterministic, SDK-encoded public Rise RPC
 * fixtures. No Phoenix trade is submitted; this is not live-venue proof.
 */
import * as anchor from "@anchor-lang/core";
import { ChildProcess, spawn, spawnSync } from "child_process";
import { createHash } from "crypto";
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync } from "fs";
import { createServer, Server } from "http";
import { tmpdir } from "os";
import { performance } from "perf_hooks";
import { join, resolve } from "path";
import { Keypair, PublicKey, Transaction } from "@solana/web3.js";
import bs58 from "bs58";
import { expect } from "chai";
import accounts from "../../crates/cinder-operator/tests/fixtures/rise-accounts.json";
import type { CinderLedger } from "../../target/types/cinder_ledger";
import type { CinderVault } from "../../target/types/cinder_vault";

const PROGRAM = "EtrnLzgbS7nMMy5fbD42kXiUzGg8XQzJ972Xtk1cjWih";
const GLOBAL = "2zskx2iyCvb6Stg7RBZkt1f6MrF4dpYtMG3yMvKwqtUZ";
const ASSET_MAP = "2nHGAaEw3D5dd4hVueaUNoygkQFmoeKqRQWnSPqSMFUC";
const QUOTE_MINT = "PhUsd11YkbjSaWjFncfAAmatntsjx3MgDR9B6g1ks3A";
const GTI = "HCrPXLByGqRh2szQi3gj7oRdRVBNi1gccAyn4CQCT3HK";
const BUFFER = "2U32rSzzrQS3eVmGHsnbw5kcqKF3wQXpHGd3hMq5YJok";
const HAWKEYE = "RiSeVw3ZjNfsaXPRb4mgaqYaEEt41pNNJoDvVh7pgQj";
const TOKEN = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const OPERATOR_DOWN = 1 << 5;
const binary = resolve(process.env.CINDER_OPERATOR_BIN || "target/debug/cinder-operator");
const encoder = resolve("target/debug/examples/recovery-fixture");
type Fault = "none" | "before-sign" | "after-send";

function account(data: Buffer, owner: string) {
  return { data: [data.toString("base64"), "base64"], owner, executable: false, lamports: 100000000, rentEpoch: 0 };
}
function magic(label: string) { return createHash("sha256").update(label).digest().subarray(0, 8); }
function margin(lots: bigint, cash: bigint) {
  const b = Buffer.alloc(112); magic("return:phoenix_hawkeye_margin").copy(b);
  b.writeUInt16LE(1, 8); b.writeUInt16LE(lots === 0n ? 0 : 1, 10);
  b.writeBigInt64LE(cash, 16); b.writeBigInt64LE(cash, 24); b.writeBigInt64LE(cash, 32);
  return b;
}
function asset(lots: bigint) {
  const b = Buffer.alloc(128); magic("return:phoenix_hawkeye_asset").copy(b);
  b.writeUInt16LE(1, 12); b[14] = lots === 0n ? 0 : 1;
  b.writeBigInt64LE(lots, 24); b.writeBigInt64LE(-lots * 719100n, 32); b.writeBigUInt64LE(7191n, 40);
  return b;
}
async function listen(server: Server) {
  await new Promise<void>((resolve, reject) => { server.once("error", reject); server.listen(0, "127.0.0.1", resolve); });
  return `http://127.0.0.1:${(server.address() as { port: number }).port}`;
}
async function close(server: Server) { server.closeAllConnections(); await new Promise<void>(r => server.close(() => r())); }
async function payload(req: import("http").IncomingMessage) {
  const chunks: Buffer[] = []; for await (const chunk of req) chunks.push(Buffer.from(chunk));
  return Buffer.concat(chunks);
}

export async function verifyOperatorRecovery(ctx: {
  base: string; qfs: string; adapter: Keypair; user: Keypair;
  ledger: anchor.Program<CinderLedger>; vault: anchor.Program<CinderVault>; privateLedger: anchor.Program<CinderLedger>;
  config: PublicKey; book: PublicKey; userLedger: PublicKey; fees: PublicKey; trader: PublicKey;
}) {
  for (const endpoint of [ctx.base, ctx.qfs]) {
    if (!new URL(endpoint).hostname.match(/^(127\.0\.0\.1|localhost)$/)) throw new Error("operator fixture requires local endpoints");
  }
  const dir = mkdtempSync(join(tmpdir(), "cinder-recovery-"));
  mkdirSync(join(dir, "locks"), { mode: 0o700 });
  writeFileSync(join(dir, "operator.json"), JSON.stringify(Array.from(ctx.adapter.secretKey)), { mode: 0o600 });
  const journal = join(dir, "private/journal.sqlite");
  let child: ChildProcess | undefined;
  let fault: Fault = "none";
  let killed = false;
  let ackSends = 0;
  let feeTotal = 0n;
  let lots = 0n;
  let snapshotSlot = 0;
  let publicSnapshots = 0;
  const receipts = new Map<string, any>();
  const baseRpc = async (method: string, params: any[]) => {
    const res = await fetch(ctx.base, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }) });
    const value = await res.json() as any;
    if (value.error) throw new Error(`fixture base RPC failed: ${method}`);
    return value.result;
  };
  const initial = await ctx.privateLedger.account.userLedger.fetch(ctx.userLedger) as any;
  const initialCash = BigInt(initial.free.toString()) + BigInt(initial.reserved.toString()) - BigInt(initial.badDebtUsdc.toString());
  const mint = Buffer.alloc(82); mint[44] = 6; mint[45] = 1;
  const publicServer = createServer(async (req, res) => {
    try {
      const rpc = JSON.parse((await payload(req)).toString());
      let result: any;
      if (rpc.method === "getMultipleAccounts" && rpc.params[0].length > 1 && rpc.params[0].includes(GLOBAL)) {
        // Freeze the fixture during one before/after public read bracket.
        if (publicSnapshots++ % 2 === 0) snapshotSlot = await baseRpc("getSlot", [{ commitment: "confirmed" }]);
        const raw = await baseRpc("getMultipleAccounts", rpc.params);
        const map = Buffer.alloc(1622064); Buffer.from(accounts.assetMapPrefix, "base64").copy(map);
        map.writeBigUInt64LE(BigInt(snapshotSlot), 80);
        const trader = Buffer.from(accounts.trader, "base64");
        ctx.trader.toBuffer().copy(trader, 24); ctx.adapter.publicKey.toBuffer().copy(trader, 56); ctx.adapter.publicKey.toBuffer().copy(trader, 120);
        trader.writeUInt32LE(0, 108); trader.writeUInt16LE(0, 152); trader[155] = 0;
        trader.fill(0, 200, 224); trader.writeBigUInt64LE(0n, 224);
        const fixtures: Record<string, any> = {
          [GLOBAL]: account(Buffer.from(accounts.globalConfig, "base64"), PROGRAM),
          [ASSET_MAP]: account(map, PROGRAM),
          [ctx.trader.toBase58()]: account(trader, PROGRAM),
          [QUOTE_MINT]: account(mint, TOKEN),
          [GTI]: account(Buffer.alloc(8), PROGRAM), [BUFFER]: account(Buffer.alloc(8), PROGRAM),
        };
        result = { context: { slot: snapshotSlot }, value: rpc.params[0].map((k: string, i: number) => fixtures[k] || raw.value[i]) };
      } else if (rpc.method === "simulateTransaction") {
        const tx = Transaction.from(Buffer.from(rpc.params[0], "base64"));
        const ix = tx.instructions[0];
        const isMargin = ix.data.subarray(0, 8).equals(magic("global:view_margin"));
        const isAsset = ix.data.subarray(0, 8).equals(magic("global:view_margin_for_asset"));
        if (ix.programId.toBase58() !== HAWKEYE || (!isMargin && !isAsset)) throw new Error("unexpected fixture simulation");
        result = { context: { slot: snapshotSlot }, value: { err: null, returnData: { programId: HAWKEYE, data: [(isMargin ? margin(lots, initialCash - feeTotal) : asset(lots)).toString("base64"), "base64"] } } };
      } else if (rpc.method === "getSignaturesForAddress" && rpc.params[0] === ctx.trader.toBase58()) {
        result = rpc.params[1].before ? [] : Array.from(receipts.keys()).reverse().map(signature => ({ signature, slot: snapshotSlot, err: null }));
      } else if (rpc.method === "getTransaction" && receipts.has(rpc.params[0])) result = receipts.get(rpc.params[0]);
      else result = await baseRpc(rpc.method, rpc.params);
      res.end(JSON.stringify({ jsonrpc: "2.0", id: rpc.id, result }));
    } catch { res.statusCode = 500; res.end("local fixture failure"); }
  });
  const coder = new anchor.BorshInstructionCoder(ctx.ledger.idl);
  const qfsServer = createServer(async (req, res) => {
    try {
      const body = await payload(req);
      const rpc = req.url!.startsWith("/auth/") ? undefined : JSON.parse(body.toString());
      if (fault === "before-sign" && rpc?.method === "getLatestBlockhash") {
        fault = "none"; killed = true; child!.kill("SIGKILL"); res.destroy(); return;
      }
      let ack = false;
      if (rpc?.method === "sendTransaction") {
        const tx = Transaction.from(Buffer.from(rpc.params[0], "base64"));
        const decoded = coder.decode(tx.instructions[0].data);
        ack = !!decoded?.name.replace(/_/g, "").toLowerCase().match(/^ackphoenix(fill|fail)guarded$/);
        if (ack) ackSends++;
      }
      const response = await fetch(`${ctx.qfs}${req.url}`, { method: req.method, headers: { "content-type": "application/json" }, ...(req.method === "POST" ? { body } : {}) });
      const responseBody = Buffer.from(await response.arrayBuffer());
      if (ack && fault === "after-send") {
        const returned = JSON.parse(responseBody.toString());
        if (returned.error) throw new Error("guarded ack rejected by QFS");
        fault = "none"; killed = true; child!.kill("SIGKILL"); res.destroy(); return;
      }
      res.statusCode = response.status; res.end(responseBody);
    } catch { res.statusCode = 500; res.end("local fixture failure"); }
  });
  const publicUrl = await listen(publicServer); const qfsUrl = await listen(qfsServer);
  const config = {
    operator_keypair: join(dir, "operator.json"), pool_lock_directory: join(dir, "locks"),
    l1_rpc_env: "CINDER_FIXTURE_L1", qfs_rpc_env: "CINDER_FIXTURE_QFS",
    phoenix_program: PROGRAM, phoenix_global_config: GLOBAL, phoenix_trader: ctx.trader.toBase58(),
    phoenix_asset_map: ASSET_MAP, phoenix_quote_mint: QUOTE_MINT,
    markets: [{ cinder_asset_id: 1, phoenix_asset_id: 0, symbol: "SOL" }], global_trader_index: [GTI], active_trader_buffer: [BUFFER],
  };
  writeFileSync(join(dir, "config.json"), JSON.stringify(config));
  const run = async () => {
    publicSnapshots = 0;
    return new Promise<{ code: number | null; signal: string | null; output: string }>((resolve, reject) => {
      child = spawn(binary, ["recover", join(dir, "config.json"), journal], { env: { ...process.env, CINDER_FIXTURE_L1: publicUrl, CINDER_FIXTURE_QFS: qfsUrl } });
      let output = ""; child.stdout!.on("data", d => { output += d; }); child.stderr!.on("data", d => { output += d; });
      const timer = setTimeout(() => { child?.kill("SIGKILL"); reject(new Error("operator fixture timed out")); }, 30000);
      child.once("error", e => { clearTimeout(timer); reject(e); });
      child.once("exit", (code, signal) => { clearTimeout(timer); resolve({ code, signal, output }); });
    });
  };
  const converge = async (label: string) => {
    // Confirmation/indexing and the strict fresh-snapshot bracket may need
    // another validator slot. Retry only the safely gated exit code, bounded
    // by a monotonic deadline; fatal runtime errors fail immediately.
    const deadline = performance.now() + 10000;
    let recovered = await run();
    while (recovered.code === 3 && performance.now() < deadline) {
      await new Promise(resolve => setTimeout(resolve, 200));
      recovered = await run();
    }
    expect(recovered.code, `${label}: ${recovered.output}`).to.equal(0);
  };
  const gate = async (down: boolean) => {
    await ctx.vault.methods.setOperatorDown(down).accountsPartial({ adapter: ctx.adapter.publicKey, config: ctx.config }).signers([ctx.adapter]).rpc();
    await ctx.privateLedger.methods.setOperatorDown(down).accountsPartial({ adapter: ctx.adapter.publicKey, config: ctx.config, book: ctx.book }).rpc();
    // Config lives on L1 and is replicated to ER asynchronously. Confirmation
    // on L1 alone does not mean a subsequent private order sees the new flag.
    const privateVault = new anchor.Program<CinderVault>(ctx.vault.idl, ctx.privateLedger.provider);
    for (let attempt = 0; attempt < 40; attempt++) {
      const [config, book] = await Promise.all([
        privateVault.account.config.fetch(ctx.config),
        ctx.privateLedger.account.book.fetch(ctx.book),
      ]);
      if (!!(config.paused & OPERATOR_DOWN) === down && !!(book.halt & OPERATOR_DOWN) === down) return;
      await new Promise(resolve => setTimeout(resolve, 100));
    }
    throw new Error("operator fixture gate was not observed through QFS");
  };
  try {
    for (const [index, mode, rejected] of [[0, "before-sign", false], [1, "after-send", false], [2, "after-send", true]] as const) {
      await gate(false);
      const before = await ctx.privateLedger.account.userLedger.fetch(ctx.userLedger) as any;
      const oid = Array(16).fill(77); // reuse after each prior operation is terminal
      try {
        await ctx.privateLedger.methods.placeOrder(1, new anchor.BN(10), oid, new anchor.BN(8000), new anchor.BN("18446744073709551615"), new anchor.BN(0), new anchor.BN(0), false, before.nonce)
          .accountsPartial({ user: ctx.user.publicKey, adapter: ctx.adapter.publicKey, config: ctx.config, book: ctx.book, userLedger: ctx.userLedger }).signers([ctx.user]).rpc();
      } catch (error) {
        // Anchor's ProgramError can have an empty message. Surface only the
        // numeric/code identity, never private transaction bodies or tokens.
        const e = error as { code?: number; error?: { errorCode?: { code?: string; number?: number } } };
        throw new Error(`local placement failed in case ${index}: ${e.error?.errorCode?.code || e.code || e.error?.errorCode?.number || "unknown"}`);
      }
      await gate(true);
      const nonce = Buffer.alloc(8); nonce.writeBigUInt64LE(BigInt(before.nonce.toString()));
      const nativeOid = Array.from(createHash("sha256").update(Buffer.concat([Buffer.from("cinder:phoenix:v1"), ctx.user.publicKey.toBuffer(), nonce, Buffer.from(oid)])).digest().subarray(0, 16));
      const signature = bs58.encode(Buffer.alloc(64, index + 40));
      // Preserve u64::MAX in the fixture encoder's JSON input without JS float
      // rounding. Production values remain typed Rust u64 throughout.
      const input = JSON.stringify({ trader: ctx.trader.toBase58(), operator: ctx.adapter.publicKey.toBase58(), program: PROGRAM, oid: nativeOid, native_asset_id: 0, lots: 10, ticks: 7191, bound: 8000, deadline: "18446744073709551615", fee: 7, time: Math.floor(Date.now() / 1000), slot: 1, signature, rejected }).replace('"deadline":"18446744073709551615"', '"deadline":18446744073709551615');
      const generated = spawnSync(encoder, [], { input, encoding: "utf8" });
      if (generated.status !== 0) throw new Error("SDK venue fixture encoder failed");
      receipts.set(signature, JSON.parse(generated.stdout));
      if (!rejected) { lots += 10n; feeTotal += 7n; }
      fault = mode; killed = false;
      // Arm the fault until the intended boundary is reached. A safely gated
      // pass can precede signing because a fresh snapshot is not yet available.
      const faultDeadline = performance.now() + 10000;
      let crashed = await run();
      while (!killed && crashed.code === 3 && performance.now() < faultDeadline) {
        await new Promise(resolve => setTimeout(resolve, 200));
        crashed = await run();
      }
      expect(killed, crashed.output).to.equal(true); expect(crashed.signal).to.equal("SIGKILL");
      await converge(`crash recovery case ${index}`);
      const after = await ctx.privateLedger.account.userLedger.fetch(ctx.userLedger) as any;
      expect(after.pendingOidCount).to.equal(0);
      expect(BigInt(after.free.toString()) + BigInt(after.reserved.toString()) - BigInt(after.badDebtUsdc.toString())).to.equal(initialCash - feeTotal);
      const book = await ctx.privateLedger.account.book.fetch(ctx.book) as any;
      expect(BigInt(book.residuals[0].lots.toString())).to.equal(lots);
      await converge(`stable replay case ${index}`);
    }
    expect(ackSends).to.equal(3); // no economic replay or extra ack on restart
    const journalBytes = readFileSync(journal);
    expect(journalBytes.includes(Buffer.from("token="))).to.equal(false);
    expect(journalBytes.includes(Buffer.from(JSON.stringify(Array.from(ctx.adapter.secretKey))))).to.equal(false);
  } finally {
    child?.kill("SIGKILL"); await close(publicServer); await close(qfsServer);
  }
}
