// Stream ordinary logs, but require successful execution rather than declarations.
import { spawn } from "node:child_process";
import { accessSync, constants, statSync } from "node:fs";
import { createInterface } from "node:readline";
import { resolve } from "node:path";
import { manifest } from "./check-acceptance.mjs";
import { root, inventory, ignoredRust } from "./test-inventory.mjs";

// All four are used by the independent private SDK process suite. In particular,
// a missing production verifier must not masquerade as a rejected fixture quote.
export const privateClientBinaries = Object.freeze([
  "cinder-service-fixture",
  "cinder-verify-fixture",
  "cinder-relay",
  "cinder-verify-quote",
]);
export function requirePrivateClientBinaries(
  target = process.env.CARGO_TARGET_DIR ?? "target",
) {
  for (const name of privateClientBinaries) {
    const path = resolve(root, target, "debug", name);
    try {
      if (!statSync(path).isFile()) throw Error("Not a file");
      accessSync(path, constants.X_OK);
    } catch (cause) {
      throw Error(
        `Missing/unusable private-client prerequisite: ${name}. Run node scripts/check.mjs --group=client.`,
        { cause },
      );
    }
  }
}

export function observer(format, required = [], allowedIgnored = new Set()) {
  const passed = new Set(),
    errors = [];
  let summaries = 0;
  return {
    line(line) {
      if (format === "rust") {
        const m = /^test (.+?) \.\.\. (ok|FAILED|ignored)(?:,.*)?$/.exec(line);
        if (m?.[2] === "ok") passed.add(m[1]);
        if (m?.[2] === "FAILED") errors.push(`Failed test: ${m[1]}`);
        if (m?.[2] === "ignored" && !allowedIgnored.has(m[1]))
          errors.push(`Unexpected ignored test: ${m[1]}`);
        if (/^test result: ok\. \d+ passed;/.test(line)) summaries++;
      } else {
        const m = /^\s*(ok|not ok) \d+ - (.+)$/.exec(line);
        if (m?.[1] === "ok" && !/ # (?:SKIP|TODO)\b/.test(m[2]))
          passed.add(m[2]);
        if (m?.[1] === "not ok") errors.push(`Failed test: ${m[2]}`);
        if (m && / # (?:SKIP|TODO)\b/.test(m[2]))
          errors.push(`Unexpected skipped/todo test: ${m[2]}`);
        if (/^# tests [1-9]\d*$/.test(line)) summaries++;
      }
    },
    errors() {
      return [
        ...errors,
        ...(!summaries || !passed.size ? ["No successful test execution"] : []),
        ...required
          .filter(
            (name) =>
              !passed.has(name) &&
              !(
                format === "rust" &&
                [...passed].some((test) => test.endsWith(`::${name}`))
              ),
          )
          .map((name) => `Missing executed test: ${name}`),
      ];
    },
  };
}
export async function checked(
  command,
  args,
  { cwd = root, env = process.env, format, required = [], allowedIgnored } = {},
) {
  const audit = observer(format, required, allowedIgnored);
  const child = spawn(command, args, {
    cwd,
    env,
    stdio: ["ignore", "pipe", "inherit"],
  });
  const lines = createInterface({ input: child.stdout });
  lines.on("line", (line) => {
    process.stdout.write(line + "\n");
    audit.line(line);
  });
  const code = await new Promise((accept, reject) => {
    child.once("error", reject);
    child.once("close", (code, signal) =>
      signal ? reject(Error(`Test process ended: ${signal}`)) : accept(code),
    );
  });
  const errors = audit.errors();
  if (code !== 0 || errors.length)
    throw Error(`Test suite failed (${code}):\n${errors.join("\n")}`);
}
export async function nodeSuite(suite, { pattern, cwd, env } = {}) {
  const files = inventory()[suite];
  if (!files) throw Error(`Unknown Node suite: ${suite}`);
  console.log(
    `Node regression suite: ${suite}, ${files.length} discovered files${pattern ? `, focus ${pattern}` : ""}`,
  );
  const required =
    suite === "vault"
      ? manifest.cases
          .filter((c) => c.runner === "vault")
          .filter(
            (c) =>
              !pattern ||
              (pattern === "^P22 " ? c.id === "normal" : c.id === "recovery"),
          )
          .flatMap((c) =>
            ["http", "websocket"].map(
              (carrier) =>
                `${c.id === "normal" ? "P22" : "P21"} ${carrier} ${c.test}`,
            ),
          )
      : [];
  await checked(
    process.execPath,
    [
      "--test",
      "--test-reporter=tap",
      ...(suite === "vault" ? ["--test-concurrency=1"] : []),
      ...(pattern ? [`--test-name-pattern=${pattern}`] : []),
      ...files.map((file) => resolve(root, file)),
    ],
    {
      format: "node",
      required,
      cwd:
        cwd ??
        (suite === "web"
          ? new URL("../tools/web-channel/", import.meta.url)
          : root),
      env,
    },
  );
}
export async function rustSuite({ release = false, defaults = false } = {}) {
  console.log(
    `Rust regression profile: ${defaults ? "shipping-default" : "all-feature"} ${release ? "release" : "debug"}`,
  );
  await checked(
    "cargo",
    [
      "test",
      "--workspace",
      "--locked",
      "--offline",
      "--no-fail-fast",
      ...(defaults ? [] : ["--all-features"]),
      ...(release ? ["--release"] : []),
    ],
    {
      format: "rust",
      allowedIgnored: ignoredRust,
      required: defaults
        ? []
        : manifest.cases
            .filter((c) => c.runner === "workspace")
            .map((c) => c.test),
    },
  );
}
// Exported suite entrypoints produce this receipt only after their assertions pass.
export function qualifyEntries(completed) {
  if (
    !Array.isArray(completed) ||
    !completed.every((name) => typeof name === "string")
  )
    throw Error("Invalid browser execution receipt");
  const required = manifest.cases
    .filter((c) => c.runner === "browser")
    .map((c) => c.test);
  const missing = required.filter((name) => !completed.includes(name));
  if (missing.length)
    throw Error(`Missing executed browser entry: ${missing.join(", ")}`);
}
