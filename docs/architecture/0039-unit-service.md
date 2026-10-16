# ADR 0039 — Shared-unit private service and derived reads

Date: 2026-10-16. P33 follows ADRs 0034–0038; native execution, custody and
deployment qualification are the following stacked phases, not inferred here.

## One controller and one publication

`api::units::service::Service` joins the existing unit authorizer and financial
controller. `service::units::Runtime` implements the existing confidential
transport Handler and can be passed to the same TLS/Noise servers. It owns one
unit journal writer, not one runtime/database per venue. Shipping v1 manifests
and bootstrap remain unchanged; there is no default-unit fallback or automatic
upgrade of active histories. Unit onboarding/custody is a P35 dependency.

The existing journal Reader/Ticket/Verified mechanism now also publishes an
immutable shared unit state. Every read obtains its own fresh witness response
outside writer/cache/publication locks. Unit selection produces a sealed Source
with only that component's financial provenance and derived customer book
changes. The legacy Source interface refuses unit state, rather than silently
projecting the first unit. Shared storage failures, unknown commits, clock errors,
panics and boot fencing invalidate all units. Known local generation races skip
periodic output, not kill the writer. Four read slots are shared across units.

Publication after a commit updates the common generation; revocation in either
unit invalidates a prepared read in both. Final delivery rechecks the request's
session, owner/global epoch, grant expiry, conservative qualified time and owning
Reader before consuming the same generation/stop gate immediately before cipher
delivery. There is no I/O or encoding under the short publication latch.

## Derived history and lifecycle

The cache is keyed by the exact opaque head, immutable registry commitment and
authorization lifetimes. Its indexed rows/grants contain no independent money or
mutable authority. Every writer cache lookup checks current storage, even for
duplicates. Storage I/O occurs before acquiring the shared cache latch. A changed
head rebuilds and verifies all original signatures and financial joins; a same-
head request reuses the immutable interpretation but authenticates anew. Cold
and repeated authenticated MAX_ITEMS measurements are retained in TRACKER.
Incremental validation is deliberately not introduced without a measured need.
The MAX_ITEMS fixture also measures 64 freshly signed/authenticated requests,
not just cache pointer lookup. Cache timings exclude native/network/hardware
latency. Plain SQLite is a local storage fixture, not a fresh-head witness.

Operation receipts reuse actual engine lifecycle. Financial recording without a
decision is `recorded` (outcome byte 7), not rejected/accepted/exposed. V2's inner
receipt/row body permits that extension; the v1 SDK decoder rejects it. Receipts
use the original v2 economic digest. Unit read revisions/cursors have a separate
hash domain and UnitId, even when both units contain identical rows. All eleven
existing projection families retain their actual evidence limits.

`UnitClient` explicitly selects v2 and binds the unit in signed bytes, economic
digests, command replies and subscription pages. It shares v1's no-retry signing,
correlation and buffer-erasure logic. `PrivateClient` stays v1. Subscription
framing accepts only a strictly decoded cursor-zero Read, never a mutation; the
server handler still performs signature/authority checks and final release.

## Qualification boundary

Synthetic controller/concurrency tests and existing carrier tests establish
offline behavior, not an attested changed-image deployment, venue integration or
production latency. No new financial policy, custody promise, native signer,
wallet, live action, API URL, external dependency or frontend is introduced.
P34 adds unit execution/reconciliation; P35 custody/recovery; P36 joined acceptance
and separately scoped native/hardware evidence. Unsupported rails stay disabled.
