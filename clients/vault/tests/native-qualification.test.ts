// Offline codec/layout tests. No ignored research, RPC or live identity.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash, createPublicKey, verify } from 'node:crypto';
import { createRequire } from 'node:module';
import { Keypair, PublicKey, Transaction, TransactionInstruction, SystemProgram } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync } from '@solana/spl-token';
const require=createRequire(import.meta.url);
const { ROUTE,decodeSigner,signNative,nativeInstruction,batchWithdrawal,verifyDeployment,verifyMint,verifyToken }=require('../../../tools/native-qualification/bindings.mjs');
const owner=Keypair.fromSeed(Buffer.alloc(32,77)).publicKey;
test('native diagnostic owns signer bytes after clearing parse input; signatures bind the expected public identity',()=>{
  const fixture=Keypair.fromSeed(Buffer.alloc(32,79)),raw=Array.from(fixture.secretKey);
  const key=decodeSigner(raw,fixture.publicKey.toBase58());assert(raw.every(n=>n===0));
  const expected=createPublicKey({key:Buffer.concat([Buffer.from('302a300506032b6570032100','hex'),fixture.publicKey.toBuffer()]),format:'der',type:'spki'});
  const message=Buffer.from('{"data":{"disabled":true},"expiry_window":30000,"timestamp":1748970123456,"type":"set_auto_lend_disabled"}');
  assert(verify(null,message,expected,signNative(message,key)));
  const tx=new Transaction({feePayer:key.publicKey,recentBlockhash:PublicKey.default.toBase58()});
  tx.add(SystemProgram.transfer({fromPubkey:key.publicKey,toPubkey:owner,lamports:1}));tx.sign(key);assert(tx.verifySignatures());
  assert.throws(()=>decodeSigner(Array.from(fixture.secretKey),owner.toBase58()),/identity/);
  for(const value of [[],Array(64).fill(-1),Array(64).fill(256),Array(64).fill(1.5)])assert.throws(()=>decodeSigner(value,fixture.publicKey.toBase58()),/shape/);
  // Reproduce the old aliasing defect independently. A stored public address
  // survives, but the erased retained seed must fail expected-public verification.
  const borrowed=Uint8Array.from(fixture.secretKey),broken=Keypair.fromSecretKey(borrowed);borrowed.fill(0);
  assert(broken.publicKey.equals(fixture.publicKey));
  assert.throws(()=>signNative(message,broken),/identity\/signature/);
});
test('native qualification reproduces the official deposit/faucet discriminators, u64 and account privileges',()=>{
  for(const name of ['deposit','mint_test_usdc']) {
    const ix=nativeInstruction(name,owner.toBase58());
    assert(ix.programId.equals(new PublicKey(ROUTE.program)));
    assert.deepEqual(ix.data.subarray(0,8),createHash('sha256').update(`global:${name}`).digest().subarray(0,8));
    assert.equal(ix.data.readBigUInt64LE(8),20000000n);assert.equal(ix.keys.length,name==='deposit'?10:8);
    assert.deepEqual(ix.keys.filter((x:{isSigner:boolean})=>x.isSigner).map((x:{pubkey:PublicKey})=>x.pubkey.toBase58()),[owner.toBase58()]);
    assert.deepEqual(ix.keys.map((x:{isWritable:boolean})=>x.isWritable),name==='deposit'?
      [true,true,true,true,false,false,false,false,false,false]:[true,true,true,true,false,false,false,false]);
    assert(ix.keys[name==='deposit'?1:2].pubkey.equals(getAssociatedTokenAddressSync(new PublicKey(ROUTE.mint),owner)));
    assert.throws(()=>nativeInstruction(name,owner.toBase58(),1n));
  }
  assert.throws(()=>nativeInstruction('withdraw',owner.toBase58()));
});
test('native qualification refuses wrong loader metadata, changed slot and authority before funding',()=>{
  // Independent protocol identifier, not a fixture owner copied from ROUTE.
  // https://solana.com/docs/core/programs/program-deployment (2026-10-08).
  const loader='BPFLoaderUpgradeab1e11111111111111111111111';
  assert.equal(ROUTE.loader,loader);
  for(const field of ['program','mint','central','vault','program_data','loader','upgrade'])assert.equal(new PublicKey(ROUTE[field]).toBase58(),ROUTE[field]);
  assert.equal(PublicKey.findProgramAddressSync([new PublicKey(ROUTE.program).toBuffer()],new PublicKey(loader))[0].toBase58(),ROUTE.program_data);
  const p=Buffer.alloc(36);p.writeUInt32LE(2);new PublicKey(ROUTE.program_data).toBuffer().copy(p,4);
  const d=Buffer.alloc(46);d.writeUInt32LE(3);d.writeBigUInt64LE(BigInt(ROUTE.slot),4);d[12]=1;new PublicKey(ROUTE.upgrade).toBuffer().copy(d,13);
  const account=(bytes:Buffer,executable:boolean)=>({owner:loader,executable,data:[bytes.toString('base64'),'base64']});
  const program=account(p,true),data=account(d,false);assert.equal(verifyDeployment(program,data).length,1);
  assert.throws(()=>verifyDeployment({...program,owner:TOKEN_PROGRAM_ID.toBase58()},data));
  assert.throws(()=>verifyDeployment({...program,owner:'BPFLoaderUpgradeab1e11111111111111111111'},data));
  d[4]^=1;assert.throws(()=>verifyDeployment(program,account(d,false)));d[4]^=1;
  d[13]^=1;assert.throws(()=>verifyDeployment(program,account(d,false)));
});
test('native qualification token layouts bind classic token program, quote mint, owner and encumbrances',()=>{
  const b=Buffer.alloc(165);new PublicKey(ROUTE.mint).toBuffer().copy(b);owner.toBuffer().copy(b,32);b.writeBigUInt64LE(20000000n,64);b[108]=1;
  const account=()=>({owner:TOKEN_PROGRAM_ID.toBase58(),executable:false,data:[b.toString('base64'),'base64']});
  assert.equal(verifyToken(account(),owner.toBase58()),20000000n);
  assert.throws(()=>verifyToken(account(),Keypair.fromSeed(Buffer.alloc(32,78)).publicKey.toBase58()));
  for(const offset of [72,108,109,129]){b[offset]^=2;assert.throws(()=>verifyToken(account(),owner.toBase58()));b[offset]^=2;}
  assert.throws(()=>verifyToken({...account(),owner:ROUTE.program},owner.toBase58()));
  const mint=Buffer.alloc(82);mint[44]=6;mint[45]=1;
  const m=()=>({owner:TOKEN_PROGRAM_ID.toBase58(),executable:false,data:[mint.toString('base64'),'base64']});
  assert.doesNotThrow(()=>verifyMint(m()));mint[44]=9;assert.throws(()=>verifyMint(m()));
});
test('native payment ABI joins retained ACK nonce, recipient and net without inventing completion',()=>{
  const authority=Keypair.fromSeed(Buffer.alloc(32,81)),program=new PublicKey(ROUTE.program),
    pda=(seed:string)=>PublicKey.findProgramAddressSync([Buffer.from(seed)],program)[0];
  const accounts=[authority.publicKey,new PublicKey(ROUTE.central),new PublicKey(ROUTE.vault),pda('pacifica-fallback'),
    TOKEN_PROGRAM_ID,ASSOCIATED_TOKEN_PROGRAM_ID,new PublicKey(ROUTE.mint),SystemProgram.programId,pda('__event_authority'),
    program,getAssociatedTokenAddressSync(new PublicKey(ROUTE.mint),owner)];
  const data=Buffer.alloc(68);createHash('sha256').update('global:batch_withdraw').digest().subarray(0,8).copy(data);
  data.writeUInt32LE(1,8);data.writeBigUInt64LE(19000000n,12);data.writeBigUInt64LE(9007199254740993n,20);
  owner.toBuffer().copy(data,28);data.writeBigUInt64LE(42n,60);
  const transaction=new Transaction({feePayer:authority.publicKey,recentBlockhash:PublicKey.default.toBase58()});
  transaction.add(new TransactionInstruction({programId:program,data,keys:accounts.map((pubkey,i)=>({pubkey,isSigner:i===0,isWritable:i<4||i===10}))}));
  transaction.sign(authority);assert(transaction.verifySignatures());
  const message=transaction.compileMessage(),base58=require('bs58'),fixture={version:'legacy',transaction:{signatures:[base58.encode(transaction.signature!)],message:{
    accountKeys:message.accountKeys.map((pubkey,i)=>({pubkey:pubkey.toBase58(),signer:message.isAccountSigner(i),writable:message.isAccountWritable(i)})),
    instructions:[{programId:program.toBase58(),accounts:accounts.map(a=>a.toBase58()),data:base58.encode(data)}]}}};
  const ack={batch:42,gross:'20000000',fee:'1000000'},decode=(t=fixture,a=ack)=>batchWithdrawal(t,owner.toBase58(),a);
  assert.deepEqual(decode(),{batch:42,net:'19000000',withdraw_id:'9007199254740993',instruction_observed:true,financial_completion:false,source_cut:null});
  const corrupt=(edit:(v:typeof fixture)=>void)=>{const v=structuredClone(fixture);edit(v);assert.throws(()=>decode(v));};
  for(const offset of [0,8,12,28,60])corrupt(v=>{const b=Buffer.from(base58.decode(v.transaction.message.instructions[0].data));b[offset]^=1;v.transaction.message.instructions[0].data=base58.encode(b);});
  corrupt(v=>v.transaction.message.instructions[0].data=base58.encode(Buffer.concat([data,Buffer.alloc(1)])));
  corrupt(v=>v.transaction.message.instructions[0].programId=TOKEN_PROGRAM_ID.toBase58());
  corrupt(v=>v.transaction.message.instructions[0].accounts[10]=owner.toBase58());
  corrupt(v=>v.transaction.message.accountKeys[1].writable=!v.transaction.message.accountKeys[1].writable);
  corrupt(v=>v.transaction.message.accountKeys[1].signer=true);
  corrupt(v=>v.transaction.message.instructions.push(v.transaction.message.instructions[0]));
  corrupt(v=>v.transaction.message.accountKeys.push(v.transaction.message.accountKeys[0]));
  corrupt(v=>v.version='0');
  for(const a of [{...ack,batch:43},{...ack,batch:Number.MAX_SAFE_INTEGER+1},{...ack,gross:'1e7'},
    {...ack,fee:'3000000'},{...ack,fee:'0'}])assert.throws(()=>decode(fixture,a));
  assert.throws(()=>batchWithdrawal(fixture,Keypair.fromSeed(Buffer.alloc(32,82)).publicKey.toBase58(),ack));
});
