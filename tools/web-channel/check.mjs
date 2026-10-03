#!/usr/bin/env node
// All preparation is explicit; checks below use locked/offline Cargo only.
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const cwd = fileURLToPath(new URL('.', import.meta.url));
function command(executable, args) { execFileSync(executable, args, { cwd, stdio: 'inherit' }); }
if (process.version !== 'v24.21.0') throw Error('Use Node 24.21.0');
if (!execFileSync('rustc', ['--version'], { encoding: 'utf8' }).startsWith('rustc 1.97.1 ')) throw Error('Use Rust 1.97.1');
const bindgen = process.env.CINDER_WASM_BINDGEN ?? 'wasm-bindgen';
if (execFileSync(bindgen, ['--version'], { encoding: 'utf8' }).trim() !== 'wasm-bindgen 0.2.129') throw Error('Use wasm-bindgen 0.2.129');
command(process.execPath, ['check-dependencies.mjs']);
command(process.execPath, ['--test', 'check-dependencies.test.mjs', 'stop-child.test.mjs']);
command('cargo', ['fmt', '--check']);
command('cargo', ['clippy', '--all-targets', '--locked', '--offline', '--', '-D', 'warnings']);
for (const mode of [[], ['--release']]) command('cargo', ['test', '--locked', '--offline', ...mode]);
command('cargo', ['build', '--bin', 'qualification-responder', '--locked', '--offline']);
command('cargo', ['clippy', '--lib', '--target', 'wasm32-unknown-unknown', '--locked', '--offline', '--', '-D', 'warnings']);
command('cargo', ['build', '--lib', '--release', '--target', 'wasm32-unknown-unknown', '--locked', '--offline']);
command(bindgen, ['target/wasm32-unknown-unknown/release/cinder_web_channel_qualification.wasm', '--target', 'web', '--out-dir', 'pkg', '--out-name', 'channel']);
command(process.execPath, ['check-browser.mjs']);
