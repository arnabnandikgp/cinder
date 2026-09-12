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

type RegisterIx = {
  programId: string;
  keys: { pubkey: string; isSigner: boolean; isWritable: boolean }[];
  data: number[];
};

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

/** Phoenix program on mainnet; present on a copy-on-read fork after first touch. */
const PHOENIX_HINT = process.env.PHOENIX_PROGRAM_ID || "";

export async function buildRegisterIxs(args: {
  traderAuthority: string;
  txFeePayer: string;
}): Promise<{ instructions: RegisterIx[]; traderPda?: string }> {
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
  return res.json();
}

export async function sendToFork(
  connection: Connection,
  payer: Keypair,
  extraSigners: Keypair[],
  ixs: RegisterIx[]
): Promise<string> {
  const tx = new Transaction();
  for (const ix of ixs) tx.add(toWeb3Ix(ix));
  tx.feePayer = payer.publicKey;
  tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
  tx.sign(payer, ...extraSigners);
  const sig = await connection.sendRawTransaction(tx.serialize(), {
    skipPreflight: true,
  });
  await connection.confirmTransaction(sig, "confirmed");
  return sig;
}

async function main() {
  if (process.env.CINDER_S5 !== "1") {
    console.log("venue-boot: set CINDER_S5=1 to run against a Surfpool fork");
    return;
  }
  if (!(await forkUp(FORK))) {
    throw new Error(`fork RPC not up: ${FORK}`);
  }
  const connection = new Connection(FORK, "confirmed");
  const wallet = anchor.Wallet.local();
  const standIn = wallet.payer;
  const feePayer = standIn;
  console.log("fee payer", feePayer.publicKey.toBase58());
  console.log("trader authority (stand-in)", standIn.publicKey.toBase58());

  const built = await buildRegisterIxs({
    traderAuthority: standIn.publicKey.toBase58(),
    txFeePayer: feePayer.publicKey.toBase58(),
  });
  console.log(`register ixs: ${built.instructions?.length ?? 0}`);
  if (built.traderPda) console.log("trader PDA (api)", built.traderPda);

  const sig = await sendToFork(
    connection,
    feePayer,
    [],
    built.instructions ?? []
  );
  console.log("register tx", sig);
}

if (require.main === module) {
  main().catch((err) => {
    console.error(err);
    process.exit(1);
  });
}
