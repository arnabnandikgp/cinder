# P23 financial continuation map

Updated 2026-10-07. An implementation/evidence map, not a new financial contract
or live authorization. [The runbook](../operations/live-qualification.md),
[ADR 0024](../architecture/0024-confidential-chain-ports.md),
[PLAN](PLAN.md#p23) and BASELINE remain authoritative.

The closed C5 run made **zero native and zero RPC connections**. It qualified
parts of transport/key/storage behavior, not deposits, venue credit, orders,
funding payments, payouts or recovery on actual rails. Historical M1/M2 results
do not grant current authority or qualify the changed measured application.

## Capability-to-code map

| Gate | Existing implementation | Missing connection / fresh evidence | Safe behavior until qualified |
| --- | --- | --- | --- |
| Account setup and lending | `funding::Setup`, `Controller::observe_setup`; settings/loan diagnostic GETs | Authenticated provider must bind the exact broker, fresh observed time, lending disabled, debt/interest and complete setup. Missing account/cache is not zero debt. Establish fresh-account bootstrap before deposit. | Native-credit readiness stays off. No historical deposit-first exception is inherited. |
| User deposit into Solana vault | `customer_deposit`, `chain_funding::deposit`, original finalized RPC/wire/account/code verifier, independent owner codec | Fresh devnet deployment, registered owner/recipient, mint/genesis/code/governance and original owner-signed deposit on the actual enclave RPC path. Present locator list is explicitly configured and bounded, not a general wallet-indexing frontend service. | No credit from a missing/unfinalized/ineligible transaction; eligible originals credit once. |
| Vault release and native deposit debit | Controller plans, enclave funds signer/chain codec, `chain_funding::issue/reconcile`, `observe_chain` | Actual allowlisted transfer and deposit effects through the shipping Runtime; retained original signature/wire, finality, counter and fee evidence | An original uncertain send remains pending; no second signature/POST. A deposit debit is not native credit. |
| Venue credit | Original finalized signature → transfer-observation join; `funding::Credit`, `Controller::observe_credit` | Fresh authenticated event capture; provider must establish native economic event, causal cut, actual gross credit/fee and operation-specific final total | Transit/holds remain; correlated observations, aggregate balance deltas, amount/time matching and ACKs cannot credit. |
| Order admission and dispatch | API intents/reservations, joined risk kernel, Gateway signatures and durable attempts | Install bounded current risk/capital/market policy; qualify native authority/agent permissions and exact grids. Connect intents to the prepared execution scheduler on the measured path. | Manifest rejects trading; API RiskAdmission disabled. No arbitrary native HTTP signer is exposed. |
| Fills, cancellations and final order cuts | Ordinary observations, `observation::Coverage`, `ingest_covered`, exact execution-set checks | Qualified provider for complete account/order history and native causal frontier, including late fills/cancel and pagination/disconnect behavior | Ordinary ingestion does not upgrade fills into qualified complete cuts. Retain uncertain commitments; do not infer terminal safety from cancel ACK/LI/end of page. |
| Perp funding and precision | Exact kernel accrual/settlement; diagnostic funding history with `FundingQualification` gap | Actual hourly boundary, sign on both sides, quote/grid/rounding and gross customer allocation when the pooled venue position is flat; connect native evidence to exact engine events | Displayed cumulative funding or a native account payout cannot directly settle every private customer. No invented rate or boundary. |
| Venue withdrawal and ordinary payout | Original UUID/native withdrawal plan, retained HTTP-200 ACK → batch mapping and transfer-observation join; `funding::Withdrawal`, `observe_withdrawal`; finalized chain return and P15 recipient payout/counters | Fresh authenticated ACK/event capture; lost native reply before durable ACK remains unresolved. Original native debit/no-later-effect evidence plus exact finalized payment to allowlisted broker; then vault return and payment to registered customer. Fresh fees must reconcile. | Request ACK or linked transfer observation does not discharge customer claim. Missing/pruned history stays pending; successful payment credits once. |
| Recovery | P21 final-cut/backing/kit workflow, native route binding and compiled P17/P15 claims | Actual native authority fencing, complete final cut, reachable funds returned, coherent finalized frozen custody state, funded claims, authorized activation and independent kit delivery/claim | No heartbeat-only payout or fabricated final cut. Unknown native exposure/backing blocks readiness; operator-assisted recovery remains explicit. |

The quoted types are trusted ports, **not** attestations produced by constructing
a Rust struct. A provider may derive a certificate from independently qualified
venue semantics and authenticated observations; a special venue-signed or ZK
API is not assumed mandatory. Conversely a response hash only binds bytes—it
does not establish completeness, causal ordering, ownership or no later effect.
If current APIs cannot support a required guarantee, report that specific gap
and seek a bounded alternative or product-policy review. Do not silently weaken
the guarantee to obtain a demo pass.

## What should happen next

The shipping continuation supplies manifest-bound packs and strict
[documented native evidence components](P23-NATIVE-EVIDENCE.md). Those parsers
do not connect or qualify a trusted financial provider. The
[approved storage run](P23-NEXT-RUN-PROPOSAL.md) now has a separate
[read-only hardware receipt](P23-PACKS-HARDWARE.md), including passing actual
Node/Chrome head-64 growth, storage faults, replay and freshness checks;
all money-movement and complete-cut gates below remain open.

1. Preserve the closed storage receipt and independently verified archive/owned
   cleanup as the baseline before financial journal traffic. The bounded growth
   gate passed; it is not production scaling or venue-semantic qualification.
2. Prepare authenticated setup/credit/withdrawal providers and their retained
   evidence contracts; connect them through the existing Controller. Reuse M1's
   scenario assertions and P22's fault cases, not its wallets, plaintext loaders,
   injected witnesses or prior permission. Start with account setup and an
   original-operation funding round trip, without trading.
   The offline capture transport/archive in
   [ADR 0027](../architecture/0027-native-transfer-capture.md) is the next bounded
   component: it retains raw transfer evidence without settlement or readiness.
   [Loaded-manifest/one-shot-worker wiring](../architecture/0028-measured-native-capture.md)
   is subsequently implemented and tested locally. Actual native qualification
   remains open; a local worker or TLS peer is not a deployed financial provider.
3. Add a **narrow measured financial activation policy** only once the relevant
   providers and bounded admission are tested. `boot::Manifest::validate` still
   rejects `funding` and `trading`; `Runtime::tick` has composed controller paths
   but leaves native finality/causal providers gated. Flipping booleans is not the
   missing implementation. Keep separate allowed actions and exposure caps.
4. Seal a fresh G05 manifest and seek approval before AWS, devnet deployment,
   faucet/account activity, sponsor-wallet use or any native POST. Include fresh
   identities, funding source/caps, fees, request/byte budgets, hard stop and
   original-operation/cleanup procedures. First live financial slice: owner
   deposit → vault → native credit → native withdrawal → vault → owner payout,
   with one lost-ACK/restart/original reconciliation case and no automatic resend.
5. Add bounded order lifecycle and funding qualification after funding round-trip
   evidence; then actual fenced/settled/backed recovery. Some observer semantics
   can be prepared independently, but trading cannot bypass credit/cut gates and
   recovery cannot bypass terminal history or returned backing.
6. Carry the existing Node/browser SDK and service qualification workflows onto
   those actual capabilities. A product trading terminal, market-data UI and
   self-service browser/agent UX remain separate follow-ups, not P23 merge gates.
   Clearly identify any separately demonstrated read-only/simulated UI. P24's
   release/customer-funds review and a new venue integration remain separate.

## Minimal evidence receipt for each provider

[The next native-semantics proposal](P23-NATIVE-QUALIFICATION.md) scopes fresh
setup/deposit/withdrawal/lost-native-reply qualification without an exploratory
AWS session. Its proposed faucet/devnet and bootstrap bounds are unapproved;
the exact runner/identity/source G05 record is still required before execution.

Record the bound environment/account/profile and source version/date, request and
response schema, authenticated channel, exact native/chain identity correlation,
event/frontier/finality and units, uncertainty/duplicate semantics, retained raw
evidence and negative tests. Private accounts, keys, signed wires and credentials
stay out of Git. Record exact source and distinguish documentation, synthetic
tests, historical runs and fresh actual observations.

This map makes the current gaps explicit; it neither declares the APIs incapable
nor presents an offline fixture as authentic native evidence. It does not add
new P23 milestones. The existing financial lifecycle and cleanup gates remain
open until their own actual receipts exist.
