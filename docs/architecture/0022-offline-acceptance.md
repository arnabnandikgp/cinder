# Joined offline acceptance

P22 joins the existing components; it does not change the approved economic
contract or authorize deployment. [Baseline](../implementation/BASELINE.md),
[phase criteria](../implementation/PLAN.md#p22),
[manifest](../implementation/acceptance-manifest.json).

## Composition and trust boundaries

The normal workflow uses the real private SDK, shared WASM Noise core, opaque
parent relay, runnable Rust private service, API authorization, risk-derived
reservations, Pacifica Gateway/funding Controller and replicated AEAD journal.
Owner signatures and pooled signing are different authorities. The worker dies
and reopens the same accepted journal; reconnect uses a fresh session/quote.

`--acceptance-web` exists only in the explicitly enabled `local-fixture` binary.
Its privileged controller inputs arrive on stdin, never an HTTP/WebSocket route.
There is no production fallback, configured wallet, remote RPC or native endpoint.
The test parent is deliberately trusted with disposable test inputs; it is not
the production relay or a demonstration of a parent reading real private state.

| Component / fact | Evidence in this suite | Not established |
| --- | --- | --- |
| SDK commands, owner/agent authorization, private reads/updates | Actual encrypted HTTP and WebSocket processes; real signed API bytes | Public deployment/self-service onboarding |
| Vault deposit, release, return and beneficiary payout | Compiled Anchor 1.2.0 SBF on offline Surfpool; signed transactions, movement accounts and token/counter reads | Devnet deployment or Pacifica bridge finality |
| Native deposit | Controller ABI/wire validation and durable persistence, then explicit synthetic debit/credit ports; local SPL burn models the debit | Executed native SBF deposit or actual venue credit |
| Native withdrawal | Original UUID-linked synthetic history and trusted payment port; local mint supplies fake settlement liquidity | A real withdrawal, authenticated bridge payment, or venue solvency |
| Trading, fills, funding and terminal coverage | Gateway signs once to a fake unknown-response transport; real native normalization/controller/ledger with qualified fake observations | Live causal completeness, funding precision, ADL qualification or mainnet parity |
| Recovery | Existing joined HTTP/WS accepted-tail outage suite, real final SBF claims and independent recipient kit submission | Operator-free exit or complete-liabilities proof |
| Attestation, time, replicas and freshness | Synthetic CA, injected wall time, local encrypted replicas/witness; existing actual browser/channel regressions | Fresh Nitro quote, independent witnesses/stores, power-loss qualification |

Physical toy quote amounts use zero decimals. They do not qualify Pacifica's wire
units. House capital starts explicitly at 1,000 in the fake native account;
customer credit starts at zero and follows the actual local deposit receipt.

## Joined normal oracle

Both carriers execute the following path. Each boundary checks independent
customer/house/native books, physical vault/broker/transit, customer holds,
unresolved inputs and shortfall, plus the signed SDK view and actual token accounts.
House cash stays 1,000 and house quantity/basis/funding stay zero throughout.
Funding below is settled; no unsettled amount is counted a second time.

| Boundary | Customer cash; q; basis | Native cash; q; basis | Vault / broker / transit | Customer hold |
| --- | --- | --- | --- | --- |
| Start | 0; 0; 0 | 1,000; 0; 0 | 0 / 0 / 0 | 0 |
| Actual deposit, duplicated credit observation | 1,000; 0; 0 | 1,000; 0; 0 | 1,000 / 0 / 0 | 0 |
| Release working collateral 60 | 1,000; 0; 0 | 1,000; 0; 0 | 940 / 60 / 0 | 0 |
| Fake native source debit | 1,000; 0; 0 | 1,000; 0; 0 | 940 / 0 / 60 | 0 |
| Partial credit 30; early order blocked | 1,000; 0; 0 | 1,030; 0; 0 | 940 / 0 / 30 | 0 |
| Final credit 30 and independent native check | 1,000; 0; 0 | 1,060; 0; 0 | 940 / 0 / 0 | 0 |
| Two-lot GTC accepted, process killed/retried | 1,000; 0; 0 | 1,060; 0; 0 | 940 / 0 / 0 | 24 |
| One lot filled at 100, fee 1 | 999; 1; 100 | 1,059; 1; 100 | 940 / 0 / 0 | 24 |
| Cancel sent; another lot fills at 100, fee 1 | 998; 2; 200 | 1,058; 2; 200 | 940 / 0 / 0 | 24 |
| Exact covered terminal execution set | 998; 2; 200 | 1,058; 2; 200 | 940 / 0 / 0 | 0 |
| Funding -2 per lot recognized and settled | 994; 2; 200 | 1,054; 2; 200 | 940 / 0 / 0 | 0 |
| Bounded private close admitted | 994; 2; 200 | 1,054; 2; 200 | 940 / 0 / 0 | 24 |
| Two lots closed at 110, realized PnL 20, fee 2 | 1,012; 0; 0 | 1,072; 0; 0 | 940 / 0 / 0 | 24 |
| Covered close completion and reconciliation | 1,012; 0; 0 | 1,072; 0; 0 | 940 / 0 / 0 | 0 |
| Payout queued, local cash insufficient | 1,012; 0; 0 | 1,072; 0; 0 | 940 / 0 / 0 | 1,012 |
| Unknown native withdrawal of 72 reconciled | 1,012; 0; 0 | 1,000; 0; 0 | 940 / 72 / 0 | 1,012 |
| Actual SBF return | 1,012; 0; 0 | 1,000; 0; 0 | 1,012 / 0 / 0 | 1,012 |
| Actual final payout and restart | 0; 0; 0 | 1,000; 0; 0 | 0 / 0 / 0 | 0 |

The recipient receives 1,012 and its persistent paid counter is 1,012. No opening
notional is debited as if this were spot. Wrong destination receipts leave the
financial oracle unchanged; exact payment receipts and native fills post once.
An accepted payout is not paid while money remains inaccessible at the venue.
Partial/late fills also emit actual private position snapshots over WebSocket,
including when commands originate on HTTP. Existing shared Node/Chrome tests
exercise dropped notifications, resnapshot, revocation and slow-consumer overflow.

## Explicit causal-coverage input

Ordinary Pacifica parsing intentionally emits no `source_cut`: `has_more=false`,
LI and a cancel ACK cannot certify economic history. This prevented the joined
fixture from completing an order without bypassing the controller.

The owning observation layer now accepts an **explicit trusted coverage input**
binding immutable profile, exact body SHA-256, positive source-local frontier and
retained certificate commitment. It is archived with raw evidence; only that
input supplies a normalized fill's cut. Wrong bindings are rejected before commit.
A later duplicate cannot upgrade an earlier unqualified fill. Default ingestion
and `View::complete_history()` remain unqualified. Construction of `Coverage`
does not verify a live certificate. No provider is enabled by this change;
P23/G01 must independently qualify how a live authenticated cut is obtained.

The terminal controller still needs the exact applied execution set and filled
quantity through that frontier. It cannot release holds on ACK/time alone.

## Independent reference, stress and adversarial replay

`crates/kernel/tests/acceptance.rs` has a separate widened-integer reference book;
it calls no production position/math functions to calculate expected results.
Exhaustive cases cover quantities -3..3, basis magnitudes 0..31, deltas -4..4
excluding zero, prices 1/7/11 and marks 1/50/250: **4,632 valid transitions**.
These are bounded synthetic cases, not exhaustive full-width verification.

Eight tracked seeds generate 64 fills each, 576 checked cuts including funding
recognition. The oracle compares each user's cash, quantity, basis and accrual,
house/native state, positive claims, deficits, assets and shortfalls. Duplicates
must be no-ops; changed same-ID fees are retained conflicts without postings;
subsequent actual losses still post. Negative equity is never collectible cash.

Reverse stress searches upward marks 100..400 for opposing 1..4-lot users, each
with collateral 100 and house capital 30. First shortfall marks are respectively
231, 166, 144 and 133. These are **expected counterexamples**, not failing tests or
recommended leverage limits. A correct net-flat ledger can be underbacked.

The manifest also traces existing production-kernel/journal tests for RF1/RF2,
fund exhaustion, remediation, repeated shocks, funding gaps, location liquidity,
pending-order/payout races, adversarial source ordering, finite parser mutation,
rollback and actual process-kill boundaries. Reuse these owning-layer tests;
do not add alternate ledgers or weaken policy to make the joined fixture pass.

## Reproduction and budgets

Use [development setup](../development.md), [browser setup](../../tools/web-channel/README.md)
and [SBF setup](../../programs/README.md). Tools/dependencies must first be
explicitly hydrated. All acceptance execution is locked/offline and loopback.

```sh
node scripts/check-acceptance.mjs --manifest-only
node scripts/check-acceptance.mjs
# Focused local normal lifecycle iteration, not full acceptance:
node scripts/check-vault.mjs --acceptance-only
```

The consolidated runner invokes the same three runners already used by hosted
CI; it does not duplicate them inside each job. Manifest validation is part of
the root checks. Missing dependencies/browser/tools fail, not skip. Seed digest
and named tests must resolve in a fresh checkout without `work/`.

Declared test-workload budgets: workspace 600s, browser 600s, SBF 1,500s.
The consolidated runner measures and rejects successful suites exceeding those
budgets; hosted jobs also impose execution deadlines. Per carrier, 64 signed
reads must finish within 30s without changing the financial cut; encrypted fixture
storage must remain below 8 MiB and the **Node harness** RSS below 512 MiB.
This is not a Rust-enclave RSS measurement or a production throughput/SLA claim.
The RF2 owning test separately enforces at most 2,000,000 work units and 1,024
segments for its trillion-lot profiles. Failures require investigation, not
silently relaxed economics. Exact measurements belong in the tracker.

## Limits and next gate

Evidence is **tested within these bounds**, not proven for all schedules, prices
or outages. A fixture cut, independent expected ledger, inclusion proof and
attested channel are not a complete-reserves or universal solvency proof.
Live native semantics/finality, calibrated coverage/capital, independent
freshness/storage, governance and the changed Nitro image remain P23/G01–G04.
P22 alone does not enable the P20 hardware runtime's disabled native admission
or constitute a live frontend/venue integration. Preserve those gates explicitly.
