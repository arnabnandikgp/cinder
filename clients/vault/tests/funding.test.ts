import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { AnchorProvider, type Idl, type Wallet } from '@anchor-lang/core';
import { ComputeBudgetProgram, Connection, Keypair, PublicKey, SystemProgram, Transaction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync } from '@solana/spl-token';
import { fundingInstruction, verifyFundingWire } from '../src/funding.ts';
import { identity, movement, receiptAddress, u64, vaultAddresses, vaultProgram } from '../src/index.ts';

// No server, live key or signing service: codec-only disposable local identities.
const idl = JSON.parse(readFileSync(new URL('../idl/cinder_vault.json', import.meta.url), 'utf8')) as Idl;
const key = () => Keypair.generate();
const funds = key(), broker = key(), owner = key(), mint = key().publicKey, native = key().publicKey;
const domain = createHash('sha256').update('funding codec deployment').digest();
const pool = createHash('sha256').update('funding codec pool').digest();
const operation = createHash('sha256').update('funding codec operation').digest();
const programId = new PublicKey(idl.address), { config, vault } = vaultAddresses(programId, domain, pool, mint);
const central = PublicKey.findProgramAddressSync([Buffer.from('central_state')], native)[0];
const brokerTokens = getAssociatedTokenAddressSync(mint, broker.publicKey);
const venueVault = getAssociatedTokenAddressSync(mint, central, true);
const bytes = (k: PublicKey | Uint8Array) => Array.from(k instanceof PublicKey ? k.toBytes() : k);
const provider = new AnchorProvider(new Connection('http://127.0.0.1:18899'), {
  publicKey: funds.publicKey, signTransaction: async () => { throw new Error('No implicit signing'); },
  signAllTransactions: async () => { throw new Error('No implicit signing'); },
} as unknown as Wallet, {});
const program = vaultProgram(provider, idl);
function contract(rail = 'Release', overrides: Record<string, unknown> = {}) {
  const c: Record<string, unknown> = { schema: 'cinder-vault-funding-v1', rail, network: bytes(Buffer.alloc(32, 1)),
    program: bytes(programId), domain: bytes(domain), pool: bytes(pool), config: bytes(config), vault: bytes(vault),
    mint: bytes(mint), funds: bytes(funds.publicKey), broker: bytes(broker.publicKey), broker_tokens: bytes(brokerTokens),
    decimals: 6, venue_program: bytes(native), venue_vault: bytes(venueVault), epoch: '1', operation: bytes(operation),
    customer: bytes(owner.publicKey), amount: '9007199254740993', sequence: '0', paid: '0',
    recipient_tokens: rail === 'Payout' ? bytes(getAssociatedTokenAddressSync(mint, owner.publicKey)) : bytes(Buffer.alloc(32)),
    expires_at_slot: '18446744073709551615', ...overrides };
  return Buffer.from(JSON.stringify(Object.fromEntries(Object.keys(c).sort().map(k => [k, c[k]]))));
}
function signed(ix: Awaited<ReturnType<typeof fundingInstruction>>['instruction'], signer = funds) {
  const tx = new Transaction({ feePayer: signer.publicKey, recentBlockhash: key().publicKey.toBase58() }).add(ix);
  tx.sign(signer); return tx.serialize();
}
test('funding contract builds the exact Anchor release with u64s above JavaScript safe integers', async () => {
  const input = contract(), built = await fundingInstruction(program, input);
  const expected = await program.methods.releaseFunding(movement(domain, 1n, operation, (1n << 64n) - 1n),
    u64(9007199254740993n), u64(0n)).accountsStrict({ config, vault, mint, funds: funds.publicKey, brokerTokens,
      receipt: receiptAddress(programId, config, operation), tokenProgram: TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId }).instruction();
  assert.deepEqual(built.instruction, expected);
  const verified = await verifyFundingWire(program, input, signed(built.instruction));
  assert.equal(verified.signature.length, 64); assert.equal(verified.binding.length, 32);
  assert.deepEqual(verified.binding, createHash('sha256').update(input).digest());
});
test('each allowed physical rail verifies its exact disposable signed wire', async () => {
  for (const rail of ['Release', 'Return', 'Deposit', 'Payout']) {
    const input = contract(rail), built = await fundingInstruction(program, input);
    await verifyFundingWire(program, input, signed(built.instruction, ['Deposit', 'Return'].includes(rail) ? broker : funds));
    if (rail === 'Deposit') {
      assert.deepEqual([...built.instruction.data.subarray(0, 8)], [242, 35, 198, 137, 82, 225, 242, 182]);
      assert.equal(built.instruction.data.readBigUInt64LE(8), 9007199254740993n);
      assert.equal(built.instruction.keys.length, 10);
    }
  }
});
test('P23 Rust funding codec vectors independently verify against Anchor and web3.js', async () => {
  const fixture = JSON.parse(readFileSync(new URL('../../../crates/pacifica/tests/fixtures/funding-codec-public.json', import.meta.url), 'utf8'));
  assert.equal(fixture.schema, 'cinder-funding-codec-public-fixtures-v1');
  assert.equal(fixture.vectors.length, 8);
  for (const v of fixture.vectors) {
    const c = v.contract;
    const contract = Buffer.from(JSON.stringify(Object.fromEntries(Object.keys(c).sort().map(k => [k, c[k]]))));
    const wire = Buffer.from(v.wire);
    const decoded = Transaction.from(wire);
    assert.deepEqual(wire.subarray(65), Buffer.from(v.message));
    assert.deepEqual(decoded.serializeMessage(), Buffer.from(v.message));
    assert.equal(decoded.instructions.length, v.compute_units ? 2 : 1);
    await verifyFundingWire(program, contract, wire);
    const altered = Buffer.from(wire); altered[altered.length - 1] ^= 1;
    await assert.rejects(verifyFundingWire(program, contract, altered));
  }
});
test('original customer deposit fixtures independently match Anchor and valid owner signatures', async () => {
  const fixtures = JSON.parse(readFileSync(new URL('../../../crates/pacifica/tests/fixtures/customer-deposit-public.json', import.meta.url), 'utf8'));
  const c = JSON.parse(readFileSync(new URL('../../../crates/pacifica/tests/fixtures/funding-codec-public.json', import.meta.url), 'utf8')).vectors[6].contract;
  assert.equal(fixtures.schema, 'cinder-customer-deposit-public-v1');
  assert.equal(fixtures.deposits.length, 2);
  const pk = (k: string) => new PublicKey(c[k]);
  const owner = pk('customer'), config = pk('config');
  for (const v of fixtures.deposits) {
    const wire = Buffer.from(v.wire), tx = Transaction.from(wire);
    assert(tx.verifySignatures());
    assert.deepEqual(tx.serialize(), wire);
    assert(tx.feePayer?.equals(owner));
    const customer = PublicKey.findProgramAddressSync([Buffer.from('customer'), config.toBuffer(), owner.toBuffer()], programId)[0];
    const receipt = PublicKey.findProgramAddressSync([Buffer.from('deposit'), config.toBuffer(), owner.toBuffer(), Buffer.from(v.operation)], programId)[0];
    const expected = await program.methods.deposit(movement(Buffer.from(c.domain), 1n, Buffer.from(v.operation), (1n << 64n) - 1n),
      u64(9007199254740993n)).accountsStrict({config, customer, owner, source: pk('recipient_tokens'),
      vault: pk('vault'), mint: pk('mint'), receipt, tokenProgram: TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId}).instruction();
    assert.equal(tx.instructions.length, v.compute_units ? 2 : 1);
    assert.deepEqual(tx.instructions.at(-1), expected);
    if (v.compute_units) assert.deepEqual(tx.instructions[0], ComputeBudgetProgram.setComputeUnitLimit({units: v.compute_units}));
    const corrupt = Buffer.from(wire); corrupt[1] ^= 1;
    assert(!Transaction.from(corrupt).verifySignatures());
  }
});
test('schema, integer, deployment, account and unsupported rail substitution fail closed', async () => {
  for (const change of [{ amount: 1 }, { amount: '01' }, { amount: '-1' }, { amount: '18446744073709551616' },
    { epoch: '0' }, { program: bytes(key().publicKey) }, { config: bytes(key().publicKey) },
    { rail: 'Withdraw' }, { arbitrary: true }, { broker: bytes(funds.publicKey) },
    { domain: Array(32).fill(256) }, { recipient_tokens: bytes(key().publicKey) }]) {
    await assert.rejects(fundingInstruction(program, contract('Release', change)));
  }
  await assert.rejects(fundingInstruction(program, Buffer.from(contract().toString().replace('"amount":', '"amount":"2","amount":'))));
  await assert.rejects(fundingInstruction(program, contract('Deposit', { venue_vault: bytes(key().publicKey) })));
  await assert.rejects(fundingInstruction(program, contract('Deposit', { broker_tokens: bytes(key().publicKey) })));
});
test('wire substitution, extra transfer, wrong signature and stale counter binding reject', async () => {
  const input = contract(), built = await fundingInstruction(program, input);
  const wire = signed(built.instruction);
  const corrupt = Buffer.from(wire); corrupt[1] ^= 1;
  await assert.rejects(verifyFundingWire(program, input, corrupt));
  await assert.rejects(verifyFundingWire(program, input, Buffer.concat([wire,Buffer.from([0])])));
  await assert.rejects(verifyFundingWire(program, contract('Release', { sequence: '1' }), wire));
  const tx = Transaction.from(wire).add(SystemProgram.transfer({ fromPubkey: funds.publicKey, toPubkey: owner.publicKey, lamports: 1 }));
  tx.sign(funds);
  await assert.rejects(verifyFundingWire(program, input, tx.serialize()));
  const foreign = await fundingInstruction(program, contract('Release', { broker_tokens: bytes(key().publicKey) }));
  await assert.rejects(verifyFundingWire(program, input, signed(foreign.instruction)));
  await assert.rejects(verifyFundingWire(program, input, Buffer.alloc(1233)));
  assert.equal(identity(operation).length, 32);
});
