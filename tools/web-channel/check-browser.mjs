#!/usr/bin/env node
// Actual headless Chrome + Node/WASM -> opaque loopback carrier -> native Rust.
// No AWS, wallet, venue, account identity, real attestation or npm dependencies.
import { spawn, execFileSync } from 'node:child_process';
import { createServer } from 'node:http';
import { readFile, mkdtemp, rm } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';
import { run } from './web/suite.mjs';
import { stopChild } from './stop-child.mjs';

const root = fileURLToPath(new URL('.', import.meta.url));
const native = join(root, 'target/debug/qualification-responder');
const chrome = process.env.CINDER_TEST_CHROME ?? (process.platform === 'darwin'
  ? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' : '/usr/bin/google-chrome');
if (!existsSync(native) || !existsSync(chrome)) throw Error('Build native fixture and install Chrome first');
const children = new Set();
const sessions = new Map();
const marker = Buffer.from('PRIVATE-WEB-QUALIFICATION-NO-PARENT-PLAINTEXT');
let leakage = false, nextId = 0, resultResolve;
const browserResult = new Promise(resolve => { resultResolve = resolve; });
const profile = await mkdtemp(join(tmpdir(), 'cinder-channel-chrome-'));
const maximum = 16384 + 16;
async function body(request, cap) {
  const chunks = []; let n = 0;
  for await (const chunk of request) {
    n += chunk.length;
    if (n > cap) throw Error('bounded carrier');
    chunks.push(chunk);
  }
  return Buffer.concat(chunks, n);
}
function fixture() {
  const child = spawn(native, [], { stdio: ['pipe', 'pipe', 'ignore'] });
  children.add(child);
  let buffered = '', pending, dead = false;
  function fail() {
    dead = true;
    if (pending) { clearTimeout(pending.timer); pending.reject(Error('fixture unavailable')); pending = undefined; }
  }
  child.on('error', fail);
  child.on('exit', () => { children.delete(child); fail(); });
  child.stdin.on('error', fail);
  child.stdout.setEncoding('ascii');
  child.stdout.on('data', chunk => {
    buffered += chunk;
    if (buffered.length > 2 * maximum + 1) { fail(); child.kill(); return; }
    const end = buffered.indexOf('\n');
    if (end < 0) return;
    const line = buffered.slice(0, end);
    buffered = buffered.slice(end + 1);
    if (!pending || buffered.length || !/^(?:[0-9a-f]{2})+$/.test(line)) { fail(); child.kill(); return; }
    const current = pending; pending = undefined;
    clearTimeout(current.timer);
    current.resolve(Buffer.from(line, 'hex'));
  });
  function receive(wire) {
    if (pending || dead) return Promise.reject(Error('fixture serialization'));
    return new Promise((resolve, reject) => {
      pending = { resolve, reject, timer: setTimeout(() => { fail(); child.kill(); }, 5000) };
      if (wire) child.stdin.write(`${wire.toString('hex')}\n`);
    });
  }
  return { child, receive, key: receive(), calls: 0 };
}
const files = new Map([
  ['/', ['web/index.html', 'text/html']], ['/suite.mjs', ['web/suite.mjs', 'text/javascript']],
  ['/channel.js', ['pkg/channel.js', 'text/javascript']], ['/channel_bg.wasm', ['pkg/channel_bg.wasm', 'application/wasm']],
]);
const server = createServer(async (request, response) => {
  request.setTimeout(5000, () => request.destroy());
  try {
    if (request.method === 'GET' && files.has(request.url)) {
      const [name, type] = files.get(request.url);
      response.setHeader('Content-Type', type);
      response.end(await readFile(join(root, name))); return;
    }
    if (request.method === 'POST' && request.url === '/new') {
      if (nextId >= 16) throw Error('fixture session limit');
      const session = fixture(); const id = String(nextId++); sessions.set(id, session);
      const key = await session.key;
      if (key.length !== 32) throw Error('fixture public key');
      response.setHeader('Content-Type', 'application/json');
      response.end(JSON.stringify({ id, key: [...key] })); return;
    }
    const match = /^\/wire\/(\d{1,2})$/.exec(request.url ?? '');
    if (request.method === 'POST' && match) {
      const session = sessions.get(match[1]);
      if (!session || ++session.calls > 130) throw Error('fixture missing');
      const wire = await body(request, maximum);
      leakage ||= wire.includes(marker);
      response.setHeader('Content-Type', 'application/octet-stream');
      const reply = await session.receive(wire);
      leakage ||= reply.includes(marker);
      response.end(reply); return;
    }
    if (request.method === 'POST' && request.url === '/result') {
      const result = JSON.parse((await body(request, 2048)).toString());
      resultResolve(result); response.end('OK'); return;
    }
    response.writeHead(404).end();
  } catch {
    response.writeHead(400).end('Fixture unavailable');
  }
});
server.headersTimeout = 5000;
server.requestTimeout = 5000;
let timeout, browser;
try {
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const base = `http://127.0.0.1:${server.address().port}`;
  const core = await import('./pkg/channel.js');
  await core.default({ module_or_path: await readFile(join(root, 'pkg/channel_bg.wasm')) });
  const nodeChecks = await run(core, base);
  browser = spawn(chrome, [
    '--headless', '--no-first-run', '--disable-background-networking', '--disable-component-update',
    '--disable-sync', '--disable-default-apps', '--no-proxy-server', `--user-data-dir=${profile}`, base,
  ], { stdio: 'ignore', detached: true });
  browser.on('error', () => resultResolve({ ok: false, error: 'Chrome launch failure' }));
  browser.on('exit', () => resultResolve({ ok: false, error: 'Chrome exited before result' }));
  const result = await Promise.race([browserResult, new Promise((_, reject) => {
    timeout = setTimeout(() => reject(Error('Browser qualification timeout')), 30000);
  })]);
  if (!result.ok || leakage || result.checks?.length !== nodeChecks.length) throw Error(result.error ?? 'Carrier leakage or incomplete browser tests');
  console.log(`Node ${process.version}: ${nodeChecks.length} qualification groups passed`);
  console.log(`${execFileSync(chrome, ['--version'], { encoding: 'utf8' }).trim()}: ${result.checks.length} qualification groups passed`);
  console.log('Carrier observation: no fixture private marker; synthetic trust, NOT Nitro attestation.');
} finally {
  clearTimeout(timeout);
  await Promise.all([
    stopChild(browser, { group: true }),
    ...[...children].map(child => stopChild(child)),
  ]);
  server.closeAllConnections();
  await new Promise(resolve => server.close(resolve));
  // Exact task-created Chrome profile only, never an existing user's profile.
  await rm(profile, { recursive: true, force: true, maxRetries: 4, retryDelay: 100 });
}
