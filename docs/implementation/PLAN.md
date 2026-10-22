# Implementation acceptance criteria

Compact phase contracts. Current status and the next authorized action are in
[TRACKER](TRACKER.md); full historical scope is preserved in the
[October 23 snapshot](../archive/implementation/PLAN-20261023.md).
No acceptance checkbox, dependency or requirement ID was removed by consolidation.
Implementation merge is not native, hardware or customer-funds qualification.

<a id="p00"></a>
## P00 — Tracked implementation foundation

Depends on: none.

Deliver: this plan, tracker, contributor guide, sanitized baseline, research reuse
map and offline documentation checks. Keep local research and legacy code intact.

Evidence: W01–W09; documentation validator/self-tests and fresh offline study run.

- [x] Plan/tracker phase IDs, dependencies and status vocabulary agree; tracked links resolve.
- [x] Approved versus open policy and prototype versus production evidence are explicit.
- [x] Fresh checkout checks need no `work/`; P01 has a concrete handoff; PR targets TEE trunk.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p00).

<a id="p01"></a>
## P01 — Production workspace and test harness

Depends on: P00.

Deliver: ADR for language, pinned supported toolchains, package ownership and error
boundaries; smallest compilable kernel/store/adapter-port workspace. Prefer a
memory-safe enclave-suitable kernel, but compare existing evidence before selecting
libraries. Add lockfiles, formatting/lint/unit/property-test commands and offline CI.
Do not scaffold a website or deploy programs.

Evidence: W01, W06, W08; reproducible
clean build and documented local/CI parity, including Linux target requirements.
Address the repository integration follow-up recorded in TRACKER: propose scoped
TEE review/protection and legacy-preview separation, obtaining any required
repository/deployment-setting authority. Do not reintroduce web code as a CI fix.

- [x] Fresh checkout builds/tests with pinned dependencies and no venue/RPC secrets.
- [x] Kernel cannot import I/O/venue packages; fake clock/venue/store ports support fault injection.
- [x] Linux/SBF/enclave checks are mapped to owning phases, not falsely reported as already qualified.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p01).

<a id="p02"></a>
## P02 — Canonical financial types and event identities

Depends on: P01.

Deliver: typed domain/account/market/asset IDs, integer quantity/price/basis codecs,
checked arithmetic, domain-separated canonical bytes and policy/schema versions.
Keep user request, signed attempt and economic event IDs distinct.

Evidence: W01,
W03; port fixture V02 and boundary/fuzz vectors to the chosen production language.

- [x] Overflow, overprecision, bad signs/units, unknown versions and ambiguous encodings reject without side effects.
- [x] Negative basis/division/remainders round as specified; no float parsing path exists.
- [x] Same ID/different payload conflicts; domains and source namespaces cannot replay into one another.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p02).

<a id="p03"></a>
## P03 — Unified ledger, positions and ownership

Depends on: P02.

Deliver: one pure transition state with customer/house/suspense ownership, cash versus
physical assets, basis/PnL and explicit shortfall diagnostics. All external changes
have named provenance; no plugin gets a second balance sheet.

Evidence: W01–W02;
V01–V03, wrong-owner and opposing-position/default conformance vectors.
Also deliver the user-requested [full system architecture](../architecture.md),
including component responsibilities, trust boundaries, end-to-end operation and
the distinction between implemented and planned phases.

- [x] Opening, scaling, partial/full closes and reversals preserve exact basis and bridge identities.
- [x] Net-flat default remains visibly underbacked; negative customer claims are not spendable assets.
- [x] Wrong-user attribution fails even when aggregate exposure/equity matches; projections cannot double-count assets.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p03).

<a id="p04"></a>
## P04 — Funding, fees and evidence reconciliation

Depends on: P03.

Deliver: qualified funding boundaries/accrual/settlement, fee/rebate ownership,
source normalization ports, explicit mismatch aging and restricted modes. Ingest
actual losses despite limit breaches.

Evidence: W01, W03; V04, gross/net funding,
entry-time/partial-close/rounding and REST/WS duplicate vectors.

- [x] Funding and fees post once with per-user attribution; inclusive native PnL cannot double-charge fees.
- [x] Missing/corrupt sources create a named unresolved condition, never fabricated zero or house profit.
- [x] Stale marks and unexplained precision differences block dependent admission, not raw evidence retention.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p04).

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

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p05).

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

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p06).

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

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p07).

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

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p08).

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

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p09).

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

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p10).

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

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p11).

<a id="p12"></a>
## P12 — Native ADL and bounded RF1/RF2 restoration

Depends on: P11.

Deliver: qualified forced-event cut, proportional reduction, scalable monotone quota
schedule, original-basis restoration and risk-unit policy. Preserve different event
families. Native account splitting is enabled only with G01 evidence; one tested
risk unit is sufficient initially.

Evidence: W01, W03, W09; V07 and allocation bounds.

- [x] Actual restoration, ADL fee refund and signed basis implement RF1; no gap funding or promised fill on an unexecuted quantity.
- [x] Each prefix respects RF2 quotas and composed bounds; no per-atomic-lot unbounded loop, operator reweighting or historical-manifest bypass.
- [x] Changed close intent, partial/late fill, prior capital reservation and post-submission shock preserve already owed economics.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p12).

<a id="p13"></a>
## P13 — Pacifica observations and capability profile

Depends on: P04, P05.

Deliver: tracked native mapping, lossless codecs, replayable sanitized evidence,
account/order/fill/funding normalization and explicit capability states. Recheck
current primary docs; use fake transport first. Do not inherit BULK fields or assume
Last ID is a global complete journal.

Evidence: W03; VM-01–VM-09 qualification map.

- [x] REST/WS overlap posts one economic fill; reversal legs remain separate and empty position snapshots mean flat.
- [x] Precision/PnL/funding gaps remain named and disabled for dependent use; the observed fee discrepancy is not auto-written off.
- [x] Pagination, gaps, corrections, stale snapshots and replay cuts have tested containment; raw evidence retains provenance.
- [x] Qualified raw-input resolution binds the original CommitId/input ordinal and exact evidence fingerprint; normalized effects or an authenticated no-effect finding commit with resolution once. Wrong scope, conflicting/repeated resolution, restart and unrelated unresolved inputs cannot clear the exposure gate. No generic administrative reset is available.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p13).

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

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p14).

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

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p15).

<a id="p16"></a>
## P16 — Funding coordinator and native round trip

Depends on: P08, P14, P15.

Deliver: vault → approved broker route → native margin → return → customer
controller, with one common journal and bounded bootstrap/config transitions.
No automatic exposure to lending or unresolved native agent rights. Offline first;
live test requires a current approved manifest.

Evidence: W04 and V01/V05.

- [x] Marginal working collateral is funded once; unconfirmed venue credit cannot admit a trade.
- [x] Kill/restart after withdrawal POST exposure reconciles the original attempt without double withdrawal or payout.
- [x] Fees, partial/failed returns, bootstrap exceptions and cleanup residuals are attributed and reported, not hidden by retries.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p16).

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

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p17).

<a id="p18"></a>
## P18 — Private API and client operation semantics

Depends on: P07, P08, P14.

Deliver: customer/agent grants, canonical auth, private views, operation/query
semantics and thin SDK. Use local encrypted-transport interfaces pending P19;
do not expose a public plaintext fallback.

Evidence: W06; auth/replay/ownership tests.

- [x] Authenticate before dedupe/private lookup; one customer's order or operation cannot be queried/cancelled by another.
- [x] Grants enforce methods, domain, expiry, limits and epoch; native authority is never exposed to customers.
- [x] ACK/fill/unknown and partial/fee semantics are explicit in SDK results; logging/errors do not leak private payloads.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p18).

<a id="p19"></a>
## P19 — Attested transport, runnable service and client release verification

Depends on: P06, P18.

Deliver: reviewed channel/crypto/server-runtime ADR, attestation verification,
fresh session binding, independently verifiable clients and a minimal runnable
parent relay/confidential service wired to P18 and the protected journal. Document
actual entry points and local startup/test commands when implemented; do not ship
only another injected transport interface. Earlier Noise experiments are candidates,
not approval of a production protocol. Qualify local positive/negative quote
fixtures; no hardware claim from mocks.

Evidence: W06–W08; SDK-driven process tests,
replay/measurement/key substitution and hostile relay/host-observation tests.

- [x] AWS chain, approved measurements, freshness and session-key binding all validate before private exchange. The AWS-root production path is implemented; positive current-session evidence is synthetic, not NSM hardware qualification.
- [x] Downgrade, replay, wrong release and error/stream paths cannot expose private plaintext to parent/relay. This profile supplies bounded snapshots, not network subscriptions.
- [x] Client distribution threat and key rotation are documented; logged/host-visible metadata is explicitly bounded.
- [x] Thin SDK reaches actual relay/service processes; an authenticated intent commits durably and its private response/query survives disconnect/restart without a second native attempt. Native venue I/O remains an explicit fixture in this slice, including a lost committed reply.
- [x] Connection/request/stream bounds, deadlines, backpressure, shutdown and failure behaviour are tested. Designated parent logs/errors/replica storage receive no private plaintext; test attestation/key providers cannot silently enable a production service. Local fixture process memory/stdin are not Nitro isolation evidence.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p19).

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

- [x] Venue TLS/auth terminates inside enclave; parent cannot read payloads or replace upstream responses undetected. Actual paper-origin TLS-only negatives pass; native credentials/actions remain gated off until P23.
- [x] Wrong measurement/debug/replayed release denies keys; old loaded keys and outstanding capabilities are fenced, not merely KMS-disabled. Actual retained-head epoch, natural lease and issued STS expiry/restart checks pass; no native capability was enabled or escaped in this qualification.
- [x] Accepted encrypted state survives qualified failure/restore; witness loss and split writer fail safely, with costs/limits recorded. Real replicas/CAS participate; lost-ack injection is inside the test enclave, not a physical packet drop.
- [x] A tagged enclave executable and parent relay run the same SDK operation contract; controller scheduling and persisted unknown attempts cannot bypass qualification or auto-retry after restart. Local fixture acceptance and actual hardware evidence are recorded separately. Exact-source ELF/EIF/PCR releases are identified in both receipts; live native controller workflows remain P22/P23 scope.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p20).

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
The [approved API scope](../archive/qualification/API_SCOPE.md) distinguishes current commands, bounded
account/history reads and separate execution enhancements. The user approved
these recommendations, including richer reads and scoped agents, on 2026-10-03.
Finalize exact schemas and transport security before coding. Define journal freeze/writer-epoch and API
availability behavior now so P21 can later integrate without a second authority
store. Do not implement fake claim/recovery endpoints to satisfy a transport test.

Evidence: W06, ADR 0018/0019/0020, P18/P19 accepted history and tracked Rust/TypeScript
protocol vectors; runnable offline browser/Node transport and adversarial process tests.

- [x] Reviewed transport ADR and exact schemas/signing vectors establish client-to-enclave confidentiality and fresh attestation before private authentication; malformed/replayed/expired/altered sessions fail closed.
- [x] Both transports reach the same command authorization, durable IDs, reservations and journal; exact retries reconcile, conflicting IDs reject, and agents cannot obtain payout or administrative authority.
- [x] Private updates follow committed owner-attributed state; bounded queues, missed-message snapshots, reconnect, revocation and cross-account isolation are exercised with no global-sequence leakage.
- [x] Actual browser and Node clients run order/query/cancel and owner-payout fixtures; relay plaintext/log checks and mid-operation disconnect/restart regressions pass offline. No synthetic fill or unavailable native rail is advertised as live.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p21a).

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

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p21b).

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

- [x] No ordinary writer/payout/escaped capability can race an activated root; unresolved native outcomes prevent unsafe activation.
- [x] Claimants obtain and verify kits without the ordinary API; package loss, stale package and privacy are tested.
- [x] Final funded claims match remaining entitlements; unavailable venue assets do not become payout cash or disappear as liabilities.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p21).

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

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p22).

<a id="p23"></a>
## P23 — Bounded devnet/testnet integration qualification

Depends on: P20, P22, P23C.

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

- [ ] Deposit, trade lifecycle, funding observations, ordinary return/payout and recovery execute or expose a named blocking capability.
- [ ] Real process/network faults and credit exhaustion reconcile safely; replay is not reported as a newly observed venue event.
- [ ] All positions/orders/funds and test costs reconcile; no real customer/mainnet assets; private evidence stays out of Git.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p23).

<a id="p23a"></a>
## P23A — Venue-directed custody contract and contributor context

Depends on: P15, P16, P17, P22.

Deliver: ADR 0030 recording user/venue/asset pending escrows, full identified-batch
sweeps, exclusive owner refunds before sweep, qualified venue credit and
venue-specific PDA return/payout protection. Update BASELINE, architecture,
README, contributor guide, evidence map and tracker without rewriting old
test receipts or implying an implemented migration. Keep AGENTS.md a compact
repo map and safety contract; detailed evolving handoffs remain in TRACKER.

Evidence: explicit October 15 user approval, existing P15/P16/P17/P22 source,
W03–W05/W09 and current primary PDA/withdrawal docs linked in ADR 0030.

- [x] Approved contract versus implemented/live behavior is clearly separated in all entrypoints.
- [x] Batch ownership, sweep/refund races, attribution timing, pending restrictions, payout counters and recovery exclusions have explicit rules and test targets.
- [x] Prior economics, public-privacy limits, unresolved test-fund inventory and separate G01–G05 gates are preserved.
- [x] Offline plan/link/contract checks pass; PR/review and the exact next slice are recorded before phase closure.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p23a).

<a id="p23b"></a>
## P23B — Atomic venue escrow program and typed client

Depends on: P15, P17, P23A.

Deliver: extend the existing Anchor 1.2 custody executable and clients/vault with
per-owner/route/mint escrow identities, permanent exact-amount batch records,
owner-signed deposit/refund and funds-authorized full-batch sweep. Preserve old
account layouts, normal/recovery counters and explicit legacy instructions.
Bind sweep consent, destination, epoch, expiry, release cap and sequence. Frozen
mode blocks new deposits/sweeps while allowing eligible never-swept owner refunds.
Concrete ABI/seeds are recorded in ADR 0030 before they become public contracts.
Do not add an HTTP signer, plaintext host keeper, new token standard or deployment.

Evidence: P15/P17 signed SBF regressions, W04/W05, exact IDL/type generation,
ADR 0030 program/client case matrix and locally compiled instruction execution.

- [x] Exact full-batch deposit/sweep/refund executes on compiled local SBF; no raw balance-based credit or arbitrary recipient/route.
- [x] Both sweep/refund orderings, duplicate/recreated IDs, two owners/batches/routes, wrong mint/authority/broker, expiry/freeze/epoch/cap/sequence and transfer-failure cases pass with conservation.
- [x] Terminal batch consumption survives replay; unsolicited surplus cannot authorize another sweep or payout; existing normal/recovery tests remain passing.
- [x] Tracked generated IDL/types match source, client checks pass, no live activity occurs and review/CI evidence is recorded before merge.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p23b).

<a id="p23c"></a>
## P23C — Escrow funding and payout integration over one journal

Depends on: P16, P18, P22, P23B.

Deliver: original sweep intent/receipt binding, pending-owner inventory and
exactly-once deposit recognition at qualified finalized handoff. Retain funding
restrictions through broker/native transit until eligible venue credit; internal
movements never create another customer deposit. Join the current encrypted
Runtime/controller/SDK path and update the offline acceptance manifest. Return
funds to the venue-specific PDA payout vault and pay the registered wallet with
shared normal/recovery consumption. Keep legacy routes explicit and version
changed journal/wire semantics; do not migrate histories or counters implicitly.

Evidence: P16/P22 controller, crash/race and joined HTTP/WebSocket/SBF suites,
ADR 0030 accounting/qualification cases, W01/W03–W05/W09 and original P23 receipts.

- [x] Unswept ingress/refunds are not spendable trading credit or duplicate recovery claims; finalized handoff recognizes the correct owner once, including fees/debt treatment.
- [x] Durable intent precedes signing; cancellation/exposure races, response loss, duplicate/conflicting receipt, restart, partial/missing native credit and route/capacity restrictions pass without uncertain resends.
- [x] Qualified return and ordinary/recovery payouts share final-payment counters; unswept refunds stay segregated and swept-but-uncredited liabilities remain visible.
- [x] Joined offline SDK/service/compiled-SBF workflows and old-route regressions pass; status fields distinguish pending, funding in progress and available collateral; no live credit or changed-image qualification is claimed.
- [x] Exact source/CI/review and remaining P23 live dependencies are recorded; frontend/onboarding and fresh AWS/venue/chain activation stay separately scoped.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p23c).

<a id="p25"></a>
## P25 — Backend onboarding and escrow preparation

Depends on: P18, P21A, P23C.

Deliver: bounded opt-in wallet-signed creation/retrieval of an empty private
account through the existing confidential SDK/API. Persist immutable ownership
and payout binding in the authoritative journal, not a signup database or
per-customer startup configuration. Discovery returns configured custody route,
account and current epoch. Unsigned client helpers prepare owner-paid customer
registration, escrow opening, exact batch deposit and eligible refund. Retain
owner-reported original transaction locators without treating them as credit;
the existing qualified chain path alone recognizes inventory/handoff. Include
new accounts in authorization, projections, risk, funding and recovery. No native
per-user account, balance injection, financial-policy relaxation, frontend,
deployment, AWS invocation or investigation of missing Pacifica credit.

Evidence: B02/B04–B08/S01–S05, ADR 0030, P18/P21A confidential carriers,
P23B unsigned builders and P23C journal/Runtime/SBF acceptance. The detailed
contract and exact-source checks belong to the owning handoff and ADR 0031.

- [x] Fresh wallets create/retrieve exactly one empty account per deployment; signatures bind service domain, policy, session and deadline; wrong signer/domain/channel, agents and malformed requests fail without disclosing another account.
- [x] Atomic registration and immutable owner/recipient survive restart, duplicate/concurrent requests and stale/uncertain writes; bounded governed capacity fails closed; old configured-account mode remains explicit.
- [x] SDK discovery and unsigned escrow preparation require no customer-specific startup edits; local compiled-program tests cover registration, opening, exact deposits and eligible refunds without public-chain activity.
- [x] Signed original deposit locators are durable hints only; private lifecycle and risk/funding/recovery account inventory use the same journal; unknown/partial credit cannot become trading collateral.
- [x] Fresh-wallet SDK/API acceptance, owner isolation, reconnects, generated ABI preservation and existing regression/CI checks pass; tracker records exact source, limitations and PR/review state before merge.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p25).

<a id="p26"></a>
## P26 — Second-venue assessment and implementation boundary

Depends on: P25.

Deliver: source-pinned BULK candidate map under the present custody model,
historical/current SDK comparison, venue-specific accounting/authority/fees and
funding-return evidence requirements. Identify actual single-venue seams in the
code and propose the smallest test-first two-unit composition that retains one
accepted financial history. No new adapter runtime, dependency, frontend or live
invocation in this slice. Do not redo financial economics or resume deferred
Pacifica credit tests.

Evidence: B02–B08/F01–F14, ADR 0030/0031, [BULK qualification map](../venues/bulk.md),
[second-venue sequence](../archive/qualification/SECOND-VENUE.md), existing P13–P18/P22 ownership/fault tests
and date-pinned primary SDK/documentation.

- [x] Current official source pins and historical comparison distinguish documented, implemented and unverified deployed behavior; no old experiment is relabeled as a new pass.
- [x] Financial/custody capability map covers exact precision, identity, cash/funding/fees, native margin and ADL, credit/returns and key powers; missing evidence has an explicit disabled treatment.
- [x] Minimum unit-scoped ownership/routing/recovery boundary and bounded next qualification are specified without a second balance database, generic plugin framework or inherited live authority.
- [x] Documentation/plan/link guards pass; tracker records exact source, review/CI, predecessor status and next action before authorized merge.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p26).

<a id="p27"></a>
## P27 — BULK exact wire and fill codec foundation

Depends on: P26.

Deliver: one narrow offline codec package, not a native Runtime integration.
Exact decimal/exponent/scaled-grid conversion, unsigned single bounded limit and
cancel encoding, scoped native order/fill identity and distinct REST/WS fill
normalization. Preserve fee/reason/range metadata without financial posting or
causal-readiness authority. No venue SDK, secrets, network, native funding,
new accounting model, cross-venue margin or frontend. Further financial mapping
and joined execution-unit ownership remain separately gated.

Evidence: F02/F03/B02/B08, [ADR 0032](../architecture/0032-bulk-codec-foundation.md),
BV-01/BV-02/BV-06/BV-07/BV-10 in [BULK map](../venues/bulk.md), pinned upstream
derived signing/hash vectors and synthetic native shape/adversarial fixtures.

- [x] Exact canonical limit/cancel bytes and index-0 order IDs match pinned source vectors; money/IDs never traverse floating-point production types; unsupported actions have no dispatch path.
- [x] REST/WS scope/side/order attribution, signed fees, wide IDs, decimal/exponent precision, duplicate/conflict, malformed/partial history and resource bounds pass offline regressions; page coverage grants no completion.
- [x] New package and existing pinned JSON raw-value feature pass dependency/lock guards; shipping service/kernel do not acquire adapter, network, signing or journal ownership through this foundation.
- [x] Existing workspace checks and new focused fixtures pass on exact tracked source; merger receipts, observed testnet availability limits and pending native/runtime qualifications are recorded before review/merge.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p27).

<a id="p28"></a>
## P28 — BULK financial observations and execution-unit contract

Depends on: P27.

Deliver: exact offline account/position, funding, fee-quote and forced-event
observations in the isolated BULK package, bound to explicit source and kernel
units. Preserve local/aggregate differences, display-versus-basis semantics,
original unsupported rows and page limits. Document the financial posting gates
and a concrete two-unit state/authorization/recovery contract before any joined
Runtime or journal change. No new ledger, dispatch, signing, live run, funding
completion, financial policy, frontend or reopened Pacifica credit investigation.

Evidence: F01–F14/B02–B08, P27 fixtures, pinned official client history/account
schemas, current primary venue documentation and SECOND-VENUE. Contracts:
[financial mappings](../architecture/0033-bulk-financial-observations.md) and
[proposed unit-state boundary](../architecture/0034-execution-unit-boundary.md);
reproducible synthetic fixtures live in `crates/bulk/tests/financial.rs`.

- [x] Account/position decoding preserves exact units, signed positions, original unsupported rows and missing-versus-zero fields; local/aggregate metrics are never added or converted into an authoritative cash/basis setter.
- [x] Funding/fee/risk observations preserve source/owner/family/ID, exact signed amounts, pagination, adverse reasons and original bytes; duplicates/conflicts and unsupported lanes cannot fabricate credit or economic completeness.
- [x] Financial mapping records cumulative versus event/settlement semantics and explicit qualification gaps; native ADL and risk-vault observations cannot independently trigger Cinder payouts or restoration.
- [x] A concrete two-unit contract specifies one atomic history, unit-scoped books/identities/permissions/capital/custody/recovery and migration/test boundaries without implementing unreviewed runtime changes.
- [x] Focused adversarial fixtures and appropriate workspace regression/dependency/docs checks pass on recorded exact source; PR publication/review and deferred native qualifications are explicit.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p28).

<a id="p29"></a>
## P29 — New-history execution-unit journal foundation

Depends on: P28.

Deliver: immutable unit registry and one durable deployment history containing
existing per-unit financial states, shared enrollment/authority epochs and
unit-owned foundation reservations. Reuse existing exact transitions, encrypted
storage/CAS/freshness contracts and retain adverse facts on rejected controls.
Keep legacy replay and shipping Runtime unchanged. No exposure, full order/funds
admission, API/SDK migration, recovery activation, native/RPC/AWS run or frontend.

Evidence: F01–F14/X01–X05/S01–S04, P05/P06 replay fixtures,
[ADR 0034](../architecture/0034-execution-unit-boundary.md),
[bounded implementation](../architecture/0035-unit-journal-foundation.md).

- [x] Registry binds domain/native/custody/source/precision/key-policy ownership, starts empty and rejects duplicate or shared physical backing aliases without changing legacy codecs.
- [x] Enrollment and revocation are atomic across units; reservation identities cannot change unit or commitment, and rejected controls do not discard actual observations.
- [x] Replay, duplicate/conflict, wrong-unit source, independent inventory, CAS/unknown outcomes, freshness fencing, encrypted storage and legacy/new-format isolation pass deterministic offline tests.
- [x] Appropriate exact-source workspace checks pass; implementation limits, PR/review/CI state and the next versioned API/controller/runtime gate are recorded without activation claims.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p29).

<a id="p30"></a>
## P30 — Signed unit-bound API and SDK contract

Depends on: P29.

Deliver: explicit v2 request/intent/reply unit binding, exact shared command
semantics, current-head owner/agent scope preflight over the shared journal and
browser/Node SDK encoding/signing/reply helpers. Preserve v1 and shipping Runtime.
Preflight is not durable financial admission: no grant installation, quota
consumption, native signing, dispatch, new endpoint, frontend or live test.

Evidence: B08/S01/S02/X01–X04, P18/P21A vectors, P29 state/replay tests and
[ADR 0036](../architecture/0036-unit-api-contract.md).

- [x] V2 binds UnitId in signed and stable economic bytes; independent Rust/SDK vectors agree; malformed/version/precision/size inputs and wrong-unit/unwrapped replies fail closed without v1 fallback.
- [x] Current-head scope verification checks owner/global epoch/component and typed accepted-grant cut; same owner/market across units cannot widen permissions, and global revocation survives replay.
- [x] SDK helpers snapshot signing bytes/metadata before async wallet work, preserve caller buffers and stable retry semantics, and perform no native/network action or implicit retry.
- [x] Appropriate exact-source workspace/SDK/legacy regressions pass; predecessor #73 and this phase's scope, evidence, PR/CI/review state and remaining controller/runtime gates are recorded.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p30).

<a id="p31"></a>
## P31 — Durable unit-scoped API history

Depends on: P30.

Deliver: original signed mutation records and immutable unit/economic ownership
under the shared journal; accepted owner grants/global revoke and a real derived
accepted-history grant adapter. Financial requests remain explicitly recorded-only.
No financial admission, runtime/listener activation, frontend or live activity.

Evidence: X01–X04/S01–S04, P29 replay/P30 scope tests and
[ADR 0037](../architecture/0037-unit-api-history.md).

- [x] Versioned shared state/encoding commits original API records, contract and immutable request ownership; old unit revision is refused without changing legacy financial replay.
- [x] Current authentication precedes dedupe/lookup; accepted grants and atomic global revocation survive replay without cross-unit authority or repeat effects.
- [x] Financial capture is recorded-only, cannot consume quota or create financial effects, and retains original owner/unit through retries, cancellation references and foundation-route conflicts.
- [x] Exact-source offline adversarial/CAS/unknown/freshness/encryption and workspace regressions pass; tracker records source, evidence limits, PR/CI/review and the remaining financial-controller gate.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p31).

<a id="p32"></a>
## P32 — Unit-scoped financial admission

Depends on: P31.

Deliver: one atomic decision joining original signed operations, current
authority/agent quota and unit-local order/cancel/payout/leverage controls with
existing risk/holds. Explicit recorded-only and refused outcomes stay distinct.
No shipping Runtime, listener, native exposure/signing, recovery activation,
frontend, live test or new financial policy.

Evidence: F01–F14/X01–X05/S01–S04,
P09/P18/P29–P31 fixtures and [ADR 0038](../architecture/0038-unit-financial-admission.md).

- [x] Versioned shared admission records and unit ownership replay atomically; old unit histories fail closed without changing legacy financial replay.
- [x] Current auth precedes dedupe; original epoch/deadlines/grant quota, cancel/attempt ownership and terminal refusals cannot be bypassed by retry or another unit.
- [x] Existing joined risk/holds constrain orders, payouts and leverage; trusted per-unit risk installation and adverse-fact retention do not introduce cross-margin or native exposure.
- [x] Exact-source adversarial/CAS/unknown/encryption/replay and workspace regressions pass; source/PR/CI/review limits and the pre-runtime MAX_ITEMS performance gate are recorded.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p32).

<a id="p33"></a>
## P33 — Unit service, private reads and SDK

Depends on: P32.

Deliver: an explicit opt-in unit service/SDK path, preserving one journal,
owner/global revocation, recorded-only versus decided lifecycle, private read
scope and final publication gates. Shipping v1 remains explicit, never fallback.

Evidence: ADRs 0034–0039, P18/P21A/P23 publication races and P29–P32 fixtures.

- [x] Measure repeated requests at MAX_ITEMS and implement only necessary derived head-keyed indexing/cache; freshness and replay validation remain authoritative.
- [x] Unit reads and operation receipts cannot leak another component, mistake recording for acceptance or reuse a cursor across units; current revocation/fencing governs publication.
- [x] Runnable handler and SDK request/update paths bind unit and stable operation identity with no implicit retries or changed native authority.
- [x] Focused performance, auth, read/update, restart/race and legacy regressions pass; exact source, review and remaining activation gates are recorded.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p33).

<a id="p34"></a>
## P34 — Unit execution and financial reconciliation

Depends on: P33.

Deliver: component-owned native preparations/exposure and BULK codec-to-engine
reconciliation alongside Pacifica, under one shared durable history. No global
readiness inferred from an ACK or an unqualified account snapshot.

Evidence: X01–X05, P13/P14/P27/P28 codecs, ADR 0034 and pinned native sources.

- [x] Original operation, attempt, unit/account, exact bytes and holds commit before signing/exposure; unknown outcomes block blind resend.
- [x] Fills, funding, fees, forced events and reconciliation preserve actual ownership/precision and ingest adverse facts while new admission is frozen. Unqualified native funding/ADL causes stay retained and unresolved, not fabricated financial events.
- [x] Cross-unit IDs, partial/late/duplicate/conflicting facts, crashes and shared freshness faults are contained or fenced as specified.
- [x] Offline joined execution tests and legacy regressions pass; every unsupported native capability stays disabled pending its own qualification. Final adapter/range review regressions and corrected-stack matrix pass; current-head hosted status is in TRACKER.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p34).

<a id="p35"></a>
## P35 — Unit funding, payouts and recovery

Depends on: P34.

Deliver: existing custody/funds/recovery machinery composed by execution unit,
with explicit native funding/return evidence adapters. No new financial promises
or automatic on-chain program proliferation.

Evidence: ADR 0030, P15–P17/P21/
P23B/P23C, unit registry backing aliases and each venue's verified rail contract.

- [x] Exact escrow batches/routes and native funding ownership remain unit-bound; source handoff and qualified arrival never credit the customer twice.
- [x] Unsupported or uncertain native credit/return remains pending and restricts dependent trading/payouts without cross-unit substitution.
- [x] Normal/recovery payouts preserve shared paid counters, final reconciliation, fenced authority and segregated actual backing.
- [x] Offline faults/partial movement/replay/wrong-unit/recovery tests pass; native rail gaps and approval requirements are explicit rather than mocked qualification. Final loaded-policy/discovery review regressions and corrected-stack matrix pass; current-head hosted status is in TRACKER.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p35).

<a id="p36"></a>
## P36 — Joined second-venue acceptance and qualification

Depends on: P35.

Deliver: reproducible SDK/service-to-journal-to-adapter acceptance for two units,
plus bounded native/hardware qualification where separately approved and
supported. Record offline completion separately from live integration completion.

Evidence: W acceptance matrix, P22/P23 harnesses, native capability receipts.

- [x] Joined lifecycle/adversarial fixtures cover admission, native preparation, partial fills, financial reconciliation, custody and recovery without independent balances. The SDK trading process, selected-adapter custody/recovery and existing carrier/SBF suites are complementary evidence, not one live financial round trip.
- [x] Concurrency/restart/revocation/cross-unit/freshness failures preserve accounting and publication; cached projections never become authority.
- [x] Clean-checkout CI exercises all new offline tests without ignored research, wallets or external requests. Final P36 head `3237023` passes all eight hosted offline jobs; reviewed P33–P36 merged as native stack #80 at `3486b9f`, tree-identical to that tip. Exact receipts are in TRACKER.
- [ ] Approved native/changed-image hardware runs have exact source and cleanup receipts; blocked/deferred capabilities are named and never counted as qualified.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p36).

<a id="p37"></a>
## P37 — Bounded BULK native qualification

Depends on: P36.

Deliver: reusable scoped preflight and native paper evidence using the existing
adapter, plus a precise supported funding/return boundary. No automatic live permission.
User direction 2026-10-20: target Pacifica's current experimental integration
level; native paper trading first, targeted funding/return investigation alongside.

Evidence: [current scope](../venues/bulk-native-qualification.md), historical M0
results, existing BULK wire/execution tests and [run proposal](../archive/qualification/UNIT-QUALIFICATION-PROPOSAL.md).

- [x] Reusable fixed-origin unsigned preflight retains original bounded responses, rejects changed approval/source/deadline and tests failures offline without wallets, external requests or retries.
- [x] Refresh primary client/keychain revisions and inspect ordinary signing/funding compatibility; identify exact test-rail/credit/settlement questions without importing dependencies or choosing a new custody contract.
- [x] Observe actual current metadata under explicit approval; examples/source schemas alone cannot choose native market grids or notional minima. Two approved GETs at source `b8cac32` returned HTTP 200 on 2026-10-20; the [dated receipt](../venues/bulk-native-qualification.md#observed-metadata-receipt--2026-10-20) retains exact hashes and BTC-USD's observed $1 minimum/grid. No signed action was authorized.
- [x] Under separately bounded current paper scope, test fresh-account agents/revocation, original order/cancel/fill/fee linkage and reserved cleanup using Cinder's existing wire/adapter contracts. The [October 20 receipt](../archive/qualification/bulk-paper-20261020.md) records two maker fills/open-close, cancel, both revocations, zero-fill IOC, full cleanup and source-backed absent-ID compatibility. Partial/IOC fills, nonzero fees and financial Gateway qualification are not claimed. Paper funds are not a Solana deposit or financial activation.
- [x] Record exact native outcomes, remaining semantics and disabled rails; promote only sanitized regressions needed by observed behavior. Tracked fixture/tests cover the native pair identity rule, null/mismatch/conflict refusal and raw retention. Fee-tier 404 and unsupported funding/return rails remain explicit. CI stays offline; no frontend/AWS/production action.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p37).

<a id="p38"></a>
## P38 — Measured two-unit deployment assembly

Depends on: P37.

Deliver: explicit version-7 two-unit manifest, separate Pacifica/BULK agent and
private configuration capsules, trusted preparation and the real enclave
TLS/Noise entrypoint over one protected unit journal. Managed empty owners are
not public custody onboarding. Trading, funding and polling stay disabled.

Evidence: [ADR 0042](../architecture/0042-unit-deployment.md), ADRs 0039–0041,
P36 acceptance and P37 native receipts.

- [x] Manifest/role/registry/egress contracts reject aliases, mismatches and legacy fallback; versions 1–6 retain their encoding and behavior.
- [x] Actual adapters, private API and replicated/fresh journal derive the application commitment; no fixture keys, balances, readiness or custody controller is deployed.
- [x] Entrypoint supplies the unit handler to both confidential carriers; finite lease, restart, managed owner inventory and sticky fencing are tested offline.
- [x] Trusted preparation, regression tests and operator handoff are tracked in [#84](https://github.com/arnabnandikgp/cinder/pull/84), stacked above #83; changed-image qualification remains separate. Technical acceptance is complete; review/CI/merge are not inferred from local checks.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p38).

<a id="p39"></a>
## P39 — Joined deployed-unit worker and SDK acceptance

Depends on: P38.

Deliver: bounded native read/execution workers and encrypted UnitClient
acceptance against the deployable composition, with qualified paper policy and
explicit capital/risk setup. Reuse P36/P37, not a second accounting model.

Evidence: ADR 0040, UNIT-ACCEPTANCE and native paper receipts.

- [x] Workers bind durable intent, escaped execution, observations and reconciliation to the original unit; network I/O never holds the financial writer.
- [x] Actual confidential carriers and Node/browser SDK cover both units, restart/revoke/fencing and cross-unit negatives with offline regressions.
- [x] Unqualified funding, uncertain credit, incomplete forced-event coverage and unavailable fee tiers cannot authorize trading or payouts.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p39).

<a id="p40"></a>
## P40 — Changed-image experimental parity qualification

Depends on: P39.

Deliver: separately approved bounded Nitro qualification and a final
Pacifica/BULK experimental capability comparison. This finishes the current
second-venue workstream, not production/customer-funds readiness.

Evidence: measured source/image receipts, P36–P39 tests and cleanup.

- [ ] Exact current image/roles, attestation, private SDK, durable restart and independent fencing pass on Nitro under current scoped approval. [October 20 receipt](../archive/qualification/unit-hardware-20261020.md) passed for `91a01ff`; October 21's startup safety correction changes application code and needs fresh image qualification, not inherited PCR evidence.
- [ ] Supported paper lifecycle runs through the assembled service, not only a direct venue runner; unobserved cases remain named gaps.
- [x] Capability matrix and cleanup receipt separate unsupported funding/return rails; review this boundary before integrating a third venue. [Comparison](../venues/experimental-parity.md) and [cleanup](../archive/qualification/unit-hardware-20261020.md#retention-and-closure) retain the open assembled-financial dependency.

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p40).

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

[Detailed scope and historical clarifications](../archive/implementation/PLAN-20261023.md#p24).
