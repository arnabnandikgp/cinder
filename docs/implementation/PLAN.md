# Implementation plan

Updated 2026-10-03. Canonical scope and acceptance criteria; actual progress lives in
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

<a id="application-assembly-and-the-integration-test-ladder"></a>
## Application assembly and the integration test ladder

Scope clarification approved 2026-10-01: implemented libraries are not a runnable
backend. P19 must introduce a minimal **parent relay + confidential application
service** and an SDK-driven local process test. P20 assembles and qualifies that
application on actual Nitro. The parent accepts connections and relays ciphertext;
private decryption, authentication, accounting, admission and signing stay inside
the confidential service. Record server/async-runtime choices with the channel ADR;
no HTTP framework or concrete deployment is selected by this plan update.

| Evidence level | Owning phase | Required boundary / limitation |
| --- | --- | --- |
| Unit and component integration | P02–P18 and every later change | Existing financial/journal/API/adapter tests and actual local SBF instructions; not a complete deployed workflow |
| Local executable service slice | P19 | SDK → real relay/service processes → signed request → protected journal → private response, with explicit attestation/infrastructure fixtures; not Nitro evidence |
| Actual confidential runtime | P20 | Same service packaged into an enclave, authenticated egress, key release and fresh encrypted restore; actual AWS use requires G03/G05 |
| Customer HTTP/WebSocket surface | P21A | Reviewed confidential browser/Node transport over the same authorization/journal, with bounded private streams; no native compatibility or production claim |
| Implemented public reference | P21B | Mintlify guides/method pages and checked examples match actual handlers and environment availability; publishing is not customer-funds release |
| Independent recovery workflow | P21 | Fence/reconcile/return/finalize/activate/claim delivery; fake hardware allowed, actual claim instruction evidence remains distinct |
| Joined offline workflows | P22 | Production SDK/service/controllers/adapters with deterministic external ports, recorded failure seeds and per-boundary accounting assertions; no live venue/RPC in routine CI |
| Actual end-to-end qualification | P23 | Tagged implementation on approved Nitro + Solana devnet + venue test environment, current capability evidence, receipts and cleanup; no mainnet inference |

Start vertical integration in P19/P20, not after all phases finish. Carry the same
workflow manifest and expected financial outcomes into P22/P23; external receipts
and capability checks replace simulation assumptions, not the financial engine.
A failed or unsupported capability stays named and disabled. P24 reviews the
evidence and remaining release gates; it is not a substitute for runnable services.

Use the thin SDK/CLI and explicitly preconfigured disposable accounts for initial
application tests. Scope and subsequent sequencing approved 2026-10-03: implement
browser-compatible confidential HTTP/WebSocket operations in P21A, then author
the public Mintlify reference against that implementation in P21B, then complete
P21 recovery integration. P21 is not a prerequisite for ordinary API transport.
Do not publish
planned endpoints as working ones. P22 must exercise the implemented transports
and P23 must qualify the changed application, not inherit P20's old measured image.
These suffixes preserve existing phase IDs; they are explicit new scope, not a
claim that P19/P20 already shipped HTTP or streaming. Browser attestation/session
security needs its own reviewed ADR before coding the transport.
Self-service private-account onboarding, full browser-agent management UX, public
market-data UI and a trading terminal remain separate product-integration follow-ups.
P21A's preconfigured browser/SDK test client is not a completed terminal or account
creation flow. Do not import the Phoenix website or block backend/recovery tests
on a frontend. HTTP/WebSocket and the Node profile share one financial journal.

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
- On publication, review/fix push and merge, refresh the current handoff and owning
  phase with exact checked head, PR base/URL, CI/review disposition and next action.
  For a stack, the tip must summarize its open predecessors as well. Preserve dated
  earlier evidence; pending checks at a new head are not inherited green results.

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

- [x] Signed domain/expiry/account/attempt matches the admitted action; replay and key-epoch changes fail safely. Native signature omissions are explicit and covered by local envelope/key-scope requirements, not misrepresented as native enforcement.
- [x] Hidden ACK, 429, delayed response and cancel race reconcile before retry; cleanup capacity cannot be spent on new orders.
- [x] Native fund-moving permissions are explicitly contained by the custody design; no false trading-only credential claim. P14 implements the narrow signer allowlist; onward vault custody and attested key release remain P15/P16/P20 gates.

<a id="p15"></a>
## P15 — Solana vault and normal-path authorization

Depends on: P02, P08.

Deliver: production account schema, program-owned vault, deposit attribution,
allowlisted funding release, role/epoch checks and payout counters. Prefer one
cohesive program unless a separate trust/upgrade boundary justifies another; record
the decision. Port M1 invariants, not disposable deployment identities.
Evidence: W04–W05; local SBF tests against actual program instruction boundaries.

- [x] Correct owner/mint/program/PDA/recipient/signer checks; wrong domain/account substitutions and duplicate receipts reject.
- [x] Funding release/payout bounds and epoch/counter changes are atomic even if downstream token transfer fails.
- [x] Upgrade/rotation and normal/recovery authority boundaries are specified; deployment stays disabled without G03/G05.

<a id="p16"></a>
## P16 — Funding coordinator and native round trip

Depends on: P08, P14, P15.

Deliver: vault → approved broker route → native margin → return → customer
controller, with one common journal and bounded bootstrap/config transitions.
No automatic exposure to lending or unresolved native agent rights. Offline first;
live test requires a current approved manifest. Evidence: W04 and V01/V05.

- [x] Marginal working collateral is funded once; unconfirmed venue credit cannot admit a trade.
- [x] Kill/restart after withdrawal POST exposure reconciles the original attempt without double withdrawal or payout.
- [x] Fees, partial/failed returns, bootstrap exceptions and cleanup residuals are attributed and reported, not hidden by retries.

Offline implementation evidence is in ADR 0016 and TRACKER. Historical bootstrap
exception permission is deliberately not a default; missing/lending-active setup
refuses admission. Native completeness/RPC authentication, production key custody
and any live manifest remain separate G01/G03/G05 qualification gates.

<a id="p17"></a>
## P17 — Recovery claims and Solana payout program

Depends on: P06, P08, P15.

Deliver: salted/domain-bound claim format, immutable final epoch, activation
authority and shared normal/recovery paid-counter semantics in the justified
program topology. Ordinary snapshots are not active recovery roots.
Evidence: W05, W07; V08 plus local SBF adversarial claim tests.

- [x] Heartbeat alone cannot activate; wrong recipient/epoch/domain/mint, duplicate proof and root reset attacks reject.
- [x] Prior ordinary payments and reservations cannot be paid again; failed downstream payout rolls back claim consumption.
- [x] Underfunded/uncertain activation fails; late deposits and post-activation impairment retain claims with safe containment.

P17 evidence: [ADR 0017](../architecture/0017-recovery-claims.md), pure recovery
codec tests and actual signed offline SBF tests, including V08. The program rejects
explicitly unresolved qualification assertions and shares actual lifetime payout
counters. P21 still owns deriving/independently qualifying those assertions from
the authoritative ledger and venue history; membership/zero fields are not a
complete-liability or solvency proof. No live integration/deployment is implied.

<a id="p18"></a>
## P18 — Private API and client operation semantics

Depends on: P07, P08, P14.

Deliver: customer/agent grants, canonical auth, private views, operation/query
semantics and thin SDK. Use local encrypted-transport interfaces pending P19;
do not expose a public plaintext fallback. Evidence: W06; auth/replay/ownership tests.

- [x] Authenticate before dedupe/private lookup; one customer's order or operation cannot be queried/cancelled by another.
- [x] Grants enforce methods, domain, expiry, limits and epoch; native authority is never exposed to customers.
- [x] ACK/fill/unknown and partial/fee semantics are explicit in SDK results; logging/errors do not leak private payloads.

<a id="p19"></a>
## P19 — Attested transport, runnable service and client release verification

Depends on: P06, P18.

Deliver: reviewed channel/crypto/server-runtime ADR, attestation verification,
fresh session binding, independently verifiable clients and a minimal runnable
parent relay/confidential service wired to P18 and the protected journal. Document
actual entry points and local startup/test commands when implemented; do not ship
only another injected transport interface. Earlier Noise experiments are candidates,
not approval of a production protocol. Qualify local positive/negative quote
fixtures; no hardware claim from mocks. Evidence: W06–W08; SDK-driven process tests,
replay/measurement/key substitution and hostile relay/host-observation tests.

- [x] AWS chain, approved measurements, freshness and session-key binding all validate before private exchange. The AWS-root production path is implemented; positive current-session evidence is synthetic, not NSM hardware qualification.
- [x] Downgrade, replay, wrong release and error/stream paths cannot expose private plaintext to parent/relay. This profile supplies bounded snapshots, not network subscriptions.
- [x] Client distribution threat and key rotation are documented; logged/host-visible metadata is explicitly bounded.
- [x] Thin SDK reaches actual relay/service processes; an authenticated intent commits durably and its private response/query survives disconnect/restart without a second native attempt. Native venue I/O remains an explicit fixture in this slice, including a lost committed reply.
- [x] Connection/request/stream bounds, deadlines, backpressure, shutdown and failure behaviour are tested. Designated parent logs/errors/replica storage receive no private plaintext; test attestation/key providers cannot silently enable a production service. Local fixture process memory/stdin are not Nitro isolation evidence.

<a id="p20"></a>
## P20 — Nitro runtime and key/storage qualification

Depends on: P06, P14, P16, P19.

Deliver: compose the API, journal/financial controllers and venue/funding adapters
into a runnable confidential application; package it reproducibly with the parent
relay, authenticated egress, key release, measurement policy, sealed state restore
and independent freshness integration. Wire real network/chain/storage ports only
under explicit qualified capability/configuration gates. Include bounded event and
dispatch loops, startup/restart/shutdown and secret-safe health/runbook contracts;
do not introduce a second ledger or plaintext host policy/signing service.
Local assembly/packaging first; actual AWS launch/spend needs G03/G05 approval.
Do not close hardware qualification with a mocked attestation document. A working
Nitro service is not already a live venue workflow; that remains P23.
Evidence: W06, W08; real attested runtime evidence and failure/revocation rehearsal.

Pre-hardware software handoff is separate from the four hardware criteria below:
consumed manifest and actual loaded-policy comparison; default-feature executable
over the real private API/journal/controllers; recipient-only separated KMS roles;
closed TLS/SigV4 cloud replicas and independent witness interface; bounded lease/
supervisor/fencing; reproducible local package plus preparation/operations runbook.
Use [ADR 0020](../architecture/0020-nitro-runtime.md) and the
[qualification runbook](../operations/nitro-qualification.md). Current qualification
build rejects native trading/funding/read activation, rather than fabricating
unqualified observations or chain authority. P22/P23 own those joined workflows.
Final disposable resource identities, EIF/PCRs and KMS/SDK measurement approval
are hardware-session outputs. The pre-hardware stop was lifted by the user on
2026-10-02 for the existing $5 disposable session. Scoped managed policies match
the prepared versions; quota/pricing/AMI checks and launch dry-run pass. Actual
launch initially failed Free account-plan eligibility, not IAM. After the user's
explicit Paid-plan upgrade, one disposable c6g.large ran the actual non-debug
application. [Hardware receipts](../operations/nitro-hardware-2026-10-02.md) record
real KMS/NSM/SDK, restore and failure evidence, three compatibility fixes, cleanup
status and the historical remaining matrix. The
[follow-up receipts](../operations/nitro-hardware-followup-2026-10-02.md) complete
the bounded P20 matrix, including actual recipient/upstream negatives, concurrent
CAS, orphan/lost-ack recovery, debug refusal, natural loaded-lease and actual
credential expiry/restart refusal. Native capabilities remain disabled: this is
runtime qualification, not an approved production topology or P23 trading test.
P20 is closed after the reviewed, green-head merge recorded in TRACKER. Historical
hardware receipts remain exact-source; later source changes qualify separately.

- [x] Venue TLS/auth terminates inside enclave; parent cannot read payloads or replace upstream responses undetected. Actual paper-origin TLS-only negatives pass; native credentials/actions remain gated off until P23.
- [x] Wrong measurement/debug/replayed release denies keys; old loaded keys and outstanding capabilities are fenced, not merely KMS-disabled. Actual retained-head epoch, natural lease and issued STS expiry/restart checks pass; no native capability was enabled or escaped in this qualification.
- [x] Accepted encrypted state survives qualified failure/restore; witness loss and split writer fail safely, with costs/limits recorded. Real replicas/CAS participate; lost-ack injection is inside the test enclave, not a physical packet drop.
- [x] A tagged enclave executable and parent relay run the same SDK operation contract; controller scheduling and persisted unknown attempts cannot bypass qualification or auto-retry after restart. Local fixture acceptance and actual hardware evidence are recorded separately. Exact-source ELF/EIF/PCR releases are identified in both receipts; live native controller workflows remain P22/P23 scope.

<a id="p21a"></a>
## P21A — Confidential HTTP and WebSocket API

Depends on: P18, P19, P20.

Deliver: a versioned customer HTTP/WebSocket contract and browser/Node SDK transport
over the existing eight P18 commands, with correlated command responses and private
operation/account subscriptions derived from committed journal state. Finalize a
reviewed ADR for browser-verifiable enclave attestation, confidential session/key
binding, signature encoding, replay/expiry and ingress topology before implementation.
Never terminate readable private requests at the parent or replace attestation with
ordinary web-PKI or an operator approval flag. Preserve financial IDs, permissions,
fee/price bounds and payout-recipient restrictions; no second authority/replay store.
Define finite message/queue/session limits, revocation checks, account-local stream
ordering and snapshot/gap recovery without exposing global journal sequence. HTTP
and WebSocket are alternate ingress paths, not independent financial engines.
Scope is existing commands, approved bounded reads and private updates; no batch/modify/trigger,
public market-data feeds, self-service onboarding, new payout recipient or automatic
uncertain-order retry. Stop for review if the proposed cryptographic/authority
profile changes an approved security guarantee. No live customer release is implied.
The [approved API scope](API_SCOPE.md) distinguishes current commands, bounded
account/history reads and separate execution enhancements. The user approved
these recommendations, including richer reads and scoped agents, on 2026-10-03.
Finalize exact schemas and transport security before coding. Define journal freeze/writer-epoch and API
availability behavior now so P21 can later integrate without a second authority
store. Do not implement fake claim/recovery endpoints to satisfy a transport test.
Evidence: W06, ADR 0018/0019/0020, P18/P19 accepted history and tracked Rust/TypeScript
protocol vectors; runnable offline browser/Node transport and adversarial process tests.

Bounded split: [ADR 0021](../architecture/0021-confidential-web-api.md) records the
2026-10-03 approval for local standard Noise NK qualification in a separate
Rust/WASM tool workspace. The first PR carries the contract, candidate dependency
policy and native/actual-browser tests; it neither links the candidate into the
shipping service nor supplies independent browser attestation. Subsequent slices
own the selected dependency/verifier and exact binding profile, HTTP integration,
read projections/WebSocket and joined acceptance. Keep P21A in progress throughout;
the four phase criteria below remain open after the first slice.

PR grouping clarification, 2026-10-03: milestone boundaries are not mandatory PR
boundaries. After #47 (contract/core) and #48 (browser verifier/binding), target
**two further stacked PRs**: (A) dependency/transport hardening, server quotes,
SDK session safeguards and HTTP integration; (B) owner-scoped reads, WebSocket
commands/updates and joined offline acceptance. Keep the submilestones/evidence
visible in TRACKER, but include their tests and documentation in the owning PR
rather than creating separate bookkeeping or test-only PRs. Split further only
for a material security choice or a genuinely unreviewable implementation, and
explain the reason. This grouping does not waive any criterion. Stop for review
before selecting a dependency fork or changing the approved security profile.
Provider decision, 2026-10-03: use pinned upstream Snow 0.10.0 for the bounded
offline V1 API implementation, not a custom handshake or local crypto fork.
Its opaque-secret erasure limitation is retained in ADR 0021. It does not block
synthetic-data offline integration; an explicit hardening/security disposition
remains required before customer use. Do not treat this decision as erasure,
audit or production qualification, or close any phase criterion without evidence.
P21A completion is offline implementation/acceptance; fresh changed-image Nitro
and actual venue/chain workflow qualification remain P23, not a new P21A PR.
HTTP implementation: shared Rust/WASM core in `crates/web-channel`, enclave ingress
in `crates/service::web`, fixed-target Node built-in HTTP relay in `services/web-relay`
and `clients/private`'s AWS-only browser/Node SDK. ADR 0021 defines exact bounds
and framing; the tool runner joins real processes to the existing journal.
The final grouped slice adds signed owner-scoped reads, immutable replay-derived
book history, the same commands over WebSocket and bounded replacement-page
subscriptions. The [versioned read/delivery contract](../architecture/private-read-contract.md)
defines exact schemas/provenance/bounds and Node/actual-browser acceptance.
All four slices were reviewed and merged through native stack #49 on 2026-10-03
at `05303740dd00129eeb038c9963f44301405725d7`. The criteria below close for the
bounded offline implementation; changed-image/native qualification remains P23.

- [x] Reviewed transport ADR and exact schemas/signing vectors establish client-to-enclave confidentiality and fresh attestation before private authentication; malformed/replayed/expired/altered sessions fail closed.
- [x] Both transports reach the same command authorization, durable IDs, reservations and journal; exact retries reconcile, conflicting IDs reject, and agents cannot obtain payout or administrative authority.
- [x] Private updates follow committed owner-attributed state; bounded queues, missed-message snapshots, reconnect, revocation and cross-account isolation are exercised with no global-sequence leakage.
- [x] Actual browser and Node clients run order/query/cancel and owner-payout fixtures; relay plaintext/log checks and mid-operation disconnect/restart regressions pass offline. No synthetic fill or unavailable native rail is advertised as live.

<a id="p21b"></a>
## P21B — Public documentation from the implemented API

Depends on: P21A.

Deliver: copy-ready MDX content in docs-content/ for the user's existing separate
Mintlify repository, with product/funds/trading,
security/risk/recovery guides, SDK quickstart and one page for each implemented
customer method plus WebSocket commands/subscriptions. Document the actual P21A
version, units, permissions, lifecycle, errors, limits and reconnect handling.
Check examples against the real contract and offline tests; no invented
HTTP route, fake deployment URL or unimplemented interactive playground. Public
publication follows implementation, with explicit environment/qualification status.
Local/tested implementation is not production availability; update live evidence
after P23. Publishing credentials, ignored research or unverified mainnet/privacy/
solvency/unconditional-exit claims is prohibited. Hosting/account setup is a distinct
external step; lack of hosting must not masquerade as a published docs deployment.
User clarification: do not build another site, add package/configuration/dev-server
tooling or new CI for this phase. Keep MDX and native Mintlify field/example
components. Endpoint references show SDK pre-encryption fields and decoded replies,
not a fictitious JSON exchange; disable the incompatible interactive playground.
The user subsequently supplied their own local `docs.json` and requested that
all `docs-content/` stay ignored, for copying into the separate Mintlify repo.
No documentation-source PR is required. Record delivery/checks here; tracked
bookkeeping can merge with P21. Relative carrier routes are ordinary reference
text, not `api:` frontmatter requiring a nonexistent public server URL.
Evidence: P21A schemas/examples, BASELINE B01–B08/F01–F14/S01–S05/R01–R05,
tracked implementation and sanitized qualification receipts; MDX compilation,
content links and SDK example checks, using temporary validation tools only.

- [x] Guides accurately state pooled execution, customer ownership, fees/protection limits, private versus public data and operator-assisted recovery; unsupported features and environments are clearly distinguished.
- [x] Every advertised method/stream maps to implemented handlers and checked examples, including authorization, exact units, uncertain outcomes, disconnect and reconciliation; example checks and their source checkpoint are recorded without adding new recurring CI.
- [x] Copy-ready MDX syntax, page links, navigation and safe-publication checks pass without secrets or ignored work; rendering/publication in the user's existing Mintlify repository has its own receipt if performed, not a claim based on local content validation.

<a id="p21"></a>
## P21 — Integrated recovery and independent claim delivery

Depends on: P10, P12, P16, P17, P19, P21A.

Deliver: controlled cutover/reconciliation, venue unwind/returns, final root/backing
verification, operator activation, encrypted proof-package publication and standalone
claimant. Pending claims/fees/late actions all use the same financial journal.
Evidence: W04–W08; live-like offline outage rehearsal. Use explicit fake hardware/
witness ports here; actual P20 runtime and live qualification join in P23. Do not
block offline recovery engineering on AWS access or label it hardware-qualified.
Exercise the implemented HTTP/WebSocket service/SDK's accepted tail and existing
payout history, not an independently seeded recovery ledger. Claim delivery and
submission must still work with the ordinary service unavailable; representative
final payouts use the actual local compiled custody/recovery program or an
explicitly separately checked SBF path. Scheduled after P21B public docs; update
those docs when integration passes, rather than claiming recovery is already live.

Implementation order within this bounded phase:

1. Derive a read-only settled financial cut from the existing journal, including
   every configured customer and ordinary payment history. Do not promote the cut
   to a sealing/fencing certificate or create another entitlement ledger.
2. Join controlled cutover, qualified native terminal outcomes and returned assets,
   then bind the exact final cut to independently checked chain counters/backing.
   Ordinary admission/exposure freeze must not disable necessary recovery-only
   reconciliation/unwind/return work or silently re-enable the ordinary writer.
3. Prepare and independently deliver recipient-encrypted packages, verify their
   exact active context and submit owner claims without the normal API. Reuse P17
   tree/wire rules; do not invent a new user encryption-key scheme without checking
   the approved private-runtime/recovery contracts and recording its disposition.
4. Rehearse the actual HTTP/WebSocket SDK/service accepted tail, late/unknown effects
   and prior payouts through local compiled recovery claims, including outages and
   races. Library fixtures alone do not close any of the phase criteria below.

- [x] No ordinary writer/payout/escaped capability can race an activated root; unresolved native outcomes prevent unsafe activation.
- [x] Claimants obtain and verify kits without the ordinary API; package loss, stale package and privacy are tested.
- [x] Final funded claims match remaining entitlements; unavailable venue assets do not become payout cash or disappear as liabilities.

Offline acceptance disposition: journal revision 21 implements explicit trusted
cutover and irreversible dispatch sealing; scoped recovery IOC closes and flat
returns never reopen ordinary spending. Installed P16 counters/backing and the
complete immutable owner-authorized encryption-key inventory bind preparation.
RSA-OAEP-SHA256/AES-256-GCM packages require both immutable-store readbacks;
the separate governed publisher signs the public ciphertext manifest. The Node
claimant needs no ordinary API and builds an unsigned claim only after checking
live ACTIVE context/counters/backing. Keys and private locators must be retained;
no automatic key rotation/lost-key reissue or public onboarding endpoint is added.
Both actual HTTP/WebSocket accepted-tail outage rehearsals execute normal payout,
return and final claims in compiled local SBF. Native completion/fencing, source
authentication and store independence are explicit qualified **fake ports** here,
not claims about live capability closure. P23/G01/G03 qualify those ports, entropy
and the changed measured image before deployment. Tests do not prove complete
liabilities or unconditional solvency/exit. P21 stays in progress until review/merge.

<a id="p22"></a>
## P22 — Joined offline adversarial acceptance suite

Depends on: P09, P12, P16, P17, P18, P19, P21, P21A.

Deliver: deterministic end-to-end fake venue/chain/time harness driving the actual
HTTP/WebSocket SDKs, runnable relay/private service, protected journal and production controllers/
adapters. Cover deposit → credit → native funding → order → partial fills/cancel →
funding/fees → close → return → beneficiary payout, plus outage/recovery/claim.
Specify per-step customer/house claims, external exposure, physical locations,
reservations, source cuts and payment expectations; a successful HTTP response is
not the oracle. Qualify representative chain/recovery effects against actual local
SBF and distinguish that evidence from deterministic fake receipts. Add differential/
reference conformance, bounded state exploration, fuzz, reverse stress and real
service-process crash tests. Bring failures back to owning phases.
Evidence: all W IDs and V01–V10; fresh-checkout repeatable seed manifest.

- [x] Opposing winners/defaults, ADL gaps, funding boundaries, house exhaustion, inaccessible cash and payout races preserve attribution/claims.
- [x] Duplicate/conflicting/late events, restart, rollback and malicious ordering cannot double-spend or silently omit loss.
- [x] Test bounds, counterexamples and unresolved proofs are explicit; measurements meet declared load/resource budgets without weakening economics.
- [x] Normal, partial/cancel and recovery workflows start through actual HTTP/WebSocket SDK/service paths, not only direct Rust method calls; private streams recover after gaps/reconnect without duplicate financial actions. Fresh-checkout offline CI reruns the recorded manifest with no live API/RPC/AWS dependency.

Technical acceptance is recorded in the [tracked manifest](acceptance-manifest.json)
and [bounded acceptance contract](../architecture/0022-offline-acceptance.md).
Local success does not close the phase before PR review/merge or enable live ports.

<a id="p23"></a>
## P23 — Bounded devnet/testnet integration qualification

Depends on: P20, P22.

Deliver: current G05-approved account/funding/environment/cleanup manifest, tagged
release tests using the actual program, adapter, enclave and recovery path. Reuse
M1/M2 scenarios but obtain new implementation evidence. No mainnet inference.
Carry P22's SDK/service workflow assertions onto actual approved infrastructure;
qualify the P21A transport and changed measured application on hardware, rather
than treating P20's exact-source receipts as evidence for a later image. Refresh
P21B public environment/availability documentation from actual current receipts;
verify implementation receipts and current native authority/precision/completeness,
not a replay labeled as live integration. Use bounded scenarios and preconfigured
test accounts; an unavailable capability is a named blocker, not a simulated pass.
Evidence: W03–W08; sanitized receipts and failure/cleanup reports linked to commit.

Execution handoff: [live qualification runbook](../operations/live-qualification.md)
tracks the audited production connections and G05 checklist. The first offline
slice added account-bound, durably budgeted native observation transport. The
continuation connects confidential chain signing/RPC, streamed executable checks,
finalized receipts, owner-deposit recognition and finite read-only scheduling;
[ADR 0024](../architecture/0024-confidential-chain-ports.md) defines the six-role
release and trust boundaries. Run the fixed local checklist before the fresh live
manifest. Trading/funding admission, authentic native setup/credit/payment/cut
and precise funding remain independently gated; diagnostics do not qualify them.
First hardware step is changed-image transport/key/read qualification, not a
claim of a complete financial live workflow.
Review corrections also belong to this fixed checklist: release exactly the
measured role set, continue after ineligible original deposits without hiding
port/storage failures, close expired unsent physical plans only on verified
no-wire/no-effect history, and never infer complete source coverage from a
single transaction lookup. Runtime request waiting and post-I/O poll cadence
remain bounded. Retained/uncertain sends still require original reconciliation.

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
Every publication/fix/merge handoff records the affected PR heads and their own
CI/review state, including open predecessors. Never overwrite historical test
evidence or mark an unmerged phase closed just to make the progress table look current.
