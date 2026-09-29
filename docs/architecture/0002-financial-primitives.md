# ADR 0002: exact financial primitives and canonical identity bytes

Date: 2026-09-30. Status: implemented in P02, pending review.
Scope: [P02](../implementation/PLAN.md#p02), [F02/X01/X02 baseline](../implementation/BASELINE.md).
No new economic terms, venue precision assumptions or custody authority.

## Units and representation

| Type | Representation | Meaning / required context |
| --- | --- | --- |
| `QuoteAtoms` | signed i128 | Cash/accrual dimension; asset ID and precision revision. Not automatically physical collateral |
| `BasisAtoms` | signed i128 | Remaining entry basis; market/revision and quote asset/revision. Short basis stays signed |
| `QuantityLots` | signed i64 | Whole normalized lots, scoped to the same complete market unit; zero can mean flat |
| `PriceTicks` | positive u64 | Whole ticks, not quote atoms per lot by default |
| Internal IDs | distinct nonzero 32-byte types | Network, deployment, private owner, physical venue account, asset, market, request and attempt are not interchangeable |
| `EconomicEventId` | 1–128 exact bytes | Qualified native execution identity, with a separate participant leg; never truncate or substitute an order ID |

These widths are implementation bounds, **not trading limits or native precision**.
Arithmetic rejects rather than wraps/saturates. Amount addition/subtraction checks
asset/market/quote identity and precision revision. Compile-fail tests distinguish
cash from basis and request from attempt. Zero/signed quantity remains useful for
positions; P07 must reject invalid order intent sizes separately.

`PrecisionVersion` and `PolicyVersion` are nonzero references, not self-authorizing
registries. Require the configured revision. `decode_in` checks amounts against
trusted units; structural `decode` alone is not an admission check. Likewise,
`DecimalScale`/`DecimalGrid` must come from qualified versioned metadata, never
attacker-selected scale fields. P13 supplies native bindings. Tick-to-quote basis
conversion and the joined fill transition belong to P03 with explicit metadata;
there is deliberately no automatic `lots * ticks == quote atoms` API here.

Decimal text is decoded directly into integers: optional minus, ordinary ASCII
digits, optional nonempty fraction, explicit scale 0–38. Reject exponent/plus/
whitespace/leading zeros/negative zero/Unicode digits, overflow and overprecision
(even extra trailing zeros). In-scale trailing zeros normalize to the same atoms.
Off-grid values reject, not round. Inputs are bounded to 80 bytes. Native adapters
must preserve raw observations, explicitly normalize permitted native syntax, and
quarantine unsupported values; rejection must not erase evidence of a real fill.

## Rounding and range argument

`mul_div(v,m,d,mode)` uses signed i128 `v` and u64 `m,d`. For magnitude `a=|v|`,
split `a=q*d+r`, then compute `q*m + floor(r*m/d)` with checked u128 arithmetic.
`r<d<=2^64-1`, so `r*m` fits u128. If the nonnegative `q*m` overflows, adding the
remaining nonnegative term cannot bring the result into i128 range. Restore sign
after rounding; magnitude `2^127` is permitted only for a negative result.

- `Exact`: reject a nonzero remainder before checking quotient range.
- `TowardZero`: truncate magnitude.
- `Floor`: add one to magnitude only for a negative fractional result.
- `Ceil`: add one only for a positive fractional result.

These are tools, not a universal instruction to round against customers. The
approved basis rule is specifically toward zero. `BasisAtoms::split(part,total)`
requires `0<=part<=total`, returns allocated plus remaining basis, and retains all
residue in the remainder. Full allocation returns the complete original basis.
This hand argument and bounded differential tests are not a machine proof.

## Canonical wire contract, schema 1

All frames begin with seven bytes `CINDER\0`, u16 big-endian schema **1**, then one
kind byte. Unknown schema/kind, invalid IDs, truncated or trailing bytes reject.
Every integer is fixed-width big-endian; signed integers use two's complement.
No native Rust memory layout, JSON/display fingerprint or float is persisted.

| Kind | Body in order |
| --- | --- |
| 1: quote amount | asset ID (32), asset precision (u32), signed atoms (i128) |
| 2: quantity | market unit, signed lots (i64) |
| 3: price | market unit, positive ticks (u64) |
| 4: basis | market unit, signed atoms (i128) |
| 16: request | network (32), deployment (32), private account (32), request ID (32) |
| 17: attempt | request body, attempt ID (32) |
| 18: economic event | network (32), deployment (32), venue (32), physical account (32), semantic namespace (32), native ID length (u16), exact ID bytes, participant leg (u32) |
| 32: payload | policy revision (u32), body length (u32), 1–65,536 semantic bytes |

Market unit = market ID (32), market precision (u32), quote asset ID (32), quote
precision (u32). Payload framing binds policy but does not validate concrete event
semantics; P03/P05 define those bodies, the full observation/authority envelope and
atomic journal. An incompatible layout requires a new schema, never reinterpretation.

Native ID namespaces must be qualified: equivalent REST/WS deliveries share a
semantic key, while unrelated histories do not. If IDs are unique only per market,
that market's namespace must be distinct. Namespace identity does not prove native
uniqueness/completeness; this remains P13 qualification. Event identity excludes
delivery timestamps, observation transport and policy revision: a changed payload
or policy under the same key is a conflict, not a convenient new event.

`require_domain` and `require_scope` reject wrong routing contexts. Comparing keys
is not signature verification. `classify_replay` compares typed keys and exact
canonical payload bytes: distinct / duplicate / conflict, without mutation.
It is not a consumed-event store or an exactly-once dispatch guarantee. P05 commits
consumption and postings atomically; no hashing/signing library is selected here.

## Evidence and promotion

All source is Cinder implementation, not copied third-party SDK/contract code.
Sanitized local requirements were promoted on 2026-09-30:

| W01/W03 source under local `work/` | SHA-256 | Promoted behavior |
| --- | --- | --- |
| `specs/financial-model/IMPLEMENTATION_HANDOFF.md` | `5a36ccf84c6cf8c2a15f6e5de5ad716d74febc36e3f23074e0d0e18c99542b93` | Typed operation/attempt/economic identity; exact units and canonical versioned bytes |
| `specs/financial-model/model.cjs` | `853d4955f0a3b0c13125558105b37566bce374e764e69782d215938aee0ef74f` | Signed basis with toward-zero allocation; preserve residue |
| `specs/financial-model/checks.cjs` | `0c9a0e36e48fee5e90ac3e3efb3865f184b850b6735fdc2246d2c580a8a091e3` | Partial basis/reversal vector; initial deposit offset removed from V02 cash |
| `venues/pacifica/financial-mapping.md` | `ec906a9c6e70f5c626dfe31b5560ab144f2260f8a2efb8bf241c97136361b341` | Lossless decimals; physical scope, semantic namespaces and distinct execution legs; no native precision claims |

Tracked fixtures: `crates/kernel/tests/fixtures/basis-v1.tsv` (V02 and mirrored/
opening/full-close cases) and `mul-div-v1.tsv` (64 independent BigInt vectors).
The latter is reproduced exactly by `scripts/generate-p02-math-vectors.mjs`; CI
checks it for drift. Rust tests include 69,632 small integer triples across four
rounding modes, full-width boundaries, 10,000 seeded byte mutations, golden layouts,
every frame truncation, unknown versions, scope separation and payload conflicts.

V02 currently runs a **test-only fill driver** around production primitives, not
the P03 ledger. No claim of integrated conservation, solvency, durable exactly-once
execution, cryptographic assurance or qualified venue behavior follows from this PR.

The kernel remains `no_std` with no third-party dependency; bounded codecs use
`alloc`. The workspace guard now checks every Rust source module for obvious `std`
escapes. This is still a review guardrail, not a sandbox. `--properties` selects
all workspace tests prefixed `property_`; the full runner already includes them.
