import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

test('full-width math golden vectors match the independent BigInt generator', () => {
  const generator = fileURLToPath(new URL('./generate-p02-math-vectors.mjs', import.meta.url));
  const actual = execFileSync(process.execPath, [generator], { encoding: 'utf8' });
  const expected = readFileSync(new URL('../crates/kernel/tests/fixtures/mul-div-v1.tsv', import.meta.url), 'utf8');
  assert.equal(actual, expected);
});
