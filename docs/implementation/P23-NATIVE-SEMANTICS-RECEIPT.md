# Native-semantics invocation receipt

Recorded 2026-10-08. This is a stopped diagnostic invocation, **not** a native
funding pass, hardware receipt or completed P23 financial workflow.

## Approved original invocation

The user explicitly approved `run-final-01` at seal
`2dbcf59f6272128e08eec9f34c113af3967549025e162ed76362d2eeff736fcc`.
The one-shot runner started once and stopped while loading the configured sponsor.
Offline preparation had retained a trailing display space in Solana CLI's
`Keypair Path` value, producing a nonexistent filename. The correct file exists
and has the required owner-only permissions; neither the RPC nor Pacifica was
responsible for this failure.

Read-only inspection of the original durable journal establishes:

| Evidence | Actual result |
| --- | --- |
| Journal | One `start` record; no consumed request or exposed-operation records |
| RPC / native HTTP / WSS calls | 0 / 0 / 0 |
| SOL funding, faucet mint, native deposit and withdrawal | Not submitted |
| Funds moved or transaction fees incurred | 0 SOL; 0 USDP |
| Simulation, native setup, payment and completeness | Not reached; unqualified |
| AWS / program deployment / shipping gate changes | None |

The original lock, approval, identities and private artifacts remain untouched.
No restart, second submission, replacement UUID, rescue transfer or cleanup
transaction was performed. Secret files and request payloads are not published.

## Local correction and replacement preparation

Offline preparation now trims CLI display padding, requires one absolute locator
and validates file type, ownership, permissions and size **without reading the
sponsor secret** before creating fresh identities. Execution validates the exact
sealed locators before locking; it does not silently trim or amend them. A later
signer-decoding failure is durably classified before network I/O.

Verification on pinned Node 24.21.0 passes:

- 15 native tool regressions, including display whitespace, missing/relative/
  ambiguous paths, permissive files and symlink refusal.
- All 58 contract/inventory checks.
- All nine existing/new vault funding/ABI checks and vault SDK type checking.

`run-final-02` is prepared offline with corrected source/configuration and fresh
disposable identities at seal
`160e00d1a5a7286f3dbabeda1aecdf5ccd9123b600eeae319548bfeeb8d1e59f`.
Its scope is unchanged: 20 faucet USDP, 0.035/0.025 devnet SOL funding within the
0.10 SOL sponsor cap, one original deposit/withdrawal and no AWS. The 2-USDP native
fee ceiling remains receipt acceptance, not a signed maximum fee.
That replacement subsequently received explicit exact-record approval; its actual
invocation is recorded below. The preparation above was not itself a live pass.

Native setup, original deposit/withdrawal/payment evidence, lost-reply recovery
and causal completeness remain unqualified. No source cut, customer credit or
financial completion certificate is inferred from this stopped invocation.

## Second approved invocation: loader preflight refusal

The user explicitly approved `run-final-02` at seal
`160e00d1a5a7286f3dbabeda1aecdf5ccd9123b600eeae319548bfeeb8d1e59f`.
Source checkpoint `90b0e00a46fec8cd27da9fdbc997983de12d105d` started once and
performed three read-only devnet RPC requests: genesis, native Program account
and its 45-byte ProgramData header. All three returned HTTP 200. The verified
genesis, program-data pointer, deployment slot `376257391` and upgrade authority
match the pinned expectation. Funding/simulation/native requests were not reached.

The diagnostic constant for the loader was **invalid**: it omitted three trailing
`1` characters. The RPC owner matches the canonical
[`BPFLoaderUpgradeab1e11111111111111111111111`](https://solana.com/docs/core/programs/program-deployment).
This is our local constant/fixture defect, not observed native deployment drift.
The old offline fixture copied its owner from the same incorrect constant, so it
could not detect the typo. This evidence is retained rather than relabeled a pass.

| Evidence | Actual result |
| --- | --- |
| RPC / native HTTP / WSS calls | 3 / 0 / 0 |
| Simulations / financial submissions | 0 / 0 |
| Funds moved or transaction fees incurred | 0 SOL; 0 USDP |
| Native setup / credit / payment / complete cut | Not reached; unqualified |
| AWS, shipping financial changes or retries | None |

The stopped lock, hash-linked journal, original approval and raw private RPC
receipts are preserved. No second invocation, signature exposure or rescue took
place. Offline replay of those retained headers passes the corrected validator;
it is not a fresh read, executable/source-equivalence proof or full preflight pass.

## Broader local boundary correction before another invocation

Route constants now parse as canonical 32-byte public keys during module loading.
The independent ABI test pins the official loader instead of copying `ROUTE`,
derives the ProgramData PDA through that loader and rejects the old typo.

Review against current Pacifica primary documentation also corrected two local
probe mappings before any assets could be committed:

- [`account_transfers`](https://docs.pacifica.fi/api-documentation/api/websocket/subscriptions/account-transfers)
  carries a single event object. The tool retains it rather than silently ignoring
  it; malformed event envelopes refuse. Bounded array fixtures remain diagnostic
  compatibility inputs, not a documented replay/completeness guarantee.
- [`account/balance/history`](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-balance-history)
  is the documented perp balance-history route. Baseline still requires an
  explicit successful empty final page; missing/404 history is not empty. Planned
  subsequent observation windows use that route and account info, not assumed
  perp withdrawal-history/pending paths or unrelated spot endpoints. Closeout
  checks actual `pending_balance`; it explicitly leaves operation completeness
  false. No UUID correlation, source cut or no-later-effect certificate is invented.

These corrections affect only the local diagnostic tool/tests. Shipping Rust
providers, source-cut/credit/payment gates, resources and economic policies are
unchanged. Pinned checks pass 17 native-tool cases, all 60 contracts/inventory
cases, all nine vault codec cases and SDK type checking. No Rust/SBF/Nitro rerun
or hosted new-head CI success is claimed by those local checks.

The offline-only replacement is `run-final-03`, seal
`ade4499008899fc2fc8c8d43b41df8a3836b7bed2a990bc9670935a56597c604`.
Its `REVIEW.md` and `MAPPINGS.md` describe the exact fresh roles, source bindings
and corrected observation contract. Amounts, devnet fee payers, bootstrap bounds,
one original deposit/withdrawal, fee-acceptance limitation and no-AWS scope remain
unchanged. **It has no approval, journal, lock or live activity.** Fresh exact-record
approval remains necessary; neither prior approval was reused.
