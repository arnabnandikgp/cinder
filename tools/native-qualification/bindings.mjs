// Public Pacifica ABI, independently reproduced and checked against the official
// IDL at 4748feca93efe2b2a5a0c95993e40f68d0ca4338 (2026-10-08).
// Full IDL SHA256 a31bb37868338e189fead472c1481954ed876844e83529e2e68b5ef0dbf427a1.
// The already locked vault SDK is an explicit legacy transaction-codec boundary;
// it is not a new client dependency, RPC default or shipping provider.
import { createRequire } from 'node:module';
import { createPrivateKey, createPublicKey, sign, verify } from 'node:crypto';
const require=createRequire(new URL('../../clients/vault/package.json',import.meta.url));
export const web3=require('@solana/web3.js'), token=require('@solana/spl-token'), base58=require('bs58');
export const ROUTE=Object.freeze({program:'peRPsYCcB1J9jvrs29jiGdjkytxs8uHLmSPLKKP9ptm',
  mint:'USDPqRbLidFGufty2s3oizmDEKdqx7ePTqzDMbf5ZKM',decimals:6,
  central:'2zPRq1Qvdq5A4Ld6WsH7usgCge4ApZRYfhhf5VAjfXxv',vault:'5SDFdHZGTZbyRYu54CgmRkCGnPHC5pYaN27p7XGLqnBs',
  program_data:'BMTXJd9CQAt2C2mUYH3URMoXrwZrHjgDPsTxHJx3coDK',
  // Canonical loader-v3 ID, independently pinned by Solana's program-deployment reference.
  loader:'BPFLoaderUpgradeab1e11111111111111111111111',
  // Historical deployment identity, revalidated before any funding, not source equivalence.
  slot:'376257391',upgrade:'49uAsjdwJzu3K3T2E86mUX1n5e5MLQajUdNJsmgPLCHK'});
const {PublicKey,TransactionInstruction,SystemProgram}=web3;
for(const field of ['program','mint','central','vault','program_data','loader','upgrade']) {
  if(new PublicKey(ROUTE[field]).toBase58()!==ROUTE[field])throw Error('Noncanonical native route identity');
}
export const pub=s=>new PublicKey(s),pda=(seeds)=>PublicKey.findProgramAddressSync(seeds,pub(ROUTE.program))[0];
export const ata=owner=>token.getAssociatedTokenAddressSync(pub(ROUTE.mint),pub(owner),true);
export function decodeSigner(raw,expected) {
  if(!Array.isArray(raw)||raw.length!==64||raw.some(n=>!Number.isInteger(n)||n<0||n>255))throw Error('Signer file shape');
  const temporary=Uint8Array.from(raw),owned=Uint8Array.from(temporary);raw.fill(0);let retained=false;
  try {
    // web3 Keypair retains the input array. It must own a separate copy before
    // clearing the parse buffer; clearing a borrowed array destroys its signer.
    const key=web3.Keypair.fromSecretKey(owned);
    if(key.publicKey.toBase58()!==expected)throw Error('Signer identity');
    retained=true;return key;
  } finally {temporary.fill(0);if(!retained)owned.fill(0);}
}
export function signNative(message,key) {
  const secret=createPrivateKey({key:Buffer.concat([Buffer.from('302e020100300506032b657004220420','hex'),Buffer.from(key.secretKey.subarray(0,32))]),format:'der',type:'pkcs8'});
  const publicKey=createPublicKey({key:Buffer.concat([Buffer.from('302a300506032b6570032100','hex'),key.publicKey.toBuffer()]),format:'der',type:'spki'});
  const signature=sign(null,message,secret);
  // Verify against the configured identity, not the private key used to sign.
  if(!verify(null,message,publicKey,signature))throw Error('Native signer identity/signature');
  return signature;
}
export function nativeInstruction(kind,owner,amount=20000000n) {
  if(!['deposit','mint_test_usdc'].includes(kind)||amount!==20000000n)throw Error('Native instruction scope');
  const who=pub(owner),central=pda([Buffer.from('central_state')]);
  if(central.toBase58()!==ROUTE.central||ata(central.toBase58()).toBase58()!==ROUTE.vault)throw Error('Route derivation');
  const meta=(pubkey,isSigner=false,isWritable=false)=>({pubkey,isSigner,isWritable});
  const data=Buffer.alloc(16);Buffer.from(kind==='deposit'?[242,35,198,137,82,225,242,182]:[118,144,78,118,155,214,185,186]).copy(data);data.writeBigUInt64LE(amount,8);
  const keys=kind==='deposit'?[meta(who,true,true),meta(ata(owner),false,true),meta(central,false,true),meta(pub(ROUTE.vault),false,true),
    meta(token.TOKEN_PROGRAM_ID),meta(token.ASSOCIATED_TOKEN_PROGRAM_ID),meta(pub(ROUTE.mint)),meta(SystemProgram.programId),
    meta(pda([Buffer.from('__event_authority')])),meta(pub(ROUTE.program))]:
    [meta(who,true,true),meta(pda([Buffer.from('user_account'),who.toBuffer()]),false,true),meta(ata(owner),false,true),
      meta(pub(ROUTE.mint),false,true),meta(central),meta(token.ASSOCIATED_TOKEN_PROGRAM_ID),meta(token.TOKEN_PROGRAM_ID),meta(SystemProgram.programId)];
  return new TransactionInstruction({programId:pub(ROUTE.program),keys,data});
}
// Strict one-recipient diagnostic ABI observation, not a finalized-payment or
// native no-later-effect certificate. It joins the ACK batch to actual on-chain
// instruction bytes; it does not recover an ACK that was never persisted.
export function batchWithdrawal(tx,broker,ack) {
  const message=tx?.transaction?.message,ix=message?.instructions;
  if(tx?.version!=='legacy'||!Array.isArray(ix)||ix.length!==1
    ||!Array.isArray(message.accountKeys)||message.accountKeys.length!==11
    ||!Array.isArray(tx.transaction.signatures)||tx.transaction.signatures.length!==1
    ||!Number.isSafeInteger(ack?.batch)||ack.batch<0)throw Error('Native payment ABI scope');
  const instruction=ix[0],keys=message.accountKeys;
  const signer=keys.filter(k=>k?.signer===true);
  if(signer.length!==1||signer[0]!==keys[0]||instruction.programId!==ROUTE.program
    ||instruction.accounts?.length!==11||typeof instruction.data!=='string')throw Error('Native payment program/accounts');
  const authority=pub(signer[0].pubkey).toBase58(),who=pub(broker);
  const expected=[authority,ROUTE.central,ROUTE.vault,pda([Buffer.from('pacifica-fallback')]).toBase58(),
    token.TOKEN_PROGRAM_ID.toBase58(),token.ASSOCIATED_TOKEN_PROGRAM_ID.toBase58(),ROUTE.mint,
    SystemProgram.programId.toBase58(),pda([Buffer.from('__event_authority')]).toBase58(),ROUTE.program,ata(broker).toBase58()];
  if(new Set(keys.map(k=>k.pubkey)).size!==keys.length
    ||new Set(expected).size!==expected.length
    ||expected.some((k,i)=>instruction.accounts[i]!==k)
    ||expected.some(k=>keys.filter(x=>x.pubkey===k).length!==1))throw Error('Native payment route');
  const writable=new Set([authority,ROUTE.central,ROUTE.vault,expected[3],expected[10]]);
  if(keys.some(k=>k.signer!==(k.pubkey===authority)||k.writable!==writable.has(k.pubkey)))throw Error('Native payment privileges');
  const data=Buffer.from(base58.decode(instruction.data));
  if(base58.encode(data)!==instruction.data||data.length!==68
    ||!data.subarray(0,8).equals(Buffer.from([37,76,149,71,94,36,245,195]))
    ||data.readUInt32LE(8)!==1||!data.subarray(28,60).equals(who.toBuffer())
    ||data.readBigUInt64LE(60)!==BigInt(ack.batch))throw Error('Native payment batch/recipient');
  for(const value of [ack.gross,ack.fee])if(typeof value!=='string'||value.length>20||!/^(0|[1-9][0-9]*)$/.test(value))throw Error('Native payment amount');
  const net=data.readBigUInt64LE(12),gross=BigInt(ack.gross),fee=BigInt(ack.fee);
  if(net===0n||gross!==20000000n||fee>2000000n||net!==gross-fee)throw Error('Native payment net amount');
  return {batch:ack.batch,net:net.toString(),withdraw_id:data.readBigUInt64LE(20).toString(),
    instruction_observed:true,financial_completion:false,source_cut:null};
}
export function verifyDeployment(program,data) {
  const p=Buffer.from(program?.data?.[0]??'','base64'),d=Buffer.from(data?.data?.[0]??'','base64');
  if(program?.owner!==ROUTE.loader||program.executable!==true||p.length!==36||p.readUInt32LE(0)!==2
    ||new PublicKey(p.subarray(4)).toBase58()!==ROUTE.program_data||data?.owner!==ROUTE.loader||data.executable!==false
    ||d.length<45||d.readUInt32LE(0)!==3||d.readBigUInt64LE(4).toString()!==ROUTE.slot||d[12]!==1
    ||new PublicKey(d.subarray(13,45)).toBase58()!==ROUTE.upgrade)throw Error('Native deployment drift');
  return d.subarray(45);
}
export function verifyMint(value) {
  const b=Buffer.from(value?.data?.[0]??'','base64');
  if(value?.owner!==token.TOKEN_PROGRAM_ID.toBase58()||value.executable!==false||b.length!==82||b[44]!==6||b[45]!==1)throw Error('Quote mint layout');
}
export function verifyToken(value,owner) {
  const b=Buffer.from(value?.data?.[0]??'','base64');
  if(value?.owner!==token.TOKEN_PROGRAM_ID.toBase58()||value.executable!==false||b.length!==165
    ||new PublicKey(b.subarray(0,32)).toBase58()!==ROUTE.mint||new PublicKey(b.subarray(32,64)).toBase58()!==owner
    ||b[108]!==1||b.readUInt32LE(72)!==0||b.readUInt32LE(109)!==0||b.readUInt32LE(129)!==0)throw Error('Token route layout/authority');
  return b.readBigUInt64LE(64);
}
