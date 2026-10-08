# Native-semantics invocation receipt

Recorded 2026-10-08. The fourth invocation establishes an actual bounded native
deposit/payment round trip, not shipping financial completeness or a Nitro/P23
pass. Earlier stopped invocations remain retained failures, not relabeled passes.

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
unchanged. That preparation had no approval, journal, lock or live activity;
neither prior approval was reused. The user subsequently approved this exact
replacement, as recorded below.

## Third approved invocation: signer-buffer aliasing refusal

The user explicitly approved `run-final-03` at seal
`ade4499008899fc2fc8c8d43b41df8a3836b7bed2a990bc9670935a56597c604`.
Source checkpoint `19a6953181695371a97ce7880c710bf6151a9d01` started once.
The devnet deployment/executable, mint/vault and fresh-identity checks passed.
Pacifica returned an explicit absent account and a successful empty balance
history page. These observations are not a complete financial source cut.

The first lending-disable POST returned HTTP 400 with
`signature_verification_failed`, before funding. Offline verification against
the configured broker public key fails. The signing preimage matches the
[documented canonical format](https://docs.pacifica.fi/api-documentation/api/signing/implementation);
the local defect is buffer ownership: `Keypair.fromSecretKey` retains its input,
which the loader immediately cleared. The stored public key remained correct,
but the retained signing bytes were zeroed. Verifying against that same private
key concealed the wrong identity. This is not a native signing-policy failure.

| Evidence | Actual result |
| --- | --- |
| RPC / native HTTP / WSS | 19 / 3 / 0 |
| Native signed requests | One explicitly rejected lending-setting request |
| Simulations / chain submissions / deposits / withdrawals | 0 / 0 / 0 / 0 |
| Funds moved / transaction fees | 0 SOL; 0 USDP |
| Settings readiness, customer credit, payment and complete cut | Unqualified |
| AWS, program deployment or shipping gate changes | None |

The original private approval, wire, response, journal and lock are preserved.
No original economic operation was exposed and no used record was restarted.

## Corrected signer boundary and fourth invocation

The diagnostic now gives the SDK a separately owned copy before clearing its
parse buffer, and verifies native signatures against the expected public
identity. A deterministic unfunded regression reproduces the old aliasing fault,
requires its rejection and verifies the corrected native and chain signatures.
Wrong identities and malformed key arrays also refuse. JS/SDK objects still
offer no secure-erasure guarantee; this is not a Nitro or customer-key loader.
All 17 core/driver cases, 60 contract/inventory cases and ten vault funding/ABI/
signer cases pass; vault type checking passes. No Rust/SBF/Nitro rerun is claimed.

The user granted standing authority on 2026-10-08 for small test-only source/
configuration corrections and reseals within unchanged devnet/testnet limits.
It does not authorize economic retries, uncertain replacement operations,
increased exposure, AWS/mainnet/customer assets or custody/security-policy changes.
Future agents must record applicable authority on a corrected seal, without
repeating a permission question for a minor test-only correction.

Under that authority, `run-final-04` has fresh disposable roles and seal
`77421841139e81672e3ea6f4eeeaa8b2adf8be7e695b4c804148c16b70990b6a`.
Its reviewed amounts, fee-acceptance limitation, bootstrap bounds, request budgets
and no-AWS scope are unchanged. Its source-bundle SHA256 is
`be4fdf99ca24059d44fc96128becea2578c48fec941d2c1b7fb57b6339f1d5da`.
The one-shot runner completed in 253,308 ms with `closeout-unresolved`, because
lending disable was rejected after deposit as well as before it. The original
result is preserved; the later settings-only check does not rewrite it.

| Evidence | Actual result |
| --- | --- |
| Main invocation RPC / native HTTP / finite WSS | 96 / 16 / 4; within approved caps |
| Controlled chain sends | Six originals, each simulated and finalized; no resend |
| Faucet and native deposit | 20 USDP; exact finalized broker debit and venue-vault credit |
| Native deposit observation | Matches the original finalized signature; no customer credit inferred |
| Native withdrawal | One original UUID, successful audit ACK, batch-linked confirmed event and independently finalized broker receipt |
| Gross / native fee / net | 20 / 1 / 19 USDP |
| Net return | Exact finalized 19 USDP back to the fresh owner; broker token balance zero |
| Last venue observations | Balance/pending balance zero, no orders/positions, borrowed/pending interest zero |
| Sponsor debit including its transaction fees | 0.06001 devnet SOL, below 0.10 SOL cap; unused test gas and account rent retained |
| Main invocation lending/bootstrap | Both disable requests returned the exact HTTP-422 rejection; setup unresolved, final setting null |
| Lost-reply experiment | Response withheld before controller acceptance; private audit retained; not actual packet loss or a process crash |
| Complete native frontier / UUID recovery / shipping credit | Unqualified; `source_cut:null`, `financial_completion:false`, no activation |
| AWS, program deployment, trading/agents/loans/recovery | None |

The streamed public native executable SHA256 is
`4806b3b684357596851a3436874ff8fb28fbea0ea0ddf5dae6c725fc6a78e901`;
the loader slot/authority were unchanged. This is byte-identity observation,
not verified-source equivalence or an audited native-program claim.
The original hash-linked journal was independently replay-verified. Private
requests, replies, captures, simulations, original wires, signatures and keys
remain in the ignored private directory; none is published or deleted.

## Bounded settings-only follow-up on the empty original broker

The original scope permits bounded disable attempts after definite setting
rejections. With all net funds returned, a separately retained settings-only
helper used the same original disposable broker, original remaining deadline/
request budgets and standing test-only authority. Its code digest, parent journal
head and scope were durably recorded before I/O. Its own lock prevents a second
invocation. It does not restart the economic runner or alter its original result.

The check used four HTTP calls: explicit empty-account read, one new
`disabled:true` request, settings readback and loan readback. It passed
`observed-disabled-no-debt`: actual setting true, borrowed and pending interest
zero. No RPC/WSS, new funding, economic POST or chain transaction was performed.
Combined fourth-run counts are 96 RPC / 20 HTTP / four WSS and three setting
attempts, within the original ceilings; all earlier records remain intact.

This establishes that disable/readback works on the established empty account.
The earlier account absence followed by successful deposit indexing and this
late successful setting is **consistent with an initialization/readiness delay**;
it does not prove the precise native failure cause, atomic bootstrap or safe
customer funding. The original 120-second bootstrap gate remains unpassed.

## Next disposition

1. Account setup: retain a staged initialized-account readiness check and positive
   disable/debt/interest readback before customer funding. Qualify any fresh-account
   bootstrap separately; do not inherit this late success as an atomic policy.
2. Native evidence: the actual signature/event/payment facts narrow the provider
   contract, but do not supply complete native causal coverage or recover an ACK
   genuinely unavailable to the controller. Final page/zero pending balance is
   not a no-later-effect certificate. Bring that named G01/interface or policy
   gap for review before shipping credit/payment activation.
3. Preserve the returned test assets, gas, private originals and reusable helper.
   No further financial invocation or changed-image AWS qualification is implied.
   P23 stays in progress; these facts do not qualify full vault/user payouts,
   trading/funding, recovery or the measured shipping financial workflow.
