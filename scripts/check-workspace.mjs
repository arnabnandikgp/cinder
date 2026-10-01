#!/usr/bin/env node
// Dependency/layout guard, not a proof that arbitrary future source is pure.
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const dependencyPolicy = JSON.parse(readFileSync(new URL('./dependency-policy.json', import.meta.url), 'utf8'));
export function validateLockfile(bytes) {
  return createHash('sha256').update(bytes).digest('hex') === dependencyPolicy.lockfileSha256
    ? [] : ['lockfile changed; review registry checksums/graph and update dependency-policy.json'];
}

const allowed = new Map([
  ['cinder-kernel', []],
  ['cinder-ports', []],
  ['cinder-test-support', ['cinder-ports:normal', 'cinder-kernel:dev']],
  ['cinder-journal', ['cinder-kernel:normal', 'rusqlite:normal', 'sha2:normal', 'chacha20poly1305:normal', 'zeroize:normal']],
  ['cinder-pacifica', ['cinder-kernel:normal', 'cinder-journal:normal', 'serde:normal', 'serde_json:normal', 'sha2:normal', 'ed25519-dalek:normal', 'bs58:normal', 'zeroize:normal']],
  ['cinder-api', ['cinder-kernel:normal', 'cinder-journal:normal', 'sha2:normal', 'ed25519-dalek:normal', 'zeroize:normal']],
]);

export function validateWorkspace(metadata) {
  const errors = [];
  const packages = metadata.packages ?? [];
  const approved = new Map(dependencyPolicy.packages.map(p => [`${p.name}@${p.version}`, p]));
  const identities = new Set(packages.map(p => `${p.name}@${p.version}`));
  if (identities.size !== packages.length || packages.length !== allowed.size + approved.size || [...allowed.keys()].some(name => !identities.has(`${name}@0.1.0`)) || [...approved.keys()].some(key => !identities.has(key))) errors.push('unexpected package set; review and update the boundary contract');
  const members = new Set(metadata.workspace_members ?? []);
  if (members.size !== allowed.size) errors.push('unexpected workspace member set');
  for (const pkg of packages) {
    if (!allowed.has(pkg.name)) {
      const expected = approved.get(`${pkg.name}@${pkg.version}`);
      if (!expected || pkg.source !== dependencyPolicy.registry || pkg.version !== expected.version || members.has(pkg.id)) errors.push(`${pkg.name}: external source/version not approved`);
      if (expected) {
        if (Boolean(pkg.targets?.some(t => t.kind.includes('custom-build'))) !== expected.buildScript) errors.push(`${pkg.name}: unapproved external build script`);
        const features = metadata.resolve?.nodes?.find(n => n.id === pkg.id)?.features;
        if (!features || JSON.stringify([...features].sort()) !== JSON.stringify([...expected.features].sort())) errors.push(`${pkg.name}: unapproved resolved features`);
      }
      continue;
    }
    if (!members.has(pkg.id)) errors.push(`${pkg.name}: package outside workspace`);
    if (pkg.source != null || pkg.version !== '0.1.0') errors.push(`${pkg.name}: expected local exact package`);
    if (pkg.targets?.some(target => target.kind.includes('custom-build'))) errors.push(`${pkg.name}: build scripts are not approved`);
    const actual = (pkg.dependencies ?? []).map(dep => `${dep.name}:${dep.kind ?? 'normal'}`).sort();
    const expected = [...(allowed.get(pkg.name) ?? [])].sort();
    if (JSON.stringify(actual) !== JSON.stringify(expected)) errors.push(`${pkg.name}: forbidden dependency edge`);
    for (const dep of pkg.dependencies ?? []) {
      if (['cinder-pacifica', 'cinder-api'].includes(pkg.name) && ['serde', 'serde_json', 'sha2', 'ed25519-dalek', 'bs58', 'zeroize'].includes(dep.name)) {
        const expected = dependencyPolicy.packages.find(p => p.name === dep.name && dep.req === `=${p.version}`);
        const features = dep.name === 'serde' ? ['derive'] : dep.name === 'ed25519-dalek' ? ['std', 'fast', 'zeroize'] : [];
        if (!expected || dep.source !== dependencyPolicy.registry || dep.path || dep.target != null || dep.optional || dep.uses_default_features !== (dep.name !== 'ed25519-dalek') || JSON.stringify(dep.features) !== JSON.stringify(features)) errors.push(`${pkg.name}: adapter dependency configuration not approved`);
        continue;
      }
      if (pkg.name === 'cinder-journal' && ['rusqlite', 'sha2', 'chacha20poly1305', 'zeroize'].includes(dep.name)) {
        const expected = dependencyPolicy.packages.find(p => p.name === dep.name && dep.req === `=${p.version}`);
        const features = dep.name === 'rusqlite' ? ['bundled'] : [];
        if (!expected || dep.source !== dependencyPolicy.registry || dep.path || dep.target != null || dep.optional || dep.uses_default_features !== (dep.name !== 'rusqlite') || JSON.stringify(dep.features) !== JSON.stringify(features)) errors.push(`${pkg.name}: storage dependency configuration not approved`);
        continue;
      }
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
  const errors = [...validateWorkspace(metadata), ...validateLockfile(readFileSync(resolve(root, 'Cargo.lock'))), ...sources(sourceRoot).flatMap(path =>
    validateKernelSource(readFileSync(path, 'utf8'), path === resolve(sourceRoot, 'lib.rs'))
      .map(error => `${path}: ${error}`))];
  if (errors.length) {
    for (const error of errors) process.stderr.write(`${error}\n`);
    process.exitCode = 1;
  } else {
    process.stdout.write('Workspace boundaries OK: 6 local packages; kernel remains dependency-free; pinned storage/crypto/JSON dependencies only.\n');
  }
}
