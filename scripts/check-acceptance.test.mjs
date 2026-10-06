import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { manifest, validate } from "./check-acceptance.mjs";
test("manifest requires declarations, not stale comments, strings or longer identifiers", () => {
  for (const [id, stale] of [
    ["private-commands", "export async function runHttpRenamed() {}"],
    ["private-streams", "export async function runWebsocketRenamed() {}"],
    ["private-commands", "// export async function runHttp() {}"],
    ["private-commands", "/*\nexport async function runHttp() {}\n*/"],
    ["private-commands", 'const stale = "runHttp";'],
    [
      "differential",
      "// property_seeded_joined_ledger_matches_independent_reference_at_every_cut",
    ],
    [
      "normal",
      "// test(`P22 http complete controller lifecycle and process faults`, () => {});",
    ],
    [
      "normal",
      'const stale = "complete controller lifecycle and process faults";',
    ],
    [
      "normal",
      'const stale = "test(`P22 http complete controller lifecycle and process faults`, () => {});";',
    ],
  ]) {
    const c = manifest.cases.find((c) => c.id === id);
    assert(c, `Missing fixture ${id}`);
    const read = (path) =>
      path === c.file
        ? Buffer.from(stale)
        : readFileSync(new URL(`../${path}`, import.meta.url));
    assert(validate(manifest, read).includes(`Missing test ${id}`), stale);
  }
});
test("tracked fixed-seed manifest traces all source families and portable scenarios", () =>
  assert.deepEqual(validate(manifest), []));
test("manifest refuses changed seeds, missing tests, unchecked budgets and hidden research", () => {
  for (const mutate of [
    (m) => (m.seedSha256 = "0".repeat(64)),
    (m) => m.seedCount++,
    (m) => (m.seedFile = "../work/private"),
    (m) => (m.cases[0].file = "work/local.rs"),
    (m) => (m.cases[0].test = "nonexistent-test"),
    (m) => (m.cases[1].id = m.cases[0].id),
    (m) => (m.budgets.readCount = 0),
    (m) => (m.budgets.nodeRssBytes = Infinity),
    (m) => (m.scope = "live"),
    (m) => (m.notQualified = []),
  ]) {
    const m = structuredClone(manifest);
    mutate(m);
    assert(validate(m).length > 0);
  }
});
test("manifest refuses missing requirements, unknown provenance and untracked files", () => {
  const m = structuredClone(manifest);
  for (const c of m.cases)
    c.requirements = c.requirements.filter((id) => id !== "W08");
  assert(validate(m).some((e) => e.includes("W08")));
  m.cases[0].evidence = "hardware";
  assert(validate(m).some((e) => e.includes("evidence")));
  assert(
    validate(manifest, () => {
      throw Error("Unavailable checkout");
    }).some((e) => e.includes("Missing source")),
  );
});
