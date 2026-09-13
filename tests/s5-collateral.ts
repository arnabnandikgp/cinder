import * as anchor from "@anchor-lang/core";
import { BN, Program } from "@anchor-lang/core";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccount,
  createMint,
  getAccount,
  getAssociatedTokenAddressSync,
  mintTo,
} from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { expect } from "chai";
import { CinderVault } from "../target/types/cinder_vault";
import { CinderLedger } from "../target/types/cinder_ledger";

const ER_VALIDATOR = new PublicKey(
  "mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev"
);
const AMOUNT = 25_000_000; // 25 USDC

function pda(programId: PublicKey, seeds: (Buffer | Uint8Array)[]): PublicKey {
  return PublicKey.findProgramAddressSync(seeds, programId)[0];
}

describe("S5 vault post/pull (stand-in, no Phoenix CPI)", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const vault = (anchor.workspace as any).cinderVault as Program<CinderVault>;
  const ledger = (anchor.workspace as any).cinderLedger as Program<CinderLedger>;
  const payer = (provider.wallet as anchor.Wallet).payer;
  const connection = provider.connection;

  const adapter = Keypair.generate();
  const standIn = Keypair.generate();
  const phoenixTrader = Keypair.generate().publicKey;

  let usdcMint: PublicKey;
  let configPda: PublicKey;
  let vaultAuthPda: PublicKey;
  let reservePda: PublicKey;
  let vaultAta: PublicKey;
  let standInAta: PublicKey;
  let bookPda: PublicKey;
  let feesPda: PublicKey;

  before(async () => {
    for (const k of [adapter, standIn]) {
      const sig = await connection.requestAirdrop(
        k.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL
      );
      await connection.confirmTransaction(sig, "confirmed");
    }
    usdcMint = await createMint(connection, payer, payer.publicKey, null, 6);
    configPda = pda(vault.programId, [Buffer.from("config")]);
    vaultAuthPda = pda(vault.programId, [Buffer.from("vault-authority")]);
    reservePda = pda(vault.programId, [Buffer.from("reserve")]);
    bookPda = pda(ledger.programId, [Buffer.from("book")]);
    feesPda = pda(ledger.programId, [Buffer.from("fees")]);
    vaultAta = getAssociatedTokenAddressSync(usdcMint, vaultAuthPda, true);
    standInAta = getAssociatedTokenAddressSync(usdcMint, standIn.publicKey);

    await vault.methods
      .initialize(
        adapter.publicKey,
        standIn.publicKey,
        phoenixTrader,
        ER_VALIDATOR
      )
      .accountsPartial({
        admin: payer.publicKey,
        config: configPda,
        vaultAuthority: vaultAuthPda,
        reserveRoot: reservePda,
        usdcMint,
        vaultUsdcAta: vaultAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    await ledger.methods
      .initialize()
      .accountsPartial({
        adapter: adapter.publicKey,
        config: configPda,
        book: bookPda,
        feeAccrual: feesPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([adapter])
      .rpc();

    await createAssociatedTokenAccount(
      connection,
      payer,
      usdcMint,
      standIn.publicKey
    );
    await mintTo(connection, payer, usdcMint, vaultAta, payer, AMOUNT * 2);
  });

  it("post_collateral moves USDC from vault ATA to stand-in ATA", async () => {
    await vault.methods
      .postCollateral(new BN(AMOUNT))
      .accountsPartial({
        adapter: adapter.publicKey,
        config: configPda,
        vaultAuthority: vaultAuthPda,
        vaultUsdcAta: vaultAta,
        destUsdcAta: standInAta,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([adapter])
      .rpc();

    const vaultAfter = await getAccount(connection, vaultAta);
    const destAfter = await getAccount(connection, standInAta);
    expect(Number(vaultAfter.amount)).to.equal(AMOUNT);
    expect(Number(destAfter.amount)).to.equal(AMOUNT);
  });

  it("pull_collateral returns USDC to vault ATA", async () => {
    await vault.methods
      .pullCollateral(new BN(AMOUNT))
      .accountsPartial({
        adapter: adapter.publicKey,
        standIn: standIn.publicKey,
        config: configPda,
        vaultUsdcAta: vaultAta,
        sourceUsdcAta: standInAta,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([adapter, standIn])
      .rpc();

    const vaultAfter = await getAccount(connection, vaultAta);
    const srcAfter = await getAccount(connection, standInAta);
    expect(Number(vaultAfter.amount)).to.equal(AMOUNT * 2);
    expect(Number(srcAfter.amount)).to.equal(0);
  });

  it("user cannot post_collateral", async () => {
    try {
      await vault.methods
        .postCollateral(new BN(1))
        .accountsPartial({
          adapter: standIn.publicKey,
          config: configPda,
          vaultAuthority: vaultAuthPda,
          vaultUsdcAta: vaultAta,
          destUsdcAta: standInAta,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([standIn])
        .rpc();
      expect.fail("non-adapter must not post");
    } catch (e: any) {
      const code = e.error?.errorCode?.code ?? e.toString();
      expect(code).to.match(/Unauthorized/);
    }
  });
});
