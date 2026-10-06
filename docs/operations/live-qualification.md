# P23 live qualification

Updated 2026-10-06. **Offline preparation only.** The phase remains in progress;
no AWS session, deployment, funding, account setup or native transaction is
authorized by this document. Historical M1/M2/P20 receipts are provenance, not
current release qualification or reusable permission. See
[PLAN](../implementation/PLAN.md#p23), [TRACKER](../implementation/TRACKER.md),
[P22 composition](../architecture/0022-offline-acceptance.md) and
[new observation port](../architecture/0023-live-observation-ports.md).

## Audited shipping connections

| Boundary | Present implementation | Required before live acceptance |
| --- | --- | --- |
| Private HTTP/WS and wallet/agent signatures | SDK/WASM Noise, opaque relay and enclave `--web` ingress invoke the same API | Qualify this changed measured image and actual SDK on non-debug Nitro; P20's TLS-only receipts are not interchangeable |
| Private journal and freshness | Two S3 ciphertext replicas, separate DynamoDB witness, recipient-bound KMS and finite lease | New exact release/root/role policies and writer epoch; fresh interruption/restore receipts |
| Native signing | Durable Gateway dispatch and one-shot TLS POST | Fresh native account/agent authority, real precision profile, explicit activation and live observations |
| Native observations | Budgeted account-scoped GET port, ordinary ingestion and diagnostic replay | Wire bounded polling into the runtime under an approved release; qualify actual transport framing/schema and cadence |
| Complete order history | Explicit trusted `Coverage` seam and exact terminal execution-set checks | A qualified live provider; a hash, page ending, cancel ACK or successful TLS cannot stand in for completeness |
| Funding | Exact kernel accrual/settlement; native history retained with a named gap | Actual hourly boundary, both-sign convention, valuation/precision and gross customer allocation when the native position is net-flat |
| Solana custody | Actual Anchor/SPL program, TypeScript instruction and signed-wire verifiers; offline SBF tests | Confidential runtime transaction builder/signer, authenticated devnet RPC, finalized account/receipt recognizers and deployment bytecode checks |
| Native deposit/withdrawal | Funding Controller's durable plans and separate broker-owner signer | Authentic deposit-linked credit and original-UUID withdrawal history plus finalized recipient payment; no mock credit or balance-delta shortcut |
| Recovery | Fence/reconcile/settle/return/final-claim controller, recipient-encrypted kits and actual local SBF claims | Actual authority fencing, complete final cut, returned vault backing, authorized activation and independent kit delivery/claim on devnet |

The audited entrypoint currently refuses all live financial gates. API risk
admission is also disabled; `tick` does not provide a native observation or chain
controller. `clients/vault` is a codec/verifier, not an enclave RPC/signing service.
Its signer must not be moved onto the parent as a quick substitute. The current
five-role release has **no Solana funds seed**: extending the measured role
manifest is part of connecting the chain signer, not permission to reuse the
broker/trading key as the funds authority.

## Venue qualification findings

Primary docs were rechecked on 2026-10-06, not existing funded accounts. The five
observation endpoints and pagination fields match the existing adapter. GET
requests are account-scoped without trading signatures in the documented
examples. Pages are requested at 32 rows and charged conservatively without
an API config key. This establishes documented routing, not live accessibility.
See the [ADR's primary sources](../architecture/0023-live-observation-ports.md).

LI orders/correlates native updates but does not itself certify every economic
event is present. REST trade history documents IDs/cursors, not an authenticated
complete-cut certificate. **Inference:** the existing coverage port cannot be
enabled merely from these documentation fields. Qualification must establish
the complete applied execution set and no-later-effect semantics for the bounded
live account/order; otherwise retain holds and report the capability blocker.
[LI](https://docs.pacifica.fi/api-documentation/api/last-id),
[trade history](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-trade-history).

The funding page still has contradictory short-side balance language. An account
payout/rate is not sufficient by itself to allocate funding to opposite gross
customer positions when native net exposure is zero. Do not paper over this by
assigning the displayed rate directly to customer cash.
[Funding policy](https://docs.pacifica.fi/trading-on-pacifica/funding-rates),
[account funding history](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-funding-history).

Historical M1's actual faucet-only round trip is valuable input: customer 20
USDP returned as 19 after a 1-USDP venue withdrawal fee, including a dropped
withdrawal acknowledgement/restart and no second POST. That run needed an
**explicitly approved** deposit-first/lending-disable exception. It cannot be
assumed for fresh P23 accounts. Historical P2/P3 also found broader native agent
powers and precision discrepancies; repeat relevant checks before enabling a
new account/profile. Local research locations are indexed in
[EVIDENCE W03–W05](../implementation/EVIDENCE.md), never published as raw keys,
signed wires or account histories.

## Bounded continuation, without new acceptance milestones

1. Complete production ports locally: native observation scheduling, exact
   Solana instruction/wire/signing boundary and finalized receipt recognition,
   setup/deposit/payment recognizers and recovery orchestration. Promote useful
   experimental assertions/ABI vectors, not the experiments' wallet loaders,
   plaintext journal, injected settlement or historical live driver.
2. Qualify complete order/funding/payment semantics explicitly. Where public
   evidence is insufficient, prepare a bounded test/venue question. Keep the
   corresponding capability off; a named blocker is an acceptable *finding*,
   not a full live workflow pass. No financial promise or source cut changes
   merely to get a demo to settle.
3. Seal a **fresh G05 execution manifest** and request user approval, then run
   the actual workflow on one disposable testnet pool, a small market selection
   and preconfigured test customers. No mainnet/customer assets or new venue.
4. Reuse P22's independently checked lifecycle assertions on actual rails;
   obtain hardware, chain and venue evidence separately. Persist attempts/wires
   before exposure. Interrupt real processes/connections at declared boundaries;
   reconcile before retry. Funding absence is not evidence of a tested payout.
5. Fence/revoke ordinary authority, return reachable collateral, finalize funded
   claims, deliver kits independently, claim once and test duplicate rejection.
   Report any unresolved authority/funds/history instead of substituting fixtures.
6. Close orders/positions and reconcile customer/house/native/transit/vault
   amounts, actual fees/rent and all retained obligations. Teardown only this
   run's cloud resources. Publish a sanitized exact-source receipt and refresh
   public environment availability from that receipt. P24 remains separate.

## G05 manifest checklist

The executable manifest must bind all of these **before the first live action**:

- Reviewed source, tool/lock, program/IDL/bytecode and measured EIF/PCR hashes;
  finite lease, image entry mode, KMS purposes, storage/witness and fresh epoch.
- Exact devnet genesis, RPC origin/CA/secret handling, Pacifica testnet origin,
  quote mint/decimals/native program/vault, market grids and qualification levels.
- Fresh deployment, customers, broker, trading agent, funds/recovery/governance
  identities and signing custody; never use fixed public fixture identities.
- Explicit sponsor funding authority, deployment/rent/fee caps in devnet SOL,
  faucet-only quote cap, order/movement/notional/loss caps, API/RPC/submission
  budgets, pacing and hard deadline. External shocks can still breach a risk cap;
  record consequences rather than suppressing events or erasing obligations.
- Current bootstrap/lending policy and stop-on-failure behavior, per-action
  simulation/summary, no auto-refreshed signature after an unknown outcome.
- AWS profile/region, eligible instance, storage/KMS/witness resources and dollar
  cap; estimate current prices before approval. Historical AWS caps do not apply.
- Fresh versus restored run identities, exact fault-injection boundaries,
  reviewer/source pins, evidence redaction, final reconciliation and cleanup list.

Numbers/identities are deliberately not fabricated now. Fill this checklist from
the locally reviewed connected runner and current environment metadata, show
the proposed test exposure/cost to the user, and stop for approval. Setting an
`approved` boolean or changing `boot::Gates` is not a substitute for the missing
connections, qualification or G05 authority.
