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
**This replacement is unstarted and requires separate exact-record approval.**

Native setup, original deposit/withdrawal/payment evidence, lost-reply recovery
and causal completeness remain unqualified. No source cut, customer credit or
financial completion certificate is inferred from this stopped invocation.
