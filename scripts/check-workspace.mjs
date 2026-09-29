#!/usr/bin/env node
// Dependency/layout guard, not a proof that arbitrary future source is pure.
import { execFileSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const allowed = new Map([
  ['cinder-kernel', []],
  ['cinder-ports', []],
  ['cinder-test-support', ['cinder-ports:normal', 'cinder-kernel:dev']],
]);

export function validateWorkspace(metadata) {
  const errors = [];
  const packages = metadata.packages ?? [];
  const names = new Set(packages.map(p => p.name));
  if (packages.length !== allowed.size || [...allowed.keys()].some(name => !names.has(name))) errors.push('unexpected package set; review and update the boundary contract');
  const members = new Set(metadata.workspace_members ?? []);
  for (const pkg of packages) {
    if (!members.has(pkg.id)) errors.push(`${pkg.name}: package outside workspace`);
    if (pkg.source != null) errors.push(`${pkg.name}: external source not approved in P01`);
    if (pkg.targets?.some(target => target.kind.includes('custom-build'))) errors.push(`${pkg.name}: build scripts are not approved`);
    const actual = (pkg.dependencies ?? []).map(dep => `${dep.name}:${dep.kind ?? 'normal'}`).sort();
    const expected = [...(allowed.get(pkg.name) ?? [])].sort();
    if (JSON.stringify(actual) !== JSON.stringify(expected)) errors.push(`${pkg.name}: forbidden dependency edge`);
    for (const dep of pkg.dependencies ?? []) {
      if (dep.source != null || !dep.path || dep.req !== '=0.1.0' || dep.target != null) errors.push(`${pkg.name}: dependency must be an unconditional exact local pin`);
    }
  }
  return errors;
}

export function validateKernelSource(source, isRoot = true) {
  const errors = [];
  if (isRoot && !/^#!\[no_std\]$/m.test(source)) errors.push('kernel: no_std boundary missing');
  if (/\bextern\s+crate\s+std\b|\bstd\s*::/.test(source)) errors.push('kernel: explicit std escape requires architecture review');
  return errors;
}

const script = fileURLToPath(import.meta.url);
if (process.argv[1] && resolve(process.argv[1]) === script) {
  const root = resolve(dirname(script), '..');
  const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--format-version', '1', '--locked', '--offline'], { cwd: root, encoding: 'utf8' }));
  const sourceRoot = resolve(root, 'crates/kernel/src');
  function sources(directory) {
    return readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
      const path = resolve(directory, entry.name);
      return entry.isDirectory() ? sources(path) : entry.name.endsWith('.rs') ? [path] : [];
    });
  }
  const errors = [...validateWorkspace(metadata), ...sources(sourceRoot).flatMap(path =>
    validateKernelSource(readFileSync(path, 'utf8'), path === resolve(sourceRoot, 'lib.rs'))
      .map(error => `${path}: ${error}`))];
  if (errors.length) {
    for (const error of errors) process.stderr.write(`${error}\n`);
    process.exitCode = 1;
  } else {
    process.stdout.write('Workspace boundaries OK: 3 local packages; no third-party dependencies.\n');
  }
}
