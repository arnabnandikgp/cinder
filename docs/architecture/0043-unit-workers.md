# ADR 0043 — Bounded shared-unit native supervisor

Status: implemented for explicit version-8 preparation and offline qualification;
changed-image hardware and native financial activation are separate gates.
Builds on [0042](0042-unit-deployment.md), [0040](0040-unit-native-execution.md)
and [0041](0041-unit-custody-recovery.md). No new economic contract.

## Measured selection and authority

Version 7 remains inactive and retains its omitted-field encoding. Version 8
requires `units.worker`: a 1–60 second poll interval, at most 32 reads per unit
per boot, and at most 16 escaped order/cancel requests per unit over the accepted
history. Public trading/read gates must exactly match these allocations. Funding,
chain/capture/demo policies and broker/funds key roles remain unavailable.
The seven existing unit roles and their purpose codes do not change.

Each private unit config declares its interpretation explicitly: Pacifica `fills`
and execution qualification, BULK `qualification`. Dispatch allocation requires
the corresponding qualified controller. A zero allocation never derives a
qualification, and a qualification never overrides an allocation. Read-only
Unknown controllers cannot sign or normalize financial facts. The actual loaded
controllers, profiles, private config hashes, ports/trust, history policy and
worker allocation bind the application commitment; the image binds the algorithm.

Worker releases require exactly one Vault, Broker and Venue source per unit,
with the native adapter selecting Venue. Omitting Broker must not accidentally
select the older no-native-funding model's ready initial state. Managed customers
start with empty books and native funding **not ready**. This release supplies
no balance, credit, mark, risk policy, insurance capital or complete financial cut.
Enabling a scheduler is insufficient to admit a trade.

## One authoritative writer, outside-lock I/O

The supervisor derives work only from the accepted component state and retained
native cursors. It has no customer command queue, arbitrary native request API,
readiness setter or independent database. Preparation under the shared writer
checks freshness, current owner epoch/deadline, holds, actual native policy and
capacity. Advisory sealed-controller previews avoid writing refused proposals;
they neither commit nor deliver exposure. The actual commit revalidates all
checks and persists the original attempt/plan/exposure before signing.

At most one native request escapes per supervisor cut. Venue and read/execution
selection alternate; candidate rotation prevents one blocked intent from
permanently starving later admitted work. This is a bounded qualification
scheduler, not a latency, fairness or market-priority SLA. BULK nonces use checked
trusted milliseconds × 1,000,000, above the durable nonce floor. They are
persisted by the existing gateway, not allocated from parent/client time.

The financial mutex is released before the actual private TLS port call. The
non-clone completion rejoins the same writer and original unit. A busy writer
before preparation causes no side effect. A completion waits for ownership and
cannot be dropped merely because another API request advanced the head.
Native timeout/error/429/ambiguous results retain their existing uncertainty and
cooldown semantics; escaped originals are never automatically resent. Read
budgets charge before I/O; dispatch ceilings count original accepted exposures
across restart and do not reset on key rotation.

Pacifica polls account/positions/trades/orders/funding with retained cursors.
BULK refreshes only bounded account/trade first pages (128 rows). Neither scheme
claims a terminal history, complete funding/ADL coverage or global financial
frontier. Unavailable/malformed facts remain contained; no successful HTTP status
turns native cash into attributable customer credit. All clocks, release leases,
independent freshness and sticky fences still apply during egress/completion.

## Executed acceptance and limits

`tools/web-channel/check-runtime.mjs --units` runs the same UnitClient scenario
in actual Node and Chrome. It traverses the real opaque HTTP/WebSocket relay,
Noise channel, signed v2 API, shared runtime/journal and native worker. It checks
unit grants/original-operation scope, outside-lock slow I/O, once-only native
exposure, duplicate/partial fill attribution, global revocation, process restart,
history-wide dispatch ceilings and shared fencing.

The `--unit-web` driver is **local-fixture-only**. Native replies, attestation
root, same-host witness, capital/marks and native credit are deliberately
synthetic. Its private stdin controls are not public API endpoints. Its fixed
disposable agent keys and financial initialization must never enter a deployment.
Deployment tests separately exercise actual v8 construction, managed empty
startup and refusal of incomplete/ambiguous qualification and source inventory.

This is not live SDK-driven paper trading, changed-image Nitro evidence, a
qualified BULK funds rail or a production readiness claim. Those require their
own evidence; old direct native runs and old hardware images cannot fill the gap.
