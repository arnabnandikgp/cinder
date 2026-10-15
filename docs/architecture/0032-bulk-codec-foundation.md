# ADR 0032 — BULK exact codec foundation, no native activation

Date: 2026-10-15. Implements the first offline slice of the approved second-venue
direction, [P27](../implementation/PLAN.md#p27). [Candidate evidence](../venues/bulk.md)
and [ownership direction](../implementation/SECOND-VENUE.md) remain authoritative.

## Boundary

`cinder-bulk` is a small codec crate, not a second financial service. It depends
on existing pinned kernel/serde/JSON/hash/base58 packages, not the journal,
private API, service, HTTP client or venue SDK. Workspace guards prevent an
accidental shipping service dependency or new signing/journal edges. No socket,
secret key, nonce allocator, funding controller, readiness certificate, customer
balance or ledger mutation is implemented. The financial core is unchanged.

Enable only `serde_json`'s existing `raw_value` feature to retain original numeric
lexemes. No new registry package/version is selected. A new local lockfile entry
and explicit dependency-policy feature/digest update are reviewed together.
Do not parse financial JSON via `Value::as_f64` or JS Number. Existing single-venue
Runtime and immutable P25 discovery are not replaced by this library.

## Exact primitives and unsigned actions

Normalize bounded decimal/exponent lexemes with integer coefficients and checked
powers of ten. Reject inexact atoms, invalid grammar, negative zero and overflow;
extra trailing zero precision can normalize exactly. Source exponent forms such
as `1e-8` do not justify adding a floating path to the kernel. Explicit market
steps and quote scale are inputs, not live-qualified defaults or native basis.

Support single GTC/IOC/ALO limit actions and cancellation by exact native order ID.
Every immediate execution has a price collar. No market, conditional, builder,
isolated, modify or batch schema is exposed. Native `reduce_only` is false: private
customer reduction must be admitted/attributed by a future joined controller,
not blindly translated into a native pooled reduction. Cancellation's JSON tag
is `cx`, binary action ordinal 3. Limit ordinal is 1; legacy actions have no
trailing builder Option byte when no builder is present.

Encoded price/size are exact 1e8-scaled integers and quoted JSON decimals. A
technical bound of 2^48 scaled units avoids the ambiguous upper range of the
native binary64-rounding convention; it is not a Cinder exposure/leverage policy.
Under normal correctly-rounded binary64 parsing/multiplication, two relative
rounding errors bound atom error by `n*(2u+u*u) < 0.5`, with `u=2^-53` and
`n <= 2^48`. A separate test-only binary64 oracle exercises a fixed 20,000-value
sample. This is not deployed-encoder qualification or a general financial proof.

Network domain/account/nonce enter native messages. Native order IDs deliberately
do not contain the network or Cinder source. Retain the full separate source
commitment, exact action/message and original nonce/attempt before any eventual
signature exposure; this crate neither persists them nor grants that exposure.
Public hashing fixes the same sole action index 0 as encoding. Only the private
pure helper is used to reproduce upstream's zero-default metadata hash vector.

## Observations without financial authority

REST fills and account WS fills use distinct direction rules; maker WS direction
is inverted, REST queried-account direction is not. Require original trade ID,
scope/account, exact order attribution, scalars, effective fee and supported native
reason. REST IDs must agree with slot/sequence; do not copy the upstream decoder's
missing-ID fallback. Preserve u64 IDs and nanosecond timestamps losslessly.

Execution keys include the Cinder deployment/source, native account and network.
Every fill also retains the exact instrument/grids/quote precision used for
normalization; a changed mapping is a conflicting interpretation of the same
execution key, not a new event or an equal fact with a hidden unit change.
Identical page duplicates collapse; conflicting economics, inconsistent reason
labels, malformed/unsorted ranges, wrong account/market, off-grid values, oversized
bodies/pages and explicit isolated-lane rows reject. Unsupported raw facts must
be retained and contained by the future controller, not silently dropped or
interpreted as no event. These codecs are independent of admission/risk state.

Page coverage/retention/cursor consistency is structural validation only. Empty,
terminal or reported-complete pages do not settle an order, establish causality,
release collateral or prove original deposit credit. Matched REST/WS normalized
facts are not an independently authenticated observation or qualified cash cut.
Current WS examples omit isolation metadata; a future activated profile must
establish direct-account scope and that no unidentifiable implicit lane is mixed in.

## Evidence and remaining work

Tests include a derived sub-vector from pinned Apache-2.0 client `56f66766…`,
source/revision notes in `crates/bulk/tests/fixtures/README.md`, synthetic native
REST/WS shapes, large identities, tiny signed fees, exact scientific notation,
duplicates/conflicts, wrong sources, adverse reasons and work bounds. No upstream
SDK is executed or imported. Existing workspace tests must remain green.

The approved three-GET public-testnet probe returned Cloudflare HTTP 502 origin
errors on every route. No current market grid, fee state or risk config was
qualified. No signed action, faucet, wallet, RPC, transfer or cloud resource was
used. Preserve raw private receipts and label this availability result narrowly.

Next: current endpoint availability and native authority/history/environment
qualification; exact funding, fee/basis/settlement/risk mappings; reviewed unit-state
composition and lifecycle joins. No native funding/trading activation, multi-venue
margin, frontend work or reopened Pacifica credit investigation follows from P27.
