import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { validateNpm } from './check-dependencies.mjs';
const manifest = readFileSync(new URL('./package.json', import.meta.url));
const lock = readFileSync(new URL('./package-lock.json', import.meta.url));
test('exact qualification-only npm pins and registry integrity pass', () => assert.deepEqual(validateNpm(manifest,lock),[]));
test('changed npm direct pin or transitive integrity requires review', () => {
  assert(validateNpm(Buffer.concat([manifest,Buffer.from(' ')]),lock).includes('review changed npm direct pins'));
  const changed = JSON.parse(lock); changed.packages['node_modules/pkijs'].integrity = 'sha512-not-original';
  assert(validateNpm(manifest,JSON.stringify(changed)).includes('review changed npm resolved packages/integrities'));
});
test('alternate npm source and extra lifecycle script refuse', () => {
  const changed = JSON.parse(lock), p = changed.packages['node_modules/pkijs'];
  p.resolved = 'file:unreviewed'; p.hasInstallScript = true;
  const errors = validateNpm(manifest,JSON.stringify(changed));
  assert(errors.includes('unapproved npm source') && errors.includes('unapproved npm lifecycle script'));
});
