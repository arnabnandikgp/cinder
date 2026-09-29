import test from 'node:test';
import assert from 'node:assert/strict';
import { validateKernelSource, validateWorkspace } from './check-workspace.mjs';

function fixture() {
  const pkg = name => ({ name, id: name, source: null, targets: [{ kind: ['lib'] }], dependencies: [] });
  const packages = ['cinder-kernel', 'cinder-ports', 'cinder-test-support'].map(pkg);
  packages[2].dependencies = [
    { name: 'cinder-ports', kind: null, path: '/repo/crates/ports', req: '=0.1.0', target: null },
    { name: 'cinder-kernel', kind: 'dev', path: '/repo/crates/kernel', req: '=0.1.0', target: null },
  ];
  return { packages, workspace_members: packages.map(p => p.id) };
}

test('the intended local graph passes', () => {
  assert.deepEqual(validateWorkspace(fixture()), []);
  assert.deepEqual(validateKernelSource('#![no_std]\npub trait Transition {}'), []);
});
test('kernel cannot acquire a port or third-party edge unnoticed', () => {
  const metadata = fixture();
  metadata.packages[0].dependencies.push(metadata.packages[2].dependencies[0]);
  assert.match(validateWorkspace(metadata).join('\n'), /forbidden dependency edge/);
});
test('build dependencies and registry sources reject', () => {
  const metadata = fixture();
  metadata.packages[2].dependencies[0].kind = 'build';
  metadata.packages[2].dependencies[0].source = 'registry+https://example.invalid';
  assert.equal(validateWorkspace(metadata).length, 2);
});
test('loose version and target-specific edges reject', () => {
  const metadata = fixture();
  metadata.packages[2].dependencies[0].req = '^0.1';
  metadata.packages[2].dependencies[1].target = 'cfg(unix)';
  assert.equal(validateWorkspace(metadata).length, 2);
});
test('extra workspace members and external packages require review', () => {
  const metadata = fixture();
  metadata.packages.push({ name: 'network-client', id: 'external', source: 'registry', dependencies: [] });
  assert.match(validateWorkspace(metadata).join('\n'), /unexpected package set/);
  assert.match(validateWorkspace(metadata).join('\n'), /external source/);
});
test('custom build scripts reject', () => {
  const metadata = fixture();
  metadata.packages[0].targets.push({ kind: ['custom-build'] });
  assert.match(validateWorkspace(metadata).join('\n'), /build scripts/);
});
test('obvious std escapes and missing boundary reject', () => {
  assert.equal(validateKernelSource('pub trait Transition {}').length, 1);
  assert.equal(validateKernelSource('#![no_std]\nextern crate std;').length, 1);
  assert.equal(validateKernelSource('#![no_std]\nuse std::fs;').length, 1);
});
test('submodules need no root attribute but cannot explicitly escape to std', () => {
  assert.deepEqual(validateKernelSource('use alloc::vec::Vec;', false), []);
  assert.equal(validateKernelSource('use std::net;', false).length, 1);
});
