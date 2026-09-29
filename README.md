# Cinder

Cinder is being designed as a private, programmatic perpetual-futures trading
layer that integrates existing venue liquidity through an attested confidential
runtime.

This is the clean TEE product workstream. Pacifica is the first integration;
AWS Nitro Enclaves is the confidential-runtime target. The former Phoenix/PER
proof of concept remains in Git history and on the separate legacy workstream.
It is not an architectural template for this branch.

## Current status

Research and bounded prototypes support starting implementation. They are not a
production deployment, audit, universal solvency proof, or guarantee of exit
without operator assistance. The planning foundation and pinned workspace are
merged, including exact financial primitives and canonical event identities.
The open P03/P04 stack adds the joined in-memory ledger, full-system architecture,
funding/fee accounting and evidence containment. Durable storage, native adapters
and deployed services remain later phases.

## Start here

1. [Progress and next handoff](docs/implementation/TRACKER.md)
2. [PR-sized implementation plan](docs/implementation/PLAN.md)
3. [Approved architecture and financial baseline](docs/implementation/BASELINE.md)
4. [Research, prototype evidence, and promotion map](docs/implementation/EVIDENCE.md)
5. [Contributor instructions](AGENTS.md)
6. [Development setup and checks](docs/development.md)
7. [Workspace decision](docs/architecture/0001-workspace.md)
8. [Financial primitives and encoding](docs/architecture/0002-financial-primitives.md)
9. [Full system architecture and operating flows](docs/architecture.md)
10. [Unified ledger decision and invariants](docs/architecture/0003-unified-ledger.md)
11. [Funding, fees and evidence reconciliation](docs/architecture/0004-funding-reconciliation.md)

These tracked documents are sufficient to start the first implementation phase.
Ignored `work/` contains the original local research; it is not a CI dependency.
Later phases must promote their needed sanitized specifications and fixtures before
relying on them. No wallets, credentials, signed live requests or private customer
data belong in the repository.

After installing Rust 1.97.1 with rustfmt/Clippy and Node 24.21.0, run all checks
offline with the same command used in CI:

```sh
node scripts/check.mjs
```

The three local crates are `cinder-kernel`, `cinder-ports` and
`cinder-test-support`. There are no registry dependencies or live endpoints in
this scaffold. Node runs repository checks, not the financial runtime.
