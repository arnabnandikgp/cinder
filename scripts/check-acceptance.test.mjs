import test from "node:test";
import assert from "node:assert/strict";
import { manifest, validate } from "./check-acceptance.mjs";
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
