// Qualification-only policy. Deliberately separate from the shipping workspace.
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';

const root = fileURLToPath(new URL('.', import.meta.url));
export const policy = JSON.parse(readFileSync(new URL('./dependency-policy.json', import.meta.url)));
export function hash(bytes) { return createHash('sha256').update(bytes).digest('hex'); }
export function validate(manifest, lock, metadata) {
  const errors = [];
  if (hash(manifest) !== policy.manifestSha256) errors.push('review changed direct pins/features');
  if (hash(lock) !== policy.lockfileSha256) errors.push('review changed registry checksums/lock graph');
  const members = metadata.workspace_members ?? [];
  const local = metadata.packages?.filter(p => p.source === null) ?? [];
  if (members.length !== 1 || local.length !== 1 || !members.includes(local[0].id)
      || local[0].name !== 'cinder-web-channel-qualification') errors.push('qualification workspace isolation');
  const graph = (metadata.packages ?? []).map(p => {
    if (p.source !== null && p.source !== 'registry+https://github.com/rust-lang/crates.io-index') errors.push('unapproved dependency source');
    const features = metadata.resolve?.nodes?.find(n => n.id === p.id)?.features ?? ['missing'];
    return [`${p.name}@${p.version}`, [...features].sort().join(','), Boolean(p.targets?.some(t => t.kind.includes('custom-build')))];
  }).sort((a, b) => a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0);
  if (JSON.stringify(graph) !== JSON.stringify(policy.packages)) errors.push('review package/feature/build-script graph');
  return errors;
}
export function validateNpm(manifest, lock) {
  const errors = [];
  if (hash(manifest) !== policy.npmManifestSha256) errors.push('review changed npm direct pins');
  if (hash(lock) !== policy.npmLockfileSha256) errors.push('review changed npm resolved packages/integrities');
  const parsed = JSON.parse(lock);
  if (parsed.lockfileVersion !== 3) errors.push('npm lock format');
  for (const [path, p] of Object.entries(parsed.packages ?? {})) {
    if (!path) continue;
    if (!p.resolved?.startsWith('https://registry.npmjs.org/') || !p.integrity?.startsWith('sha512-') || p.link) errors.push('unapproved npm source');
    // esbuild's install script is deliberately NOT run. npm ci --ignore-scripts
    // installs its locked host-platform optional binary, used only for bundling.
    if (p.hasInstallScript && path !== 'node_modules/esbuild') errors.push('unapproved npm lifecycle script');
  }
  return errors;
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--format-version', '1', '--locked', '--offline'], { cwd: root, encoding: 'utf8' }));
  const errors = [
    ...validate(readFileSync(resolve(root, 'Cargo.toml')), readFileSync(resolve(root, 'Cargo.lock')), metadata),
    ...validateNpm(readFileSync(resolve(root, 'package.json')), readFileSync(resolve(root, 'package-lock.json'))),
  ];
  if (errors.length) throw Error(errors.join('; '));
  console.log(`Isolated WASM build graph checked: ${policy.packages.length - 1} registry packages; service compiles the same shared core source.`);
}
