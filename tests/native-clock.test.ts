import { expect } from "chai";
import { Connection, PublicKey } from "@solana/web3.js";
import * as rise from "@ellipsis-labs/rise";
import { localClockTimestampMs, settleLocalClock, refreshLocalMark, setLocalFundingGeneration, waitForLocalBlockTime } from "./support/native-oracle";

describe("local fork clock fixture", function () {
    // Real SDK decoding of the 1024-entry native layout can exceed Mocha's
    // default 2-second budget on Linux CI; this is not an RPC freshness limit.
    this.timeout(15000);
    it("waits for a block produced after the pre-read clock check", async () => {
        let wall = 10769;
        const waits: number[] = [];
        await waitForLocalBlockTime(11, () => wall, async delay => { waits.push(delay); wall += delay; });
        expect(waits).to.deep.equal([232]);
        expect(wall).to.be.greaterThan(11000);
    });
    it("does not delay current or past block timestamps", async () => {
        for (const timestamp of [9, 10]) await waitForLocalBlockTime(timestamp, () => 10000, async () => { throw new Error("unexpected wait"); });
    });
    it("rechecks the wall clock when a timer wakes one millisecond early", async () => {
        let wall = 10769;
        const waits: number[] = [];
        await waitForLocalBlockTime(11, () => wall, async delay => {
            waits.push(delay);
            wall += waits.length === 1 ? delay - 2 : delay;
        });
        expect(waits).to.deep.equal([232, 2]);
        expect(wall).to.be.at.least(11000);
    });
    it("rejects invalid and excessively future block timestamps before waiting", async () => {
        for (const timestamp of [NaN, 0, -1, 10.5, Number.MAX_SAFE_INTEGER, 16]) {
            let error: unknown;
            try { await waitForLocalBlockTime(timestamp, () => 10000, async () => { throw new Error("unexpected wait"); }); } catch (e) { error = e; }
            expect((error as Error)?.message).to.equal(timestamp === 16 ? "local fork clock is too far ahead" : "invalid local block time");
        }
    });
    it("converts Clock seconds to Phoenix oracle milliseconds", () => {
        const data = Buffer.alloc(40);
        data.writeBigInt64LE(1789681805n, 32);
        expect(localClockTimestampMs({ data, owner: new PublicKey("Sysvar1111111111111111111111111111111111111") })).to.equal(1789681805000n);
    });

    it("refuses untrusted or invalid Clock sysvars", () => {
        const owner = new PublicKey("Sysvar1111111111111111111111111111111111111");
        expect(() => localClockTimestampMs(null)).to.throw("untrusted local clock sysvar");
        expect(() => localClockTimestampMs({ owner, data: Buffer.alloc(39) })).to.throw("untrusted local clock sysvar");
        expect(() => localClockTimestampMs({ owner: PublicKey.default, data: Buffer.alloc(40) })).to.throw("untrusted local clock sysvar");
        expect(() => localClockTimestampMs({ owner, data: Buffer.alloc(40) })).to.throw("invalid local clock timestamp");
    });
    function fixture(timestamp: number | null, rpcEndpoint = "http://127.0.0.1:8989") {
        const calls: string[] = [];
        const connection = {
            rpcEndpoint,
            getSlot: async () => { calls.push("getSlot"); return 42; },
            getBlockTime: async (slot: number) => {
                expect(slot).to.equal(42);
                calls.push("getBlockTime");
                return timestamp;
            },
        } as Connection;
        const rpc = async (method: string, params: unknown[]) => {
            expect(params).to.deep.equal([]);
            calls.push(method);
        };
        return { connection, rpc, calls };
    }

    async function rejects(action: Promise<unknown>, message: string) {
        let failure: unknown;
        try { await action; } catch (error) { failure = error; }
        expect(failure).to.be.instanceOf(Error);
        expect((failure as Error).message).to.equal(message);
    }

    it("leaves an already non-future clock running without account mutations", async () => {
        const f = fixture(Math.floor(Date.now() / 1000) - 10);
        await settleLocalClock(f.connection, f.rpc);
        expect(f.calls).to.deep.equal(["getSlot", "getBlockTime"]);
    });

    it("refuses public endpoints before making any RPC calls", async () => {
        const f = fixture(0, "https://api.mainnet-beta.solana.com");
        await rejects(settleLocalClock(f.connection, f.rpc), "clock fixture refuses non-local RPC");
        expect(f.calls).to.deep.equal([]);
    });

    it("waits for the reported block time before resuming", async () => {
        const timestamp = Math.floor(Date.now() / 1000) + 1;
        const f = fixture(timestamp);
        await settleLocalClock(f.connection, async (method, params) => {
            if (method === "surfnet_resumeClock") expect(Date.now()).to.be.at.least(timestamp * 1000);
            await f.rpc(method, params);
        });
        expect(f.calls.at(-1)).to.equal("surfnet_resumeClock");
    });

    it("resumes the clock when reading block time fails after pausing", async () => {
        const f = fixture(Math.floor(Date.now() / 1000) + 1);
        let reads = 0;
        f.connection.getBlockTime = async () => ++reads === 1 ? Math.floor(Date.now() / 1000) + 1 : null;
        await rejects(settleLocalClock(f.connection, f.rpc), "missing local block time");
        expect(f.calls.at(-1)).to.equal("surfnet_resumeClock");
    });

    it("bounds clock catch-up instead of waiting on arbitrary future timestamps", async () => {
        const f = fixture(Math.floor(Date.now() / 1000) + 60);
        await rejects(settleLocalClock(f.connection, f.rpc), "local fork clock is too far ahead");
        expect(f.calls).to.deep.equal(["getSlot", "getBlockTime"]);
    });

    function assetMapFixture() {
        const program = new PublicKey(Buffer.alloc(32, 1));
        const assetMap = new PublicKey(Buffer.alloc(32, 2));
        const market = new PublicKey(Buffer.alloc(32, 7));
        const markOffset = 80, fundingOffset = markOffset + 1264;
        const updated = BigInt(Math.floor(Date.now() / 1000) - 3600);
        // Pinned Rise layout, decoded by the real SDK. This in-memory account
        // is only a unit fixture; the integration test still reads actual RPCs.
        let data = Buffer.alloc(48 + 1024 * 1584);
        Buffer.from(rise.ACCOUNT_DISCRIMINANTS.PERP_ASSET_MAP).copy(data);
        data.writeUInt16LE(1, 24);
        data.writeUInt32LE(1, 32);
        data.writeBigUInt64LE(1024n, 40);
        data.write("SOL", 48);
        data.writeBigUInt64LE(42n, markOffset);
        data.writeBigUInt64LE(100n, markOffset + 8);
        market.toBuffer().copy(data, markOffset + 888);
        data.writeBigUInt64LE(1n, markOffset + 832);
        data.writeBigInt64LE(10n, fundingOffset + 32);
        data.writeBigUInt64LE(updated - updated % 3600n, fundingOffset + 40);
        data.writeBigUInt64LE(updated, fundingOffset + 48);
        data.writeBigUInt64LE(3600n, fundingOffset + 56);
        const clock = Buffer.alloc(40);
        clock.writeBigInt64LE(BigInt(Math.floor(Date.now() / 1000)), 32);
        const connection = {
            rpcEndpoint: "http://127.0.0.1:8989",
            getSlot: async () => 84,
            getAccountInfo: async (key: PublicKey) => key.equals(assetMap)
                ? { owner: program, executable: false, data: Buffer.from(data) }
                : { owner: new PublicKey("Sysvar1111111111111111111111111111111111111"), data: clock },
        } as Connection;
        const rpc = async (method: string, params: unknown[]) => {
            expect(method).to.equal("surfnet_setAccount");
            expect(params[0]).to.equal(assetMap.toBase58());
            data = Buffer.from((params[1] as { data: string }).data, "hex");
        };
        const metadata = () => rise.decodePerpAssetMap(data).metadata.entries[0].value;
        return { connection, rpc, program, assetMap, updated, metadata };
    }

    for (const first of ["oracle", "funding"]) {
        it(`serializes ${first}-first account updates without losing funding or oracle clocks`, async () => {
            const f = assetMapFixture();
            let release!: () => void, observed!: () => void;
            const blocked = new Promise<void>(resolve => release = resolve);
            const firstRead = new Promise<void>(resolve => observed = resolve);
            let reads = 0;
            const read = f.connection.getAccountInfo.bind(f.connection);
            f.connection.getAccountInfo = async key => {
                const snapshot = await read(key);
                if (key.equals(f.assetMap) && ++reads === 1) {
                    observed();
                    await blocked;
                }
                return snapshot;
            };
            // Separate Connection instances must share the same account lock.
            const secondConnection = { ...f.connection } as Connection;
            const oracle = () => refreshLocalMark(f.connection, f.rpc, f.program, f.assetMap, "SOL");
            const funding = () => setLocalFundingGeneration(secondConnection, f.rpc, f.program, f.assetMap, "SOL", 20n, f.updated + 60n);
            const leading = first === "oracle" ? oracle() : funding();
            await firstRead;
            const trailing = first === "oracle" ? funding() : oracle();
            try {
                await new Promise<void>(resolve => setImmediate(resolve));
                expect(reads, "second read must wait for the entire first read/modify/write").to.equal(1);
            } finally {
                release();
                await Promise.all([leading, trailing]);
            }
            const state = f.metadata();
            expect(state.fundingAccumulator.cumulativeFundingRate).to.equal(20n);
            expect(state.fundingAccumulator.lastFundingUpdateTimestamp).to.equal(f.updated + 60n);
            expect(state.oraclePrice.markPrice.price.slot).to.equal(84n);
            expect(state.oraclePrice.markPrice.price.ticks).to.equal(100n);
        });
    }

    it("releases the account queue after a rejected fixture update", async () => {
        const f = assetMapFixture();
        const read = f.connection.getAccountInfo.bind(f.connection);
        f.connection.getAccountInfo = async key => ({ ...(await read(key))!, owner: PublicKey.default });
        await rejects(refreshLocalMark(f.connection, f.rpc, f.program, f.assetMap, "SOL"), "untrusted native asset map");
        f.connection.getAccountInfo = read;
        await setLocalFundingGeneration(f.connection, f.rpc, f.program, f.assetMap, "SOL", 20n, f.updated + 60n);
        expect(f.metadata().fundingAccumulator.cumulativeFundingRate).to.equal(20n);
    });
});
