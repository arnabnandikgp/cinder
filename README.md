# Cinder

Cinder is being designed as a private, programmatic perpetual-futures trading
layer that integrates existing venue liquidity through an attested confidential
runtime.

This is the clean TEE product workstream. Pacifica is the first integration;
AWS Nitro Enclaves is the confidential-runtime target. The former Phoenix/PER
proof of concept remains in Git history and on the separate legacy workstream.
It is not an architectural template for this branch.

## Current status

The financial engine, protected journal, venue adapter, Solana custody/recovery,
private HTTP/WebSocket SDK and joined offline acceptance are merged through P22.
Bounded Nitro hardware qualification is recorded for its exact image and scope.
P23's implementation stack is merged, but the complete live deposit, trading,
funding, withdrawal and venue-backed recovery workflow remains unqualified. The
Pacifica missing-credit investigation is deferred; see the current tracker handoff.

The approved next custody model is [venue-directed escrow](docs/architecture/0030-venue-directed-escrow.md):
customers deposit to their venue/asset escrow, may reclaim an unswept batch, and
authorize its full sweep through Cinder's supported venue funding route. Returns
use a venue-specific PDA payout/recovery vault and shared payment counters, ending
at the registered user's wallet. Merged P23B/P23C provide the local escrow
program/client and financial journal/Runtime join. The legacy
common-vault instructions remain explicit. No deployment or history migration
is implied. [P25 backend onboarding](docs/architecture/0031-backend-onboarding.md)
adds opt-in fresh-wallet account discovery and unsigned escrow preparation;
frontend integration and deployment remain separate work.

[BULK candidate work](docs/venues/bulk.md) adds only offline exact wire/fill
codecs, not a second enabled venue. Current native environment, accounting and
funding/return evidence remain qualification gates; no cross-venue margin follows.

Tests, implementation, hardware evidence and production readiness are distinct.
There is no audit, universal solvency proof, unconditional exit guarantee or
customer-funds release implied by this repository. Use the tracker, plan and
baseline below for actual status, approved contracts and remaining gates.

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
12. [Durable journal and replay boundary](docs/architecture/0005-durable-journal.md)
13. [Encrypted durability and writer epochs](docs/architecture/0006-encrypted-durability.md)
14. [Bound order intents and completion](docs/architecture/0007-order-lifecycle.md)
15. [Partial funds movements and payouts](docs/architecture/0008-funds-payouts.md)
16. [Joined risk and peak-path capital](docs/architecture/0009-joined-risk.md)
17. [Protection claims and repeated deficits](docs/architecture/0010-protection-claims.md)
18. [Bounded liquidation and close exceptions](docs/architecture/0011-liquidation-exceptions.md)
19. [ADL allocation and restoration](docs/architecture/0012-adl-restoration.md)
20. [Pacifica observation qualification](docs/architecture/0013-pacifica-observations.md)
21. [Durable native signing and API credits](docs/architecture/0014-pacifica-execution.md)
22. [Solana custody vault and normal authorization](docs/architecture/0015-solana-vault.md)
23. [Funding coordinator and exact round-trip settlement](docs/architecture/0016-funding-coordinator.md)
24. [Private API, grants and operation semantics](docs/architecture/0018-private-api.md)
25. [Attested TLS, service entrypoints and trust boundaries](docs/architecture/0019-attested-service.md)
26. [Confidential HTTP/WebSocket and browser attestation](docs/architecture/0021-confidential-web-api.md)
27. [Exact private reads, paging and WebSocket delivery](docs/architecture/private-read-contract.md)

Resume implementation from the current handoff in the tracker.
Public documentation is maintained separately in the user's Mintlify repository.
Local `docs-content/` is ignored and is not required for checkout or CI.
Ignored `work/` contains the original local research; it is not a CI dependency.
Later phases must promote their needed sanitized specifications and fixtures before
relying on them. No wallets, credentials, signed live requests or private customer
data belong in the repository.

After installing Rust 1.97.1 with rustfmt/Clippy, a C compiler and Node 24.21.0,
prepare pinned dependencies with `cargo fetch --locked` and
`npm ci --ignore-scripts --prefix clients/private`, then run all checks
offline with the same command used in CI:

```sh
node scripts/check.mjs
```

The eight local crates are `cinder-kernel`, `cinder-ports`, `cinder-test-support`,
`cinder-journal`, `cinder-pacifica`, `cinder-api`, `cinder-service` and
`cinder-web-channel`. Storage, cryptography and native JSON/signing
dependencies stay outside the dependency-free kernel. No check uses live endpoints.
Node runs repository/SDK checks, not the financial runtime.

The isolated Anchor 1.2 workspace in `programs/` and unsigned TypeScript client
in `clients/vault/` are verified separately with `node scripts/check-vault.mjs`.
See [vault setup](programs/README.md) for the pinned tools. Signed transaction
tests run only in offline localhost Surfpool; no deployment is authorized.
