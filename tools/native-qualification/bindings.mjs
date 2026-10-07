// Public Pacifica ABI, independently reproduced and checked against the official
// IDL at 4748feca93efe2b2a5a0c95993e40f68d0ca4338 (2026-10-08).
// Full IDL SHA256 a31bb37868338e189fead472c1481954ed876844e83529e2e68b5ef0dbf427a1.
// The already locked vault SDK is an explicit legacy transaction-codec boundary;
// it is not a new client dependency, RPC default or shipping provider.
import { createRequire } from 'node:module';
const require=createRequire(new URL('../../clients/vault/package.json',import.meta.url));
export const web3=require('@solana/web3.js'), token=require('@solana/spl-token'), base58=require('bs58');
export const ROUTE=Object.freeze({program:'peRPsYCcB1J9jvrs29jiGdjkytxs8uHLmSPLKKP9ptm',
  mint:'USDPqRbLidFGufty2s3oizmDEKdqx7ePTqzDMbf5ZKM',decimals:6,
  central:'2zPRq1Qvdq5A4Ld6WsH7usgCge4ApZRYfhhf5VAjfXxv',vault:'5SDFdHZGTZbyRYu54CgmRkCGnPHC5pYaN27p7XGLqnBs',
  program_data:'BMTXJd9CQAt2C2mUYH3URMoXrwZrHjgDPsTxHJx3coDK',
  loader:'BPFLoaderUpgradeab1e11111111111111111111',
  // Historical deployment identity, revalidated before any funding, not source equivalence.
  slot:'376257391',upgrade:'49uAsjdwJzu3K3T2E86mUX1n5e5MLQajUdNJsmgPLCHK'});
const {PublicKey,TransactionInstruction,SystemProgram}=web3;
export const pub=s=>new PublicKey(s),pda=(seeds)=>PublicKey.findProgramAddressSync(seeds,pub(ROUTE.program))[0];
export const ata=owner=>token.getAssociatedTokenAddressSync(pub(ROUTE.mint),pub(owner),true);
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
