// Complete SDK -> opaque relay -> private service -> controllers -> AEAD journal
// workflow. Native deposit/history/rates are EXPLICIT fake ports. Vault release,
// return and beneficiary payouts execute real local SBF. No remote origin/wallet.
import assert from "node:assert/strict";
import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { once } from "node:events";
import { createPrivateKey, randomBytes, sign } from "node:crypto";
import { mkdtemp, mkdir, readFile, readdir, stat, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { Program } from "@anchor-lang/core";
import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  type TransactionInstruction,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  createAssociatedTokenAccount,
  getAccount,
  getAssociatedTokenAddress,
  mintTo,
  burn,
} from "@solana/spl-token";
import type { CinderVault } from "../src/cinder_vault.ts";
import {
  customerAddress,
  depositReceiptAddress,
  identity,
  movement,
  receiptAddress,
  u64,
  vaultAddresses,
} from "../src/index.ts";
import { fundingInstruction, verifyFundingWire } from "../src/funding.ts";
import {
  PrivateClient,
  type Command,
  type MessageSigner,
  type ReadPage,
} from "../../private/src/index.ts";
import {
  WebChannel,
  type WebCore,
} from "../../private/src/web-channel-core.ts";
import { verifyWithRoot } from "../../../tools/web-channel/attestation/verifier.mjs";
// @ts-expect-error opaque JavaScript parent has no private financial API
import { createWebRelay } from "../../../services/web-relay/server.mjs";

interface Environment {
  program: Program<CinderVault>;
  connection: Connection;
  governance: Keypair;
  programData: PublicKey;
  fund(k: Keypair): Promise<void>;
  send(ix: TransactionInstruction, signers: Keypair[]): Promise<string>;
}
type Book = { cash: string; funding: string; quantity: string; basis: string };
type Oracle = {
  ledger_version: number;
  customer: Book;
  house: Book;
  native: Book;
  vault: string;
  broker: string;
  transit: string;
  customer_reserved: string;
  venue_reserved: string;
  unresolved: number;
  active_holds: number;
  exposed: number;
  claims: string;
  shortfall: string;
  frozen: boolean;
  orders: {
    id: number[];
    client_id: string;
    filled: number;
    complete: boolean;
    cuts: number[];
  }[];
};
const bytes = (p: PublicKey | Uint8Array) =>
  Array.from(p instanceof PublicKey ? p.toBytes() : p);
const tag = (n: number) => Buffer.alloc(32, n);
const flat = (cash: bigint): Book => ({
  cash: String(cash),
  funding: "0",
  quantity: "0",
  basis: "0",
});
const key = (k: Keypair) =>
  createPrivateKey({
    key: Buffer.concat([
      Buffer.from("302e020100300506032b657004220420", "hex"),
      Buffer.from(k.secretKey.subarray(0, 32)),
    ]),
    format: "der",
    type: "pkcs8",
  });
async function lines(child: ChildProcessWithoutNullStreams) {
  let buffer = "",
    pending: ((v: string) => void) | undefined,
    rejected: ((e: Error) => void) | undefined,
    dead = false;
  const queue: string[] = [];
  child.stdout.on("data", (data) => {
    buffer += data;
    while (buffer.includes("\n")) {
      const at = buffer.indexOf("\n"),
        line = buffer.slice(0, at);
      buffer = buffer.slice(at + 1);
      if (pending) {
        const deliver = pending;
        pending = undefined;
        rejected = undefined;
        deliver(line);
      } else queue.push(line);
    }
  });
  child.on("error", () => {
    dead = true;
    rejected?.(Error("Acceptance child failed"));
  });
  child.on("exit", () => {
    dead = true;
    rejected?.(Error("Acceptance child exited"));
  });
  child.stdin.on("error", () => {});
  child.stderr.resume();
  return async () => {
    if (queue.length) return queue.shift()!;
    if (dead) throw Error("Acceptance child unavailable");
    return new Promise<string>((resolve, reject) => {
      const timer = setTimeout(() => {
        pending = undefined;
        rejected = undefined;
        reject(Error("Acceptance response deadline"));
      }, 10_000);
      pending = (v) => {
        clearTimeout(timer);
        resolve(v);
      };
      rejected = (e) => {
        clearTimeout(timer);
        reject(e);
      };
    });
  };
}
async function stop(child: ChildProcessWithoutNullStreams | undefined) {
  if (!child || child.exitCode !== null || child.signalCode !== null) return;
  const exited = once(child, "exit");
  child.kill("SIGKILL");
  await exited;
}

/** Same normal and fault workflow over each real public SDK carrier. */
export async function runAcceptance(
  e: Environment,
  carrier: "http" | "websocket",
) {
  const { program, connection, governance, programData } = e,
    owner = Keypair.generate(),
    funds = Keypair.generate(),
    recovery = Keypair.generate(),
    broker = Keypair.generate(),
    native = Keypair.generate();
  const budgets = JSON.parse(
    await readFile(
      new URL(
        "../../../docs/implementation/acceptance-manifest.json",
        import.meta.url,
      ),
      "utf8",
    ),
  ).budgets;
  await Promise.all([owner, funds, recovery, broker].map(e.fund));
  // Match the sandbox's signed SBF sender: execute, rather than simulate, setup
  // instructions and confirm at the explicitly selected commitment. The first
  // SPL transaction must not inherit the helper's implicit preflight defaults.
  const setupConfirm = {
    commitment: "confirmed" as const,
    preflightCommitment: "confirmed" as const,
    skipPreflight: true,
  };
  const mint = await createMint(
    connection,
    governance,
    governance.publicKey,
    null,
    0,
    Keypair.generate(),
    setupConfirm,
  );
  const source = await createAccount(
    connection,
    governance,
    mint,
    owner.publicKey,
    undefined,
    setupConfirm,
  );
  const brokerTokens = await createAssociatedTokenAccount(
    connection,
    governance,
    mint,
    broker.publicKey,
    setupConfirm,
  );
  const central = PublicKey.findProgramAddressSync(
    [Buffer.from("central_state")],
    native.publicKey,
  )[0];
  const venueVault = await getAssociatedTokenAddress(mint, central, true);
  const domain = tag(2),
    pool = randomBytes(32),
    { config, vault } = vaultAddresses(program.programId, domain, pool, mint),
    customer = customerAddress(program.programId, config, owner.publicKey);
  await e.send(
    await program.methods
      .initialize(identity(domain), identity(pool), u64(10_000n), u64(10_000n))
      .accountsStrict({
        governance: governance.publicKey,
        funds: funds.publicKey,
        recovery: recovery.publicKey,
        program: program.programId,
        programData,
        mint,
        brokerTokens,
        config,
        vault,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .instruction(),
    [governance, funds, recovery],
  );
  await e.send(
    await program.methods
      .registerCustomer(identity(domain))
      .accountsStrict({
        config,
        owner: owner.publicKey,
        customer,
        systemProgram: SystemProgram.programId,
      })
      .instruction(),
    [owner],
  );
  await mintTo(
    connection,
    governance,
    mint,
    source,
    governance,
    1000n,
    [],
    setupConfirm,
  );
  const binding = {
    account: broker.publicKey.toBase58(),
    broker_seed: bytes(broker.secretKey.subarray(0, 32)),
    keys: [],
    route: {
      domain: bytes(domain),
      pool: bytes(pool),
      funds: bytes(funds.publicKey),
      beneficiaries: [
        {
          account: bytes(tag(1)),
          wallet: bytes(owner.publicKey),
          tokens: bytes(source),
        },
      ],
      program: bytes(program.programId),
      config: bytes(config),
      vault: bytes(vault),
      mint: bytes(mint),
      broker: bytes(broker.publicKey),
      broker_tokens: bytes(brokerTokens),
      venue_program: bytes(native.publicKey),
      venue_vault: bytes(venueVault),
      epoch: 1,
      decimals: 0,
      withdrawal: "Qualified",
      chain: "Qualified",
      settings: "Qualified",
      withdrawal_cost: 10,
      maximum_movement: 10_000,
      maximum_fee: 0,
      setup_max_age: 86_400_000,
    },
  };
  const directory = await mkdtemp(join(tmpdir(), "cinder-p22-acceptance-")),
    store = join(directory, "store");
  await mkdir(store, { mode: 0o700 });
  const storageKey = randomBytes(32),
    configBytes = Buffer.from(JSON.stringify(binding)),
    prefix = Buffer.alloc(4);
  prefix.writeUInt32BE(configBytes.length);
  let child: ChildProcessWithoutNullStreams | undefined,
    relay: ReturnType<typeof createWebRelay> | undefined,
    channel: WebChannel | undefined,
    api!: PrivateClient;
  let streamChannel: WebChannel | undefined,
    stream: AsyncIterator<ReadPage> | undefined,
    rootDer: Buffer;
  const privateBodies: Buffer[] = [];
  let nextLine: () => Promise<string>;
  const coreModule = await import(
    new URL("../../../tools/web-channel/pkg/channel.js", import.meta.url).href
  );
  await coreModule.default({
    module_or_path: await readFile(
      new URL(
        "../../../tools/web-channel/pkg/channel_bg.wasm",
        import.meta.url,
      ),
    ),
  });
  const core = coreModule as WebCore,
    policy = {
      network: tag(1),
      deployment: domain,
      manifest: tag(21),
      pcrs: [
        Buffer.alloc(48, 22),
        Buffer.alloc(48, 23),
        Buffer.alloc(48, 24),
      ] as const,
    };
  const signer: MessageSigner = {
    publicKey: owner.publicKey.toBytes(),
    signMessage: async (b) => sign(null, b, key(owner)),
  };
  let readId = 180;
  const request = (id: number, command: Command) => ({
    id: tag(id),
    epoch: 1n,
    expiresAt: BigInt(Date.now() + 60_000),
    command,
  });
  const ask = async (v: unknown) => {
    child!.stdin.write(JSON.stringify(v) + "\n");
    return JSON.parse(await nextLine());
  };
  const restart = async () => {
    streamChannel?.close();
    streamChannel = undefined;
    stream = undefined;
    channel?.close();
    await relay?.close();
    relay = undefined;
    await stop(child);
    child = spawn(
      new URL("../../../target/debug/cinder-service-fixture", import.meta.url)
        .pathname,
      [
        "127.0.0.1:0",
        store,
        owner.publicKey.toBuffer().toString("hex"),
        "--acceptance-web",
      ],
    );
    nextLine = await lines(child);
    child.stdin.write(Buffer.concat([storageKey, prefix, configBytes]));
    const [address, root] = (await nextLine()).split(" ");
    rootDer = Buffer.from(root!, "hex");
    relay = createWebRelay({
      target: { host: "127.0.0.1", port: Number(address!.split(":")[1]) },
    });
    relay.server.listen(0, "127.0.0.1");
    await once(relay.server, "listening");
    channel = await WebChannel.connect(
      {
        baseUrl: `http://127.0.0.1:${relay.server.address().port}`,
        policy,
        core,
        transport: carrier,
      },
      (q, p, c) => verifyWithRoot(q, p, c, Buffer.from(root!, "hex")),
    );
    api = new PrivateClient(channel, signer, {
      domain: policy,
      account: tag(1),
      policy: 1,
    });
  };
  let c = flat(0n),
    v = flat(1000n),
    vaultAmount = 0n,
    brokerAmount = 0n,
    transit = 0n,
    held = 0n;
  const expectedOrders = new Map<
    number,
    { filled: number; complete: boolean; cuts: number[] }
  >();
  const boundary = async (label: string) => {
    process.stdout.write(`P22 ${carrier}: ${label}\n`);
    const got = (await ask({ op: "oracle" })) as Oracle;
    assert.equal(
      (got as unknown as { error?: string }).error,
      undefined,
      label,
    );
    assert.deepEqual(got.customer, c, label + " customer");
    assert.deepEqual(got.native, v, label + " native");
    assert.deepEqual(got.house, flat(1000n), label + " house");
    assert.equal(got.vault, String(vaultAmount), label + " vault");
    assert.equal(got.broker, String(brokerAmount), label + " broker");
    assert.equal(got.transit, String(transit), label + " transit");
    assert.equal(got.customer_reserved, String(held), label + " holds");
    assert.equal(
      got.venue_reserved,
      held === 24n ? "24" : "0",
      label + " native hold",
    );
    const equity =
      BigInt(c.cash) +
      BigInt(c.funding) +
      BigInt(c.quantity) * 100n -
      BigInt(c.basis);
    assert.equal(
      got.claims,
      String(equity > 0n ? equity : 0n),
      label + " positive claim",
    );
    assert.equal(got.frozen, false, label + " ordinary mode");
    for (const [id, expected] of expectedOrders) {
      const actual = got.orders.find((order) => order.id[0] === id);
      assert(actual, label + " retained order");
      assert.deepEqual(
        { filled: actual.filled, complete: actual.complete, cuts: actual.cuts },
        expected,
        label + " source-cut/lifecycle",
      );
    }
    assert.equal(got.unresolved, 0, label + " raw");
    assert.equal(got.shortfall, "0", label + " solvency");
    assert.equal(
      (await getAccount(connection, vault)).amount,
      vaultAmount,
      label + " actual SBF vault",
    );
    assert.equal(
      (await getAccount(connection, brokerTokens)).amount,
      brokerAmount,
      label + " actual SPL broker",
    );
    const view = await api.request(request(readId++, { kind: "view" }));
    assert(view.kind === "view", label + " SDK view");
    assert.equal(view.cash, BigInt(c.cash), label + " private cash");
    assert.equal(view.held, held, label + " private holds");
    assert.equal(
      view.positions[0]!.lots,
      BigInt(c.quantity),
      label + " private lots",
    );
    assert.equal(
      view.positions[0]!.basis,
      BigInt(c.basis),
      label + " private basis",
    );
    return got;
  };
  const reconcile = async (event: number) => {
    assert.equal(
      (
        await ask({
          op: "reconcile",
          event,
          cash: v.cash,
          quantity: Number(v.quantity),
          basis: v.basis,
          funding: v.funding,
        })
      ).error,
      undefined,
    );
  };
  const streamedPosition = async (lots: bigint, basis: bigint) => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
      const page = await Promise.race([
        stream!.next(),
        new Promise<never>((_, reject) => {
          timer = setTimeout(
            () => reject(Error("Financial stream deadline")),
            6000,
          );
        }),
      ]);
      assert(!page.done);
      assert.equal(page.value.family, "positions");
      const position = page.value.rows.find((row) => row.kind === "position");
      assert(position?.kind === "position");
      assert.equal(position.lots, lots);
      assert.equal(position.basis, basis);
    } finally {
      clearTimeout(timer);
    }
  };
  const counters = (sequence = 0, paid = 0, recipient = Array(32).fill(0)) => ({
    sequence,
    paid,
    recipient_tokens: recipient,
    expires_at_slot: Number.MAX_SAFE_INTEGER,
  });
  // Native deposit ABI/signature is checked but its receipt is synthetic. There
  // is deliberately no native SBF deployment or HTTP action in this offline run.
  const chain = async (
    id: number,
    rail: "Release" | "Deposit" | "Return" | "Payout",
    amount: bigint,
    sequence = 0,
    paid = 0,
  ) => {
    const prepared = await ask({
      op: "prepare",
      id,
      rail,
      amount: Number(amount),
      counters: counters(
        sequence,
        paid,
        rail === "Payout" ? bytes(source) : undefined,
      ),
    });
    assert(prepared.contract, "controller did not prepare " + rail);
    const contract = Buffer.from(prepared.contract),
      built = await fundingInstruction(program, contract),
      block = await connection.getLatestBlockhash();
    const k = rail === "Return" || rail === "Deposit" ? broker : funds,
      tx = new Transaction({ feePayer: k.publicKey, ...block }).add(
        built.instruction,
      );
    tx.sign(k);
    const checked = await verifyFundingWire(program, contract, tx.serialize());
    assert.equal(
      (
        await ask({
          op: "wire",
          id,
          signature: bytes(checked.signature),
          wire: bytes(checked.wire),
        })
      ).error,
      undefined,
      "persist before send",
    );
    if (rail === "Deposit")
      await burn(
        connection,
        governance,
        brokerTokens,
        mint,
        broker,
        amount,
        [],
        setupConfirm,
      ); // fake native custody debit
    else {
      const signature = await connection.sendRawTransaction(checked.wire, {
        skipPreflight: true,
      });
      assert.equal(
        (
          await connection.confirmTransaction(
            { signature, ...block },
            "finalized",
          )
        ).value.err,
        null,
      );
    }
    const auth = JSON.parse(contract.toString()),
      receipt = {
        id,
        signature: bytes(checked.signature),
        slot: await connection.getSlot("finalized"),
        network: bytes(tag(1)),
        program:
          rail === "Deposit"
            ? bytes(native.publicKey)
            : bytes(program.programId),
        mint: bytes(mint),
        source:
          rail === "Return" || rail === "Deposit"
            ? bytes(brokerTokens)
            : bytes(vault),
        destination:
          rail === "Release"
            ? bytes(brokerTokens)
            : rail === "Deposit"
              ? bytes(venueVault)
              : rail === "Return"
                ? bytes(vault)
                : bytes(source),
        amount: Number(amount),
        operation: rail === "Deposit" ? null : auth.operation,
        epoch: rail === "Deposit" ? null : Number(auth.epoch),
        config: rail === "Deposit" ? null : bytes(config),
        paid: rail === "Payout" ? paid + Number(amount) : null,
        sequence: rail === "Release" || rail === "Payout" ? sequence + 1 : null,
      };
    if (rail !== "Deposit") {
      const onchain = await program.account.movementReceipt.fetch(
        receiptAddress(
          program.programId,
          config,
          Uint8Array.from(auth.operation),
        ),
      );
      assert.equal(onchain.amount.toString(), String(amount));
      assert.equal(onchain.epoch.toString(), auth.epoch);
      assert.equal(
        onchain.destination.toBase58(),
        new PublicKey(receipt.destination).toBase58(),
      );
    }
    // Wrong destination cannot advance a controller or spend a second time.
    const before = (await ask({ op: "oracle" })) as Oracle;
    assert(
      (
        await ask({
          op: "chain",
          receipt: { ...receipt, destination: bytes(owner.publicKey) },
        })
      ).error,
    );
    assert.deepEqual(await ask({ op: "oracle" }), before);
    assert.equal(
      (await ask({ op: "chain", receipt })).error,
      undefined,
      "qualified " + rail,
    );
    assert.equal(
      (await ask({ op: "chain", receipt })).error,
      undefined,
      "duplicate " + rail,
    );
    return checked.signature;
  };
  const trade = async (
    id: number,
    event: number,
    lots: bigint,
    price: bigint,
    fee: bigint,
    pnl: bigint,
  ) => {
    const o = ((await ask({ op: "oracle" })) as Oracle).orders.find(
      (o) => o.id[0] === id,
    )!;
    const body = JSON.stringify({
      success: true,
      has_more: false,
      data: [
        {
          history_id: event,
          order_id: 900 + id,
          client_order_id: o.client_id,
          symbol: "BTC",
          amount: String(lots < 0n ? -lots : lots),
          price: String(price),
          entry_price: "100",
          fee: String(fee),
          pnl: String(pnl - fee),
          event_type: "fulfill_taker",
          side: lots > 0n ? "open_long" : "close_long",
          cause: "normal",
          created_at: event,
        },
      ],
    });
    privateBodies.push(Buffer.from(body));
    assert.equal(
      (await ask({ op: "trades", body, through: event })).error,
      undefined,
    );
    const before = await ask({ op: "oracle" });
    assert.equal(
      (await ask({ op: "trades", body, through: event })).error,
      undefined,
    );
    const after = (await ask({ op: "oracle" })) as Oracle;
    const { ledger_version: beforeVersion, ...beforeEconomics } = before;
    const { ledger_version: afterVersion, ...afterEconomics } = after;
    // A distinct raw observation is retained even when its economic ID has
    // already posted. Audit progress is not a second economic posting.
    assert.equal(
      afterVersion,
      beforeVersion + 1,
      "duplicate observation retained",
    );
    assert.deepEqual(
      afterEconomics,
      beforeEconomics,
      "REST duplicate no economics",
    );
  };
  try {
    await restart();
    await boundary("empty customer, explicit house");
    const deposit = movement(domain, 1n, tag(80), (1n << 64n) - 1n);
    const ix = await program.methods
      .deposit(deposit, u64(1000n))
      .accountsStrict({
        config,
        vault,
        mint,
        customer,
        owner: owner.publicKey,
        source,
        receipt: depositReceiptAddress(
          program.programId,
          config,
          owner.publicKey,
          tag(80),
        ),
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
    const block = await connection.getLatestBlockhash(),
      tx = new Transaction({ feePayer: owner.publicKey, ...block }).add(ix);
    tx.sign(owner);
    const signature = await connection.sendRawTransaction(tx.serialize(), {
      skipPreflight: true,
    });
    assert.equal(
      (
        await connection.confirmTransaction(
          { signature, ...block },
          "finalized",
        )
      ).value.err,
      null,
    );
    const deposited = await program.account.movementReceipt.fetch(
      depositReceiptAddress(
        program.programId,
        config,
        owner.publicKey,
        tag(80),
      ),
    );
    assert.equal(deposited.amount.toString(), "1000");
    const credit = {
      op: "deposit",
      amount: 1000,
      signature: bytes(tx.signature!),
      slot: await connection.getSlot("finalized"),
    };
    await ask(credit);
    await ask(credit);
    c = flat(1000n);
    vaultAmount = 1000n;
    await boundary("deposit credited once");
    await chain(81, "Release", 60n);
    vaultAmount -= 60n;
    brokerAmount += 60n;
    await boundary("actual SBF release, not notional");
    const depositSig = await chain(82, "Deposit", 60n);
    brokerAmount -= 60n;
    transit += 60n;
    await boundary("native debit remains transit");
    await ask({
      op: "credit",
      id: 82,
      signature: bytes(depositSig),
      event: 90,
      amount: 30,
      final_credit: false,
    });
    transit -= 30n;
    v.cash = String(BigInt(v.cash) + 30n);
    await boundary("partial native credit");
    const order: Command = {
      kind: "order",
      market: tag(7),
      lots: 2n,
      minimum: 90n,
      maximum: 110n,
      fee: 1n,
      tif: "GTC",
      reduceOnly: false,
      goodUntil: BigInt(Date.now() + 60_000),
    };
    const early = await api.request(request(40, order));
    assert(
      early.kind === "error" ||
        (early.kind === "receipt" && early.outcome === "rejected"),
      "unsettled native ingress allowed trading",
    );
    await ask({
      op: "credit",
      id: 82,
      signature: bytes(depositSig),
      event: 91,
      amount: 30,
      final_credit: true,
    });
    transit -= 30n;
    v.cash = String(BigInt(v.cash) + 30n);
    await reconcile(92);
    await boundary("complete native credit");
    const placed = await api.request(request(41, order));
    assert(placed.kind === "receipt" && placed.possiblyExposed);
    expectedOrders.set(41, { filled: 0, complete: false, cuts: [] });
    held = 24n;
    await boundary("actual risk holds before signed exposure");
    await restart();
    const exposed = ((await ask({ op: "oracle" })) as Oracle).exposed;
    const retry = await api.request(request(41, order));
    assert(retry.kind === "receipt" && retry.possiblyExposed);
    assert.equal(
      ((await ask({ op: "oracle" })) as Oracle).exposed,
      exposed,
      "restart signed twice",
    );
    streamChannel = await WebChannel.connect(
      {
        baseUrl: `http://127.0.0.1:${relay!.server.address().port}`,
        policy,
        core,
        transport: "websocket",
      },
      (q, p, c) => verifyWithRoot(q, p, c, rootDer),
    );
    stream = (
      await new PrivateClient(streamChannel, signer, {
        domain: policy,
        account: tag(1),
        policy: 1,
      }).subscribe({
        id: tag(249),
        epoch: 1n,
        expiresAt: BigInt(Date.now() + 60_000),
        query: { family: "positions", limit: 64 },
      })
    )[Symbol.asyncIterator]();
    await streamedPosition(0n, 0n);
    await trade(41, 100, 1n, 100n, 1n, 0n);
    c = { ...c, cash: "999", quantity: "1", basis: "100" };
    v = { ...v, cash: "1059", quantity: "1", basis: "100" };
    expectedOrders.set(41, { filled: 1, complete: false, cuts: [100] });
    await boundary("partial actual attributed fill");
    await streamedPosition(1n, 100n);
    const cancel = await api.request(
      request(42, {
        kind: "cancel",
        target: tag(41),
        attempt: tag(52),
        goodUntil: BigInt(Date.now() + 60_000),
      }),
    );
    assert(cancel.kind === "receipt");
    await boundary("cancel keeps unresolved fill holds");
    await trade(41, 101, 1n, 100n, 1n, 0n);
    c = { ...c, cash: "998", quantity: "2", basis: "200" };
    v = { ...v, cash: "1058", quantity: "2", basis: "200" };
    expectedOrders.set(41, { filled: 2, complete: false, cuts: [100, 101] });
    await boundary("late fill after cancellation");
    await streamedPosition(2n, 200n);
    await stream.return?.();
    stream = undefined;
    streamChannel.close();
    streamChannel = undefined;
    assert.equal(
      (
        await ask({
          op: "terminal",
          id: 41,
          filled: 2,
          events: [100, 101],
          through: 101,
        })
      ).error,
      undefined,
    );
    held = 0n;
    expectedOrders.set(41, { filled: 2, complete: true, cuts: [100, 101] });
    await reconcile(110);
    await boundary("certified terminal releases only order hold");
    assert.equal(
      (
        await ask({
          op: "funding",
          event: 120,
          numerator: "-2",
          native: "-4",
          settle: true,
        })
      ).error,
      undefined,
    );
    c.cash = "994";
    v.cash = "1054";
    await reconcile(130);
    await boundary("funding recognized and settled once");
    const close: Command = {
      ...order,
      lots: -2n,
      tif: "IOC",
      reduceOnly: true,
      goodUntil: BigInt(Date.now() + 60_000),
    };
    const closing = await api.request(request(43, close));
    assert(closing.kind === "receipt" && closing.possiblyExposed);
    expectedOrders.set(43, { filled: 0, complete: false, cuts: [] });
    held = 24n;
    await boundary("bounded private close");
    await trade(43, 140, -2n, 110n, 2n, 20n);
    c = flat(1012n);
    v = flat(1072n);
    expectedOrders.set(43, { filled: -2, complete: false, cuts: [140] });
    await boundary("close realizes perp PnL, not trade notional");
    await ask({
      op: "terminal",
      id: 43,
      filled: -2,
      events: [140],
      through: 140,
    });
    held = 0n;
    expectedOrders.set(43, { filled: -2, complete: true, cuts: [140] });
    await reconcile(150);
    await boundary("closed and reconciled");
    const queued = await api.request(
      request(44, {
        kind: "payout",
        net: 1012n,
        maximumFee: 0n,
        allowPartial: false,
        goodUntil: BigInt(Date.now() + 60_000),
      }),
    );
    assert(queued.kind === "receipt" && queued.outcome === "accepted");
    held = 1012n;
    assert.equal(
      (
        await ask({
          op: "prepare",
          id: 44,
          rail: "Payout",
          amount: 1012,
          counters: counters(0, 0, bytes(source)),
        })
      ).error,
      "fixture-control-rejected",
      "inaccessible funds allowed payout",
    );
    await boundary("backed claim, insufficient local payout cash");
    await ask({ op: "withdraw", id: 83, amount: 72 });
    // Fake venue settles its native ledger; mint is fixture liquidity, NOT a
    // Pacifica bridge receipt. Actual return/payout then consume those tokens.
    const paymentBlock = await connection.getLatestBlockhash();
    const { createMintToInstruction } = await import("@solana/spl-token");
    const paymentTx = new Transaction({
      feePayer: governance.publicKey,
      ...paymentBlock,
    }).add(
      createMintToInstruction(mint, brokerTokens, governance.publicKey, 72n),
    );
    paymentTx.sign(governance);
    const payment = await connection.sendRawTransaction(paymentTx.serialize(), {
      skipPreflight: true,
    });
    assert.equal(
      (
        await connection.confirmTransaction(
          { signature: payment, ...paymentBlock },
          "finalized",
        )
      ).value.err,
      null,
    );
    const observed = await connection.getTransaction(payment, {
      commitment: "finalized",
      maxSupportedTransactionVersion: 0,
    });
    assert(observed?.meta && !observed.meta.err);
    await ask({
      op: "withdrawal",
      id: 83,
      amount: 72,
      signature: bytes(paymentTx.signature!),
      slot: await connection.getSlot("finalized"),
    });
    v.cash = "1000";
    brokerAmount = 72n;
    await reconcile(160);
    await boundary("unknown withdrawal reconciles to original UUID");
    await chain(84, "Return", 72n);
    brokerAmount = 0n;
    vaultAmount = 1012n;
    await boundary("actual SBF return without customer recredit");
    await chain(44, "Payout", 1012n);
    c = flat(0n);
    vaultAmount = 0n;
    held = 0n;
    await boundary("final beneficiary payout exactly once");
    assert.equal((await getAccount(connection, source)).amount, 1012n);
    assert.equal(
      (await program.account.customerCounter.fetch(customer)).paid.toString(),
      "1012",
    );
    await restart();
    await boundary("final restart has no revived entitlement");
    const final = await ask({ op: "oracle" }),
      started = performance.now();
    for (let i = 0; i < budgets.readCount; i++) {
      const read = await api.request(request(250, { kind: "view" }));
      assert(read.kind === "view");
      assert.equal(read.cash, 0n);
      assert.equal(read.held, 0n);
    }
    const elapsed = performance.now() - started;
    assert(
      elapsed < budgets.readMilliseconds,
      "bounded read workload exceeds declared budget",
    );
    assert.deepEqual(
      await ask({ op: "oracle" }),
      final,
      "read workload mutated the financial cut",
    );
    const size = async (path: string): Promise<number> => {
      const info = await stat(path);
      if (!info.isDirectory()) {
        const bytes = await readFile(path);
        for (const needle of [storageKey, ...privateBodies])
          assert(
            !bytes.includes(needle),
            "private fixture content leaked to storage",
          );
        return info.size;
      }
      return (
        await Promise.all(
          (await readdir(path)).map((name) => size(join(path, name))),
        )
      ).reduce((a, b) => a + b, 0);
    };
    const journalBytes = await size(store),
      rss = process.memoryUsage().rss;
    assert(
      journalBytes < budgets.journalBytes,
      "fixture journal exceeds declared byte budget",
    );
    assert(
      rss < budgets.nodeRssBytes,
      "Node harness exceeds declared RSS budget",
    );
    process.stdout.write(
      `P22 ${carrier} budgets: ${budgets.readCount} reads ${Math.ceil(elapsed)} ms; journal ${journalBytes} bytes; Node RSS ${rss} bytes\n`,
    );
  } finally {
    streamChannel?.close();
    channel?.close();
    await relay?.close();
    await stop(child);
    storageKey.fill(0);
    configBytes.fill(0);
    await rm(directory, { recursive: true, force: true });
  }
}
