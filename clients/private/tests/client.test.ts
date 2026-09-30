import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash, createPrivateKey, createPublicKey, sign, verify } from 'node:crypto';
import { signingMessage, intentDigest, decodeResponse, PrivateClient, type Envelope, type Command, type ConfidentialChannel } from '../src/index.ts';
const id = (n: number) => new Uint8Array(32).fill(n);
const secret = createPrivateKey({ key: Buffer.concat([Buffer.from('302e020100300506032b657004220420', 'hex'), Buffer.alloc(32, 9)]), format: 'der', type: 'pkcs8' });
const publicKey = new Uint8Array(createPublicKey(secret).export({ format: 'der', type: 'spki' }).subarray(-32));
const signer = { publicKey, async signMessage(bytes: Uint8Array) { return new Uint8Array(sign(null, bytes, secret)); } };
const domain = { network: id(1), deployment: id(2) };
const ctx = { ...domain, binding: id(8), expiresAt: 100n };
function envelope(command: Command): Envelope { return { domain, account: id(1), id: id(40), policy: 1, epoch: 1n, signer: publicKey, session: id(8), expiresAt: 100n, command }; }
const commands: Command[] = [{ kind: 'view' }, { kind: 'operation', target: id(40) },
  { kind: 'order', market: id(7), lots: -2n, minimum: 90n, maximum: 9007199254740997n, fee: 1n << 100n, tif: 'IOC', reduceOnly: true, goodUntil: 90n },
  { kind: 'cancel', target: id(40), attempt: id(50), goodUntil: 90n },
  { kind: 'payout', net: 10n, maximumFee: 2n, allowPartial: true, goodUntil: 90n },
  { kind: 'grant', grant: { key: publicKey, methods: 7, market: id(7), maximumLots: 2n, maximumFee: 1n, maximumOrders: 1n, expiresAt: 90n } },
  { kind: 'revoke' }, { kind: 'leverage', market: id(7), leverage: 20000n, goodUntil: 90n }];
const hex = (b: Uint8Array) => Buffer.from(b).toString('hex');
const reply = new Uint8Array(Buffer.from(readFileSync(new URL('../../../crates/api/tests/fixtures/reply.hex', import.meta.url), 'utf8').trim(), 'hex'));
const header = new TextEncoder().encode('CINDER-API-REPLY\0\x00\x01').length;
const input = (command: Command) => ({ id: id(40), epoch: 1n, expiresAt: 100n, command });
function client(channel: ConfidentialChannel) { return new PrivateClient(channel, signer, { domain, account: id(1), policy: 1 }); }

test('all eight methods match shared Rust bytes, SHA-256 and independent Ed25519 signatures', async () => {
  const lines = readFileSync(new URL('../../../crates/api/tests/fixtures/vectors.tsv', import.meta.url), 'utf8').trim().split('\n');
  assert.equal(lines.length, commands.length);
  for (let i = 0; i < commands.length; i++) {
    const [kind, wire, hash, digest, signature] = lines[i].split('\t'); const e = envelope(commands[i]); const m = signingMessage(e);
    assert.equal(e.command.kind, kind); assert.equal(hex(m), wire); assert.equal(createHash('sha256').update(m).digest('hex'), hash);
    assert.equal(hex(await intentDigest(e)), digest); assert.equal(hex(await signer.signMessage(m)), signature);
    assert.equal(verify(null, m, createPublicKey(secret), Buffer.from(signature, 'hex')), true);
  }
});
test('economic retry identity is session/epoch independent but financial deadline sensitive', async () => {
  const e = envelope(commands[2]); const d = await intentDigest(e);
  const reconnect: Envelope = { ...e, session: id(9), signer: id(12), epoch: 2n, expiresAt: 110n };
  assert.equal(hex(await intentDigest(reconnect)), hex(d));
  assert.notEqual(hex(signingMessage({ ...e, session: id(9) })), hex(signingMessage(e)));
  const c = commands[2]; assert.equal(c.kind, 'order'); if (c.kind !== 'order') return;
  assert.notEqual(hex(await intentDigest({ ...e, command: { ...c, goodUntil: 91n } })), hex(d));
});
test('shared lifecycle reply preserves signed lots, actual payments, fee cap and unknown', () => {
  const r = decodeResponse(reply); assert.equal(r.kind, 'receipt'); if (r.kind !== 'receipt') return;
  assert.equal(r.outcome, 'unknown'); assert.equal(r.filled, -1n); assert.equal(r.paid, 4n); assert.equal(r.railFees, 1n);
  assert.equal(r.feeCap, 10n); assert.equal(r.possiblyExposed, true); assert.equal(r.allowPartial, true);
  for (const [tag, outcome] of ['rejected', 'accepted', 'dispatched', 'acknowledged', 'partial', 'complete', 'unknown'].entries()) {
    const b = reply.slice(); b[header + 1 + 64 + 4] = tag;
    const parsed = decodeResponse(b); assert.equal(parsed.kind, 'receipt'); if (parsed.kind === 'receipt') assert.equal(parsed.outcome, outcome);
  }
});
test('bounded reply parser rejects truncation, trailing data, bad tags and booleans', () => {
  for (let n = 0; n < reply.length; n++) assert.throws(() => decodeResponse(reply.slice(0, n)), /Invalid private protocol/);
  assert.throws(() => decodeResponse(new Uint8Array([...reply, 0])));
  const bad = reply.slice(); bad[bad.length - 1] = 2; assert.throws(() => decodeResponse(bad));
  bad[header] = 255; assert.throws(() => decodeResponse(bad));
  assert.throws(() => decodeResponse(new Uint8Array(1_048_577)));
});
test('SDK signs exact channel-bound bytes and validates response request/digest', async () => {
  const c = commands[2]; let calls = 0; let captured: Uint8Array | undefined;
  const digest = await intentDigest(envelope(c)); const r = reply.slice(); r.set(digest, header + 1 + 32);
  const sdk = client({ context: () => ctx, async exchange(wire) {
    calls++; captured = wire;
    assert.equal(verify(null, wire.subarray(0, -64), createPublicKey(secret), wire.subarray(-64)), true);
    assert.equal(hex(wire.subarray(0, -64)), hex(signingMessage(envelope(c)))); return r.slice();
  } });
  const response = await sdk.request(input(c)); assert.equal(response.kind, 'receipt'); assert.equal(calls, 1);
  assert.ok(captured?.every(b => b === 0));
});
test('unknown transport failure is redacted and never triggers execution retry', async () => {
  let calls = 0; const sdk = client({ context: () => ctx, async exchange() { calls++; throw new Error('private-native-secret'); } });
  await assert.rejects(sdk.request(input(commands[2])), e => e instanceof Error && !e.message.includes('secret') && e.message.includes('reconcile'));
  assert.equal(calls, 1);
});
test('wrong domain refuses before signing or exchanging; wrong reply digest/id refuses', async () => {
  let signed = 0, sent = 0;
  const sdk = new PrivateClient({ context: () => ({ ...ctx, network: id(3) }), async exchange() { sent++; return reply.slice(); } },
    { publicKey, async signMessage() { signed++; return new Uint8Array(64); } }, { domain, account: id(1), policy: 1 });
  await assert.rejects(sdk.request(input(commands[2])), /session mismatch/); assert.equal(signed, 0); assert.equal(sent, 0);
  await assert.rejects(client({ context: () => ctx, async exchange() { return reply.slice(); } }).request(input(commands[2])), /reconcile/);
  const wrongId = reply.slice(); wrongId.set(id(41), header + 1);
  await assert.rejects(client({ context: () => ctx, async exchange() { return wrongId; } }).request(input(commands[1])), /reconcile/);
});
test('confidential errors are typed and redacted instead of outer native error bodies', async () => {
  const bytes = new TextEncoder().encode('CINDER-API-REPLY\0\x00\x01\x02\x03');
  assert.deepEqual(await client({ context: () => ctx, async exchange() { return bytes.slice(); } }).request(input(commands[2])), { kind: 'error', code: 'conflict' });
});
test('finite precision is enforced without floats, negative fees, ambiguous TIF or overflow', () => {
  const c = commands[2]; if (c.kind !== 'order') return;
  for (const cmd of [{ ...c, lots: 1n << 63n }, { ...c, fee: -1n }, { ...c, tif: 'market' }, { ...c, fee: 1n << 127n }, { ...c, reduceOnly: 1 }]) {
    assert.throws(() => signingMessage(envelope(cmd as Command)));
  }
  assert.throws(() => signingMessage({ ...envelope(c), epoch: 1 as unknown as bigint }));
});
