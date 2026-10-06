#!/usr/bin/env node
// Offline fixture processes only. No venue, wallet or research access.
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { nodeSuite, requirePrivateClientBinaries } from "./test-runner.mjs";
const expected = readFileSync(
  new URL("../.node-version", import.meta.url),
  "utf8",
).trim();
if (process.versions.node !== expected) throw new Error(`Use Node ${expected}`);
// Fail before any fixture starts: a missing relay otherwise leaks partially
// started services, and a missing verifier can falsely satisfy assert.rejects.
requirePrivateClientBinaries();
const root = new URL("../", import.meta.url);
execFileSync(
  process.execPath,
  [
    "clients/private/node_modules/typescript/bin/tsc",
    "--project",
    "clients/private/tsconfig.json",
  ],
  { cwd: root, stdio: "inherit" },
);
await nodeSuite("private");
