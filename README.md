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
P03's joined ledger and the reviewed P04–P16 stack are merged into `product/tee-v1`.
P04–P14 add funding/fee accounting,
evidence containment, atomic replay, AEAD ciphertext replication and writer fencing
against a trusted witness port, plus bound order intents and terminal-history
reconciliation, partial funds movements, queued collateral-aware payouts and
joined bounded margin/capital/liquidity admission under synthetic policies, plus
bounded liquidation, close exceptions, RF1/RF2 ADL restoration and bounded Pacifica
observation codecs with durable provenance and explicit qualification gaps, plus
a scoped native signing boundary and shared API-credit admission tested offline.
Independent witness/Nitro qualification, live native qualification and deployed
services remain later phases.
P15/P16 add the Anchor 1.2 custody vault and a durable vault/broker/venue funding
coordinator with original-attempt reconciliation and offline signed SBF tests.
P17 extends the same vault with immutable final recovery statements, separate
operator activation, owner-signed claims and shared lifetime payout counters.
Its joined finalization/private delivery workflow remains P21; passing membership
checks is not evidence of complete liabilities or full solvency.
P18 adds protected-journal wallet/agent authorization, account-private operation
views and an exact-integer client SDK. P19 adds a runnable TLS 1.3 relay/service
slice and attestation-gated Node channel, tested with explicit local fixtures.
Actual Nitro/NSM/key-release qualification remains P20; no deployed private API,
plaintext fallback or live trading readiness is implied.

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

Resume implementation from the current handoff in the tracker.
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

The six local crates are `cinder-kernel`, `cinder-ports`, `cinder-test-support`,
`cinder-journal`, `cinder-pacifica` and `cinder-api`. Storage, cryptography and native JSON/signing
dependencies stay outside the dependency-free kernel. No check uses live endpoints.
Node runs repository/SDK checks, not the financial runtime.

The isolated Anchor 1.2 workspace in `programs/` and unsigned TypeScript client
in `clients/vault/` are verified separately with `node scripts/check-vault.mjs`.
See [vault setup](programs/README.md) for the pinned tools. Signed transaction
tests run only in offline localhost Surfpool; no deployment is authorized.
