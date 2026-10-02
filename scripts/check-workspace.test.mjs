import test from 'node:test';
import assert from 'node:assert/strict';
import { dependencyPolicy, validateKernelSource, validateLockfile, validateWorkspace } from './check-workspace.mjs';
import { readFileSync } from 'node:fs';

function fixture() {
  const pkg = name => ({ name, id: name, version: '0.1.0', source: null, targets: [{ kind: ['lib'] }], dependencies: [] });
  const packages = ['cinder-kernel', 'cinder-ports', 'cinder-test-support', 'cinder-journal', 'cinder-pacifica', 'cinder-api', 'cinder-service'].map(pkg);
  packages[2].dependencies = [
    { name: 'cinder-ports', kind: null, path: '/repo/crates/ports', req: '=0.1.0', target: null },
    { name: 'cinder-kernel', kind: 'dev', path: '/repo/crates/kernel', req: '=0.1.0', target: null },
  ];
  packages[3].dependencies = [
    { ...packages[2].dependencies[1], kind: null },
    ...['rusqlite', 'sha2', 'chacha20poly1305', 'zeroize'].map(name => ({name, kind: null, source: dependencyPolicy.registry, req: `=${dependencyPolicy.packages.find(p => p.name === name).version}`, target: null, features: name === 'rusqlite' ? ['bundled'] : [], uses_default_features: name !== 'rusqlite', optional: false})),
  ];
  packages[4].dependencies = [
    {...packages[2].dependencies[1], kind: null},
    {name: 'cinder-journal', kind: null, path: '/repo/crates/journal', req: '=0.1.0', target: null},
    ...['serde', 'serde_json', 'sha2', 'ed25519-dalek', 'bs58', 'zeroize'].map(name => ({name, kind: null, source: dependencyPolicy.registry, req: `=${dependencyPolicy.packages.find(p => p.name === name).version}`, target: null, features: name === 'serde' ? ['derive'] : name === 'ed25519-dalek' ? ['std', 'fast', 'zeroize'] : [], uses_default_features: name !== 'ed25519-dalek', optional: false})),
  ];
  packages[5].dependencies = packages[4].dependencies.filter(dep => ['cinder-kernel', 'cinder-journal', 'sha2', 'ed25519-dalek', 'zeroize'].includes(dep.name));
  packages[6].dependencies = [
    ...packages[5].dependencies.filter(dep => ['cinder-kernel', 'cinder-journal'].includes(dep.name)),
    {name: 'cinder-api',kind:null,path:'/repo/crates/api',req:'=0.1.0',target:null},
    {name: 'cinder-pacifica',kind:null,path:'/repo/crates/pacifica',req:'=0.1.0',target:null},
    ...['openssl', 'aws-nitro-enclaves-cose', 'aws-nitro-enclaves-nsm-api', 'serde_cbor', 'zeroize'].map(name => ({name,kind:null,source:dependencyPolicy.registry,req:`=${dependencyPolicy.packages.find(p => p.name === name).version}`,target:null,features:[],uses_default_features:true,optional:false})),
    {name:'socket2',kind:null,source:dependencyPolicy.registry,req:'=0.6.5',target:null,features:['all'],uses_default_features:false,optional:false},
  ];
  packages[6].features={default:[], 'local-fixture':[]};
  packages[6].targets.push(...['cinder-service-fixture','cinder-verify-fixture'].map(name => ({name,kind:['bin'],'required-features':['local-fixture']})));
  const workspace_members = packages.map(p => p.id);
  packages.push(...dependencyPolicy.packages.map(p => ({...pkg(p.name), id: `${p.name}@${p.version}`, version: p.version, source: dependencyPolicy.registry, targets: [{kind: ['lib']}, ...(p.buildScript ? [{kind: ['custom-build']}] : [])]})));
  return { packages, workspace_members, resolve: {nodes: dependencyPolicy.packages.map(p => ({id: `${p.name}@${p.version}`, features: p.features}))} };
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
test('storage pins, features and external build scripts cannot expand silently', () => {
  const metadata = fixture();
  metadata.packages[3].dependencies[1].uses_default_features = true;
  assert.match(validateWorkspace(metadata).join('\n'), /configuration not approved/);
  metadata.resolve.nodes.find(n => n.id.startsWith('rusqlite@')).features = ['bundled', 'load_extension'];
  assert.match(validateWorkspace(metadata).join('\n'), /unapproved resolved features/);
  metadata.packages.find(p => p.name === 'sha2').targets.push({kind: ['custom-build']});
  assert.match(validateWorkspace(metadata).join('\n'), /unapproved external build script/);
});
test('registry lock checksums and edges are pinned by an explicit policy digest', () => {
  const lock = readFileSync(new URL('../Cargo.lock', import.meta.url));
  assert.deepEqual(validateLockfile(lock), []);
  assert.equal(validateLockfile(Buffer.concat([lock, Buffer.from('\n')])).length, 1);
});

test('multiple pinned versions keep distinct graph identities and features', () => {
  const metadata = fixture();
  const syn = metadata.packages.filter(p => p.name === 'syn');
  assert.ok(syn.length >= 2);
  assert.deepEqual(validateWorkspace(metadata), []);
  const index = metadata.packages.indexOf(syn[0]);
  metadata.packages[index] = {...syn[1]};
  assert.match(validateWorkspace(metadata).join('\n'), /unexpected package set/);
});

test('fixture trust roots and key providers cannot silently become default entrypoints', () => {
  const metadata=fixture();metadata.packages[6].features.default=['local-fixture'];
  assert.match(validateWorkspace(metadata).join('\n'),/fixture feature/);
  metadata.packages[6].features.default=[];delete metadata.packages[6].targets[1]['required-features'];
  assert.match(validateWorkspace(metadata).join('\n'),/fixture binary/);
});

test('NSM driver features, pin and platform cannot silently select another provider', () => {
  for (const change of [dep => { dep.req = '^0.5'; }, dep => { dep.features = ['test-hooks']; }, dep => { dep.uses_default_features = false; }, dep => { dep.target = 'cfg(unix)'; }]) {
    const metadata = fixture();
    change(metadata.packages[6].dependencies.find(dep => dep.name === 'aws-nitro-enclaves-nsm-api'));
    assert.match(validateWorkspace(metadata).join('\n'), /transport dependency configuration not approved/);
  }
});

test('socket ownership wrapper has an exact pin, explicit feature and unconditional edge', () => {
  for (const change of [dep => { dep.req = '^0.6'; }, dep => { dep.features = []; }, dep => { dep.uses_default_features = true; }, dep => { dep.target = 'cfg(target_os="linux")'; }, dep => { dep.optional = true; }]) {
    const metadata = fixture();
    change(metadata.packages[6].dependencies.find(dep => dep.name === 'socket2'));
    assert.match(validateWorkspace(metadata).join('\n'), /socket dependency configuration not approved/);
  }
});
