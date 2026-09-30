# ADR 0016 — Durable funding over one financial journal

Date: 2026-09-30. Scope: [P16](../implementation/PLAN.md#p16).
Depends on P08 funds, P14 shared API credits and P15 custody. This is an
offline-tested coordinator and chain codec, not a live deployment, RPC trust
certificate or independently qualified venue-completeness guarantee.

## Physical locations, not another balance ledger

The supported route is:

```text
Vault --release--> Broker --native deposit--> Transit --qualified credit--> Venue
Vault <--return--- Broker <--finalized native withdrawal------------------ Venue
Vault --authorized normal payout--> registered customer's token account
```

`Broker` is a distinct physical location in the existing kernel, not a customer
book or another venue account. These tokens are neither vault payout capacity nor
native margin. The existing bridge and marked diagnostics include broker cash
exactly once. Prepared holds and scenario liquidity have a separate broker
dimension. Policies for configured broker routes must include all three locations;
a policy that omits the third location is invalid. Inaccessibility affects deadline
liquidity independently of aggregate economic backing.

The journal engine revision is 20. Older semantic histories fail closed; there is no
implicit migration that reinterprets historical transfers. Funding records are
encrypted evidence/projections in the common journal, never independent assets.

## Authorization and exact dispatch

`cinder_pacifica::funding::Controller` accepts explicitly qualified immutable
routes, mint precision, broker owner, deployment/program identities, caps, source
namespaces and private-account/public-beneficiary bindings. It has no ambient
wallet, clock, RPC client or API retry. A private customer ID is not assumed to be
a Solana wallet public key. Authenticated beneficiary changes require a reviewed
migration, not a different controller silently reopening the journal.

The controller derives an immutable plan from a **prepared P08 funds mandate**.
It cannot invent a new payout or reserve outside that lifecycle. Its operation
hash binds the full attempt and route contract; P15 counters, authority epoch and
slot deadline bind the actual instruction. Logical event milliseconds, native
causal cuts and Solana slots remain different units.

For chain rails, exposure and plan persist atomically before the non-clone unsigned
capability is returned. The unsigned JSON contract carries explicit public route
keys and decimal-string u64s, not private request IDs or abstract journal payloads.
`clients/vault/src/funding.ts` builds only release, return, normal payout or the
minimal native deposit ABI. It has no transport or implicit signing. It rejects
inexact numbers, noncanonical/duplicate-key JSON, wrong PDAs, unsupported rails,
native ATA substitution and zero identities.

The confidential chain port signs the selected instruction, then verifies the
exact serialized legacy transaction: one expected signer/fee payer, exact program,
accounts, flags and data, and no extra transfer/instruction. One bounded
compute-unit-limit instruction is optional; a fee-price instruction is not.
Versioned transactions and arbitrary CPI are not accepted. `VerifiedWire` is the
output of this trusted codec port, **not** authentication merely because a Rust
caller constructs the struct.

`persist_wire` consumes the original non-clone capability and stores that exact
private signed wire, signature and contract hash
before returning the one-use chain delivery. A stale writer, uncertain commit,
duplicate persistence, revoked grant/frozen financial cut, stale setup or
mismatched contract returns no delivery. Read-only contract inspection cannot
recreate this private capability. After restart,
the retained plan/wire is reconciliation-only: never select a new operation,
blockhash or signature for that possibly exposed attempt. Signed bytes stay inside
the protected runtime/storage; they are not host logs or public fixture artifacts.

Native withdrawal uses a separately injected **broker owner** signing key, not
the Gateway's trading-agent key. It shares P14's durable API-credit history,
cleanup reserve, source cooldown and active-writer/key fencing. The deterministic
UUID binds the full original attempt. The immutable preimage includes gross atoms,
timestamp and expiry; expiry is capped by the same credit-window policy. Plan,
credit spend and exposure commit together before signing and one POST. HTTP
success, error, timeout, 409 duplicate or batch nonce neither credits cash nor
discharges a commitment. There is no resend path with a refreshed UUID/time.

## Settlement and bootstrap

All financial changes use the existing consumed-event keys, receipts, source cuts
and atomic postings. Qualified chain observations must verify cluster identity,
finalized execution/error, the original exact signed wire, token owner/mint and
deltas, program/config/PDA and receipt contents. A successful P15 receipt must
match operation, epoch, amount and relevant sequence/paid counters. Finalized
atomic failure settles zero effect and consumes no customer paid amount. A missing
transaction is **not** proof of zero effect.

A successful native deposit first debits broker tokens into transit. Only
independently qualified native credits tied to the original deposit signature
create venue cash. Partial credits post real effects but retain the remaining
transit and disable new native risk. A final credit requires exact total coverage,
qualified setup and completion of all other pending deposits. Chain and venue
completion use separate source cuts, not a fabricated common sequence. Evidence
from failed controls/rejected postings remains retained but cannot later authorize
credit-readiness or signing.

Withdrawal settlement requires complete original-UUID native history **and** the
actual finalized owner-directed broker-token payment. The qualified port must
establish the original signed request's full/no-later-effect semantics; an empty
page, balance increase, caller-selected hash or unrelated payment is insufficient.
Pacifica's public histories do not automatically supply this qualification.
Production recognizers/RPC transport remain G01/P23 and attested integration
remains P19/P20. Offline synthetic observations deliberately exercise the port,
not pretend to authenticate these sources.

Native setup must be fresh and complete, with lending disabled and exact zero
borrowed balance/pending interest. A missing account, undocumented default,
regressing observation or unresolved setup does not qualify. Binding starts with
native risk fenced. Dirty setup is recorded and keeps funding/new risk disabled.
The historical fresh-account experiment's 120-second deposit-first exception is
**not standing authorization**. This phase refuses that exception; production
bootstrap must use a separately approved bounded manifest/qualified account.
No local function silently enables lending or broadens an agent's native rights.

## Working collateral, costs and residuals

The risk controller supplies a qualified target of working collateral, **not perp
notional**. The coordinator derives:

```text
projected = qualified native free collateral
          + unreserved allocated broker tokens
          + remaining prepared release/deposit ingress
additional = max(0, target - projected)
```

Prepared ingress is counted even before physical dispatch, so reservations cannot
cause a duplicate allocation in that gap. This forecast is not available venue
cash, liquidity, margin or permission to trade. P09 still owns joined risk and
P08 still owns entitlement/capacity reservation. The journal's credit-readiness
fence blocks order admission/exposure while native credit is incomplete; it does
not erase real adverse observations or prevent funds reconciliation. Qualified
emergency liquidation/house cleanup and attributed cancellation remain available
without credit-readiness, but retain their existing freeze, authority, evidence,
close bounds and funded-resource checks. Ordinary orders, generic exposure and
restoration replacements remain fenced; restoration is not an emergency exemption.

Native internal-transfer fees follow P08's **house-owned cost** policy. A fee
overrun records the actual cost and retained fault, not the maximum expected fee.
Recognition and payment are not counted twice. A return is no new customer
deposit; a final payout reduces the actual private entitlement and paid counter
once. In the round-trip fixture, 20 venue atoms return as 19 broker atoms with
one house-paid fee: paying the customer 19 leaves their remaining claim of one.
It does not silently charge that fee to the customer or delete the claim.

Residual reporting derives vault, broker, native cash, transit and unresolved
exposed attempts from the one common state. Cleanup cannot invent balances or
hide residuals behind a retry. SOL transaction/rent expenses are separate operating
resources, not unreported quote-token fees; the gross quote caps do not approve
production SOL budgets. Freeze/recovery orchestration belongs to P17/P21, not
an automatic ordinary payout or heartbeat-only recovery path here.

## Provenance and verification

Promoted W04 requirements and independently implemented minimal ABI, not research
run accounts, wallet material, live signed payloads or historical permissions:

| Original local source | SHA-256 |
| --- | --- |
| W04 `fixture/controller/run.mjs` | `93377bf2428c7adef0df263b0a6dcc7439ba0754a606c2ce889e349849a402da` |
| W04 `fixture/controller/evidence.mjs` | `67965cc9598010b63282640cb4a4631a321e378a55630f5c107bf4a425958051` |
| W04 `fixture/adapter/bindings.mjs` | `c864a8843ea8c13efcf7e4b9a8f4da810d69419eb8733b89b852854d02759da5` |
| Pacifica public `pacifica_solana.json` IDL | `a31bb37868338e189fead472c1481954ed876844e83529e2e68b5ef0dbf427a1` |

The deposit subset is discriminator `[242,35,198,137,82,225,242,182]` plus LE u64,
then depositor, its classic ATA, `central_state` PDA, that PDA's mint ATA, classic
Token, Associated Token, mint, System, `__event_authority` PDA and native program.
No environment's historic program/mint is a default. The full upstream IDL is
not copied or required in CI; tests independently pin this sanitized subset.
Changed ABI/deployment identities require fresh qualification, not inference.

Primary documentation rechecked 2026-09-30:
[Pacifica withdrawal](https://docs.pacifica.fi/api-documentation/api/rest-api/account/request-withdrawal),
[lending settings](https://docs.pacifica.fi/api-documentation/api/rest-api/account/toggle-auto-lending),
[Solana signature status](https://solana.com/docs/rpc/http/getsignaturestatuses).
Withdrawal ACK data/idempotency are documented; source-complete no-later-effect
classification remains a qualified dependency, not established by those pages.

Tests cover the common-ledger round trip, partial credits, fees/overrun, wrong
network/route/mint/receipt/counters, exact wire persistence/replacement, marginal
allocation, broker liquidity, lending/setup refusal, API budgets/backoff and
unknown HTTP outcomes. An actual child is killed after POST exposure and before
reply persistence; replay refuses a second send and reconciles the original
withdrawal once. Its explicitly ignored child entry is invoked by that parent test
in both debug/release; it is not a skipped acceptance scenario.

Chain codec tests verify real disposable Ed25519 transaction signatures and extra
instruction rejection, including u64s above JavaScript safe integer range. The
P15 program executes codec-selected custody rails in offline Surfpool SBF.
Native deposit ABI tests are codec tests, **not** fresh native-program execution
or live venue finality evidence. Financial synthetic ports use fixture protection
marked **not encryption**; P06's separately tested AEAD/freshness remains the
production storage interface. Results are tests, not a proof/audit or live release.
