/** Local-fork oracle clock fixture; prices and native validity rules stay real. */
import * as rise from "@ellipsis-labs/rise";
import { Connection, PublicKey } from "@solana/web3.js";

export async function refreshLocalMark(connection: Connection, rpc: (method: string, params: unknown[]) => Promise<unknown>, program: PublicKey, assetMap: PublicKey, symbol: string) {
    const endpoint = new URL(connection.rpcEndpoint);
    if (endpoint.protocol !== "http:" || !["127.0.0.1", "localhost"].includes(endpoint.hostname)) throw new Error("oracle fixture refuses non-local RPC");
    const account = await connection.getAccountInfo(assetMap);
    if (!account?.owner.equals(program) || account.executable) throw new Error("untrusted native asset map");
    const decoded = rise.decodePerpAssetMap(account.data);
    const matches = decoded.metadata.entries.filter(e => e.key === symbol);
    if (matches.length !== 1) throw new Error("ambiguous native market");
    const metadata = matches[0].value;
    const needle = Buffer.alloc(16);
    needle.writeBigUInt64LE(metadata.oraclePrice.markPrice.price.slot);
    needle.writeBigUInt64LE(metadata.oraclePrice.markPrice.price.ticks, 8);
    const offset = account.data.indexOf(needle);
    if (offset < 0 || offset + 872 > account.data.length || metadata.oraclePrice.markPrice.oracleData.length !== 5) throw new Error("native mark layout not found");
    const data = Buffer.from(account.data);
    const slot = BigInt(await connection.getSlot());
    // Official getMarkPriceDecoder layout. Do not change validation caches.
    for (const relative of [0, 16, 32, 48, 64, 80, 152, 168, 184, 200, 216, 248, 288, 304, 320]) {
        if (data.readBigUInt64LE(offset + relative + 8) !== 0n) data.writeBigUInt64LE(slot, offset + relative);
    }
    data.writeBigUInt64LE(slot, offset + 112);
    for (let i = 0; i < 5; i++) {
        if (!data.subarray(offset + 592 + i * 40, offset + 624 + i * 40).equals(new PublicKey(metadata.oraclePrice.markPrice.oracleData[i].oraclePubkey).toBuffer())) throw new Error("native oracle layout mismatch");
        const timestamp = offset + 832 + i * 8;
        if (data.readBigUInt64LE(timestamp) !== 0n) data.writeBigUInt64LE(BigInt(Math.floor(Date.now() / 1000)), timestamp);
    }
    await rpc("surfnet_setAccount", [assetMap.toBase58(), { owner: program.toBase58(), data: data.toString("hex") }]);
    return metadata;
}
