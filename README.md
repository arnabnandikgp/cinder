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
without operator assistance. The implementation foundation is the first PR;
production components are still to be built.

## Start here

1. [Progress and next handoff](docs/implementation/TRACKER.md)
2. [PR-sized implementation plan](docs/implementation/PLAN.md)
3. [Approved architecture and financial baseline](docs/implementation/BASELINE.md)
4. [Research, prototype evidence, and promotion map](docs/implementation/EVIDENCE.md)
5. [Contributor instructions](AGENTS.md)

These tracked documents are sufficient to start the first implementation phase.
Ignored `work/` contains the original local research; it is not a CI dependency.
Later phases must promote their needed sanitized specifications and fixtures before
relying on them. No wallets, credentials, signed live requests or private customer
data belong in the repository.

Check this foundation offline with Node 24:

```sh
node scripts/check-implementation-plan.mjs
node --test scripts/check-implementation-plan.test.mjs
```

The product language, package layout and pinned toolchain are a P01 deliverable;
Node here runs documentation checks, not a selected financial runtime.
