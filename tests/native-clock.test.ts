import { expect } from "chai";
import { Connection, PublicKey } from "@solana/web3.js";
import { localClockTimestampMs, settleLocalClock } from "./support/native-oracle";

describe("local fork clock fixture", () => {
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
});
