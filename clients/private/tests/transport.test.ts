// Real separate service/relay/verifier processes, TLS and protected disk journal.
// All keys/accounts/market data are disposable fixtures. No venue/RPC/AWS calls.
import test from 'node:test';
import assert from 'node:assert/strict';
import { spawn, execFile } from 'node:child_process';
import { once } from 'node:events';
import { generateKeyPairSync, sign, randomBytes } from 'node:crypto';
import { mkdtemp, mkdir, readdir, readFile, rm, cp } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import net from 'node:net';
import tls from 'node:tls';
import { PrivateClient, type MessageSigner, type Command } from '../src/index.ts';
import { AttestedNodeChannel, nativeQuoteVerifier, type QuoteVerifier, type ReleasePolicy } from '../src/node-channel.ts';

const bins = resolve(process.env.CARGO_TARGET_DIR ?? 'target', 'debug');
const id = (n: number) => new Uint8Array(32).fill(n);
const domain = { network: id(1), deployment: id(2) };
const policy: ReleasePolicy = { ...domain, manifest: id(21), pcrs: [new Uint8Array(48).fill(22), new Uint8Array(48).fill(23), new Uint8Array(48).fill(24)] };
function verifier(root: string): QuoteVerifier {
  return { verify: input => new Promise((resolve, reject) => {
    const child = execFile(join(bins, 'cinder-verify-fixture'), [root], { encoding: 'buffer', timeout: 4000, maxBuffer: 1024 }, (error, out) => error ? reject(new Error('fixture quote rejected')) : resolve(out));
    child.stdin?.on('error', () => reject(new Error('fixture verifier failed'))); child.stdin?.end(input);
  }) };
}
async function process_(name: string, args: string[], key?: Buffer) {
  const child = spawn(join(bins, name), args, { stdio: ['pipe', 'pipe', 'pipe'] });
  let out = '', error = ''; child.stderr.on('data', b => { error += b.toString(); });
  child.stdout.on('data', b => { out += b.toString(); });
  if (key) child.stdin.write(key);
  const line = await new Promise<string>((resolve, reject) => {
    const timer = setTimeout(() => { child.kill('SIGKILL'); reject(new Error('fixture startup deadline')); }, 5000);
    child.stdout.on('data', () => { if (out.includes('\n')) { clearTimeout(timer); resolve(out.split('\n')[0]); } });
    child.once('exit', () => { clearTimeout(timer); reject(new Error(`fixture did not start: ${error}`)); });
    child.once('error', e => { clearTimeout(timer); reject(e); });
  });
  const stop = async (crash = false) => {
    if (child.exitCode !== null || child.signalCode !== null) return;
    const done = once(child, 'exit');
    if (crash) child.kill('SIGKILL'); else child.stdin.end();
    const timer = setTimeout(() => child.kill('SIGKILL'), 5000);
    const [code, signal] = await done; clearTimeout(timer);
    if (!crash) { assert.equal(code, 0, error); assert.equal(signal, null); }
  };
  return { child, line, stop, logs: () => out + error };
}
async function observer(target: number) {
  const captured: Buffer[] = [], sockets = new Set<net.Socket>();
  let corruptRequest = false, corruptReply = false;
  const server = net.createServer(client => {
    const upstream = net.connect(target, '127.0.0.1'); sockets.add(client); sockets.add(upstream);
    client.on('data', b => { captured.push(Buffer.from(b)); if (corruptRequest) { corruptRequest = false; b[b.length - 1] ^= 1; } });
    upstream.on('data', b => { captured.push(Buffer.from(b)); if (corruptReply) { corruptReply = false; b[b.length - 1] ^= 1; } });
    for (const s of [client, upstream]) { s.on('error', () => { client.destroy(); upstream.destroy(); }); s.on('close', () => { sockets.delete(s); client.destroy(); upstream.destroy(); }); }
    client.pipe(upstream); upstream.pipe(client);
  });
  server.listen(0, '127.0.0.1'); await once(server, 'listening');
  return { port: (server.address() as net.AddressInfo).port, captured,
    corruptNextRequest: () => { corruptRequest = true; }, corruptNextReply: () => { corruptReply = true; },
    stop: async () => { for (const s of sockets) s.destroy(); await new Promise<void>(r => server.close(() => r())); } };
}
async function files(root: string): Promise<Buffer[]> {
  const out: Buffer[] = [];
  for (const e of await readdir(root, { withFileTypes: true })) {
    const p = join(root, e.name); if (e.isDirectory()) out.push(...await files(p)); else out.push(await readFile(p));
  }
  return out;
}
async function harness() {
  const root = await mkdtemp(join(tmpdir(), 'cinder-p19-')); const store = join(root, 'parent-storage'); await mkdir(store, { mode: 0o700 });
  const keys = generateKeyPairSync('ed25519'); const publicKey = new Uint8Array(keys.publicKey.export({ format: 'der', type: 'spki' }).subarray(-32));
  const signer: MessageSigner = { publicKey, signMessage: async b => sign(null, b, keys.privateKey) };
  const key = randomBytes(32); let service = await process_('cinder-service-fixture', ['127.0.0.1:0', store, Buffer.from(publicKey).toString('hex')], key);
  const startNetwork = async () => {
    const [addr, ca] = service.line.split(' '), port = Number(addr.split(':')[1]);
    const observed = await observer(port); const relay = await process_('cinder-relay', ['127.0.0.1:0', `127.0.0.1:${observed.port}`]);
    const connect = (v: QuoteVerifier = verifier(ca), p: ReleasePolicy = policy) => AttestedNodeChannel.connect({ host: '127.0.0.1', port: Number(relay.line.split(':')[1]), policy: p, verifier: v });
    return { observed, relay, ca, connect };
  };
  let network = await startNetwork(); const sessions: AttestedNodeChannel[] = [];
  const connect = async (v?: QuoteVerifier, p?: ReleasePolicy) => { const ch = await network.connect(v, p); sessions.push(ch); return ch; };
  return { root, store, signer, key, connect, network: () => network, service: () => service,
    client: (ch: AttestedNodeChannel) => new PrivateClient(ch, signer, { domain, account: id(1), policy: 1 }),
    restart: async () => {
      for (const s of sessions) s.close(); await network.relay.stop(); await network.observed.stop(); await service.stop(true);
      service = await process_('cinder-service-fixture', ['127.0.0.1:0', store, Buffer.from(publicKey).toString('hex')], key); network = await startNetwork();
    },
    close: async () => { for (const s of sessions) s.close(); await network.relay.stop(); await network.observed.stop(); await service.stop(); key.fill(0); await rm(root, { recursive: true }); } };
}
function request(id_: number, command: Command) { return { id: id(id_), epoch: 1n, expiresAt: BigInt(Date.now() + 20_000), command }; }
function closed(socket: net.Socket): Promise<void> { return socket.destroyed ? Promise.resolve() : new Promise(resolve => socket.once('close', () => resolve())); }

test('SDK reaches actual processes; durable dispatch/retry/query survive service death without second attempt or plaintext parent data', { timeout: 30_000 }, async () => {
  const h = await harness();
  try {
    let ch = await h.connect(); let client = h.client(ch);
    const view = await client.request(request(30, { kind: 'view' })); assert.equal(view.kind, 'view'); if (view.kind !== 'view') throw new Error(); assert.equal(view.cash, 1000n);
    const command: Command = { kind: 'order', market: id(7), lots: 2n, minimum: 90n, maximum: 110n, fee: 1n, tif: 'GTC', reduceOnly: false, goodUntil: BigInt(Date.now() + 60_000) };
    const first = await client.request(request(40, command)); assert.equal(first.kind, 'receipt'); if (first.kind !== 'receipt') throw new Error(); assert.equal(first.outcome, 'dispatched'); assert.equal(first.possiblyExposed, true);
    const head = await readFile(join(h.store, 'accepted'));
    const captured = Buffer.concat(h.network().observed.captured);
    assert.ok(captured.length > 1000);
    for (const marker of [Buffer.from('CINDER-API\0'), Buffer.from('CINDER-API-REPLY\0'), Buffer.from('CINDER-API-RECORD-1\0'), Buffer.from('synthetic-source-only'), Buffer.from(h.signer.publicKey), h.key]) {
      assert.equal(captured.includes(marker), false);
      for (const b of await files(h.store)) assert.equal(b.includes(marker), false);
      assert.equal(Buffer.from(h.network().relay.logs()).includes(marker), false);
    }
    await h.restart(); ch = await h.connect(); client = h.client(ch);
    assert.deepEqual(await client.request(request(40, command)), first);
    assert.deepEqual(await client.request(request(41, { kind: 'operation', target: id(40) })), first);
    assert.deepEqual(await readFile(join(h.store, 'accepted')), head);
    const after = await client.request(request(42, { kind: 'view' })); assert.equal(after.kind, 'view'); if (after.kind !== 'view') throw new Error(); assert.equal(after.cash, 1000n); assert.deepEqual(after.operations, [id(40)]);
    assert.deepEqual(await client.request(request(40, { ...command, lots: 1n })), { kind: 'error', code: 'conflict' });
  } finally { await h.close(); }
});

test('production root rejects fixture; wrong release, socket substitution and replay fail before private exchange', { timeout: 30_000 }, async () => {
  const h = await harness();
  try {
    const head = await readFile(join(h.store, 'accepted'));
    await assert.rejects(h.connect(nativeQuoteVerifier(join(bins, 'cinder-verify-quote'))));
    await assert.rejects(h.connect(undefined, { ...policy, manifest: id(99) }));
    await assert.rejects(h.connect({ verify: input => { const changed = Buffer.from(input); changed[272] ^= 1; return verifier(h.network().ca).verify(changed); } }));
    let old: Uint8Array | undefined;
    const ch = await h.connect({ verify: input => { old = input.slice(); return verifier(h.network().ca).verify(input); } }); ch.close();
    await assert.rejects(h.connect({ verify: _ => verifier(h.network().ca).verify(old!) }));
    assert.deepEqual(await readFile(join(h.store, 'accepted')), head);
    const good = await h.connect(); const reply = await h.client(good).request(request(1, { kind: 'view' })); assert.equal(reply.kind, 'view');
  } finally { await h.close(); }
});

test('bounded requests, backpressure and orderly shutdown do not create financial mutations', { timeout: 20_000 }, async () => {
  const h = await harness();
  try {
    const ch = await h.connect(); const head = await readFile(join(h.store, 'accepted'));
    const first = ch.exchange(new Uint8Array([1])); await assert.rejects(ch.exchange(new Uint8Array([1]))); await first;
    await assert.rejects(ch.exchange(new Uint8Array(1025))); await assert.rejects(ch.exchange(new Uint8Array([1])));
    const next = await h.connect(); for (let i = 0; i < 128; i++) await next.exchange(new Uint8Array([1])); await assert.rejects(next.exchange(new Uint8Array([1])));
    assert.deepEqual(await readFile(join(h.store, 'accepted')), head);
    const live = await h.connect(); await h.service().stop(); await assert.rejects(live.exchange(new Uint8Array([1])));
  } finally { await h.close(); }
});

test('old ciphertext cannot restore an accepted newer head; missing witness refuses startup rather than resetting it', { timeout: 30_000 }, async () => {
  const h = await harness();
  try {
    const old = join(h.root, 'old'); await cp(h.store, old, { recursive: true });
    const ch = await h.connect(); await h.client(ch).request(request(50, { kind: 'revoke' }));
    const head = await readFile(join(h.store, 'accepted'));
    // Delete only disposable test replicas. Preserve current accepted register.
    for (const name of ['first', 'second']) { await rm(join(h.store, name), { recursive: true }); await cp(join(old, name), join(h.store, name), { recursive: true }); }
    await assert.rejects(h.restart(), /fixture did not start/);
    assert.deepEqual(await readFile(join(h.store, 'accepted')), head);
  } finally { await h.close(); }
});

test('TLS downgrade/plaintext fail closed; finite handshake capacity and deadlines recover after idle peers', { timeout: 20_000 }, async () => {
  const h = await harness(); const sockets: net.Socket[] = [];
  try {
    const port = Number(h.service().line.split(' ')[0].split(':')[1]);
    let established = false;
    const legacy = tls.connect({ host: '127.0.0.1', port, maxVersion: 'TLSv1.2', rejectUnauthorized: false, ALPNProtocols: ['cinder-private/1'] });
    legacy.on('secureConnect', () => { established = true; });
    legacy.on('error', () => {}); sockets.push(legacy);
    await Promise.race([once(legacy, 'error'), once(legacy, 'close')]); assert.equal(established, false);
    const plain = net.connect(port, '127.0.0.1'); plain.on('error', () => {}); sockets.push(plain);
    await once(plain, 'connect'); plain.write('GET /health HTTP/1.1\r\n\r\n'); await closed(plain);
    // Each connect is ordered, so listener has admitted all eight idle sessions.
    for (let i = 0; i < 8; i++) { const s = net.connect(port, '127.0.0.1'); s.on('error', () => {}); sockets.push(s); await once(s, 'connect'); }
    const ninth = net.connect(port, '127.0.0.1'); ninth.on('error', () => {}); sockets.push(ninth);
    await closed(ninth);
    await Promise.all(sockets.map(closed));
    const good = await h.connect(); assert.equal((await h.client(good).request(request(1, { kind: 'view' }))).kind, 'view');
  } finally { for (const s of sockets) s.destroy(); await h.close(); }
});

test('witness loss produces only an encrypted redacted failure, never a stale account view', { timeout: 20_000 }, async () => {
  const h = await harness();
  try {
    const ch = await h.connect(); await rm(join(h.store, 'accepted'));
    assert.deepEqual(await h.client(ch).request(request(1, { kind: 'view' })), { kind: 'error', code: 'unavailable' });
    assert.equal(h.service().logs().includes('CINDER-API'), false);
    await assert.rejects(h.restart(), /fixture did not start/);
    await assert.rejects(readFile(join(h.store, 'accepted')));
  } finally { await h.close(); }
});

test('hostile relay corruption cannot forge a request; lost committed reply reconciles without another economic attempt', { timeout: 30_000 }, async () => {
  const h = await harness();
  try {
    const command: Command = { kind: 'order', market: id(7), lots: 2n, minimum: 90n, maximum: 110n, fee: 1n, tif: 'GTC', reduceOnly: false, goodUntil: BigInt(Date.now() + 60_000) };
    const before = await readFile(join(h.store, 'accepted')); let channel = await h.connect();
    h.network().observed.corruptNextRequest(); await assert.rejects(h.client(channel).request(request(60, command)), /reconcile/);
    assert.deepEqual(await readFile(join(h.store, 'accepted')), before);
    channel = await h.connect(); h.network().observed.corruptNextReply();
    await assert.rejects(h.client(channel).request(request(60, command)), /reconcile/);
    const committed = await readFile(join(h.store, 'accepted')); assert.notDeepEqual(committed, before);
    await h.restart(); channel = await h.connect(); const client = h.client(channel);
    const known = await client.request(request(61, { kind: 'operation', target: id(60) }));
    assert.equal(known.kind, 'receipt'); if (known.kind !== 'receipt') throw new Error(); assert.equal(known.outcome, 'dispatched'); assert.equal(known.possiblyExposed, true);
    assert.deepEqual(await client.request(request(60, command)), known);
    assert.deepEqual(await readFile(join(h.store, 'accepted')), committed);
  } finally { await h.close(); }
});
