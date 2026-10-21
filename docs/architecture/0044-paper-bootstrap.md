# ADR 0044 — Quarantined fresh-account paper bootstrap

Status: user-approved test boundary; implemented for offline qualification.
Native execution and changed-image Nitro evidence require a separate run.
Owns [P40's paper gate](../implementation/P40-PAPER-GATE.md), builds on
[0042](0042-unit-deployment.md) and [0043](0043-unit-workers.md).
No change to customer custody, RF1/RF2, production credit or recovery backing.

## Explicit purpose and trust

Manifest **9**, not an implicit version-8 flag, selects one private `paper`
configuration. Purpose is exactly `closed-fresh-paper-1`; omission is preserved
in version-7/8 CBOR. The actual configuration, original setup/quote commitments,
native profile, owner, account, quote identity/precision, finite policy and
gateway/egress bind the measured application. All seven existing KMS roles remain
separate. There is no broker/master/funds key, Solana route, public balance import
or readiness endpoint. Pacifica stays Unknown/read-only and starts empty.

This is an **assumption-backed experiment**, not stronger native evidence: a
fresh, exclusively controlled BULK paper account, no prior activity, original
faucet/setup records retained on the trusted operator side and authenticated
current account/history responses establish its initial bounded test cut.
The prepared quote is also a trusted, retained, time-bound test input, not a new
production oracle. Hashes bind these originals; a hash cannot prove freshness,
exclusive control or authenticity by itself. Trusted preparation must verify them.

The actual previously retained fullAccount schema omits optional child/lock/
on-fill inventories, and fills report unknown coverage. Those remain unknown in
the ordinary codecs. For this closed account only, the explicit approved
assumption supplies the missing absence/completeness knowledge. Partial pages,
continuations or a known positive retention floor still refuse initialization.
This cannot be generalized to active accounts, external funding, ADL or recovery.

## Observe before accounting

After managed empty enrollment and local native-agent epoch installation, but
before the listener/financial worker starts, reserve four original queries:
fullAccount, fills, fundingHistory and riskHistory. Reservations and response IDs
derive from the exact private policy. Every request consumes durable native
credit before TLS I/O; original sanitized responses and HTTP/body linkage remain
in encrypted history. No accepted reservation without its response is resent.
An already retained response must be the correct profile/family and successful
body commitment, at most 60 seconds old on first initialization.

Require the exact registered enclave agent as the sole reported agent, positive
cash sufficient for customer allocation plus house buffer, equal available/total
margin, explicit zero position use/notional/realized PnL/unrealized PnL/fees/funding,
no positions/resting orders and empty bounded histories. Any reported child,
lock, on-fill plan, unmapped/isolated position or history row refuses. Unexpected
or malformed facts are retained, never turned into a zero account.

For actual observed paper cash `F` and explicitly configured customer allocation
`C <= 20 USD`, one atomic transaction records:

```text
native cash = F
customer cash = C
house cash = F - C
vault cash = broker cash = 0
customer cash + house cash = native cash
```

Both receipt legs use one original policy/evidence identity with different legs.
This is not a customer deposit and never produces `F + C` backing. The exact
closed-account reconciliation assumption and known-empty raw history findings
are retained with that transaction. `source_cut` remains None; a native global
frontier is not manufactured from an as-of slot or pagination flag.

The same transaction installs collateral/risk and native-readiness controls via
existing journal semantics. Advisory preview must accept every input/control;
the actual common-head CAS revalidates. Unknown commits poison the writer and
require protected reopen. The original bootstrap is then recognized without
re-querying or crediting again. A changed policy cannot reallocate an active book.

## Quarantine and finite risk

Accepted history retains its paper purpose, not just a runtime Boolean. Startup
checks every unit before enrollment/publication: an ordinary version-7/8 release
or a different paper policy cannot reopen a history carrying this bootstrap.
Removing a capsule field or changing an approved manifest does not promote paper
cash or its completeness assumption into production backing. Legacy journal
formats are not migrated.

The one-market test policy uses full-notional private/native initial margin, 1x
maximum leverage, 50% maintenance, explicit lower/upper price paths, a house buffer
at least the configured at-most-$20 notional cap and location-specific liquidity.
Only observed Venue cash is accessible; Vault/Broker have zero cash. These are
conservative **test settings**, not calibrated capital, exact native margin or
an all-market solvency guarantee. Actual qualified grids/conversions are required.

The fixed paper lifetime is at most 30 minutes. The actual leased clock is capped
at the earlier of boot expiry and paper expiry, covering startup, TLS, reads,
signing, completions and private responses. Restart cannot renew it.

## Admission, fills and cleanup

Real signed UnitClient requests pass the existing authorization, journal and
joined risk engine. The paper admission adds a history-wide at-most-five-order,
at-most-$20 cumulative upper-limit-notional budget. Openings must share one side;
no opening follows cleanup. Every possibly filled opening lot reserves cleanup
at the configured upper price. Cancel, ACK, expiry and abandonment refund no
paper budget. Cleanup must be an opposite-side, bounded-quantity private
reduce-only IOC; the venue does not acquire a native reduce-only guarantee.

The deployed worker preserves its final dispatch slot for that cleanup, excluding
cancels/openings. Native credits also cover four bootstrap reads, all configured
reads across the separately governed eight-boot ceiling, dispatches and the
gateway's cancellation reserve. This does not promise execution through a price
gap, unknown status, insufficient collateral or exhausted live-run permission.

Retained original attempts attribute real fills/fees once to the correct customer
and unit. A timeout remains unknown, and even flat positions do not produce a
terminal-history witness or release conservative holds. Size the test so existing
margin plus retained holds plus cleanup fit within `C`; a budget reservation is
not an override of financial admission. In particular, spending most of `C` on
the opening may prevent another ordinary close while its hold remains active.

After initialization, a reported nonzero cumulative funding amount becomes an
unresolved retained input. It is not ignored or posted as another payment;
dependent risk remains closed pending actual qualification. Other unsupported
funding/risk semantics remain unresolved under their existing controllers.

## Evidence and exclusions

`crates/bulk/tests/paper.rs` covers the cash identity, once-only reopen/unknown
commit, invalid/missing/nonfresh evidence, scope/production refusal, fixed expiry,
explicit unknown-coverage assumption, funding containment and cleanup notional.
`crates/service/src/units/deployment/tests/paper.rs` checks measured version/roles,
history quarantine and the real signed Handler → admission → original attempt →
worker → native signature → customer fill/duplicate → bounded close path, using
synthetic native ports and RecordCipher. The pending cancel cannot spend the final
cleanup slot. Existing production/recovery tests remain required.

Offline passes are neither a native lifecycle nor Nitro qualification. No escrow
sweep, native deposit credit, transfer, withdrawal, payout, insurance guarantee,
venue-backed recovery or third venue is activated by this profile.
