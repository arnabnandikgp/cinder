# Two-unit acceptance and qualification boundary

Date: 2026-10-17. P33–P36 are merged for their offline implementation scope at
`3486b9f`, not a second live venue or production release. [PLAN](PLAN.md), [TRACKER](TRACKER.md),
[native execution](../architecture/0040-unit-native-execution.md),
[custody/recovery](../architecture/0041-unit-custody-recovery.md).

## Reproducible offline evidence

| Workflow / property | Executed implementation and evidence | Limit |
| --- | --- | --- |
| Owner bootstrap and unit discovery | UnitOnboardingClient → purpose-separated real owner signature → Rust API → shared enrollment; canonical recipients and retained proof survive reopen | Disposable identities, synthetic session; not attestation or original chain deposit |
| Private strategy flow | UnitClient → actual unit Runtime Handler → API authorization/risk/holds → same protected journal | New-history opt-in, no shipping manifest migration |
| BULK native execution | Original admitted order → durable attempt/exposure → exact binary native signature, independently verified in Node | No HTTP send; explicit synthetic native qualification and funded collateral |
| Pacifica native execution | Same shared owner, different unit/source/account/agent → durable exposure and actual canonical native signature, independently verified in Node | No native send; does not inherit old native/hardware qualification |
| Attribution and uncertainty | Unknown native reply keeps holds; partial/duplicate/late fills update only original owner/unit; original-ID lookup and restart do not produce a second signature | No terminal frontier inferred from ACK, pagination or a fully filled quantity |
| Financial containment | A retained raw gap restricts A's new risk while B can admit; actual late A fills still ingest. Shared revoke and boot fence govern both | No cross-margin or mutualized losses |
| Custody | Exact original batch, exclusive refund/sweep, once-only broker recognition, full linked allocation, missing-credit restriction and replay | Journal unit tests and actual selected Pacifica adapter fixtures; not live BULK funding |
| Recovery | Atomic all-unit cutover; fresh shared head, each unit's own final reconciliation, canonical route, actual backing and lifetime paid counters | Existing encrypted package/dual-readback and SBF recovery suites reused; no new live root/activation |
| Publication and concurrency | Unit Runtime's witness/writer race, shared read-slot, revoke, expiry, unknown commit and final consuming-generation tests | Local storage/fault ports, not independent production witnesses |
| Confidential carriers | Existing Node/Chrome HTTP/WebSocket/Noise and TLS/opaque-relay suites plus explicit v2 request/update bindings | Carrier suites and joined native-driver suite are separate tests, not one changed-image live run |

The new joined SDK scenario is `clients/private/tests/units-process.test.ts`.
It starts the existing **feature-gated** service-fixture binary in `--unit-pipe`
mode. This is bounded private test IPC, **not** a public endpoint or encrypted
carrier. It uses actual SDK/application signatures, Runtime, API, financial
engine, record AEAD, replicated accepted history and both native adapters. Only
external balances/setup/observations, session/time and same-host witness are
explicit fixtures. It never reads wallet/cloud config or opens a socket.
Its fixed public disposable native seeds and seeded money must never be deployed.
Discovery in this fixture is explicitly synthetic: its crate-private,
`local-fixture`-gated constructor does not qualify a loaded funding inventory.
Shipping onboarding uses `open_with_custody` and rejects absent or mismatched
controllers/decimals/epochs/deposit limits. BULK has no qualified controller;
this offline driver does not circumvent that live startup gate.

The acceptance manifest now requires the named SDK case to actually run in the
private suite; it also requires unit custody/recovery cases in Rust. Deleted,
renamed/commented-out tests, wrong source roots and skipped execution cannot
satisfy the declaration/execution guards. Standard `node scripts/check.mjs`
discovers all new tests from tracked roots, with no ignored research prerequisite.
Use the pinned Node/Rust toolchains and prehydrated locked dependencies.

## What is not qualified

P39 adds a joined **encrypted** carrier case in
`clients/private/tests/unit-web-suite.ts`, run by
`tools/web-channel/check-runtime.mjs --units` in Node and Chrome and included in
the standard browser gate. It uses the actual unit Runtime and new bounded
worker, with explicit synthetic ports/capital/credit and a local attestation
root. It covers HTTP/WebSocket unit scope, native exposure, partial/duplicate
fill attribution, global revoke, durable restart/dispatch limits, outside-lock
I/O and fencing. It supersedes only the earlier separation of those offline
carrier/worker tests, not the live/hardware limits below. P38's inactive version 7
remains available; [ADR 0043](../architecture/0043-unit-workers.md) defines v8.

1. **BULK funding and returns:** no enabled rail/controller or invented bridge.
   Order/account APIs and paper balances are not proof of attributable Solana
   deposits or final beneficiary payment. Missing capability cannot be replaced
   with Pacifica evidence or a setter.
2. **Native causal completeness:** unsupported funding/ADL cause or coverage
   remains unresolved; read pagination is not a certified financial frontier.
3. **Live two-unit release:** actual loaded key-role/egress/chain policy, native
   authorities and consumed multi-unit release manifest must be assembled and
   qualified before serving this opt-in path. Existing shipping v1 remains v1;
   no implicit default unit, active-history migration or inherited KMS approval.
4. **Changed-image Nitro:** old P20/P23 receipts qualify their exact image/scope,
   not these changes. A new run needs a separate approved proposal, measured
   source/image, non-fixture configuration, permissions and cleanup receipt.
5. **Financial round trip:** no live funded BULK trade/funding/return/payout or
   venue-backed outage recovery is established here. Pacifica's deferred
   missing-credit investigation stays deferred.
6. **Production:** capital/coverage calibration, complete-liability/solvency
   assurance, independent security review and G01–G05 gates remain separate.

October 16's approved capability refresh made exactly two unsigned public GETs
to exchangeInfo/feeState, both origin 502. It establishes no new current private
schema/financial capability and is not proof of a global outage. No financial,
wallet/faucet/RPC or AWS action occurred in this implementation run.

## Next controlled qualification

See [the separate proposal](UNIT-QUALIFICATION-PROPOSAL.md). The user-authorized
merge of #78/#79/#81/#82 followed green exact heads and resolved review threads;
it does not approve that live proposal. Keep P36's native/hardware criterion open
rather than call offline fixtures an integrated live deployment. Next is bounded
native capability qualification, then non-fixture release assembly and separately
approved changed-image Nitro testing; unsupported funding/return rails stay disabled.
