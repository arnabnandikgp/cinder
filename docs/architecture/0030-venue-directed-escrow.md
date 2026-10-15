# ADR 0030 — Venue-directed escrow ingress and shared payout protection

Date: 2026-10-15. Status: user-approved product direction; P23B implements the
local program/client boundary on its dependent branch. P23C adds the offline
journal/controller/Runtime/SDK join. Deployment, migration and live qualification
are not completed by this ADR; exact checks and review state are in TRACKER.
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

### P23B concrete local ABI

The immutable `VaultConfig` remains the pool/asset custody domain. One new
`VenueRoute` PDA (`["venue_route", config]`) binds that config to a nonzero
32-byte venue identity and the config's immutable broker owner/native account.
Governance registers it once in normal mode. A config cannot be relabeled for
another venue or share a second route; another risk cohort uses another config.
The program enforces the funding-wallet destination, not native venue credit.

`VenueEscrow` uses `["venue_escrow", route, owner]`; its classic SPL token account
uses `["escrow_tokens", escrow]` with the escrow PDA as authority.
`open_venue_escrow` requires the existing customer record and owner's signature;
`register_venue_route` requires governance authorization. `EscrowBatch` uses
`["escrow_batch", escrow, deposit_operation]`. All bumps are canonical.

New instructions are `register_venue_route(domain, epoch, venue)`,
`open_venue_escrow(domain)`, `deposit_escrow(auth, amount)`,
`sweep_escrow(auth, expected_sequence)` and `refund_escrow(domain)`. The sweep's
`auth.operation` is a separately persisted operator movement identity; it cannot
be squatted by the owner's deposit operation. There is no sweep amount argument:
the program uses the full stored batch amount. The permanent operator receipt
uses the existing `["receipt", config, sweep_operation]` namespace and kind `4`,
binding owner, immutable broker recipient, amount and funding sequence. The batch
stores that operation/sequence and terminal slot; refund records the owner's
actual recipient and terminal slot, with zero movement operation/sequence.

New schema-1 layouts (serialized size excludes the 8-byte discriminator):

| Account | Ordered stored fields | Size |
| --- | --- | --- |
| VenueRoute | schema/bump u8; config Pubkey; venue [u8;32]; native_account Pubkey | 98 |
| VenueEscrow | schema/bump/tokens_bump u8; route/owner Pubkey; pending u64 | 75 |
| EscrowBatch | schema/bump/state u8; escrow Pubkey; operation [u8;32]; amount/epoch/expires_at_slot u64; terminal_operation [u8;32]; destination Pubkey; sequence/terminal_slot u64 | 171 |

Batch state values are `0 Pending`, `1 Swept`, `2 Refunded`. Amount, consent epoch,
deadline and original operation never change. No close/reinitialize/update/drain
instruction exists. Each token movement requires backing for **all** recorded
pending batches, not just the selected one. A token impairment blocks consumption
without deleting claims. Surplus has no batch and cannot be swept or refunded.

Deposit/open/route registration require normal mode. Sweep additionally checks
both current movement authorization and original deposit consent epoch/deadline,
the shared gross release cap and funding sequence. Refund checks the fixed domain
but deliberately not the current epoch, deadline or mode. Only sweep increases
legacy `config.deposited`/`customer.deposited` attribution and shared release
counters; pending deposit/refund change neither attribution nor paid counters.
These are public custody totals, not spendable ledger credits. P23C's journal
owns qualified finalized recognition and native-credit restrictions below.

The three new layouts and appended error codes leave all existing layouts,
instruction constructors and receipt kinds unchanged. This is a local ABI only:
no live deployment or migration is authorized by P23B.

Rent payers are explicit: governance creates the route; the owner opens the
escrow/token account and each batch; funds authority creates the sweep receipt.
Refund creates no account and can use an independently selected transaction fee
payer. None of these lamports is deducted from the recorded SPL batch amount.

The signed-SBF tests in `clients/vault/tests/vault.test.ts` use the existing fresh
local harness and typed builders. They cover both terminal orderings and concurrent
competition; repeated IDs/open attempts; two owners and the same wallet/ID on a
second route/mint; source/recipient/signature/token-program substitution;
original versus operator expiry and rotation; shared legacy/escrow gross caps;
real frozen-token CPI rollback/retry; synthetic backing impairment and arithmetic
exhaustion; surplus isolation; active recovery refunds; and unchanged legacy layouts.
Fixture-only state edits isolate adverse expiry/backing/counter states; they are
not reachable owner mutation APIs or evidence about a native venue.

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

### P23C journal and runtime contract

`Controller::new_escrow` is an explicit new-history composition, bound to the
canonical route and actual configured venue. It cannot reinterpret a legacy or
demo controller's accepted history. Journal engine revision **24** appends
`Control::Escrow` (tag 13) and `AttemptKind::Escrow` (tag 6); transaction envelope
version 8 remains unchanged. Older engine histories reject without migration.
The additional chain contract is `cinder-vault-funding-v2`, with exact original
batch/PDA/deadline metadata and a 96-byte, amount-free sweep instruction. Legacy
rails keep their v1 contract and cannot borrow sweep fields or authority.

`journal::escrow::Entry` is replay-derived lifecycle metadata, not another balance
book. Pending inventory consumes its operation identity but adds no ledger cash,
active financial hold or recovery claim. Qualified handoff atomically applies the
existing kernel receipt and its restrictions within one control candidate. Its
economic key binds the immutable route and batch, independent of lookup/commit
IDs. The actual finalized Solana handoff slot is retained; it is not a native
venue global causal frontier.

For a gross deposit `d`, let `c_before` and `c_after` be that customer's cash around
the kernel receipt. The retained restrictions are:

```text
customer restriction = max(c_after, 0) - max(c_before, 0)
house restriction    = d - customer restriction
broker restriction   = d
```

The house residual includes both repayment of unabsorbed customer debt and
replenishment after prior absorption. These are restrictions, not extra postings;
customer plus house restriction equals `d`. The original kernel receipt remains
the sole owner of debt/recovery accounting. Accepting the exact full native
funding intent replaces the broker restriction with the existing funds hold;
customer/house restrictions persist until strong, full, zero-fee native completion.
Partial credit, a setup-ready flag or weaker demo assurance cannot release them.
Unsolved native credit keeps funding pending and blocks new native risk.

Original intent precedes counter reads/signing. Fresh chain/code/token/batch
checks run outside the Runtime journal writer; their result rejoins and rechecks
the original authority/state before signature delivery. The original signed wire
is persisted before send. A lost reply is reconciled with that wire, never signed
again. Expiry can close an exposed-but-unsent plan only when the trusted controller
checks the independently verified current head and proves no signed wire was
retained; expiry with a retained wire does not establish zero execution. Exposed
reconciliation has priority over queued new work and continues during freeze.
Finalized original owner refunds are recordable before or after inventory discovery.

Fenced recovery can recall a swept batch still at the broker, using the existing
Broker → Vault return rail. If native funding was accepted, it must first be
terminal with no effects; an exposed attempt additionally needs strong no-later-
execution closure. Unknown or partially credited native funding cannot use this
shortcut. A full qualified return releases restrictions and marks `returned`,
without adding another deposit or incrementing customer-paid counters. The same
funding sequence and ordinary/recovery payout counters remain authoritative.
Recovery custody observations cannot predate a retained handoff's finalized slot.

The confidential Runtime configuration selects this mode explicitly with `escrow`.
Existing demo/financial live manifests reject activation of the new composition:
the offline controller/Runtime tests are not a new live release authorization.
Joined fixtures exercise actual SDK/carriers, protected journal and compiled SBF;
the venue/native observations remain explicit test doubles. Static configured
owner deposit locators are reused; automatic onboarding/discovery and a backend-
connected terminal are not introduced here. See [private ingress reads](private-read-contract.md)
and the [P23C handoff](../implementation/TRACKER.md).

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
