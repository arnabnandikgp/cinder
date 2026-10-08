# Pacifica native evidence contract

Updated 2026-10-08. Documented schemas and offline tests, **not fresh venue
qualification or financial activation**. This supplements
[the financial gates](P23-FINANCIAL-GATES.md); the approved ledger, custody and
recovery contracts are unchanged.

Latest deposit-only exception: [ADR 0029](../architecture/0029-demo-deposit-confirmation.md)
implements the user-approved testnet bookkeeping assumption with explicit journal
demo state (introduced in engine revision 22; current revision 23 also retains
the separately approved flat-only allocation certificate). It requires an original finalized signature,
matching full deposit-history credit and available balance evidence on an idle
initialized account. These local tests neither observe the current endpoint nor
establish final/no-later-effect semantics. Strong `Credit`, `Withdrawal` and
recovery cut ports remain unchanged. Version 6 has a locally implemented two-leg
allocation gate, not qualified live financial activation; earlier versions stay off.

## Primary sources

| Source | Useful evidence | Not established by this source |
| --- | --- | --- |
| [Account transfers](https://docs.pacifica.fi/api-documentation/api/websocket/subscriptions/account-transfers) | Exact account/asset, deposit transaction ID; pending/confirmed withdrawal batch nonce, transfer amount (`am`), requested amount before fees (`ra`) and fee (`f`) | Deposit final-total semantics; persisted withdrawal UUID → batch correlation after lost ACK; durable replay, complete causal frontier or no later effects |
| [Account settings](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-settings) | Explicit `auto_lend_disabled`, configured margin modes/leverage. Null lending flag is the enabled default; absent margin overrides use native defaults. | Atomic settings/debt observation, completion of all pending setup, safe fresh-account bootstrap |
| [Account loan](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-loan-info) | Exact `borrowed` and `pending_interest` | A missing cache/account means zero debt; atomicity or causal completeness with settings |
| [Withdrawal request](https://docs.pacifica.fi/api-documentation/api/rest-api/account/request-withdrawal) | Optional UUID idempotency key; response batch nonce/gross/fee; duplicate UUID returns conflict | ACK is payment; a retry is safe; a lost ACK can be reconstructed from amount/time alone |
| [Official MCP tools](https://docs.pacifica.fi/api-documentation/api/mcp/tools), [pinned account client](https://github.com/pacifica-fi/pacifica-mcp/blob/4748feca93efe2b2a5a0c95993e40f68d0ca4338/src/tools/account.ts) | Perp deposit-history, withdrawal-history and pending-withdrawal routes; account/limit/cursor queries. Retained historical withdrawal rows expose batch/payment transaction. | Original UUID lookup, native economic terminality, retention/completeness guarantees or current-profile qualification |
| [Balance history](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-balance-history) | Paginated balance movements and timestamps | Original deposit signature or withdrawal UUID linkage, complete/no-later-effect certificate |
| [Funding history](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-funding-history) | Account payout, history ID, side, amount, rate and time | Complete hourly/gross-private-user allocation, especially when the external pooled position is flat |
| [WebSocket lifecycle](https://docs.pacifica.fi/api-documentation/api/websocket) | Separate testnet socket origin, ping/pong and finite connection lifetime | Authenticated replay or completeness after disconnect; the existing REST egress automatically supports native WebSocket |

The separate spot withdrawal history endpoint is **not** the current perp quote
route. Do not substitute it to close a perp withdrawal gate. Schemas were checked
against the primary documentation on the date above; example fields are not an
observation of our account, asset or environment.
Correction from the read-only investigation: these perp routes are documented
by the official MCP tool list/client, although detailed REST pages are absent
from the inspected index. Current diagnostic withdrawal requests still refuse before spending
read credits; their tags remain readable in archives. That is Cinder's current
unqualified-provider gate, not evidence that Pacifica lacks the endpoints. Do not
re-enable settlement merely because a GET route exists.

[Official MCP PR #5](https://github.com/pacifica-fi/pacifica-mcp/pull/5), merged
July 9, 2026 into the pinned source, intentionally removes executing withdrawal
tools while retaining read-only withdrawal queries. This explains the tool-list
and current client discrepancy; it does not remove the venue withdrawal API or
establish UUID lookup, native processing completeness or terminality.

<a id="completion-contract-audit-2026-10-08"></a>
## Completion-contract audit (2026-10-08)

`journal::funds::funds_complete` already checks **one original operation**: exact
debit/settled totals and receipt set, with independently qualified coverage for
each receipt's source. It does not require a universal account/trading frontier.
The earlier global-frontier explanation was too broad. The current trusted
`Credit.cut`, `Withdrawal.cut`, final-total and no-later-effect requirements
remain unchanged; a provider must justify them, not supply an arbitrary integer
that passes the journal's structural comparison. Recovery separately requires
all exposed lifecycles closed, no unresolved commitments, flat settled accounts,
returned backing, final native reconciliation and authority fencing.

Read-only reinspection of 35 retained September loan/withdrawal responses found
successful batch/transaction history and pending withdrawals, but no original
request UUID or `withdraw_id` in their rows. In the inspected positive withdrawal,
history/pending `amount` is **net**, not fee-inclusive native debit. Current loan
REST documentation and retained replies include the parser-required `updated_at`;
its omission from the MCP example is not a missing venue capability.

The closed October receipts additionally contain a finalized `DepositEvent`,
`BatchCompletedEvent` and `WithdrawEvent`, matching original gross/broker and
withdrawal instruction batch/net/withdrawal ID respectively. They strengthen
chain-effect evidence, not final off-chain credit/debit processing. The historical
bridge audit contains nonce checks/counter increments, but a withdrawal event
field differs from this observed deployment; audited source cannot be promoted
as current bytecode equivalence. Deposit nonce, withdrawal nonce and symbol LI
are not one interchangeable native sequence.

Detailed provenance and a reproducible read-only inspector are retained locally
at `work/venues/pacifica/experiments/completion-evidence-2026-10-08/`. No original
archive was changed, no financial provider/serialized contract was altered and
no fresh venue-account/RPC/AWS action occurred. The decision to keep the current
contract remains binding for strong completion; the later approved demo deposit
policy above is a separately labeled exception. Obtain supported final-credit/debit semantics and lost
UUID correlation before qualifying those ports; an identical economic rerun or
finite silence does not supply them.

## Promoted components

`pacifica::funding::evidence` decodes bounded documented bodies. It checks exact
account/asset, age, positive canonical 64-byte transaction IDs, exact quote atoms,
gross = net + fee, required batch/linkage fields, and rejects duplicate/unknown
transfer fields or explicit-null optional transfer fields. Only deposits and
withdrawals are handled; subaccount transfers are unsupported. Settings/loan
decoding exposes observations, including disabled-lending and compatible-margin
flags, rather than accepting unsafe settings or a missing debt response as ready.
Debug output redacts values. Unit/time/asset inputs require the independently
qualified profile; no decimal precision is inferred from an example string.
The gross/net conservation equality is a parser invariant and was observed in
the bounded native round trip; the field documentation does not itself state
that equation or establish final-total semantics.

`withdrawal_acknowledgment` strictly decodes the documented success response's
batch nonce, gross requested amount and advertised fee. The response contains
neither account nor request UUID. `Controller::withdrawal_acknowledgment` binds it
only through the original exposed plan and its retained HTTP-200 response in the
same verified journal. It rebuilds that mapping after restart, checks exact gross,
and refuses a batch shared by multiple retained local requests. Failed, missing,
malformed or backdated replies do not establish a binding. No response-record
format, financial posting or new persistence store is introduced.

`Controller::linked_deposit_transfer` requires a deposit plan and the exact
successful finalized original chain signature already retained by the chain
controller. It validates account/asset/time and rejects reported amounts above
the original gross. Partial and whole reported amounts remain observations:
neither creates venue cash nor establishes operation-specific final credit.

`Controller::linked_withdrawal_transfer` checks a subsequent source-authenticated
message against that mapping, the configured account/asset, gross, event age and
original dispatch time. Pending and confirmed events remain observations. Fees
that differ from the advertised fee remain visible; neither the ACK nor the join
declares the fee paid, fabricates chain finality, advances a native frontier or
releases a hold. Opaque retained reply/wire/failed-chain bodies cannot be replayed
as controller/gateway metadata even when they contain an internal magic prefix.

These types are **not** `Setup.complete`, `Credit`, `Withdrawal` or `Coverage`.
The module opens no socket, reads no wallet and changes no financial state. An
untrusted caller cannot make its bytes authoritative by invoking the parser.
There is intentionally no conversion that manufactures a source cut, finalized
payment or readiness from these fields. A free-standing decoded ACK is not an
original UUID binding; that mapping comes from the retained original dispatch.

Tests in `crates/pacifica/tests/funding_evidence.rs` cover exact observations,
scope/time/size/linkage failures, null/missing/duplicate fields, malformed/zero
signatures, overflow/inexact amounts, default lending, debt and incompatible
margin. Existing P16 tests continue to own idempotent original-operation matching,
credit/payment gates and uncertain/no-resend behavior. Neither set is live proof.
The controller regressions also cover deposit linkage before/after original
chain finality and restart without minting credit; durable ACK restart; wrong account/asset/
batch/gross, ambiguous batch reuse, fee overruns, hostile raw response prefixes
and lost-native-ACK refusal. These are offline fixtures, not authenticated native
capture or hardware qualification.

## Provider qualification required before wiring financial admission

1. Bind fresh account/profile/asset and exact precision to authenticated enclave
   observations. Keep TLS termination and private response processing inside the
   enclave. Native WSS needs its own measured fixed-origin transport; do not feed
   parent-decoded event bodies into trusted ports.
2. Persist each original operation/signature/UUID before exposure. Match a deposit
   event to that signature and verify its configured finalized Solana effect.
   Establish operation-specific final credit totals/fees; aggregate balance or
   matching amount/time is insufficient.
3. Qualify the retained successful-ACK path on actual rails and resolve the
   separate case where the native reply is lost before durable acceptance. The
   implemented mapping recovers a lost *client* reply after the native ACK was
   retained; it cannot reconstruct a lost *native* ACK from amount/time alone.
   Then establish terminal native debit/no-later-effect and independently
   finalized payment to the allowlisted broker. Batch identity alone is not UUID
   correlation; `withdrawal_confirmed` alone is not Solana finality.
4. Establish complete bounded execution histories, disconnect/pagination behavior
   and precise funding boundaries. Keep unresolved holds and claims until those
   contracts are justified. A final page or snapshot is not an inferred frontier.
5. Handle fresh-account setup explicitly. Historical M1 used an approved
   deposit-first exception; this version does not inherit it. If the current
   venue still requires first collateral before disabling lending, obtain a
   bounded qualification-only exception or choose another supported bootstrap.

Historical M1's single-intent fresh-account amount/time correlation is useful
scenario provenance, but weaker than the current operation-specific contract.
Do not promote that heuristic as a shipping certificate. If documented/tested
rails cannot support a required guarantee, identify the exact missing guarantee
and request a policy decision; never quietly label synthetic evidence as native.

## Current boundary

The offline component adds a budgeted bounded native-transfer capture:
`pacifica::capture::{prepare, Archive}` and
`service::native_capture::Port`, with a distinct fixed-host opaque WSS relay.
[ADR 0027](../architecture/0027-native-transfer-capture.md) records the actual
TLS/upgrade/limits, pinned protocol dependency, compiled-out plaintext logging,
independent local peer tests and exact remaining activation gates. Every retained
input remains raw (`event=None`, no causal cut or authority epoch); capture cannot
manufacture setup, credit or payment certificates. The loaded runtime/manifest
has a subsequent [version-4 one-shot worker](../architecture/0028-measured-native-capture.md)
that binds actual account/origin/route/root/limits and drops the writer guard for
network I/O. Local runtime activation is implemented; this is neither actual
native capture nor hardware evidence. The prior closed AWS receipt does not
qualify the new image. No setup/credit/payment certificate follows from capture.

Shipping manifest versions 1–5 forbid funding and trading. The separately approved
v6 composition permits only the private grant's two original ingress operations;
all versions forbid trading. Its local tests do not qualify financial activation.
Strong setup/credit/payment, complete execution/funding cuts and final native
recovery remain named open gates. The approved demo setup/credit exception is
described separately in [ADR 0029](../architecture/0029-demo-deposit-confirmation.md).
The retained-pack hardware invocation is closed, with all fixed read-only cells
passing. It grants no financial activity. Actual provider capture and financial
activation require a separate bounded manifest and qualified evidence.
