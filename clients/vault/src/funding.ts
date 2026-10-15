/** Exact unsigned P16 chain contract. No RPC, wallet loading, signing or submission. */
import { createHash } from 'node:crypto';
import type { Program } from '@anchor-lang/core';
import { ComputeBudgetProgram, PublicKey, SystemProgram, Transaction, TransactionInstruction } from '@solana/web3.js';
import type { CinderVault } from './cinder_vault.ts';
import { customerAddress, movement, receiptAddress, u64, vaultAddresses } from './index.ts';
import { escrowBatchAddress, sweepEscrowInstruction, venueEscrowAddresses } from './escrow.ts';

const TOKEN = new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
const ASSOCIATED = new PublicKey('ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL');
const fields = ['schema', 'rail', 'network', 'program', 'domain', 'pool', 'config', 'vault', 'mint', 'funds',
  'broker', 'broker_tokens', 'decimals', 'venue_program', 'venue_vault', 'epoch', 'operation', 'customer',
  'amount', 'sequence', 'paid', 'recipient_tokens', 'expires_at_slot'].sort();
const byteFields = ['network', 'program', 'domain', 'pool', 'config', 'vault', 'mint', 'funds', 'broker',
  'broker_tokens', 'venue_program', 'venue_vault', 'operation', 'customer', 'recipient_tokens'] as const;
const integerFields = ['epoch', 'amount', 'sequence', 'paid', 'expires_at_slot'] as const;
type ByteField = typeof byteFields[number];
type IntegerField = typeof integerFields[number];
type Rail = 'Release' | 'Deposit' | 'Return' | 'Payout' | 'Sweep';
type Sweep = {route:number[];escrow:number[];tokens:number[];batch:number[];operation:number[];expires_at_slot:string};
type Contract = Record<ByteField, number[]> & Record<IntegerField, string> &
  { schema: 'cinder-vault-funding-v1'|'cinder-vault-funding-v2'; rail: Rail; decimals: number; sweep?:Sweep };

function parse(bytes: Uint8Array): Contract {
  if (bytes.length === 0 || bytes.length > 8192) throw new Error('Contract size');
  const c = JSON.parse(Buffer.from(bytes).toString('utf8')) as Contract;
  const sweep=c?.rail==='Sweep', expected=sweep?[...fields,'sweep'].sort():fields;
  if (!c || typeof c !== 'object' || Array.isArray(c) || Object.keys(c).sort().join() !== expected.join()
    || c.schema !== (sweep?'cinder-vault-funding-v2':'cinder-vault-funding-v1') || !['Release', 'Deposit', 'Return', 'Payout','Sweep'].includes(c.rail)
    || !Number.isInteger(c.decimals) || c.decimals < 0 || c.decimals > 18) throw new Error('Contract schema');
  for (const key of byteFields) {
    if (!Array.isArray(c[key]) || c[key].length !== 32 || c[key].some(x => !Number.isInteger(x) || x < 0 || x > 255)
      || !['customer', 'recipient_tokens'].includes(key) && !c[key].some(x => x !== 0)) throw new Error('Contract identity');
  }
  for (const key of integerFields) {
    if (typeof c[key] !== 'string' || !/^(0|[1-9][0-9]{0,19})$/.test(c[key])) throw new Error('Contract integer');
    u64(BigInt(c[key]));
  }
  if (BigInt(c.epoch) === 0n || BigInt(c.amount) === 0n || BigInt(c.expires_at_slot) === 0n) throw new Error('Empty movement');
  if(sweep){
    const s=c.sweep!, names=['batch','escrow','expires_at_slot','operation','route','tokens'];
    if(!s||typeof s!=='object'||Array.isArray(s)||Object.keys(s).sort().join()!==names.join())throw new Error('Sweep schema');
    for(const k of ['route','escrow','tokens','batch','operation'] as const)
      if(!Array.isArray(s[k])||s[k].length!==32||s[k].some(x=>!Number.isInteger(x)||x<0||x>255)||!s[k].some(x=>x))throw new Error('Sweep identity');
    if(typeof s.expires_at_slot!=='string'||!/^[1-9][0-9]{0,19}$/.test(s.expires_at_slot)||BigInt(c.expires_at_slot)>BigInt(s.expires_at_slot))throw new Error('Sweep deadline');
    u64(BigInt(s.expires_at_slot));
    // Canonical nested-map order is part of the Rust contract binding too.
    c.sweep=Object.fromEntries(names.map(k=>[k,s[k as keyof Sweep]])) as Sweep;
  }
  // Rust's serde_json map emits sorted keys. Also rejects duplicate JSON keys,
  // alternate spellings and rounded numbers before computing the binding hash.
  const canonical = JSON.stringify(Object.fromEntries(expected.map(key => [key, c[key as keyof Contract]])));
  if (!Buffer.from(bytes).equals(Buffer.from(canonical))) throw new Error('Noncanonical contract');
  return c;
}
const key = (c: Contract, field: ByteField) => new PublicKey(Uint8Array.from(c[field]));
const ata = (mint: PublicKey, owner: PublicKey) => PublicKey.findProgramAddressSync(
  [owner.toBuffer(), TOKEN.toBuffer(), mint.toBuffer()], ASSOCIATED)[0];

/** Builds only the journal-selected rail, with exact u64s and explicit deployment keys. */
export async function fundingInstruction(program: Program<CinderVault>, bytes: Uint8Array) {
  const c = parse(bytes), programId = key(c, 'program'), mint = key(c, 'mint');
  const config = key(c, 'config'), vault = key(c, 'vault');
  const derived = vaultAddresses(programId, Uint8Array.from(c.domain), Uint8Array.from(c.pool), mint);
  if (!program.programId.equals(programId) || !derived.config.equals(config) || !derived.vault.equals(vault)
    || key(c, 'funds').equals(key(c, 'broker')) || vault.equals(key(c, 'broker_tokens'))
    || vault.equals(key(c, 'venue_vault')) || key(c, 'broker_tokens').equals(key(c, 'venue_vault'))) throw new Error('Contract route');
  const auth = movement(Uint8Array.from(c.domain), BigInt(c.epoch), Uint8Array.from(c.operation), BigInt(c.expires_at_slot));
  const common = { config, vault, mint, tokenProgram: TOKEN, systemProgram: SystemProgram.programId,
    receipt: receiptAddress(programId, config, Uint8Array.from(c.operation)) };
  let instruction: TransactionInstruction;
  const signer = key(c, c.rail === 'Return' || c.rail === 'Deposit' ? 'broker' : 'funds');
  if (c.rail !== 'Payout' && (BigInt(c.paid) !== 0n || c.recipient_tokens.some(x => x !== 0))) throw new Error('Internal payout fields');
  if(!['Payout','Sweep'].includes(c.rail)&&c.customer.some(x=>x))throw new Error('Internal owner field');
  switch (c.rail) {
    case 'Sweep': {
      if(!c.customer.some(x=>x)||!c.sweep)throw new Error('Sweep owner');
      const owner=key(c,'customer'), s=c.sweep, derived=venueEscrowAddresses(programId,config,owner);
      if(!derived.route.equals(new PublicKey(s.route))||!derived.escrow.equals(new PublicKey(s.escrow))
        ||!derived.escrowTokens.equals(new PublicKey(s.tokens))||!escrowBatchAddress(programId,derived.escrow,Uint8Array.from(s.operation)).equals(new PublicKey(s.batch)))throw new Error('Sweep route');
      instruction=await sweepEscrowInstruction(program,{domain:Uint8Array.from(c.domain),pool:Uint8Array.from(c.pool),mint,config,owner},Uint8Array.from(s.operation),
        {epoch:BigInt(c.epoch),operation:Uint8Array.from(c.operation),expiresAtSlot:BigInt(c.expires_at_slot)},signer,key(c,'broker_tokens'),BigInt(c.sequence));break;
    }
    case 'Release':
      instruction = await program.methods.releaseFunding(auth, u64(BigInt(c.amount)), u64(BigInt(c.sequence)))
        .accountsStrict({ ...common, funds: signer, brokerTokens: key(c, 'broker_tokens') }).instruction(); break;
    case 'Return':
      instruction = await program.methods.returnFunding(auth, u64(BigInt(c.amount)))
        .accountsStrict({ ...common, broker: signer, source: key(c, 'broker_tokens') }).instruction(); break;
    case 'Payout': {
      if (!c.customer.some(x => x !== 0) || !c.recipient_tokens.some(x => x !== 0)) throw new Error('Payout recipient');
      const owner = key(c, 'customer');
      instruction = await program.methods.normalPayout(auth, u64(BigInt(c.amount)), u64(BigInt(c.paid)), u64(BigInt(c.sequence)))
        .accountsStrict({ ...common, funds: signer, owner, customer: customerAddress(programId, config, owner),
          destination: key(c, 'recipient_tokens') }).instruction(); break;
    }
    case 'Deposit': {
      // Minimal, independently implemented public Pacifica deposit ABI; no SDK
      // program/mint defaults. Provenance and live-qualification limits: ADR 0016.
      const native = key(c, 'venue_program');
      const central = PublicKey.findProgramAddressSync([Buffer.from('central_state')], native)[0];
      const event = PublicKey.findProgramAddressSync([Buffer.from('__event_authority')], native)[0];
      if (!ata(mint, signer).equals(key(c, 'broker_tokens')) || !ata(mint, central).equals(key(c, 'venue_vault'))) {
        throw new Error('Native deposit route');
      }
      const data = Buffer.alloc(16); Buffer.from([242, 35, 198, 137, 82, 225, 242, 182]).copy(data);
      data.writeBigUInt64LE(BigInt(c.amount), 8);
      instruction = new TransactionInstruction({ programId: native, data, keys: [
        { pubkey: signer, isSigner: true, isWritable: true },
        { pubkey: key(c, 'broker_tokens'), isSigner: false, isWritable: true },
        { pubkey: central, isSigner: false, isWritable: true },
        { pubkey: key(c, 'venue_vault'), isSigner: false, isWritable: true },
        ...[TOKEN, ASSOCIATED, mint, SystemProgram.programId, event, native].map(pubkey => ({ pubkey, isSigner: false, isWritable: false })),
      ] }); break;
    }
  }
  return { instruction, signer, binding: createHash('sha256').update(bytes).digest() };
}

/** Qualified codec port only. It grants no sending capability; pass its output to
 * Controller.persist_wire inside the confidential runtime before any egress. */
export async function verifyFundingWire(program: Program<CinderVault>, contract: Uint8Array, wire: Uint8Array) {
  if (wire.length === 0 || wire.length > 1232) throw new Error('Wire size');
  const expected = await fundingInstruction(program, contract), tx = Transaction.from(wire);
  if (!tx.feePayer?.equals(expected.signer) || tx.signatures.length !== 1 || !tx.verifySignatures(true)) throw new Error('Wire signatures');
  if (!tx.serialize().equals(Buffer.from(wire))) throw new Error('Noncanonical wire');
  const instructions = [...tx.instructions];
  if (instructions[0]?.programId.equals(ComputeBudgetProgram.programId)) {
    const budget = instructions.shift()!;
    if (budget.keys.length !== 0 || budget.data.length !== 5 || budget.data[0] !== 2
      || budget.data.readUInt32LE(1) === 0 || budget.data.readUInt32LE(1) > 1_400_000) throw new Error('Compute budget');
  }
  if (instructions.length !== 1) throw new Error('Extra instruction');
  const actual = instructions[0], want = expected.instruction;
  if (!actual.programId.equals(want.programId) || !actual.data.equals(want.data) || actual.keys.length !== want.keys.length
    || actual.keys.some((m, i) => !m.pubkey.equals(want.keys[i].pubkey) || m.isSigner !== want.keys[i].isSigner
      || m.isWritable !== want.keys[i].isWritable)) throw new Error('Wire instruction');
  return { binding: expected.binding, signature: Buffer.from(tx.signature!), wire: Buffer.from(wire) };
}
