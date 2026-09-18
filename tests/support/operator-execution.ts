/** Real localhost Phoenix + PER/QFS execution. No native-account RPC stubs. */
import * as anchor from "@anchor-lang/core";
import * as rise from "@ellipsis-labs/rise";
import { ComputeBudgetProgram, Connection, Keypair, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import { EPHEMERAL_VAULT_ID, MAGIC_PROGRAM_ID, PERMISSION_PROGRAM_ID, DELEGATION_PROGRAM_ID, permissionPdaFromAccount, getAuthToken, magicFeeVaultPdaFromValidator, delegationRecordPdaFromDelegatedAccount, delegationMetadataPdaFromDelegatedAccount, delegateBufferPdaFromDelegatedAccountAndOwnerProgram, createTopUpEscrowInstruction } from "@magicblock-labs/ephemeral-rollups-sdk";
import { expect } from "chai";
import nacl from "tweetnacl";
import { spawn, execFileSync } from "child_process";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync, readFileSync } from "fs";
import { tmpdir } from "os";
import { join } from "path";
import { createServer } from "http";
import type { AddressInfo } from "net";
import type { CinderLedger } from "../../target/types/cinder_ledger";
import type { CinderVault } from "../../target/types/cinder_vault";
import { refreshLocalMark, settleLocalClock, setLocalFundingGeneration } from "./native-oracle";
import {feeBalance,delegateFeeBalance,magicVaultInstructions} from "../../scripts/provision-per-fees";

export async function verifyOperatorExecution(ctx: {
    endpoint: string; operator: Keypair; vault: anchor.Program<CinderVault>; config: PublicKey;
    native: PublicKey; global: PublicKey; trader: PublicKey; quote: PublicKey;
    source: PublicKey; indexes: string[]; buffers: string[];
    rpc: (method: string, params: unknown[]) => Promise<unknown>;
    send: (ixs: anchor.web3.TransactionInstruction[], pad?: boolean) => Promise<string>;
    postCollateral: (amount: number) => Promise<void>;
}) {
    const qfsEndpoint = "http://127.0.0.1:6699";
    const accountLatencyMs = Number(process.env.CINDER_R5_ACCOUNT_LATENCY_MS ?? "0");
    if (!Number.isInteger(accountLatencyMs) || accountLatencyMs < 0 || accountLatencyMs > 1000) throw new Error("invalid local account-read latency");
    const connection = new Connection(ctx.endpoint, "confirmed");
    for (const endpoint of [ctx.endpoint, qfsEndpoint]) {
        const url = new URL(endpoint);
        if (url.protocol !== "http:" || !["127.0.0.1", "localhost"].includes(url.hostname)) throw new Error("execution proof refuses non-local RPC");
    }
    const ledgerIdl = JSON.parse(readFileSync("target/idl/cinder_ledger.json", "utf8"));
    await ctx.rpc("surfnet_writeProgram", [ledgerIdl.address, readFileSync("target/deploy/cinder_ledger.so").toString("hex"), 0]);
    const ledger = new anchor.Program<CinderLedger>(ledgerIdl, ctx.vault.provider);
    const user = Keypair.generate();
    await ctx.rpc("surfnet_setAccount", [user.publicKey.toBase58(), { lamports: 1000000000 }]);
    const pda = (seed: string, extra: Buffer[] = []) => PublicKey.findProgramAddressSync([Buffer.from(seed), ...extra], ledger.programId)[0];
    const book = pda("book"), fees = pda("fees"), userLedger = pda("user", [user.publicKey.toBuffer()]);
    const validator = new PublicKey("mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev");
    await ctx.send([await ctx.vault.methods.setAllowlist([1]).accountsPartial({ admin: ctx.operator.publicKey, config: ctx.config }).instruction()]);
    await ctx.send([await ledger.methods.initialize().accountsPartial({ adapter: ctx.operator.publicKey, config: ctx.config, book, feeAccrual: fees, systemProgram: SystemProgram.programId }).instruction()]);
    await ledger.methods.initUser().accountsPartial({ adapter: ctx.operator.publicKey, user: user.publicKey, config: ctx.config, userLedger, book, systemProgram: SystemProgram.programId }).signers([user]).rpc();
    await ctx.send([await ledger.methods.creditDeposit(new anchor.BN(100000000)).accountsPartial({ adapter: ctx.operator.publicKey, config: ctx.config, book, userLedger }).instruction()]);
    const healthyUser = process.env.CINDER_R6_MAINTENANCE === "1" ? Keypair.generate() : undefined;
    const healthyLedger = healthyUser && pda("user", [healthyUser.publicKey.toBuffer()]);
    if (healthyUser && healthyLedger) {
        // Additional private cash is backed by an explicit genesis mint fixture
        // in the disposable vault, never by an invented accounting correction.
        const cash = await connection.getAccountInfo(ctx.source);
        if (!cash) throw new Error("missing collateral fixture");
        const data = Buffer.from(cash.data);data.writeBigUInt64LE(data.readBigUInt64LE(64) + 100000000n, 64);
        await ctx.rpc("surfnet_setAccount", [ctx.source.toBase58(), { data: data.toString("hex") }]);
        await ctx.rpc("surfnet_setAccount", [healthyUser.publicKey.toBase58(), { lamports: 1000000000 }]);
        await ledger.methods.initUser().accountsPartial({ adapter: ctx.operator.publicKey, user: healthyUser.publicKey, config: ctx.config, userLedger: healthyLedger, book, systemProgram: SystemProgram.programId }).signers([healthyUser]).rpc();
        await ctx.send([await ledger.methods.creditDeposit(new anchor.BN(100000000)).accountsPartial({ adapter: ctx.operator.publicKey, config: ctx.config, book, userLedger: healthyLedger }).instruction()]);
    }
    await ctx.send([await ledger.methods.delegateBook().accountsPartial({ adapter: ctx.operator.publicKey, config: ctx.config, book, validator }).instruction()]);
    await ctx.send([await ledger.methods.delegateFees().accountsPartial({ adapter: ctx.operator.publicKey, config: ctx.config, feeAccrual: fees, validator }).instruction()]);
    await ledger.methods.delegateUser().accountsPartial({ adapter: ctx.operator.publicKey, user: user.publicKey, config: ctx.config, userLedger, validator }).signers([user]).rpc();
    if (healthyUser && healthyLedger) await ledger.methods.delegateUser().accountsPartial({ adapter: ctx.operator.publicKey, user: healthyUser.publicKey, config: ctx.config, userLedger: healthyLedger, validator }).signers([healthyUser]).rpc();
    const auth = await getAuthToken(qfsEndpoint, ctx.operator.publicKey, message => Promise.resolve(nacl.sign.detached(message, ctx.operator.secretKey)));
    const qfs = new Connection(`${qfsEndpoint}?token=${auth.token}`, { wsEndpoint: `ws://127.0.0.1:6700?token=${auth.token}`, commitment: "confirmed" });
    for (const address of [book, fees, userLedger, ...(healthyLedger ? [healthyLedger] : [])]) {
        const deadline = performance.now() + 15000;
        for (;;) {
            const account = await qfs.getAccountInfo(address);
            if (account?.owner.equals(ledger.programId)) break;
            if (performance.now() >= deadline) throw new Error("delegation did not become visible through local QFS");
            await new Promise(resolve => setTimeout(resolve, 250));
        }
    }
    const privateLedger = new anchor.Program<CinderLedger>(ledgerIdl, new anchor.AnchorProvider(qfs, new anchor.Wallet(ctx.operator), { commitment: "confirmed" }));
    const permissionAccounts = (account: PublicKey) => ({ adapter: ctx.operator.publicKey, config: ctx.config, permission: permissionPdaFromAccount(account), magicProgram: MAGIC_PROGRAM_ID, permissionProgram: PERMISSION_PROGRAM_ID, ephemeralVault: EPHEMERAL_VAULT_ID });
    await privateLedger.methods.initBookPermission().accountsPartial({ ...permissionAccounts(book), book }).rpc({ skipPreflight: true });
    await privateLedger.methods.initFeesPermission().accountsPartial({ ...permissionAccounts(fees), feeAccrual: fees }).rpc({ skipPreflight: true });
    await privateLedger.methods.initUserPermission().accountsPartial({ ...permissionAccounts(userLedger), userLedger }).rpc({ skipPreflight: true });
    if (healthyLedger) await privateLedger.methods.initUserPermission().accountsPartial({ ...permissionAccounts(healthyLedger), userLedger: healthyLedger }).rpc({ skipPreflight: true });
    await privateLedger.methods.updateBookCollateral(new anchor.BN(50000000)).accountsPartial({ adapter: ctx.operator.publicKey, config: ctx.config, book }).rpc();
    const publicConfig = await ctx.vault.account.config.fetch(ctx.config);
    expect(publicConfig.adapter.equals(ctx.operator.publicKey), "fixture operator binding").to.equal(true);
    expect(publicConfig.phoenixTrader.equals(ctx.trader), "fixture trader binding").to.equal(true);
    expect(publicConfig.vaultAuthority.equals(PublicKey.findProgramAddressSync([Buffer.from("vault-authority")], ctx.vault.programId)[0]), "fixture custody binding").to.equal(true);
    const discriminator = ledger.idl.accounts!.find(a => a.name.replace(/_/g, "").toLowerCase() === "userledger")!.discriminator;
    const bs58 = require("bs58");
    // Surfpool 1.5's owner index is stale after CPI assign. Capture its two
    // actual registry scans once: this fixture creates no further users. Every
    // subsequent registry response reads these accounts afresh and filters
    // CURRENT ownership. This also avoids repeated upstream-mainnet discovery
    // of the fork's disposable accounts. Financial RPCs are never cached.
    const owners = [ledger.programId.toBase58(), "DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh"];
    let registryKeys: string[] | undefined;
    let nativeReadOpen = false;
    let nativeProbe = "not read";
    let openingRows = "";
    let openingTime = 0;
    let lastBlockAgeMs = 0;
    const publicRpc = async (method: string, params: unknown[]) => {
        // A native read can abort before its closing bracket. Registry scans
        // occur outside that bracket and start a new pass, so reset the oracle
        // fixture marker instead of assuming every failed read consumed a pair.
        if (method === "getProgramAccounts") nativeReadOpen = false;
        if (method === "getMultipleAccounts" && (params[0] as string[]).length >= 7 && (params[0] as string[]).includes(assetMap.toBase58())) {
            if (!nativeReadOpen) {
                // Deliberately model RPC latency: startup observations may age
                // out while a later, independently fresh admission succeeds.
                // Delay before publishing the oracle fixture, so the injected
                // latency does not itself stale native slot-based components.
                if (accountLatencyMs) await new Promise(resolve => setTimeout(resolve, accountLatencyMs));
                await settleLocalClock(connection,ctx.rpc);
                await refreshMark();
            }
            nativeReadOpen = !nativeReadOpen;
        }
        if (method !== "getProgramAccounts" || !owners.includes(String(params[0]))) {
            const result = await ctx.rpc(method, params);
            if (method === "getBlockTime" && typeof result === "number") lastBlockAgeMs = Date.now() - result * 1000;
            if (method === "getMultipleAccounts" && (params[0] as string[]).length >= 7 && (params[0] as string[]).includes(assetMap.toBase58())) {
                const rows = JSON.stringify((result as { value: unknown }).value);
                if (nativeReadOpen) { openingRows = rows; openingTime = Date.now(); }
                else nativeProbe = `stable=${openingRows === rows}, elapsedMs=${Date.now()-openingTime}, blockAgeMs=${lastBlockAgeMs}`;
            }
            return result;
        }
        type Row = { pubkey: string; account: { owner: string; executable: boolean; data: [string, string] } };
        const merged = new Map<string, Row>();
        for (const owner of registryKeys ? [] : owners) {
            const rows = await ctx.rpc(method, [owner, ...params.slice(1)]) as Row[];
            for (const row of rows) {
                if (!owners.includes(row.account.owner) || row.account.executable || !Buffer.from(row.account.data[0], "base64").subarray(0, 8).equals(Buffer.from(discriminator))) throw new Error("untrusted actual registry row");
                const previous = merged.get(row.pubkey);
                if (previous && JSON.stringify(previous) !== JSON.stringify(row)) throw new Error("conflicting actual registry row");
                merged.set(row.pubkey, row);
            }
        }
        registryKeys ??= Array.from(merged.keys());
        const current = await ctx.rpc("getMultipleAccounts", [registryKeys, { encoding: "base64", commitment: "confirmed" }]) as { value: (Row["account"] | null)[] };
        if (current.value.length !== registryKeys.length || current.value.some(row => !row)) throw new Error("incomplete actual fixture registry");
        merged.clear();
        registryKeys.forEach((pubkey, i) => merged.set(pubkey, { pubkey, account: current.value[i]! }));
        return Array.from(merged.values()).filter(row => row.account.owner === params[0]);
    };
    type Crash = "funding" | "native" | "ack" | "fundingEpoch" | "fold";
    let crashAt: Crash | undefined;
    let activeChild: ReturnType<typeof spawn> | undefined;
    const sends = { funding: 0, native: 0, ack: 0, fundingEpoch: 0, fold: 0 };
    const vaultInstructions = new anchor.BorshInstructionCoder(ctx.vault.idl);
    const ledgerInstructions = new anchor.BorshInstructionCoder(ledger.idl);
    function sent(method: string, params: unknown[], privateRpc: boolean) {
        if (method !== "sendTransaction") return false;
        const tx = Transaction.from(Buffer.from(String(params[0]), "base64"));
        let stage: Crash | undefined;
        for (const ix of tx.instructions) {
            if (!privateRpc && ix.programId.equals(ctx.native)) stage = "native";
            if (!privateRpc && ix.programId.equals(ctx.vault.programId) && vaultInstructions.decode(ix.data)?.name.replace(/_/g, "").toLowerCase() === "fundphoenix") stage = "funding";
            if (privateRpc && ix.programId.equals(ledger.programId) && ["ackphoenixfillguarded", "ackphoenixfailguarded"].includes(ledgerInstructions.decode(ix.data)?.name.replace(/_/g, "").toLowerCase() || "")) stage = "ack";
            if (privateRpc && ix.programId.equals(ledger.programId)) {
                const decoded = ledgerInstructions.decode(ix.data);
                const name = decoded?.name.replace(/_/g, "").toLowerCase();
                if (name === "bumpfundingepoch") stage = "fundingEpoch";
                if (name === "allocatefunding" && (decoded!.data as { fold: boolean }).fold) stage = "fold";
            }
        }
        if (!stage) return false;
        sends[stage]++;
        if (stage !== crashAt) return false;
        crashAt = undefined;
        activeChild!.kill("SIGKILL");
        return true;
    }
    const publicServer = createServer(async (request, response) => {
        let requestId: unknown = null;
        try {
            const chunks: Buffer[] = [];
            for await (const chunk of request) { chunks.push(Buffer.from(chunk)); if (chunks.reduce((n,b) => n+b.length,0) > 2*1024*1024) throw new Error("oversized local request"); }
            const input = JSON.parse(Buffer.concat(chunks).toString());
            requestId = input.id;
            recentCalls.push(`L1:${input.method}`);
            if (recentCalls.length > 40) recentCalls.shift();
            const started = performance.now();
            const result = await publicRpc(input.method, input.params);
            if (input.method === "simulateTransaction" && (result as {value:{err:unknown}}).value.err) {
                lastNativeError = JSON.stringify((result as {value:{err:unknown}}).value.err) + " " + ((result as {value:{logs:string[]}}).value.logs || []).filter(line => /Error|Invalid|failed|Cannot|ORACLE|Funding|funding/.test(line)).slice(-25).join("\n");
            }
            recentCalls.push(`L1:${input.method}:done:${Math.round(performance.now()-started)}ms`);
            if (recentCalls.length > 40) recentCalls.shift();
            if (sent(input.method, input.params, false)) { response.destroy(); return; }
            response.end(JSON.stringify({ jsonrpc: "2.0", id: input.id, result }));
        } catch (error) {
            const rpcError = (error as { rpcError?: { code?: number; data?: { err?: unknown; logs?: string[] } } }).rpcError;
            if (rpcError && Number.isInteger(rpcError.code)) {
                lastNativeError = `RPC ${rpcError.code}: ${JSON.stringify(rpcError)} `;
                response.end(JSON.stringify({ jsonrpc: "2.0", id: requestId, error: rpcError }));
                return;
            }
            response.statusCode = 500; response.end("local registry transport failed");
        }
    });
    const recentCalls: string[] = [];
    let lastNativeError = "none";
    const qfsServer = createServer(async (request, response) => {
        try {
            const chunks: Buffer[] = []; for await (const chunk of request) chunks.push(Buffer.from(chunk));
            const body = Buffer.concat(chunks);
            const auth = request.url!.startsWith("/auth/");
            if (!auth && JSON.parse(body.toString()).method === "getProgramAccounts") nativeReadOpen = false;
            recentCalls.push(auth ? "QFS:auth" : `QFS:${JSON.parse(body.toString()).method}`);
            if (recentCalls.length > 40) recentCalls.shift();
            const started = performance.now();
            const upstream = await fetch(`${qfsEndpoint}${request.url}`, { method: request.method, headers: { "content-type": "application/json" }, signal: AbortSignal.timeout(12000), ...(request.method === "POST" ? { body } : {}) });
            const output = Buffer.from(await upstream.arrayBuffer());
            if (!auth && upstream.ok && !JSON.parse(output.toString()).error && sent(JSON.parse(body.toString()).method, JSON.parse(body.toString()).params, true)) { response.destroy(); return; }
            response.statusCode = upstream.status; response.end(output);
            recentCalls.push(`${auth ? "QFS:auth" : `QFS:${JSON.parse(body.toString()).method}`}:done:${Math.round(performance.now()-started)}ms`);
            if (recentCalls.length > 40) recentCalls.shift();
        } catch { response.statusCode = 500; response.end("local QFS transport failed"); }
    });

    const assetMap = new PublicKey("2nHGAaEw3D5dd4hVueaUNoygkQFmoeKqRQWnSPqSMFUC");
    // The fork has no live oracle crank. Refresh the real venue price-component
    // slots/oracle timestamps in local storage, preserving all prices, weights,
    // validity rules and economic metadata. Native execution still validates
    // and recomputes the mark from those components.
    // This is a disclosed oracle fixture, not a synthetic Hawkeye RPC return.
    async function refreshMark() {
        return refreshLocalMark(connection, ctx.rpc, ctx.native, assetMap, "SOL");
    }
    const directory = mkdtempSync(join(tmpdir(), "cinder-live-execution-"));
    try {
        await new Promise<void>(resolve => publicServer.listen(0, "127.0.0.1", resolve));
        await new Promise<void>(resolve => qfsServer.listen(0, "127.0.0.1", resolve));
        const publicUrl = `http://127.0.0.1:${(publicServer.address() as AddressInfo).port}`;
        const privateUrl = `http://127.0.0.1:${(qfsServer.address() as AddressInfo).port}`;
        for (const owner of owners) {
            const rows = await publicRpc("getProgramAccounts", [owner, { encoding: "base64", filters: [{ memcmp: { offset: 0, bytes: bs58.encode(Buffer.from(discriminator)) } }] }]) as { account: { owner: string } }[];
            expect(rows.every(row => row.account.owner === owner), "normalized current owner registry").to.equal(true);
        }
        mkdirSync(join(directory, "locks"), { mode: 0o700 });
        writeFileSync(join(directory, "operator.json"), JSON.stringify(Array.from(ctx.operator.secretKey)), { mode: 0o600 });
        const metadata = await refreshMark();
        const unitQuote = metadata.oraclePrice.markPrice.price.ticks * metadata.staticMarketParams.tickSize;
        const settings = {
            operator_keypair: join(directory, "operator.json"), pool_lock_directory: join(directory, "locks"), l1_rpc_env: "CINDER_LIVE_L1", qfs_rpc_env: "CINDER_LIVE_QFS",
            phoenix_program: ctx.native.toBase58(), phoenix_global_config: ctx.global.toBase58(), phoenix_trader: ctx.trader.toBase58(), phoenix_asset_map: assetMap.toBase58(), phoenix_quote_mint: ctx.quote.toBase58(),
            markets: [{ cinder_asset_id: 1, phoenix_asset_id: metadata.staticMarketParams.assetId, symbol: "SOL" }], global_trader_index: ctx.indexes, active_trader_buffer: ctx.buffers,
            solvency_policy: { max_gross_notional_usdc: 1000000000, market_gross_limits_usdc: { "1": 1000000000 }, scenarios: [{ mark_factors_bps: { "1": 11000 }, close_cost_bps: 100 }, { mark_factors_bps: { "1": 9000 }, close_cost_bps: 100 }] },
            // Explicit sandbox allowance, not an estimate of the UI fee rate.
            // Actual fees (not this 1-USDC ceiling) must be acknowledged.
            execution_policy: { quote_headroom_bps: 2000, market_quote_limits_usdc: { "1": Number(unitQuote * 1000n) }, market_fee_limits_usdc: { "1": 1000000 }, max_admission_lots: 1024 },
            maintenance_policy: {liquidation_slippage_bps:200,liquidation_deadline_slots:64,native_ws_env:"CINDER_LIVE_WS",fee_balance_index:0,minimum_fee_balance_lamports:100000,minimum_magic_vault_lamports:100000},
        };
        const configPath = join(directory, "config.json"), journalPath = join(directory, "private/journal.sqlite");
        writeFileSync(configPath, JSON.stringify(settings));
        async function nativeView(asset: boolean) {
            await refreshMark();
            const accounts = { phoenixProgramAddress: ctx.native.toBase58(), globalConfigurationAddress: ctx.global.toBase58(), traderAccount: ctx.trader.toBase58(), perpAssetMap: assetMap.toBase58(), globalTraderIndex: ctx.indexes, activeTraderBuffer: ctx.buffers };
            const ix = (asset ? rise.buildHawkeyeViewMarginForAssetIx({ ...accounts, assetId: metadata.staticMarketParams.assetId } as never) : rise.buildHawkeyeViewMarginIx(accounts as never)) as unknown as { programAddress: string; accounts: { address: string; role: number }[]; data: Uint8Array };
            const instruction = new anchor.web3.TransactionInstruction({ programId: new PublicKey(ix.programAddress), keys: ix.accounts.map(a => ({ pubkey: new PublicKey(a.address), isWritable: (a.role & 1) !== 0, isSigner: (a.role & 2) !== 0 })), data: Buffer.from(ix.data) });
            const latest = await connection.getLatestBlockhash();
            const tx = new Transaction({ feePayer: ctx.operator.publicKey, recentBlockhash: latest.blockhash }).add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }), instruction);
            tx.sign(ctx.operator);
            const result = await connection.simulateTransaction(tx);
            expect(result.value.err, "actual native accounting view").to.equal(null);
            expect(result.value.returnData?.programId).to.equal(String(rise.HAWKEYE_PROGRAM_ADDRESS));
            return rise.decodeHawkeyeReturnData(Buffer.from(result.value.returnData!.data[0], "base64"));
        }
        const run = async (mode = "execute") => {
            nativeReadOpen = false;
            await settleLocalClock(connection, ctx.rpc);
            await refreshMark();
            return new Promise<{ code: number | null; output: string }>((resolve, reject) => {
                const child = spawn("target/debug/cinder-operator", [mode, configPath, journalPath], { env: { ...process.env, CINDER_LIVE_L1: publicUrl, CINDER_LIVE_QFS: privateUrl, CINDER_LIVE_WS: "ws://127.0.0.1:8990" }, stdio: ["ignore", "pipe", "pipe"] });
                activeChild = child;
                let output = "";
                child.stdout.on("data", data => output += data); child.stderr.on("data", data => output += data);
                child.on("error", reject); child.on("exit", code => resolve({ code, output }));
            });
        };
        const converge = async (caseIndex: number) => {
            // Funding prepare/send/confirm/Book sync, native crash recovery and
            // ACK confirmation each need separate one-shot operator passes.
            // Surfpool can also transiently report a future block time: retain
            // the real freshness rejection and allow a later pass to retry.
            const deadline = performance.now() + 60000;
            let result = await run();
            while ((result.code === 3 || result.code === null || result.code === 1 && result.output.includes("operator runtime: Stale")) && performance.now() < deadline) { await new Promise(resolve => setTimeout(resolve, 250)); result = await run(); }
            let journalState = "";
            if (result.code !== 0) {
                try {
                    journalState = execFileSync("sqlite3", [journalPath, "SELECT state,last_error_code,count(*) FROM operations GROUP BY state,last_error_code; SELECT 'funding',count(*),count(signature),count(confirmed_slot),count(book_synced_slot),count(failed_slot) FROM funding_outbox;"], { encoding: "utf8", timeout: 5000, stdio: ["ignore", "pipe", "pipe"] });
                } catch {
                    journalState = "journal diagnostic unavailable (sqlite3 missing, failed or timed out)";
                }
            }
            expect(result.code, `case ${caseIndex}: ${result.output}\nJournal state/error counts: ${journalState}\nLast native error: ${lastNativeError}\nRecent RPC methods: ${recentCalls.join(", ")}`).to.equal(0);
        };
        const fundingStart = BigInt(Math.floor(Date.now() / 1000) - 24 * 3600 - 60);
        const fundingRate = metadata.fundingAccumulator.cumulativeFundingRate;
        if (process.env.CINDER_R6_MAINTENANCE === "1") {
            await setLocalFundingGeneration(connection, ctx.rpc, ctx.native, assetMap, "SOL", fundingRate, fundingStart);
        }
        const bootstrap = await run("accrue");
        expect(bootstrap.code, bootstrap.output).to.equal(0);
        for (const [caseIndex, request] of [10, 100, -5, -25, 1, 1].entries()) {
            const previousSends = { ...sends };
            crashAt = (["funding", "native", "ack"] as Crash[])[caseIndex];
            if (caseIndex === 1) { settings.execution_policy.market_quote_limits_usdc["1"] = Number(unitQuote * 5n); writeFileSync(configPath, JSON.stringify(settings)); }
            if (caseIndex === 2) { settings.execution_policy.market_quote_limits_usdc["1"] = Number(unitQuote * 1000n); writeFileSync(configPath, JSON.stringify(settings)); }
            await ctx.send([await ctx.vault.methods.setOperatorDown(false).accountsPartial({ adapter: ctx.operator.publicKey, config: ctx.config }).instruction()]);
            await privateLedger.methods.setOperatorDown(false).accountsPartial({ adapter: ctx.operator.publicKey, config: ctx.config, book }).rpc();
            const before = await privateLedger.account.userLedger.fetch(userLedger);
            const beforeFees = await privateLedger.account.feeAccrual.fetch(fees);
            const orderbook = new PublicKey(metadata.staticMarketParams.marketAccount);
            const beforeNativeFees = rise.decodeOrderbook((await connection.getAccountInfo(orderbook))!.data).header.totalTakerQuoteLotFees;
            const current = before.positions[0]?.lots.toNumber() || 0;
            const post = current + request;
            const mark = metadata.oraclePrice.markPrice.price.ticks;
            const limit = caseIndex === 4 ? 1n : request > 0 ? mark * 102n / 100n : mark * 98n / 100n;
            const expiry = caseIndex === 5 ? 1 : await connection.getSlot() + 5000;
            const notional = BigInt(Math.abs(post)) * unitQuote;
            // Perp notional is not collateral. Quote the user's own post-size
            // with the official Rise engine, then apply Cinder's 1.25x IM.
            const risk = metadata.riskParams;
            const marketParams = rise.normalizeMarketParams({
                symbol: "SOL", assetId: metadata.staticMarketParams.assetId,
                markPriceTicks: mark.toString(), tickSize: metadata.staticMarketParams.tickSize.toString(),
                baseLotDecimals: metadata.staticMarketParams.baseLotDecimals,
                leverageTiers: risk.leverageTiers.map(tier => ({ upperBoundSize: tier.upperBoundSize.toString(), maxLeverage: tier.maxLeverage.toString(), limitOrderRiskFactorBps: tier.limitOrderRiskFactor.toString() })),
                riskFactors: { maintenanceMarginFactorBps: risk.riskFactors[0].toString(), backstopMarginFactorBps: risk.riskFactors[1].toString(), highRiskMarginFactorBps: risk.riskFactors[2].toString() },
                cancelOrderRiskFactorBps: risk.cancelOrderRiskFactor.toString(),
                upnlRiskFactor: risk.upnlRiskFactor.toString(), upnlRiskFactorForWithdrawals: risk.upnlRiskFactorForWithdrawals.toString(), isolatedOnly: risk.isolatedOnly !== 0,
            });
            const margin = rise.computeSubaccountMarginFromInputs({
                subaccountIndex: 0, collateralBalanceQuoteLots: before.free.add(before.reserved).toString(),
                markets: [{ symbol: "SOL", position: { basePositionLots: post.toString(), virtualQuotePositionLots: (-BigInt(post) * unitQuote).toString(), entryPriceTicks: mark.toString(), unsettledFundingQuoteLots: "0", accumulatedFundingQuoteLots: "0" } }],
            }, { SOL: marketParams });
            const postIm = (BigInt(margin.margin.initialMarginQuoteLots) * 12500n + 9999n) / 10000n;
            try {
                await privateLedger.methods.placeOrder(1, new anchor.BN(request), Array(16).fill(71 + caseIndex), new anchor.BN(limit.toString()), new anchor.BN(expiry), new anchor.BN(postIm.toString()), new anchor.BN(notional.toString()), false, before.nonce)
                    .accountsPartial({ user: user.publicKey, adapter: ctx.operator.publicKey, config: ctx.config, userLedger, book }).signers([user]).rpc();
            } catch (error) {
                const failure = error as { code?: number; error?: { errorCode?: { code?: string; number?: number } }; logs?: string[] };
                throw new Error(`private placement case ${caseIndex}: ${JSON.stringify(failure.error?.errorCode ?? failure.code)} ${(failure.logs || []).filter(line => /Error Code:|Error Message:/.test(line)).join("\n")}`);
            }
            await converge(caseIndex);
            expect(crashAt, "each crash boundary must actually be exercised").to.equal(undefined);
            expect(sends.native - previousSends.native, "expiry never sends; restart never submits a second IOC").to.equal(caseIndex === 5 ? 0 : 1);
            expect(sends.ack - previousSends.ack, "applied ACK must not be re-signed").to.equal(1);
            expect(sends.funding - previousSends.funding, "restart must not debit the vault twice").to.be.at.most(1);
            const after = await privateLedger.account.userLedger.fetch(userLedger);
            expect(after.pendingOidCount).to.equal(0); expect(after.badDebtUsdc.toString()).to.equal("0");
            const afterFees = await privateLedger.account.feeAccrual.fetch(fees);
            const afterNativeFees = rise.decodeOrderbook((await connection.getAccountInfo(orderbook))!.data).header.totalTakerQuoteLotFees;
            expect(BigInt(afterFees.phoenixFeesPaid.sub(beforeFees.phoenixFeesPaid).toString()), "charge actual native fees, not the policy ceiling").to.equal(afterNativeFees - beforeNativeFees);
            const asset = await nativeView(true), native = await nativeView(false);
            if (asset.kind !== "view_margin_for_asset" || native.kind !== "view_margin") throw new Error("unexpected native accounting return");
            const confirmedBook = await privateLedger.account.book.fetch(book);
            expect(BigInt(after.positions[0].lots.toString()), "I1: private equals native inventory").to.equal(asset.baseLots);
            expect(BigInt(confirmedBook.residuals[0].lots.toString()), "I1: Book equals native inventory").to.equal(asset.baseLots);
            const cash = BigInt(after.free.add(after.reserved).sub(after.badDebtUsdc).toString());
            const vaultAccount = await connection.getAccountInfo(ctx.source);
            if (!vaultAccount) throw new Error("missing real vault cash account");
            const vaultCash = vaultAccount.data.readBigUInt64LE(64);
            expect(cash + BigInt(after.positions[0].unsettledFunding.toString()) + (healthyLedger ? 100000000n : 0n), "I2: basis-aware raw equity").to.equal(vaultCash + native.collateralQuoteLots + native.unsettledFundingQuoteLots + BigInt(after.positions[0].entryQuoteLots.toString()) + asset.virtualQuoteLots);
            const delta = after.positions[0].lots.toNumber() - current;
            if (caseIndex === 1) expect(delta > 0 && delta < request).to.equal(true);
            else expect(delta).to.equal(caseIndex >= 4 ? 0 : request);
            if (caseIndex >= 4) {
                expect(after.positions[0].entryQuoteLots.toString()).to.equal(before.positions[0].entryQuoteLots.toString());
                expect(after.free.add(after.reserved).toString()).to.equal(before.free.add(before.reserved).toString());
            }
            // Restart must not retry an IOC remainder or charge the fee twice.
            const saved = await privateLedger.coder.accounts.encode("userLedger", after);
            await converge(caseIndex);
            expect(await privateLedger.coder.accounts.encode("userLedger", await privateLedger.account.userLedger.fetch(userLedger))).to.deep.equal(saved);
        }
        if (process.env.CINDER_R6_MAINTENANCE==="1") {
            // Installed public validator identity is a disposable genesis
            // signer fixture (like Phoenix onboarding). Actual DLP processors
            // provision/delegate the fee accounts; no PER ownership RPC stubs.
            const balance=feeBalance(ctx.operator.publicKey,0);
            await ctx.send([createTopUpEscrowInstruction(balance,ctx.operator.publicKey,ctx.operator.publicKey,10_000_000,0),delegateFeeBalance(ctx.operator.publicKey,validator,0)]);
            const existingVault = await connection.getAccountInfo(magicFeeVaultPdaFromValidator(validator));
            const vaultSetup = magicVaultInstructions(ctx.operator.publicKey, validator);
            const existingVaultRecord = await connection.getAccountInfo(delegationRecordPdaFromDelegatedAccount(magicFeeVaultPdaFromValidator(validator)));
            if (existingVaultRecord) {
                expect(existingVaultRecord.owner.equals(DELEGATION_PROGRAM_ID)).to.equal(true);
                // Installed genesis includes a delegated validator-owned vault.
                // The service verifies the actual record/authority and PER copy.
            } else if (existingVault) {
                // mb-test-validator installs an initialized DLP-owned vault.
                // Exercise delegation, not a second initialization.
                expect(existingVault.owner.equals(DELEGATION_PROGRAM_ID)).to.equal(true);
                expect(existingVault.data.length).to.equal(8);
                await ctx.send([vaultSetup[1]], true);
            } else { await ctx.send(vaultSetup, true); }
            const vault=magicFeeVaultPdaFromValidator(validator);
            const feeDeadline=performance.now()+15000;
            while (!(await qfs.getAccountInfo(balance))?.owner.equals(SystemProgram.programId) || !(await qfs.getAccountInfo(vault))?.owner.equals(DELEGATION_PROGRAM_ID)) {
                if (performance.now()>feeDeadline) throw new Error("fee delegation did not reach PER");
                await new Promise(resolve=>setTimeout(resolve,100));
            }
            // Start the actual long-running coordinator, with no new user
            // orders or manual maintenance calls. It must scan, heartbeat,
            // synchronize collateral and publish the dirty ACK root itself.
            let output = "";
            function startService() {
                const worker = spawn("target/debug/cinder-operator", ["run", configPath, journalPath], { env: { ...process.env, CINDER_LIVE_L1: publicUrl, CINDER_LIVE_QFS: privateUrl, CINDER_LIVE_WS: "ws://127.0.0.1:8990" }, stdio: ["ignore", "pipe", "pipe"] });
                activeChild = worker;
                worker.stdout.on("data", d => output += d); worker.stderr.on("data", d => output += d);
                const ended = new Promise<{ code: number | null }>((resolve, reject) => { worker.on("error", reject); worker.on("exit", code => resolve({ code })); });
                return { child: worker, ended };
            }
            let {child, ended} = startService();
            try {
                const reserve=PublicKey.findProgramAddressSync([Buffer.from("reserve")],ctx.vault.programId)[0];
                const deadline=performance.now()+60000;let root=await ctx.vault.account.reserveRoot.fetch(reserve);
                while (root.epoch.isZero() || (await privateLedger.account.book.fetch(book)).lastScanMs.isZero()) {
                    if (child.exitCode!==null || performance.now()>deadline) throw new Error(`autonomous publication did not complete: ${output}`);
                    await new Promise(resolve=>setTimeout(resolve,250));root=await ctx.vault.account.reserveRoot.fetch(reserve);
                }
                expect(root.userCount).to.equal(2);expect(root.totalBadDebt.toString()).to.equal("0");
                const scanned=await privateLedger.account.book.fetch(book);expect(scanned.lastScanMs.toNumber()).to.be.greaterThan(0);
                expect(scanned.lastScanMs.toNumber()).to.be.at.most(Date.now());
                const userState=await privateLedger.account.userLedger.fetch(userLedger);
                expect(root.totalFree.add(root.totalReserved).toString()).to.equal(userState.free.add(userState.reserved).add(new anchor.BN(100000000)).toString());
                // Replay 24 observed hourly generations, not 24 wall-clock
                // timers. Each update is a real public native accumulator
                // fixture; only the autonomous service writes private epochs.
                const cashBefore = userState.free.add(userState.reserved).toString();
                const heldLots = BigInt(userState.positions[0].lots.toString());
                const epochBefore = BigInt(scanned.fundingEpoch.toString());
                for (let hour = 1; hour <= 24; hour++) {
                    if (hour === 1) crashAt = "fundingEpoch";
                    await setLocalFundingGeneration(connection, ctx.rpc, ctx.native, assetMap, "SOL", fundingRate + BigInt(hour * 100), fundingStart + BigInt(hour * 3600));
                    const allocationDeadline = performance.now() + 30000;
                    for (;;) {
                        if (hour === 1 && child.signalCode === "SIGKILL") {
                            await ended;
                            expect(crashAt, "funding crash boundary must occur").to.equal(undefined);
                            ({child, ended} = startService());
                        }
                        const b = await privateLedger.account.book.fetch(book);
                        const l = await privateLedger.account.userLedger.fetch(userLedger);
                        const completed = Number(execFileSync("sqlite3", [journalPath, "SELECT count(*) FROM funding_epochs WHERE completed=1;"], { encoding: "utf8" }));
                        if (BigInt(b.fundingEpoch.toString()) === epochBefore + BigInt(hour)
                            && BigInt(l.lastFundingEpoch.toString()) === epochBefore + BigInt(hour) && completed === hour) {
                            expect(BigInt(l.positions[0].unsettledFunding.toString()), `hour ${hour}: signed native funding`).to.equal(-BigInt(hour * 100) * heldLots);
                            expect(l.free.add(l.reserved).toString(), "allocation is not a cash fold").to.equal(cashBefore);
                            const flat = await privateLedger.account.userLedger.fetch(healthyLedger!);
                            expect(flat.lastFundingEpoch.toString()).to.equal(l.lastFundingEpoch.toString());
                            expect(flat.free.toString()).to.equal("100000000");
                            break;
                        }
                        if (child.exitCode !== null || performance.now() > allocationDeadline) throw new Error(`hour ${hour} allocation did not complete: ${output}\nNative observation: ${nativeProbe}, blockAgeMs=${lastBlockAgeMs}\nRecent RPCs: ${recentCalls.join(", ")}\nNative error: ${lastNativeError}`);
                        await new Promise(resolve => setTimeout(resolve, 100));
                    }
                }
                const allocated = await privateLedger.account.userLedger.fetch(userLedger);
                const fundingView = await nativeView(false);
                if (fundingView.kind !== "view_margin") throw new Error("unexpected funding margin view");
                expect(fundingView.unsettledFundingQuoteLots).to.equal(BigInt(allocated.positions[0].unsettledFunding.toString()));
                expect(sends.fundingEpoch, "restart must not re-sign an applied epoch bump").to.equal(24);
                // A genuine user reduction makes Phoenix settle its funding.
                // No private fold is sent by this harness. Restart the daemon
                // and kill it after its fold is accepted to prove exact replay.
                child.kill("SIGTERM");expect((await ended).code, output).to.equal(0);
                const current = BigInt(allocated.positions[0].lots.toString());
                const delta = current > 0n ? -1n : 1n;
                const remaining = current + delta;
                const fresh = await refreshMark();
                const risk = fresh.riskParams;
                const params = rise.normalizeMarketParams({
                    symbol: "SOL", assetId: fresh.staticMarketParams.assetId,
                    markPriceTicks: fresh.oraclePrice.markPrice.price.ticks.toString(), tickSize: fresh.staticMarketParams.tickSize.toString(), baseLotDecimals: fresh.staticMarketParams.baseLotDecimals,
                    leverageTiers: risk.leverageTiers.map(t => ({ upperBoundSize: t.upperBoundSize.toString(), maxLeverage: t.maxLeverage.toString(), limitOrderRiskFactorBps: t.limitOrderRiskFactor.toString() })),
                    riskFactors: { maintenanceMarginFactorBps: risk.riskFactors[0].toString(), backstopMarginFactorBps: risk.riskFactors[1].toString(), highRiskMarginFactorBps: risk.riskFactors[2].toString() },
                    cancelOrderRiskFactorBps: risk.cancelOrderRiskFactor.toString(), upnlRiskFactor: risk.upnlRiskFactor.toString(), upnlRiskFactorForWithdrawals: risk.upnlRiskFactorForWithdrawals.toString(), isolatedOnly: risk.isolatedOnly !== 0,
                });
                const margin = rise.computeSubaccountMarginFromInputs({ subaccountIndex: 0, collateralBalanceQuoteLots: allocated.free.add(allocated.reserved).toString(), markets: [{ symbol: "SOL", position: { basePositionLots: remaining.toString(), virtualQuotePositionLots: allocated.positions[0].entryQuoteLots.neg().toString(), entryPriceTicks: "0", unsettledFundingQuoteLots: allocated.positions[0].unsettledFunding.toString(), accumulatedFundingQuoteLots: "0" } }] }, { SOL: params });
                const postIm = (BigInt(margin.margin.initialMarginQuoteLots) * 12500n + 9999n) / 10000n;
                await ctx.send([await ctx.vault.methods.setOperatorDown(false).accountsPartial({ adapter: ctx.operator.publicKey, config: ctx.config }).instruction()]);
                await privateLedger.methods.setOperatorDown(false).accountsPartial({ adapter: ctx.operator.publicKey, config: ctx.config, book }).rpc();
                const mark = fresh.oraclePrice.markPrice.price.ticks;
                const limit = delta > 0n ? mark * 102n / 100n : mark * 98n / 100n;
                await privateLedger.methods.placeOrder(1, new anchor.BN(delta.toString()), Array(16).fill(97), new anchor.BN(limit.toString()), new anchor.BN(await connection.getSlot() + 5000), new anchor.BN(postIm.toString()), new anchor.BN((remaining < 0n ? -remaining : remaining).toString()).mul(new anchor.BN(unitQuote.toString())), true, allocated.nonce)
                    .accountsPartial({ user: user.publicKey, adapter: ctx.operator.publicKey, config: ctx.config, userLedger, book }).signers([user]).rpc();
                await converge(6);
                const settled = await nativeView(false);
                if (settled.kind !== "view_margin") throw new Error("unexpected settlement margin view");
                expect(settled.unsettledFundingQuoteLots, "native trade must actually settle funding").to.equal(0n);
                const unfurled = await privateLedger.account.userLedger.fetch(userLedger);
                expect(unfurled.positions[0].unsettledFunding.toString()).to.equal(allocated.positions[0].unsettledFunding.toString());
                const cashBeforeFold = BigInt(unfurled.free.add(unfurled.reserved).toString());
                crashAt = "fold";({child, ended} = startService());
                const foldDeadline = performance.now() + 60000;
                for (;;) {
                    if (child.signalCode === "SIGKILL") {
                        await ended;expect(crashAt).to.equal(undefined);({child, ended} = startService());
                    }
                    const folded = await privateLedger.account.userLedger.fetch(userLedger);
                    if (folded.positions[0].unsettledFunding.isZero() && crashAt === undefined && child.signalCode === null) {
                        expect(BigInt(folded.free.add(folded.reserved).toString())).to.equal(cashBeforeFold + BigInt(unfurled.positions[0].unsettledFunding.toString()));
                        expect(sends.fold, "unknown applied fold cannot be signed a second time").to.equal(1);
                        break;
                    }
                    if (child.exitCode !== null || performance.now() > foldDeadline) throw new Error(`observed funding fold did not complete: ${output}`);
                    await new Promise(resolve => setTimeout(resolve, 100));
                }
                // Fund native collateral through real vault-PDA/Ember/Phoenix
                // CPIs before the public feed shock. It remains pooled backing,
                // not new user cash or an operator bailout.
                await ctx.postCollateral(110000000);
                const victim = await privateLedger.account.userLedger.fetch(userLedger);
                const victimLots = BigInt(victim.positions[0].lots.toString());
                expect(victimLots < 0n, "fixture retains a confirmed short").to.equal(true);
                const currentRisk = rise.computeSubaccountMarginFromInputs({ subaccountIndex: 0, collateralBalanceQuoteLots: victim.free.add(victim.reserved).toString(), markets: [{ symbol: "SOL", position: { basePositionLots: victimLots.toString(), virtualQuotePositionLots: victim.positions[0].entryQuoteLots.neg().toString(), entryPriceTicks: "0", unsettledFundingQuoteLots: "0", accumulatedFundingQuoteLots: "0" } }] }, { SOL: params });
                const mm = BigInt(currentRisk.margin.maintenanceMarginQuoteLots);
                const equity = BigInt(currentRisk.margin.effectiveCollateralQuoteLots);
                expect(mm > 0n).to.equal(true);
                const chargePerLot = (equity - mm / 2n) / (-victimLots);
                expect(chargePerLot > 0n).to.equal(true);
                const nativeSends = sends.native;
                const rootBeforeLiquidation = (await ctx.vault.account.reserveRoot.fetch(reserve)).epoch;
                const publicTime = fundingStart + 24n * 3600n + 30n;
                await setLocalFundingGeneration(connection, ctx.rpc, ctx.native, assetMap, "SOL", fundingRate + 2400n - chargePerLot, publicTime);
                const liquidationDeadline = performance.now() + 90000;
                for (;;) {
                    const l = await privateLedger.account.userLedger.fetch(userLedger);
                    const b = await privateLedger.account.book.fetch(book);
                    if (l.pendingOidCount === 0 && l.positionsLen === 0 && b.residualLen === 0) break;
                    if (child.exitCode !== null || performance.now() > liquidationDeadline) throw new Error(`quiet-book liquidation did not complete: ${output}\n${lastNativeError}`);
                    await new Promise(resolve => setTimeout(resolve, 100));
                }
                expect(sends.native - nativeSends, "quiet book must dispatch a real liquidation IOC without a user order").to.equal(1);
                const protectedUser = await privateLedger.account.userLedger.fetch(healthyLedger!);
                expect(protectedUser.free.toString()).to.equal("100000000");
                expect(protectedUser.badDebtUsdc.toString()).to.equal("0");
                const liquidated = await privateLedger.account.userLedger.fetch(userLedger);
                const nativeFlat = await nativeView(false);
                if (nativeFlat.kind !== "view_margin") throw new Error("unexpected flat margin view");
                expect(nativeFlat.positionCount).to.equal(0);
                const finalVault = (await connection.getAccountInfo(ctx.source))!.data.readBigUInt64LE(64);
                const privateCash = BigInt(liquidated.free.add(liquidated.reserved).sub(liquidated.badDebtUsdc).toString()) + 100000000n;
                expect(privateCash, "I2 after quiet-book liquidation").to.equal(finalVault + nativeFlat.collateralQuoteLots);
                const incidentDeadline = performance.now() + 60000;
                for (;;) {
                    const published = await ctx.vault.account.reserveRoot.fetch(reserve);
                    if (published.epoch.gt(rootBeforeLiquidation) && published.totalFree.add(published.totalReserved).sub(published.totalBadDebt).toString() === privateCash.toString()) {
                        expect(published.userCount).to.equal(2);
                        break;
                    }
                    if (child.exitCode !== null || performance.now() > incidentDeadline) throw new Error(`liquidation root did not finalize: ${output}`);
                    await new Promise(resolve => setTimeout(resolve, 100));
                }
            } finally {child.kill("SIGTERM");const result=await ended;expect(result.code,output).to.equal(0);}
        }
        expect((await ctx.vault.account.config.fetch(ctx.config)).paused & 32).to.equal(32);
        let anonymousVisible = false;
        try { anonymousVisible = !!await new Connection(qfsEndpoint, "confirmed").getAccountInfo(userLedger); } catch { /* denied */ }
        expect(anonymousVisible, "anonymous QFS must not reveal the private ledger").to.equal(false);
    } finally { await Promise.all([publicServer, qfsServer].map(server => new Promise<void>(resolve => server.close(() => resolve())))); rmSync(directory, { recursive: true, force: true }); }
}
