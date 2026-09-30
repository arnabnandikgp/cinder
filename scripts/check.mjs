#!/usr/bin/env node
// Offline entry point after toolchain installation and locked dependency fetch.
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
if (args.length > 1 || (args.length === 1 && args[0] !== '--properties')) throw new Error('Usage: node scripts/check.mjs [--properties]');
const expectedNode = readFileSync(resolve(root, '.node-version'), 'utf8').trim();
if (process.versions.node !== expectedNode) throw new Error(`Use pinned Node ${expectedNode}; got ${process.versions.node}`);
const expectedRust = readFileSync(resolve(root, 'rust-toolchain.toml'), 'utf8').match(/^channel = "([\d.]+)"$/m)?.[1];
const rust = execFileSync('rustc', ['--version'], { cwd: root, encoding: 'utf8' }).trim();
if (!expectedRust || !rust.startsWith(`rustc ${expectedRust} `)) throw new Error(`Rust toolchain mismatch: ${rust}`);

function run(command, arguments_) {
  process.stdout.write(`check: ${command === process.execPath ? 'node' : command} ${arguments_.join(' ')}\n`);
  execFileSync(command, arguments_, { cwd: root, stdio: 'inherit' });
}

if (args[0] === '--properties') {
  run('cargo', ['test', '--workspace', '--locked', '--offline', 'property_']);
} else {
  run(process.execPath, ['scripts/check-implementation-plan.mjs']);
  run(process.execPath, ['--test', 'scripts/check-implementation-plan.test.mjs', 'scripts/check-workspace.test.mjs', 'scripts/check-financial-fixtures.test.mjs']);
  run(process.execPath, ['scripts/check-workspace.mjs']);
  run('cargo', ['fmt', '--all', '--', '--check']);
  run('cargo', ['clippy', '--workspace', '--all-targets', '--locked', '--offline', '--', '-D', 'warnings']);
  run('cargo', ['clippy', '--workspace', '--all-targets', '--all-features', '--locked', '--offline', '--', '-D', 'warnings']);
  run('cargo', ['build', '--workspace', '--all-targets', '--all-features', '--locked', '--offline']);
  run('cargo', ['test', '--workspace', '--all-features', '--locked', '--offline']);
  run('cargo', ['test', '--workspace', '--all-features', '--release', '--locked', '--offline']);
}
