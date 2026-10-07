# Pacifica native evidence contract

Updated 2026-10-07. Documented schemas and offline tests, **not fresh venue
qualification or financial activation**. This supplements
[the financial gates](P23-FINANCIAL-GATES.md); the approved ledger, custody and
recovery contracts are unchanged.

## Primary sources

| Source | Useful evidence | Not established by this source |
| --- | --- | --- |
| [Account transfers](https://docs.pacifica.fi/api-documentation/api/websocket/subscriptions/account-transfers) | Exact account/asset, deposit transaction ID; pending/confirmed withdrawal batch nonce, requested gross, net and fee | Deposit final-total semantics; persisted withdrawal UUID → batch correlation after lost ACK; durable replay, complete causal frontier or no later effects |
| [Account settings](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-settings) | Explicit `auto_lend_disabled`, configured margin modes/leverage. Null lending flag is the enabled default; absent margin overrides use native defaults. | Atomic settings/debt observation, completion of all pending setup, safe fresh-account bootstrap |
| [Account loan](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-loan-info) | Exact `borrowed` and `pending_interest` | A missing cache/account means zero debt; atomicity or causal completeness with settings |
| [Withdrawal request](https://docs.pacifica.fi/api-documentation/api/rest-api/account/request-withdrawal) | Optional UUID idempotency key; response batch nonce/gross/fee; duplicate UUID returns conflict | ACK is payment; a retry is safe; a lost ACK can be reconstructed from amount/time alone |
| [Balance history](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-balance-history) | Paginated balance movements and timestamps | Original deposit signature or withdrawal UUID linkage, complete/no-later-effect certificate |
| [Funding history](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-funding-history) | Account payout, history ID, side, amount, rate and time | Complete hourly/gross-private-user allocation, especially when the external pooled position is flat |
| [WebSocket lifecycle](https://docs.pacifica.fi/api-documentation/api/websocket) | Separate testnet socket origin, ping/pong and finite connection lifetime | Authenticated replay or completeness after disconnect; the existing REST egress automatically supports native WebSocket |

The separate spot withdrawal history endpoint is **not** the current perp quote
route. Do not substitute it to close a perp withdrawal gate. Schemas were checked
against the primary documentation on the date above; example fields are not an
observation of our account, asset or environment.

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

These types are **not** `Setup.complete`, `Credit`, `Withdrawal` or `Coverage`.
The module opens no socket, reads no wallet and changes no financial state. An
untrusted caller cannot make its bytes authoritative by invoking the parser.
There is intentionally no conversion that manufactures a source cut, finalized
payment, UUID binding or readiness from these fields.

Tests in `crates/pacifica/tests/funding_evidence.rs` cover exact observations,
scope/time/size/linkage failures, null/missing/duplicate fields, malformed/zero
signatures, overflow/inexact amounts, default lending, debt and incompatible
margin. Existing P16 tests continue to own idempotent original-operation matching,
credit/payment gates and uncertain/no-resend behavior. Neither set is live proof.

## Provider qualification required before wiring financial admission

1. Bind fresh account/profile/asset and exact precision to authenticated enclave
   observations. Keep TLS termination and private response processing inside the
   enclave. Native WSS needs its own measured fixed-origin transport; do not feed
   parent-decoded event bodies into trusted ports.
2. Persist each original operation/signature/UUID before exposure. Match a deposit
   event to that signature and verify its configured finalized Solana effect.
   Establish operation-specific final credit totals/fees; aggregate balance or
   matching amount/time is insufficient.
3. Bind a withdrawal's persisted UUID to its batch without requiring a successful
   ACK, then establish terminal native debit/no-later-effect and independently
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

The shipping manifest still forbids funding and trading. Setup/credit/payment,
complete execution/funding cuts and final native recovery are named open gates.
The next storage hardware run is independent: it can qualify the changed image
and history growth without enabling money movement. Actual provider capture and
financial activation require a separate bounded manifest and qualified evidence.
