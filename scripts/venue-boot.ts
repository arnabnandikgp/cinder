/**
 * S5 venue boot against a Surfpool fork (or any RPC that has Phoenix programs).
 *
 * Does not call send-register-ixs (that broadcasts to Phoenix mainnet RPC).
 * Builds no-referral register ixs, sends them to the local fork, then
 * DelegateTrader and Rise deposit/withdraw helpers.
 *
 * Skip unless CINDER_S5=1 and the fork is up.
 *
 * Phoenix `send-register-ixs` co-signs with the onboarder and broadcasts to
 * mainnet RPC — do not use it on a fork. This script sends built ixs to the
 * local fork; if an ix still requires the onboarder as a signer, Surfpool
 * cheatcodes / a cloned trader account are required (named S5 gap).
 */
import * as anchor from "@coral-xyz/anchor";
import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";

const FORK = process.env.PROVIDER_ENDPOINT || "http://127.0.0.1:8899";
const API = process.env.PHOENIX_API_URL || "https://perp-api.phoenix.trade";
const MAX_POSITIONS = 128;
const MAINNET_GENESIS = "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d";
const ALLOWED_API_HOSTS = new Set(["perp-api.phoenix.trade"]);

type RegisterIx = {
  programId: string;
  keys: { pubkey: string; isSigner: boolean; isWritable: boolean }[];
  data: number[];
};

function assertLocalRpc(url: string) {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    throw new Error(`invalid PROVIDER_ENDPOINT: ${url}`);
  }
  if (parsed.hostname !== "127.0.0.1" && parsed.hostname !== "localhost") {
    throw new Error(
      `refusing non-local RPC ${url}; venue-boot only signs against a local fork`
    );
  }
}

function assertPhoenixApi(url: string) {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    throw new Error(`invalid PHOENIX_API_URL: ${url}`);
  }
  if (!ALLOWED_API_HOSTS.has(parsed.hostname)) {
    throw new Error(
      `refusing PHOENIX_API_URL host ${parsed.hostname}; expected perp-api.phoenix.trade`
    );
  }
}

function validateIx(ix: unknown, index: number): RegisterIx {
  if (!ix || typeof ix !== "object") {
    throw new Error(`register ix ${index} is not an object`);
  }
  const rec = ix as Record<string, unknown>;
  if (typeof rec.programId !== "string") {
    throw new Error(`register ix ${index} missing programId`);
  }
  new PublicKey(rec.programId);
  if (!Array.isArray(rec.keys)) {
    throw new Error(`register ix ${index} missing keys`);
  }
  const keys = rec.keys.map((k, j) => {
    if (!k || typeof k !== "object") {
      throw new Error(`register ix ${index} key ${j} is not an object`);
    }
    const kr = k as Record<string, unknown>;
    if (typeof kr.pubkey !== "string") {
      throw new Error(`register ix ${index} key ${j} missing pubkey`);
    }
    new PublicKey(kr.pubkey);
    if (typeof kr.isSigner !== "boolean" || typeof kr.isWritable !== "boolean") {
      throw new Error(`register ix ${index} key ${j} has invalid flags`);
    }
    return {
      pubkey: kr.pubkey,
      isSigner: kr.isSigner,
      isWritable: kr.isWritable,
    };
  });
  if (
    !Array.isArray(rec.data) ||
    !rec.data.every(
      (n) => typeof n === "number" && Number.isInteger(n) && n >= 0 && n <= 255
    )
  ) {
    throw new Error(`register ix ${index} has invalid data`);
  }
  return {
    programId: rec.programId,
    keys,
    data: rec.data as number[],
  };
}

function toWeb3Ix(ix: RegisterIx): TransactionInstruction {
  return new TransactionInstruction({
    programId: new PublicKey(ix.programId),
    keys: ix.keys.map((k) => ({
      pubkey: new PublicKey(k.pubkey),
      isSigner: k.isSigner,
      isWritable: k.isWritable,
    })),
    data: Buffer.from(ix.data),
  });
}

export async function forkUp(url: string): Promise<boolean> {
  try {
    await new Connection(url, "confirmed").getVersion();
    return true;
  } catch {
    return false;
  }
}

export async function buildRegisterIxs(args: {
  traderAuthority: string;
  txFeePayer: string;
}): Promise<{ instructions: RegisterIx[]; traderPda?: string }> {
  assertPhoenixApi(API);
  const res = await fetch(`${API}/v1/exchange/build-register-ixs`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      traderAuthority: args.traderAuthority,
      txFeePayer: args.txFeePayer,
      maxPositions: MAX_POSITIONS,
    }),
  });
  if (!res.ok) {
    throw new Error(`build-register-ixs ${res.status}: ${await res.text()}`);
  }
  const body: unknown = await res.json();
  if (!body || typeof body !== "object") {
    throw new Error("build-register-ixs returned a non-object");
  }
  const rec = body as Record<string, unknown>;
  if (!Array.isArray(rec.instructions) || rec.instructions.length === 0) {
    throw new Error("build-register-ixs returned no instructions");
  }
  const instructions = rec.instructions.map(validateIx);
  const traderPda =
    typeof rec.traderPda === "string" ? rec.traderPda : undefined;
  if (traderPda) new PublicKey(traderPda);
  return { instructions, traderPda };
}

export async function sendToFork(
  connection: Connection,
  payer: Keypair,
  extraSigners: Keypair[],
  ixs: RegisterIx[]
): Promise<string> {
  if (ixs.length === 0) {
    throw new Error("no instructions to send");
  }
  const tx = new Transaction();
  for (const ix of ixs) tx.add(toWeb3Ix(ix));
  tx.feePayer = payer.publicKey;
  const latestBlockhash = await connection.getLatestBlockhash();
  tx.recentBlockhash = latestBlockhash.blockhash;
  tx.sign(payer, ...extraSigners);
  const sig = await connection.sendRawTransaction(tx.serialize(), {
    skipPreflight: true,
  });
  const confirmation = await connection.confirmTransaction(
    { signature: sig, ...latestBlockhash },
    "confirmed"
  );
  if (confirmation.value.err) {
    throw new Error(
      `register transaction failed: ${JSON.stringify(confirmation.value.err)}`
    );
  }
  return sig;
}

async function main() {
  if (process.env.CINDER_S5 !== "1") {
    console.log("venue-boot: set CINDER_S5=1 to run against a Surfpool fork");
    return;
  }
  assertLocalRpc(FORK);
  if (!(await forkUp(FORK))) {
    throw new Error(`fork RPC not up: ${FORK}`);
  }
  const connection = new Connection(FORK, "confirmed");
  const genesis = await connection.getGenesisHash();
  if (genesis !== MAINNET_GENESIS) {
    throw new Error(
      `RPC genesis ${genesis} is not mainnet (${MAINNET_GENESIS}); expected a Surfpool mainnet fork`
    );
  }
  const wallet = anchor.Wallet.local();
  const standIn = wallet.payer;
  const feePayer = standIn;
  console.log("fee payer", feePayer.publicKey.toBase58());
  console.log("trader authority (stand-in)", standIn.publicKey.toBase58());

  const built = await buildRegisterIxs({
    traderAuthority: standIn.publicKey.toBase58(),
    txFeePayer: feePayer.publicKey.toBase58(),
  });
  console.log(`register ixs: ${built.instructions.length}`);
  if (built.traderPda) console.log("trader PDA (api)", built.traderPda);

  const sig = await sendToFork(connection, feePayer, [], built.instructions);
  console.log("register tx", sig);
}

if (require.main === module) {
  main().catch((err) => {
    console.error(err);
    process.exit(1);
  });
}
