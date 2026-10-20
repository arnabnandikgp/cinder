import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, rmSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { manifest, commitment, approval, qualify, boundedBody, PATHS, ORIGIN, LIMITS } from '../tools/bulk-qualification/metadata.mjs';

const m = () => manifest('a'.repeat(64));
const a = () => ({ manifest: commitment(m()), approved: true, reference: 'offline fixture only', not_after: 100000 });
function fixture(replies = [[200, '[{"symbol":"BTC-USD","tickSize":0.00000001,"large":9007199254740993}]'], [200, '{}']]) {
  const calls = [], records = [];
  let now = 1000;
  return { calls, records, advance: t => { now = t; }, ports: {
    now: () => now,
    retain: (name, value) => { if (records.some(r => r.name === name)) throw Error('Existing artifact'); records.push({ name, value }); },
    fetch: async (url, options) => { calls.push({ url, options, retained: records.at(-1).name });
      const [status, body] = replies.shift(); return new Response(body, { status }); },
  } };
}
test('BULK metadata has fixed testnet GET-only scope and retains original bytes before interpretation', async () => {
  const f = fixture(), r = await qualify(m(), a(), f.ports);
  assert.equal(r.outcome, 'metadata-observed'); assert.equal(r.requests, 2);
  for (const [i, c] of f.calls.entries()) {
    assert.equal(c.url.origin, ORIGIN); assert.equal(c.url.pathname, PATHS[i]);
    assert.equal(c.options.method, 'GET'); assert.equal(c.options.redirect, 'error');
    assert.equal(c.options.credentials, 'omit'); assert.equal(c.retained, `request-${i + 1}`);
    assert.deepEqual(c.options.headers, { accept: 'application/json' }); assert.equal(c.options.body, undefined);
  }
  const raw = Buffer.from(f.records.find(r => r.name === 'response-1').value.raw_base64, 'base64').toString();
  assert(raw.includes('9007199254740993')); assert(raw.includes('0.00000001'));
  assert.deepEqual(f.records.find(r => r.name === 'observation-1').value, { market_count: 1, symbols: ['BTC-USD'] });
  for (const k of ['wallets', 'signatures', 'faucet', 'orders', 'transfers', 'rpc', 'aws']) assert.equal(r[k], 0);
  assert.equal(r.shipping_ready, false);
});
test('metadata refuses missing/expired approval and changed scope before any I/O', async () => {
  for (const change of [{ origin: 'https://mainnet-api1.bulk.trade' }, { paths: ['/api/v1/order'] },
    { limits: { ...LIMITS, requests: 3 } }, { wallets: 1 }, { shipping_ready: true }]) {
    const f = fixture(); await assert.rejects(qualify({ ...m(), ...change }, a(), f.ports));
    assert.equal(f.calls.length, 0); assert.equal(f.records.length, 0);
  }
  for (const value of [{}, { ...a(), approved: false }, { ...a(), not_after: 1000 }, { ...a(), not_after: 1000000 }]) {
    assert.throws(() => approval(m(), value, 1000));
  }
});
test('rate-limit/upstream/transport/schema failure stops without fallback, retry or signing', async () => {
  for (const [status, raw] of [[429, '{}'], [502, 'upstream'], [403, '{}'], [200, 'not JSON'], [200, '{}']]) {
    const f = fixture([[status, raw]]), r = await qualify(m(), a(), f.ports);
    assert.notEqual(r.outcome, 'metadata-observed'); assert.equal(f.calls.length, 1);
    assert(f.records.some(r => r.name === 'response-1')); assert.equal(r.signatures, 0);
  }
  const f = fixture(); f.ports.fetch = async () => { f.calls.push({}); throw Error('Uncertain transport'); };
  assert.equal((await qualify(m(), a(), f.ports)).outcome, 'unavailable'); assert.equal(f.calls.length, 1);
});
test('retention failure and rerun refusal prevent new network work', async () => {
  const f = fixture(); f.ports.retain = () => { throw Error('Storage unavailable'); };
  await assert.rejects(qualify(m(), a(), f.ports)); assert.equal(f.calls.length, 0);
  const second = fixture(); await qualify(m(), a(), second.ports);
  await assert.rejects(qualify(m(), a(), second.ports)); assert.equal(second.calls.length, 2);
  const after = fixture(), retain = after.ports.retain;
  after.ports.retain = (name, value) => { if (name === 'observation-1') throw Error('Disk unavailable'); retain(name, value); };
  await assert.rejects(qualify(m(), a(), after.ports)); assert.equal(after.calls.length, 1);
  assert(!after.records.some(r => r.name === 'result'));
});
test('late/backward clock cannot continue to the next request; oversize body cancels reader', async () => {
  for (const t of [999, 100000]) {
    const f = fixture(), get = f.ports.fetch;
    f.ports.fetch = async (...args) => { const r = await get(...args); f.advance(t); return r; };
    assert.equal((await qualify(m(), a(), f.ports)).outcome, 'expired'); assert.equal(f.calls.length, 1);
  }
  await assert.rejects(boundedBody(new Response('x'.repeat(LIMITS.body_bytes + 1))), /limit/);
});
test('CLI preparation performs no network; missing approval and altered source refuse execution', () => {
  const directory = mkdtempSync(join(tmpdir(), 'cinder-bulk-preflight-offline-'));
  const run = join(directory, 'new'), script = fileURLToPath(new URL('../tools/bulk-qualification/read.mjs', import.meta.url));
  const child = mode => execFileSync(process.execPath, ['--input-type=module', '-e',
    `globalThis.fetch=()=>{throw Error('OFFLINE NETWORK GUARD');};process.argv=[process.execPath,${JSON.stringify(script)},${JSON.stringify(mode)},${JSON.stringify(run)}];await import(${JSON.stringify(new URL('../tools/bulk-qualification/read.mjs', import.meta.url).href)});`],
    { encoding: 'utf8', stdio: 'pipe' });
  try {
    const prepared = JSON.parse(child('--prepare'));
    assert.equal(prepared.network_requests, 0); assert.equal(prepared.execution_approval, 'pending');
    assert.throws(() => child('--run-approved'));
    const altered = JSON.parse(readFileSync(join(run, 'manifest.json'))); altered.source = 'b'.repeat(64);
    writeFileSync(join(run, 'manifest.json'), JSON.stringify(altered), { mode: 0o600 });
    writeFileSync(join(run, 'approval.json'), JSON.stringify({ ...a(), manifest: commitment(altered), not_after: Date.now() + 10000 }), { mode: 0o600 });
    assert.throws(() => child('--run-approved'), error => error.stderr.includes('Changed runner/source scope'));
  } finally { rmSync(directory, { recursive: true, force: true }); }
});
