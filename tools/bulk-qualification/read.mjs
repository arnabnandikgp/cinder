#!/usr/bin/env node
// Explicit current approval + private fresh evidence directory. No automatic run.
import { readFileSync, mkdirSync, lstatSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { canonical, sha, privateRead, exclusive } from '../native-qualification/core.mjs';
import { manifest, commitment, qualify } from './metadata.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const source = sha(['tools/bulk-qualification/read.mjs', 'tools/bulk-qualification/metadata.mjs', 'tools/native-qualification/core.mjs']
  .map(p => `${p}\0${sha(readFileSync(join(root, p)))}`).join('\n'));
const [mode, path, ...extra] = process.argv.slice(2);
if (!['--prepare', '--run-approved'].includes(mode) || !path || extra.length || resolve(path) !== path)
  throw Error('Usage: read.mjs --prepare|--run-approved ABSOLUTE_PRIVATE_DIRECTORY');
if (process.versions.node !== readFileSync(join(root, '.node-version'), 'utf8').trim()) throw Error('Use pinned Node');
if (process.env.NODE_TLS_REJECT_UNAUTHORIZED === '0') throw Error('TLS verification required');
if (mode === '--prepare') {
  mkdirSync(path, { mode: 0o700 });
  const m = manifest(source); exclusive(join(path, 'manifest.json'), m);
  console.log(JSON.stringify({ directory: path, commitment: commitment(m), execution_approval: 'pending', network_requests: 0 }));
} else {
  const st = lstatSync(path);
  if (!st.isDirectory() || st.isSymbolicLink() || st.uid !== process.getuid() || st.mode & 0o077) throw Error('Private evidence directory required');
  const m = JSON.parse(privateRead(join(path, 'manifest.json'))), a = JSON.parse(privateRead(join(path, 'approval.json')));
  if (canonical(m) !== canonical(manifest(source))) throw Error('Changed runner/source scope');
  const result = await qualify(m, a, { now: Date.now, fetch: globalThis.fetch,
    retain: (name, value) => exclusive(join(path, `${name}.json`), value) });
  console.log(JSON.stringify({ directory: path, ...result }));
  if (result.outcome !== 'metadata-observed') process.exitCode = 2;
}
