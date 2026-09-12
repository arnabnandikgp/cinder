/**
 * S5 venue boot against a Surfpool mainnet fork.
 *
 * Uses Rise on-chain builders (RegisterTrader, DelegateTrader, deposit/withdraw
 * flows) and sends them to localhost. Does not call send-register-ixs.
 */
import * as anchor from "@coral-xyz/anchor";
import * as fs from "fs";
import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";

const FORK = process.env.PROVIDER_ENDPOINT || "http://127.0.0.1:8899";
const API = process.env.PHOENIX_API_URL || "https://perp-api.phoenix.trade";
const MAINNET_GENESIS = "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d";
const WALLET_USDC = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const DEFAULT_POST_USDC = BigInt("25000000");

type KitIx = {
  programAddress: string;
  accounts: readonly { address: string; role: number }[];
  data: ArrayLike<number>;
};

function assertLocalRpc(url: string) {
  const parsed = new URL(url);
  if (parsed.hostname !== "127.0.0.1" && parsed.hostname !== "localhost") {
    throw new Error(
      `refusing non-local RPC ${url}; venue-boot only signs against a local fork`
    );
  }
}

function kitToWeb3(ix: KitIx): TransactionInstruction {
  return new TransactionInstruction({
    programId: new PublicKey(ix.programAddress),
    keys: ix.accounts.map((a) => ({
      pubkey: new PublicKey(a.address),
      isSigner: a.role === 2 || a.role === 3,
      isWritable: a.role === 1 || a.role === 3,
    })),
    data: Buffer.from(ix.data),
  });
}

export async function sendToFork(
  connection: Connection,
  payer: Keypair,
  extraSigners: Keypair[],
  ixs: KitIx[]
): Promise<string> {
  if (ixs.length === 0) throw new Error("no instructions to send");
  const tx = new Transaction();
  for (const ix of ixs) tx.add(kitToWeb3(ix));
  tx.feePayer = payer.publicKey;
  const latestBlockhash = await connection.getLatestBlockhash();
  tx.recentBlockhash = latestBlockhash.blockhash;
  // Local fork only: pad missing required signatures (Phoenix onboarder).
  // Requires surfpool --skip-signature-verification.
  tx.partialSign(payer, ...extraSigners);
  const msg = tx.compileMessage();
  const known = new Set(
    [payer, ...extraSigners].map((k) => k.publicKey.toBase58())
  );
  for (let i = 0; i < msg.header.numRequiredSignatures; i++) {
    const pk = msg.accountKeys[i];
    if (!known.has(pk.toBase58()) && !tx.signatures[i].signature) {
      tx.addSignature(pk, Buffer.alloc(64));
    }
  }
  const sig = await connection.sendRawTransaction(
    tx.serialize({ requireAllSignatures: true, verifySignatures: false }),
    { skipPreflight: true }
  );
  const confirmation = await connection.confirmTransaction(
    { signature: sig, ...latestBlockhash },
    "confirmed"
  );
  if (confirmation.value.err) {
    throw new Error(
      `transaction failed: ${JSON.stringify(confirmation.value.err)}`
    );
  }
  return sig;
}

async function surfnetSetTokenAccount(
  rpcUrl: string,
  owner: PublicKey,
  mint: PublicKey,
  amount: bigint
) {
  const res = await fetch(rpcUrl, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "surfnet_setTokenAccount",
      params: [owner.toBase58(), mint.toBase58(), { amount: Number(amount) }],
    }),
  });
  const body = (await res.json()) as { error?: { message: string } };
  if (body.error) {
    throw new Error(`surfnet_setTokenAccount: ${body.error.message}`);
  }
}

function loadOrCreateAdapter(): Keypair {
  const path = process.env.CINDER_ADAPTER_KEYPAIR || ".cinder-adapter.json";
  if (fs.existsSync(path)) {
    const raw = JSON.parse(fs.readFileSync(path, "utf8")) as number[];
    return Keypair.fromSecretKey(Uint8Array.from(raw));
  }
  const kp = Keypair.generate();
  fs.writeFileSync(path, JSON.stringify(Array.from(kp.secretKey)));
  return kp;
}

function positionAuthorityOf(trader: {
  state?: { positionAuthority?: unknown };
  positionAuthority?: unknown;
}): string | null {
  const v = trader.state?.positionAuthority ?? trader.positionAuthority;
  return v == null ? null : String(v);
}

export async function bootVenue(opts: {
  adapter: Keypair;
  connection?: Connection;
  authority?: Keypair;
  postAmount?: bigint;
  skipWithdraw?: boolean;
}): Promise<{
  traderPda: PublicKey;
  quoteLotCollateralBefore: bigint;
  quoteLotCollateralAfterPost: bigint;
  quoteLotCollateralAfterPull: bigint | null;
  withdrawQueued: boolean;
  registerSig: string;
  delegateSig: string;
  depositSig: string;
  withdrawSig: string | null;
}> {
  const connection = opts.connection ?? new Connection(FORK, "confirmed");
  const rpcUrl = connection.rpcEndpoint;
  assertLocalRpc(rpcUrl);
  const genesis = await connection.getGenesisHash();
  if (genesis !== MAINNET_GENESIS) {
    throw new Error(
      `RPC genesis ${genesis} is not mainnet; expected a Surfpool mainnet fork`
    );
  }

  const rise = await import("@ellipsis-labs/rise");
  const client = rise.createPhoenixClient({
    apiUrl: API,
    rpcUrl,
    ws: false,
    exchangeMetadata: { stream: false },
  });
  await client.exchange.ready();

  const wallet = opts.authority ?? anchor.Wallet.local().payer;
  const adapter = opts.adapter;
  const authority = wallet.publicKey.toBase58();
  const postAmount = opts.postAmount ?? DEFAULT_POST_USDC;

  const traderPdaStr = await client.pda.getTraderAddress({
    authority: authority as never,
    traderPdaIndex: 0,
    subaccountIndex: 0,
  });
  const traderPda = new PublicKey(traderPdaStr);

  let registerSig = "already-registered";
  const existing = await connection.getAccountInfo(traderPda);
  if (!existing) {
    const registerIx = await client.ixs.buildRegisterTrader({
      authority: authority as never,
      marginType: rise.MarginType.Cross,
      traderPdaIndex: 0,
      traderSubaccountIndex: 0,
    });
    console.log("sending RegisterTrader");
    registerSig = await sendToFork(connection, wallet, [], [
      registerIx as unknown as KitIx,
    ]);
  }

  let delegateSig = "skipped";
  try {
    const delegateIx = await client.ixs.buildDelegateTrader({
      traderWallet: authority as never,
      traderPdaIndex: 0,
      traderSubaccountIndex: 0,
      newPositionAuthority: adapter.publicKey.toBase58() as never,
    });
    delegateSig = await sendToFork(connection, wallet, [], [
      delegateIx as unknown as KitIx,
    ]);
  } catch (err) {
    const trader = await rise.fetchTrader({
      client: client.rpc.accounts,
      address: traderPdaStr,
      skipCache: true,
    });
    const stored = positionAuthorityOf(trader);
    if (stored !== adapter.publicKey.toBase58()) {
      throw err instanceof Error ? err : new Error(String(err));
    }
    delegateSig = "already-delegated";
  }

  try {
    const perm = await client.api.invite().getReferralActivationPermission();
    const onboardIx = await client.ixs.buildOnboardTraderDelegated({
      authority: perm.trader_onboarder as never,
      traderAuthority: authority as never,
      permissionAccount: perm.permission_account as never,
      traderPdaIndex: 0,
      traderSubaccountIndex: 0,
    });
    console.log("sending OnboardTraderDelegated");
    await sendToFork(connection, wallet, [], [onboardIx as unknown as KitIx]);
  } catch (err) {
    console.warn(
      "OnboardTraderDelegated failed (continuing):",
      err instanceof Error ? err.message : err
    );
  }

  await surfnetSetTokenAccount(
    rpcUrl,
    wallet.publicKey,
    new PublicKey(WALLET_USDC),
    postAmount
  );

  const traderBefore = await rise.fetchTrader({
    client: client.rpc.accounts,
    address: traderPdaStr,
    skipCache: true,
  });

  const deposit = await client.ixs.buildDepositIxs({
    authority: authority as never,
    amount: postAmount,
    traderPdaIndex: 0,
    traderSubaccountIndex: 0,
  });
  console.log("sending deposit flow");
  const depositSig = await sendToFork(
    connection,
    wallet,
    [],
    deposit.instructions as unknown as KitIx[]
  );

  const traderAfterPost = await rise.fetchTrader({
    client: client.rpc.accounts,
    address: traderPdaStr,
    skipCache: true,
  });

  let withdrawSig: string | null = null;
  let withdrawQueued = false;
  let quoteLotCollateralAfterPull: bigint | null = null;
  if (!opts.skipWithdraw) {
    const withdraw = await client.ixs.buildWithdrawIxs({
      authority: authority as never,
      amount: postAmount,
      traderPdaIndex: 0,
      traderSubaccountIndex: 0,
    });
    try {
      withdrawSig = await sendToFork(
        connection,
        wallet,
        [],
        withdraw.instructions as unknown as KitIx[]
      );
      const traderAfterPull = await rise.fetchTrader({
        client: client.rpc.accounts,
        address: traderPdaStr,
        skipCache: true,
      });
      quoteLotCollateralAfterPull = BigInt(
        traderAfterPull.state.quoteLotCollateral.toString()
      );
      withdrawQueued = traderAfterPull.withdrawQueueNode !== null;
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      if (!/queue/i.test(msg)) {
        throw err;
      }
      const queuedTrader = await rise.fetchTrader({
        client: client.rpc.accounts,
        address: traderPdaStr,
        skipCache: true,
      });
      if (queuedTrader.withdrawQueueNode == null) {
        throw err instanceof Error ? err : new Error(msg);
      }
      withdrawQueued = true;
      quoteLotCollateralAfterPull = BigInt(
        queuedTrader.state.quoteLotCollateral.toString()
      );
    }
  }

  client.dispose();

  return {
    traderPda,
    quoteLotCollateralBefore: BigInt(
      traderBefore.state.quoteLotCollateral.toString()
    ),
    quoteLotCollateralAfterPost: BigInt(
      traderAfterPost.state.quoteLotCollateral.toString()
    ),
    quoteLotCollateralAfterPull,
    withdrawQueued,
    registerSig,
    delegateSig,
    depositSig,
    withdrawSig,
  };
}

async function main() {
  if (process.env.CINDER_S5 !== "1") {
    console.log("venue-boot: set CINDER_S5=1 to run against a Surfpool fork");
    return;
  }
  const out = await bootVenue({ adapter: loadOrCreateAdapter() });
  console.log("trader PDA", out.traderPda.toBase58());
  console.log("register", out.registerSig);
  console.log("delegate", out.delegateSig);
  console.log("deposit", out.depositSig);
  console.log(
    "collateral before/after post",
    out.quoteLotCollateralBefore.toString(),
    out.quoteLotCollateralAfterPost.toString()
  );
  if (out.withdrawQueued) {
    console.log("withdraw queued (no silent debit)");
  } else {
    console.log("withdraw", out.withdrawSig);
    console.log(
      "collateral after pull",
      out.quoteLotCollateralAfterPull?.toString()
    );
  }
}

if (require.main === module) {
  main().catch((err) => {
    console.error(err);
    process.exit(1);
  });
}
