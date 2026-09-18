/** Mirror mb-test-validator's installed public fixtures on a localhost fork. */
import { readFileSync, readdirSync } from "fs";
import { join } from "path";
import { PublicKey } from "@solana/web3.js";

async function main() {
    const endpoint = process.env.PROVIDER_ENDPOINT || "http://127.0.0.1:8989";
    const url = new URL(endpoint);
    if (url.protocol !== "http:" || !["localhost", "127.0.0.1"].includes(url.hostname)) throw new Error("refusing non-local fixture installation");
    const directory = process.env.CINDER_MB_TEST_DUMPS;
    if (!directory) throw new Error("CINDER_MB_TEST_DUMPS must identify installed local-test fixtures");
    const rpc = async (method: string, params: unknown[]) => {
        const response = await fetch(endpoint, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }), signal: AbortSignal.timeout(30000) });
        const body = await response.json() as { error?: unknown };
        if (!response.ok || body.error) throw new Error(`local fixture ${method} failed`);
    };
    for (const file of readdirSync(directory)) {
        if (file.endsWith(".so")) {
            const address = file.slice(0, -3); new PublicKey(address);
            await rpc("surfnet_writeProgram", [address, readFileSync(join(directory, file)).toString("hex"), 0]);
        } else if (file.endsWith(".json")) {
            const fixture = JSON.parse(readFileSync(join(directory, file), "utf8"));
            if (!fixture.pubkey || !fixture.account) throw new Error("invalid installed public account fixture");
            new PublicKey(fixture.pubkey); new PublicKey(fixture.account.owner);
            const a = fixture.account;
            if (a.data[1] !== "base64") throw new Error("unsupported installed fixture encoding");
            await rpc("surfnet_setAccount", [fixture.pubkey, { lamports: a.lamports, owner: a.owner, executable: a.executable, data: Buffer.from(a.data[0], "base64").toString("hex") }]);
        }
    }
}
main().catch(() => { console.error("Could not install the local MagicBlock test fixtures"); process.exitCode = 1; });
