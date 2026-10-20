# Proposed second-venue qualification — not live authority

Updated: 2026-10-20. This proposal follows the [offline acceptance boundary](UNIT-ACCEPTANCE.md).
No financial/RPC/AWS action is authorized by this document or old test permissions.

<a id="a-native-paper-account-capability-before-hardware"></a>
## A. Native paper-account capability, before hardware

Current metadata prerequisite is complete: the separately approved two-GET run
at source `b8cac32` returned HTTP 200 for exchange info and fee state on October
20. The [receipt](../venues/bulk-native-qualification.md#observed-metadata-receipt--2026-10-20)
records BTC-USD's $1 native minimum and exact price/size grid. Earlier 502s are
historical. This approval covered no account or signed activity. The following
revised paper scope is **proposed, not approved**, and replaces the earlier
two-order/$10 suggestion, which could not cover resting, opening and closing.

Suggested bound, subject to separate explicit approval:

- One fresh disposable paper account; official faucet funds only; at most two
  fresh agent registrations and their revocations. No existing wallets, internal
  or external transfers, Solana RPC/L1 transactions, mainnet/customer assets,
  cloud resources or frontend integration.
- At most five order attempts **including cleanup**: a bounded resting/cancel
  probe, bounded IOC opening and confirmed-position closing, with remaining
  attempts reserved for known residual exposure. At most 20 paper USD cumulative
  order notional including all closing attempts, not collateral or leverage.
  Revalidate current minimums/grid and reserve close capacity before opening;
  stop rather than increase caps if the scenario cannot fit.
- At most 14 signed native requests, 24 private account reads and 12 unsigned
  public metadata/book reads, 30-minute run. These are ceilings, not targets.
  Fixed official testnet origin, no redirect, arbitrary endpoint or hidden retry.
- Explicit nonce/original-action persistence before signing. Any uncertain
  original is reconciled, never blindly resent. Small test collateral does not
  justify ambiguous behavior or false confirmation.
- Independently verify agent scope/revocation, original order IDs, observed
  fills/fee identity/precision and response schemas; record whether partial fills
  actually occur rather than force or invent that coverage. Record exact
  source versions and sanitized receipts; no keys/signed wires in public logs.
- Cancel/close known disposable paper exposure within the same cap where
  possible, revoke agents and record remaining uncertainty. Inability to clean
  up is reported, not declared success. No BULK funding bridge is tested here.

This would establish only native paper capabilities, not qualification of the
assembled Cinder gateway, custody, margin isolation,
fee aggregation, complete ADL/funding coverage or mainnet parity. Scope or native
minimum changes require discussion, not an automatic increase.

## B. Measured multi-unit assembly and changed-image Nitro

Only after A gives usable evidence: build the non-fixture multi-unit release
manifest from actually loaded role keys, both fixed egress origins/roots,
qualified clocks/storage and immutable registry/SDK/custody policies. Review it
before key release. Never deploy local-fixture code, fixed seeds, synthetic money,
same-host freshness witnesses or a manual native-readiness assertion.

Prepare a separate AWS request with exact image/source/measurements and narrowly
scoped IAM/KMS/storage resources. A starting proposal is one short-lived Nitro
host, $5/four-hour/eight-boot maximum with complete cleanup; it needs fresh user
approval. Do not inherit permission from previous P20/P23 resources or images.

First qualify attestation/key release, parent opacity, exact origin/context,
private owner/agent access, storage restore/fencing, concurrent reads/commits and
two-unit binding with **no native financial action**. Combining paper trades with
that run requires explicit added authority and exact native capability evidence.

## C. Funding/returns and venue-backed recovery — separate gate

Do not schedule this until a supported test funding/return rail and adequate
operation-linked evidence exist. BULK order/account APIs do not supply a bridge.
The stronger funds completion contract remains unchanged. Pacifica's deferred
missing-credit investigation is not reopened implicitly.

Then propose the exact deposit/sweep/credit/trade/settlement/return/payout and
fenced-recovery matrix, disposable accounts, actual quote/gas caps, cleanup and
failure reconciliation. If rails/evidence remain unavailable, preserve the
disabled integration boundary and report it. No mock financial completion can
substitute for this gate.
