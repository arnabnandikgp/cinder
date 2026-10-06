#!/usr/bin/env node
// Offline entry point after toolchain installation and locked dependency fetch.
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { nodeSuite, rustSuite } from "./test-runner.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const groups = ["contracts", "lint", "rust", "client"];
const group = args[0]?.startsWith("--group=") ? args[0].slice(8) : undefined;
if (
  args.length > 1 ||
  (args.length === 1 && args[0] !== "--properties" && !groups.includes(group))
)
  throw new Error(
    "Usage: node scripts/check.mjs [--properties | --group=contracts|lint|rust|client]",
  );
const expectedNode = readFileSync(
  resolve(root, ".node-version"),
  "utf8",
).trim();
if (process.versions.node !== expectedNode)
  throw new Error(
    `Use pinned Node ${expectedNode}; got ${process.versions.node}`,
  );
const expectedRust = readFileSync(
  resolve(root, "rust-toolchain.toml"),
  "utf8",
).match(/^channel = "([\d.]+)"$/m)?.[1];
const rust = execFileSync("rustc", ["--version"], {
  cwd: root,
  encoding: "utf8",
}).trim();
if (!expectedRust || !rust.startsWith(`rustc ${expectedRust} `))
  throw new Error(`Rust toolchain mismatch: ${rust}`);

function run(command, arguments_) {
  process.stdout.write(
    `check: ${command === process.execPath ? "node" : command} ${arguments_.join(" ")}\n`,
  );
  execFileSync(command, arguments_, { cwd: root, stdio: "inherit" });
}

if (args[0] === "--properties") {
  run("cargo", ["test", "--workspace", "--locked", "--offline", "property_"]);
} else {
  for (const name of group ? [group] : groups) {
    if (name === "contracts") {
      run(process.execPath, ["scripts/check-implementation-plan.mjs"]);
      await nodeSuite("contracts");
      run(process.execPath, ["scripts/check-workspace.mjs"]);
    }
    if (name === "lint") {
      run("cargo", ["fmt", "--all", "--", "--check"]);
      run("cargo", [
        "clippy",
        "--workspace",
        "--all-targets",
        "--locked",
        "--offline",
        "--",
        "-D",
        "warnings",
      ]);
      run("cargo", [
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--locked",
        "--offline",
        "--",
        "-D",
        "warnings",
      ]);
      run("cargo", [
        "build",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--locked",
        "--offline",
      ]);
    }
    if (name === "rust") {
      const failures = [];
      for (const profile of [{}, { release: true }, { defaults: true }]) {
        try {
          await rustSuite(profile);
        } catch (error) {
          failures.push(error);
        }
      }
      if (failures.length)
        throw new AggregateError(failures, "Rust regression profiles failed");
    }
    if (name === "client") {
      run("cargo", [
        "build",
        "-p",
        "cinder-service",
        "--features",
        "local-fixture",
        "--bin",
        "cinder-service-fixture",
        "--bin",
        "cinder-verify-fixture",
        "--locked",
        "--offline",
      ]);
      await nodeSuite("relay");
      run(process.execPath, ["scripts/check-private-client.mjs"]);
    }
  }
}
