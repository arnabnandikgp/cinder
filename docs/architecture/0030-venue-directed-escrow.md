# ADR 0030 — Venue-directed escrow ingress and shared payout protection

Date: 2026-10-15. Status: user-approved product direction and implementation
contract; not implemented, deployed, migrated or live-qualified by this ADR.
Owns [P23A–P23C](../implementation/PLAN.md#p23a). Extends B04/B05 without changing
the individual-entitlement, pooled-execution, RF1/RF2 or operator-assisted recovery
contracts in [BASELINE](../implementation/BASELINE.md).

## Decision and scope

Use a program-controlled token escrow for each customer, venue route and supported
collateral asset. A deposit means **fund this customer's Cinder account on the
selected venue**, authorizing the full identified deposit batch to be swept through
that route. It is collateral, not the notional of a perp order.

The user can reclaim an unswept batch. Once its on-chain sweep commits, refunding
that same escrow batch is forbidden; any later withdrawal follows Cinder's ordinary
financial and payout checks. Escrows are staging accounts, not persistent native
venue accounts or a blanket self-custody guarantee for deployed collateral.

Retain one cohesive Anchor custody/recovery executable. Its venue-specific
program-controlled payout vault can reuse the existing custody config, token
authority and normal/recovery payment counters. Do not introduce a second program,
an additional common deposit vault or separate native accounts per customer.

Pacifica remains the only initial integration. A venue route is an immutable
binding to its deployment, native account and funding wallet, not just a UI label.
Adding escrows does not authorize BULK, multi-venue trading, cross-venue margin,
new coverage terms, live transactions or a production deployment.

## Physical route and authorities

```text
Ingress:
customer wallet → customer × venue × asset escrow
                → allowlisted Cinder venue funding wallet
                → venue deposit program/vault → pooled native account credit

Ordinary return and payout:
pooled native account → Cinder venue funding wallet
                     → venue-specific Cinder PDA payout vault → customer wallet
```

The funding wallet and PDA payout vault are different physical locations. The
former has a venue-recognized signing key; the latter's token authority is a Solana
PDA. The TEE approves normal funding/payout instructions using a designated funds
authority; the program executes only its constrained transfer. It does not know
private positions or independently calculate the withdrawable amount.

Keep governance, funds, recovery, native trading, transport and storage authorities
separate. The native wallet remains an explicit custody risk: a Solana program
cannot constrain transfers that bypass it from a key-controlled funding wallet.
Nor does an escrow PDA gain the ability to sign the venue's ordinary HTTP requests.

The public Pacifica withdrawal request has `account`, amount, signature and optional
agent/idempotency fields but no arbitrary payout-recipient field. The intended
return therefore uses the supported broker-owner route, then Cinder's payout
program; do not invent direct venue-to-customer support. This is a documented API
boundary, not fresh testnet/mainnet withdrawal qualification.

## On-chain batch contract

An escrow identity binds custody domain/config, registered customer owner,
immutable venue route, mint and canonical PDA bump. A separate permanent batch
record binds an owner-selected nonzero operation ID, exact positive quote atoms,
funding consent, authority epoch and sweep deadline. Concrete seeds/layouts/ABI
belong to P23B; they must preserve these identities and existing payout layouts.

Each supported deposit uses an owner-signed instruction that atomically transfers
the exact asset amount and creates a `Pending` batch receipt. A plain SPL transfer
or an unexplained token balance is not an eligible deposit receipt. Use the
currently qualified classic SPL asset boundary; Token-2022 extensions are not
implicitly enabled. Do not collect protocol/private trading state on chain.

| Instruction | Preconditions | Atomic result |
| --- | --- | --- |
| Deposit batch | Registered owner signs; normal route; exact mint/accounts; positive amount; unused operation ID; bounded consent/deadline | Tokens arrive in the correct escrow; immutable batch becomes `Pending` |
| Cancel/refund batch | Owner signs; that batch is still `Pending`; exact owner recipient and sufficient escrow backing | Full batch amount returns to the owner; batch becomes `Refunded` |
| Sweep batch | Current funds authority signs; normal mode; original consent and unexpired deadline; exact batch amount; immutable broker destination; cumulative release cap and sequence checks | Full batch amount transfers to the allowlisted funding wallet; batch becomes `Swept`; permanent movement evidence records identity and amount |

Sweep and refund mutate the same batch state in their token-transfer transaction.
One can win; both cannot. Failed token CPI rolls back the transition. A `Swept` or
`Refunded` batch is never made pending again, closed/recreated to reset consumption,
or retargeted to another venue. An expired unswept batch remains owner-refundable;
expiry does not create keeper funding authority. Freeze stops new deposits/sweeps,
but does not confer ownership of never-swept batches on the recovery operator.
Owner refunds of those segregated batches must remain possible in frozen/recovery
mode, subject to actual token transfer availability and the same batch checks.

The full-sweep rule means **full recorded batch amount**, not drain the account's
changing balance. Multiple batches may share the user/route escrow, but sweeping
one cannot spend another. Unsolicited excess is neither credited nor swept; record
it separately for a later provenance/refund decision. No arbitrary admin drain is
introduced. Transaction/network fees and account rent need explicit payer handling;
they must not silently reduce recorded collateral or redefine the full amount.

## One ledger, explicit pending ownership

The financial kernel and protected journal remain authoritative. Escrow monitoring
and UI projections must not become a second balance database.

Before sweep, the batch is customer-owned pending ingress, not spendable Cinder
trading cash, venue margin, free house capital or recovery payout capacity. If a
consolidated inventory includes its tokens, it must also include the matching
pending-owner obligation; do not count the assets while omitting their owner.
Refunding an unswept batch is the return of pending ingress, not payment of a
previously booked trading entitlement or consumption of its normal paid counter.

The handoff to pooled custody is the qualified finalized sweep receipt. P23C must
atomically consume that receipt, recognize the attributable deposit once (including
existing approved debt-recovery treatment), book broker-location assets and retain
the full funding restriction. Replay, delayed observation and response loss cannot
credit it again. This recognition point replaces the old common-vault deposit
recognition for new escrows; legacy histories keep their original meaning.

Thereafter, broker debit creates transit, and qualified venue credit moves transit
to native cash and completes eligible funding restrictions. Those are internal
movements of the same entitlement, **not second customer deposits**. Partial or
unconfirmed venue credit does not enable new trading. Never silently promote ADR
0029's first-idle-account demo assumption into general production credit finality.

For one fully recognized batch of `d` quote atoms with no fee or debt:

```text
Before sweep: escrow_pending_owner = d; spendable_trading_cash = 0
After finalized sweep recognition: customer_cash += d; broker_assets += d
                                  funding_restriction = d
After native debit: broker_assets -= d; transfer_receivable += d
After qualified full credit: transfer_receivable -= d; venue_assets += d
                            customer_cash unchanged
```

Tradability requires the remaining ordinary margin, capital, native-readiness and
location-liquidity checks, not merely the last equation. Fees, debt recoveries and
actual partial/adverse effects follow their existing attributed transitions. A
new physical location is not permission to treat separate venues as fungible
collateral or change risk-cohort ownership.

P23C reserves identities/capacity and persists sweep intent before signature
exposure. A possibly exposed sweep that races with cancellation remains uncertain
until qualified original chain evidence determines its outcome. Never treat a
missing transaction or UI timeout as proof that refund/sweep failed or may be
resigned. Late facts remain recordable during freezes.

## Ordinary payouts and recovery

Withdrawals end at the registered user's wallet token account, not the ingress
escrow. Venue returns first establish actual funding-wallet tokens, then qualified
return establishes PDA payout liquidity. Neither event discharges the customer
claim; only the exact final program payment does so.

Use the existing authenticated destination, epoch/mode, operation identity,
sequence and lifetime paid-counter controls. Normal and recovery payments share
the same claim-consumption scope; separate route/config identities cannot make the
same private entitlement payable twice. Profits may exceed the original deposit;
neither the escrow amount nor its deposit counter caps the final entitlement.

No payout path bypasses these counters. Returned withdrawal funds are never
placed into an auto-swept ingress escrow. Payout-vault tokens are reserved for their
qualified obligations, not silently reused as sweep capital.

Recovery remains operator-assisted: fence authorities/exposed operations, restore
fresh accepted history, reconcile native exposure, return reachable assets, finalize
liabilities, fund immutable claims, activate, distribute private kits and allow
independent claims. Pending unswept batches are separately refundable and excluded
from trading/recovery claims. A swept-but-uncredited batch is still an obligation
with uncertain/inaccessible backing; a timeout cannot erase it or invent recovery
liquidity. New escrows do not solve native-fencing, final-cut or missing-credit gaps.

## Privacy, compatibility and migration

User/venue escrow addresses and public movements expose funding attribution and
timing. Balances, orders and positions within Cinder remain private to the
confidential runtime, with this additional public metadata acknowledged. Random
seeds are not assumed to hide owners or transaction links.

Existing P15/P16/P17/P22 tests and P23 receipts remain valid for their recorded
old-route scope. Do not rewrite their claims into escrow evidence. Preserve the
unresolved test-fund inventory and earlier configuration identities. No deployment,
account migration, legacy liability transfer or payout-counter reset is authorized
here. P23B extends the program without changing existing account layouts; P23C must
version any new journal/wire semantics and fail closed on incompatible histories.
Legacy funding constructors remain explicit, not a silent fallback for new escrows.

## Acceptance and implementation order

| Boundary | Required cases | Owning slice |
| --- | --- | --- |
| Contract/context | Approval versus implementation marked; current baseline/flows/guide agree; history and residuals preserved | P23A |
| Program/client | Deposit/full sweep/refund; both race orderings; duplicate/reused IDs; two users/batches/routes; wrong owner/mint/broker; CPI failure; freeze, expiry, authority rotation, cap/counter exhaustion; unsolicited surplus | P23B |
| Journal/controller | No trading before credit; exactly one recognition; cancel before/after exposed sweep; duplicate/conflict receipts; crash/unknown original; partial native credit; fees/debt; pool/location constraints | P23C |
| Joined payout/recovery | Qualified return and one wallet payment; replay across ordinary/recovery; prior paid counters; unswept refunds excluded; swept pending liabilities retained; old-route regressions | P23C |
| Actual rails | New exact-source SBF/SDK/Nitro/chain/venue evidence and reconciled costs/residuals under a fresh approved manifest | P23, after P23C |

P23A is the docs-only foundation. P23B is one reviewable Anchor/client slice. P23C
joins the existing journal, funding coordinator and offline workflow suite.
Self-service onboarding and Standard frontend integration follow that contract;
they are not introduced by a keeper script or this ADR. Production G01–G04 and
fresh live-run G05 approvals remain separate.

## Primary references

Checked 2026-10-15 for documented shape, not live financial qualification:

- [Solana PDA authority](https://solana.com/docs/core/pda): a PDA has no private key;
  the deriving program signs via CPI. It does not authenticate an ordinary HTTP request.
- [Pacifica withdrawal request](https://docs.pacifica.fi/api-documentation/api/rest-api/account/request-withdrawal): no arbitrary recipient field is documented.
- Existing source and evidence: [custody](0015-solana-vault.md),
  [funding coordinator](0016-funding-coordinator.md),
  [recovery](0017-recovery-claims.md),
  [demo credit limits](0029-demo-deposit-confirmation.md) and
  [deferred live financial gates](../implementation/P23-FINANCIAL-GATES.md).
