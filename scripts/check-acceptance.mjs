#!/usr/bin/env node
// Tracked offline manifest and consolidated runner. No commands from JSON, live
// URLs, research tree, credentials or environment-dependent funding sources.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
export const manifest = JSON.parse(
  readFileSync(
    resolve(root, "docs/implementation/acceptance-manifest.json"),
    "utf8",
  ),
);
const required = [
  ...Array.from({ length: 9 }, (_, i) => `W${String(i + 1).padStart(2, "0")}`),
  ...Array.from({ length: 10 }, (_, i) => `V${String(i + 1).padStart(2, "0")}`),
];
// Trace the declaration forms used by this bounded manifest, not arbitrary text.
// This is a source-presence guard; execution remains the runner's responsibility.
function declaresTest(file, name, text) {
  const source = text.replace(/\/\*[\s\S]*?\*\/|^[ \t]*\/\/[^\n]*/gm, "");
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  if (file.endsWith(".rs"))
    return new RegExp(`^\\s*#\\[test\\]\\s*fn\\s+${escaped}\\s*\\(`, "m").test(
      source,
    );
  if (/^[A-Za-z_]\w*$/.test(name))
    return new RegExp(
      `^[ \\t]*(?:export\\s+)?(?:async\\s+)?function\\s+${escaped}\\s*\\(`,
      "m",
    ).test(source);
  // Node test names here are suffixes of a carrier-specific template literal.
  return new RegExp(
    "^[ \\t]*(?:for\\s*\\([^\\n]*\\)\\s*)?test\\(\\s*`[^`\\n]* " +
      escaped +
      "`\\s*,",
    "m",
  ).test(source);
}
export function validate(
  m,
  read = (path) => readFileSync(resolve(root, path)),
) {
  const errors = [];
  if (m.schema !== 1 || m.scope !== "offline")
    errors.push("Only schema 1 offline evidence is accepted");
  const safe = (path) =>
    typeof path === "string" &&
    /^[a-zA-Z0-9_./-]+$/.test(path) &&
    !path
      .split("/")
      .some((p) => !p || p === ".." || p === "work" || p === "stays");
  if (!safe(m.seedFile)) errors.push("Unsafe seed path");
  else
    try {
      const bytes = read(m.seedFile),
        seeds = bytes.toString().trim().split("\n");
      if (createHash("sha256").update(bytes).digest("hex") !== m.seedSha256)
        errors.push("Seed digest changed");
      if (
        seeds.length !== m.seedCount ||
        new Set(seeds).size !== seeds.length ||
        !seeds.every((s) => /^[0-9a-f]{16}$/.test(s))
      )
        errors.push("Seed cardinality/encoding changed");
    } catch {
      errors.push("Missing seed file");
    }
  for (const name of [
    "workspaceSeconds",
    "browserSeconds",
    "vaultSeconds",
    "readCount",
    "readMilliseconds",
    "nodeRssBytes",
    "journalBytes",
  ]) {
    if (!Number.isSafeInteger(m.budgets?.[name]) || m.budgets[name] <= 0)
      errors.push(`Invalid budget ${name}`);
  }
  if (
    !m.bounds ||
    Object.values(m.bounds).some((n) => !Number.isSafeInteger(n) || n <= 0)
  )
    errors.push("Invalid exploration bounds");
  const seen = new Set(),
    coverage = new Set();
  if (!Array.isArray(m.cases) || !m.cases.length)
    errors.push("No acceptance cases");
  for (const c of Array.isArray(m.cases) ? m.cases : []) {
    if (typeof c.id !== "string" || !c.id || seen.has(c.id))
      errors.push("Missing/duplicate case identity");
    seen.add(c.id);
    if (
      !["workspace", "browser", "vault"].includes(c.runner) ||
      !["synthetic", "local-sbf"].includes(c.evidence)
    )
      errors.push(`Invalid evidence/runner ${c.id}`);
    if (!safe(c.file) || typeof c.test !== "string" || !c.test)
      errors.push(`Unsafe/missing test ${c.id}`);
    else
      try {
        if (!declaresTest(c.file, c.test, read(c.file).toString()))
          errors.push(`Missing test ${c.id}`);
      } catch {
        errors.push(`Missing source ${c.id}`);
      }
    if (
      !Array.isArray(c.requirements) ||
      !c.requirements.length ||
      c.requirements.some((id) => !required.includes(id))
    )
      errors.push(`Invalid requirements ${c.id}`);
    else for (const id of c.requirements) coverage.add(id);
  }
  for (const id of required)
    if (!coverage.has(id)) errors.push(`Missing trace ${id}`);
  if (!Array.isArray(m.notQualified) || !m.notQualified.length)
    errors.push("Missing evidence limits");
  return errors;
}
if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const args = process.argv.slice(2);
  if (args.length > 1 || (args.length && args[0] !== "--manifest-only"))
    throw Error("Usage: node scripts/check-acceptance.mjs [--manifest-only]");
  const errors = validate(manifest);
  if (errors.length) throw Error(errors.join("\n"));
  console.log(
    `P22 manifest: ${manifest.cases.length} named cases, W01–W09/V01–V10, ${manifest.seedCount} fixed seeds`,
  );
  if (!args.length) {
    const results = [];
    for (const [runner, budget] of [
      ["scripts/check.mjs", "workspaceSeconds"],
      ["tools/web-channel/check.mjs", "browserSeconds"],
      ["scripts/check-vault.mjs", "vaultSeconds"],
    ]) {
      const start = performance.now();
      execFileSync(process.execPath, [runner], { cwd: root, stdio: "inherit" });
      const milliseconds = Math.ceil(performance.now() - start);
      results.push({
        runner,
        milliseconds,
        budgetSeconds: manifest.budgets[budget],
      });
      if (milliseconds > manifest.budgets[budget] * 1000)
        throw Error(`Offline resource budget exceeded: ${runner}`);
    }
    console.log(
      "P22 complete offline acceptance timings:",
      JSON.stringify(results),
    );
  }
}
