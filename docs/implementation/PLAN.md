# Implementation plan

2026-09-30. Canonical scope and acceptance criteria; actual progress lives in
[TRACKER](TRACKER.md). Read [BASELINE](BASELINE.md) for approved policy and
[EVIDENCE](EVIDENCE.md) for research/prototype provenance and regression obligations.
P00 establishes this foundation; it does not implement or deploy the broker.

## How work advances

Each phase below is one reviewable PR by default, not an assertion about equal
effort. Prefer a complete, testable boundary over a broad skeleton. If a phase
outgrows that boundary, propose a documented split with stable new IDs, update both
documents and dependencies, and preserve completed evidence. Do not silently defer
an acceptance criterion or mark a partially completed parent closed.

The phases are in a safe default order, not a mandate to block independent work.
Listed dependencies are implementation dependencies; upper-stack dependencies may
be implemented but not merged. A branch may build on their tested commit, while its
tracker remains `in progress`. No phase can close ahead of its dependencies.

Global definition of done, in addition to each phase's checkboxes:

- Scope and public behavior match approved policy; new decisions are recorded.
- Needed contracts, synthetic fixtures and provenance are tracked; a fresh checkout
  runs its ordinary checks without ignored research, credentials or network access.
- Positive, negative, duplicate/conflict and relevant crash/race cases pass against
  the implementation, not only the reference study. Record commands, commit, date,
  environment, counts and limitations; never substitute a passing study for a port.
- Relevant independent review/CI findings are addressed or explicitly deferred with
  reason and owning phase. Financial/security-sensitive changes need focused review.
- Secrets/private evidence are excluded; behavior/docs/tracker are updated; required
  checks pass; PR is reviewed and merged. **Ready or green is not closed.**

When an unavailable native capability or material new policy blocks a phase,
record the blocker under `in progress`, disable the feature and move only to an
independent authorized phase. Do not manufacture guarantees to keep the stack moving.

## Workspace and dependency boundaries

P01 records the language/toolchain/layout ADR before building production components.
Recommended boundaries, not pre-created empty services:

| Boundary | Responsibility / allowed dependency |
| --- | --- |
| Domain + accounting kernel | Pure deterministic units, identities and transitions; no network, clock, wallet, database or venue SDK imports |
| Journal + state store | Atomic commits, replay, reservations, schema versions and encrypted persistence; kernel remains independently testable |
| Policy + execution controllers | Admission, forced events, claims and funds orchestration through typed ports; cannot bypass postings or authorization |
| Venue adapters | Native codecs, authenticated observations and narrowly scoped action execution; never decide customer loss allocation |
| Solana custody/recovery | Vault authority, epochs, final claims and payout protection; cannot assume on-chain access to native venue funds |
| Enclave + host boundary | Enclave runs private state/signing; host relays ciphertext and permitted metadata, not private policy decisions |
| API + client SDK | Customer capabilities, durable operation semantics and verified private transport; not a raw pooled-account proxy |
| Tests + fixtures | Sanitized conformance vectors, fake venue/chain/time ports, crash harnesses; live qualification is opt-in and isolated |

Do not copy all CJS study modules into production. First import their requirements
and golden vectors; then implement one joined state. Promote precise research
material just in time, with source date/hash, license check for third-party content,
and an explicit distinction between a tested example and a production assumption.

## PR and stack workflow

- Work in `cinder-tee`; TEE trunk is `product/tee-v1`, not legacy `main`.
- P00 branch: `tee/p00-implementation-foundation`. Later branches use
  `tee/pNN-short-name`. One phase per PR unless an explicitly recorded split.
- Use a shallow native GitHub stack, normally at most 2–3 open layers. First base is
  `product/tee-v1`; a dependent PR bases on the prior layer's branch. Independent
  phases can have separate stacks after shared dependencies land.
  Run-specific exception, 2026-09-30: user explicitly authorized successive stacked
  P06–P14 PRs while away. Keep each phase independently reviewable and verified;
  do not merge any layer or start P15. Stop for material new policy/security choices
  or external permissions. This supersedes the normal shallow-depth preference,
  not any acceptance criterion or production gate.
- Initial local setup: `gh stack init --base product/tee-v1 <branch>`. Inspect
  `gh stack view` before `gh stack submit`; submission can push every stack branch.
  Review the current CLI help before publishing. Do not accidentally open against
  `main` or batch-mark unfinished upper layers ready.
- Merge bottom-up only after user authorization and checks. Inspect changed base
  commits, update/retest dependent branches and their evidence. Do not force-push
  unrelated work or merge an unreviewed whole stack from its top layer.
- A new TEE trunk does not automatically inherit legacy branch protections. Verify
  its actual required checks/merge rules; report missing protections rather than
  asserting they exist or changing repository governance without approval.
- Tracker PR fields carry the actual URL and merge commit once known. The next
  authorized branch records a completed predecessor's merge; do not forge a merge
  hash inside an unmerged PR to satisfy the status rules.

Native stacks are a public-preview workflow checked on 2026-09-30; same-repository
branches and non-default trunks are supported. References:
[GitHub stack behavior](https://docs.github.com/en/pull-requests/get-started/about-stacked-prs),
[CLI commands](https://docs.github.com/en/pull-requests/reference/stacked-prs-cli-commands).

## Phases

Each `Evidence` entry names local research IDs in EVIDENCE and the required tracked
test output. Those outputs are **deliverables**, not commands or files claimed to
exist already. P01 establishes the actual build/test commands; every owning phase
records its exact command in TRACKER rather than leaving a fictitious placeholder.

<a id="p00"></a>
## P00 — Tracked implementation foundation

Depends on: none.

Deliver: this plan, tracker, contributor guide, sanitized baseline, research reuse
map and offline documentation checks. Keep local research and legacy code intact.
Evidence: W01–W09; documentation validator/self-tests and fresh offline study run.

- [x] Plan/tracker phase IDs, dependencies and status vocabulary agree; tracked links resolve.
- [x] Approved versus open policy and prototype versus production evidence are explicit.
- [x] Fresh checkout checks need no `work/`; P01 has a concrete handoff; PR targets TEE trunk.

<a id="p01"></a>
## P01 — Production workspace and test harness

Depends on: P00.

Deliver: ADR for language, pinned supported toolchains, package ownership and error
boundaries; smallest compilable kernel/store/adapter-port workspace. Prefer a
memory-safe enclave-suitable kernel, but compare existing evidence before selecting
libraries. Add lockfiles, formatting/lint/unit/property-test commands and offline CI.
Do not scaffold a website or deploy programs. Evidence: W01, W06, W08; reproducible
clean build and documented local/CI parity, including Linux target requirements.
Address the repository integration follow-up recorded in TRACKER: propose scoped
TEE review/protection and legacy-preview separation, obtaining any required
repository/deployment-setting authority. Do not reintroduce web code as a CI fix.

- [x] Fresh checkout builds/tests with pinned dependencies and no venue/RPC secrets.
- [x] Kernel cannot import I/O/venue packages; fake clock/venue/store ports support fault injection.
- [x] Linux/SBF/enclave checks are mapped to owning phases, not falsely reported as already qualified.

<a id="p02"></a>
## P02 — Canonical financial types and event identities

Depends on: P01.

Deliver: typed domain/account/market/asset IDs, integer quantity/price/basis codecs,
checked arithmetic, domain-separated canonical bytes and policy/schema versions.
Keep user request, signed attempt and economic event IDs distinct. Evidence: W01,
W03; port fixture V02 and boundary/fuzz vectors to the chosen production language.

- [x] Overflow, overprecision, bad signs/units, unknown versions and ambiguous encodings reject without side effects.
- [x] Negative basis/division/remainders round as specified; no float parsing path exists.
- [x] Same ID/different payload conflicts; domains and source namespaces cannot replay into one another.

<a id="p03"></a>
## P03 — Unified ledger, positions and ownership

Depends on: P02.

Deliver: one pure transition state with customer/house/suspense ownership, cash versus
physical assets, basis/PnL and explicit shortfall diagnostics. All external changes
have named provenance; no plugin gets a second balance sheet. Evidence: W01–W02;
V01–V03, wrong-owner and opposing-position/default conformance vectors.
Also deliver the user-requested [full system architecture](../architecture.md),
including component responsibilities, trust boundaries, end-to-end operation and
the distinction between implemented and planned phases.

- [x] Opening, scaling, partial/full closes and reversals preserve exact basis and bridge identities.
- [x] Net-flat default remains visibly underbacked; negative customer claims are not spendable assets.
- [x] Wrong-user attribution fails even when aggregate exposure/equity matches; projections cannot double-count assets.

<a id="p04"></a>
## P04 — Funding, fees and evidence reconciliation

Depends on: P03.

Deliver: qualified funding boundaries/accrual/settlement, fee/rebate ownership,
source normalization ports, explicit mismatch aging and restricted modes. Ingest
actual losses despite limit breaches. Evidence: W01, W03; V04, gross/net funding,
entry-time/partial-close/rounding and REST/WS duplicate vectors.

- [x] Funding and fees post once with per-user attribution; inclusive native PnL cannot double-charge fees.
- [x] Missing/corrupt sources create a named unresolved condition, never fabricated zero or house profit.
- [x] Stale marks and unexplained precision differences block dependent admission, not raw evidence retention.

Implemented in-memory evidence gate and retained normalized observations are
specified in [ADR 0004](../architecture/0004-funding-reconciliation.md). P09 composes
the gate into full admission; P05/P13 own durable/raw-wire retention and native
qualification. These checked technical criteria are not PR review/merge closure.

<a id="p05"></a>
## P05 — Durable atomic journal and replay

Depends on: P04.

Deliver: versioned persistent journal, atomic postings/consumption/holds, immutable
evidence and deterministic rebuild. Serialize economic state changes with bounded
CAS/retry semantics. Persist exact attempted action before capability exposure.
Evidence: W01, W04–W05; process-kill tests at each commit/exposure boundary.

- [x] Restart yields the same state and consumed IDs; torn commit cannot credit without deduplication or vice versa.
- [x] Concurrent requests cannot spend the same entitlement/reserve; duplicate/conflicting receipts stay distinct.
- [x] Schema migration, audit replay and secret-safe evidence retention are tested; disk/commit failure stops dispatch.

Technical evidence: [ADR 0005](../architecture/0005-durable-journal.md) and the
journal tests. Secret-safe retention means opaque storage and redacted diagnostics
behind a mandatory protection interface; it does not claim production encryption
or hostile-host freshness. Those remain P06. Flat-capacity/one-attempt holds are
the bounded P05 infrastructure slice, not full P07/P09 authorization/risk policy.
Review and merge remain open in TRACKER.

<a id="p06"></a>
## P06 — Encrypted durability and writer epochs

Depends on: P05.

Deliver: AEAD record/snapshot format, authenticated chain/head, replication and
freshness-witness interfaces, restore procedure and writer fencing. A local fake
witness tests the contract, not independence; production witness choice is G03.
Evidence: W05–W06; corruption, stale valid ciphertext and split-writer kill tests.

- [x] Acknowledgement waits for specified durable commit; one replica loss restores, insufficient valid evidence fails closed.
- [x] Restoring an old authentic snapshot cannot silently roll back claims/paid counters or revive an old writer.
- [x] Host-visible files/logs exclude private plaintext; witness/key/availability assumptions are documented and testable.

Implementation and trusted-port limits: [ADR 0006](../architecture/0006-encrypted-durability.md).
Witness independence, actual Nitro key release and native credential fencing are
not established by local files/process tests; G03/P20 remain explicit gates.

<a id="p07"></a>
## P07 — Order intents and shared reservations

Depends on: P05.

Deliver: authorized intent → durable acceptance → attempted dispatch → qualified
outcome/reconciliation state machine; exact partial/cancel/unknown accounting.
Join order, transfer, payout and capital commitments in one reservation API.
Evidence: W01, W03; partial/cancel races, independent pending buy/sell outcomes.

- [x] ACK is not fill; cancel ACK is not complete history; unknown outcomes retain holds.
- [x] No new signed capability escapes before durable reservation and attempt record.
- [x] Changed request under an existing operation ID rejects; a real out-of-policy fill is booked and contained.

<a id="p08"></a>
## P08 — Funds and payout state joined to trading

Depends on: P07.

Deliver: locations, partial debit/arrival/return/impairment, receipt suspense,
exposure holds and normal payout queue in the same state as positions. This
replaces the flat-only study limitation; actual on-chain/venue sends remain ports.
Evidence: W01, W04–W05; V01/V05, all 24 receipt permutations, fills versus payouts.

- [x] Final payment discharges its owner's claim once; source debit/transfer arrival creates no second deposit credit.
- [x] Late or out-of-order receipts reconcile after freezes; overpayment is a house obligation, not another customer's loss.
- [x] Free-collateral withdrawal with open positions uses shared holds; FIFO/partial-consent rules and total debit fees are explicit.

<a id="p09"></a>
## P09 — Joined admission, margin and capital envelope

Depends on: P08.

Deliver: selectable leverage under versioned market caps, user gross/native net
margin, pending-outcome evaluation, peak-path capital depletion, concentration and
location/deadline liquidity. Synthetic profiles exercise gates; no live calibration.
Evidence: W01, W09; every-prefix shocks/recoveries, cap-change and reduce-only cases.

- [x] One atomic version/cut accounts for all existing commitments and proposed reachable outcomes.
- [x] Insolvency, illiquidity, margin breach and capital-target breach remain distinguishable; external shocks still ingest.
- [x] Unbounded/missing data restricts new risk; resource-location reuse and imagined future revenues cannot pass admission.

<a id="p10"></a>
## P10 — Protection claims and repeated deficit episodes

Depends on: P09.

Deliver: house protection designation, typed coverage policies, recognition,
commitment, absorption, payment, debt recovery and capital withdrawal restrictions.
Remove the study's single-episode limitation. Live coverage/priority remains G02;
unsupported shortfalls preserve claims and halt preferential payout.
Evidence: W01, W09; V06, repeated/adaptive loss, overpayment and recovery vectors.

- [x] Recognition/absorption/payment cannot consume the same free capital twice or create phantom assets.
- [x] Multiple episodes and later recoveries replenish the correct resource once; excess returns to the customer.
- [x] Capital providers cannot withdraw ahead of incurred obligations; coordinated insurance extraction is stress-tested and bounded or disabled.

The bounded implementation contract is [ADR 0010](../architecture/0010-protection-claims.md).
G02 production coverage/priority and live capital calibration remain open; native
house capital cash withdrawals are disabled, not misrepresented as customer payouts.

<a id="p11"></a>
## P11 — Customer liquidation and late-close exceptions

Depends on: P10.

Deliver: deterministic liquidation controller with exit-capacity/price/time bounds,
partial execution and exception-house ownership/unwind. Distinguish private close
from native pooled reduce-only. No real market calibration or speculative trades.
Evidence: W01, W03, W09; gaps, reduced depth, late private close after forced reduction.

- [x] Underfilled liquidation retains residual risk and obligations; timeout never manufactures a flat account.
- [x] Excess late close goes only to approved funded exception accounting, never silently reverses the customer.
- [x] House unwind failures, fees and repeated losses feed admission/claims; exhausted budget does not erase actual fills.

[ADR 0011](../architecture/0011-liquidation-exceptions.md) defines the funded
exit envelope, split-fill residue, local abandonment and containment limits.

<a id="p12"></a>
## P12 — Native ADL and bounded RF1/RF2 restoration

Depends on: P11.

Deliver: qualified forced-event cut, proportional reduction, scalable monotone quota
schedule, original-basis restoration and risk-unit policy. Preserve different event
families. Native account splitting is enabled only with G01 evidence; one tested
risk unit is sufficient initially. Evidence: W01, W03, W09; V07 and allocation bounds.

- [x] Actual restoration, ADL fee refund and signed basis implement RF1; no gap funding or promised fill on an unexecuted quantity.
- [x] Each prefix respects RF2 quotas and composed bounds; no per-atomic-lot unbounded loop, operator reweighting or historical-manifest bypass.
- [x] Changed close intent, partial/late fill, prior capital reservation and post-submission shock preserve already owed economics.

<a id="p13"></a>
## P13 — Pacifica observations and capability profile

Depends on: P04, P05.

Deliver: tracked native mapping, lossless codecs, replayable sanitized evidence,
account/order/fill/funding normalization and explicit capability states. Recheck
current primary docs; use fake transport first. Do not inherit BULK fields or assume
Last ID is a global complete journal. Evidence: W03; VM-01–VM-09 qualification map.

- [x] REST/WS overlap posts one economic fill; reversal legs remain separate and empty position snapshots mean flat.
- [x] Precision/PnL/funding gaps remain named and disabled for dependent use; the observed fee discrepancy is not auto-written off.
- [x] Pagination, gaps, corrections, stale snapshots and replay cuts have tested containment; raw evidence retains provenance.
- [x] Qualified raw-input resolution binds the original CommitId/input ordinal and exact evidence fingerprint; normalized effects or an authenticated no-effect finding commit with resolution once. Wrong scope, conflicting/repeated resolution, restart and unrelated unresolved inputs cannot clear the exposure gate. No generic administrative reset is available.

<a id="p14"></a>
## P14 — Pacifica signed execution boundary

Depends on: P07, P13.

Deliver: bounded GTC/ALO/IOC and attributed cancel actions, durable signing/attempt
matching, auth capability checks, rolling API-credit limits and cleanup reserve.
No blanket modify/DCA/conditional-order support or pooled cancel-all exposure.
Evidence: W03–W04; captured sanitized signing vectors and fake-server failure tests.

- [ ] Signed domain/expiry/account/attempt matches the admitted action; replay and key-epoch changes fail safely.
- [ ] Hidden ACK, 429, delayed response and cancel race reconcile before retry; cleanup capacity cannot be spent on new orders.
- [ ] Native fund-moving permissions are explicitly contained by the custody design; no false trading-only credential claim.

<a id="p15"></a>
## P15 — Solana vault and normal-path authorization

Depends on: P02, P08.

Deliver: production account schema, program-owned vault, deposit attribution,
allowlisted funding release, role/epoch checks and payout counters. Prefer one
cohesive program unless a separate trust/upgrade boundary justifies another; record
the decision. Port M1 invariants, not disposable deployment identities.
Evidence: W04–W05; local SBF tests against actual program instruction boundaries.

- [ ] Correct owner/mint/program/PDA/recipient/signer checks; wrong domain/account substitutions and duplicate receipts reject.
- [ ] Funding release/payout bounds and epoch/counter changes are atomic even if downstream token transfer fails.
- [ ] Upgrade/rotation and normal/recovery authority boundaries are specified; deployment stays disabled without G03/G05.

<a id="p16"></a>
## P16 — Funding coordinator and native round trip

Depends on: P08, P14, P15.

Deliver: vault → approved broker route → native margin → return → customer
controller, with one common journal and bounded bootstrap/config transitions.
No automatic exposure to lending or unresolved native agent rights. Offline first;
live test requires a current approved manifest. Evidence: W04 and V01/V05.

- [ ] Marginal working collateral is funded once; unconfirmed venue credit cannot admit a trade.
- [ ] Kill/restart after withdrawal POST exposure reconciles the original attempt without double withdrawal or payout.
- [ ] Fees, partial/failed returns, bootstrap exceptions and cleanup residuals are attributed and reported, not hidden by retries.

<a id="p17"></a>
## P17 — Recovery claims and Solana payout program

Depends on: P06, P08, P15.

Deliver: salted/domain-bound claim format, immutable final epoch, activation
authority and shared normal/recovery paid-counter semantics in the justified
program topology. Ordinary snapshots are not active recovery roots.
Evidence: W05, W07; V08 plus local SBF adversarial claim tests.

- [ ] Heartbeat alone cannot activate; wrong recipient/epoch/domain/mint, duplicate proof and root reset attacks reject.
- [ ] Prior ordinary payments and reservations cannot be paid again; failed downstream payout rolls back claim consumption.
- [ ] Underfunded/uncertain activation fails; late deposits and post-activation impairment retain claims with safe containment.

<a id="p18"></a>
## P18 — Private API and client operation semantics

Depends on: P07, P08, P14.

Deliver: customer/agent grants, canonical auth, private views, operation/query
semantics and thin SDK. Use local encrypted-transport interfaces pending P19;
do not expose a public plaintext fallback. Evidence: W06; auth/replay/ownership tests.

- [ ] Authenticate before dedupe/private lookup; one customer's order or operation cannot be queried/cancelled by another.
- [ ] Grants enforce methods, domain, expiry, limits and epoch; native authority is never exposed to customers.
- [ ] ACK/fill/unknown and partial/fee semantics are explicit in SDK results; logging/errors do not leak private payloads.

<a id="p19"></a>
## P19 — Attested transport and client release verification

Depends on: P06, P18.

Deliver: reviewed channel protocol/crypto-library ADR, attestation verification,
fresh session binding and independently verifiable client releases. Earlier Noise
experiments are candidates, not approval of a production protocol. Qualify local
positive/negative quote fixtures; no hardware claim from mocks.
Evidence: W06–W08; replay/measurement/key substitution and host-observation tests.

- [ ] AWS chain, approved measurements, freshness and session-key binding all validate before private exchange.
- [ ] Downgrade, replay, wrong release and error/stream paths cannot expose private plaintext to parent/relay.
- [ ] Client distribution threat and key rotation are documented; logged/host-visible metadata is explicitly bounded.

<a id="p20"></a>
## P20 — Nitro runtime and key/storage qualification

Depends on: P06, P14, P19.

Deliver: reproducible enclave packaging, authenticated egress, key release,
measurement policy, sealed state restore and independent freshness integration.
Local packaging first; actual AWS launch/spend needs G03/G05 approval. Do not close
hardware qualification with a mocked attestation document.
Evidence: W06, W08; real attested runtime evidence and failure/revocation rehearsal.

- [ ] Venue TLS/auth terminates inside enclave; parent cannot read payloads or replace upstream responses undetected.
- [ ] Wrong measurement/debug/replayed release denies keys; old loaded keys and outstanding capabilities are fenced, not merely KMS-disabled.
- [ ] Accepted encrypted state survives qualified failure/restore; witness loss and split writer fail safely, with costs/limits recorded.

<a id="p21"></a>
## P21 — Integrated recovery and independent claim delivery

Depends on: P10, P12, P16, P17, P19.

Deliver: controlled cutover/reconciliation, venue unwind/returns, final root/backing
verification, operator activation, encrypted proof-package publication and standalone
claimant. Pending claims/fees/late actions all use the same financial journal.
Evidence: W04–W08; live-like offline outage rehearsal. Use explicit fake hardware/
witness ports here; actual P20 runtime and live qualification join in P23. Do not
block offline recovery engineering on AWS access or label it hardware-qualified.

- [ ] No ordinary writer/payout/escaped capability can race an activated root; unresolved native outcomes prevent unsafe activation.
- [ ] Claimants obtain and verify kits without the ordinary API; package loss, stale package and privacy are tested.
- [ ] Final funded claims match remaining entitlements; unavailable venue assets do not become payout cash or disappear as liabilities.

<a id="p22"></a>
## P22 — Joined offline adversarial acceptance suite

Depends on: P09, P12, P16, P17, P18, P21.

Deliver: deterministic end-to-end fake venue/chain/time harness driving production
components; differential/reference conformance, bounded state exploration, fuzz,
reverse stress and process crash tests. Bring failures back to owning phases.
Evidence: all W IDs and V01–V10; fresh-checkout repeatable seed manifest.

- [ ] Opposing winners/defaults, ADL gaps, funding boundaries, house exhaustion, inaccessible cash and payout races preserve attribution/claims.
- [ ] Duplicate/conflicting/late events, restart, rollback and malicious ordering cannot double-spend or silently omit loss.
- [ ] Test bounds, counterexamples and unresolved proofs are explicit; measurements meet declared load/resource budgets without weakening economics.

<a id="p23"></a>
## P23 — Bounded devnet/testnet integration qualification

Depends on: P20, P22.

Deliver: current G05-approved account/funding/environment/cleanup manifest, tagged
release tests using the actual program, adapter, enclave and recovery path. Reuse
M1/M2 scenarios but obtain new implementation evidence. No mainnet inference.
Evidence: W03–W08; sanitized receipts and failure/cleanup reports linked to commit.

- [ ] Deposit, trade lifecycle, funding observations, ordinary return/payout and recovery execute or expose a named blocking capability.
- [ ] Real process/network faults and credit exhaustion reconcile safely; replay is not reported as a newly observed venue event.
- [ ] All positions/orders/funds and test costs reconcile; no real customer/mainnet assets; private evidence stays out of Git.

<a id="p24"></a>
## P24 — Release safety case and customer-funds gates

Depends on: P23.

Deliver: requirements-to-test matrix, independent financial/security review,
policy/coverage/capital calibration, G01–G04 sign-offs, D14 verification scope,
runbooks, monitoring and staged-release proposal. This phase produces a release
decision package; completing it does not itself authorize mainnet deployment.
Evidence: all W IDs; measured venue/recovery limits, review disposition and approvals.

- [ ] Actual native capabilities, thresholds, claim priority and key/upgrade/recovery governance have explicit reviewable sign-off.
- [ ] User-visible privacy, fees, risk and exit claims match verified capabilities and state the operator/venue/data assumptions.
- [ ] Operational alerts/drills, dependency/supply-chain review and remaining-risk register are complete; deployment remains a separate permission.

## Required handoff after every work session

Update the owning TRACKER entry with implemented paths, checked acceptance items,
exact test commands/results/environment/commit, PR/review state, unexpected work,
unresolved decisions and one concrete next action. Record skipped/failed checks
honestly. Never use only a chat summary as the handoff. Technical acceptance may
be complete while status stays `in progress` pending review/merge.
