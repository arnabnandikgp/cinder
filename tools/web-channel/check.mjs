#!/usr/bin/env node
// All preparation is explicit; checks below use locked/offline Cargo only.
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { checked, nodeSuite } from '../../scripts/test-runner.mjs';
const cwd = fileURLToPath(new URL('.', import.meta.url));
const args = process.argv.slice(2);
if (args.length > 1 || (args.length && !['--standard-only','--packed-only'].includes(args[0]))) {
  throw Error('Usage: check.mjs [--standard-only | --packed-only]');
}
const standard = args[0] !== '--packed-only', packed = args[0] !== '--standard-only';
function command(executable, args) { execFileSync(executable, args, { cwd, stdio: 'inherit' }); }
if (process.version !== 'v24.21.0') throw Error('Use Node 24.21.0');
if (!execFileSync('rustc', ['--version'], { encoding: 'utf8' }).startsWith('rustc 1.97.1 ')) throw Error('Use Rust 1.97.1');
const bindgen = process.env.CINDER_WASM_BINDGEN ?? 'wasm-bindgen';
if (execFileSync(bindgen, ['--version'], { encoding: 'utf8' }).trim() !== 'wasm-bindgen 0.2.129') throw Error('Use wasm-bindgen 0.2.129');
command(process.execPath, ['check-dependencies.mjs']);
// Discovered certificate tests invoke the independent native verifier. Build
// prerequisites before discovery execution; never depend on a warm host target.
execFileSync('cargo', ['build', '-p', 'cinder-service', '--features', 'local-fixture', '--bin', 'cinder-verify-fixture', '--bin', 'cinder-service-fixture', '--locked', '--offline'],
  { cwd: fileURLToPath(new URL('../..', import.meta.url)), stdio:'inherit' });
if (standard) {
  await nodeSuite('web');
  await nodeSuite('relay');
  command('cargo', ['fmt', '--check']);
  command('cargo', ['clippy', '--all-targets', '--locked', '--offline', '--', '-D', 'warnings']);
  for (const mode of [[], ['--release']]) await checked('cargo', ['test', '--locked', '--offline', ...mode], {cwd, format:'rust'});
  command('cargo', ['build', '--bin', 'qualification-responder', '--locked', '--offline']);
  command('cargo', ['clippy', '--lib', '--target', 'wasm32-unknown-unknown', '--locked', '--offline', '--', '-D', 'warnings']);
}
command('cargo', ['build', '--lib', '--release', '--target', 'wasm32-unknown-unknown', '--locked', '--offline']);
command(bindgen, ['target/wasm32-unknown-unknown/release/cinder_web_channel_qualification.wasm', '--target', 'web', '--out-dir', 'pkg', '--out-name', 'channel']);
command(process.execPath, ['attestation/build.mjs']);
if (standard) {
  command(process.execPath, ['build-sdk.mjs']);
  command(process.execPath, ['check-browser.mjs']);
  command(process.execPath, ['check-http.mjs']);
  command(process.execPath, ['check-runtime.mjs']);
}
if (packed) command(process.execPath, ['check-runtime.mjs', '--packed-only']);
