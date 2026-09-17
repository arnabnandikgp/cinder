/** Opt-in native proof. Only localhost receives transactions or state overrides. */
import * as anchor from "@anchor-lang/core";
import * as rise from "@ellipsis-labs/rise";
import { Connection, Keypair, PublicKey, SystemProgram, Transaction, TransactionInstruction, ComputeBudgetProgram, SYSVAR_INSTRUCTIONS_PUBKEY } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync, getAccount, createAssociatedTokenAccountInstruction } from "@solana/spl-token";
import { readFileSync } from "fs";
import { expect } from "chai";
import permission from "../scripts/fixtures/phoenix-referral-activation-permission.json";
import type { CinderVault } from "../target/types/cinder_vault";
import type { CinderLedger } from "../target/types/cinder_ledger";
import { createHash } from "crypto";
import { mkdtempSync, writeFileSync, rmSync } from "fs";
import { tmpdir } from "os";
import { join } from "path";
import { execFileSync } from "child_process";
import { verifyOperatorExecution } from "./support/operator-execution";
import { refreshLocalMark } from "./support/native-oracle";
import { activateLocalPhoenix } from "../scripts/fixtures/local-phoenix";
import { confirmForkTransaction } from "./support/native-confirmation";
type KitIx = {
    programAddress: string;
    data: ArrayLike<number>;
    accounts: readonly {
        address: string;
        role: number;
    }[];
};
function web3(ix: KitIx): TransactionInstruction {
    return new TransactionInstruction({ programId: new PublicKey(ix.programAddress), data: Buffer.from(ix.data), keys: ix.accounts.map(a => ({ pubkey: new PublicKey(a.address), isSigner: a.role >= 2, isWritable: (a.role & 1) !== 0 })) });
}
describe("runtime atomic PDA funding (native Phoenix fork)", function () {
    this.timeout(180000);
    const endpoint = process.env.PROVIDER_ENDPOINT || "http://127.0.0.1:8989";
    let connection: Connection;
    let vault: anchor.Program<CinderVault>;
    const operator = Keypair.generate();
    const native = new PublicKey("EtrnLzgbS7nMMy5fbD42kXiUzGg8XQzJ972Xtk1cjWih");
    const global = new PublicKey("2zskx2iyCvb6Stg7RBZkt1f6MrF4dpYtMG3yMvKwqtUZ");
    const log = new PublicKey("GdxfTLSsdSY37G6fZoYtdGDSfgFnbT2EmRpuePZxWShS");
    const usdc = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
    const ember = new PublicKey("EMBERpYNE6ehWmXymZZS2skiFmCa9V5dp14e1iduM5qy");
    let quote: PublicKey, nativeVault: PublicKey, authority: PublicKey, config: PublicKey, trader: PublicKey, source: PublicKey, transit: PublicKey;
    let indexes: string[], buffers: string[];
    const amount = 50000000;
    const id = Array(32).fill(41);
    async function rpc(method: string, params: unknown[]) {
        const response = await fetch(endpoint, { method: "POST", redirect: "error", headers: { "content-type": "application/json" }, body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }), signal: AbortSignal.timeout(30000) });
        const body = await response.json() as {
            error?: unknown;
            result: unknown;
        };
        if (!response.ok || body.error)
            throw Object.assign(new Error(`local fork ${method} failed`), { rpcError: body.error });
        return body.result;
    }
    async function send(ixs: TransactionInstruction[], padOnboarder = false) {
        const latest = await connection.getLatestBlockhash();
        const tx = new Transaction({ feePayer: operator.publicKey, recentBlockhash: latest.blockhash });
        tx.add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }), ...ixs);
        tx.partialSign(operator);
        // The sandbox skips signature verification solely for native onboarding.
        // All Cinder calls have a real operator signature and use invoke_signed.
        if (padOnboarder) {
            const message = tx.compileMessage();
            for (let i = 0; i < message.header.numRequiredSignatures; i++)
                if (!tx.signatures[i].signature)
                    tx.addSignature(message.accountKeys[i], Buffer.alloc(64));
        }
        const encoded = tx.serialize({ verifySignatures: !padOnboarder });
        // Run even deliberately failing transactions through native simulation;
        // sending then verifies that failure leaves actual local state unchanged.
        await rpc("simulateTransaction", [encoded.toString("base64"), { encoding: "base64", sigVerify: false }]);
        const signature = await connection.sendRawTransaction(encoded, { skipPreflight: true, maxRetries: 0 });
        const result = await confirmForkTransaction(connection, { ...latest, signature });
        if (result.value.err) {
            const receipt = await connection.getTransaction(signature, { commitment: "confirmed", maxSupportedTransactionVersion: 0 });
            throw new Error(`local transaction failed: ${JSON.stringify(result.value.err)}\n${receipt?.meta?.logMessages?.join("\n")}`);
        }
        return signature;
    }
    function receipt(fundingId: number[]) {
        return PublicKey.findProgramAddressSync([Buffer.from("phoenix-funding"), Buffer.from(fundingId)], vault.programId)[0];
    }
    async function funding(fundingId: number[], value: number, signer = operator.publicKey) {
        return vault.methods.fundPhoenix(fundingId, new anchor.BN(value), indexes.length).accountsPartial({
            adapter: signer, config, vaultAuthority: authority, fundingReceipt: receipt(fundingId), usdcMint: usdc, vaultUsdcAta: source,
            phoenixQuoteMint: quote, vaultPhoenixAta: transit, emberProgram: ember,
            emberState: PublicKey.findProgramAddressSync([native.toBuffer(), Buffer.from("state")], ember)[0],
            emberVault: PublicKey.findProgramAddressSync([native.toBuffer(), Buffer.from("vault")], ember)[0],
            phoenixProgram: native, phoenixLogAuthority: log, phoenixGlobalConfig: global, phoenixTrader: trader, phoenixGlobalVault: nativeVault,
            tokenProgram: TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId,
        }).remainingAccounts([...indexes, ...buffers].map(s => ({ pubkey: new PublicKey(s), isSigner: false, isWritable: true }))).instruction();
    }
    before(async function () {
        if (process.env.CINDER_R5_FORK !== "1") {
            this.skip();
            return;
        }
        const url = new URL(endpoint);
        if (url.protocol !== "http:" || !["127.0.0.1", "localhost"].includes(url.hostname))
            throw new Error("R5 proof refuses non-local RPC");
        connection = new Connection(endpoint, "confirmed");
        expect(await connection.getGenesisHash()).to.equal("5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d");
        await rpc("surfnet_setAccount", [operator.publicKey.toBase58(), { lamports: 10000000000 }]);
        const idl = JSON.parse(readFileSync("target/idl/cinder_vault.json", "utf8"));
        await rpc("surfnet_writeProgram", [idl.address, readFileSync("target/deploy/cinder_vault.so").toString("hex"), 0]);
        const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(operator), { commitment: "confirmed" });
        vault = new anchor.Program<CinderVault>(idl, provider);
        authority = PublicKey.findProgramAddressSync([Buffer.from("vault-authority")], vault.programId)[0];
        config = PublicKey.findProgramAddressSync([Buffer.from("config")], vault.programId)[0];
        trader = new PublicKey(await rise.getPhoenixTraderSubaccountAddress({ authority: authority.toBase58() as never, traderPdaIndex: 0, subaccountIndex: 0, phoenixProgramAddress: native.toBase58() as never }));
        const decoded = await activateLocalPhoenix(connection);
        quote = new PublicKey(decoded.canonicalTokenMintKey);
        nativeVault = new PublicKey(decoded.globalVaultKey);
        const lookup = { addresses: { globalConfigurationAddress: global.toBase58(), phoenixProgramAddress: native.toBase58() }, fetchAccount: async (address: string) => ({ data: (await connection.getAccountInfo(new PublicKey(address)))!.data }) };
        indexes = await rise.getGlobalTraderIndexAddresses(lookup as never);
        buffers = await rise.getActiveTraderBufferAddresses(lookup as never);
        source = getAssociatedTokenAddressSync(usdc, authority, true);
        transit = getAssociatedTokenAddressSync(quote, authority, true);
        const initialize = await vault.methods.initialize(operator.publicKey, authority, trader, new PublicKey("mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev")).accountsPartial({
            admin: operator.publicKey, config, vaultAuthority: authority, reserveRoot: PublicKey.findProgramAddressSync([Buffer.from("reserve")], vault.programId)[0],
            usdcMint: usdc, vaultUsdcAta: source, tokenProgram: TOKEN_PROGRAM_ID, associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId,
        }).instruction();
        await send([initialize, createAssociatedTokenAccountInstruction(operator.publicKey, transit, authority, quote)]);
        await send([web3(rise.buildRegisterTraderIx({ payer: operator.publicKey.toBase58(), trader: authority.toBase58(), traderAccount: trader.toBase58(), maxPositions: 128n, traderPdaIndex: 0, traderSubaccountIndex: 0 } as never) as KitIx)]);
        await send([web3(rise.buildOnboardTraderDelegatedIx({ authority: permission.trader_onboarder, permissionAccount: permission.permission_account, traderAccount: trader.toBase58(), activeTraderBuffer: buffers, globalTraderIndex: indexes } as never) as KitIx)], true);
        await send([await vault.methods.delegatePhoenixTrader().accountsPartial({ adapter: operator.publicKey, config, vaultAuthority: authority, phoenixProgram: native, phoenixLogAuthority: log, phoenixGlobalConfig: global, phoenixTrader: trader }).instruction()]);
        await rpc("surfnet_setTokenAccount", [authority.toBase58(), usdc.toBase58(), { amount: amount * 2 }]);
    });
    it("funds the native cross trader atomically with actual vault PDA CPIs", async () => {
        const nativeBefore = (await getAccount(connection, nativeVault)).amount;
        await send([await funding(id, amount)]);
        expect((await getAccount(connection, source)).amount).to.equal(BigInt(amount));
        expect((await getAccount(connection, transit)).amount).to.equal(0n);
        expect((await getAccount(connection, nativeVault)).amount - nativeBefore).to.equal(BigInt(amount));
        const proof = await vault.account.phoenixFundingReceipt.fetch(receipt(id));
        expect(proof.amount.toString()).to.equal(String(amount));
        expect(proof.phoenixTrader.equals(trader)).to.equal(true);
        expect(proof.phoenixProgram.equals(native)).to.equal(true);
    });
    it("rejects replay without a second vault debit", async () => {
        let failed = false;
        try {
            await send([await funding(id, amount)]);
        }
        catch {
            failed = true;
        }
        expect(failed).to.equal(true);
        expect((await getAccount(connection, source)).amount).to.equal(BigInt(amount));
    });
    it("rolls back an insufficient USDC conversion and its receipt", async () => {
        const failedId = Array(32).fill(42);
        const nativeBefore = (await getAccount(connection, nativeVault)).amount;
        let failed = false;
        try {
            await send([await funding(failedId, amount + 1)]);
        }
        catch {
            failed = true;
        }
        expect(failed).to.equal(true);
        expect((await getAccount(connection, source)).amount).to.equal(BigInt(amount));
        expect((await getAccount(connection, transit)).amount).to.equal(0n);
        expect((await getAccount(connection, nativeVault)).amount).to.equal(nativeBefore);
        expect(await connection.getAccountInfo(receipt(failedId))).to.equal(null);
    });
    it("rolls back a successful Ember conversion when the native deposit fails", async () => {
        const failedId = Array(32).fill(43);
        const nativeBefore = (await getAccount(connection, nativeVault)).amount;
        const original = indexes;
        // Deliberately malformed native-owned spill page, created only on the
        // disposable fork. Phoenix must reject its layout after Ember succeeds.
        const badPage = Keypair.generate().publicKey;
        await rpc("surfnet_setAccount", [badPage.toBase58(), { owner: native.toBase58(), lamports: 1000000, data: "00".repeat(224) }]);
        indexes = [...indexes, badPage.toBase58()];
        let message = "";
        try {
            await send([await funding(failedId, amount)]);
        }
        catch (error) {
            message = error instanceof Error ? String(error) : JSON.stringify(error);
        }
        finally {
            indexes = original;
        }
        expect(message).to.include(`Program ${ember.toBase58()} success`);
        expect(message).to.include(`Program ${native.toBase58()} failed`);
        expect((await getAccount(connection, source)).amount).to.equal(BigInt(amount));
        expect((await getAccount(connection, transit)).amount).to.equal(0n);
        expect((await getAccount(connection, nativeVault)).amount).to.equal(nativeBefore);
        expect(await connection.getAccountInfo(receipt(failedId))).to.equal(null);
    });
    it("confirms posted cash through the authoritative hot/cold Hawkeye view", async () => {
        const view = web3(rise.buildHawkeyeViewMarginIx({ phoenixProgramAddress: native.toBase58(), globalConfigurationAddress: global.toBase58(), traderAccount: trader.toBase58(),
            perpAssetMap: "2nHGAaEw3D5dd4hVueaUNoygkQFmoeKqRQWnSPqSMFUC", globalTraderIndex: indexes, activeTraderBuffer: buffers } as never) as KitIx);
        const latest = await connection.getLatestBlockhash();
        const tx = new Transaction({ feePayer: operator.publicKey, recentBlockhash: latest.blockhash }).add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }), view);
        const result = await rpc("simulateTransaction", [tx.serialize({ requireAllSignatures: false, verifySignatures: false }).toString("base64"), { encoding: "base64", sigVerify: false }]) as {
            value: {
                err: unknown;
                returnData: {
                    programId: string;
                    data: [
                        string,
                        string
                    ];
                };
            };
        };
        expect(result.value.err).to.equal(null);
        expect(result.value.returnData.programId).to.equal(String(rise.HAWKEYE_PROGRAM_ADDRESS));
        const health = rise.decodeHawkeyeReturnData(Buffer.from(result.value.returnData.data[0], "base64"));
        if (health.kind !== "view_margin")
            throw new Error("unexpected native view");
        expect(health.collateralQuoteLots).to.equal(BigInt(amount));
        expect(health.positionCount).to.equal(0);
        expect(health.initialMarginQuoteLots).to.equal(0n);
        expect(health.isLiquidatable).to.equal(false);
    });
    it("executes through the real operator and private PER/QFS ledger", async function () {
        // Six scenarios may each need up to 60 seconds to converge after
        // funding/native/ACK crash boundaries on a slower Linux runner.
        this.timeout(420000);
        if (process.env.CINDER_R5_PRIVATE !== "1") { this.skip(); return; }
        await verifyOperatorExecution({ endpoint, operator, vault, config, native, global, trader, quote, source, indexes, buffers, rpc, send });
    });
    it("verifies native multi-price reversal math through the Rust recovery decoder", async function () {
        if (process.env.CINDER_R5_PRIVATE === "1") { this.skip(); return; }
        const assetMap = new PublicKey("2nHGAaEw3D5dd4hVueaUNoygkQFmoeKqRQWnSPqSMFUC");
        const mapInfo = await connection.getAccountInfo(assetMap);
        expect(mapInfo?.owner.equals(native)).to.equal(true);
        const map = rise.decodePerpAssetMap(mapInfo!.data);
        const metadata = map.metadata.entries.find(e => e.key === "SOL")!.value;
        const orderbook = new PublicKey(metadata.staticMarketParams.marketAccount);
        const spline = PublicKey.findProgramAddressSync([Buffer.from("spline"), orderbook.toBuffer()], native)[0];
        const bookInfo = await connection.getAccountInfo(orderbook);
        expect(bookInfo?.owner.equals(native)).to.equal(true);
        const book = rise.decodeOrderbook(bookInfo!.data);
        const bids = book.bids.filter(b => b.order.numBaseLotsRemaining > 0n).sort((a, b) => a.orderId.priceInTicks > b.orderId.priceInTicks ? -1 : a.orderId.priceInTicks < b.orderId.priceInTicks ? 1 : 0);
        expect(bids.length).to.be.greaterThan(1);
        // Cross the first price level and part of the next. This deliberately
        // distinguishes per-maker updates from one aggregate taker update.
        const firstPrice = bids[0].orderId.priceInTicks;
        const firstLevel = bids.filter(b => b.orderId.priceInTicks === firstPrice).reduce((s, b) => s + b.order.numBaseLotsRemaining, 0n);
        const second = bids.find(b => b.orderId.priceInTicks < firstPrice)!;
        expect(second).to.not.equal(undefined);
        // Splines add executable liquidity not represented by visible FIFO
        // orders. A synthetic 2m-USDC sweep ensures the fixture exercises the
        // venue's combined matching path rather than assuming FIFO-only depth.
        const unitQuote = metadata.oraclePrice.markPrice.price.ticks * metadata.staticMarketParams.tickSize;
        const sweep = 2000000000000n / unitQuote;
        const quantity = firstLevel + 1n > sweep ? firstLevel + 1n : sweep;
        expect(quantity <= 0xffffffffn).to.equal(true);
        const localFunds = quantity * metadata.oraclePrice.markPrice.price.ticks * metadata.staticMarketParams.tickSize + 100000000n;
        expect(localFunds <= BigInt(Number.MAX_SAFE_INTEGER)).to.equal(true);
        await rpc("surfnet_setTokenAccount", [authority.toBase58(), usdc.toBase58(), { amount: Number(localFunds) }]);
        await send([await funding(Array(32).fill(44), Number(localFunds))]);
        const accounts = { phoenixProgramAddress: native.toBase58(), globalConfigurationAddress: global.toBase58(), traderAccount: trader.toBase58(), perpAssetMap: assetMap.toBase58(), globalTraderIndex: indexes, activeTraderBuffer: buffers };
        async function view(asset: boolean) {
            await refreshLocalMark(connection, rpc, native, assetMap, "SOL");
            const ix = web3((asset ? rise.buildHawkeyeViewMarginForAssetIx({ ...accounts, assetId: metadata.staticMarketParams.assetId } as never) : rise.buildHawkeyeViewMarginIx(accounts as never)) as KitIx);
            const latest = await connection.getLatestBlockhash();
            const tx = new Transaction({ feePayer: operator.publicKey, recentBlockhash: latest.blockhash }).add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }), ix);
            const result = await rpc("simulateTransaction", [tx.serialize({ requireAllSignatures: false, verifySignatures: false }).toString("base64"), { encoding: "base64", sigVerify: false }]) as {
                value: {
                    err: unknown;
                    returnData: {
                        programId: string;
                        data: [
                            string,
                            string
                        ];
                    };
                };
            };
            expect(result.value.err).to.equal(null);
            expect(result.value.returnData.programId).to.equal(String(rise.HAWKEYE_PROGRAM_ADDRESS));
            return rise.decodeHawkeyeReturnData(Buffer.from(result.value.returnData.data[0], "base64"));
        }
        // Synthetic private identity is used only for correlation, never put
        // into public instructions. The native packet carries its hash only.
        const user = Keypair.generate().publicKey;
        const clientOid = Buffer.alloc(16, 47);
        const nonce = Buffer.alloc(8);
        nonce.writeBigUInt64LE(1n);
        async function trade(side: rise.Side, lots: bigint, price: bigint, oid: bigint) {
            await refreshLocalMark(connection, rpc, native, assetMap, "SOL");
            const deadline = BigInt(await connection.getSlot()) + 5000n;
            const ix = web3(rise.buildPlaceMarketOrderDelegatedIx({ ...accounts, traderWallet: operator.publicKey.toBase58(), permissionAccount: operator.publicKey.toBase58(), orderbook: orderbook.toBase58(), splineCollection: spline.toBase58(), orderPacket: { side, priceInTicks: price, numBaseLots: lots, numQuoteLots: null, minBaseLotsToFill: 0n, minQuoteLotsToFill: 0n, selfTradeBehavior: rise.SelfTradeBehavior.Abort, matchLimit: null, clientOrderId: oid, lastValidSlot: deadline, orderFlags: rise.OrderFlags.None, cancelExisting: false } } as never) as KitIx);
            const signature = await send([ix]);
            return { signature, deadline };
        }
        const mark = metadata.oraclePrice.markPrice.price.ticks;
        await trade(rise.Side.Bid, 1n, mark * 102n / 100n, 901n);
        const preAsset = await view(true), preMargin = await view(false);
        const preFees = rise.decodeOrderbook((await connection.getAccountInfo(orderbook))!.data).header.totalTakerQuoteLotFees;
        const globalOid = createHash("sha256").update(Buffer.from("cinder:phoenix:v1")).update(user.toBuffer()).update(nonce).update(clientOid).digest().subarray(0, 16);
        const oid = globalOid.readBigUInt64LE(0) + (globalOid.readBigUInt64LE(8) << 64n);
        const reversal = await trade(rise.Side.Ask, quantity, second.orderId.priceInTicks, oid);
        const postAsset = await view(true), postMargin = await view(false);
        const postFees = rise.decodeOrderbook((await connection.getAccountInfo(orderbook))!.data).header.totalTakerQuoteLotFees;
        const receipt = await connection.getTransaction(reversal.signature, { commitment: "confirmed", maxSupportedTransactionVersion: 0 });
        expect(receipt?.meta?.err).to.equal(null);
        const directory = mkdtempSync(join(tmpdir(), "cinder-r5-native-"));
        try {
            const artifact = join(directory, "evidence.json");
            writeFileSync(artifact, JSON.stringify({ endpoint, operator: operator.publicKey.toBase58(), trader: trader.toBase58(), program: native.toBase58(), user: user.toBase58(), client_oid: Array.from(clientOid), quantity: quantity.toString(), bound: second.orderId.priceInTicks.toString(), deadline: reversal.deadline.toString(), asset_id: metadata.staticMarketParams.assetId, tick_size: metadata.staticMarketParams.tickSize.toString(), signature: reversal.signature, receipt, pre_asset: preAsset, pre_margin: preMargin, post_asset: postAsset, post_margin: postMargin, indexes, buffers, asset_map_account: { owner: mapInfo!.owner.toBase58(), executable: mapInfo!.executable, data: [mapInfo!.data.toString("base64"), "base64"] }, quote_budget: (unitQuote * 5n).toString(), budget_bid_price: (mark * 102n / 100n).toString(), budget_ask_price: (mark * 98n / 100n).toString() }, (_, v) => typeof v === "bigint" ? v.toString() : v));
            execFileSync("cargo", ["test", "--locked", "-p", "cinder-operator", "--lib", "native_multi_price_reversal_proof", "--", "--ignored", "--nocapture"], { env: { ...process.env, CARGO_INCREMENTAL: "0", CINDER_R5_NATIVE_EVIDENCE: artifact }, stdio: "pipe", timeout: 120000 });
            const proof = JSON.parse(readFileSync(artifact, "utf8"));
            // This is a native-code accounting proof, not a PER privacy proof:
            // synthetic private starting accounts live only on this fork.
            const ledgerIdl = JSON.parse(readFileSync("target/idl/cinder_ledger.json", "utf8"));
            await rpc("surfnet_writeProgram", [ledgerIdl.address, readFileSync("target/deploy/cinder_ledger.so").toString("hex"), 0]);
            const ledgerProgram = new anchor.Program<CinderLedger>(ledgerIdl, vault.provider);
            const [userLedger, bump] = PublicKey.findProgramAddressSync([Buffer.from("user"), user.toBuffer()], ledgerProgram.programId);
            const [privateBook, bookBump] = PublicKey.findProgramAddressSync([Buffer.from("book")], ledgerProgram.programId);
            const [fees, feeBump] = PublicKey.findProgramAddressSync([Buffer.from("fees")], ledgerProgram.programId);
            const bn = (value: string | number | bigint) => new anchor.BN(String(value));
            expect(proof.pre_margin.unsettledFundingQuoteLots).to.equal("0");
            expect(proof.post_margin.unsettledFundingQuoteLots).to.equal("0");
            const positions = Array.from({ length: 16 }, () => ({ assetId: 0, lots: bn(0), entryQuoteLots: bn(0), unsettledFunding: bn(0), reservedIm: bn(0) }));
            positions[0] = { assetId: 1, lots: bn(BigInt(proof.pre_asset.baseLots) - quantity), entryQuoteLots: bn(-BigInt(proof.pre_asset.virtualQuoteLots)), unsettledFunding: bn(0), reservedIm: bn(0) };
            const openOids = Array.from({ length: 8 }, () => ({ clientOid: Array(16).fill(0), assetId: 0, lotsDelta: bn(0), state: 0, limitPriceTicks: bn(0), lastValidSlot: bn(0) }));
            openOids[0] = { clientOid: Array.from(clientOid), assetId: 1, lotsDelta: bn(-quantity), state: 0, limitPriceTicks: bn(second.orderId.priceInTicks), lastValidSlot: bn(reversal.deadline) };
            const encodedLedger = await ledgerProgram.coder.accounts.encode("userLedger", { schemaVersion: 1, user, free: bn(proof.pre_margin.collateralQuoteLots), reserved: bn(0), withdrawable: bn(0), badDebtUsdc: bn(0), pendingOidCount: 1, nonce: bn(2), lastFundingEpoch: bn(0), positionsLen: 1, positions, openOids, bump });
            const residuals = Array.from({ length: 32 }, () => ({ assetId: 0, lots: bn(0) }));
            residuals[0] = { assetId: 1, lots: bn(proof.pre_asset.baseLots) };
            const encodedBook = await ledgerProgram.coder.accounts.encode("book", { schemaVersion: 1, residualLen: 1, residuals, phoenixCollateral: bn(proof.pre_margin.collateralQuoteLots), lastAckSlotEr: bn(0), invariantOk: 1, halt: 32, fundingEpoch: bn(0), lastScanMs: bn(0), bump: bookBump });
            const encodedFees = await ledgerProgram.coder.accounts.encode("feeAccrual", { phoenixFeesPaid: bn(0), cinderFeesAccrued: bn(0), bump: feeBump });
            for (const [address, data] of [[userLedger, encodedLedger], [privateBook, encodedBook], [fees, encodedFees]] as const)
                await rpc("surfnet_setAccount", [address.toBase58(), { owner: ledgerProgram.programId.toBase58(), lamports: 1000000000, data: data.toString("hex") }]);
            const guard = { clientOid: Array.from(clientOid), placementNonce: bn(1), observedLedgerNonce: bn(2), observedLedgerHash: Array.from(createHash("sha256").update(encodedLedger.subarray(8)).digest()), kind: 1, assetId: 1, requestedLots: bn(-quantity), limitPriceTicks: bn(second.orderId.priceInTicks), lastValidSlot: bn(reversal.deadline) };
            const facts = proof.native_facts;
            expect(postFees - preFees, "native taker counter must equal the actual decoded IOC fee").to.equal(BigInt(facts.fee));
            const margin = (BigInt(proof.post_asset.initialMarginQuoteLots) * 12500n + 9999n) / 10000n;
            const ack = await ledgerProgram.methods.ackPhoenixFillGuarded(guard, bn(facts.lots), bn(facts.fee), bn(facts.quote), bn(facts.fill_price), bn(margin)).accountsPartial({ adapter: operator.publicKey, config, userLedger, book: privateBook, feeAccrual: fees }).instruction();
            await send([ack]);
            const after = await ledgerProgram.account.userLedger.fetch(userLedger);
            const afterBook = await ledgerProgram.account.book.fetch(privateBook);
            const afterFees = await ledgerProgram.account.feeAccrual.fetch(fees);
            expect(after.pendingOidCount).to.equal(0);
            expect(after.positions[0].lots.toString()).to.equal(proof.post_asset.baseLots);
            expect(after.positions[0].entryQuoteLots.toString()).to.equal(facts.basis);
            expect(afterBook.residuals[0].lots.toString()).to.equal(proof.post_asset.baseLots);
            expect(afterFees.phoenixFeesPaid.toString()).to.equal(facts.fee);
            expect(afterBook.halt & 32).to.equal(32);
            expect(after.free.add(after.reserved).sub(after.badDebtUsdc).toString()).to.equal(proof.post_margin.collateralQuoteLots);
            const sourceCash = (await getAccount(connection, source)).amount;
            expect(sourceCash).to.equal(0n);
            // I1 and basis-aware I2 hold after the real native fill is acked.
            expect(BigInt(after.free.add(after.reserved).sub(after.badDebtUsdc).toString())).to.equal(sourceCash + BigInt(proof.post_margin.collateralQuoteLots) + BigInt(after.positions[0].entryQuoteLots.toString()) + BigInt(proof.post_asset.virtualQuoteLots));
            let replayRejected = false;
            try {
                await send([ack]);
            }
            catch {
                replayRejected = true;
            }
            expect(replayRejected).to.equal(true);
            const replayState = await ledgerProgram.account.userLedger.fetch(userLedger);
            expect(replayState.free.toString()).to.equal(after.free.toString());
            expect(replayState.reserved.toString()).to.equal(after.reserved.toString());
            expect((await ledgerProgram.account.feeAccrual.fetch(fees)).phoenixFeesPaid.toString()).to.equal(facts.fee);
            // Simulate exact production-builder instructions from immutable
            // journal budgets, proving the native cap in BOTH IOC directions.
            for (const side of [rise.Side.Bid, rise.Side.Ask]) {
                const price = side === rise.Side.Bid ? mark * 102n / 100n : mark * 98n / 100n;
                const quoteBudget = unitQuote * 5n;
                const bounded = web3(proof.budget_instructions[side === rise.Side.Bid ? 0 : 1] as KitIx);
                const assetView = web3(rise.buildHawkeyeViewMarginForAssetIx({ ...accounts, assetId: metadata.staticMarketParams.assetId } as never) as KitIx);
                const latest = await connection.getLatestBlockhash();
                const simulation = new Transaction({ feePayer: operator.publicKey, recentBlockhash: latest.blockhash }).add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }), bounded, assetView);
                simulation.sign(operator);
                const result = await rpc("simulateTransaction", [simulation.serialize().toString("base64"), { encoding: "base64", sigVerify: true }]) as {
                    value: {
                        err: unknown;
                        returnData: {
                            programId: string;
                            data: [
                                string,
                                string
                            ];
                        };
                    };
                };
                expect(result.value.err).to.equal(null);
                expect(result.value.returnData.programId).to.equal(String(rise.HAWKEYE_PROGRAM_ADDRESS));
                const simulated = rise.decodeHawkeyeReturnData(Buffer.from(result.value.returnData.data[0], "base64"));
                if (simulated.kind !== "view_margin_for_asset")
                    throw new Error("unexpected quote-budget view");
                const delta = simulated.baseLots - BigInt(proof.post_asset.baseLots);
                expect(side === rise.Side.Bid ? delta > 0n : delta < 0n).to.equal(true);
                const absolute = delta < 0n ? -delta : delta;
                // For asks the bound is a minimum executable price. For bids
                // compare a generous 2x mark lower bound only in this fixture.
                const minimumPrice = side === rise.Side.Ask ? price : mark / 2n;
                expect(absolute <= quoteBudget / (minimumPrice * metadata.staticMarketParams.tickSize)).to.equal(true);
                expect(absolute < 100n).to.equal(true);

                // Execute the production fee/collateral fence, not just an
                // off-chain estimate of a public UI fee rate. Its trailing
                // failure must undo the native fill and fee atomically.
                const marketBook = new PublicKey(metadata.staticMarketParams.marketAccount);
                await refreshLocalMark(connection, rpc, native, assetMap, "SOL");
                const snapshotKeys = [config, global, assetMap, trader, source, usdc, quote, ...indexes.map(s => new PublicKey(s)), ...buffers.map(s => new PublicKey(s))];
                const snapshot = await connection.getMultipleAccountsInfoAndContext(snapshotKeys);
                let digest = createHash("sha256").update("cinder:phoenix-snapshot:v1").digest();
                snapshot.value.forEach((account, index) => {
                    if (!account || account.executable) throw new Error("invalid local snapshot");
                    digest = createHash("sha256").update(digest).update(snapshotKeys[index].toBuffer()).update(account.owner.toBuffer()).update(createHash("sha256").update(account.data).digest()).digest();
                });
                const header = rise.decodeOrderbook((await connection.getAccountInfo(marketBook))!.data).header;
                const clockTime = await connection.getBlockTime(snapshot.context.slot);
                if (clockTime === null) throw new Error("missing local block time");
                const guard = { snapshotHash: Array.from(digest), iocDataHash: Array.from(createHash("sha256").update(bounded.data).digest()), observedSlot: new anchor.BN(snapshot.context.slot), oldestObservedMs: new anchor.BN(clockTime * 1000), takerFeeCounter: new anchor.BN(header.totalTakerQuoteLotFees.toString()), maxFeeUsdc: new anchor.BN(1000000), gtiCount: indexes.length };
                const fence = (after: boolean, value = guard) => vault.methods.guardPhoenixExecution(value, after).accountsPartial({ adapter: operator.publicKey, config, phoenixProgram: native, phoenixGlobalConfig: global, phoenixAssetMap: assetMap, phoenixTrader: trader, orderbook: marketBook, hawkeyeProgram: new PublicKey(String(rise.HAWKEYE_PROGRAM_ADDRESS)), instructions: SYSVAR_INSTRUCTIONS_PUBKEY }).remainingAccounts(snapshotKeys.slice(4).map(pubkey => ({ pubkey, isSigner: false, isWritable: false }))).instruction();
                const fenced = new Transaction({ feePayer: operator.publicKey, recentBlockhash: latest.blockhash }).add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }), await fence(false), bounded, await fence(true));
                fenced.sign(operator);
                expect(fenced.serialize().length).to.be.at.most(1232);
                const checked = await rpc("simulateTransaction", [fenced.serialize().toString("base64"), { encoding: "base64", sigVerify: true }]) as { value: { err: unknown; logs: string[] } };
                expect(checked.value.err, checked.value.logs?.join("\n")).to.equal(null);
                for (const instructions of [
                    [await fence(false), bounded],
                    [await fence(false), bounded, await fence(true, { ...guard, maxFeeUsdc: new anchor.BN(999999) })],
                    [await fence(false, { ...guard, snapshotHash: Array(32).fill(0) }), bounded, await fence(true, { ...guard, snapshotHash: Array(32).fill(0) })],
                ]) {
                    const malformed = new Transaction({ feePayer: operator.publicKey, recentBlockhash: latest.blockhash }).add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }), ...instructions);
                    malformed.sign(operator);
                    const result = await rpc("simulateTransaction", [malformed.serialize().toString("base64"), { encoding: "base64", sigVerify: true }]) as {value:{err:unknown}};
                    expect(result.value.err, "missing/mismatched peer or snapshot must refuse execution").not.to.equal(null);
                }
                const forbidden = { ...guard, maxFeeUsdc: new anchor.BN(0) };
                const reject = new Transaction({ feePayer: operator.publicKey, recentBlockhash: latest.blockhash }).add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }), await fence(false, forbidden), bounded, await fence(true, forbidden));
                reject.sign(operator);
                const denied = await rpc("simulateTransaction", [reject.serialize().toString("base64"), { encoding: "base64", sigVerify: true }]) as { value: { err: unknown; logs: string[] } };
                expect(denied.value.err, "a zero fee ceiling must reject a fee-paying IOC").not.to.equal(null);
                expect(denied.value.logs.some(line => line.includes("ExecutionGuardFailed"))).to.equal(true);
                const beforeAccounts = await connection.getMultipleAccountsInfo([trader, marketBook, ...indexes.map(s => new PublicKey(s)), ...buffers.map(s => new PublicKey(s))]);
                const rejectedSig = await connection.sendRawTransaction(reject.serialize(), { skipPreflight: true, maxRetries: 0 });
                const rejectedResult = await confirmForkTransaction(connection, { ...latest, signature: rejectedSig });
                expect(rejectedResult.value.err).not.to.equal(null);
                const afterAccounts = await connection.getMultipleAccountsInfo([trader, marketBook, ...indexes.map(s => new PublicKey(s)), ...buffers.map(s => new PublicKey(s))]);
                expect(afterAccounts.map(a => a!.data.toString("hex"))).to.deep.equal(beforeAccounts.map(a => a!.data.toString("hex")));
            }
        }
        finally {
            rmSync(directory, { recursive: true, force: true });
        }
    });
});
