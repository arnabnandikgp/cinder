# P23 live qualification

Updated 2026-10-06. **Offline preparation only.** The phase remains in progress;
no AWS session, deployment, funding, account setup or native transaction is
authorized by this document. Historical M1/M2/P20 receipts are provenance, not
current release qualification or reusable permission. See
[PLAN](../implementation/PLAN.md#p23), [TRACKER](../implementation/TRACKER.md),
[P22 composition](../architecture/0022-offline-acceptance.md) and
[observation port](../architecture/0023-live-observation-ports.md) and
[confidential chain ports](../architecture/0024-confidential-chain-ports.md).

## Audited shipping connections

| Boundary | Present implementation | Required before live acceptance |
| --- | --- | --- |
| Private HTTP/WS and wallet/agent signatures | SDK/WASM Noise, opaque relay and enclave `--web` ingress invoke the same API | Qualify this changed measured image and actual SDK on non-debug Nitro; P20's TLS-only receipts are not interchangeable |
| Private journal and freshness | Two S3 ciphertext replicas, separate DynamoDB witness, recipient-bound KMS and finite lease | New exact release/root/role policies and writer epoch; fresh interruption/restore receipts |
| Native signing | Durable Gateway dispatch and one-shot TLS POST | Fresh native account/agent authority, real precision profile, explicit activation and live observations |
| Native observations | Finite read-only scheduler, budgeted account-scoped GETs and diagnostic archives | Qualify actual transport framing/schema/cadence and current account setup; diagnostic bodies cannot supply missing causal certificates |
| Complete order history | Explicit trusted `Coverage` seam and exact terminal execution-set checks | A qualified live provider; a hash, page ending, cancel ACK or successful TLS cannot stand in for completeness |
| Funding | Exact kernel accrual/settlement; native history retained with a named gap | Actual hourly boundary, both-sign convention, valuation/precision and gross customer allocation when the native position is net-flat |
| Solana custody | Controller-bound enclave codec/signing/RPC, streamed approved code, finalized receipts/counters and owner-deposit observer; independent Anchor vectors/SBF tests | Qualify changed-image six-role release, actual devnet deployment/governance/CA/genesis/framing and original transaction effects |
| Native deposit/withdrawal | Funding Controller's durable plans and separate broker-owner signer | Authentic deposit-linked credit and original-UUID withdrawal history plus finalized recipient payment; no mock credit or balance-delta shortcut |
| Recovery | Fence/reconcile/settle/return/final-claim controller, recipient-encrypted kits and actual local SBF claims | Actual authority fencing, complete final cut, returned vault backing, authorized activation and independent kit delivery/claim on devnet |

The entrypoint still refuses live **financial** activation and API risk admission
is disabled. Version 2 now permits bounded read-only qualification, including
one configured original owner deposit per cadence; it does not create funds/order
intents. The six-role release adds a distinct Solana funds seed and binds actual
loaded RPC/policy/key identity into the application commitment. Version 1 remains
the old inactive five-role encoding. Neither broker/trading nor storage seeds may
substitute for funds. Parent relays forward opaque HTTPS bytes only; `clients/vault`
remains an independent owner-side codec/oracle, not a plaintext parent signer.

The chain controller locally issues/reconciles existing durable mandates, but
shipping manifest validation **still rejects** `funding` and `trading`. Do not
flip them before a bounded policy and authentic evidence providers are qualified.
Current prepared recovery interfaces likewise do not invent a final native cut.

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

Settings and loan diagnostics preserve the distinction between default-enabled
lending, missing cache and proven zero debt. The documented balance-history
schema does not provide original deposit signature or withdrawal UUID linkage.
Historically observed withdrawal-history/pending routes are diagnostic candidates,
not a documented current payment certificate. A response or amount/time match
does not enable native credit, payment, complete coverage or recovery readiness.
[Settings](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-settings),
[loan](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-loan-info),
[balance history](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-balance-history).

## Fixed local gate checklist

These are preparation gates for the existing phase, not new implementation phases.
The completed-run receipt belongs in TRACKER/EVIDENCE; live acceptance below stays
open until actual evidence is recorded.

| Gate | Exact required evidence |
| --- | --- |
| Controller and owner codecs | Four physical rails plus owner deposit, optional CU limit, exact large-u64/signature/ABI vectors independently checked in Rust and Anchor/web3.js |
| Enclave RPC and receipts | TLS/root/hostname/time/framing negatives; original finalized status/wire, classic SPL/mint/recipient/counter checks; bounded full-code/tail/header stream |
| Joined durable funds | One authoritative journal; lost ACK/restart preserves original wire without retry; missing history/simulation/fee failure cannot credit; release/return/payout and owner credit exactly once; native debit stays pending |
| Release and scheduler | Version-1 compatibility, version-2 six-role private preparation, distinct seeds, actual component binding, finite lease/stop, durable read budget/cooldown/revocation |
| Full host regression | Pinned `node scripts/check.mjs`: contracts, dependencies, strict default/all-feature lint/build, debug/release/default Rust, relay and private SDK |
| Program and browser | Full `node scripts/check-vault.mjs` with offline Surfpool; actual Node/Chrome/WASM `node tools/web-channel/check.mjs`, no skipped required workflows |
| Shipping package | Source-only, network-disabled ARM64 default-feature package; independent application rebuild has identical ELF; ordinary non-Nitro boot refuses |

No native semantics are silently marked passed by these local fixtures. Native
setup/credit/payment/cut/funding and actual cloud/entropy/attestation evidence
remain the named live qualification targets.

## First AWS step after local gates

Start with the changed-image **read-only** qualification: actual Noise HTTP/WS
SDK → opaque relay → non-debug Nitro; six-purpose KMS release and expected
application commitment; devnet RPC/TLS/genesis/deployed-code observation;
account-bound native settings/loan/history schema reads, finite budgets and
interruption/restore/fencing. Observe one fresh owner-signed P15 deposit only
after its separate devnet funding permission is in the current manifest.

If a capability is unavailable, preserve exact private evidence and report the
named blocker. Financial activation is a follow-on qualification inside P23,
not an assertion that this read-only build executes every trading/recovery
workflow. Define the authentic setup/credit/payment/cut providers and finite
financial policy from observed evidence before issuing any native mandate.
No existing wallet, deployment account, quote balance, AWS resource, price or
authority approval is assumed current.

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

1. Complete the fixed local preparation gates above, using the shipping controller
   and promoted ABI/failure assertions rather than prototype wallet loaders,
   plaintext journals, injected native settlement or historical live drivers.
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
