# ADR 0014 — Durable scoped signing and shared API credits

Status: implemented for offline qualification; unmerged P14. 2026-09-30.

## Authority and exposure

`Gateway` accepts only a prepared P07 order/cancel backed by the same authoritative
journal. It derives all native fields from that immutable intent, exact profile
grids and domain-separated attempt client ID. A persisted signing plan binds the
profile/policy commitment, native account, origin, agent/key epoch, private grant
epoch, original/dispatch attempt, abstract action and exact canonical preimage.
The plan and API debit commit with `Control::Expose` before signing. No caller can
replace a prepared payload or invoke an arbitrary-message signer through Gateway.

Exposure must return the one-shot journal delivery with matching abstract bytes.
The gateway rechecks current writer/key authority before signing. Lost/uncertain
commits yield no signature delivery. An already exposed attempt cannot be re-signed
with a new transaction ID after restart. Epoch activation/deactivation are explicit
trusted administrator/key-release ports, not public customer operations. They
replay durably; key rotation cannot reset credit usage. Full private API verification
and actual key-release governance remain P18/P20. The gateway does not authenticate
a customer merely by accepting a Rust `Approval` produced by another trusted port.

Real Ed25519 signatures and base58 encoding are implemented using pinned libraries.
Key import takes an explicit zeroizing seed from the trusted runtime, with no file,
wallet, environment or remote key loader. The narrow operation set is create-limit
with GTC/ALO/IOC and scoped cancel-by-client-ID. Private reduce-only remains a Cinder
rule, not native pool reduce-only. No modify, conditional order, unrestricted market
order, cancel-all, transfer, withdrawal, registration or leverage-setting signer is
exported. Emergency/restoration dispatch remains disabled until its separate native
event/cut requirements qualify; P11/P12's abstract controllers remain testable.

Pacifica's native limit is **one-sided**: maximum buy price or minimum sell price.
It does not enforce the intent's opposite/favorable-price boundary or a fee ceiling.
P07 still contains an actual out-of-collar/fee-bound execution and records it;
P14 does not promise the venue enforces both sides or silently relax authorization.
Native price-improvement/risk-collar qualification remains explicit before live use.

## Native signature limits

The native signed object does not include Cinder's full domain, account or key epoch.
The local durable envelope binds them, and the egress origin is fixed with no
redirects. A native signature alone cannot prove these stronger local guarantees.
Each native agent key must be exclusively registered to its bound pool/environment;
independent generation and attested release must enforce that operational premise.
An escaped signature/key cannot be retroactively revoked by a local epoch change.

Historical Pacifica testnet agents could initiate owner-directed withdrawals and
parent/child movement. The native key is therefore **not trading-only**. This
gateway's operation allowlist is a Cinder-enforced boundary, not narrower native
credential powers. Onward owner-wallet/vault custody remains P15/P16; recovery is
operator-assisted. No unrestricted owner/master credential is loaded as a fallback.

## Capacity and outcomes

One immutable execution contract shares integer tenths-of-credit across new orders,
cancels and reads in the pool journal. New risk/ordinary reads cannot consume the
cleanup reserve. Worst-case read cost and total quota require explicit qualification;
they are not copied from an unrelated account tier. Credit reservations remain for
the 60-second native window **plus the configured maximum delivery lifetime**, so
a delayed valid send cannot spend credits already released locally. Read permits
are non-clone, one-use and expire. Trusted egress sends immediately after consuming
them. All other traffic sharing the native account/IP must use the same scheduler;
future multi-journal deployments need a shared coordinator, not one allowance each.

Requests are sent once. Timeout, malformed/ambiguous response, 400/401/429/500 and
lost reply are unknown, never permission to resend. A 429 persists a bounded
cooldown across restart; it can temporarily block cleanup too. The reserve is a
local capacity guarantee, not a promise the venue remains available. Even a failed
local expose conservatively retains its recorded credit debit until expiry.
ACK/cancel ACK changes no cash/position and releases no hold. P13's unresolved
completeness gate is not bypassed by a closed status. Response provenance, exact
body and normalized status are persisted separately from the exposure transaction.

The synchronous trusted transport seam is exercised by fake servers only. Its
contract forbids retries, redirects, mutation and plaintext host logging; actual
enclave TLS, fresh clocks and hostile-host qualification belong to P19/P20. No
production-qualified profile, live call, wallet, AWS resource or deployment was used.

## Verification and dependencies

Tests cover all three TIFs/both sides, official canonical bytes, the public RFC8032
key vector and a request signature independently generated with Node/OpenSSL;
scoped cancellation and a subsequent fill; unknown ACK/429/error/restart behavior;
shared reads/cleanup credits and delayed-delivery expiry; policy/key/grant revocation;
crash before/after exposure commit, stale writer before signing and lost reply commit.
The full kernel/journal suite still owns financial invariants—no adapter balance book.

Exact new pins: `ed25519-dalek 2.2.0` with std/fast/zeroize, `bs58 0.5.1`, and
existing `zeroize 1.8.2`. Version 2.2.0 is deliberately selected, not misidentified
as the newest major. Lock/checksum/feature/build-script policy covers the additional
signature/curve/derive graph. The guard now distinguishes multiple package versions
(serde derive and curve derive use different syn majors). No custom cryptography,
network client, PKCS8 loader or optional random-key generator is introduced.

See [native mapping](../venues/pacifica.md), PLAN and TRACKER for current acceptance,
evidence and open production gates. Stop before P15 until the user resumes it.
