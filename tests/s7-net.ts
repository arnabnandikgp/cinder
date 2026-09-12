import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  createMint,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import { expect } from "chai";
import nacl from "tweetnacl";
import {
  EPHEMERAL_VAULT_ID,
  getAuthToken,
  MAGIC_PROGRAM_ID,
  PERMISSION_PROGRAM_ID,
  permissionPdaFromAccount,
} from "@magicblock-labs/ephemeral-rollups-sdk";
import { CinderVault } from "../target/types/cinder_vault";
import { CinderLedger } from "../target/types/cinder_ledger";

const ER_VALIDATOR = new PublicKey(
  "mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev"
);
const BASE = process.env.PROVIDER_ENDPOINT || "http://127.0.0.1:8899";
const ER = process.env.EPHEMERAL_PROVIDER_ENDPOINT || "http://127.0.0.1:7799";
const QFS = process.env.TEE_PROVIDER_ENDPOINT || "http://127.0.0.1:6699";
const QFS_WS = process.env.TEE_WS_ENDPOINT || "ws://127.0.0.1:6700";
const ASSET_SOL = 1;
const CREDIT = 100_000_000;
const LOTS = 10;

function pda(programId: PublicKey, seeds: (Buffer | Uint8Array)[]): PublicKey {
  return PublicKey.findProgramAddressSync(seeds, programId)[0];
}

function oid(tag: number): number[] {
  return Array.from({ length: 16 }, (_, i) => (i + tag) % 256);
}

function sleep(ms: number) {
  return new Promise((r) => setTimeout(r, ms));
}

async function rpcUp(url: string): Promise<boolean> {
  try {
    await new Connection(url, "confirmed").getVersion();
    return true;
  } catch {
    return false;
  }
}

async function airdrop(connection: Connection, pk: PublicKey) {
  const sig = await connection.requestAirdrop(
    pk,
    2 * anchor.web3.LAMPORTS_PER_SOL
  );
  await connection.confirmTransaction(sig, "confirmed");
}

async function authToken(payer: Keypair) {
  return getAuthToken(QFS, payer.publicKey, (message: Uint8Array) =>
    Promise.resolve(nacl.sign.detached(message, payer.secretKey))
  );
}

function qfsConnection(token: string) {
  return new Connection(`${QFS}?token=${token}`, {
    wsEndpoint: `${QFS_WS}?token=${token}`,
    commitment: "confirmed",
  });
}

async function waitErOwner(
  er: Connection,
  address: PublicKey,
  owner: PublicKey,
  label: string
) {
  for (let i = 0; i < 40; i++) {
    const info = await er.getAccountInfo(address);
    if (info && info.owner.equals(owner)) return info;
    await sleep(500);
  }
  throw new Error(`${label} did not appear on ER owned by ${owner.toBase58()}`);
}

describe("S7 two-user net + QFS isolation after trades", function () {
  this.timeout(180_000);

  let skip = false;
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const vault = (anchor.workspace as any).cinderVault as Program<CinderVault>;
  const ledger = (anchor.workspace as any).cinderLedger as Program<CinderLedger>;
  const payer = (provider.wallet as anchor.Wallet).payer;
  const base = provider.connection;
  const erConn = new Connection(ER, "confirmed");

  const adapter = Keypair.generate();
  const userA = Keypair.generate();
  const userB = Keypair.generate();
  const phoenixTrader = Keypair.generate().publicKey;

  let configPda: PublicKey;
  let bookPda: PublicKey;
  let feesPda: PublicKey;
  let ledgerA: PublicKey;
  let ledgerB: PublicKey;
  let erProgram: Program<CinderLedger>;

  before(async function () {
    if (!(await rpcUp(QFS)) || !(await rpcUp(ER)) || !(await rpcUp(BASE))) {
      skip = true;
      this.skip();
    }
    await airdrop(base, adapter.publicKey);
    await airdrop(base, userA.publicKey);
    await airdrop(base, userB.publicKey);

    const usdcMint = await createMint(base, payer, payer.publicKey, null, 6);
    configPda = pda(vault.programId, [Buffer.from("config")]);
    const vaultAuth = pda(vault.programId, [Buffer.from("vault-authority")]);
    const reservePda = pda(vault.programId, [Buffer.from("reserve")]);
    bookPda = pda(ledger.programId, [Buffer.from("book")]);
    feesPda = pda(ledger.programId, [Buffer.from("fees")]);
    ledgerA = pda(ledger.programId, [
      Buffer.from("user"),
      userA.publicKey.toBuffer(),
    ]);
    ledgerB = pda(ledger.programId, [
      Buffer.from("user"),
      userB.publicKey.toBuffer(),
    ]);
    const vaultAta = getAssociatedTokenAddressSync(usdcMint, vaultAuth, true);

    await vault.methods
      .initialize(adapter.publicKey, vaultAuth, phoenixTrader, ER_VALIDATOR)
      .accounts({
        admin: payer.publicKey,
        config: configPda,
        vaultAuthority: vaultAuth,
        reserveRoot: reservePda,
        usdcMint,
        vaultUsdcAta: vaultAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    await vault.methods
      .setAllowlist([ASSET_SOL])
      .accounts({ admin: payer.publicKey, config: configPda })
      .rpc();
    await ledger.methods
      .initialize()
      .accounts({
        adapter: adapter.publicKey,
        config: configPda,
        book: bookPda,
        feeAccrual: feesPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([adapter])
      .rpc();

    for (const [user, userLedger] of [
      [userA, ledgerA],
      [userB, ledgerB],
    ] as const) {
      await ledger.methods
        .initUser()
        .accounts({
          adapter: adapter.publicKey,
          user: user.publicKey,
          config: configPda,
          userLedger,
          systemProgram: SystemProgram.programId,
        })
        .signers([adapter, user])
        .rpc();
    }

    const sendBase = async (builder: any, signers: Keypair[]) => {
      const tx = await builder.transaction();
      return provider.sendAndConfirm(tx, signers, {
        skipPreflight: true,
        commitment: "confirmed",
      });
    };
    await sendBase(
      ledger.methods.delegateBook().accountsPartial({
        adapter: adapter.publicKey,
        config: configPda,
        book: bookPda,
        validator: ER_VALIDATOR,
      }),
      [adapter]
    );
    await sendBase(
      ledger.methods.delegateFees().accountsPartial({
        adapter: adapter.publicKey,
        config: configPda,
        feeAccrual: feesPda,
        validator: ER_VALIDATOR,
      }),
      [adapter]
    );
    await sendBase(
      ledger.methods.delegateUser().accountsPartial({
        adapter: adapter.publicKey,
        user: userA.publicKey,
        config: configPda,
        userLedger: ledgerA,
        validator: ER_VALIDATOR,
      }),
      [adapter, userA]
    );
    await sendBase(
      ledger.methods.delegateUser().accountsPartial({
        adapter: adapter.publicKey,
        user: userB.publicKey,
        config: configPda,
        userLedger: ledgerB,
        validator: ER_VALIDATOR,
      }),
      [adapter, userB]
    );

    await waitErOwner(erConn, bookPda, ledger.programId, "book");
    await waitErOwner(erConn, ledgerA, ledger.programId, "user A");
    await waitErOwner(erConn, ledgerB, ledger.programId, "user B");

    const adapterWallet = new anchor.Wallet(adapter);
    const erProvider = new anchor.AnchorProvider(erConn, adapterWallet, {
      commitment: "confirmed",
    });
    erProgram = new Program(
      ledger.idl as CinderLedger,
      erProvider
    ) as Program<CinderLedger>;
    const sendEr = async (builder: any, extra?: Keypair[]) => {
      let tx = await builder.transaction();
      tx.feePayer = adapter.publicKey;
      tx.recentBlockhash = (await erConn.getLatestBlockhash()).blockhash;
      tx = await adapterWallet.signTransaction(tx);
      if (extra) for (const s of extra) tx.partialSign(s);
      return erProvider.sendAndConfirm(tx, extra ?? [], { skipPreflight: true });
    };
    const perm = (account: PublicKey) => ({
      adapter: adapter.publicKey,
      config: configPda,
      permission: permissionPdaFromAccount(account),
      magicProgram: MAGIC_PROGRAM_ID,
      permissionProgram: PERMISSION_PROGRAM_ID,
      ephemeralVault: EPHEMERAL_VAULT_ID,
    });
    await sendEr(
      erProgram.methods.initBookPermission().accountsPartial({
        ...perm(bookPda),
        book: bookPda,
      })
    );
    await sendEr(
      erProgram.methods.initFeesPermission().accountsPartial({
        ...perm(feesPda),
        feeAccrual: feesPda,
      })
    );
    await sendEr(
      erProgram.methods.initUserPermission().accountsPartial({
        ...perm(ledgerA),
        userLedger: ledgerA,
      })
    );
    await sendEr(
      erProgram.methods.initUserPermission().accountsPartial({
        ...perm(ledgerB),
        userLedger: ledgerB,
      })
    );

    await sendEr(
      erProgram.methods.creditDeposit(new BN(CREDIT)).accounts({
        adapter: adapter.publicKey,
        config: configPda,
        book: bookPda,
        userLedger: ledgerA,
      })
    );
    await sendEr(
      erProgram.methods.creditDeposit(new BN(CREDIT)).accounts({
        adapter: adapter.publicKey,
        config: configPda,
        book: bookPda,
        userLedger: ledgerB,
      })
    );
  });

  it("offsetting fills net Book to 0; QFS isolation still holds", async function () {
    if (skip) this.skip();

    const place = async (user: Keypair, userLedger: PublicKey, lots: number, tag: number) => {
      const adapterWallet = new anchor.Wallet(user);
      const erUser = new Program(
        ledger.idl as CinderLedger,
        new anchor.AnchorProvider(erConn, adapterWallet, { commitment: "confirmed" })
      ) as Program<CinderLedger>;
      let tx = await erUser.methods
        .placeOrder(ASSET_SOL, new BN(lots), oid(tag), 50, false, new BN(0))
        .accounts({
          user: user.publicKey,
          config: configPda,
          book: bookPda,
          userLedger,
        })
        .transaction();
      tx.feePayer = user.publicKey;
      tx.recentBlockhash = (await erConn.getLatestBlockhash()).blockhash;
      tx = await adapterWallet.signTransaction(tx);
      await erConn.sendRawTransaction(tx.serialize(), { skipPreflight: true });
    };

    await place(userA, ledgerA, LOTS, 1);
    await place(userB, ledgerB, -LOTS, 2);

    const adapterWallet = new anchor.Wallet(adapter);
    const sendAck = async (userLedger: PublicKey, tag: number, lots: number) => {
      let tx = await erProgram.methods
        .ackPhoenixFill(oid(tag), new BN(lots), new BN(0), new BN(0))
        .accounts({
          adapter: adapter.publicKey,
          config: configPda,
          userLedger,
          book: bookPda,
          feeAccrual: feesPda,
        })
        .transaction();
      tx.feePayer = adapter.publicKey;
      tx.recentBlockhash = (await erConn.getLatestBlockhash()).blockhash;
      tx = await adapterWallet.signTransaction(tx);
      const sig = await erConn.sendRawTransaction(tx.serialize(), {
        skipPreflight: true,
      });
      await erConn.confirmTransaction(sig, "confirmed");
    };
    await sendAck(ledgerA, 1, LOTS);
    await sendAck(ledgerB, 2, -LOTS);

    const book = await erProgram.account.book.fetch(bookPda);
    expect(book.residualLen).to.equal(0);

    const ulA = await erProgram.account.userLedger.fetch(ledgerA);
    const ulB = await erProgram.account.userLedger.fetch(ledgerB);
    const lotsA = ulA.positions[0].lots.toNumber();
    const lotsB = ulB.positions[0].lots.toNumber();
    expect(lotsA + lotsB).to.equal(0);

    const tokA = await authToken(userA);
    const tokB = await authToken(userB);
    const connA = qfsConnection(tokA.token);
    const connB = qfsConnection(tokB.token);
    expect(await connA.getAccountInfo(ledgerA)).to.not.equal(null);
    expect(await connB.getAccountInfo(ledgerB)).to.not.equal(null);
    let bDenied = false;
    try {
      bDenied = (await connB.getAccountInfo(ledgerA)) === null;
    } catch {
      bDenied = true;
    }
    let aDenied = false;
    try {
      aDenied = (await connA.getAccountInfo(ledgerB)) === null;
    } catch {
      aDenied = true;
    }
    expect(bDenied, "B must not read A after trades").to.equal(true);
    expect(aDenied, "A must not read B after trades").to.equal(true);
  });
});
