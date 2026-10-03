# ADR 0017 — Immutable final recovery in the custody program

Status: implemented for offline review; not deployed, live-qualified or independently
audited. Date: 2026-10-01. Owns PLAN P17. Extends [ADR 0015](0015-solana-vault.md);
the financial ledger remains [P03–P16](../implementation/TRACKER.md).

## Scope and economic contract

One cohesive `cinder-vault` executable owns custody, the ordinary payout counters
and recovery claims. A second executable would add upgrade/CPI authority without
solving a separate trust boundary. Existing config/customer/receipt layouts are
unchanged. Two new accounts hold a final epoch and permanent claim consumption.
This is an offline source extension, not permission to upgrade a deployed program.

Recovery is operator-assisted: restore authenticated private history, fence native
effects, reconcile, settle and return assets, finalize unpaid obligations, deliver
private claim packages, publish, then separately activate. An outage, elapsed slots,
ordinary snapshot or heartbeat is never sufficient authority. P21 implements the
actual joined preparation/delivery workflow. P17 supplies its on-chain contract
and pure packaging primitives, not a second financial ledger or venue unwind.

For a customer, final `amount` is already net of settled losses and ordinary
payments at the cutoff. `paid_base` and `payout_sequence_base` bind the existing
lifetime counter state. The full recovery amount is paid once; do not subtract the
ordinary payout again. V08: initial claims 100/200, Alice ordinary payment 20 and
settled loss 35 give recovery claims **45/200**; final lifetime payments **65/200**.
Neither stale 100/200 nor double-debited 25/200 is payable.

This bounded slice has positive claims only, one per registered wallet/config,
no new recovery fee and no partial-claim/resume/rollover mechanism. The ordinary
per-call payout cap applies only to normal fund-authorized instructions; activated
claims are instead bounded by the approved immutable leaf and funded total, so a
valid full claim need not be artificially split. These semantics do not approve
production coverage, priority or loss-sharing terms (G02). An all-zero-liability
estate remains fenced and needs no payout root; do not invent a positive claim.

## Authorities and state machine

| State | Entry / permitted effect | Forbidden |
| --- | --- | --- |
| NORMAL = 0 | Existing deposits, registration, normal funding/payout/rotation | Claim publication/activation/claim |
| FROZEN = 1 | Existing recovery signer freezes and increments the authority epoch; broker returns remain possible | Ordinary spending and new customers |
| STAGED = 2 | Governance publisher creates exactly one final statement for the frozen config | Root replacement, ordinary spending, claims |
| ACTIVE = 3 | Distinct recovery signer activates exactly that root against actual vault backing; owner-signed claims pay without per-claim operator signature | Competing normal payouts, reactivation, root replacement |
| CLOSED = 4 | Exact final claim count and zero unpaid remainder | Resume, root reset, second recovery estate on the same config |

Roles retain P15's enforced separation: governance, funds, recovery and broker
are distinct. `stage_recovery` requires current governance; `activate_recovery`
requires current recovery authority and exact domain/epoch/root. A caller cannot
impersonate either role by selecting an account key. Canonical PDAs, program-owned
typed accounts, SPL ownership and signer requirements apply at every instruction.
No instruction closes/reinitializes customer counters, recovery epochs or receipts.

The final epoch PDA is `["recovery", config]`, deliberately **not** per epoch:
publishing another root cannot reset prior claim consumption. A claim receipt is
`["recovery_paid", config, owner]`, not per leaf alone. Even a malicious tree with
two leaves for the same owner and an advanced second counter basis cannot pay the
owner twice. Each fresh recovery epoch belongs to a different, explicitly governed
config; migration/restart is not implemented here or an excuse to erase obligations.

## Statement and source qualification boundary

The immutable statement records domain/epoch/root, positive-leaf count, total unpaid
amount, accepted journal cutoff/hash, reconciliation/delivery evidence hash, policy
hash, aggregate normal-paid basis and funding sequence. It also records explicit
publisher assertions: zero unresolved outgoing operations, zero outstanding normal
reservations, zero unresolved source inputs, zero venue exposure and available claim
packages. False/nonzero assertions, zero identities/cutoff, stale counters and
out-of-bound sizes reject publication atomically.

These assertions are **not independently measured on-chain**. The program cannot
see the private ledger, revoke a Pacifica agent, prove complete history, derive a
settled entitlement or verify package availability. Nonzero hashes are bindings,
not completeness proofs. The separate operator must independently qualify the
statement and evidence before activation; P21/G01/G03 implement/approve that trust
boundary. A dishonest publisher **and** approving operator can publish incorrect
claims; neither a Merkle proof nor the zero fields makes that safe. Do not call
this D14's full solvency proof, attestation verification or unconditional exit.

Publication binds aggregate ordinary paid and funding-sequence counters; activation
rechecks them. Freeze blocks fresh normal wires. Exposed-but-unsettled historical
venue actions remain a P21 blocker even if the custody program is already frozen.
The cutoff must incorporate their resolution, actual funding, fees, losses and prior
payments; a heartbeat or favorable stale snapshot cannot replace that work.

## Exact commitment encoding

SHA-256 uses Solana's hashing syscall in SBF and the same pinned hasher for host
builds. All integers are unsigned little-endian (`u64`, leaf count/index `u32`);
money is mint atoms, never floating point. Public keys and digests are exactly
32 bytes, decimals one byte. Context hash is SHA-256 of the following ordered
concatenation (the root is excluded to avoid a circular dependency):

```text
"CINDER_RECOVERY_CONTEXT_V1"
program || config || deployment_domain || pool || mint || classic_SPL_program || decimals
epoch_u64 || tree_size_u32 || total_unpaid_u64 || journal_cutoff_u64
journal_hash || evidence_hash || policy_hash || aggregate_normal_paid_u64 || funding_sequence_u64
```

Deployment domain must be a governed network/deployment binding (P15/G03), not a
freely reused label. The config binds pool, mint and canonical custody; production
genesis/identity qualification is still a release gate. Leaf hash:

```text
SHA256(0x00 || "CINDER_RECOVERY_CLAIM_V1" || context_hash || index_u32 || claim_id
       || owner || destination_tokens || amount_u64 || paid_base_u64
       || payout_sequence_base_u64 || salt)
parent = SHA256(0x01 || left || right)
```

The ordered tree follows [RFC 9162 §2.1.3](https://www.rfc-editor.org/rfc/rfc9162.html#section-2.1.3),
splitting at the largest power of two smaller than the subtree size. It does not
sort hashes or duplicate the last leaf of odd trees. Verification consumes the
exact path with at most **16** siblings and **65,536** leaves. The client constructs
the tree recursively; the bounded on-chain verifier uses the independent iterative
RFC algorithm. Client checks canonical indices, unique owners/claim IDs, positive
claims, exact checked sums and contextual hashing. It does not invent entitlements.

Salt is 32 nonzero bytes. Production preparation must generate an unpredictable
cryptographic salt inside the private runtime, not a wallet-derived hash or the
deterministic **test-only** salts. Private claim packages carry their own statement,
leaf/counter values, salt and path. Public root publication reveals aggregate total
and count; an actual claim reveals owner/recipient/amount/salt/proof, as permitted
by B06. It does not publish positions/history or all private account balances.

## Backing, shared counters and containment

Activation requires unfrozen canonical classic-SPL custody to cover **all**
declared unpaid claims, not an off-chain balance estimate. Normal spending remains
disabled throughout. Each claim rechecks aggregate remaining backing before transfer,
valid owner/recipient/mint/program/context/proof and exact lifetime counter basis.
Checked arithmetic computes the new paid/sequence/aggregate/remaining/count; SPL
transfer and receipt initialization commit in the same transaction. Failed transfers
roll back all consumption and token movements, allowing the identical claim to retry.

Post-activation impairment blocks **every** claim if vault amount is below total
remaining, including small claims that could be paid ahead of others. No automatic
haircut, first-come deficit payout or obligation deletion is introduced. Immutable
claims remain; actual returned backing can restore the ability to pay. A frozen
destination or missing recipient requires repair; this slice cannot retarget a
published leaf. Final count must equal tree size and final remaining must equal zero.

Broker `return_funding` remains receivable in every recovery mode and creates no
new customer credit/root capacity. New admitted deposits and registrations reject
after freeze. Unsolicited SPL transfers cannot be prevented; they do not change
entitlements or the root. P21 tracks late assets in suspense/refund accounting and
resolves any third-party obligations; do not infer that all excess vault cash is
free protocol capital. This slice has no surplus sweep.

An ordinary Merkle tree proves membership, **not** that totals match all leaves
or that every entitled customer appears. Checked packaging prevents accidental
sum/owner mistakes, but the contract retains deliberate dishonest-publisher
counterexamples: understated total blocks an otherwise included claim; an
overstated total still pays each valid, fully backed included claim, including the
last claimant, but blocks terminal close rather than silently destroying the
remainder. The epoch stays ACTIVE with its unexplained remaining amount; ordinary
payouts remain fenced and paid claims cannot replay. A duplicate-owner tree cannot
pay twice. Incomplete estates stay
contained and require a separately reviewed remediation, not a root-reset escape.

## Evidence and implementation boundaries

### P21 financial-cut projection

`crates/journal/src/recovery.rs` now derives a private read-only cut from the
existing journal. It revalidates the backend's current head/authority before
reading; plain SQLite still supplies no independent rollback witness. The caller
must name the exact reviewed head and an applied, source-covered complete native
check following every financial observation. Controls alone may follow that check.

The cut requires a durable ordinary-path freeze, closed order/funds lifecycles,
no active holds or unqualified generic attempts, no unresolved inputs/attribution/
reconciliation/protection and no open **gross** user/house/suspense/native positions.
Every funding boundary must be settled even when net accrual is zero. Venue,
broker and transit balances must be returned; only recorded vault cash backs the
positive claims. The full configured inventory includes zero/negative accounts;
customer debt is retained but never offsets someone else's positive entitlement.
Confirmed beneficiary payments remain a separate counter basis, not a second
deduction from already-net cash. Reserve designation creates no new assets.

The cut remains a financial projection, **not** root-publication permission or
escaped-capability revocation. P21 now joins it to explicit cutover, durable sealing,
installed-custody matching and independently retrievable encrypted packages. These
are separate checks; a valid cut alone still authorizes no claim. The joined changes
advance journal `ENGINE_REVISION` to **21**; revision 20 histories reject rather than
silently replaying under new permission semantics. Migration is not implemented.

Sixteen synthetic journal tests cover settled/replayed cuts, payout/fee accounting,
stale heads and late deposits, missing causal coverage, gross positions, unsettled
zero-net funding, external/transit assets, queued/unfinalized movements, generic
attempts, evidence gaps, remediation underbacking, negative debt, zero accounts,
unattributed deposits and freshness failure. They are not a new hardware/live/SBF
qualification receipt.

### P21 installed-custody matching

`crates/pacifica/src/funding/recovery.rs` joins the cut to P16's already-installed
route, precision and beneficiary mappings. It does not select recipients or
recalculate entitlements. Matching requires the exact frozen P15 domain/pool/mint/
program/config/vault/broker/epoch, a complete unique counter inventory and an exact
returned vault amount. All chain observations must cover the latest applied chain
receipt and accepted raw-resolution slots; venue causal cuts and milliseconds are
not compared with Solana slots. Unknown or extra tokens require reconciliation,
not automatic house credit. Program mint atoms must fit u64 without truncation.

Successful original P16 plans/receipts reconstruct lifetime paid and payout/release
sequences from the fresh P15 zero-counter origin. Duplicate receipts do not consume
them twice; unsupported historical counter baselines or unexplained jumps refuse
matching and require explicit migration. Returns never reset the funding sequence.
The positive claim basis contains the fixed wallet/token recipient, already-net
amount and checked prior-counter bases; zero/negative users remain in the cut.

The chain input is an explicitly trusted **decoded observation port**, not a host
assertion promoted to proof. Its cluster/finality/schema/PDA/SPL authority checks
remain mandatory for the actual chain integration. The result's private Debug is
redacted and it authorizes neither staging nor activation. Six added P16 tests cover
duplicate receipt/replay, 22 context/inventory/counter mutations, return/release
sequence preservation, unexplained counter jumps, u64 precision and qualified
resolution of previously unknown chain inputs. `clients/vault/src/claims.ts` now
checks coherent raw classic-SPL/program/config/counter accounts, canonical PDAs,
schemas, role separation and exact integer fields before producing the trusted P16
observation. Its caller must authenticate genesis, finality and the complete request
set: a parent-supplied `finalized` flag or network label is not validator proof.
Hardware/live source qualification remains P23/G01 evidence.

### P21 controlled cutover and sealing

The trusted recovery port first authenticates independent writer takeover, custody
freeze and native capability closure. It retains the evidence referenced by `fence`
and commits `Recovery::Begin` with the exact financial version, operator epoch and
deadline. A nonzero fence digest is a binding, **not** a proof of those external
facts. Begin permanently freezes ordinary admission/payout and cannot be repeated
to replace the recovery session. An expired permission stops new dispatch; it never
settles an unknown action.

Existing order observations, original receipts and reconciliation continue to post
to the same journal. Recovery closes use existing bounded IOC sizing, deterministic
allocation, house stress support and ordinary unknown-outcome retention. The
adapter's separate testnet `dispatch_recovery` path cannot be borrowed by ordinary
dispatch; native `reduce_only` stays false because a private close can increase pool
net exposure. Cleanup credit eligibility is independent of create/cancel routing.
Live recovery execution remains a G01/P23 qualification, not an implicit mainnet
switch.

Gross-flat recovery returns allow only Venue → Broker → Vault, with house-owned
fees, shared source reservations and existing P16 original-attempt/receipt rules.
They cannot authorize releases, deposits, customer payouts or a previously prepared
ordinary capability. Returning existing settled assets need not satisfy new-risk
capital targets, but customer debt and unsettled transit are never available cash.
The ordinary payout route binds the authorized **token account**, not its wallet;
both remain independently checked by the custody boundary.

`Recovery::Seal` irreversibly stops further signing/exposure after all orders,
movements, reservations and evidence are resolved and gross positions and external/
broker/transit balances are zero. Facts still arrive and change the authoritative
head; a late fact invalidates an already prepared publication. Publication rechecks
the sealed accepted head and custody observation. Separate governed staging and
recovery activation are still required by the program.

### P21 recipient encryption and independent claimant

Each registered owner authorizes a **dedicated RSA-3072/e=65537 SPKI** with an
Ed25519 signature over SHA256(`CINDER_RECOVERY_KEY_V1` || domain || pool ||
SHA256(SPKI)). The complete immutable key inventory is recorded inside the existing
encrypted journal before cutover. Preparation rejects missing, substituted,
duplicated or unregistered keys. This is an attested private configuration port,
not a newly advertised public onboarding endpoint. There is no wallet-secret KDF,
operator decryptor, key rotation or lost-key reissue. Owners must retain the key;
G03 must approve production provisioning/backup and any future replacement policy.

Private preparation derives every positive claim from the matched sealed journal;
zero/negative accounts remain in the inventory. A cached ordered tree bounds proof
construction to O(n log n), 65,536 leaves and 16 siblings. Cryptographic entropy
supplies independent salts, claim IDs, opaque locators, AES keys and GCM nonces.
Each package is AES-256-GCM encrypted with root/context/locator-bound AAD; its key
is wrapped using RSA-OAEP with SHA-256 and MGF1-SHA-256. Packages are bounded at
32,768 bytes, stored immutably in two approved failure domains and read back exactly
from **both** before availability is reported. Distinct labels alone do not prove
independent failure domains. A failed readback leaves only orphan ciphertext, not
publication authority.

The preparation evidence precommits the two store identities and opaque locator
layout **before** constructing the root. Ciphertext digests cannot be included in
their own plaintext/root commitment without circularity. Instead a public compact
manifest binds the final statement, stores and exact ciphertext digests, and the
separate governed publisher signs its exact bytes after readback/revalidation.
Public entries contain no wallet, destination, amount, salt, leaf index or path.
Opaque locators must be delivered privately and retained locally; there is no
public wallet-to-locator directory.

The independent Node client verifies that governed signature, GET-only replica
failover, ciphertext digest, OAEP/GCM authentication, canonical bounded JSON,
context/root and exact P17 inclusion proof. It decrypts locally using the dedicated
key, never through Cinder. `independentClaim` then checks authenticated live custody,
ACTIVE mode, immutable statement/PDA, fixed owner/recipient, paid bases, unconsumed
receipt and aggregate remaining backing before constructing an unsigned instruction.
The owner's wallet signs; the program repeats the checks atomically. Browser key
storage, hosted package endpoints, rotation/reissue and deployment are not supplied
by this offline SDK slice. OpenSSL owns OAEP randomness/opaque crypto buffers;
native entropy/erasure and the changed measured image still require G03/P23 review.

### Joined offline acceptance

Both HTTP and WebSocket tests initiate a real signed SDK order and payout through
the Noise/WASM client, parent relay and private service. They kill those processes,
restore the same encrypted accepted tail, execute the original normal payout in
compiled SBF, freeze custody, reject the old writer, resolve the unknown order only
with an explicitly **synthetic** native completion certificate, and return synthetic
native house collateral through the existing controller plus an actual SBF return.
The final seal/counter/backing match produces recipient-only packages. With service
and worker stopped, one replica is removed, the independent client retrieves from
the other, rejects corruption/wrong keys/stale context, and submits the real owner
claim to SBF. Loss of both stores fails retrieval; a retained local ciphertext copy
still verifies. Normal 20 + recovery 980 pays the original 1,000 once, while the
separate 1,000 house contribution remains in custody. Replay and old-writer restart
fail. Fake native/witness ports are feature-gated and are **not** live evidence.

P22 expands this representative acceptance into the complete adversarial lifecycle.
P23 qualifies actual native capability fencing, receipts/RPC finality, independent
stores, attested entropy and the changed image. A successful local claim neither
proves all liabilities on-chain nor guarantees exit when a venue remains inaccessible.

`clients/vault/tests/vault.test.ts` executes compiled Anchor 1.2 SBF through the
check-owned offline Surfpool with disposable in-memory keys. It covers V08, strict
owner/domain/epoch/root/asset/recipient checks, unknown reservations/source state,
actual frozen-token CPI rollback, immutable roots, late arrivals, impaired backing,
counter overflow, duplicate-owner and false-total counterexamples, odd ordered
trees and the maximum bounded path in an actual transaction. Pure codec tests check
all indices of sizes 1–65, exact-unit/canonical/sum validation and maximum depth.
Acceptance means **tested**, not a machine proof or independent security audit.
The maximum-size estate is a synthetic wire/compute capacity fixture; it is not
evidence that every fictional recipient has a live claim package or funded account.

The only new direct Rust dependency is exact `solana-sha256-hasher = 3.1.0` with
host `sha2`; it already exists in the isolated pinned graph. No transitive package
version changes, kernel dependency, network client or Solana monolith is introduced.
The dependency guard and lockfile fingerprint make the reviewed change explicit.

Local research consulted as provenance (not imported keys/deployments/SDK types):

| Source | SHA-256 |
| --- | --- |
| W05 M2 program `work/experiments/private-recovery/src/lib.rs` | `0cee93393ef24a49bcb8263b49cade29d96a8ba3d3f965ca6a400b43e0bd1088` |
| W05 ordered-tree prototype `work/experiments/private-recovery/proofs.mjs` | `609a698b9da8afadc7d856ae60f9f3a2e8bcc92627ac1b57f5f2a223b826a615` |
| W05 closeout | `42072e3058379769f799fc64ebc8de725e1ceeda269717fa1b334a7ac8295106` |
| W07 `work/specs/verification-recovery.md` | `06d8ab815edaf1f583e4538bb236a3ad0c9c08d9f5e3bb56930df1827657f7e2` |

No `work/` content is required for checkout tests. P18 owns customer grants/API;
P19/P20 own approved attested code/transport/hardware; P21 owns ledger-to-final
claim derivation, reconciliation, fencing and independent encrypted delivery;
P23/G01 qualify actual venue/RPC behavior; G02/G03/G05 approve live terms, keys and
deployment. None is replaced by this local contract passing tests.
