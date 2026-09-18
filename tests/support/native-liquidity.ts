/** Real, non-expiring maker orders on a disposable localhost Phoenix fork. */
import * as rise from "@ellipsis-labs/rise";
import { Connection, Keypair, PublicKey, Transaction, TransactionInstruction, ComputeBudgetProgram } from "@solana/web3.js";
import { createAssociatedTokenAccountInstruction, getAssociatedTokenAddressSync, getAccount } from "@solana/spl-token";
import permission from "../../scripts/fixtures/phoenix-referral-activation-permission.json";
import { confirmForkTransaction } from "./native-confirmation";
import { refreshLocalMark } from "./native-oracle";

type KitIx = { programAddress: string; data: ArrayLike<number>; accounts: readonly { address: string; role: number }[] };
const web3 = (ix: KitIx) => new TransactionInstruction({ programId: new PublicKey(ix.programAddress), data: Buffer.from(ix.data), keys: ix.accounts.map(a => ({ pubkey: new PublicKey(a.address), isSigner: a.role >= 2, isWritable: (a.role & 1) !== 0 })) });

export async function seedLocalLiquidity(ctx: {
    connection: Connection; operator: Keypair; native: PublicKey; quote: PublicKey;
    assetMap: PublicKey; indexes: string[]; buffers: string[];
    rpc: (method: string, params: unknown[]) => Promise<unknown>;
    send: (ixs: TransactionInstruction[], pad?: boolean) => Promise<string>;
}) {
    const { connection, operator, native, quote, assetMap, indexes, buffers } = ctx;
    const endpoint = new URL(connection.rpcEndpoint);
    if (endpoint.protocol !== "http:" || !["127.0.0.1", "localhost"].includes(endpoint.hostname)) throw new Error("liquidity fixture refuses non-local RPC");
    if (await connection.getGenesisHash() !== "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d") throw new Error("liquidity fixture requires a mainnet fork");
    // This is an independent venue counterparty, not another Cinder trader.
    // Genesis USDC is the only synthetic cash: Ember and Phoenix perform the
    // real conversion/deposit, and maker orders execute real native code.
    const maker = Keypair.generate();
    const trader = await rise.getPhoenixTraderSubaccountAddress({ authority: maker.publicKey.toBase58() as never, traderPdaIndex: 0, subaccountIndex: 0, phoenixProgramAddress: native.toBase58() as never });
    await ctx.rpc("surfnet_setAccount", [maker.publicKey.toBase58(), { lamports: 1000000000 }]);
    await ctx.send([web3(rise.buildRegisterTraderIx({ payer: operator.publicKey.toBase58(), trader: maker.publicKey.toBase58(), traderAccount: trader, maxPositions: 128n, traderPdaIndex: 0, traderSubaccountIndex: 0 } as never) as KitIx)]);
    await ctx.send([web3(rise.buildOnboardTraderDelegatedIx({ authority: permission.trader_onboarder, permissionAccount: permission.permission_account, traderAccount: trader, activeTraderBuffer: buffers, globalTraderIndex: indexes } as never) as KitIx)], true);
    const global = rise.decodeGlobalConfiguration((await connection.getAccountInfo(new PublicKey(String(rise.PHOENIX_GLOBAL_CONFIGURATION_ADDRESS))))!.data);
    const usdc = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
    const source = getAssociatedTokenAddressSync(usdc, maker.publicKey);
    const transit = getAssociatedTokenAddressSync(quote, maker.publicKey);
    const amount = 1000000000;
    await ctx.rpc("surfnet_setTokenAccount", [maker.publicKey.toBase58(), usdc.toBase58(), { amount }]);
    await ctx.send([createAssociatedTokenAccountInstruction(operator.publicKey, transit, maker.publicKey, quote)]);
    async function sendMaker(ixs: TransactionInstruction[]) {
        const latest = await connection.getLatestBlockhash();
        const tx = new Transaction({ feePayer: operator.publicKey, recentBlockhash: latest.blockhash }).add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }), ...ixs);
        tx.sign(operator, maker);
        const simulated = await connection.simulateTransaction(tx);
        if (simulated.value.err) throw new Error(`native maker simulation failed: ${JSON.stringify(simulated.value.err)}\n${simulated.value.logs?.join("\n")}`);
        const signature = await connection.sendRawTransaction(tx.serialize(), { skipPreflight: true, maxRetries: 0 });
        const confirmed = await confirmForkTransaction(connection, { ...latest, signature });
        if (confirmed.value.err) throw new Error(`native maker transaction failed: ${JSON.stringify(confirmed.value.err)}`);
    }
    const ember = new PublicKey(String(rise.EMBER_PROGRAM_ADDRESS));
    await sendMaker([
        web3(rise.buildEmberDepositIx({ owner: maker.publicKey.toBase58(), inputMint: usdc.toBase58(), outputMint: quote.toBase58(), inputTokenAccount: source.toBase58(), outputTokenAccount: transit.toBase58(), emberState: PublicKey.findProgramAddressSync([native.toBuffer(), Buffer.from("state")], ember)[0].toBase58(), emberVault: PublicKey.findProgramAddressSync([native.toBuffer(), Buffer.from("vault")], ember)[0].toBase58(), amount: BigInt(amount) } as never) as KitIx),
        web3(rise.buildDepositFundsIx({ trader: maker.publicKey.toBase58(), traderAccount: trader, mint: quote.toBase58(), traderTokenAccount: transit.toBase58(), globalVault: global.globalVaultKey, globalTraderIndex: indexes, activeTraderBuffer: buffers, amount: BigInt(amount) } as never) as KitIx),
    ]);
    if ((await getAccount(connection, source)).amount !== 0n || (await getAccount(connection, transit)).amount !== 0n) throw new Error("maker collateral was not fully converted and deposited");
    const makerAccount = await connection.getAccountInfo(new PublicKey(trader));
    if (!makerAccount?.owner.equals(native)) throw new Error("untrusted maker trader account");
    // Cold Trader.state alone is not authoritative for an onboarded hot trader.
    const latest = await connection.getLatestBlockhash();
    const view = new Transaction({ feePayer: operator.publicKey, recentBlockhash: latest.blockhash }).add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }), web3(rise.buildHawkeyeViewMarginIx({ phoenixProgramAddress: native.toBase58(), globalConfigurationAddress: String(rise.PHOENIX_GLOBAL_CONFIGURATION_ADDRESS), traderAccount: trader, perpAssetMap: assetMap.toBase58(), globalTraderIndex: indexes, activeTraderBuffer: buffers } as never) as KitIx));
    view.sign(operator);
    const result = await connection.simulateTransaction(view);
    if (result.value.err || result.value.returnData?.programId !== String(rise.HAWKEYE_PROGRAM_ADDRESS)) throw new Error("maker collateral view failed");
    const margin = rise.decodeHawkeyeReturnData(Buffer.from(result.value.returnData.data[0], "base64"));
    if (margin.kind !== "view_margin" || margin.collateralQuoteLots !== BigInt(amount) || margin.positionCount !== 0) throw new Error("maker native collateral not backed by actual deposit");
    const metadata = await refreshLocalMark(connection, ctx.rpc, native, assetMap, "SOL");
    const orderbook = new PublicKey(metadata.staticMarketParams.marketAccount);
    const spline = PublicKey.findProgramAddressSync([Buffer.from("spline"), orderbook.toBuffer()], native)[0];
    const mark = metadata.oraclePrice.markPrice.price.ticks;
    const prices: bigint[] = [];
    const key = (e: rise.OrderbookEntry) => `${e.orderId.priceInTicks}:${e.orderId.orderSequenceNumber}`;
    for (const side of [rise.Side.Bid, rise.Side.Ask]) {
        await refreshLocalMark(connection, ctx.rpc, native, assetMap, "SOL");
        const before = rise.decodeOrderbook((await connection.getAccountInfo(orderbook))!.data);
        const previous = new Set((side === rise.Side.Bid ? before.bids : before.asks).map(key));
        await sendMaker([web3(rise.buildPlacePostOnlyOrderIx({ trader: maker.publicKey.toBase58(), traderAccount: trader, perpAssetMap: assetMap.toBase58(), globalTraderIndex: indexes, activeTraderBuffer: buffers, orderbook: orderbook.toBase58(), splineCollection: spline.toBase58(), orderPacket: { side, priceInTicks: mark * (side === rise.Side.Bid ? 99n : 101n) / 100n, numBaseLots: 256n, clientOrderId: side === rise.Side.Bid ? 123n : 124n, slide: true, lastValidSlot: null, orderFlags: rise.OrderFlags.None, cancelExisting: false } } as never) as KitIx)]);
        const after = rise.decodeOrderbook((await connection.getAccountInfo(orderbook))!.data);
        const added = (side === rise.Side.Bid ? after.bids : after.asks).filter(e => !previous.has(key(e)));
        if (added.length !== 1 || added[0].order.numBaseLotsRemaining !== 256n || added[0].order.lastValidSlot !== null || added[0].order.reduceOnly) throw new Error("maker fixture did not create one full non-expiring native order");
        prices.push(added[0].orderId.priceInTicks);
    }
    return { bid: prices[0], ask: prices[1] };
}
