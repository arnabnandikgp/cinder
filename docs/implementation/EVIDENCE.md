# Research evidence and promotion register

Snapshot: 2026-09-30. This is a curated map, not an archive upload. The original
`work/` remains local and ignored. Paths below are intentionally code text, not
broken public links. [BASELINE](BASELINE.md) and [CONFORMANCE](CONFORMANCE.md) carry
the initial tracked contracts; [PLAN](PLAN.md) assigns the remaining promotion.

## How to reuse the research

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

## Requirement-to-phase traceability

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

## Source fingerprints

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
