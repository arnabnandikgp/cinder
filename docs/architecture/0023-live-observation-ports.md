# ADR 0023 — Budgeted private observation ports

Date: 2026-10-06. Scope: [P23](../implementation/PLAN.md#p23).
Status: local implementation; not live activation or a qualification receipt.
[ADR 0013](0013-pacifica-observations.md), [ADR 0014](0014-pacifica-execution.md)
and [baseline](../implementation/BASELINE.md) remain binding.

## Request and evidence boundary

`cinder_pacifica::reads::poll` connects the existing pooled Gateway's durable
credit budget to the existing observation ingestion layer. It neither creates
another ledger nor changes customer economics. The immutable Gateway profile
supplies the account and origin; callers cannot pass another account or URL.
Supported GETs are account, positions, trade history, funding history and order
history. Stream and disconnect kinds cannot be used as REST routes.

Each call handles **one** bounded page. History requests explicitly limit rows
to 32. Cursors are length-bounded, escaped opaque values; subsequent requests
must follow cursor progress already committed in the same journal. Starting a
new scan is permitted, but does not establish historical completeness. There
is no automatic page loop, retry, cursor reset after error, or account override.

Reserve credits durably before returning the non-clone request to private
transport. The transport consumes the permit at its current trusted time before
opening a socket. An expired/early permit or failed delivery does not refund
spent credits. Existing gateway activation, fencing, cleanup reservation and
post-exposure cooldown rules apply. Response and reservation IDs must differ;
reuse of either cannot create a second request/credit debit.

This port sends no API config key. Therefore its configured worst-case read
charge must be at least **120 tenths of a credit**, the upper end of Pacifica's
published unidentified-IP heavy-GET range. This conservative local floor is not
a guarantee of a future server quota or endpoint classification. A different
credential tier/cost profile requires a reviewed change and fresh qualification.
HTTP 429 persists a pool-wide cooldown before any further scheduling; the
existing minimum of one full venue window remains unchanged after restart.

## TLS and normalization

`cinder_service::egress::Egress` implements the read transport using the same
explicit trust root, hostname, trusted clock, TLS 1.3 and fixed parent-vsock
route as signed POSTs. Account/cursor paths exist only inside enclave TLS;
the parent forwards encrypted bytes. No caller-supplied URL, redirect, system
CA fallback, signed request logging or network retry is introduced. The narrow
existing response framing is retained: bounded JSON with Content-Length,
identity encoding and no chunked framing. Unsupported framing remains unknown,
not permission to loosen authentication during a live test.

A successful body is archived with trusted route/account/receive metadata and
passed to **ordinary** observation ingestion. Malformed supported JSON becomes
retained unnormalized evidence. Invalid UTF-8, oversized bodies or regressing
receive time fail without credit refund. Account equity is diagnostic, not
another cash asset. Funding observations still identify their qualification gap.

The port **never** constructs `Coverage`, calls `ingest_covered`, marks an order
terminal or authorizes a payout. TLS authenticates response bytes; cursor
exhaustion, LI, a cancellation status and elapsed time are not complete-fill
certificates. Qualifying a live terminal/funding/payment provider remains P23.

## Activation and validation

The shipping `boot::Manifest` still rejects trading, funding and native-read
activation. No scheduler, CLI or public endpoint enables the new port. This is
prepared composition, not a runnable live integration. The remaining wiring and
fresh authority requirements are listed in the
[live qualification runbook](../operations/live-qualification.md).

Offline regressions cover exact account routing, escaped/committed pagination,
invalid requests before exposure, missing/revoked authority, duplicate IDs,
expiry, unknown delivery, durable 429 containment, restart and protected cleanup
capacity, malformed evidence and unchanged entitlement/backing semantics.
Loopback TLS checks exercise both GET and POST with correct/wrong root,
hostname and certificate time. These are local tests, not fresh Nitro evidence.

Primary sources rechecked 2026-10-06:
[trade history](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-trade-history),
[funding history](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-funding-history),
[positions](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-positions),
[account](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-info),
[order history](https://docs.pacifica.fi/api-documentation/api/rest-api/orders/get-order-history),
[rate limits](https://docs.pacifica.fi/api-documentation/api/rate-limits),
[LI](https://docs.pacifica.fi/api-documentation/api/last-id).
