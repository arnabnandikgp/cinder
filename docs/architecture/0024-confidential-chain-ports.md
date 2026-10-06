# ADR 0024 — Confidential chain funding and finalized receipts

Date: 2026-10-06. Scope: [P23](../implementation/PLAN.md#p23).
Status: local implementation and synthetic port tests; not live qualification.
[Custody](0015-solana-vault.md), [funding coordinator](0016-funding-coordinator.md)
and [recovery](0017-recovery-claims.md) retain their economic and authority rules.

## One controller, one journal

`pacifica::funding::chain` encodes only existing controller-produced release,
native deposit, return and beneficiary payout mandates. It cannot sign an
arbitrary transaction or native HTTP withdrawal. Exact u64 decimal strings,
canonical PDAs, account privileges, instruction data, signer and deployment are
bound before simulation. A legacy packet may contain the one intended instruction
and an optional bounded compute-unit limit; no nonce, lookup table, priority-price
instruction, extra transfer, extra signer or replacement target is accepted.

`service::chain_funding::Port` composes this codec with the original Controller
and the caller's authoritative journal. It checks current program governance and
counters, persists the exposed plan, simulates the exact unsigned message, signs
with the appropriate enclave-owned seed, commits the exact signed wire, and
submits once with RPC retries disabled. Simulation or persistence failure never
exposes a signed replacement. After possible exposure, reconciliation uses the
retained original contract/signature/wire; it never obtains another blockhash,
signs a replacement or resubmits. Missing/pruned/nonfinal history remains pending.

| Rail | Signing purpose | Settlement evidence |
| --- | --- | --- |
| Vault release | Separate `Funds` seed | Original finalized transaction, exact debit/credit and permanent P15 movement receipt |
| Broker → native venue | Existing `Broker` seed | Chain debit only; independently qualified native credit is still required |
| Broker → vault return | Existing `Broker` seed | Original finalized debit/credit, governed epoch/mode and P15 receipt |
| Vault → bound customer | Separate `Funds` seed | Exact original beneficiary payment, P15 receipt and monotonic customer paid counter |

Signed wires, RPC targets and raw evidence are private. Neither ACK, aggregate
balance change nor program-counter growth alone settles a transfer or claim.
Lamport fees are retained as execution costs, not silently converted to quote
fees. The existing quote-asset funding lifecycle remains unchanged.

## Enclave-owned RPC and deployment trust

`chain_rpc` owns authenticated TLS 1.3 inside the enclave. Public measured policy
fixes the devnet host, CA/hash, genesis and parent CID 3 port; encrypted configuration
holds the credential-bearing path, finite RPC budget, freshness and lamport fee
limits. Only `api.devnet.solana.com` and `devnet.helius-rpc.com` are permitted.
The parent `cinder-nitro-egress` relay forwards opaque TLS to fixed HTTPS port 443;
it receives no path, request body, signing seed or financial state.

Responses have bounded identity framing and unique JSON keys. No redirects,
compression, chunked framing, socket retries or system-root fallback are accepted.
Genesis, block height, last-valid height, slot and trusted milliseconds have
distinct meanings. Finality requires the original signature's finalized status
and exact legacy transaction, not `sendTransaction` acceptance.

Approved code cannot fit in a small ordinary RPC account body. `chain_code`
streams at most 64-KiB data slices, hashes the complete approved ELF and checks
all allocated tail bytes are zero. Allocation is bounded at 10 MiB, including
tail. Loader headers, program pointer, upgrade slot and explicit upgrade authority
are checked before and after the stream. A changed header, byte, owner, slot or
exhausted call budget refuses recognition. Financial records retain bounded chunk
metadata/hashes rather than megabytes of executable bytes.

This trusts the authenticated RPC provider and stated upgrade governance. It is
not a Solana consensus/light-client proof. Actual deployment bytes, provider
framing, slot behavior and trust roots require the new live qualification.

## Owner deposits without invented funds

`customer_deposit` observes up to sixteen **preconfigured original transaction
locators**, not an open signing/onboarding endpoint. Its typed codec verifies the
registered owner's signature, exact P15 deposit instruction, token source/vault,
nonce and deployment. Recognition additionally checks finalized transaction
token deltas, classic SPL mint/ownership, approved executable, immutable deposit
receipt and customer counter. Only then is one customer receipt normalized into
the same Vault source/journal. Domain-bound identity and retained applied outcome
make repeated polls/restarts credit once without another request.

Customer deposits are public Solana transfers. Confidentiality protects private
accounting and enclave signing, not transfer addresses or amounts. Neither the
deposit locator nor a successful vault receipt qualifies native venue credit.

## Versioned release and finite scheduling

Version 1's five-role encoding/digest remains unchanged when chain policy is absent.
Version 2 uses six distinct KMS purposes and `CKR2` capsules: Configuration,
Storage, Trading, Broker, Witness and Funds. Storage/trading/broker/funds seeds
must be nonzero, exactly 32 bytes and pairwise distinct, in preparation, recipient
release and runtime construction. The funds public key must match the configured
vault authority. Actually consumed chain policy/key identity contributes to the
application commitment; a caller's hash or `approved` boolean cannot substitute.
Recipient-KMS's 4-KiB plaintext limit remains enforced.

Version 2 may enable **read-only** native/chain qualification. Each scheduler cut
uses the finite NSM-clock lease and independent witness-backed journal; it resumes
durable read credits without resetting/reviving a revoked trading epoch. Account
reads and diagnostics share the existing credit/cooldown/cleanup budget. It may
observe one configured owner deposit per cadence. Every egress/signing step
rechecks the finite lease and sticky stop flag.

The shipping manifest still rejects trading/funding activation, and API risk
admission remains disabled. Scheduler branches for existing admitted mandates
are prepared, not a way to bypass these gates. A new bounded G05 execution policy
and G01 evidence must precede any financial activation.

## Local evidence and remaining gates

Public deterministic **unfunded** fixtures independently built/verified with
Anchor 1.2.0 and web3.js bind all four rails, optional compute limits, customer
deposits and u64s above JavaScript's safe-integer range. They are codec vectors,
not deployment wallets or live signed transactions.

Joined tests use the real journal/controller/codec/signatures/recognizers with
synthetic RPC and clock. They cover ACK loss, restart, retained original wires,
failed simulation/excess fee, missing history, release/return/payout, deposit-only
debit, authentic owner-deposit recognition, bad receipts and no duplicate credit.
Loopback TLS, bounded RPC and bytecode-stream hostile cases cover trusted ports.
The full local SBF/browser suites separately test program and transport contracts;
they do not turn injected native settlement into live evidence.

G01 still needs live lending setup, deposit-linked native credit, original native
withdrawal correlation, complete fill cuts and precise funding/boundary semantics.
Complete recovery additionally needs native fencing/final cut and actually returned
backing. These remain named blockers; [the runbook](../operations/live-qualification.md)
defines the fixed local checklist and bounded next hardware step.
