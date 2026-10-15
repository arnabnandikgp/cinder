# Cinder TEE contributor guide

Work in `cinder-tee`, based on `product/tee-v1`, never legacy `main`.
Cinder is a private perp broker: individual customer entitlements over pooled
venue execution, Pacifica first, AWS Nitro. BULK is paused. Phoenix/PER,
the website/demo and Mintlify docs are separate workstreams, not backend templates.

## Product context

- Merged foundation/API/custody/recovery/offline acceptance reaches P22. P23's
  code stack is merged; its full live financial acceptance remains incomplete.
  Hardware evidence qualifies its exact source/image/scope, not later changes.
- Approved next custody: [ADR 0030](docs/architecture/0030-venue-directed-escrow.md),
  customer/venue/asset escrow → full identified-batch sweep → allowlisted funding
  wallet → venue credit. Unswept batches are owner-refundable. Returns use a
  venue-specific PDA payout vault, shared normal/recovery counters and user-wallet
  destination. P23A–P23C are merged: custody contract, local Anchor/client boundary
  and journal/Runtime/private SDK integration, qualified offline. None implies
  migration, deployment, native credit qualification or general self-custody of
  venue collateral.
- Missing-credit investigation/live financial runs remain deferred until reopened.
  P25 adds opt-in, new-history backend/SDK onboarding (ADR 0031), not a terminal.
  Frontend integration remains separately scoped;
  paper UI behavior and marketing promises are not native qualification.

## Repo map

| Location | Responsibility |
| --- | --- |
| `crates/kernel` | Dependency-free `no_std` money, ledger, positions, risk and allocation |
| `crates/journal` | One atomic accepted history, replay/holds, encrypted replication/freshness |
| `crates/api`, `clients/private` | Owner/agent auth, private views, confidential browser/Node SDK |
| `crates/pacifica` | Venue observations, native signing and funding coordination |
| `crates/service`, `crates/web-channel`, `services/web-relay` | Enclave runtime, shared Noise core, opaque host relay |
| `programs/cinder-vault`, `clients/vault` | Anchor custody/recovery, generated IDL/types, chain client |
| `crates/ports`, `crates/test-support`, `scripts`, `tools` | Typed boundaries, explicit fixtures, checks, scoped qualification |
| `docs/architecture.md`, `docs/architecture`, `docs/operations` | System design, decisions, operating procedures |

## Resume and hand off

1. Check branch, `git status` and current user instructions. Preserve unrelated
   changes/ignored research; never stage the whole directory.
2. Read [TRACKER](docs/implementation/TRACKER.md)'s opening table/current handoff
   and owning phase, [PLAN](docs/implementation/PLAN.md),
   [BASELINE](docs/implementation/BASELINE.md) and relevant ADRs.
3. Consult [EVIDENCE](docs/implementation/EVIDENCE.md) for provenance. `work/` is
   optional ignored research, never a checkout/CI prerequisite. Promote only
   needed sanitized contracts/fixtures; report missing facts instead of inventing them.
4. Implement one bounded PR-sized slice on its branch. Record scope/dependencies,
   deviations, exact checked source/commands/results, review/CI and one next action.
5. A phase closes only after acceptance, relevant review and merge. Refresh the
   owning phase/tip on publication, fix and merge; old green heads stay historical.
   Follow PLAN's shallow bottom-up stack workflow; no automatic merge.

## Invariants that must survive every change

- One authoritative financial ledger/journal. No independent additive balances,
  internal crossing, per-user native accounts by default or hidden loss mutualization.
- Checked integer money, signed basis and explicit rounding/precision. No floating
  money, invented native precision or uncollectible debt counted as cash.
- Persist operation/attempt identity before signature exposure. Atomically commit
  postings, consumed keys and holds. Timeouts/response loss are not rejection or resend authority.
- Record real adverse events even when new admission is frozen. Cancel ACK does
  not prove all fills arrived. Unexplained differences remain visible/contained.
- Separate customer assets, house capital, coverage commitments and liquidity;
  never count designation as new assets or absorption as a second loss.
- RF1/RF2 economics are approved; do not reopen them casually. Numerical limits,
  coverage/priority, capital calibration and production assurance remain separate gates.
- Escrow sweep/refund is exclusive and exact-batch. No raw-balance sweep, trading
  before qualified credit, duplicate deposit, post-sweep refund or auto-swept payout return.
- Recovery is operator-assisted. No heartbeat-only payout, stale root, competing
  ordinary payout, reset paid counters or claim that Merkle inclusion proves solvency.
- Nitro has no durable private disk. External persistence is authenticated ciphertext
  with fresh-head/rollback protection; host plaintext contains no private records or keys.
- Private decryption/auth/accounting/signing stay in the enclave. Host relays ciphertext
  and bounded metadata. Library interfaces, fixtures and image builds are not live services.
- Native credentials retain their actual powers; a trading-only app policy cannot
  narrow a venue key that permits money movement. Keep role/key separation explicit.

## Checks and toolchains

Use Rust **1.97.1** and Node **24.21.0** from repo pins. The isolated program/client
pin Anchor language/SPL/CLI and `@anchor-lang/core` **1.2.0**. Setup, Agave/SBF,
Surfpool and IDL details live in [development](docs/development.md) and
[vault setup](programs/README.md); never replace global tools implicitly.

```sh
node scripts/check.mjs
node scripts/check.mjs --properties # focused iteration; full runner includes these
node scripts/check-vault.mjs        # owning program/client slice, offline local SBF
git diff --check
```

Cargo is locked/offline after explicit dependency hydration. Review dependency
policy changes; keep kernel `no_std` and free of I/O, clocks, SDKs and test doubles.
Never deploy `test-hooks`, fixture trust, insecure journal protection or fixed test
identities. Use `NO_DNA=1` for Anchor/Surfpool; do not inherit a configured wallet.
Routine macOS checks plus hosted Linux CI suffice unless the phase requires more.
For CI failures allow two locally verified, evidence-based fixes before Linux
reproduction. Needed Linux tests use the container skill/Apple containers,
source-only read-only exports, no network and container-local output; never mount
ignored research, wallets/cloud configuration or unrelated user work.

## Authority and evidence

Offline fixtures are the default. Each live venue/RPC/devnet/AWS deployment/run
needs current scoped approval; past experiment permission is not standing authority.
Mainnet/customer funds are a separate gate. Never print credentials or signed wires.
Stop for material new economics/security or scope changes; record alternatives.
Read-only review/status requests do not authorize fixes, deployment or messages.
No automatic subagents; use a review agent only with explicit authorization.
Label documented/observed/tested/derived/assumed/unresolved evidence accurately;
tests are not an audit, mathematical proof or mainnet qualification.

Keep this guide a stable entrypoint. Update product/code boundaries as they change;
session logs, counts, PR receipts and pending decisions belong in TRACKER.
