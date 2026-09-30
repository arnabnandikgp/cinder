# ADR 0013 — Pacifica evidence, not inferred finality

Status: implemented for offline qualification; unmerged P13. 2026-09-30.

`cinder-pacifica` is outside the dependency-free kernel. It decodes bounded native
JSON into the existing journal. Exact decimal grids, quote scale, account, network,
market conversion, source namespace and evidence revision form an immutable
profile commitment. Documented/observed capabilities do not enable financial
normalization; explicit qualification is required. Tests qualify synthetic profiles
only. There is no default production profile or network client.

## Durable boundary

Every accepted response has an encrypted journal evidence attachment containing
its exact UTF-8 body and configured source/request provenance. Normalized effects
commit in that same transaction. Views and cursors rebuild from accepted history;
they are not another balance database. Unknown commit outcomes poison the journal;
an exact retry produces no second effect. Changed profile/history requires an
explicit migration, not a reinterpretation. Journal wire revision is 8 and engine
revision is 17 after the stacked review corrections; old formats reject.

Each normalized input stores a compact `CINDER-PACIFICA-INPUT-1\0` reference:
SHA-256 of its transaction's exact evidence attachment followed by a big-endian
u32 normalized-event ordinal (`u32::MAX` for an unnormalized/gap input). The full
body is stored once, not copied into each of up to 32 inputs. Replay retains the
archive and every reference. The journal enforces its cumulative opaque-byte
budget before append, using the same limit as reopen; exceeding it contains
ingestion and does not discard evidence or grant an exposure.

Trade identity is `(network, deployment, venue, native account, trade namespace,
history_id)`, never order ID, client ID, arrival time or LI. REST/WS share the key;
different reversal legs retain their different history IDs. Economic fields decode
without floats. Known duplicate JSON keys/aliases reject. A durable attempt's
derived native client ID supplies customer attribution; pooled `close_long` does
not identify a customer's close. Missing routing goes to suspense. No correction
overwrites old economics. A newly discovered earlier execution is retained and
contained rather than inserted into today's cost basis. Batch sorting does not
prove global completeness.

Account metrics remain distinct. Position messages replace the whole view,
including explicit flat markets on an empty array; stale LI snapshots cannot
resurrect positions. Rounded entry price is not exact basis. Equal-LI conflicting
snapshots, disconnects, cursor cycles/skips, unsupported forced events, precision
gaps and unqualified funding produce named conditions mirrored into the journal's
unresolved-input gate. There is no generic gap reset. Financial reconciliation is
still the explicit P04 `NativeCheck`/evidence resolution path, never a balance setter.

Pagination exhaustion does not prove an execution set complete. LI is scoped and
cannot become `Input.source_cut` or a terminal-history certificate. Funding history
does not establish a private gross-position funding cut/rate. `settlement` does
not uniquely mean ADL. P12 restoration classification cannot be enabled from these
fields alone. Those capabilities remain disabled pending G01/P23 evidence.

## Limits and dependencies

Responses are limited to 16 KiB and 32 rows; scans to 128 cursors; diagnostic order
and trade identities to 4096. Journal history/frame limits also apply. Exceeding
these limits requires containment and explicit evidence handling, not truncation.
Transport must enforce bounded reads before constructing a message and exclude
credentials/headers. P19 owns authenticated enclave egress, not this codec.

Reviewed new direct dependencies: exact `serde 1.0.229` with derive and
`serde_json 1.0.151`; existing `sha2 0.10.9` binds profile/client identities. The
locked derive/JSON graph adds proc-macro2, quote, syn, unicode-ident, serde_core,
serde_derive, itoa, memchr and zmij. Build scripts and resolved features are pinned
in the workspace guard. No HTTP/TLS client, key store or SDK enters the kernel.

See [native mapping](../venues/pacifica.md), joined adapter tests and TRACKER for
the exact verification result. Offline checks are not live source qualification,
an independent audit, custody assurance or permission to trade.
