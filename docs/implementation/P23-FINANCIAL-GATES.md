# P23 financial continuation map

Updated 2026-10-08. An implementation/evidence map, not a new financial contract
or live authorization. [The runbook](../operations/live-qualification.md),
[ADR 0024](../architecture/0024-confidential-chain-ports.md),
[PLAN](PLAN.md#p23) and BASELINE remain authoritative.

**Current priority, 2026-10-08:** the user has deferred the current missing-credit
investigation. Do not repeat deposit initialization, poll these credits or spend
another AWS session trying to bypass that prerequisite. Preserve the retained
originals and residual accounting. Continue independent P23 implementation,
regressions and evidence upkeep, then prepare stack review/CI disposition.
Dependent live funding, trading and venue-backed recovery runs remain deferred
with their capability gates closed. The older investigation-first instructions
below are historical. This is not authorization to invent native credit or
qualify a fixture as live; a possible later simulated demo stays separately
labeled and isolated. Stack merge readiness is not full financial qualification.

<a id="merge-scope-and-deferred-acceptance"></a>
## Merge scope and deferred acceptance

The latest user direction authorizes completing the independent implementation,
tests and review/CI disposition, then merging the existing stack into
`product/tee-v1`. It does not authorize another venue/RPC/AWS invocation, invented
native evidence or closure of unpassed live criteria. No new phase or terminal
feature is a merge prerequisite.

| Workstream | Stack deliverable | Acceptance disposition |
| --- | --- | --- |
| Transport, concurrency and bounded history | Single-writer staging, opaque HTTP/WS, manifest-bound encrypted packs and 64-record regression coverage | Earlier exact read-only image passed the bounded hardware gate; retain that receipt, not an assertion about every subsequent financial image. |
| Original funding and demo setup/credit | Retained original identities, confidential chain/signing ports, strict authenticated parsers, explicit demo provenance and restart-safe journal state | Finish offline checks of the current source. Actual changed-image top-up/credit remains deferred. No synthetic credit or general readiness. |
| Governed allocation | Exactly one verified owner deposit and two fixed cash-only ingress operations; epoch/expiry, capacity and late-event checks | Finish offline controller/Runtime checks. Manifest v6 cannot dispatch unrelated funding, withdrawal, payout or trading. |
| Review and CI | Resolve valid findings against the integrated source, run offline regressions, require successful current-head hosted checks | Required before merging; old green heads are historical evidence only. |
| Native trading and perp funding | Existing execution/observation interfaces and exact offline financial engine | Complete causal coverage, actual hourly precision/allocation and measured trading activation remain independently unqualified; not all are caused by missing credit. |
| Native withdrawal and recovery | Existing once-only original withdrawal and fenced/funded final-claim contracts | Lost-native-ACK/no-later-effect, actual finalized return, final cut and live claim workflow remain independently unqualified. No recovery activation. |
| Residuals | Retained original receipts and exact test-fund inventory | 80 faucet USDP at the native vault remain uncredited/unreturned; a separate unlanded attempt's 20 USDP were last verified in its broker. Deferral is not return, credit or write-off. |

The phase remains `in progress` after an implementation merge. A later resumption
must identify the exact missing guarantee and fresh authorized run; it must not
resume a closed invocation or treat the merged code as financial qualification.

Earlier user decision, 2026-10-08: **implement a bounded testnet demo deposit mode**
using the original finalized transaction's exact deposit-history record and
non-pending balance corroboration, with explicit trust in venue bookkeeping.
[ADR 0029](../architecture/0029-demo-deposit-confirmation.md) defines that narrow
exception and its offline implementation. It supersedes the earlier wait-only
deposit instruction, not the withdrawal, order/funding or recovery contracts.
Demo confirmation releases only the original deposit lifecycle hold, is retained
separately from strong completion, and grants neither native risk readiness nor
recovery eligibility. Shipping activation and fresh actual-rail qualification
remain off; this is not yet an end-to-end funding pass.

Subsequent approval: initial idle testnet setup may use fresh authenticated
non-atomic settings/loan/account reads under explicit exclusive wallet/agent
control, disabled lending and zero debt/interest. ADR 0029 defines its separate
measured-policy opt-in, retained provenance and first-original-ingress scope.
This resolves the demo setup policy choice, not the strong production contract,
withdrawal completion, native risk readiness or recovery. No live activation
follows from approval or local tests.

Subsequent explicit approval covers the local one-time allocation grant in
private encrypted configuration. Its actual loaded binding and journal acceptance
are implemented offline: one verified original owner deposit, one fixed amount,
two fixed request IDs, customer epoch and absolute expiry. This closes the local
allocation-authority decision, not measured activation or live qualification.

The subsequent offline continuation splits financial network work from the single
journal writer and adds measured manifest v6 for **only these two retained grant
acceptances**. Versions 1–5 remain nonfinancial. V6 requires the private grant,
approved initial-demo setup, chain/packs and native reads; it rejects trading,
generic capture and unrelated funding/withdrawal/payout dispatch. This is local
composition, not G05 authority or a live funding pass. The existing collateral
admission rule exposed the fresh-start boundary subsequently resolved below.

The actual v6 configuration now uses explicitly read-only/demo-ingress
constructors: execution/fills/withdrawal stay `Unknown`, settings stay `Observed`,
and general readiness remains false. Normal qualified constructors are unchanged.
The user-approved fresh funding-only run began with initialization capital but
is **blocked before AWS**: the original 20-USDP Solana deposit finalized, while
the fresh native account/loan caches and original credit remain absent.
[The retained receipt](P23-NATIVE-SEMANTICS-RECEIPT.md#fresh-funding-initialization)
records the exact residual and bounded observations. No customer top-up, native
withdrawal, new vault/program or paid host occurred. Resolve that original;
do not resend or infer readiness from a successful chain debit.

The subsequent user approval authorizes one **independent additional** 20-faucet-
USDP staged diagnostic, with fresh identities and unchanged per-invocation caps.
[The current receipt](P23-NATIVE-SEMANTICS-RECEIPT.md#independent-second-initialization)
records the new seal and actual stopped outcome. The second original was RPC-
accepted but not observed on-chain before expiry; its full 20 USDP remains in
the disposable broker wallet. It is not a second native-credit failure. This
does not renew the first run, erase its missing-credit residual or grant
customer/Nitro readiness. Review bounded transaction delivery before a future
separately sealed attempt; no replacement signature or repeated funding is implied.

The latest user subsequently authorizes persistent independent devnet/testnet
diagnostics. [The multi-deposit receipt](P23-NATIVE-SEMANTICS-RECEIPT.md#persistent-credit-investigation)
records three additional **finalized** 20-USDP originals across owner-to-broker
and direct-faucet routes, with exact broker debit/vault credit/native events but
no native account or original credit after repeated and later checks. Independent
larger recent deposits also lack original matches while an older original's
history is available. This distinguishes current missing credit from the prior
unlanded submission; it does not prove a particular backend fault. Total native
residual is 80 USDP uncredited/unreturned, plus the separate unlanded broker's
last-verified 20 USDP. The customer top-up remains unused and no AWS host was
launched. Native setup/credit is still a real prerequisite, not a failed assertion
to bypass. No diagnostic policy grants shipping, trading or recovery readiness.

<a id="fresh-flat-account-admission-decision--pending-user-review"></a>
## Fresh flat-account funding admission — approved

**Decision, 2026-10-08:** the user approved option 2. The narrowly scoped rule is
implemented locally; current executed checks belong in TRACKER. It is not yet a
live funding pass. Trading, withdrawal/recovery and exact live authorization
remain separate gates. The stable older anchor above preserves existing links.

The original owning fresh-boot regression reproduced the missing prerequisite without
invented native/market evidence. A real-code finalized owner-deposit fixture and
the approved authenticated idle-account setup suffice to recognize the original
and establish **demo setup readiness**, but not to accept the allocation.
`funds_ready` requires a collateral cut; `collateral_capacity` requires
`qualified_diagnostics`, including a complete current native check and qualified
marks. The earlier allocation/chain fixtures explicitly seeded these prerequisites.
The shipping startup/read workers do not supply them. Ordinary funding still
correctly retains `ControlError::Unqualified`, with no intent, attempt or signature;
the scoped allocation now uses a distinct admission certificate instead. This is
not a new native API or a general marked-portfolio provider.

Options, without reopening the original amount/route/allocation decision:

| Option | Consequence | Recommendation |
| --- | --- | --- |
| Keep the universal marked/native-check prerequisite | Build and qualify the general risk/collateral provider before any original allocation. Do not label non-atomic setup reads as a complete native check. | Appropriate for trading and ordinary risk/payout paths; unnecessarily couples a flat funding-only demo to those unqualified semantics. |
| Explicit narrowly scoped flat-funding admission | Add a separate common-journal guard for this governed original Vault → Broker → Venue allocation. Derive capacity from the original qualified vault receipt, exact attributed cash/location/transit and the approved idle-account assumption, not invented marks or `NativeCheck.complete=true`. | **Approved and implemented locally** for this fixed demo only. Offline tests are not a mathematical proof or live qualification. |

The implemented scoped guard requires the exact private grant, current customer
epoch, unexpired first acceptance, configured owner/asset/route, retained approved
initial setup, zero user/house/native positions and funding, no trading/order
commitments, debt, active protection, unresolved attribution or unrelated funds
operations. Both fixed legs must fit exact physical and attributed capacities;
the second remains blocked until actual full first-leg completion. Existing
reservations, once-only identities, durable exposure, signing simulation/code/
counter/finality checks and restart rules remain mandatory. Reject rather than
fall back if those conditions change. A pending original may still reconcile;
failure/timeout must not erase it or authorize another attempt.

This is **a new scoped admission rule**, not a fake general collateral cut
or a blanket bypass of the financial engine. It must grant no trading readiness,
ordinary withdrawal/payout, recovery proof or extra allocation. Native risk
readiness and all stronger completion/cut contracts remain false/unqualified.
Fresh measured AWS/devnet activity still requires its own exact G05 review.

`funds::FlatAllocation` binds the actual loaded allocation commitment, applied
original vault receipt/amount and exact two intents. `FlatAccept` retains it on
only those operations. Acceptance reserves both customer cash and physical source
capacity; prepare/exposure recheck flatness, current epoch/expiry, unresolved
facts, holds and exact cash/location backing. The deposit also requires the first
movement's full zero-fee terminal evidence. Observations remain recordable if the
rule later fails, but there is no new signature or replacement allocation.
Journal engine revision 23 refuses revision-22 histories rather than silently
reinterpreting their authority; the service's loaded allocation binding is also
domain-separated from the earlier admission semantics.

No general financial gate was relaxed. Fresh SQLite/controller tests use the
actual owner-deposit recognizer and original codec/signing paths without seeded
complete native/mark/collateral cuts. The actual encrypted Runtime test separately
exercises both originals and a revocation during simulation; only its recorded
result in TRACKER is evidence. Strong withdrawal/trading/recovery prerequisites
are independent, and live activity still needs exact changed-image qualification.

The closed C5 run made **zero native and zero RPC connections**. It qualified
parts of transport/key/storage behavior, not deposits, venue credit, orders,
funding payments, payouts or recovery on actual rails. Historical M1/M2 results
do not grant current authority or qualify the changed measured application.

## Capability-to-code map

| Gate | Existing implementation | Missing connection / fresh evidence | Safe behavior until qualified |
| --- | --- | --- | --- |
| Account setup and lending | `funding::Setup`, `Controller::observe_setup`; `funding::setup` bounded authenticated GET/archive/replay and v5 scheduler wiring; explicit approved `initial_setup` policy and separate retained demo provenance; actual fresh test-capital initialization and empty-account readback outside Nitro | Strong setup remains unqualified. Initial-demo readiness is implemented offline under the named exclusive-control/cache assumptions; fresh changed-image/native qualification still required. A fresh account/loan 404 is not zero debt. Main pre-withdrawal staging remains failed; separate empty-account readback does not rewrite it. | `Setup.complete` and native-credit readiness stay false. Only initial original demo ingress can use this assumption; no old keys/credit or atomic certificate is inherited. |
| User deposit into Solana vault | `customer_deposit`, `chain_funding::deposit`, original finalized RPC/wire/account/code verifier, independent owner codec | Fresh devnet deployment, registered owner/recipient, mint/genesis/code/governance and original owner-signed deposit on the actual enclave RPC path. Present locator list is explicitly configured and bounded, not a general wallet-indexing frontend service. | No credit from a missing/unfinalized/ineligible transaction; eligible originals credit once. |
| Vault release and native deposit debit | Strong Controller/codec/signers/original reconciliation; approved private two-leg grant and scoped flat cash-only admission; writer-free financial stages and restricted manifest v6 | Qualify actual Runtime effects on a fresh approved image/route. The flat rule does not qualify the general collateral provider. | Versions 1–5 reject signing; v6 cannot authorize unrelated funds or trading. Flat-rule changes refuse new exposure/signing; ordinary funding still needs its stronger readiness. Originals take precedence; second leg needs actual first-leg completion. No replacement/repeat allocation. A deposit debit is not native credit. |
| Venue credit | Strong original-signature provider: `funding::Credit`, `Controller::observe_credit`; approved `funding::demo` GET/confirmation port and manifest-v5 loaded-policy/scheduler wiring; current deposit/balance GET shapes observed outside Nitro | Strong credit still needs qualified final total/cut. Demo needs fresh initialized idle account, actual eligible original deposit and changed-image qualification; current historical deposit is already withdrawn | No balance-only credit or recredit of old deposited-and-withdrawn funds. Unmatched/partial/ambiguous data stays pending; only the explicit demo action can use its weaker bookkeeping assumption. |
| Order admission and dispatch | API intents/reservations, joined risk kernel, Gateway signatures and durable attempts | Install bounded current risk/capital/market policy; qualify native authority/agent permissions and exact grids. Connect intents to the prepared execution scheduler on the measured path. | Manifest rejects trading; API RiskAdmission disabled. No arbitrary native HTTP signer is exposed. |
| Fills, cancellations and final order cuts | Ordinary observations, `observation::Coverage`, `ingest_covered`, exact execution-set checks | Qualified provider for complete account/order history and native causal frontier, including late fills/cancel and pagination/disconnect behavior | Ordinary ingestion does not upgrade fills into qualified complete cuts. Retain uncertain commitments; do not infer terminal safety from cancel ACK/LI/end of page. |
| Perp funding and precision | Exact kernel accrual/settlement; diagnostic funding history with `FundingQualification` gap | Actual hourly boundary, sign on both sides, quote/grid/rounding and gross customer allocation when the pooled venue position is flat; connect native evidence to exact engine events | Displayed cumulative funding or a native account payout cannot directly settle every private customer. No invented rate or boundary. |
| Venue withdrawal and ordinary payout | Original UUID/native withdrawal plan, retained HTTP-200 ACK → batch mapping and transfer-observation join; `funding::Withdrawal`, `observe_withdrawal`; finalized chain return and P15 recipient payout/counters | Fresh authenticated ACK/event capture; lost native reply before durable ACK remains unresolved. Original native debit/no-later-effect evidence plus exact finalized payment to allowlisted broker; then vault return and payment to registered customer. Fresh fees must reconcile. | Request ACK or linked transfer observation does not discharge customer claim. Missing/pruned history stays pending; successful payment credits once. |
| Recovery | P21 final-cut/backing/kit workflow, native route binding and compiled P17/P15 claims | Actual native authority fencing, complete final cut, reachable funds returned, coherent finalized frozen custody state, funded claims, authorized activation and independent kit delivery/claim | No heartbeat-only payout or fabricated final cut. Unknown native exposure/backing blocks readiness; operator-assisted recovery remains explicit. |

The quoted types are trusted ports, **not** attestations produced by constructing
a Rust struct. A provider may derive a certificate from independently qualified
venue semantics and authenticated observations; a special venue-signed or ZK
API is not assumed mandatory. Conversely a response hash only binds bytes—it
does not establish completeness, causal ordering, ownership or no later effect.
If current APIs cannot support a required guarantee, report that specific gap
and seek a bounded alternative or product-policy review. Do not silently weaken
the guarantee to obtain a demo pass.

## What should happen next

Current merge order, 2026-10-08: finish independent implementation and offline
tests, address relevant stack reviews, verify current-head CI, then merge the
existing stack. The [merge disposition](#merge-scope-and-deferred-acceptance)
supersedes the earlier live-work-first sequence. Deferred native readiness,
funding, trading and recovery qualification are not implementation-merge gates.
The live sequence below describes a later separately authorized resumption,
not a request to reopen the parked investigation or current invocation.
P23 remains in progress after merge; unavailable capabilities stay disabled.

The shipping continuation supplies manifest-bound packs and strict
[documented native evidence components](P23-NATIVE-EVIDENCE.md). Those parsers
do not connect or qualify a trusted financial provider. The
[approved storage run](P23-NEXT-RUN-PROPOSAL.md) now has a separate
[read-only hardware receipt](P23-PACKS-HARDWARE.md), including passing actual
Node/Chrome head-64 growth, storage faults, replay and freshness checks;
all money-movement and complete-cut gates below remain open.

1. Preserve the closed storage receipt and independently verified archive/owned
   cleanup as the baseline before financial journal traffic. The bounded growth
   gate passed; it is not production scaling or venue-semantic qualification.
2. Prepare authenticated setup/credit/withdrawal providers and their retained
   evidence contracts; connect them through the existing Controller. Reuse M1's
   scenario assertions and P22's fault cases, not its wallets, plaintext loaders,
   injected witnesses or prior permission. Start with account setup and an
   original-operation funding round trip, without trading.
   The offline capture transport/archive in
   [ADR 0027](../architecture/0027-native-transfer-capture.md) is the next bounded
   component: it retains raw transfer evidence without settlement or readiness.
   [Loaded-manifest/one-shot-worker wiring](../architecture/0028-measured-native-capture.md)
   is subsequently implemented and tested locally. Actual native qualification
   remains open; a local worker or TLS peer is not a deployed financial provider.
3. The narrow measured composition is now implemented offline as v6, with actual
   private-grant binding, two-original dispatch limits and writer-free financial
   I/O. The user-approved fresh cash-only admission is implemented locally;
   finish its owning checks and review exact changed-image scope before live use.
   Versions 1–5 reject funding; all versions reject trading. Native finality/
   causal providers stay independently gated; flipping booleans is not closure.
4. Seal a fresh G05 manifest and seek approval before AWS, devnet deployment,
   faucet/account activity, sponsor-wallet use or any native POST. Include fresh
   identities, funding source/caps, fees, request/byte budgets, hard stop and
   original-operation/cleanup procedures. The immediate candidate is funding-only:
   owner deposit → vault → broker → original native deposit → retained demo credit.
   This is one step toward the required round trip, not its completion. V6 cannot
   authorize native withdrawal or customer payout. Review an explicit test-only
   asset containment/return procedure before funding; do not strand funds or use
   it to fabricate shipping withdrawal completion. Native withdrawal → vault →
   owner payout and lost-ACK reconciliation retain their independent gates.
5. Add bounded order lifecycle and funding qualification after funding round-trip
   evidence; then actual fenced/settled/backed recovery. Some observer semantics
   can be prepared independently, but trading cannot bypass credit/cut gates and
   recovery cannot bypass terminal history or returned backing.
6. Carry the existing Node/browser SDK and service qualification workflows onto
   those actual capabilities. A product trading terminal, market-data UI and
   self-service browser/agent UX remain separate follow-ups, not P23 merge gates.
   Clearly identify any separately demonstrated read-only/simulated UI. P24's
   release/customer-funds review and a new venue integration remain separate.

<a id="next-changed-image-funding-only-review"></a>
### Next changed-image funding-only review

The user subsequently approved one host at $5/four hours/eight boots, one 20-USDP
customer top-up and at most 20 separate USDP for initialization, with explicitly
diagnostic return after fencing and no trading. That approval is not itself an
actual sealed application manifest or a completed prerequisite. Initialization
is currently blocked as recorded above; no AWS session has begun.
Finalize actual source/build measurements and identities only after local checks;
do not inherit an earlier invocation's resources, accounts, image or authority.

| Required boundary | Next invocation must bind |
| --- | --- |
| Source and restore | Exact default-feature source/package/EIF/PCRs, v6 role/policy envelopes and a fresh engine-23 journal; no history reset or revision-22 migration |
| Customer original | One disposable owner, registered recipient, exact asset/amount, deposit nonce and finalized original signature; current devnet genesis, vault/native program code and governance |
| Setup and custody | Fresh testnet account plus explicitly authorized test-capital initialization/return if required; inventory of every wallet/agent authority; disabled lending, zero debt/interest and no exposure/orders; no customer funding until retained setup qualifies |
| Allocation | One fixed amount, two distinct fixed requests, customer epoch, absolute expiry and actual loaded private grant; no API-created substitute or repeated allocation |
| Effects | Only the two original ingress sends. First-leg full completion precedes deposit; credit uses the approved original-signature/history/balance demo contract; no trading or ordinary withdrawal/payout/recovery readiness |
| Fault evidence | Original-ID restart/uncertain chain reply without replacement signing; pre-signature revoke/freeze refusal. Do not turn a destructive live fault into an economic retry |
| Resources and exit | Separately accepted AWS/funds/request/time caps, teardown ledger and exact test-only containment/return authority. Unknown economic execution stops; record residual assets and unresolved claims rather than report a clean round trip |

Strong withdrawal, hourly funding/execution cuts and final recovery evidence are
not supplied by this review. Complete their independent local work where possible,
then obtain the smallest missing semantic/policy decision rather than silently
borrowing the approved deposit exception.

<a id="funding-completion-contract-decision"></a>
## Funding completion contract decision

Earlier decision on 2026-10-08: **keep the current contract and wait for stronger venue
evidence**. The user explicitly declined the operation-scoped alternative below;
it is retained as considered context, not an implementation instruction. The
later approved deposit-only demo mode is recorded above and in ADR 0029; the
following investigation describes the stronger contract, not a stop instruction
for implementing that mode. This is
the first-workstream security-contract boundary, not another completion milestone.
Rechecking the [withdrawal API](https://docs.pacifica.fi/api-documentation/api/rest-api/account/request-withdrawal),
[transfer stream](https://docs.pacifica.fi/api-documentation/api/websocket/subscriptions/account-transfers)
and [balance history](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-balance-history)
confirms documented UUID deduplication and positive transfer evidence, but no
documented perp UUID lookup after a genuinely lost ACK. The inspected schemas do
not yet justify final native credit/debit totals, no-later-effect semantics and
the native cuts required by the present `Credit.cut`, `Withdrawal.cut` and
`funds::Terminal.coverage` ports. This is not a claim that Pacifica is incapable,
or that a special signed/ZK API is necessary.

The subsequent [read-only contract/source investigation](P23-NATIVE-EVIDENCE.md#completion-contract-audit-2026-10-08)
corrects two earlier overstatements. Funding completion is already scoped to one
original operation, **not a universal account/trading frontier**. Perp
deposit/withdrawal-history and pending-withdrawal methods are documented in the
official MCP tools/client, and historical withdrawal responses worked. Their
inspected rows contain batch/payment identities but no original UUID; a final
page is not a native no-later-effect guarantee. Existing finalized deposit and
batch-completion events strengthen chain evidence without proving off-chain
credit/debit completion. Current program source equivalence remains unverified.
No alternate proof kind, relaxed guarantee or financial activation was adopted
at that investigation checkpoint. The later demo marker does not qualify these
stronger native completion ports.

The read-only instruction check in
[the existing receipt](P23-NATIVE-SEMANTICS-RECEIPT.md) now independently matches
the retained ACK batch to actual on-chain payment bytes. It does **not** justify
using that batch nonce as a frontier for other deposits, withdrawals or trades,
nor correlate a UUID whose ACK was never retained.

Considered alternative (**not approved**): an explicit **operation-scoped completion contract**
for the bounded testnet funding provider, separate from complete native account
history. Its acceptance relies on qualified once-only venue processing of the
specific deposit and UUID, plus source-authenticated positive evidence. Those
are named native-correctness/idempotency assumptions, not a universal proof from
a finite observation window. A decoder, ACK, balance delta or hash alone cannot
produce the completion. The proposed evidence sets are:

| Original operation | Required positive evidence | Uncertainty remains contained |
| --- | --- | --- |
| Deposit | Durable original attempt/wire; exact finalized original native-program effect; authenticated broker/asset transfer event linked by that signature; qualified full credit and actual fee matching the original gross | Missing/partial credit, source mismatch or conflicting duplicate stays unresolved. No credit from an amount/time match or substitute signature. |
| Withdrawal with retained native ACK | Durable original UUID/request and successful authenticated ACK; unique batch binding; authenticated confirmed transfer's gross/fee/net; batch-bound finalized native-program payment to the allowlisted broker; qualified once-only debit/payment semantics | Missing debit/payment evidence or ambiguous batch stays pending. A confirmation label is not chain finality and advertised fees are not automatically actual fees. |
| ACK lost before persistence | No completion certificate from the current interfaces | Retain original exposure/holds; freeze dependent admission/payouts and reconcile the original through a genuinely supported identity lookup if it becomes available. No resend, new UUID, amount/time attribution or audit-copy promotion. |

An implementation would encode this proof kind explicitly in the common journal,
validate the complete operation receipt set/totals once, preserve the evidence on
restart and reject conflicting late effects. It must **not** fill `through` with
an invented timestamp, local journal counter, native batch nonce or Solana slot
for an unrelated native source. Existing causal certificates retain their
stronger meaning. Refused or insolvent states keep correct actual postings and
unpaid claims; this change grants no new customer-loss allocation or payout right.

The alternative is to keep the current contract disabled until stronger native
semantics/evidence can satisfy it. Neither option proves automatic recovery of a
lost native ACK. Native account bootstrap/readiness also needs positive disabled
lending and zero debt/interest before customer funding; stage any bootstrap with
house-owned test funds rather than importing the diagnostic deposit-first exception.

The stronger completion ports and shipping financial gate remain unchanged.
The later ADR 0029 explicitly changes serialized demo lifecycle state; it does
not approve the broader withdrawal alternative considered here. Do not treat
standing small-test-correction authority as approval for that alternative.
To qualify strong settlement, obtain independently
qualifiable original-operation completion/correlation semantics that satisfy the
current ports, with retained authenticated provenance. No special venue-signed
API is required in principle; inventing a frontier or promoting a response hash
is still invalid. If available documentation cannot establish that contract,
request those exact native semantics/interface details from the venue rather
than starting another uninformative economic run.

The concrete evidence request is narrowly about the existing rails:

1. For an original deposit signature, which authoritative native record identifies
   its economic credit(s), actual fees, final credited total and completion? What
   source-local ordering/coverage semantics does that record establish, and can
   it be retrieved after stream disconnection without amount/time matching?
2. For an original withdrawal UUID, can its accepted/rejected/pending/completed
   result and batch/payment identity be queried after the ACK is lost? Which
   authoritative record establishes the final total native debit/fee/payment
   and no remaining effect, and what are the UUID retention/expiry semantics?

Retain the primary reference or reproducible source-authenticated result for each
answer; a vendor statement alone is not an implemented verifier. Unsupported
answers remain capability gaps. Do not send a message or run an economic test
merely because these questions are listed here.

### Effect on the remaining workstreams

| Existing workstream | Current dependency / safe next action |
| --- | --- |
| Native providers/readiness | Implement the approved demo deposit port's measured policy/scheduler binding and actual schema/readiness qualification. Separately obtain stronger final-credit/debit and lost-ACK semantics; demo deposits do not qualify them. Keep unreviewed activation disabled. |
| Measured funding round trip | Requires that provider and fresh exact-source G05 approval for the vault deployment/Nitro/customer round trip; the standalone native probe is not this pass. |
| Bounded trading/funding | Admission depends on reconciled actual native credit. Complete order lifecycle and hourly gross funding still require their own native semantic tests, not funding-operation certificates. |
| Operator-assisted recovery | Depends on fencing and reconciling original native operations, actual returned backing and a final claim cut. Existing offline fault/claim tests remain useful, not a live recovery pass. |

No independent new live work is authorized by the closed native probe. Safe local
corrections/tests have continued; prior-stack housekeeping remains deferred as
directed. The latest user decision is recorded above and in TRACKER. Do not ask
the same deposit-policy choice again, silently select the declined withdrawal
alternative or stop approved demo work on the superseded wait-only instruction.

## Minimal evidence receipt for each provider

[The next native-semantics proposal](P23-NATIVE-QUALIFICATION.md) scopes fresh
setup/deposit/withdrawal/lost-native-reply qualification without an exploratory
AWS session. Its faucet/devnet and diagnostic bootstrap bounds were approved
2026-10-08. The [native-semantics receipt](P23-NATIVE-SEMANTICS-RECEIPT.md)
records the actual original deposit/payment round trip and late setting readback,
not shipping completeness, lost-ACK recovery or a measured funding pass.
Further financial/native/AWS activity still needs its applicable exact G05 record;
standing minor-correction authority never permits uncertain economic retries.

Record the bound environment/account/profile and source version/date, request and
response schema, authenticated channel, exact native/chain identity correlation,
event/frontier/finality and units, uncertainty/duplicate semantics, retained raw
evidence and negative tests. Private accounts, keys, signed wires and credentials
stay out of Git. Record exact source and distinguish documentation, synthetic
tests, historical runs and fresh actual observations.

This map makes the current gaps explicit; it neither declares the APIs incapable
nor presents an offline fixture as authentic native evidence. It does not add
new P23 milestones. The existing financial lifecycle and cleanup gates remain
open until their own actual receipts exist.
