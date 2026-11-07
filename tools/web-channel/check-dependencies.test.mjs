import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { validate } from './check-dependencies.mjs';

const cwd = new URL('.', import.meta.url);
const manifest = readFileSync(new URL('Cargo.toml', cwd));
const lock = readFileSync(new URL('Cargo.lock', cwd));
const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--format-version', '1', '--locked', '--offline'], { cwd, encoding: 'utf8' }));
test('exact isolated graph passes', () => assert.deepEqual(validate(manifest, lock, metadata), []));
test('manifest and registry checksum changes require review', () => {
  assert.match(validate(Buffer.concat([manifest, Buffer.from('\n')]), lock, metadata).join(), /direct pins/);
  assert.match(validate(manifest, Buffer.concat([lock, Buffer.from('\n')]), metadata).join(), /registry checksums/);
});
test('resolver features, build scripts and source changes require review', () => {
  for (const mutate of [
    m => m.resolve.nodes[0].features.push('unreviewed'),
    m => m.packages[0].targets.push({ kind: ['custom-build'] }),
    m => { m.packages[0].source = 'git+https://unreviewed.invalid'; },
  ]) {
    const changed = structuredClone(metadata); mutate(changed);
    assert.notDeepEqual(validate(manifest, lock, changed), []);
  }
});
test('another local member or dependency breaks isolation', () => {
  const changed = structuredClone(metadata);
  changed.packages.push({ id: 'local-escape', name: 'cinder-api', version: '0.1.0', source: null });
  changed.workspace_members.push('local-escape');
  assert.match(validate(manifest, lock, changed).join(), /isolation/);
});
