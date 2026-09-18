/** Explicit SOL fee provisioning, separate from Phoenix trades and user USDC.
 * Instruction layouts match magicblock-delegation-program-api =3.1.0.
 * Validator-owned vault setup requires the validator's actual signature.
 */
import { Connection, Keypair, PublicKey, SystemProgram, Transaction, TransactionInstruction } from "@solana/web3.js";
import { DELEGATION_PROGRAM_ID as DLP, createDelegateInstruction, createTopUpEscrowInstruction, delegationRecordPdaFromDelegatedAccount as record, delegationMetadataPdaFromDelegatedAccount as metadata, delegateBufferPdaFromDelegatedAccountAndOwnerProgram as buffer, magicFeeVaultPdaFromValidator } from "@magicblock-labs/ephemeral-rollups-sdk";
import { readFileSync, lstatSync } from "fs";
import * as anchor from "@anchor-lang/core";

export const feeBalance = (payer:PublicKey,index:number) => {
    if (!Number.isInteger(index) || index<0 || index>=255) throw new Error("fee index must be 0..254 (255 is Magic Actions escrow)");
    return PublicKey.findProgramAddressSync([Buffer.from("balance"),payer.toBuffer(),Buffer.from([index])],DLP)[0];
};
const key=(pubkey:PublicKey,isSigner=false,isWritable=false)=>({pubkey,isSigner,isWritable});
const instruction=(kind:number,keys:ReturnType<typeof key>[],args=Buffer.alloc(0))=>{
    const tag=Buffer.alloc(8);tag.writeBigUInt64LE(BigInt(kind));
    return new TransactionInstruction({programId:DLP,keys,data:Buffer.concat([tag,args])});
};
export function delegateFeeBalance(payer:PublicKey,validator:PublicKey,index:number) {
    const balance=feeBalance(payer,index);
    const args=createDelegateInstruction({payer,delegatedAccount:balance,ownerProgram:SystemProgram.programId,validator},{commitFrequencyMs:0,seeds:[]}).data.subarray(8);
    return instruction(10,[key(payer,true,true),key(payer,true),key(balance,false,true),key(buffer(balance,SystemProgram.programId),false,true),key(record(balance),false,true),key(metadata(balance),false,true),key(SystemProgram.programId),key(DLP)],Buffer.concat([args,Buffer.from([index])]));
}
export function magicVaultInstructions(payer:PublicKey,validator:PublicKey) {
    const vault=magicFeeVaultPdaFromValidator(validator);
    const registered=PublicKey.findProgramAddressSync([Buffer.from("v-fees-vault"),validator.toBuffer()],DLP)[0];
    return [instruction(24,[key(payer,true,true),key(validator,true),key(registered),key(vault,false,true),key(SystemProgram.programId)]),
        instruction(25,[key(payer,true,true),key(validator,true),key(registered),key(vault,false,true),key(buffer(vault,DLP),false,true),key(record(vault),false,true),key(metadata(vault),false,true),key(SystemProgram.programId),key(DLP)])];
}
function signer(path:string) {
    const st=lstatSync(path);
    if (!st.isFile() || st.isSymbolicLink() || st.size>4096 || (st.mode&0o077)!==0) throw new Error("keypair must be a restricted regular file (chmod 600)");
    return Keypair.fromSecretKey(Uint8Array.from(JSON.parse(readFileSync(path,"utf8"))));
}
async function main() {
    const args=process.argv.slice(2);
    const allowed=new Set(["--send","--validator-vault"]);
    const balanceArg=args.find(a=>a.startsWith("--balance-lamports="));
    if (args.some(a=>!allowed.has(a) && a!==balanceArg)) throw new Error("usage: --balance-lamports=<initial SOL lamports> [--validator-vault] [--send]");
    const amount=Number(balanceArg?.split("=")[1]);
    if (!Number.isSafeInteger(amount) || amount<=0 || amount>1_000_000_000) throw new Error("explicit initial fee balance of 1..1e9 lamports required");
    const endpoint=process.env.CINDER_L1_RPC;
    if (!endpoint) throw new Error("CINDER_L1_RPC is required");
    const url=new URL(endpoint);
    if (!(url.protocol==="https:" || url.protocol==="http:" && ["127.0.0.1","localhost"].includes(url.hostname)) || url.username || url.password) throw new Error("HTTPS or localhost RPC required");
    const payer=signer(process.env.CINDER_OPERATOR_KEYPAIR || ".cinder-operator/operator.json");
    const connection=new Connection(endpoint,"confirmed");
    const idl=JSON.parse(readFileSync("target/idl/cinder_vault.json","utf8"));
    const program=new anchor.Program(idl,new anchor.AnchorProvider(connection,new anchor.Wallet(payer),{}));
    const config=PublicKey.findProgramAddressSync([Buffer.from("config")],program.programId)[0];
    const cfg=await (program.account as any).config.fetch(config);
    if (!cfg.adapter.equals(payer.publicKey)) throw new Error("operator does not match Config");
    const validator:PublicKey=cfg.erValidator;
    const index=Number(process.env.CINDER_FEE_BALANCE_INDEX || "0");
    const balance=feeBalance(payer.publicKey,index);
    // Never resend an initial deposit into an already delegated balance.
    // Replenishment is an explicit undelegate/reconcile/redelegate operation.
    if (await connection.getAccountInfo(record(balance)) || await connection.getAccountInfo(balance)) throw new Error("balance already exists: reconcile it before reprovisioning");
    const instructions=[createTopUpEscrowInstruction(balance,payer.publicKey,payer.publicKey,amount,index),delegateFeeBalance(payer.publicKey,validator,index)];
    if (args.includes("--validator-vault")) {
        if (await connection.getAccountInfo(magicFeeVaultPdaFromValidator(validator))) throw new Error("validator vault already exists; do not initialize it again");
        instructions.push(...magicVaultInstructions(payer.publicKey,validator));
    }
    if (!args.includes("--send")) {
        console.log(JSON.stringify(instructions.map(ix=>({program:ix.programId.toBase58(),accounts:ix.keys.map(k=>({address:k.pubkey.toBase58(),signer:k.isSigner,writable:k.isWritable})),data:ix.data.toString("base64")})),null,2));
        return;
    }
    const signers=[payer];
    if (args.includes("--validator-vault")) {
        if (!process.env.CINDER_VALIDATOR_KEYPAIR) throw new Error("validator signature requires CINDER_VALIDATOR_KEYPAIR; ask your PER operator if you do not own it");
        const authority=signer(process.env.CINDER_VALIDATOR_KEYPAIR);
        if (!authority.publicKey.equals(validator)) throw new Error("validator signer does not match Config");
        signers.push(authority);
    }
    // One atomic initial provisioning transaction: no automatic retry after an
    // uncertain send/confirmation, and never an operation on user USDC.
    const latest=await connection.getLatestBlockhash();
    const tx=new Transaction({feePayer:payer.publicKey,recentBlockhash:latest.blockhash}).add(...instructions);tx.sign(...signers);
    const signature=await connection.sendRawTransaction(tx.serialize(),{maxRetries:0});
    console.log(`Fee provisioning signature: ${signature}`);
    const result=await connection.confirmTransaction({...latest,signature},"finalized");
    if (result.value.err) throw new Error("fee provisioning rejected; inspect the printed signature");
    console.log("Fee provisioning finalized; wait for PER visibility before starting run");
}
if (require.main===module) main().catch(()=>{console.error("Fee provisioning failed. Do not retry an uncertain send before checking its signature/account state.");process.exitCode=1;});
