# ADR 0015 — Public custody vault, private entitlement authorization

Date: 2026-09-30. Scope: [P15](../implementation/PLAN.md#p15).
Approved contract: B04–B07, F01/F09, S01, R01–R03. This is an offline-tested
program boundary, not an approved deployment or claim that the vault controls a
Pacifica HTTP account. [P16](../implementation/PLAN.md#p16) owns the joined funding
controller; [P17](../implementation/PLAN.md#p17) owns finalized recovery claims.

## Decision and package boundary

Use one cohesive custody program. Normal and eventual recovery payments must
share the same token authority, permanent receipts and customer paid counters.
There is no separate trust/upgrade boundary that justifies a second program now.
P17 will extend this schema through a reviewed program change; it must not create
an independent vault or reset payment history. There is no recovery payout in P15.

`programs/` is an isolated Cargo/Anchor workspace; `clients/vault/` is its typed
instruction client. Neither enters the off-chain Rust dependency graph. The
financial kernel remains dependency-free: the on-chain program records custody
movements, not a second ledger of user equity, positions, margin or funding.

At the user's request, language, SPL wrappers, CLI and TypeScript core use exact
Anchor **1.2.0** pins. The generated IDL and TS type are tracked and checked against
Rust source. TypeScript uses `@anchor-lang/core`, with legacy `@solana/web3.js` v1
as required by Anchor's client. No former Anchor package is introduced.
Agave/SBF CLI **3.1.10**, platform-tools **1.52**, SBPF **v0** and Surfpool **1.5.0**
are explicit harness pins. Using the already-qualified SBF toolchain avoids
implicitly upgrading it to Anchor CLI's newer default; the emitted program is
actually executed in Surfpool. Future changes require fresh binary qualification.

Two independent lockfiles have checked SHA-256 fingerprints in
`scripts/vault-dependencies.json`. The 231-package program graph is isolated;
all Anchor 1.x language/derive packages resolve to 1.2.0. Anchor's token-init macro
requires the Token-2022 Rust module, but the typed classic Token/Mint/TokenAccount
boundary **does not accept Token-2022**. Transfer fees/hooks need a future separately
qualified asset policy, not an implicit interface widening. Classic mint freezing
is an external availability risk; a frozen token transfer fails atomically.

## Account schema and public visibility

Every seed includes the program identity implicitly. All PDAs use canonical
bumps; domain and pool are nonzero 32-byte deployment identifiers. Governance
must provision a network/deployment-specific domain before deployment. This
argument is not a program-discovered genesis hash or attestation certificate.

| Account | PDA seeds | Meaning |
| --- | --- | --- |
| `VaultConfig` | `cinder_vault`, domain, pool, mint | Schema 1, authorities, immutable asset/route, current epoch/mode, custody limits and aggregate movement counters |
| Classic token vault | `tokens`, config | SPL-owned token account whose transfer authority is the program-owned config PDA |
| `CustomerCounter` | `customer`, config, owner | Wallet attribution and lifetime public deposit/paid totals plus payout sequence; no current private claim balance |
| Deposit `MovementReceipt` | `deposit`, config, owner, operation ID | Immutable actual deposit evidence scoped to its customer |
| Operator `MovementReceipt` | `receipt`, config, operation ID | Immutable funding/return/payout evidence, shared across those kinds and authority epochs |

Public receipts contain config, operation, epoch, kind, owner, destination, exact
amount and sequence. Deposits/payouts are already public under B06. IDs must be
opaque, not private account-state hashes vulnerable to dictionary lookup. There
are no position, balance-snapshot or private journal payloads on chain. The counter
account is not a separate per-user **venue** account.

Receipt and counter accounts have no close/reset/reinitialize path. Rent is an
explicit controller cost, not a hidden token debit. Prefunded system-owned PDAs
are handled by Anchor's normal initialization; they do not change canonical seeds.
Customer deposit IDs cannot squat a pending operator receipt. An existing
operator receipt rejects reuse even with a different movement kind or epoch.
An operator retry observes the original receipt; it does not select a new ID.

## Authorities and instructions

Governance, funds, recovery and broker owner are pairwise-distinct signing-capable
keys. Production key custody, co-signing, upgrade control and release approval
remain G03; fixture signers are not Nitro evidence. A role may ultimately be
backed by a separately reviewed signing arrangement. P15 does not assert native
PDA/multisig compatibility or silently select production threshold custody.

| Instruction | Required authority | What it enforces |
| --- | --- | --- |
| `initialize` | Actual loader upgrade authority + funds + recovery signatures | Canonical config/vault; correct executable and ProgramData relationship; mint, distinct roles, explicit positive caps and fixed broker token account |
| `register_customer` | Customer wallet | One public attribution/counter account per wallet/config; cannot reset payments |
| `deposit` | Customer wallet | Correct source owner/mint, current domain/epoch, unexpired positive movement, actual checked token transfer and fresh customer receipt |
| `release_funding` | Funds authority | Exact allowlisted broker token address/owner, current normal epoch, expected lifetime funding sequence and remaining gross epoch budget |
| `return_funding` | Broker owner | Actual checked transfer from the same fixed route; normal or frozen mode; no second customer credit |
| `normal_payout` | Funds authority | Registered customer, destination token owner/mint, per-call bound, expected cumulative paid total **and** payout sequence, current normal epoch |
| `rotate` | Old and new governance + new funds/recovery signatures | Normal mode, exact old epoch, distinct new roles, increment epoch; only epoch funding consumption resets |
| `freeze` | Recovery authority | Normal-to-frozen fence and checked epoch increment; no token movement, activation, heartbeat or resume |

Bootstrap checks the real upgradeable-loader ProgramData owner/type, its address
linked by this executable and its upgrade authority. Supplying an arbitrary
governance signer cannot win a first-initializer race. Once initialized, config
governance and loader upgrade authority are distinct **responsibilities**: rotating
config governance does not automatically rotate the loader's authority. Loader
authority can replace program code, so deployment governance must cover both.
Immutable programs need a separately approved bootstrap model; initialization
does not fall back to accepting an arbitrary signer when upgrade authority is absent.

The vault pins the classic SPL Token executable and typed mint/token owners.
Relations bind config, customer, mint, source/destination and exact route. Token
delegates/explicit close authorities are rejected. No arbitrary CPI target or
caller-supplied signer seeds exists. No mutable token self-transfer is allowed.
Destination accounts need not be ATAs; correct token ownership/mint is enforced
and the precise destination is bound by the signed instruction.

All movement amounts/counters/slots are unsigned 64-bit integers in mint atoms.
SDK conversions require `bigint`, reject overflow/negative values and copy IDs.
The private ledger's wider signed accounting units require checked conversion
at P16's boundary. Orders and perp notional are not token-transfer quantities.

For amount `a > 0`, define lifetime funding sequence `f`, epoch gross released
`r`, configured cap `L`, customer lifetime paid `p` and payout sequence `s`:

```text
Release: expected_f = f; r+a <= L; (r,f) := (r+a,f+1)
Payout:  expected_p = p; expected_s = s; a <= payout_cap;
         (p,s,aggregate_paid) := (p+a,s+1,aggregate_paid+a)
Rotate:  epoch := epoch+1; r := 0; paid/sequence/receipts unchanged
Freeze:  mode := frozen; epoch := epoch+1; paid/sequence/receipts unchanged
```

Every addition is checked. Token CPI success, receipt initialization and counter
updates share one Solana transaction. Any downstream failure restores state and
receipt creation (transaction SOL fees are not refunded). A payout race with the
same starting counters has at most one successful transaction, even with distinct
operation IDs. A stale signed request cannot regain authority after rotation.
Rotating a cap authorizes a new **epoch** budget; it is not a lifetime release cap.
The private admission controller must still enforce pool risk/location limits.

## Financial, custody and recovery boundaries

The normal funds signer must authorize only a durable, financially qualified P08
payout under the P09 gate. This program does not verify private balances or prove
solvency. In particular, `paid <= deposited` would incorrectly prohibit profits;
it is deliberately **not** a payout rule. The program's per-call cap is not a
substitute for the ledger's entitlement check. A compromised funds signer remains
a custody risk; production key release is disabled until G03. The recovery signer
cannot pay anyone through the normal entry point.

A direct donation changes physical custody, not any customer's recognized
entitlement. Return receipts move custody back without creating a new deposit.
P16 must bind finalized chain receipts to existing operations, classify donations
in suspense, and debit/credit the authoritative ledger exactly once. Observing a
vault token balance is not a substitute for receipt correlation or finality.

The allowlisted broker wallet has ordinary native signing authority. A release
does not prove venue deposit, force a return, or make venue assets program-owned.
G01/G03 qualification and the funds controller protect that separate boundary.
Released assets must not remain counted as liquid vault cash in the private ledger.

Freeze invalidates ordinary custody authorizations but does not revoke escaped
native requests or venue credentials. External fencing/reconciliation remains
required. While frozen, only actual broker returns are allowed; normal deposits,
funding, payouts, registration and rotation cannot reopen the pool. P17/P21 must
finalize claims using the preserved customer paid totals, settle/return backing,
and require authorized activation. Heartbeat loss has no payment authority.
Late raw donations remain attributable/suspense work, never recovery credit by default.

## Promotion and verification

W04/W05 contributed requirements, not disposable keys or the one-customer/run
fixture account model. Selected original source SHA-256 fingerprints:

- W04 `fixture/src/lib.rs`: `53fca8ddfb4d8d69caec78e6ad388e8ef3d5c613a71a658050fd3a475e31bb08`.
- W04 `fixture/tests/vault.test.mjs`: `7a2ff54a79dadb1b3c94ee44f031ab9be583e5f239e294a54af07976c90dff30`.
- W05 `private-recovery/src/lib.rs`: `0cee93393ef24a49bcb8263b49cade29d96a8ba3d3f965ca6a400b43e0bd1088`.

The tracked suite executes actual signed SBF instructions in Surfpool with real
signature checks and classic token CPIs. Cheatcodes only load program/genesis
fixtures and construct explicitly marked arithmetic/owner boundary states; no
cheatcode replaces a vault transition. Preflight is disabled for negative tests
so failures execute, rather than merely simulate. Distinct test wire signatures
ensure replay tests hit program receipt checks, not RPC signature deduplication.

Coverage includes bootstrap hijack/reinit, wrong owner/mint/program/PDA/recipient,
signer substitution, expiry/domain/epoch, cap/counter races, immutable receipts,
two customers in one vault, donations/profit payouts, prefunded PDAs, actual
frozen-token CPI rollback and retry, checked overflow, authority handoff and frozen
return-only behavior. Strict Rust/TS checks and generated-IDL equality accompany
the compiled-SBF tests. The runner rejects stack-overflow diagnostics even when
the compiler exits zero. Account wrappers are boxed to stay within SBF frames.

Tests use loopback RPC, `--offline --no-deploy`, an in-memory database and fresh
in-memory signing identities. No configured wallet, RPC key, faucet, venue API or
ignored `work/` input is used. The fixed program ID is local-only; the unused
build-generated keypair is ignored and is not a deploy identity. Anchor's provider
wallet is `/dev/null`. No live deployment commands or approved manifests exist.
G03/G05 must approve a fresh identity, code/IDL rebuild, roles, asset/route and
limits before external deployment. Local tests do not establish a hardware
enclave, independent audit, hosted Linux result or complete recovery lifecycle.
