import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = path => readFileSync(new URL('../' + path, import.meta.url), 'utf8');

test('ARM64 qualification pins signed dated Debian sources without changing package versions', () => {
  const sources = read('tools/nitro-linux/debian.sources');
  const blocks = sources.trim().split(/\n\s*\n/).map(block => Object.fromEntries(
    block.split('\n').map(line => {
      const colon = line.indexOf(':');
      assert(colon > 0);
      return [line.slice(0, colon), line.slice(colon + 1).trim()];
    }),
  ));
  assert.deepEqual(blocks, [
    { Types: 'deb', URIs: 'https://snapshot.debian.org/archive/debian/20261022T000000Z/', Suites: 'bookworm bookworm-updates', Components: 'main', 'Signed-By': '/usr/share/keyrings/debian-archive-keyring.gpg', 'Check-Valid-Until': 'no' },
    { Types: 'deb', URIs: 'https://snapshot.debian.org/archive/debian-security/20261022T000000Z/', Suites: 'bookworm-security', Components: 'main', 'Signed-By': '/usr/share/keyrings/debian-archive-keyring.gpg', 'Check-Valid-Until': 'no' },
  ]);
  const recipe = read('tools/nitro-linux/Containerfile');
  assert.match(recipe, /^COPY debian\.sources \/etc\/apt\/sources\.list\.d\/debian\.sources$/m);
  assert.match(recipe, /libssl-dev=3\.0\.22-1~deb12u1 pkg-config=1\.8\.1-1/);
  assert.doesNotMatch(recipe + sources, /allow-unauthenticated|trusted\s*[:=]\s*(?:yes|true)|AllowInsecureRepositories|Verify-(?:Peer|Host)\s*[:=]\s*false/i);
});

test('ARM64 intermediate cache includes the dated repository input and keeps package execution offline', () => {
  const workflow = read('.github/workflows/nitro-package.yml');
  const key = workflow.split('\n').find(line => line.includes('key: arm64-package-'));
  assert(key);
  for (const path of ['tools/nitro-linux/Containerfile', 'tools/nitro-linux/debian.sources', 'tools/nitro-linux/cargo-config.toml', 'tools/nitro-runtime/package-check.sh']) assert(key.includes("'" + path + "'"));
  assert.match(workflow, /docker run --rm --network none --platform linux\/arm64/);
});
