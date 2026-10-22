# Implementation tracker

Current implementation milestones; evidence and deployment limits are in
[status](../status.md). Progress values remain `open`, `in progress`, `closed`.
Merge does not close an unmet qualification criterion.

| Phase | Progress | Invariant / deliverable | PR / merge |
| --- | --- | --- | --- |
| [P00 — Foundation](PLAN.md#p00) | closed | Every phase has approved scope, testable completion criteria and a reproducible next-agent handoff. | [#23](https://github.com/arnabnandikgp/cinder/pull/23), merged `587a8ca` |
| [P01 — Workspace and harness](PLAN.md#p01) | closed | A pinned, offline-buildable workspace keeps the pure kernel separate from I/O and supplies deterministic fault-test ports. | [#24](https://github.com/arnabnandikgp/cinder/pull/24), merged `5aea02e` |
| [P02 — Financial types and identities](PLAN.md#p02) | closed | Exact units and canonical identities prevent precision loss, overflow and cross-domain replay. | [#25](https://github.com/arnabnandikgp/cinder/pull/25), merged `17a29e2` |
| [P03 — Unified ledger](PLAN.md#p03) | closed | One attributed ledger reconciles user/house claims and external exposure without hiding deficits or double-counting assets. | [#26](https://github.com/arnabnandikgp/cinder/pull/26), merged `3bd8c12` |
| [P04 — Funding, fees and reconciliation](PLAN.md#p04) | closed | Funding and fees post once to the correct owner; unexplained differences remain visible and restrict dependent actions. | [#27](https://github.com/arnabnandikgp/cinder/pull/27), merged `95fe3b7` |
| [P05 — Durable journal](PLAN.md#p05) | closed | Postings, holds and consumed-event keys commit atomically and rebuild identically after a crash. | [#29](https://github.com/arnabnandikgp/cinder/pull/29), merged `95fe3b7` |
| [P06 — Encrypted durability](PLAN.md#p06) | closed | Private state survives qualified failures without plaintext leakage, silent rollback or revived stale writers. | [#30](https://github.com/arnabnandikgp/cinder/pull/30), merged `95fe3b7` |
| [P07 — Order intents and reservations](PLAN.md#p07) | closed | Durable reservations precede dispatch; acknowledgements and timeouts cannot fabricate fills or release unknown commitments. | [#31](https://github.com/arnabnandikgp/cinder/pull/31), merged `95fe3b7` |
| [P08 — Funds and payouts](PLAN.md#p08) | closed | Partial money movements reconcile by location; only final payment discharges a user claim, exactly once. | [#32](https://github.com/arnabnandikgp/cinder/pull/32), merged `95fe3b7` |
| [P09 — Joined risk admission](PLAN.md#p09) | closed | New actions pass user, pool, capital and location-liquidity checks across bounded pending outcomes. | [#33](https://github.com/arnabnandikgp/cinder/pull/33), merged `95fe3b7` |
| [P10 — Protection claims](PLAN.md#p10) | closed | House protection absorbs eligible losses once; repeated defaults and recoveries preserve ownership and unpaid claims. | [#34](https://github.com/arnabnandikgp/cinder/pull/34), merged `95fe3b7` |
| [P11 — Liquidation and exceptions](PLAN.md#p11) | closed | Bounded liquidation and late-close handling record residual risk without silently reversing customers or erasing losses. | [#35](https://github.com/arnabnandikgp/cinder/pull/35), merged `95fe3b7` |
| [P12 — ADL and restoration](PLAN.md#p12) | closed | Qualified ADL/restoration obeys RF1 basis economics and RF2 proportional quotas within funded execution limits. | [#36](https://github.com/arnabnandikgp/cinder/pull/36), merged `95fe3b7` |
| [P13 — Pacifica observations](PLAN.md#p13) | closed | Native observations normalize losslessly with explicit provenance, completeness limits and capability qualification. | [#37](https://github.com/arnabnandikgp/cinder/pull/37), merged `95fe3b7` |
| [P14 — Pacifica execution](PLAN.md#p14) | closed | Only authorized, reserved actions are signed; unknown outcomes reconcile before retry and cleanup capacity stays available. | [#38](https://github.com/arnabnandikgp/cinder/pull/38), merged `95fe3b7` |
| [P15 — Solana vault](PLAN.md#p15) | closed | Vault movements enforce asset, authority, recipient, epoch and atomic payout-counter boundaries. | [#39](https://github.com/arnabnandikgp/cinder/pull/39), merged `95fe3b7` |
| [P16 — Funding coordinator](PLAN.md#p16) | closed | Collateral completes the supported vault/venue round trip without duplicate funding, withdrawal or customer credit. | [#40](https://github.com/arnabnandikgp/cinder/pull/40), merged `95fe3b7` |
| [P17 — Recovery program](PLAN.md#p17) | closed | Authorized, funded final claims pay once; stale roots, wrong recipients and prior ordinary payouts cannot replay. | [#41](https://github.com/arnabnandikgp/cinder/pull/41), merged `1f750b0` |
| [P18 — Private API and SDK](PLAN.md#p18) | closed | Customers access only their own scoped operations and views through a secure SDK, never raw pooled-account authority. | [#42](https://github.com/arnabnandikgp/cinder/pull/42), merged `1f750b0` |
| [P19 — Attested transport and runnable service](PLAN.md#p19) | closed | A runnable relay/private-service slice carries authenticated SDK operations through verified encrypted sessions; parent sees no private plaintext. | [#44](https://github.com/arnabnandikgp/cinder/pull/44), merged `dc5f01d` |
| [P20 — Nitro application qualification](PLAN.md#p20) | closed | The assembled confidential application runs on actual Nitro with qualified key release, egress, restore and fencing; mocks are not hardware evidence. | [#45](https://github.com/arnabnandikgp/cinder/pull/45), merged `76f3295` |
| [P21A — HTTP and WebSocket API](PLAN.md#p21a) | closed | Browser/Node HTTP and WebSocket commands and private updates preserve attested confidentiality, owner permissions and one authoritative financial journal. | #47, #48, #50 and [#51](https://github.com/arnabnandikgp/cinder/pull/51), merged `0530374` |
| [P21B — Public documentation](PLAN.md#p21b) | closed | Copy-ready MDX guides and structured method references describe implemented contracts and actual environment availability, not planned endpoints or unverified guarantees. | [#52](https://github.com/arnabnandikgp/cinder/pull/52), merged `1c2c3eb` (tracked delivery receipt only) |
| [P21 — Recovery integration](PLAN.md#p21) | closed | Fenced and reconciled funds back final claims; users obtain claim packages without the ordinary trading API. | [#52](https://github.com/arnabnandikgp/cinder/pull/52), merged `1c2c3eb` |
| [P22 — Offline adversarial acceptance](PLAN.md#p22) | closed | SDK/service-driven trading, funding and recovery workflows preserve accounting under reproducible external fixtures, process faults and races. | [#53](https://github.com/arnabnandikgp/cinder/pull/53), merged `8247d24` |
| [P23A — Venue-directed custody contract](PLAN.md#p23a) | closed | Approved escrow ownership, full-batch sweep/refund, credit and payout/recovery rules are explicit and distinguish future behavior from existing evidence. | [#64](https://github.com/arnabnandikgp/cinder/pull/64), merged `09494db` |
| [P23B — Atomic escrow program/client](PLAN.md#p23b) | closed | A deposit batch is fully swept or owner-refunded once under the fixed venue route; normal/recovery payment history cannot reset. | [#65](https://github.com/arnabnandikgp/cinder/pull/65), merged `b162d8f` |
| [P23C — Escrow funding integration](PLAN.md#p23c) | closed | One journal recognizes handoff once, gates trading on qualified credit and preserves liabilities, refunds and shared payout protection through faults. | [#67](https://github.com/arnabnandikgp/cinder/pull/67), merged `7e04a94` |
| [P23 — Live integration qualification](PLAN.md#p23) | in progress | The same workflow assertions run on approved actual Nitro/chain/venue rails; receipts, test exposure, funds and cleanup reconcile. | [#55](https://github.com/arnabnandikgp/cinder/pull/55) → [#56](https://github.com/arnabnandikgp/cinder/pull/56) → [#58](https://github.com/arnabnandikgp/cinder/pull/58) → [#59](https://github.com/arnabnandikgp/cinder/pull/59) → [#60](https://github.com/arnabnandikgp/cinder/pull/60) → [#61](https://github.com/arnabnandikgp/cinder/pull/61) → [#62](https://github.com/arnabnandikgp/cinder/pull/62) → [#63](https://github.com/arnabnandikgp/cinder/pull/63), merged through `8b93adb`; live financial acceptance deferred |
| [P25 — Backend onboarding and escrow preparation](PLAN.md#p25) | closed | A fresh wallet discovers and establishes its own private account and prepares escrow funding through the SDK without customer-specific startup configuration or a frontend. | [#68](https://github.com/arnabnandikgp/cinder/pull/68), merged `52a36ef` |
| [P26 — Second-venue assessment and implementation boundary](PLAN.md#p26) | closed | A source-pinned candidate and explicit unit ownership/evidence gates guide one second venue without inventing native support or cross-venue collateral. | [#69](https://github.com/arnabnandikgp/cinder/pull/69), merged `0af365e` |
| [P27 — BULK exact wire and fill codec foundation](PLAN.md#p27) | closed | Exact offline native bytes, identities and fill facts remain separate from financial posting, live dispatch and causal-readiness authority. | [#71](https://github.com/arnabnandikgp/cinder/pull/71), merged `b818655` |
| [P28 — BULK financial observations and execution-unit contract](PLAN.md#p28) | closed | Native financial observations retain explicit units, ownership and qualification limits; a reviewed two-unit contract precedes runtime composition. | [#72](https://github.com/arnabnandikgp/cinder/pull/72), merged `b4dce81` |
| [P29 — New-history execution-unit journal foundation](PLAN.md#p29) | closed | Explicit unit components share one durable head, customer ownership and revocation without sharing assets or exposure authority. | [#73](https://github.com/arnabnandikgp/cinder/pull/73), merged `793a05c` |
| [P30 — Signed unit-bound API and SDK contract](PLAN.md#p30) | closed | Signatures, economic identity, replies and owner/agent checks bind one explicit execution unit before later financial admission. | [#74](https://github.com/arnabnandikgp/cinder/pull/74), merged `1e9cd2d` |
| [P31 — Durable unit-scoped API history](PLAN.md#p31) | closed | Original signed operations and accepted unit-scoped grants survive replay under one head; recording does not imply financial admission. | [#76](https://github.com/arnabnandikgp/cinder/pull/76), merged `d23e33c` |
| [P32 — Unit-scoped financial admission](PLAN.md#p32) | closed | Original requests, permissions, quota and unit-local risk/holds form one replay-safe decision without native exposure. | [#77](https://github.com/arnabnandikgp/cinder/pull/77), merged `1134f94` |
| [P33 — Unit service, reads and SDK](PLAN.md#p33) | closed | Explicit unit requests and private projections share current authority, one history and bounded replay cost. | [#78](https://github.com/arnabnandikgp/cinder/pull/78), merged `3486b9f` |
| [P34 — Unit execution and reconciliation](PLAN.md#p34) | closed | Unit-owned durable preparation precedes native exposure; qualified observations reconcile only their original component. | [#79](https://github.com/arnabnandikgp/cinder/pull/79), merged `3486b9f` |
| [P35 — Unit funding and recovery](PLAN.md#p35) | closed | Unit custody routes, movements, payouts and recovery share ownership and counters without invented venue credit. | [#81](https://github.com/arnabnandikgp/cinder/pull/81), merged `3486b9f` |
| [P36 — Joined second-venue acceptance](PLAN.md#p36) | in progress | Joined offline fault tests and separately qualified native evidence establish the actual integration boundary. | [#82](https://github.com/arnabnandikgp/cinder/pull/82), merged `3486b9f`; offline complete, native/hardware qualification open |
| [P37 — Bounded BULK native qualification](PLAN.md#p37) | in progress | Current paper trading and test-rail evidence support only the capabilities actually observed, without production activation. | [#83](https://github.com/arnabnandikgp/cinder/pull/83), three review fixes verified; new-head hosted checks separate |
| [P38 — Measured two-unit deployment](PLAN.md#p38) | in progress | Real enclave entrypoint loads separate native roles and one protected unit journal, without fixture authority or legacy fallback. | [#84](https://github.com/arnabnandikgp/cinder/pull/84), two startup review fixes verified; new-head hosted checks separate |
| [P39 — Joined deployed-unit workers/SDK](PLAN.md#p39) | in progress | Qualified native work and confidential SDK preserve exact unit ownership, history and failure semantics. | [#86](https://github.com/arnabnandikgp/cinder/pull/86), restacked; combined offline acceptance passed |
| [P40 — Changed-image experimental parity](PLAN.md#p40) | in progress | Measured Nitro and supported paper evidence establish the final bounded Pacifica/BULK comparison. | [#87](https://github.com/arnabnandikgp/cinder/pull/87) → [#88](https://github.com/arnabnandikgp/cinder/pull/88); corrected Nitro SDK open/fill passes; Cinder close/cash bridge and full admin acceptance remain open |
| [P24 — Release safety case](PLAN.md#p24) | open | Reviewed evidence, calibrated policy and explicit governance support an honest release decision, not automatic deployment. | — |


## Current handoff

Updated 2026-10-23. The user approved documentation consolidation above the
reviewed P37–P40 stack, then merging the implemented work and stopping before
the local precision/funding correction. No financial-policy change, frontend
integration, live request or AWS run is part of this cleanup.

- Review: PR #83's cumulative paper cleanup allowance was reproduced failing,
  corrected to consume all prior cleanup quantities, and verified with nine
  paper-tool tests and 98 repository contract tests. Unknown/zero-fill attempts
  and cancellation do not refund that allowance. The dependent stack is restacked.
- Cleanup verification: 22 relocated/snapshotted documents were compared with
  pre-cleanup `0ae5218`; content differs only by relative-link rebasing and
  explicit phase anchors. All 46 phase IDs/dependencies and 165 acceptance
  checkboxes are preserved. The complete contract suite passes 99 tests, with
  no failures/skips; documentation links and workspace boundaries pass.
- Latest native result: actual Nitro bootstrap, encrypted SDK opening and fill
  attribution passed. Cinder close remains blocked by active-account decimal
  precision/funding handling. Separate authorized teardown finished flat/revoked;
  AWS resources are removed and original private evidence preserved.
- Next after this user-requested cleanup/merge: stop. The subsequent proposed
  work is local sanitized regressions, reported-metric precision treatment and
  funding reconciliation, then joined lifecycle verification. It is not started.
- Deferred: Pacifica missing-credit/live financial qualification, current-image
  full SDK close/cash bridge/admin acceptance, production financial/security
  gates. No new live-run authority follows from a merge.

[Phase handoffs](../archive/implementation/TRACKER-20261023.md) retain original
Work/Verification/Unexpected/Next/PR/Merge fields and complete dated sessions.
New work should update this current handoff and add only its owning phase's
changed handoff here; do not append another full session transcript.
