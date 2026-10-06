import test from "node:test";
import assert from "node:assert/strict";
import { classify, inventory, ignoredRust } from "./test-inventory.mjs";
import { checked, observer, qualifyEntries } from "./test-runner.mjs";
const files = [
  "scripts/a.test.mjs",
  "services/web-relay/a.test.mjs",
  "clients/private/tests/a.test.ts",
  "tools/web-channel/attestation/a.test.mjs",
  "clients/vault/tests/a.test.ts",
];
test("every source test has one suite; newly added files are automatically assigned", () => {
  assert.deepEqual(
    classify([...files, "clients/private/tests/new.test.ts"]).private,
    ["clients/private/tests/a.test.ts", "clients/private/tests/new.test.ts"],
  );
  assert(inventory().contracts.includes("scripts/test-runner.test.mjs"));
  assert.throws(
    () => classify([...files, "clients/new/a.test.ts"]),
    /Unassigned/,
  );
  assert.throws(() => classify(files.slice(1)), /Empty/);
});
test("declarations, renamed tests, empty runs and filtered wrappers cannot establish execution", () => {
  for (const format of ["rust", "node"]) {
    const audit = observer(format, ["required"]);
    assert(audit.errors().length);
    audit.line("fn required() {}");
    audit.line(
      format === "rust"
        ? "test required_renamed ... ok"
        : "ok 1 - file-wrapper",
    );
    audit.line(
      format === "rust"
        ? "test result: ok. 1 passed; 0 failed; 0 ignored;"
        : "# tests 1",
    );
    assert(
      audit.errors().some((e) => e.includes("Missing executed test: required")),
    );
    audit.line(format === "rust" ? "test required ... ok" : "ok 2 - required");
    assert.deepEqual(audit.errors(), []);
  }
});
test("unknown ignored Rust tests fail; only named hardware/parent workers are exempt", () => {
  const audit = observer("rust", [], ignoredRust);
  audit.line("test good ... ok");
  audit.line("test result: ok. 1 passed; 0 failed; 1 ignored;");
  audit.line("test encrypted_crash_worker ... ignored, parent-invoked");
  assert.deepEqual(audit.errors(), []);
  audit.line("test forgotten_regression ... ignored, later");
  assert.match(audit.errors().join("\n"), /Unexpected ignored/);
});
test("Rust module-qualified names require an exact terminal name", () => {
  const audit = observer("rust", ["required"]);
  audit.line("test module::required_renamed ... ok");
  audit.line("test result: ok. 1 passed; 0 failed; 0 ignored;");
  assert.match(audit.errors().join("\n"), /Missing executed/);
  audit.line("test module::required ... ok");
  assert.deepEqual(audit.errors(), []);
});
test("Node skip, todo, cancellation/failure cannot be counted as success", () => {
  for (const result of [
    "ok 2 - required # SKIP later",
    "ok 2 - required # TODO later",
    "not ok 2 - required",
  ]) {
    const audit = observer("node", ["required"]);
    for (const line of ["ok 1 - good", "# tests 2", result]) audit.line(line);
    assert(audit.errors().length);
    assert(audit.errors().some((e) => e.includes("Missing executed")));
  }
});
test("browser acceptance requires both actually completed shared entrypoints", () => {
  assert.throws(() => qualifyEntries(["runHttp"]), /runWebsocket/);
  assert.doesNotThrow(() => qualifyEntries(["runHttp", "runWebsocket"]));
});
test("process execution rejects nonzero exits, absent required tests and skipped tests", async () => {
  const run = (lines, code = 0) =>
    checked(
      process.execPath,
      [
        "-e",
        `console.log(${JSON.stringify(lines.join("\n"))});process.exitCode=${code}`,
      ],
      { format: "node", required: ["required"] },
    );
  await run(["ok 1 - required", "# tests 1"]);
  await assert.rejects(
    run(["ok 1 - required", "# tests 1"], 1),
    /Test suite failed/,
  );
  await assert.rejects(
    run(["ok 1 - file-wrapper", "# tests 1"]),
    /Missing executed/,
  );
  await assert.rejects(
    run(["ok 1 - required # SKIP later", "# tests 1"]),
    /Unexpected skipped/,
  );
});
