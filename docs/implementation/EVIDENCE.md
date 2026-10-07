# Research evidence and promotion register

Snapshot: 2026-09-30. This is a curated map, not an archive upload. The original
`work/` remains local and ignored. Paths below are intentionally code text, not
broken public links. [BASELINE](BASELINE.md) and [CONFORMANCE](CONFORMANCE.md) carry
the initial tracked contracts; [PLAN](PLAN.md) assigns the remaining promotion.

## P21A initial web-channel qualification

[ADR 0021](../architecture/0021-confidential-web-api.md) and the
[isolated tool](../../tools/web-channel/README.md) promote W06/W08's channel
requirements and a single public Cacophony NK vector—not ignored experimental
authority or private data. Snow 0.10.0 package archive SHA-256:
`599b506ccc4aff8cf7844bc42cf783009a434c1e26c964432560fb6d6ad02d82`.
Expected vector output is from the upstream release, not calculated by Cinder;
the retained license/source is in the tool. A separate manifest/lock/graph policy
pins the candidate suite/entropy and browser build, with no shipping graph change.
Native debug/release vectors and hostile-wire cases, plus actual Node/WASM and
Chrome/WASM → native Rust tests, are reproducible without `work/` or live services.
Synthetic responder trust is explicit. This does not verify a Nitro quote, prove
key erasure, qualify an HTTP financial API or close P21A. The recorded opaque-key
erasure limitation must be resolved/reviewed before candidate production use.

The next stacked slice promotes [browser verifier and binding qualification](../../tools/web-channel/ATTESTATION.md):
exact public purpose/context vectors, 40 fresh signed synthetic fixtures checked
against the existing separately compiled native oracle, Node/actual-Chrome tests
and five historical public AWS DER certificates from P20's public-only clock probe.
The retained source quote SHA-256 is
`b9352dd188179fb5492160291b3cd9bb7aefcaf98d673ca971c713e1e103e61e`;
only its certificates/capture time are promoted, not the original quote or ignored
research tree. Historical certificate shape/path is not fresh web attestation.
Source review found PKI.js needs explicit path-length/closed-policy checks, and
actual AWS leaves use noncritical KU with no AKI/SKI; both have bounded regression
evidence without weakening the native path. Candidate accepts only a narrower
AWS-shaped subset, not arbitrary PKIX equivalence. No shipping graph/Node TLS,
private authority, wallet, AWS resource or customer financial state is changed.
Snow erasure, SDK lifetime/fencing, HTTP/WS and changed-image qualification remain
open; passing this candidate must not close them.

## How to reuse the research

P23 additionally promotes [confidential chain ports](../architecture/0024-confidential-chain-ports.md):
controller-bound signing/finality, purpose-separated release, original owner
deposits and bounded diagnostic scheduling. Deterministic **unfunded** public
codec fixtures are in `crates/pacifica/tests/fixtures/funding-codec-public.json`
and `customer-deposit-public.json`; `scripts/generate-funding-codec-fixtures.mjs`
reproduces them using the locked Anchor/web3.js client. Independent Rust/Anchor
checks cover large u64s and exact signatures/ABI. These are not the historical
M1 wallets, live wire artifacts or a native-credit provider. The existing journal,
funds controller and P22 lifecycle assertions are reused, not the experiments'
plaintext state or injected completion certificates. Current primary diagnostic
sources and remaining causal/funding/cut gates live in the
[runbook](../operations/live-qualification.md). Exact final local/hardware results
are recorded in TRACKER, not inferred from this promotion register.

Pre-review local application checkpoint `33dae28b2c3846d9956a7e906f5663d78d93cd76`
passes the fixed host and exact-source ARM64 shipping gates; the unchanged
program/browser owning sources retain the 59/59 SBF and actual Node/Chrome/WASM
receipts. Completed original receipts survive a later one-way freeze, while new
old-epoch actions stay fenced. See [the P23 receipt](TRACKER.md#p23--live-integration-qualification)
for commands, logs, exact artifacts, counterexamples and current live blockers.
No historical experiment authority or injected native completion was promoted.
Subsequent PR #55 corrections qualify exact role coverage, rejected-deposit
progress, verified expired no-wire closure, absence of unsupported source cuts
and bounded lock backpressure. They require new host/ARM application evidence;
the old ELF is not the current image. TRACKER records their exact gate receipt.
Correction source `5acf79bd2f931e85325841a06bd87b038cb1cf28` passes the full
pinned host runner and isolated exact-source ARM package/rebuild/refusal checks.
Its new enclave ELF hash is
`0f5da0b061380ad16d1e139765b505ee3d6e3cdf94fc9195b5dc50975d8a1ff1`;
this is not an EIF/PCR or changed-image hardware qualification.

The 2026-10-06 read-only hardware session additionally promotes a **public
certificate-only** AWS path capture in
`tools/web-channel/attestation/aws-serial-padding-capture.json`. It exercises the
observed positive 20-byte serial magnitude plus minimal sign padding without
changing fixed-root, certificate-signature/path, PCR/context or freshness checks.
Negative/nonminimal/oversized serials, expiry and signature corruption reject.
No quote context, customer data, signed command, credential or private key is in
the capture. The same session's transport regressions cover bounded opaque idle
ingress, skipped busy Read polls and post-I/O cadence, plus a separately bounded
15-second application reply with five-second handshake/frame limits unchanged.
Actual grant delivery/update progressed, but full live concurrent stream handling
is **not qualified**: cloud history verification under the shared journal lock
causes long commits and bounded read refusals. Detailed artifact/fault/cleanup
receipts and this limitation belong in TRACKER, not a fabricated live pass.

P22's [tracked acceptance manifest](acceptance-manifest.json) maps W01–W09 and
V01–V10 to named executable owning-layer/joined tests. Its
[acceptance contract](../architecture/0022-offline-acceptance.md) distinguishes
fake native ports, compiled local SBF and historical/hardware limits. All required
seeds and expected outcomes are tracked; no ignored research is needed to run it.
This promotes reproducible requirements/economics, not experimental authority.

1. **Reuse requirements and test vectors first.** The production implementation
   must reproduce approved economics, not the accidental structure of a prototype.
2. **Adapt reviewed mechanisms.** Extract canonical encodings, authority checks,
   persistence patterns and failure cases only after review in the new architecture.
3. **Do not promote experimental authority.** No disposable keypairs, deployment
   accounts, raw signed payloads, private journals, credentials, API keys, test
   faucet assumptions or locally trusted witnesses become production defaults.
4. **Promote before depending.** Each phase adds its precise sanitized specification,
   fixtures and source manifest to tracked docs/tests. Once promoted, CI and review
   use those tracked files. If unavailable, mark the detail unverified rather than
   depending on somebody else's ignored directory or guessing a wire contract.
5. **Separate evidence levels.** Documented interface, observed bounded test,
   synthetic test, hand derivation and production/hardware qualification are not
   interchangeable. Dates/hashes identify what was reviewed, not its correctness.

## Source families and owners

| ID | Local sources under `work/` | Reuse / owning phases | Evidence limits |
| --- | --- | --- | --- |
| W01 | `specs/financial-model.md`; `specs/financial-model/IMPLEMENTATION_HANDOFF.md`; `specs/financial-model/ROADMAP.md`; `specs/financial-model/verification.md`; the eleven suites below | Approved accounting contract, transitions, adversarial examples and conformance vectors; P02–P12, P22 | Exact-integer reference projections, not a joined durable production engine or universal solvency proof |
| W02 | `FINANCIAL_CORE.md`; `specs/financial-execution.md`; `analysis/scenarios.cjs`; `analysis/reconciliation-scenarios.cjs`; `specs/spec-scenarios.cjs` | Requirements history and winner/loser/opposing-account cases; P03–P04, P22 | Older proposals do not override later decisions; internal-crossing scenarios are counterfactual, not a feature |
| W03 | `venues/pacifica/financial-mapping.md`; `venues/pacifica/research-closeout.md`; `venues/pacifica/remaining-research.md`; `venues/pacifica/omnibus-subaccounts.md`; `venues/pacifica/experiments/p1-offline/README.md`; `venues/pacifica/experiments/p2-authority/README.md`; `venues/pacifica/experiments/p2-funded/README.md`; `venues/pacifica/experiments/p3-execution/README.md` | Active native capability/precision/authority map and sanitized signing/execution fixtures; P13–P14, P23 | Partial documented/testnet qualification, not mainnet parity, complete native ADL evidence or exact engine precision |
| W04 | `venues/pacifica/experiments/funding-round-trip/live-result.md`; `venues/pacifica/experiments/funding-round-trip/fixture/README.md`; `venues/pacifica/experiments/funding-round-trip/fixture/controller/README.md`; `venues/pacifica/experiments/funding-round-trip/fixture/live/README.md` | M1 vault/controller invariants, attempt persistence and receipt correlation; P08, P15–P16, P23 | Real bounded devnet/testnet funds, not production custody or native PDA control; never copy live-run secrets |
| W05 | `experiments/private-recovery/CLOSEOUT.md`; `experiments/private-recovery/MILESTONE.md`; `experiments/private-recovery/README.md` | M2 crash/freshness/claim/fencing tests; P06, P17, P21–P23 | Local processes, synthetic claims and local SBF; no actual Nitro or integrated venue outage |
| W06 | `specs/private-api-runtime.md`; `tee/tee-learning-handoff.md` | Private capabilities, transport/key separation, durable acceptance and threat model; P06, P18–P20 | Candidate crypto/channel choices need ADR/security review; parent encryption alone is insufficient |
| W07 | `specs/verification-recovery.md`; `analysis/verification-target.md`; `analysis/d14-evidence-map.md` | Separate code/channel, inclusion, complete backing and executable exit assurances; P17, P21, P24 | Historical BULK bindings are not Pacifica interfaces. Merkle inclusion is not completeness or full solvency |
| W08 | `analysis/nitro-qualification.md`; `experiments/nitro-d14/README.md`; `experiments/nitro-d14/manifest.json` | Qualification checklist, attestation/transport negative tests and release evidence; P19–P20, P24 | Local/offline evidence does not establish live enclave, KMS policy or independent rollback witness |
| W09 | `analysis/omnibus-risk-mitigation.md`; `specs/financial-model/pool-protection-review.md`; `specs/financial-model/restoration-review.md`; `specs/financial-model/proportional-allocation.md`; `specs/financial-model/funds-and-capital.md`; `specs/financial-model/protected-lifecycle.md` | PF/RF decisions, bounded capital/quotas and failure counterexamples; P09–P12, P22–P24 | Numerical limits, final priority, independent review and calibration still open |

BULK research, downloaded HyperLink contracts/audits and Phoenix history remain
comparison/provenance resources, not dependencies or templates for Pacifica
production code. Review licenses before publishing third-party sources. Do not
upload the entire local research tree to make links work.

## Verified evidence versus new work

### P20: loaded-policy and NSM first slice

Historical checkpoints below are superseded by the pre-hardware assembly and
dated hardware receipts at the end of this section. The earlier offline results
must not be read as claims about actual hardware or later source revisions.

[ADR 0020](../architecture/0020-nitro-runtime.md) implements actual loaded-policy
commitments and AWS's pinned NSM driver boundary. W06–W08's no-host-approval,
exact-release, role-separation and no-fallback requirements have offline regression
tests. The default build cannot construct an injected fixture NSM device. Tests
cover synthetic NSM protocol replies and the ordinary-host refusal path, not
real enclave attestation or entropy. This is not a complete runtime manifest,
runnable Nitro financial application, KMS release or fresh-witness qualification.
No native capability is activated and no experimental authority is promoted.

The 2026-10-02 continuation adds purpose-separated signed NSM clock verification
and explicit TLS certificate clock selection. Six additional local regression
groups use fresh synthetic X.509/COSE fixtures or private protocol-test seams;
production still accepts only the AWS root and actual NSM device. This identifies
and fixes a real-NSM request-ordering bug without weakening client freshness.
No real timestamp, driver-latency, RNG or hardware guarantee is inferred. See the
ADR's time-bootstrap assumptions and remaining image/runtime qualification.

The socket/HTTPS continuation is tested on macOS and network-disabled Linux
ARM64 with an exact source export. It uses owned AF_VSOCK APIs, bounded opaque
parent ingress/egress, and in-enclave hostname/CA/time-verified one-shot HTTPS.
Wrong-root/hostname/time tests are actual loopback TLS with synthetic certificates;
Linux owned-stream tests use a Unix pair, not a Nitro device. No live native action,
AWS resource, production key or customer account is used. The local image/source
fingerprints, setup deviations, HTTP subset and remaining measured-app/KMS/
fresh-witness gates are recorded in ADR 0020. This is not hardware evidence or
a complete release manifest. No P20 acceptance criterion is closed.

The completed pre-hardware assembly consumes a full measured public manifest,
uses fresh purpose-bound NSM recipients for five KMS roles, and constructs the
actual private API/gateway/funding objects over one encrypted S3/DynamoDB journal.
Default binaries contain no fixture provider, fake funds or flat admission policy.
The official pinned SigV4 signer and closed temporary credentials are explicit;
no default chain/retry/provider appears through a full cloud SDK. Missing witness,
rollback/epoch/CAS errors and clock expiry fail closed, not by reinitializing files.

Local evidence: manifest/role tampering, synthetic recipient replay separation,
strict AWS framing, exact signed payloads and strong-read/CAS construction,
policy-derived customer/native margin, actual configuration construction and
real trusted-preparation process/file permissions. Full Linux debug/release tests
and default-feature package/rebuild comparison pass against the exact source
recorded in TRACKER. Synthetic CMS is not AWS OAEP interoperability; separate
credential IDs/buckets are not proof of IAM or administrative independence.
At that pre-hardware checkpoint there was no real NSM/vsock/cloud acceptance
receipt. Native trading, funding
and reads stay disabled until G01/G02/P23; readiness means private API/storage
readiness, not live brokerage. Package/runbook and outstanding hardware tests are
in [ADR 0020](../architecture/0020-nitro-runtime.md) and
[operations](../operations/nitro-qualification.md). No criterion is closed by an
ELF digest, local fixture or document update.

Actual disposable hardware continuation, 2026-10-02: the non-debug application
ran on Nitro with actual five-role recipient KMS release, the production-root SDK
verifier, owner views/grants/revocation and the encrypted journal. Restart,
one-replica recovery, missing/corrupt accepted state refusal, witness-route loss
and retained-head writer-epoch fencing were observed. Three real AWS encoding/
certificate compatibility fixes are checkpointed at `37487ca`; current changed
source passed macOS release/workspace/SDK tests and Linux service tests/Clippy.
Exact ELF/EIF/PCRs, failed probes, evidence limits, verified cleanup and conservative
spend are in [the dated receipt](../operations/nitro-hardware-2026-10-02.md).
This is partial P20 evidence, not a production topology, native workflow,
independent audit or completion of the remaining negative matrix.

Second hardware session: [follow-up receipts](../operations/nitro-hardware-followup-2026-10-02.md)
preserve the failed journal restores and the subsequent actual independently
verified `security-batch/PASS`. Restore now reuses validated immutable replicas;
a create-only regression reproduced the original failure before the fix.
Current code export `5557d3350f371d616965006a90ea03b55420400f` passes macOS
release workspace, targeted macOS/Linux journal/service checks and all 17 SDK
tests. Wrong-measurement release rejection passes for all five roles with the
same KMS approval and capsules, bracketed by verified positive recipient runs.
The separate-register writer race, final shipping application SDK/restart
regression, debug refusal and natural 90-second loaded-lease fence also pass.
Actual 900-second witness STS expiry and same-expired-capsule restart refusal
also pass, with a longer boot lease, unchanged accepted head, enabled KMS keys
and a live parent relay. The bounded P20 hardware matrix is complete; PR
review/merge and production/native gates remain separate. Teardown is recorded
in the follow-up; no native workflow or production governance is inferred.

Review fixes, 2026-10-03: runtime commit
`82073c221895f505e4d029f2b4e99c2a6e13031e` adds offline regressions for repeated
ignored response headers, interrupted-genesis funding binding, per-connection
IPv4 resolution without retries and the named strict-path AKI error. Full pinned
macOS debug/release/workspace and 17 SDK checks pass. This changes source; the
earlier measured export and actual Nitro receipts are not relabeled as hardware
qualification for it. The updated joined release is qualified separately in P23.

Final outside-diff review fix: `c94adc94c3fa72b545c0aa5ce1448668309e239c`
applies the same bounded duplicate-field policy to cloud responses. Its new
regression fails before the fix and passes afterward, including 64/65 total
field-line boundaries. Full offline Rust debug/release workspace tests, strict
all-feature Clippy and format pass. This is new offline evidence, not a new AWS
hardware receipt; no native capability is enabled.

### P19: attestation-gated runnable service slice

[ADR 0019](../architecture/0019-attested-service.md) promotes W06–W08's exact-key
binding, no early private identity, bounded framing, independent client release
and opaque-parent requirements. The new Node/TLS/service tests run over actual
processes and the existing AEAD replicated journal, including service death,
reconnection, original-operation query, changed-economics conflict and hostile
channel substitutions. The pinned public AWS root is checked against its published
DER fingerprint; positive quotes use a fresh synthetic ECDSA/X.509 CA, not NSM.
Fixture trust has a separate feature/entrypoint; production verifier rejects it.
Trusted entropy/time/NSM/key release/vsock/independent witness and live egress stay
P20 gates. No private audit source, experimental key, website or native account
is promoted. Local process memory isolation is not a TEE security result.

Source fingerprints at promotion (SHA-256; research is not a build dependency):
W06 `private-api-runtime.md`:
`6ee0084666c8681f9b894a9d1a9c645997aa10d0bf352b257053f74798b03522`;
TEE handoff:
`11c0d09f2d1bb75f46b69ab14db28f217e2bcba16382927199abc69ba24ce1e4`;
W08 Nitro qualification:
`819297c008072b07599a75dbad233c70afce2185c88c451812e600dd50c69e71`;
D14 experiment README:
`4dfd26b3a44d862e7d5b9d0ddf122c6f556fd165879667e9bc5e34fad2e8d0c8`.

### P02: initial production primitive promotion

[ADR 0002](../architecture/0002-financial-primitives.md) records exact source
fingerprints, signed-basis/identity requirements, tracked golden vectors and their
limits. Primitive tests no longer depend on ignored research. V02's joined ledger
transition remains P03; native precision/causal qualification remains P13.

### P03: joined position and location ledger

[ADR 0003](../architecture/0003-unified-ledger.md) records the one-book bridge,
source hashes, attribution boundary and promoted implementation tests. V01/V03
and the winner/loser/default cases run against the Rust ledger; V02 now invokes
the production fill transition. The [system architecture](../architecture.md)
maps approved W01–W09 requirements to their future components without treating
prototypes as deployed services. Later phases own durable consumption, admission,
full transfer lifecycle and native qualification.

### P04: funding, costs and discrepancy containment

[ADR 0004](../architecture/0004-funding-reconciliation.md) promotes W01/W03 and the
relevant W02 reconciliation cases onto the same Rust ledger. V04 and delayed
funding-cut, fee-inclusive PnL, source mismatch, aging, correction and replay tests
run without local research or endpoints. A synthetic normalization-port fixture
checks REST/WS semantic equality and lossless/unknown fields; it is not native
qualification. Exact source hashes and the still-unexplained Pacifica precision
gap are recorded in the ADR. Durable raw evidence remains P05/P13, not the
in-memory retained normalized observations delivered here.

### P05: durable atomic transition and exposure boundary

[ADR 0005](../architecture/0005-durable-journal.md) promotes W01 atomicity,
W04 immutable attempts/unknown outcomes and W05 crash/rollback lessons into the
tracked journal and synthetic process tests. Ten actual child kills cover five
commit points for joined credit/hold/consumption and prepared-action exposure.
Replay, concurrent capacity, schema migration, real SQLite disk-full, source
evidence retention and secret-safe diagnostics no longer depend on ignored files.
The valid-history rollback counterexample is retained for P06: the fixture
protector and public hash chain are not cryptographic or freshness evidence.
No funding prototype, wallet, signed live payload or M1/M2 authority was promoted.

### P06: encrypted accepted tail and writer epochs

[ADR 0006](../architecture/0006-encrypted-durability.md) promotes W05/W06 corruption,
rollback, two-copy acceptance, orphans and writer-fencing cases into the same Rust
journal. Real AEAD and local ciphertext file/crash tests replace toy protection
for this path. The witness remains a trusted synthetic port; no independent remote
witness, hardware evidence, live signing authority or experimental keys are promoted.

### P07: bound intents and qualified terminal history

[ADR 0007](../architecture/0007-order-lifecycle.md) promotes W01/W03 order-identity,
partial/terminal, duplicate, bound-violation and independent pending-direction cases
into the same durable journal. Its source hashes and trusted authentication/causal
qualification boundaries are explicit. No reference ledger is added to assets.

### P08: partial movements and collateral-aware payouts

[ADR 0008](../architecture/0008-funds-payouts.md) promotes W01/W04–W05 partial
receipt, FIFO, paid-counter and payout-race cases into the existing Rust ledger
and journal. All 24 four-receipt permutations preserve each prefix's bridge,
including arrival-before-debit. Open-position tests use qualified synthetic marks
and explicit initial margin; pending-order outcomes remain P09's prerequisite.
No fixture wallet, live authority, native finality or funding sender is promoted.

### P09: joined bounded admission

[ADR 0009](../architecture/0009-joined-risk.md) promotes W01/W09 leverage, gross/net,
pending-order, every-prefix capital and location-liquidity cases into one derived
report over the production ledger and shared holds. The source hashes, conservative
interval derivation and finite-path limitations are explicit. No experimental
ledger contributes assets, historical ADL candidate becomes policy, or synthetic
limit becomes live calibration. P10–P12 extend scenario transitions in their layers.

### P10: repeated protection episodes

[ADR 0010](../architecture/0010-protection-claims.md) promotes W01/W09 and V06 into
the same financial ledger, durable policy/decision controller and joined capital
gate. The source hashes and single-episode limitation removed from the study are
recorded. A tracked opposing-account counterexample demonstrates why aggregate
reconciliation is not extraction resistance; global lifetime caps remain consumed
across recoveries, account changes and policy revisions. G02 allocation approval,
actual authentication and live insurance calibration are not simulated into truth.

### P11: bounded liquidation and close exceptions

[ADR 0011](../architecture/0011-liquidation-exceptions.md) promotes W01/W03/W09
close/unwind economics into the existing lifecycle and ledger. Its 432 bounded
long/short allocation cases include exact whole-fill conservation; dedicated
fractional conversion tests retain split-owner residue. Actual adverse execution
is not capped at a prior reserve. Synthetic depth, prices and support budgets
are qualification inputs, not live calibration or an execution guarantee.

### P12: original-basis restoration and bounded quota compilation

[ADR 0012](../architecture/0012-adl-restoration.md) promotes W01/W03/W09 RF1/RF2
into the single ledger/journal. Source reference hashes:
`proportional-allocation.cjs` SHA-256
`6d79cbe82ce9190741a10ae4bb0ecf80e18fe5409dcd14720d82151828ad2c3b`;
`proportional-allocation-checks.cjs`
`69b7f19ba330e2f05cb8c8204ae529ecfb0095fbb6c9cfa433f926323b6b325f`.
The production compiler uses certified repeated blocks and explicit resource
limits instead of the research per-lot tape. Tracked differential tests preserve
the 3,905 small, 250 seeded, 10,000-lot and account-splitting vectors; additional
large profiles and bounded failure are tested. Joined ledger tests cover actual
basis/refunds, 1,024 long/short fill chunkings, native conversion residues, no gap
funding, changed intent, terminal-history release and post-submission shocks.
Synthetic policies are not calibration, independent review or universal solvency.

### Financial studies: rerun at P00

Command from the existing research worktree:

```sh
node work/specs/financial-model/run-checks.cjs
```

Rerun on 2026-09-30: **11/11 suites, 233 groups pass**, offline. This is not part
of fresh-checkout CI because those research files are intentionally untracked.

| Suite | Groups | Production promotion |
| --- | ---: | --- |
| `checks.cjs` | 20 | P02–P04, P10, P17 |
| `risk-policy-checks.cjs` | 16 | P09–P12 |
| `execution-lifecycle-checks.cjs` | 23 | P07, P13–P14 |
| `pool-protection-checks.cjs` | 42 | P03, P09–P12 |
| `protected-lifecycle-checks.cjs` | 48 | P10–P12, P21 |
| `proportional-allocation-checks.cjs` | 16 | P12 |
| `funds-lifecycle-checks.cjs` | 19 | P08, P16–P17 |
| `capital-envelope-checks.cjs` | 14 | P09–P10 |
| `analysis/scenarios.cjs` | 14 | P03–P04 |
| `analysis/reconciliation-scenarios.cjs` | 10 | P04–P05, P13 |
| `specs/spec-scenarios.cjs` | 11 | P07–P09, P22 |

First eight suites live in `work/specs/financial-model/`; final three paths are
relative to `work/`. Counts are check groups, not independent mathematical proofs.
Preserve named counterexamples and expected failures while porting; do not simply
count tests or turn an adverse solvency outcome into a test failure.

Known reference limitations to remove: separate base/pool/funds projections,
in-memory durability, flat-only payout slice, single deficit episode per customer,
assumed causal/completeness witnesses, bounded lot-by-lot schedule and synthetic
capital paths. String fingerprints are not production cryptographic commitments.

### Custody M1: historical bounded observation

The 2026-09-27 report records an actual devnet/Pacifica-testnet round trip:
**20 faucet USDP → 19 customer return + 1 venue fee**. It reports **81/81** local
fixture checks and a controller killed after withdrawal POST exposure, restarted
and reconciled against the original intent with one POST total. No orders,
mainnet funds or AWS were involved. Bootstrap lending-disable timing was a specific
observed exception, not a standing permission to lend user funds. These tests were
**not rerun in P00**; P16/P23 must test the production controller afresh.

### Recovery M2: historical bounded local observation

The closeout reports **23/23** local checks, real process-kill boundaries,
replicated authenticated ciphertext, stale/corrupt data and conflicting-writer
tests, standalone local claimants and compiled-SBF recovery validation. One missing
copy could recover; insufficient valid evidence stopped. Synthetic claims totaled
300 before normal payout/loss adjustment. Historical focused review is not a
production audit. **Not rerun in P00**; no live Nitro or venue-outage guarantee.

## Active Pacifica qualification cautions

Preserve these in P13's tracked adapter map; recheck current primary documentation
and controlled evidence before enabling the corresponding capability:

- Decimal strings require lossless decoding and versioned tick/lot/asset units.
  Test-USDP collateral does not choose production collateral.
- P3 observed execution price distinct from entry basis, and two reversal execution
  records under one order. Order ID alone cannot deduplicate economic fills.
- Sampled native PnL included fees; a **-0.000002 test-USDP** replay discrepancy
  remains identified, not explained away as a universal rounding rule. Unequal-price
  entries, fractional basis/fees and maker rebate precision need qualification.
- REST and WS may repeat the same economic execution. Last ID/history cursors are
  not proven complete, globally causal financial journals. Snapshot flatness does
  not replace missing execution evidence.
- A native agent's actual fund-moving rights need explicit treatment. User consent
  to Cinder trading is not a native grant over the omnibus identity.
- Rolling API-credit costs matter, not request count alone. Preserve cleanup
  capacity and reconcile unknown mutations; do not evade quotas with new identities.
- Native reduce-only applies to pooled exposure. Gross private funding needs
  qualified boundary/rate evidence even when native exposure is net-flat.
- Native margin isolation, parent/subaccount authority and fee aggregation need
  evidence independently; do not implement a per-user account as an easy escape.

Upstream starting points (not newly requalified in P00):
[Pacifica docs](https://docs.pacifica.fi/),
[trade history](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-trade-history),
[orders](https://docs.pacifica.fi/api-documentation/api/rest-api/orders/create-limit-order),
[rate limits](https://docs.pacifica.fi/api-documentation/api/rate-limits).

## P13 native observation promotion

2026-09-30: promoted [VM-01–VM-09](../venues/pacifica.md), an immutable qualification
profile and lossless native schemas into `cinder-pacifica`. Twelve joined tests
include a sanitized six-row numeric transcription, REST/WS dedup, reversal legs,
whole-snapshot replacement, exact large IDs, funding/forced-event containment,
cursor gaps and an explicit two-atom cash discrepancy. Raw bodies/provenance share
the protected journal transaction; independent diagnostic views replay that history.
See [ADR 0013](../architecture/0013-pacifica-observations.md). Current primary docs
were checked; historical observations were not rerun or upgraded into live evidence.

2026-10-01 review correction: malformed multi-row/pagination pages were reproduced
as partially mutating replayed diagnostics; transactional inspection now retains
only prior accepted progress plus named gaps. The exact-input raw-resolution port
is offline-tested with seven synthetic groups and immutable evidence bindings.
It does not supply a live authenticated complete-history/no-effect provider;
that capability remains disabled pending G01/P23 qualification. See ADR 0013.

## P14 native execution promotion

2026-09-30: [ADR 0014](../architecture/0014-pacifica-execution.md) promotes official
canonical signing and scoped create/cancel fields, actual Ed25519/base58, durable
preimage/key binding and conservative shared API-credit accounting. Public docs
were rechecked; the historical RFC8032/golden request vector is tracked in unit
tests and cross-checked against Node/OpenSSL. Fake transport covers lost ACK,
429/delay, cancellation races and commit/fencing faults. No live test, key import
from a wallet, deployment or production capability promotion occurred. P13's hosted
checks and CodeRabbit were green with no inline findings at this phase boundary.

## P15 custody instruction promotion

[ADR 0015](../architecture/0015-solana-vault.md) promotes W04's actual-transfer,
asset/recipient/authority and rollback requirements, and W05's normal/recovery
fence and permanent paid-history requirements. The original source hashes are
recorded there. Tracked tests execute the new Anchor 1.2 SBF in offline Surfpool,
not a copied one-run vault or mock instruction validator. Public custody evidence
is not another financial ledger. No experimental deployment account, live signed
payload, RPC key or customer private state is promoted. The venue controller,
recovery proof/activation and live governance remain P16/P17/G03/G05 boundaries.

## P16 funding-controller promotion

[ADR 0016](../architecture/0016-funding-coordinator.md) promotes W04's durable
original-attempt controller, causal credit/withdrawal correlation, actual-fee
accounting and crash/cleanup requirements into the existing P08 journal. It
records the original controller/binding/public-IDL hashes and a sanitized minimal
deposit ABI. Broker custody is a separate location in the same financial state.
The tracked client verifies exact signed wires and executes custody rails in SBF;
the coordinator's actual killed-child test exercises POST-before-reply restart.
None of these checks uses ignored research, wallet material, external endpoints,
research account defaults or historical bootstrap permissions. Source-complete
native settlement and authenticated chain finality remain qualified ports; codec
and fake-observation tests do not upgrade them into live G01/P23 evidence.

## Requirement-to-phase traceability

P23 remediation promotion (2026-10-07):
Review follow-up at published `e50dc9d`: both bots' current findings are verified
against actual code, with pre-fix failing regressions. Nonfatal ordinary read
refusal is distinct from periodic skip and fatal fence; boot-local immutable
deposit rejections avoid RPC-budget depletion without creating credit; opaque
reply waiting and absolute frame/body assembly have separate bounds. Tests and
new-source/package receipts at `d2bf4b6` are in TRACKER. Old ARM/EIF identities cannot
qualify these changes. The user approved the bounded C5 proposal and renewed
AWS login. Exact fresh-policy/identity, current AMI/price/quota/network and owned
termination checks pass, and one fresh C5 host launched 2026-10-07 03:13:48 UTC.
Its public-only image is independently downloaded/inspected before six exact
PCR/context key policies; parent plaintext-decrypt and witness-access denials
pass. Initial actual Node/Chrome concurrent reads, same-socket grant/two watches,
agent revocation and natural expiry pass with zero financial/native/RPC activity.
The strong initialized head is sequence 2, explicitly not a sequence-1 pass;
exact sequence-8/32 reads pass too. Cut 64 is unpassed; its unavailable original
grant reconciles as accepted head 51 with no resend. Actual encrypted restart
preserves all 49 operations/digests and authority epoch 3; replica loss refuses
the one retained revoke, proved not accepted after restart. Independent witness
loss and epoch 1→2 fence stop private reads while preserving head/hash. The
90-second lease naturally stops the enclave while parent/credentials remain
valid. Separate real 900-second witness STS expiry also refuses private reads
with parent alive, longer application lease and no clock change/renewal. This
is expiry/freshness refusal, not a captured AWS ExpiredToken response. Six boots;
final runtime connection counts are S3 2,936/1,500, DDB 778, KMS 36, native/RPC
0/0. The 116-object/93,238-byte ciphertext archive and all three strong heads
verify locally. Independent teardown confirms terminated host, absent tagged EBS/
buckets/table/roles/SG/SSH, six seven-day PendingDeletion keys and preserved
administrator policies. Combined operator/runtime calls: KMS 147, S3 4,690, DDB
813, within approved caps; billing is delayed, not a final invoice observation.
The invocation is closed; the 64-record gate is still unpassed. Actual final source,
image/client receipts and limitations belong in TRACKER; this evidence does not
close P23 or any financial/production gate.

[ADR 0025](../architecture/0025-concurrent-private-reads.md) and the
[C1–C5 contract](P23-REMEDIATION.md) record the approved single-journal accepted
read/publication and scheduling boundaries. Tracked journal/API/Runtime races,
the actual Node/Chrome joined grid and default-feature ARM package qualify the
bounded local slice at source `b38849cf93761f380e195a2731698d862ea84f4d`.
Exact commands, artifacts, measurements and limitations are in TRACKER. Synthetic
clock/witness/fault ports are not AWS/STS authority; quiet-window read recovery,
small fixture memory and full-history repair do not establish production liveness
or capacity. No historical EIF/PCR or native financial receipt qualifies this
changed application. Fresh hardware and G01–G05 evidence remain separate.

P23 initial promotion (2026-10-06):
[ADR 0023](../architecture/0023-live-observation-ports.md) connects current primary
account/position/history GET schemas to the existing private observation journal
and pooled request budget. Tracked tests use fake delivery and authenticated
loopback TLS, not live venue accounts or old experimental signers. W03/W04's
precision, broader-agent-power, original-withdrawal and actual-fee findings remain
qualification inputs, not shipping permission. The
[live runbook](../operations/live-qualification.md) separates prepared components,
missing ports, current documented limits and fresh G05 authority. No raw research,
key, RPC secret, signed wire, complete-cut assertion or hardware receipt is promoted.

| Contract | Owning implementation | Mandatory evidence |
| --- | --- | --- |
| B02–B03, F01–F04 | P02–P04 | V01–V04; per-user attribution, signed-basis and default counterexamples |
| B04–B05, F09, X01–X05 | P05–P09, P15–P16 | V01, V05, V09; atomic holds, replay, partial movement and payout races |
| F05–F08, F10 | P09–P10 | V03, V06, V10; no double absorption, repeated defaults/recoveries, path depletion |
| F11–F14 | P11–P12 | V07, V10; distinct forced events, RF quota/basis and late-close exceptions |
| B01, B08, X01–X04 | P13–P14, P18 | Native capability/precision map, authenticated attributed actions, unknown ACK reconciliation |
| B06, S01–S05 | P06, P18–P20 | Private transport, secret-safe storage, stale writer/quote/key rejection and hardware evidence |
| B07, R01–R04 | P17, P21 | V08–V09; funded final claims, shared counters, independent package delivery |
| R05, G01–G05 | P22–P24 | Joined fault suite, bounded live evidence, completeness/exit limits and explicit approvals |

## P17 final-claim contract promotion

2026-10-01: [ADR 0017](../architecture/0017-recovery-claims.md) promotes W05/W07's
strict final-root/normal-paid basis, independent operator activation, ordered odd-tree
membership and rollback/containment requirements into the existing Anchor 1.2 vault.
V08 now executes actual normal payment 20 and final claims 45/200, preserving total
lifetime payments 65/200; the 35 loss is an explicit local fixture, not venue evidence.
P17 replaces M2's 64-bit bitmap with permanent owner/config claim receipts and a
bounded 16-sibling verifier. Salts in tests are deterministic only. It preserves
false-total and duplicate-owner counterexamples: ordinary inclusion proves neither
complete liabilities nor solvency. No experimental account, live endpoint, key or
separate balance projection is promoted. P21 owns actual finalization/settlement and
independent encrypted package delivery; G01/G03/G05 remain qualification gates.
New test outcomes are recorded in TRACKER after verification, not inferred from M2.

## Source fingerprints

P18 promotion: [ADR 0018](../architecture/0018-private-api.md) extracts W06's
canonical capabilities, confidential transport seam, current-epoch authorization
before replay/query, durable operation IDs and secret-safe projections. Tracked
Rust tests use actual encrypted SQLite records and synthetic qualified policies;
eight independent-binding wire vectors and TypeScript tests cover every method.
No native credential, ignored research, RPC/venue call or live wallet is required.
Numerical admission and channel/attestation implementations remain qualified ports,
not promoted from a fake transport or illustrative hold amount. No external Cargo
version/feature changed; the local API dependency/lockfile policy is explicit.

SHA-256 of the selected local inputs when the baseline was extracted. Originals
remain unchanged; later source revisions require a dated promotion note, not silent
replacement of approval history. Fingerprints are provenance, not trusted execution.

| Local path under `work/` | SHA-256 |
| --- | --- |
| `specs/financial-model.md` | `d729b31a4d01aa3b25f1236d9e0f59e2b9e01cc85cb0ada6c5bce8deeedb476f` |
| `specs/financial-model/IMPLEMENTATION_HANDOFF.md` | `5a36ccf84c6cf8c2a15f6e5de5ad716d74febc36e3f23074e0d0e18c99542b93` |
| `specs/financial-model/proportional-allocation.md` | `2da738fed995904b9f3a94df25a226dabd413cc8b691c5f20a144ebc1c5335e5` |
| `venues/pacifica/financial-mapping.md` | `ec906a9c6e70f5c626dfe31b5560ab144f2260f8a2efb8bf241c97136361b341` |
| `venues/pacifica/experiments/funding-round-trip/live-result.md` | `3031ee75bc1c5b0e5fc07bcb882c254d1cc50a13cf6e7ee6fd1a6224ba15835b` |
| `experiments/private-recovery/CLOSEOUT.md` | `42072e3058379769f799fc64ebc8de725e1ceeda269717fa1b334a7ac8295106` |
| `specs/private-api-runtime.md` | `6ee0084666c8681f9b894a9d1a9c645997aa10d0bf352b257053f74798b03522` |
| `specs/verification-recovery.md` | `06d8ab815edaf1f583e4538bb236a3ad0c9c08d9f5e3bb56930df1827657f7e2` |
| `analysis/nitro-qualification.md` | `819297c008072b07599a75dbad233c70afce2185c88c451812e600dd50c69e71` |
| `analysis/d14-evidence-map.md` | `12ea2a2745bb06e0e3ef09e0107a70ceb4223aff140432cd3009b89e3cc0d639` |
| `specs/financial-execution.md` | `e1f0e8f2b9e08577a4eb005efe52cb29cf0250c72c231780f6892cd4638b7b06` |
| `FINANCIAL_CORE.md` | `248e4ffca40d5f2d38fc16e895fce8774e693ae326e76977b188ba800beb4e85` |
