# ADR 0018: private customer authorization and application operations

Date: 2026-10-01. Status: implemented for offline review, not deployed.
Scope: [P18](../implementation/PLAN.md#p18). [Baseline](../implementation/BASELINE.md)
remains authoritative for economics. Channel/attestation qualification is P19/P20.

## Boundary and dependency decision

Add `crates/api` (`cinder-api`), a venue-independent runtime boundary over
`cinder-kernel` and `cinder-journal`. Reuse the existing exact SHA-256, Ed25519
and zeroize versions/features from P14; no new external Cargo version or feature.
The kernel stays dependency-free/no_std. This crate cannot sign native orders,
load wallets, make HTTP/RPC requests, install numerical risk policies or move funds.
Add `clients/private`, a browser/Node-compatible TypeScript semantic client with
no runtime npm dependency. Its development tools reuse pinned TypeScript 7.0.2
and Node types 24.19.0. The Anchor SDK remains separate in `clients/vault`.

`ConfidentialChannel` is a **trusted injected port**, not a cryptographic proof or
host-provided flag. Its domain, binding and expiry must come from an established,
fresh, release-qualified encrypted session. P18 ships no network listener, URL,
channel implementation, plaintext fallback or attestation success shortcut.
Success, errors and snapshots all use the same private channel. P19 must implement
and qualify its framing/crypto/verifier; P20 must qualify the actual Nitro runtime.

## Authority and grants

Governed onboarding supplies every configured private account's owner Ed25519 key
and owner-controlled quote token account. The bindings must agree with P16's route
and actual Solana asset/owner checks; this API cannot prove token ownership merely
from bytes. Installing its immutable contract fingerprint and initial authority
epochs is one protected journal transaction. A changed owner/recipient/limit
contract refuses restart until an explicit future migration, not a silent reset.
Runtime onboarding, owner-key rotation and alternate recipients are not introduced.

Owner requests may read, trade, cancel, request payouts, select bounded private
leverage, grant agents and revoke. Agents never receive omnibus venue credentials,
payout, redelegation, leverage-settings or recovery/house administration authority.
The bounded initial grant has one market, READ/TRADE/CANCEL permissions, per-order
absolute lot and fee-cap limits, a durable lifetime accepted-order count, an expiry
and the current account epoch. READ covers the owner's account views/operations;
TRADE-only retries can reveal only that same signer's admitted operation. Agent
cancellation covers only its original orders in the granted market. Owner access
covers all attributed operations. Multiple-market automation currently needs
separate scoped agent keys; general grant portfolios are not silently inferred.

Financial order/cancel deadlines cannot exceed agent grant expiry. Exact economic
retries do not spend another order count; rejected journal controls do not consume
an accepted-order budget. Existing commitments still constrain the risk engine.
Revocation advances the journal's **customer authority epoch** and invalidates all
old grants. A prepared old action cannot expose its signature under that epoch.
An already exposed action cannot be undone: retain holds and reconcile/cancel.
Revocation of this Cinder agent is not native venue-agent revocation.

## Canonical wire and replay

Requests are at most 1,024 bytes: `CINDER-API\0`, u16 version 1, then fixed-width
big-endian fields. Network, deployment, account, request ID, signer and session
binding are each 32 bytes; policy is u32, epoch/auth expiry are u64. The exact
tagged command follows, then the 64-byte Ed25519 signature. Signed quantities are
i64 lots/i128 quote atoms; prices/deadlines/limits are u64; flags are exactly 0/1.
No JSON duplicate keys, floats, permissive method aliases or trailing bytes exist.
Market ID plus signed policy selects the **server-qualified** precision/quote grid.
This is a Cinder application signature, not a forwarded Pacifica signature.

| Tag | Command | Exact body after tag |
| --- | --- | --- |
| 0 | View | Empty |
| 1 | Operation | Original private request ID |
| 2 | Order | Market, signed lots, min/max ticks, fee cap/lot, GTC/ALO/IOC byte, reduce-only, financial expiry |
| 3 | Cancel | Original request ID, distinct cancel attempt ID, financial expiry |
| 4 | Payout | Net recipient amount, additional fee cap, partial consent, financial expiry |
| 5 | Grant | Agent key, permission mask, market, max lots, max fee/lot, lifetime accepted-order count, grant expiry |
| 6 | Revoke | Empty; advances current epoch |
| 7 | Leverage | Market, leverage scaled by 10,000, selection expiry |

The signature binds **all** fields, including fresh channel transcript/challenge
binding, policy, epoch and authentication expiry. The economic SHA-256 digest
uses a separate `CINDER-API-INTENT` domain and includes deployment/account/ID,
policy and exact command, but not replaceable session/signer/auth expiry/epoch.
Financial expiry stays immutable. An authenticated owner can therefore re-sign an
unchanged intent after reconnect/revocation without creating a new native attempt.
Request ID is the application economic nonce; cancel attempt ID, native nonce,
cipher counter, journal commit ID and writer epoch remain separate.

Order: bound session/domain/expiry/signature → authenticated current journal head →
current customer epoch/grant → account-scoped dedupe/query → controls → durable
CAS commit → private response. Never query a saved outcome first. Mutation IDs
share one per-account namespace across methods; changed economics conflicts.
Accepted **and rejected** journal outcomes retain exact IDs in versioned private
provenance. Grants/replay are reconstructed from this same protected history,
not a second authority database. A CAS loser/uncertain write returns unavailable;
reload and authenticate/query before considering another action. No auto-retry.

READ requests are snapshots/queries without financial side effects and do not
consume mutation IDs. Current permissions/expiry still apply. An owner that loses
a revoke response can re-sign/query using its known next epoch; a revoked agent
cannot retrieve old private receipts. No cross-customer target/account is accepted
in a cancel/query body.

## Joined financial operation semantics

Orders construct P07 intents/verified-port approvals, never raw native commands.
Private reduce-only, prices, fee caps, precision, policy and deadlines stay bound.
There is no legacy flat-cash risk fallback: P09 policy must be installed. A trusted
server-side `Admission` port computes holds at the exact state; P09 evaluates the
full candidate, all existing commitments and qualified marks at commit time.
This port supplies no numerical production defaults; calibration remains G02.
P14 later prepares/persists/exposes the original native action under its own gates.
No native attempt is created merely because an API order was accepted.

An unexposed cancel releases only that attributed order's hold; an exposed cancel
prepares one distinct cancellation attempt. ACK does not certify missing fills or
completion. Cancel outcomes follow the target order's qualified lifecycle, not
the successful creation of a cancel intent.

Payouts are owner-only P08 intents from vault to the fixed registered token account:
**net recipient amount + maximum extra customer-paid fee**, with explicit partial
dispatch consent. Source debit/HTTP ACK is not payment. Actual beneficiary receipts
and charged customer rail fees come from the same ledger movement. P16 handles
fund retrieval, source qualification, actual counters and eventual settlement.

Receipts distinguish rejected, durable acceptance, possible dispatch, ACK,
partial actual execution/payment, qualified completion and unknown. Filled lots,
paid amounts, rail fees, fee cap, partial consent and exposure flag are separate.
Order fee cap is **not actual order fees**; actual allocated economics already
affect authoritative cash. No new fee/funding allocation engine is introduced.
Cancelled/rejected/reduced orders are not fabricated fills. Receipts are not signed
solvency proofs or promises that every action will execute.

## Private projections, SDK and limits

Snapshot fields are only this customer's settled cash, unsettled funding, explicit
holds, signed positions/basis and retained private operation IDs. Cash is not equity
or withdrawable collateral; holds are not pending-inclusive margin. Views expose
neither native IDs, pooled/house/suspense books, global journal sequence nor raw
evidence. Another customer's mutation does not change an otherwise identical view.
Reconnect/poll always gets a full authenticated snapshot. Network subscriptions,
ordered stream framing/backpressure and attested channel implementation remain
P19; no global sequence is disguised as an account-local stream.

The SDK injects wallet/agent `signMessage` and the trusted channel, encodes the same
bytes, preserves bigint precision, verifies reply type/ID/policy/digest and bounds,
and redacts transport exceptions. It never calls native APIs, generates hidden
venue nonces or automatically repeats an uncertain economic action. Typed protected
errors contain only invalid/unauthorized/not-found/conflict/unavailable codes.
Buffer clearing is best effort in JavaScript, not a GC/memory-erasure guarantee.
Public market data, website/terminal UI and release packaging are not added here.

## Evidence and assurance

W06 source: `work/specs/private-api-runtime.md`, SHA-256
`6ee0084666c8681f9b894a9d1a9c645997aa10d0bf352b257053f74798b03522`.
This ADR and tracked fixtures promote the applicable semantic contract; a fresh
checkout never needs ignored research. Eight synthetic request vectors compare
exact Rust/TypeScript bytes, hashes and independent Ed25519 bindings, including
negative lots and prices beyond 2^53. Encrypted-journal tests exercise ownership,
grant limits/expiry, revoke/exposure, restart, exact retries, CAS races, rejected
IDs, ACK/partial/unknown/cancel and actual partial beneficiary settlement.
SDK tests cover strict replies, exact signatures, redacted failures and no retries.
These are offline implementation tests/self-review, not formal proof, independent
security audit, transport qualification, source completeness or deployment evidence.
