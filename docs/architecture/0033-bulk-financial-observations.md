# ADR 0033 — BULK financial observations, without posting authority

Date: 2026-10-16. P28 implementation contract; no native activation.
[Baseline](../implementation/BASELINE.md), [BULK map](../venues/bulk.md),
[codec foundation](0032-bulk-codec-foundation.md),
[unit proposal](0034-execution-unit-boundary.md).

## Boundary and source

`cinder-bulk::financial` implements bounded pure REST normalization for
`fullAccount`, `fundingHistory`, `riskHistory` and `feeTier`. It reuses the exact
decimal codec and kernel AssetUnit/MarketUnit/QuoteAtoms/QuantityLots. No SDK,
network, secret, journal writer, kernel Event constructor or readiness grant is
added. Native source identity is not a private customer allocation route.

Schemas come from official client pin
[`56f66766…`](https://github.com/Bulk-trade/bulk-client/tree/56f6676611a0c05dbd187d406dced26168516070),
`crates/api-rust/src/msgs/{account,history}.rs`, and official
[account](https://docs.bulk.trade/api-reference/getAccount),
[funding](https://docs.bulk.trade/bulk-exchange/funding) and
[ADL](https://docs.bulk.trade/bulk-exchange/ADL) documentation rechecked October 16.
Tests are synthetic Cinder-authored shapes, not deployed schema qualification.
Ignored original captures/pins are not build/CI dependencies.

The three prior approved metadata GETs returned 502. P28 performs no new venue,
RPC, wallet, faucet, cloud or financial invocation. Pacifica credit stays deferred.

## Exact interpretation

Profile binds Scope, explicit quote unit/scale, master or pinned-parent subaccount
target, and a bounded unique symbol → kernel market-unit mapping. Its commitment
includes source, target, grids, units and scales in canonical symbol order.
Duplicate symbols/internal markets and quote/precision mismatches reject.
Construction establishes consistency, not source authority or qualification.

The native quote is not automatically a physical Solana token. USD, USDC and
paper assets need independent rail/asset evidence, not an assumed price parity.
Original numeric JSON lexemes never pass through floating point. Quote amounts
must map exactly to configured atoms; signed positions map to whole lots, including
explicit zero. Missing optional data stays unknown. Local and aggregate margin
stay separate, and cumulative metrics are not replayed as payments.

ReportedDecimal preserves display/rate text and exact lexical parts, with canonical
numeric equality. It supplies no atom scale, funding convention, oracle authority
or cost basis. Off-resting-tick average entries and forced prices are not rounded
into price ticks or multiplied into exact basis. All public financial Debug
representations redact contents; original bodies are private evidence, not logs.

## Financial mapping and gates

| Native fact | Meaning retained | Required before financial posting |
| --- | --- | --- |
| Local total/available/transferable margin | Distinct native reported metrics | Qualified cash/components, outstanding movements, source cut and risk/liquidity checks; none is a cash setter or customer withdrawal right |
| Optional `agg*` metrics | Parent-plus-isolated reports, not sibling subaccounts | Qualified lane/asset ownership; never add local and aggregate or count both as backing |
| Position signed size | Configured native lots | Reconciled source cut and attributed actual fills; no fabricated missing execution |
| Average entry, fair price, cumulative PnL | Reported averages/marks/totals | Exact remaining basis from actual fills; no average × quantity basis or unrealized-to-cash shortcut |
| Cumulative fees/funding | Totals with original native sign | Qualified sign, inclusion/reset and event boundaries; no blind delta or second charge/payment |
| P27 actual fill fee | Signed execution cost/rebate | Original execution owner, units, gross/net convention and source qualification; use EconomicChange::Execution once |
| Funding history payment | Actual signed native margin payment | Original boundary/inventory, rate/mark/rounding and linkage; existing FundingBoundary/Inputs/Settlement contracts, not another accrual |
| Fee-tier quote | Reported bps, volume and tier | Current eligibility plus actual execution evidence; quoted volume is not collateral or a guaranteed saving |
| Liquidation/ADL history | Native family, absolute quantity, original side flag, price/reason and optional margins | Qualified cause and linked actual fill/cash consequences; margin difference is not another loss or insurance claim |
| Risk-vault/unknown risk family | Original held row | Resolve actual units/meaning; its amount is not assumed to be position lots, Cinder capital or a customer payout |

BULK documents hourly funding applied directly to margin. Kernel FundingRate is
quote atoms **received per long lot**; BULK's positive rate denotes a cost to
longs. Future conversion must explicitly establish sign, mark, quote-per-lot
ratio, time boundary and rounding. P28 preserves the actual payment and reported
rate without inventing this conversion. Opposing private gross positions still
need a qualified rate/boundary when native net exposure and payment are zero.
Existing suspense and recognition-versus-settlement rules remain authoritative.

BULK distinguishes underfill versus shortfall ADL pricing. Human-readable reason
text is retained, not parsed into an approved cause. Risk and fill feeds can
describe one economic action: family-scoped observation IDs must not become two
postings. There is no assumed universal link between their slot/sequence values.
Current WS risk examples lack stable execution IDs; timestamp/size/price hashing
does not create them. P28 implements history facts, not a WS risk-posting path.

## Evidence, missing data and containment

History checks owner, required isolation flag, symbol, native slot/sequence,
timestamp, range, order and duplicate consistency. IDs stay u64. Keys bind source,
account, network and family, but deliberately exclude mapping revisions: changed
precision changes facts/context, not the original economic event's identity.

Known and held rows retain interleaved order. Unknown markets, isolated lanes and
unsupported risk families preserve original JSON without erasing known adverse
facts. Equal normalized duplicates collapse; conflicting rows reject. Held
duplicates require exact raw equality, not guessed economic equivalence.

Cursor presence, continuation, retention and range checks are structural only.
An empty, terminal or reported-complete page does not release holds, establish
original deposit credit or certify a complete economic frontier. The future
controller must retain authenticated original request/body before parsing and
contain rejected/held input before any dependent action. No parser output
authenticates its own upstream connection or completes a multi-page scan.

Current account bodies do not echo the queried address. Their profile/body hash
binds the caller's trusted request, not independent native authenticity. Kind and
parent must match that request profile. Orders/protection/children/locks remain
explicit inventories; unmodeled orders are not absent. Missing children/locks
remain distinguishable from empty lists. Unknown optional margin fields are not
zero-filled, including partial aggregate reports.

Bounds cover original response, rows, configured markets, object fields/keys,
numeric text, cursor and reason text. Errors contain no original record data.
Shipping runtime, journal, financial policy, custody/recovery and RF1/RF2 are
unchanged. Native qualification and the reviewed unit contract precede posting.
