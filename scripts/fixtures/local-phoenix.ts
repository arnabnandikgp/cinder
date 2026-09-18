/** Local-fork fixture only; never changes Phoenix's public-chain state. */
import {
  decodeGlobalConfiguration,
  PHOENIX_GLOBAL_CONFIGURATION_ADDRESS,
  PHOENIX_PROGRAM_ADDRESS,
} from "@ellipsis-labs/rise";
import { Connection, PublicKey } from "@solana/web3.js";

export async function activateLocalPhoenix(connection: Connection) {
  const url = new URL(connection.rpcEndpoint);
  if (
    url.protocol !== "http:" ||
    !["127.0.0.1", "localhost"].includes(url.hostname)
  ) {
    throw new Error("Phoenix fixture refuses non-local RPC");
  }
  if (
    (await connection.getGenesisHash()) !==
    "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d"
  ) {
    throw new Error("Phoenix fixture requires a mainnet fork");
  }
  const global = new PublicKey(PHOENIX_GLOBAL_CONFIGURATION_ADDRESS);
  const info = await connection.getAccountInfo(global);
  if (!info || !info.owner.equals(new PublicKey(PHOENIX_PROGRAM_ADDRESS))) {
    throw new Error("Phoenix global configuration missing or incorrectly owned");
  }
  const decoded = decodeGlobalConfiguration(info.data);
  // Official native GlobalConfig prefix: status at 504, restart ACK at 1096.
  // A fork has its own restart lifecycle and no venue admin crank. Activate
  // only this local fixture, preserving all economic parameters and bindings.
  if (info.data.length < 1104 || info.data[504] !== decoded.exchangeStatus) {
    throw new Error("Unexpected Phoenix global configuration layout");
  }
  const data = Buffer.from(info.data);
  data[504] = (data[504] | 0x81) & ~0x04;
  data.writeBigUInt64LE(0n, 1096);
  const response = await fetch(connection.rpcEndpoint, {
    method: "POST",
    redirect: "error",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "surfnet_setAccount",
      params: [global.toBase58(), { data: data.toString("hex") }],
    }),
    signal: AbortSignal.timeout(30_000),
  });
  const body = (await response.json()) as { error?: unknown };
  if (!response.ok || body.error) {
    throw new Error("Failed to activate local Phoenix fixture");
  }
  return decoded;
}
