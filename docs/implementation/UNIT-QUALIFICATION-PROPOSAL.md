# Proposed second-venue qualification — not live authority

Date: 2026-10-17. This proposal follows the [offline acceptance boundary](UNIT-ACCEPTANCE.md).
No financial/RPC/AWS action is authorized by this document or old test permissions.

## A. Native paper-account capability, before hardware

First obtain useful current official metadata/schema evidence. The last two
approved public requests returned origin 502. A separately approved run could
allow at most 12 unsigned metadata reads and then one disposable native paper
account with two fresh agents, official faucet funds only. No existing wallets,
Solana RPC, L1 transfers, mainnet, customer assets or cloud resources.

Suggested bound, subject to explicit approval and confirmed native minimums:

- One paper account; at most two agent registrations and their revocations.
- At most two order attempts total: one small bounded resting/cancel probe and
  one bounded immediate-execution probe, aggregate order notional no more than
  10 paper USD. If native minimums exceed this, stop rather than increase it.
- At most 12 signed native POSTs and 24 private account reads, 30-minute run.
  Fixed official testnet origin, no redirect, arbitrary endpoint or hidden retry.
- Explicit nonce/original-action persistence before signing. Any uncertain
  original is reconciled, never blindly resent. Small test collateral does not
  justify ambiguous behavior or false confirmation.
- Independently verify agent scope/revocation, original order IDs, actual
  partial-fill/fee identity/precision and observed response schemas. Record exact
  source versions and sanitized receipts; no keys/signed wires in public logs.
- Cancel/close known disposable paper exposure within the same cap where
  possible, revoke agents and record remaining uncertainty. Inability to clean
  up is reported, not declared success. No BULK funding bridge is tested here.

This establishes only native paper capabilities, not custody, margin isolation,
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
