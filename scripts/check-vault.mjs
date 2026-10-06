#!/usr/bin/env node
// Reproducible local SBF boundary check. Never forks remote state, inherits a wallet, or deploys externally.
import assert from 'node:assert/strict';
import { execFileSync, spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { delimiter, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const focused=process.argv.slice(2);
assert(focused.length===0||(focused.length===1&&['--recovery-only','--acceptance-only'].includes(focused[0])),'Only explicit local recovery/acceptance focuses are supported');
const programs = resolve(root, 'programs');
const client = resolve(root, 'clients/vault');
const anchor = process.env.CINDER_ANCHOR_TOOL || 'anchor';
const env = { ...process.env, PATH: dirname(process.execPath) + delimiter + process.env.PATH,
  NO_DNA: '1', CARGO_TARGET_DIR: resolve(programs, 'target') };
assert.equal(process.versions.node, '24.21.0', 'Use the repository Node pin');
function run(command, args, cwd = programs) {
  let output;
  try { output = execFileSync(command, args, { cwd, env, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024, stdio: ['ignore', 'pipe', 'pipe'] }); }
  catch (error) {
    process.stderr.write(error.stdout || ''); process.stderr.write(error.stderr || ''); throw error;
  }
  return output;
}
assert.equal(run(anchor, ['--version']).trim(), 'anchor-cli 1.2.0');
assert.equal(run('surfpool', ['--version']).trim(), 'surfpool 1.5.0');
run(process.execPath,['scripts/prepare-recovery-test.mjs'],root);
assert.match(run('cargo-build-sbf', ['--version']), /solana-cargo-build-sbf 3\.1\.10\b/);
const policy = JSON.parse(readFileSync(resolve(root, 'scripts/vault-dependencies.json'), 'utf8'));
for (const [path, hash] of Object.entries(policy.lockfiles)) {
  assert.equal(createHash('sha256').update(readFileSync(resolve(root, path))).digest('hex'), hash,
    `${path} changed: review the isolated dependency graph and update its policy`);
}
const metadata = JSON.parse(run('cargo', ['metadata', '--no-deps', '--locked', '--offline', '--format-version', '1']));
assert.equal(metadata.packages.length, 1, 'Unexpected Solana workspace package');
assert.equal(metadata.packages[0].name, 'cinder-vault');
assert.deepEqual(metadata.packages[0].dependencies.map(d => [d.name, d.req, d.kind, d.uses_default_features, d.features]), [
  ['anchor-lang', '=1.2.0', null, true, []], ['anchor-spl', '=1.2.0', null, false, ['token', 'token_2022']],
  ['solana-sha256-hasher', '=3.1.0', null, true, ['sha2']],
], 'Review any vault dependency-boundary change');
const manifest = JSON.parse(readFileSync(resolve(client, 'package.json'), 'utf8'));
assert.deepEqual(manifest.dependencies, { '@anchor-lang/core': '1.2.0', '@solana/web3.js': '1.99.0' });
assert.deepEqual(manifest.devDependencies, { '@solana/spl-token': '0.4.15', '@types/node': '24.19.0', typescript: '7.0.2' });
run('cargo', ['fmt', '--all', '--', '--check']);
run('cargo', ['clippy', '--all-targets', '--locked', '--offline', '--', '-D', 'warnings']);
// Cargo-SBF and IDL accept Cargo flags at different nesting levels; build separately.
// Capture stderr too: LLVM can report an unsafe frame while returning exit status zero.
const build = spawn(anchor, ['build', '--ignore-keys', '--no-idl', '--tools-version', 'v1.52', '--arch', 'v0',
  '--', '--', '--locked', '--offline'], { cwd: programs, env, stdio: ['ignore', 'pipe', 'pipe'] });
let buildLog = '';
for (const stream of [build.stdout, build.stderr]) stream.on('data', bytes => { buildLog += bytes; });
await new Promise((accept, reject) => {
  build.on('error', reject); build.on('close', (code, signal) => {
    if (code !== 0 || /Stack (?:offset|frame).*exceeded|exceeding the maximum stack/i.test(buildLog)) {
      process.stderr.write(buildLog); reject(new Error(`Unsafe/failed SBF build: ${code ?? signal}`));
    } else accept();
  });
});
run(anchor, ['idl', 'build', '-o', 'target/idl/cinder_vault.json', '-t', 'target/types/cinder_vault.ts', '--', '--locked', '--offline']);
assert.deepEqual(JSON.parse(readFileSync(resolve(programs, 'target/idl/cinder_vault.json'), 'utf8')),
  JSON.parse(readFileSync(resolve(client, 'idl/cinder_vault.json'), 'utf8')), 'Tracked IDL differs from program source');
assert.equal(readFileSync(resolve(programs, 'target/types/cinder_vault.ts'), 'utf8'),
  readFileSync(resolve(client, 'src/cinder_vault.ts'), 'utf8'), 'Tracked TypeScript IDL differs from program source');
assert.equal(readFileSync(resolve(programs, 'target/types/cinder_vault_errors.ts'), 'utf8'),
  readFileSync(resolve(client, 'src/cinder_vault_errors.ts'), 'utf8'), 'Tracked Anchor error codes differ from program source');
run(resolve(client, 'node_modules/.bin/tsc'), ['--noEmit'], client);

async function unused(port) {
  const server = createServer();
  await new Promise((accept, reject) => { server.once('error', reject); server.listen(port, '127.0.0.1', accept); });
  await new Promise(accept => server.close(accept));
}
await unused(18899); await unused(18900);
const sandbox = mkdtempSync(resolve(tmpdir(), 'cinder-vault-surfpool-'));
const surfpool = spawn('surfpool', ['start', '--offline', '--no-deploy', '--ci', '--host', '127.0.0.1',
  '--port', '18899', '--ws-port', '18900', '--airdrop-amount', '0', '--airdrop-keypair-path', '/dev/null', '--db', ':memory:'],
{ cwd: sandbox, env, stdio: ['ignore', 'pipe', 'pipe'] });
let serverLog = '', serverError;
for (const stream of [surfpool.stdout, surfpool.stderr]) stream.on('data', bytes => { serverLog = (serverLog + bytes).slice(-16_384); });
surfpool.on('error', error => { serverError = error; });
try {
  let ready = false;
  for (let tries = 0; tries < 100; tries++) {
    if (serverError) throw serverError;
    if (surfpool.exitCode !== null || surfpool.signalCode !== null) throw new Error(`Surfpool exited: ${serverLog}`);
    try {
      const response = await fetch('http://127.0.0.1:18899', { method: 'POST', headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'getHealth' }), signal: AbortSignal.timeout(200) });
      const body = await response.json(); if (body.result === 'ok') { ready = true; break; }
    } catch { /* Server startup only; never retry a money-moving operation. */ }
    await delay(100);
  }
  assert(ready, `Surfpool startup timeout: ${serverLog}`);
  // Asynchronous child keeps the parent available to supervise/clean up the sandbox.
  const tests = spawn(process.execPath, ['--test', '--test-concurrency=1',...(focused.length?[`--test-name-pattern=^${focused[0]==='--acceptance-only'?'P22':'P21'} `]:[]), 'tests/funding.test.ts', 'tests/recovery.test.ts', 'tests/vault.test.ts'], { cwd: client, env, stdio: 'inherit' });
  await new Promise((accept, reject) => {
    tests.on('error', reject); tests.on('exit', (code, signal) => code === 0 ? accept() : reject(new Error(`Vault tests failed: ${code ?? signal}`)));
  });
} finally {
  surfpool.kill('SIGTERM');
  for (let i = 0; i < 50 && surfpool.exitCode === null && surfpool.signalCode === null; i++) await delay(100);
  if (surfpool.exitCode === null && surfpool.signalCode === null) surfpool.kill('SIGKILL');
  rmSync(sandbox, { recursive: true, force: true });
}
process.stdout.write(focused.length?`Focused ${focused[0]}: strict checks and joined HTTP/WebSocket local SBF tests passed.\n`:'Vault: locked SBF/IDL, strict Rust/TypeScript checks and offline signed transaction tests passed.\n');
