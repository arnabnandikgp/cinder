/** Local-fork oracle clock fixture; prices and native validity rules stay real. */
import * as rise from "@ellipsis-labs/rise";
import { Connection, PublicKey, SYSVAR_CLOCK_PUBKEY } from "@solana/web3.js";

/** Solana Clock is in seconds; Phoenix oracle update timestamps are in ms. */
export function localClockTimestampMs(clock: { owner: PublicKey; data: Buffer } | null): bigint {
    if (!clock || clock.data.length !== 40 || !clock.owner.equals(new PublicKey("Sysvar1111111111111111111111111111111111111"))) throw new Error("untrusted local clock sysvar");
    const timestamp = clock.data.readBigInt64LE(32) * 1000n;
    if (timestamp <= 0n) throw new Error("invalid local clock timestamp");
    return timestamp;
}

/** Let wall time catch up with Surfpool's actual clock before a new process. */
export async function settleLocalClock(connection: Connection, rpc: (method: string, params: unknown[]) => Promise<unknown>) {
    const endpoint = new URL(connection.rpcEndpoint);
    if (endpoint.protocol !== "http:" || !["127.0.0.1", "localhost"].includes(endpoint.hostname)) throw new Error("clock fixture refuses non-local RPC");
    const blockTime = async () => {
        const timestamp = await connection.getBlockTime(await connection.getSlot());
        if (timestamp === null || !Number.isSafeInteger(timestamp)) throw new Error("missing local block time");
        return timestamp * 1000;
    };
    const initialDelay = await blockTime() - Date.now();
    if (initialDelay > 5000) throw new Error("local fork clock is too far ahead");
    if (initialDelay <= 0) return;
    await rpc("surfnet_pauseClock", []);
    try {
        // Surfpool 1.5 can produce extra blocks for blockhash expiration in
        // clock mode. Pause production until its real reported block time is
        // no longer in the future; never rewrite timestamps or RPC data.
        const delay = await blockTime() - Date.now();
        if (delay > 5000) throw new Error("local fork clock is too far ahead");
        if (delay > 0) await new Promise(resolve => setTimeout(resolve, delay));
    } finally {
        await rpc("surfnet_resumeClock", []);
    }
}

export async function refreshLocalMark(connection: Connection, rpc: (method: string, params: unknown[]) => Promise<unknown>, program: PublicKey, assetMap: PublicKey, symbol: string) {
    const endpoint = new URL(connection.rpcEndpoint);
    if (endpoint.protocol !== "http:" || !["127.0.0.1", "localhost"].includes(endpoint.hostname)) throw new Error("oracle fixture refuses non-local RPC");
    const account = await connection.getAccountInfo(assetMap);
    if (!account?.owner.equals(program) || account.executable) throw new Error("untrusted native asset map");
    const decoded = rise.decodePerpAssetMap(account.data);
    const matches = decoded.metadata.entries.filter(e => e.key === symbol);
    if (matches.length !== 1) throw new Error("ambiguous native market");
    const metadata = matches[0].value;
    const data = Buffer.from(account.data);
    const clock = await connection.getAccountInfo(SYSVAR_CLOCK_PUBKEY);
    // RPC simulations use confirmed contexts. The Clock sysvar can already
    // describe the next, unconfirmed slot; do not publish that future slot.
    const slot = BigInt(await connection.getSlot());
    const unixTimestampMs = localClockTimestampMs(clock);
    // Cross-margin matching can value counterparties' other markets too.
    // Publish the whole venue's frozen price fixture in one atomic account
    // update; refreshing SOL alone leaves those risk inputs to expire.
    for (const entry of decoded.metadata.entries) {
        const market = entry.value;
        if (market.oraclePrice.markPrice.price.ticks === 0n) continue;
        const needle = Buffer.alloc(16);
        needle.writeBigUInt64LE(market.oraclePrice.markPrice.price.slot);
        needle.writeBigUInt64LE(market.oraclePrice.markPrice.price.ticks, 8);
        // Bind to the unique market account, not a slot/tick pair that two
        // markets could share. In the pinned layout it follows MarkPrice by
        // 888 bytes; entries start at byte 48 and occupy 1584 bytes each.
        const offset = account.data.indexOf(new PublicKey(market.staticMarketParams.marketAccount).toBuffer()) - 888;
        if (offset < 80 || (offset - 80) % 1584 !== 0 || offset + 872 > account.data.length || !account.data.subarray(offset, offset + 16).equals(needle) || market.oraclePrice.markPrice.oracleData.length !== 5) throw new Error("native mark layout not found");
        // Official getMarkPriceDecoder layout. Do not change validation caches.
        for (const relative of [0, 16, 32, 48, 64, 80, 152, 168, 184, 200, 216, 248, 288, 304, 320]) {
            if (data.readBigUInt64LE(offset + relative + 8) !== 0n) data.writeBigUInt64LE(slot, offset + relative);
        }
        data.writeBigUInt64LE(slot, offset + 112);
        for (let i = 0; i < 5; i++) {
            if (!data.subarray(offset + 592 + i * 40, offset + 624 + i * 40).equals(new PublicKey(market.oraclePrice.markPrice.oracleData[i].oraclePubkey).toBuffer())) throw new Error("native oracle layout mismatch");
            const timestamp = offset + 832 + i * 8;
            if (data.readBigUInt64LE(timestamp) !== 0n) data.writeBigUInt64LE(unixTimestampMs, timestamp);
        }
    }
    await rpc("surfnet_setAccount", [assetMap.toBase58(), { owner: program.toBase58(), data: data.toString("hex") }]);
    return metadata;
}
