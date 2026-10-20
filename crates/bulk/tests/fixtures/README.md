# BULK codec provenance

Native byte layout and order-ID expectations are derived from official client
`56f6676611a0c05dbd187d406dced26168516070`,
`crates/api-rust/src/transaction/fixtures/sdk-signing-vectors.json`, vector
`legacy`: the limit subaction is used without its unrelated market action.
A single-action message has the correspondingly changed vector-length prefix.
The upstream `default_meta_order_ids` use zero account/nonce/index, NOT the
fixture's signable-message account `[1;32]`, nonce 42 or array index 1; the
wire unit test checks the original default-meta hash through the private pure
hash helper without relaxing real Scope/nonce guards. The integration hash is
separately derived for actual account/nonce/index-0 context using Node crypto and
the fixed action bytes. These are **derived sub-vectors**, not a claim of a new
upstream execution. Original vector metadata names SDK
revision `bc0ea23f17b42090d73177beb424ad6516d58386`.

Upstream is Apache-2.0 (Copyright BULK); source/license:
https://github.com/Bulk-trade/bulk-client/tree/56f6676611a0c05dbd187d406dced26168516070
and https://www.apache.org/licenses/LICENSE-2.0.
No upstream implementation or dependency is copied into this crate.

JSON history/stream inputs are Cinder-authored **synthetic** shape fixtures based
on official account/stream documentation captured 2026-10-15. They deliberately
use public fixed byte identities, wide integers and exact tiny signed fees. They
are not native recordings or qualified live financial observations. Adverse
cases mutate these inputs, not ignored user data. Native GET qualification and
credit/return, funding/basis/risk parity remain separate.

P28 financial fixtures are synthetic account/funding/risk/fee-tier shapes from the
same client pin's `msgs/account.rs` and `msgs/history.rs` and the official account
reference (including its schema section), rechecked 2026-10-16. Chosen atom units,
grids, quotes and IDs are test configuration, not current venue precision or a
live account. No schema decoder's default zero or synthesized ID is promoted.
Displayed averages/rates deliberately exercise more precision than resting ticks;
tests do not turn them into exact cost basis or qualified funding allocation.

`paper-20261020-missing-trade-id.json` is a **sanitized observed paper shape**,
not a synthetic successful fill. Its source is the approved October 20 fresh
paper run's original response 13, SHA-256
`8d0a8f9e9ffc7c8c0c4d426e9530019fe766767b9d1be92a773dd2b19a644c7d`.
Maker/taker/order addresses and the counterparty hint are replaced with fixed
test identities; native numeric fields, absent `tradeId` and unknown coverage
are retained. The original bytes remain private ignored evidence. The official
SDK's `HistoryFillWire`/`HistoryFill::deserialize` explicitly uses the required
native `slot`/`sequence` pair when `tradeId` is absent, but rejects explicit null.
Source: `crates/api-rust/src/msgs/history.rs`, revision
`5012611bd0fd36e12523cd15bac763f3a600b1e9`, verified 2026-10-20, original file
SHA-256 `6387851fde317239633a86cdda827219f8cfd28d4d0816ac1a0c760197bd9b55`.
This rule also exists in Cinder's retained source pin. Cinder preserves its
stricter explicit-ID/pair equality, required fields, bounds and scoped dedup keys.
Tests compare both spellings without rewriting the retained response, retain
invalid/null observations without posting, and reject missing/invalid native
fields. No timestamp, order ID or invented sequence substitutes for native identity.
