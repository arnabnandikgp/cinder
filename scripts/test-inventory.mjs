// Approved source roots only: never discover tests in ignored research or caches.
import { readdirSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

export const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const suites = {
  contracts: /^scripts\/.+\.test\.mjs$/,
  relay: /^services\/web-relay\/.+\.test\.mjs$/,
  private: /^clients\/private\/tests\/.+\.test\.ts$/,
  web: /^tools\/web-channel\/.+\.test\.mjs$/,
  vault: /^clients\/vault\/tests\/.+\.test\.ts$/,
};
export function classify(files) {
  const result = Object.fromEntries(Object.keys(suites).map((k) => [k, []]));
  for (const file of [...files].sort()) {
    const owners = Object.entries(suites).filter(([, pattern]) =>
      pattern.test(file),
    );
    if (owners.length !== 1) throw Error(`Unassigned test file: ${file}`);
    result[owners[0][0]].push(file);
  }
  for (const [suite, files] of Object.entries(result))
    if (!files.length) throw Error(`Empty test suite: ${suite}`);
  return result;
}
export function inventory() {
  const files = [];
  function walk(path) {
    for (const entry of readdirSync(resolve(root, path), {
      withFileTypes: true,
    })) {
      if (["node_modules", "target", "pkg"].includes(entry.name)) continue;
      const file = `${path}/${entry.name}`;
      if (entry.isSymbolicLink())
        throw Error(`Source symlink requires review: ${file}`);
      if (entry.isDirectory()) walk(file);
      else if (/\.test\.[^/]+$/.test(entry.name)) files.push(file);
    }
  }
  for (const path of [
    "scripts",
    "services",
    "clients",
    "tools",
    "crates",
    "programs",
  ])
    walk(path);
  return classify(files);
}
// Exact intentional skips. These are never enabled by routine CI.
export const ignoredRust = new Set([
  "encrypted_crash_worker",
  "crash_child_worker",
  "funding_post_child",
  ...[
    "hardware_recipient_rejection_matrix",
    "hardware_competing_witness_writers",
    "hardware_journal_orphan_stale_and_uncertain_commit",
    "hardware_upstream_tls_rejection_matrix",
    "hardware_security_batch",
    "hardware_witness_race_receipt",
    "hardware_unapproved_measurement_receipt",
  ].map((name) => `boot::hardware::${name}`),
]);
