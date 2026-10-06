#!/usr/bin/env node
// Offline after explicit locked install. No transport, wallet or research access.
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { nodeSuite } from "./test-runner.mjs";
const expected = readFileSync(
  new URL("../.node-version", import.meta.url),
  "utf8",
).trim();
if (process.versions.node !== expected) throw new Error(`Use Node ${expected}`);
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
