#!/usr/bin/env node
// Reproducible public synthetic vectors; no RPC, ambient wallet or real funding.
// Prints JSON only. Reviewed output belongs in the tracked fixture, not work/.
import { createPrivateKey, sign } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { fundingInstruction, verifyFundingWire } from '../clients/vault/src/funding.ts';
import { vaultAddresses, vaultProgram, movement, u64 } from '../clients/vault/src/index.ts';
const require = createRequire(new URL('../clients/vault/package.json', import.meta.url));
const { AnchorProvider } = require('@anchor-lang/core');
const { Connection, Keypair, PublicKey, Message, ComputeBudgetProgram, Transaction, SystemProgram } = require('@solana/web3.js');
const { TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync } = require('@solana/spl-token');
const idl = JSON.parse(readFileSync(new URL('../clients/vault/idl/cinder_vault.json', import.meta.url)));
// Public deterministic test seeds, never a deployment identity or live wallet.
const funds = Keypair.fromSeed(Buffer.alloc(32, 7)), broker = Keypair.fromSeed(Buffer.alloc(32, 9));
const owner = Keypair.fromSeed(Buffer.alloc(32, 11)).publicKey;
const mint = new PublicKey(Buffer.alloc(32, 23)), native = new PublicKey(Buffer.alloc(32, 25));
const domain = Buffer.alloc(32, 2), pool = Buffer.alloc(32, 18), operation = Buffer.alloc(32, 41);
const programId = new PublicKey(idl.address), { config, vault } = vaultAddresses(programId, domain, pool, mint);
const central = PublicKey.findProgramAddressSync([Buffer.from('central_state')], native)[0];
const brokerTokens = getAssociatedTokenAddressSync(mint, broker.publicKey);
const venueVault = getAssociatedTokenAddressSync(mint, central, true);
const bytes = k => [...(k instanceof PublicKey ? k.toBytes() : k)];
const provider = new AnchorProvider(new Connection('http://127.0.0.1:18899'), {
  publicKey: funds.publicKey, signTransaction: async () => { throw Error('No implicit signing'); },
  signAllTransactions: async () => { throw Error('No implicit signing'); },
}, {});
const program = vaultProgram(provider, idl), blockhash = new PublicKey(Buffer.alloc(32, 51)).toBase58();
const vectors = [];
for (const rail of ['Release', 'Deposit', 'Return', 'Payout']) for (const units of [0, 300_000]) {
  const c = { schema:'cinder-vault-funding-v1', rail, network:bytes(Buffer.alloc(32,1)),
    program:bytes(programId), domain:bytes(domain), pool:bytes(pool), config:bytes(config), vault:bytes(vault),
    mint:bytes(mint), funds:bytes(funds.publicKey), broker:bytes(broker.publicKey), broker_tokens:bytes(brokerTokens),
    decimals:6, venue_program:bytes(native), venue_vault:bytes(venueVault), epoch:'1', operation:bytes(operation),
    customer:rail==='Payout'?bytes(owner):bytes(Buffer.alloc(32)), amount:'9007199254740993', sequence:'3',
    paid:rail==='Payout'?'9007199254740994':'0',
    recipient_tokens:rail==='Payout'?bytes(getAssociatedTokenAddressSync(mint,owner)):bytes(Buffer.alloc(32)),
    expires_at_slot:'18446744073709551615' };
  const contract = Buffer.from(JSON.stringify(Object.fromEntries(Object.keys(c).sort().map(k=>[k,c[k]]))));
  const { instruction:ix, signer } = await fundingInstruction(program, contract);
  // The narrow Rust codec's stable key order is intentional, not web3.js's
  // locale-dependent ordering. web3.js independently decodes/verifies the packet
  // and Anchor independently builds the desired instruction and account flags.
  const keys=[signer];
  for(const writable of [true,false]) for(const m of ix.keys) {
    if(!m.isSigner && m.isWritable===writable && !keys.some(k=>k.equals(m.pubkey))) keys.push(m.pubkey);
  }
  if(!keys.some(k=>k.equals(ix.programId))) keys.push(ix.programId);
  if(units) keys.push(ComputeBudgetProgram.programId);
  const index=k=>keys.findIndex(x=>x.equals(k));
  const instructions=[];
  if(units) {
    const data=Buffer.alloc(5);data[0]=2;data.writeUInt32LE(units,1);
    instructions.push({programIdIndex:index(ComputeBudgetProgram.programId),accounts:[],data:require('bs58').encode(data)});
  }
  instructions.push({programIdIndex:index(ix.programId),accounts:ix.keys.map(m=>index(m.pubkey)),data:require('bs58').encode(ix.data)});
  const message=new Message({header:{numRequiredSignatures:1,numReadonlySignedAccounts:0,
    numReadonlyUnsignedAccounts:keys.slice(1).filter(k=>!ix.keys.some(m=>m.pubkey.equals(k)&&m.isWritable)).length},
    accountKeys:keys,recentBlockhash:blockhash,instructions});
  // Sign exactly these bytes; Transaction.sign would recompile/sort the keys.
  const seed=Buffer.alloc(32,['Deposit','Return'].includes(rail)?9:7);
  const privateKey=createPrivateKey({key:Buffer.concat([Buffer.from('302e020100300506032b657004220420','hex'),seed]),format:'der',type:'pkcs8'});
  const signature=sign(null,message.serialize(),privateKey);
  const wire=Buffer.concat([Buffer.from([1]),signature,message.serialize()]);
  await verifyFundingWire(program,contract,wire);
  vectors.push({rail,compute_units:units,contract:c,message:[...message.serialize()],wire:[...wire]});
}
const deposits=[];
for(const compute_units of [0,300_000]) {
  const customer=PublicKey.findProgramAddressSync([Buffer.from('customer'),config.toBuffer(),owner.toBuffer()],programId)[0];
  const receipt=PublicKey.findProgramAddressSync([Buffer.from('deposit'),config.toBuffer(),owner.toBuffer(),operation],programId)[0];
  const source=getAssociatedTokenAddressSync(mint,owner);
  const ix=await program.methods.deposit(movement(domain,1n,operation,(1n<<64n)-1n),u64(9007199254740993n)).accountsStrict({config,customer,owner,source,vault,mint,receipt,tokenProgram:TOKEN_PROGRAM_ID,systemProgram:SystemProgram.programId}).instruction();
  const tx=new Transaction({feePayer:owner,recentBlockhash:blockhash});
  if(compute_units)tx.add(ComputeBudgetProgram.setComputeUnitLimit({units:compute_units}));
  tx.add(ix);tx.sign(Keypair.fromSeed(Buffer.alloc(32,11)));
  deposits.push({account:bytes(Buffer.alloc(32,1)),operation:bytes(operation),compute_units,wire:[...tx.serialize()]});
}
const provenance='Anchor 1.2 IDL + web3.js independent instruction and signature verification; synthetic offline identities only';
process.stdout.write(JSON.stringify(process.argv.includes('--deposits-only')?
  {schema:'cinder-customer-deposit-public-v1',provenance,deposits}:
  {schema:'cinder-funding-codec-public-fixtures-v1',provenance,vectors})+'\n');
