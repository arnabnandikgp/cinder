# P40 assembled paper-trading gate

Status: user-approved local implementation boundary, 2026-10-21. Not a live approval or a
change to customer funding, collateral ownership, recovery or production policy.
[Plan](../../implementation/PLAN.md#p40), [existing evidence](../../venues/experimental-parity.md).

## What remains

The SDK, shared Runtime/journal, financial admission, native workers and real
testnet egress exist. Synthetic joined tests exercise them together. A direct
BULK paper runner has actual native open/close evidence, and the exact October
20 Nitro image has real two-unit read/admin evidence. No single test joins all
of these with genuine external trading and attributable financial initialization.

Fresh shipping books deliberately contain no cash, native credit readiness,
collateral marks, joined risk policy or capital. `Admission::order_holds`,
`State::exposure_qualified` and the native gateways reject that state. The local
fixture's `seed_for`/`NativeCreditReady(true)` path is synthetic and cannot be
copied into a deployment or relabeled as native evidence.

## Recommended bounded paper-only contract

Use one fresh, exclusively controlled BULK **paper** account and a disposable
Cinder test customer. No existing wallet, customer assets, Solana transaction,
bridge, withdrawal or recovery payment. Pacifica remains read-only; its deferred
missing-credit investigation is not reopened.

For example, if the fresh account actually receives 1,000 faucet paper USD,
record the exact observed amount once in the authoritative ledger. Designate
20 paper USD as the disposable customer's test entitlement and the remainder as
test house resources. These are allocations of the **same** observed paper cash,
not 1,000 plus 20 of assets, a real customer deposit or extra collateral created
by a risk policy. Require sufficient actual faucet backing; never assume the
faucet allocation. No house/customer record can exist before its evidence.

This needs an explicit paper-only evidence assumption: fresh-account exclusive
control, observed setup, no prior activity and retained authenticated account/
position/history responses may establish the initial bounded test cut. The
current native API does not provide a general causally complete financial
certificate. If these assumptions are not approved, leave ordinary shipping
admission closed rather than fabricate a qualified frontier.

Primary documentation rechecked October 21: the [API overview](https://docs.bulk.trade/api-reference/introduction)
separates testnet/mainnet; the [faucet](https://docs.bulk.trade/api-reference/requestFaucet)
is a signed paper-funds action; [account queries](https://docs.bulk.trade/api-reference/getAccount)
distinguish current state from paginated fill/funding/risk histories. A history
page's coverage is not automatically a complete cross-family financial cut.
The proposed closed fresh-account treatment is an explicit test assumption,
not a new native guarantee inferred from those docs. No live venue request was
made in this documentation recheck.

Implement the boundary separately from the read-only converter, default disabled
and bound into the measured release/private configuration. It must be testnet-
only, one-shot, short-lived and original-evidence-bound, with no public readiness
setter, unrestricted cash import, broker/master fallback or fixture feature.
Use explicit small test risk/capital/scenario limits; do not advertise them as
calibrated production policy. Native decimal/fee semantics still need actual
versioned evidence, not an activated Boolean.

Then place orders through the real UnitClient → confidential handler → admission
→ original journal attempt → worker → native agent path. Actual fills/fees must
reach the original unit/customer exactly once. Response loss stays uncertain;
no automatic replacement, terminal-history invention or customer payout follows.
Retain holds when completion cannot be established. Known exposure must have
separately reserved native cleanup capacity; cleanup is not proof of ordinary
order completion. Finish flat, cancel known resting orders and revoke test agents
where the original evidence and finite allowance permit.

## Sequence and finite gates

1. **Approved:** the paper-only initialization/evidence contract above.
   This is a product/test decision, not permission to move real funds or launch AWS.
2. Implement it with deterministic offline tests for once-only allocation,
   altered/reused source evidence, missing backing, wrong unit/account/asset,
   expiry, restart, signed intent attribution and refusal of production use.
   Preserve the existing production funding/completeness contracts.
3. Prepare one fresh source-bound native run proposal: at most five order
   attempts and $20 cumulative native limit notional including cleanup, fresh
   faucet/agent setup, explicit signed/read budgets and a 30-minute deadline.
   Validate current market minima/grids and reserve cleanup before opening.
   Old paper run identities, original requests and exhausted budgets are never
   reused. No execution occurs just because this document exists.
4. Run the actual service/SDK paper lifecycle only within its approved scope.
   Record the original native evidence and remaining unknown cases honestly.
5. Requalify the final measured source on a separately approved fresh Nitro run;
   combine the reviewed startup fixes and paper slice into one image/run where
   feasible, instead of paying for repeated intermediate images. Keep all AWS,
   financial-action and cleanup caps explicit. Never deploy local-fixture code.

The implementation uses a separate **version-9** measured/private profile;
versions 7/8 omit it and remain uninitialized. [ADR 0044](../../architecture/0044-paper-bootstrap.md)
defines its exact assumption, authoritative cash split, history quarantine,
finite risk settings and cleanup reservation. Native `coverage: unknown` and
missing optional snapshot inventories remain unknown in the ordinary codecs;
only fresh exclusive control/retained setup closes that gap for this paper test.
No native `source_cut`, terminal certificate or production funding proof is invented.
See the [fresh run proposal](P40-PAPER-RUN-PROPOSAL.md) before any live activity.

The October 21 recipient/lease review fixes change enclave code. October 20's
[hardware receipt](unit-hardware-20261020.md) remains valid for its
recorded image only; rebase or green offline CI does not qualify a new image.

## Alternative

Keep the full current evidence bar and wait for supported native funding and
causal reconciliation evidence. The common SDK/worker implementation and existing
paper/read-only receipts remain useful, but the assembled financial criterion
stays open. Neither option authorizes a production rollout or a third venue.
