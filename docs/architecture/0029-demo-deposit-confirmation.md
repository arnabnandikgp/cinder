# ADR 0029 — Transaction-linked demo deposit confirmation

Status: user-approved testnet-only policy, implemented and tested offline on
2026-10-08, including loaded-policy and scheduler wiring. Not financial
activation, venue qualification or a measured live pass.

## Decision and scope

The user approved a practical MVP path while stronger Pacifica completion evidence
is unavailable: poll venue bookkeeping for the original deposit, corroborate
available collateral, and explicitly trust that bookkeeping for the demo. This
supersedes the earlier wait-for-evidence decision **only for this deposit mode**.
The stronger `Credit`, `Withdrawal`, `Terminal` and recovery contracts do not
change. No internal crossing, second ledger, new financial phase or customer-loss
policy is introduced.

Balance-only observation is not enough. The first bounded implementation requires:

1. The original durable Broker → Venue plan and independently verified successful
   finalized Solana deposit signature. A debit alone does not credit the venue.
2. A source-authenticated GET from the fixed testnet account's deposit history,
   carrying that exact signature and the full original quote amount.
3. A subsequent balance-history response whose unique latest eligible deposit or
   deposit-release row shows the same full amount and available balance, with
   zero pending balance. Its timestamp must be coherent with the deposit and
   actual receive time. `include_trades=true` prevents deliberately hiding trades
   from that query; it does not prove native history completeness.

This is deliberately the **first credit into an otherwise idle native account**:
zero prior native cash/funding/exposure, no live orders, no competing native money
movement, no unresolved raw evidence and positively qualified account setup.
Zero deposit fee and full credit are required. Partial credit, conflicting records,
inexact precision, missing fields, unrelated activity or changed scope remain
pending or refuse; there is no amount/time match or silent fallback. Later
deposits into a trading account need an explicit extension, not reuse of this
fresh-account balance predicate.

## Trust and evidence

The demo assumes Pacifica correctly associates the original signature with one
full credit and reports usable collateral correctly. Finite polling does **not**
prove once-only future behavior, absence of later effects or a final account cut.
TLS authenticates the source, not its economic correctness. Primary provenance:

- [Official MCP tools](https://docs.pacifica.fi/api-documentation/api/mcp/tools)
  and the [pinned account client](https://github.com/pacifica-fi/pacifica-mcp/blob/4748feca93efe2b2a5a0c95993e40f68d0ca4338/src/tools/account.ts)
  identify `/api/v1/account/deposit/history`, account/cursor queries and example
  `amount`, `transaction_id`, `created_at` fields. The initial fixtures used these
  documented client examples. The separately recorded current GET observation
  below now confirms this shape, not economic finality or a fresh deposit.
- [Balance history](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-balance-history)
  documents amount, balance, pending balance, event type, time and pagination.
  It does not independently identify the deposit signature.
- [Account transfers](https://docs.pacifica.fi/api-documentation/api/websocket/subscriptions/account-transfers)
  documents an optional deposit `tx`. Absence is not evidence of rejection.
  This implementation uses retained GETs, not an invented stream replay guarantee.

The [separate 2026-10-08 read-only receipt](../implementation/P23-NATIVE-SEMANTICS-RECEIPT.md#separate-current-testnet-schema-observation)
confirms actual testnet deposit/balance response shapes and established-account
predicates through Node TLS, outside Nitro. It also observes default settings
despite account/loan 404 for an uninitialized address. Fresh account bootstrap,
shipping provider/precision and changed-image confirmation still need qualification.
Unsupported shape stays pending; do not broaden a parser just to make a run pass.

The subsequent [fresh staged-test receipt](../implementation/P23-NATIVE-SEMANTICS-RECEIPT.md#separate-staged-bootstrap)
observes initialization and a full native asset round trip, then a separately
sealed successful lending-disable/readback on the empty account. The initial
staging sequence failed on a diagnostic `account_equity` mapping error and stays
failed; its withdrawn original is not new customer credit. Current field/exposure
fixtures refine parsing only. Shipping setup, original route creation and fresh
measured confirmation remain separate gates; no test key/history is reused.

## Durable implementation

`pacifica::funding::demo::Policy` has no default/mainnet equivalent. Its nonzero
revision binds the Controller contract, actual Gateway release/profile and all
polling limits. Both objects must name the exact testnet origin and a qualified
precision profile. Changing policy for an existing original attempt refuses.

`prepare_demo_deposit_poll` returns one prepared GET or Waiting/Exhausted/Confirmed.
It never opens a socket or resends a transfer. Preparation atomically retains the
request's endpoint/cursor/attempt/policy with the shared Gateway read spend before
I/O. `reads::complete` rejoins the same writer and retains bounded private reply
bytes separately from economic inputs. Missing replies still consume their read
budget. Shared HTTP-429 cooldown persists even with the extra polling marker.

Cadence, exponential backoff for unavailable/invalid/negative replies, page progress,
cursor-loop rejection, original deadline and total GET attempts rebuild from that
journal after restart. Limits are 2–64 GETs, 1–8 pages per deposit scan, a minimum
one-second cadence, a maximum 30-second backoff and a maximum ten-minute lifetime.
Every page counts as a GET. Reaching a cap requires reconciliation; it does not
release the hold. Example values for a future bounded demo (not live approval):

```text
revision=1, maximum_reads=32, maximum_pages=4,
interval_ms=2000, maximum_backoff_ms=15000, lifetime_ms=600000
```

Responses are capped at 8 KiB before journal encoding, with at most 32 rows per
page. Oversized responses refuse without a partial parse or credit. The measured
record allowance reserves worst-case JSON byte encoding plus frame overhead;
the storage backend still independently enforces remaining history/write limits.

`confirm_demo_deposit` posts the full original credit and
`funds::Action::ConfirmDemoDeposit` together. The economic key is derived from the
original signature, never the polling request, timestamp or amount. The common
journal requires exactly the retained finalized Broker debit and uncut Venue
credit, equal debit/settled/arrived totals, zero fees and the original exposed
attempt. It stores `Operation.demo` separately from `Operation.proof`, releases
only this deposit's lifecycle hold, and retains the weaker policy commitment.
Customer cash does not increase a second time when collateral moves to the venue.
Reconfirmation/restart does not post another credit or make another GET.

No native source cut is invented. Demo provenance remains private journal evidence;
it does not manufacture an unresolved raw economic event. A contradictory late
economic observation is retained and faults the operation rather than leaving it
healthy. The marker cannot be upgraded to a strong terminal proof; both recovery
sealing and final-cut extraction explicitly reject demo-backed operations. Native
risk readiness is **not** automatically granted by this confirmation.
The strong credit provider likewise rejects an existing demo operation before
posting another credit or making it risk-ready. The existing enclave TLS read
transport accepts only the fixed testnet deposit-history route for this new GET;
no mainnet counterpart, arbitrary host or signing endpoint is added.

Journal engine revision **22** includes this action and state commitment. Revision
21 archives refuse without an explicit migration; closed invocation archives and
their results are not rewritten. A measured test must use its reviewed current
source and fresh journal, not reopen a historical run.

## Measured service composition

Manifest version **5** explicitly selects this bounded deposit-observation mode.
It requires chain routing, retained-pack limits, native reads and the exact demo
policy; it rejects generic WSS capture, funding and trading activation. Versions
1–4 omit the new field and retain their prior release encodings. Version 5 uses
its own manifest domain and `CKR5` role envelopes, with the same six separate
roles, not a new signer or unrestricted master.

The application commitment includes the actual loaded Controller/Gateway scope
and policy. `Loaded::open` compares the consumed policy to the manifest before
clock/cloud preparation; echoing an old application hash cannot substitute new
limits. Mainnet/unqualified precision refuses construction. The boot lease must
cover the policy lifetime and the configured poll interval must not exceed the
policy cadence.

The scheduler selects only the unique existing original Broker → Venue deposit
from the authoritative journal. Missing original plan, finality or positive fresh
setup waits without a GET; conflicting native operations or faulted history refuse.
It never creates a transfer, signs a replacement, supplies setup evidence or
enables API risk admission. This mode skips the generic diagnostic poll rotation:
unrelated raw observations cannot be silently marked reconciled to make an idle
account eligible. Existing chain customer-deposit observation is unchanged.

One durable spend precedes one authenticated enclave GET. The journal guard is
released during the exchange; private commands/reads can proceed. Completion
rejoins the current writer, checks lease/time/policy, archives the response, then
attempts the atomic confirmation. If both responses were retained before a crash,
the scheduler can confirm without another GET. Lost replies, backoff, 429 and
budget exhaustion retain the original hold; resetting volatile scheduling state
does not renew the durable budget. Expired/fenced workers cannot publish a credit.

## Verification and remaining boundary

### Existing authorized original chain composition — 2026-10-08

`chain_funding::Port::issue_demo_ingress` explicitly selects the approved initial
setup assumption for an **existing accepted and prepared** Release/Deposit.
`validate_demo_ingress` checks that retained assumption and the actual prepared
movement/authority/expiry before RPC. Actual chain counters, code/governance,
current financial gates and setup are checked again at exposure. The same
simulation, separated signer, exact signed-wire persistence and original-only
submission/reconciliation path is used. Unsupported rails, recovery, absent or
changed policy and failed setup refuse; there is no strong-path fallback.

The dormant scheduler selects these explicit ports for the loaded initial-setup
opt-in, preserving pending-original precedence. Versions 1–5 still reject
financial activation. This is not an allocation-authority decision: no new
intent, customer signature, working-collateral target or customer credit is
created. A new governed allocation grant requires separate explicit approval,
actual loaded-policy binding and fresh live scope. The proposal was initially
rejected and not applied; the subsequent explicit user approval and bounded local
implementation are recorded below. This does not rewrite the earlier checkpoint.

Joined offline tests use the actual codec/signing/retention/receipt controllers
with synthetic RPC, setup replies and time. Both original ingress rails and lost
submission-ACK replay pass; no native credit follows from a chain deposit debit.
Changed/omitted policy, stale setup, revoked authority, freeze and non-ingress
refuse before RPC. Failed simulation/excess fee retains the unsent original plan,
never submits, and cannot close it before expiry plus verified no-wire history.
These checks are not live authority, genuine native evidence or changed-image
qualification. Strong setup/credit, withdrawals and recovery remain unchanged.

### Approved one-time internal demo allocation — 2026-10-08

The user explicitly approved local implementation/testing of exactly one working-
collateral allocation. `runtime::Configuration.demo_allocation` contains the
original owner-deposit locator, fixed quote-atom amount, distinct Release/Deposit
request IDs, current customer authority epoch and absolute expiry. It is private
encrypted configuration, not a public manifest, health field or API command.
Omission preserves previous configuration encoding and authority. The actual
Controller, Gateway, demo policy and chain component bind the application digest
and KMS context; the original must be the sole configured eligible deposit.
Mainnet, missing initial-setup opt-in and unqualified precision refuse loading.

`demo_funding::Authorization` checks the exact retained original deposit receipt,
its attributed amount, current customer cash, flat private/native state, fresh
approved demo setup and existing journal risk/capacity/liquidity controls. It
accepts an ordinary P08 intent through a private governed port. Its `Approval`
means internal configuration authority, **not** a claimed wallet signature. It
does not itself sign, open a socket, prepare an attempt or create customer credit.

Two stable journal acceptance identities plus encrypted provenance prevent a
restart or changed grant from creating another allocation. Pending originals
remain reconciliation-only. The second intent requires actual full first-leg
debit, arrival and terminal completion, with zero fees/returns/impairment; a
cancelled release or rejected acceptance cannot authorize it. New acceptance
rechecks epoch, expiry, freeze/recovery and source backing. Exactly the configured
full amount moves Vault → Broker → Venue, with no partial or repeat allocation.
Demo credit still cannot become strong completion, native risk readiness or
recovery backing.

Seven focused offline regressions use the original-deposit recognizer, durable
SQLite journal, actual controllers/codec/signers and synthetic RPC/time. The
actual release-preparation process tests policy substitution before clock/cloud
access and absence of customer details from the public manifest. Its one-owner
demo configuration fits the existing 4-KiB KMS role envelope; the larger inherited
two-owner fixture explicitly refuses rather than increasing that limit. These
are local implementation checks, not measured or financial live evidence.

The subsequent funding scheduler connects the grant to writer-free financial
stages. Shipping manifest versions 1–5 still reject funding/trading activation;
v6 enables only the two original retained grant acceptances, requires private
allocation plus initial setup/chain/packs/reads, and forbids unrelated funds,
generic capture and trading. Its distinct release/KMS envelope preserves legacy
encodings. Context/code/counter RPC, simulation, submission and finality happen
outside the writer; current authority is rechecked before exposure/signature,
exact original wire is persisted before one-use delivery, and completion rejoins
the current head. Late native ACKs do not lose original correlation merely because
another writer/revocation advanced the head. Native requests cannot send expired
HTTP bytes after TLS. None of this manufactures a financial provider.

The original fresh-boot regression exposed the collateral prerequisite that earlier
fixtures seeded: the approved owner deposit and idle setup are not a complete
native check/marked collateral cut. Allocation correctly refuses with no intent
or signature. The [scoped flat-funding rule](../implementation/P23-FINANCIAL-GATES.md#fresh-flat-account-admission-decision--pending-user-review)
was subsequently approved and implemented as the separate certificate below,
not as fictional evidence or a change to ordinary funding admission.
Fresh G05 scope/approval remains required for live transfers or an AWS run. No
withdrawal, payout, trading or public funding authority is added by this grant.

### Approved cash-only admission certificate — 2026-10-08

The user approved option 2 for local implementation and testing. The journal
retains `funds::FlatAllocation` only on the fixed grant's two `FlatAccept`
operations. Its fields bind the actual loaded application/allocation commitment,
the applied original owner receipt and amount, and both exact intents. The
certificate is a trusted internal port; a nonzero digest alone is not an
authentication proof and no customer API can construct this action.

Acceptance, preparation and exposure require current customer epoch/expiry,
exact policy/domain/units/route, zero user/house/native positions and funding,
no unexplained facts, other orders/holds/attempts, active protection, recovery
or installed marked-risk/collateral model. All customer cash must be attributable
to the named original; house/suspense and native cash are zero. Physical vault
and broker cash must exactly match the applicable leg, with no spendable transit.
The adapter additionally rechecks the separately approved idle native setup and
its debt/lending/exclusive-control assumptions. Both customer cash and source
liquidity are reserved; the existing capacity checks are not skipped.

The second leg requires the first original's strong full terminal evidence,
exact receipt set and zero-fee debit/arrival/settlement, with no return or
impairment. Current conditions are rechecked before signing after RPC simulation.
Once a durable wire has been released, adverse facts and original reconciliation
remain recordable; expiry/revocation cannot authorize a replacement signature.
Deposit credit retains the earlier weaker demo marker, never strong completion,
native risk readiness, ordinary payout or recovery authority.

The allocation digest uses `CINDER-LOADED-DEMO-ALLOCATION-2`; journal engine
revision 23 commits the retained certificate and refuses revision-22 financial
histories. There is no automatic migration, pruning or reopening of historical
live streams. Fresh allocation fixtures remove their former synthetic complete
native check, marks and collateral cut. The actual encrypted-runtime regression
also checks two original sends and intervening revocation during simulation.
Executed evidence belongs in TRACKER; none is hardware or a live financial pass.

### Approved initial demo setup — 2026-10-08

The user separately approved **initial idle testnet setup readiness** from fresh
enclave-authenticated Settings → Loan → Account replies, under exclusive control
of every relevant wallet/agent authority and no unresolved setup operation.
This is distinct from the earlier deposit-confirmation approval. Separate REST
reads are not atomic and TLS does not certify cache correctness. This explicit
testnet venue-read assumption is acceptable for the first bounded demo deposit;
it is neither a production setup certificate nor a solvency/exit proof.

Opt-in is `demo_deposit.initial_setup = { revision: 1, exclusive_control: true }`
inside the approved measured policy. Omission preserves the original stronger
setup requirement and canonical policy bytes. The exclusive-control statement
requires run-level key/agent inventory and no external writers; it cannot be
established by these three replies or a caller-selected boolean alone. Freshness
uses the route's existing `setup_max_age`; polling uses the existing finite
budget/cadence/lifetime. Missing caches/fields, stale or contradictory replies,
enabled lending, nonzero debt/interest, incompatible margin, initial positions,
orders, stop orders, pending movements or spot balances cannot qualify.

The implementation records a distinct demo setup assumption with policy, route/
authority binding and the three retained read identities. `Setup.complete` and
native risk readiness stay false. Only an already authorized initial ingress can
use the explicit demo path; normal strong ingress cannot borrow the assumption.
Original attempt/wire persistence and all financial authorization, liquidity and
risk checks remain. A completed preflight cannot be refreshed to authorize a
second native deposit. The original qualified deposit may still be reconciled
after preflight age expires, within its own original confirmation deadline;
expiry never licenses another signature, transfer or release of an unknown hold.

Startup initialization, if the venue returns missing account/loan caches, must
finish via separately authorized test/house bootstrap before this preflight.
No automatic settings mutation or customer-funded initialization is introduced.
Withdrawal, lost-ACK, terminal-cut and operator-assisted recovery contracts are
unchanged. Financial activation and fresh changed-image/native qualification
remain separate gates; this approval does not authorize a live invocation.

### Authenticated setup reads and original ingress preparation

The 2026-10-08 offline continuation adds `funding::setup` and the measured-v5
scheduler connection. It uses only fixed testnet Settings → Loan → Account GETs,
charged before I/O and archived as private non-economic evidence. The setup
scan is separately domain-bound to the existing explicit demo policy, Controller
route/epoch and Gateway profile. Its finite GET cap/lifetime reuse that policy's
bounds; the deposit scan retains its own original-attempt budget. These are two
bounded read budgets, not permission to double a previously approved live budget.
Lost replies and HTTP 429 consume original requests; restart does not reset
cadence, cap, lifetime or the shared Gateway cooldown. At least three successful
reads are necessary for one observed round. Missing loan cache is not zero debt.

The current primary [account](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-info),
[settings](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-settings)
and [loan](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-loan-info)
schemas were rechecked on 2026-10-08. Account parsing uses `account_equity`,
stop-order counts and spot/margin fields; malformed amounts, future source times,
missing required fields and contradictory error envelopes do not produce a
snapshot. Native `updated_at` can remain old on an idle account: receive age
bounds this read observation, not an invented economic frontier. Parsed lending,
debt, margin-compatibility and idle predicates remain observable, including
adverse values. No private values are emitted in Debug output.

`observe_setup_reads` derives its input from those exact retained responses,
posts an idempotent `Setup` with **complete=false**, and leaves native readiness
off. Socket work does not hold the journal writer; completion checks current
boot/clock/policy and rejoins the current head. A caller-constructed snapshot or
three positive GETs cannot supply the stronger certificate. This is deliberately
the authenticated observation half of the stronger provider. The subsequent
explicit `initial_setup` policy records separate weaker demo provenance as
described above; it never changes `complete` or the stronger trust contract.

`Controller::prepare_ingress` prepares only an **already accepted** Vault → Broker
or Broker → Venue intent. Existing journal authority, risk, liquidity and
reservation checks remain authoritative; full original amounts and zero ingress
fees must fit the immutable route. Missing/stale/incomplete setup, recovery,
expired/completed/faulted/previously attempted operations and unsupported rails
refuse. It creates no entitlement, approval, signed wire or send. The existing
funding-gated scheduler calls this only when no original attempt is pending;
pending attempts are reconciled rather than replaced. Versions 1–5 still
reject that financial gate; v6 may dispatch only the private grant above.
This is not automatic allocation of arbitrary deposited funds
or a new customer transfer endpoint.

The production setup question remains: which independently qualified native
and exclusive-authority semantics establish no unresolved setup/configuration?
For the demo, the separate approved policy above resolves the initial-readiness
choice without asserting atomicity. `prepare_demo_ingress` and
`expose_demo_ingress` tag only an already accepted first release/deposit with
that provenance. Unknown prior attempts, expiry, adverse setup and another
allocation refuse. Original wire persistence rechecks authority and preflight
expiry; later reconciliation does not authorize another send. Strong ingress
cannot borrow this policy, and a demo-tagged plan cannot accept stronger credit.
Neither contract changes withdrawal, final-cut or recovery requirements.

Owning regressions are `demo_tests` in `crates/pacifica/tests/funding.rs`, plus
journal revision/replay tests in `crates/journal/tests/durability.rs`. They cover
full original credit, duplicate rows and confirmation, restart, missing replies,
bounded reads/backoff/pages, cursor escaping/loops, mainnet/policy rebinding,
missing original finality, partial/conflicting credit, pending/ambiguous/unrelated
balance effects, stale/future/malformed/late replies, HTTP 429, late economic
contradiction and attempted proof upgrade. Exact executed checks are in
[TRACKER](../implementation/TRACKER.md), not implied by this list.

Versions 1–5 still reject financial activation. Measured scheduler and
loaded policy binding are implemented locally; current response shapes now have
a separate GET-only receipt. Authenticated setup observation and original accepted
ingress preparation are now wired locally, with the separately approved weaker
initial setup path. Original intent allocation is now separately approved and
implemented locally. Writer-free financial I/O and narrow v6 composition are now
implemented/tested offline. The user subsequently approved the flat-account
admission certificate above; its local implementation does not close changed-image
qualification or establish a live end-to-end funding pass.
The service tests use real Runtime/API/AEAD/replicated-journal paths with synthetic
prerequisites and GETs, not actual chain signatures or venue confirmation.
This is not yet a working deployed deposit service. The approved policy does not relax
withdrawal debit/payment completion, lost-ACK containment, order/funding semantics
or final recovery-cut requirements. Those remain explicit in the
[financial continuation map](../implementation/P23-FINANCIAL-GATES.md).

<a id="funding-only-capability-construction"></a>
## Funding-only capability construction — 2026-10-08

Building the actual v6 configuration exposed an implementation mismatch: ordinary
Gateway/Controller construction required qualified execution, fills, withdrawal
and strong settings even though the approved release must not qualify them.
Supplying fictional `Qualified` values is not a remedy.

Separate `Gateway::new_read_only` and `Controller::new_demo_ingress` constructors
now admit only the narrow testnet profile: precision/chain route remain qualified,
execution/fills/withdrawal remain `Unknown`, and settings remain `Observed`.
Exact keys, units, grids, credits and immutable profile/policy binding still
validate. The loaded application commits those actual levels. Only manifest v6
selects this construction; ordinary construction and v1–v5 stay unchanged.

The Gateway may spend its bounded authenticated GET budget but refuses all
order/cancel/restoration dispatch before exposure. The Controller permits only
an already authorized original Release/Deposit with explicit demo setup. Native
withdrawal and chain broker return refuse at planning/signing, and `ready` stays
false even if a caller supplies `Setup.complete=true`. This does not introduce a
new grant, relax setup, bypass flat admission or qualify an unavailable provider.

Owning tests use the same read-only/ingress composition for all twelve allocation
cases and the joined actual Runtime/encrypted-journal workflow. They cover
capability-promotion/mainnet/unknown-precision refusal, no dispatch/I/O/exposure,
ordinary funding/return/payout borrowing and retained unqualified readiness.
Executed results are in TRACKER. External I/O there remains synthetic. The
[fresh live initialization](../implementation/P23-NATIVE-SEMANTICS-RECEIPT.md#fresh-funding-initialization)
stopped before AWS because its successful original chain deposit had no native
credit; this correction is not a live funding pass.
