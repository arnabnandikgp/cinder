# ADR 0028 — Measured one-shot native capture worker

Status: implemented local runtime continuation of [ADR 0027](0027-native-transfer-capture.md).
Changed-image/native qualification remains open. This is neither financial
activation nor a new continuous-feed or custody policy.

## Loaded contract

Manifest version 4 requires the existing six-role chain/retained-pack profile,
`native_reads=true`, and an explicit `native_capture` policy. That policy binds a
distinct parent-CID-3 port, canonical DER root and its approved digest, and the
same finite capture limits as ADR 0027. It rejects every other manifest-route
collision, duration beyond the boot lease, and message limits whose conservative
JSON/frame overhead exceeds the declared record ceiling. The backend independently
enforces actual remaining history and PUT budgets; these limits do not reserve
unlimited storage or promise completion.

`Configuration::construct_for` validates the actual Gateway's canonical pooled
account and heavy-read cost, consumes the actual origin/root/route/limits into
`native_capture::Loaded`, and derives the application component from them. The
existing Gateway component already binds the private account/profile. A supplied
digest cannot replace those loaded objects. `Loaded::open` rejects a changed
capture policy before cloud restore or capture I/O.

Version 4 uses `CINDER-RUNTIME-MANIFEST-4` and `CKR4`; it does not change key
purposes or add a master key. Legacy versions 1/2/3 omit the new field, reject a
capture policy, and retain their original manifest/envelope encodings. Both
`funding` and `trading` are still rejected for every version; API risk admission
remains disabled. No parent or client endpoint can choose a capture host/account.

## Worker and journal ownership

The enclave entrypoint launches one joinable capture worker independently of the
ordinary supervisor and private ingress. `Runtime::capture_once` takes the loaded
port exactly once before preparation. Concurrent/repeated calls, connection
failure and quiet/resource endings cannot reconnect during that boot. A fresh
boot may make one **new** independently charged capture; it cannot resume a
socket, replay an old event as fresh, or resend a financial operation. Live run
authorization must cap boots/connections separately.

The worker verifies lease/fence/read budget and durably reserves/prepares under
the authoritative writer guard, then drops it for all socket I/O. Each record
reacquires the guard, checks the current signed clock and witness, refuses a
future observation, and appends through the original Archive against the current
head. Neither the writer nor general supervisor I/O guard spans the capture.
Concurrent account commands therefore do not discard valid raw observations.

Exhausted shared credits make no capture request and cannot cause a later retry
in the same boot. An insufficient remaining lease refuses preparation. An
uncertain preparation/archive commit, writer epoch change, invalid clock or
worker panic stickily fences the boot; the existing read publication gate closes.
No replacement journal identity or weak entropy fallback is installed. Socket
shutdown/join remains bounded by ADR 0027's finite I/O/duration limits, not
instant cancellation of a blocked peer.

All archived inputs are still raw, without economic events, source cuts or
authority epochs. A retained `Opened` is not subscription readiness; an ended
capture is not a complete history. There is no cash posting, setup certificate,
native credit, final debit, payment, or hold release in this worker.

Parent-PR review corrections bound the vsock connect to the remaining capture
duration and retain already received bodies when the post-read clock/deadline
fails. Those bodies are explicitly time-unverified and terminal for the capture;
they are never fresh financial observations. Current journal clock/witness checks
remain mandatory, so unavailable storage authority still refuses and fences.

## Local evidence and exact next boundary

Tests reuse the actual runtime/one-journal/AEAD ports with explicit synthetic
transport callbacks. They prove within those fixtures that a command/read can
complete during capture I/O, a valid result rejoins the advanced head, and
duplicate worker calls, exhausted budgets, short leases, future timestamps,
epoch changes and transport panic cannot expose another request or financial
posting. Separate ADR 0027 tests own real local TLS/RFC6455 behavior.

Actual-policy tests change origin, route, root and each resource bound and require
different loaded components. Manifest tests reject unsafe combinations and legacy
schema contamination. The provisioning process exercises all four versions,
purpose-separated envelopes and private file permissions. These are local checks,
not Nitro, Pacifica, an independent audit or production capacity evidence.

Next: qualify the authenticated setup/credit/payment provider semantics in
[P23-NATIVE-EVIDENCE](../implementation/P23-NATIVE-EVIDENCE.md), including current
fresh-account bootstrap and lost-native-ACK handling. Public schemas do not by
themselves establish atomic setup, operation-specific final credit or no later
withdrawal effects. Retain those as explicit gaps rather than fabricate a cut.
Any native/AWS/devnet activity requires a fresh bounded G05 manifest; the closed
version-3 storage receipt does not qualify this new image or dependency graph.
