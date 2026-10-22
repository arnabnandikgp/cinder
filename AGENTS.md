# Cinder contributor guide

Work in `cinder-tee` on the `product/tee-v1` workstream, not legacy Phoenix/PER
`main`. Cinder is a private perp broker: individual customer entitlements over
pooled venue execution, Pacifica and opt-in BULK adapters, AWS Nitro runtime.
Website, terminal and Mintlify repositories are separate workstreams.

## Start and hand off

1. Check branch, `git status`, and the latest user scope. Preserve unrelated and
   ignored files; never stage the whole repository.
2. Read [status](docs/status.md) and the current
   [handoff](docs/implementation/TRACKER.md). Consult only the relevant
   [acceptance criteria](docs/implementation/PLAN.md),
   [financial contract](docs/implementation/BASELINE.md) and
   [decisions](docs/architecture/README.md) for the task.
3. Implement one bounded, tested change. Update the owning current handoff with
   exact source, commands/results, unexpected work, PR/review state and next step.
   Do not append another session transcript or overwrite dated evidence.
4. Keep implementation, local tests, exact-image hardware/native evidence and
   production readiness separate. A merge does not close unmet qualification.
5. Merge only with current user authorization and passing relevant checks/review.
   Native stacks target `product/tee-v1`; inspect `gh stack view` before pushing
   or merging. No automatic subagents or unrelated repository-governance changes.

## Code ownership and debugging

| Change / symptom | Start here |
| --- | --- |
| Money, basis, funding, margin, liquidation, ADL | `crates/kernel`; financial contract and ADRs 0002–0012 |
| Replay, duplicate effects, stale reads, missing state | `crates/journal`; ADRs 0005/0006/0025/0026 |
| Owner/agent permission, operation lookup, private views | `crates/api`, `clients/private`; ADRs 0018/0021/0036–0039 |
| Native signing, response/fill precision or attribution | `crates/pacifica`, `crates/bulk`; venue maps and ADRs 0032/0033/0040 |
| Scheduling, enclave startup, unit workers | `crates/service`; ADRs 0042–0044 and current status |
| Browser encryption or opaque relay | `crates/web-channel`, `services/web-relay`, `tools/web-channel` |
| Escrow, funding movement, payment/recovery counters | `programs/cinder-vault`, `clients/vault`, journal/service funds controllers; ADRs 0015–0017/0030/0041 |
| Test discovery, CI or fixture contracts | `scripts`, `tools`, `.github/workflows`; development guide |

## Non-negotiable invariants

- One authoritative journal and financial ledger. Customer assets, house capital
  and suspense stay distinct. No hidden loss mutualization, internal crossing,
  default per-user native accounts or cross-venue collateral substitution.
- Checked integer ledger money, explicit units, signed basis and qualified native
  conversion rules. Reported prices/equity are not cash or basis setters. Never
  invent precision, credit, source completeness or collectible customer debt.
- Persist original operation/attempt identity and commitments before signature
  exposure. Atomically accept postings, consumed IDs and holds. Timeout, cancel
  ACK, not-found and a later balance match are not resend or settlement authority.
- Preserve real adverse observations during freezes; unresolved differences stay
  visible. RF1/RF2 economics remain approved; numerical limits, coverage and
  production calibration are separate decisions.
- Escrow refund and full identified-batch sweep are exclusive. No raw-balance
  sweep, duplicate credit, post-sweep refund or trading before qualified credit.
  Returns use the payout vault, never the auto-swept ingress escrow.
- Ordinary and recovery payments share counters. Recovery requires fencing,
  reconciliation, final claims, available backing and operator activation—not
  heartbeat failure or an old Merkle root. Inclusion alone is not solvency proof.
- Private authentication, accounting, decryption and signing stay inside the
  enclave. Parent storage holds authenticated ciphertext; Nitro has no durable
  private disk. Fresh-head/rollback protection and writer fencing are mandatory.
- Native credentials retain their actual powers. Keep trading, funds, storage
  and recovery authority explicit; app policy cannot narrow a stronger native key.
- Legacy and opt-in unit histories are not silently migrated or substituted.
  Separate paper manifests/history cannot become production backing or reopen
  after expiry. A fixture never supplies shipping readiness or attestation trust.

## Verification

Pins: Rust **1.97.1**, Node **24.21.0**; isolated Anchor language/CLI/SPL and
`@anchor-lang/core` **1.2.0**. Do not replace global tools implicitly.

```sh
node scripts/check.mjs --group=contracts
node scripts/check.mjs
node scripts/check-vault.mjs
git diff --check
```

Use focused checks while iterating, then the relevant complete groups. Cargo is
locked/offline after explicit hydration; the kernel stays dependency-free `no_std`.
See [development](docs/development.md) for separate browser, SBF and history-growth
jobs. Never deploy `test-hooks`, fixture trust or insecure storage. SBF tests use
`NO_DNA=1`, local Surfpool and disposable keys, not a configured wallet.

Routine macOS tests plus hosted Linux CI suffice. Allow two locally verified,
evidence-based CI fixes before a Linux reproduction. If containers are needed,
use the container skill/Apple containers: source-only read-only inputs, no
network, wallets, cloud configuration, ignored research or unrelated user files.

## Authority and documentation

Offline fixtures are the default. Every live venue/RPC/devnet/AWS run needs
current bounded approval; consumed experiments do not renew it. Mainnet/customer
funds, material economic/security changes and production release are separate
gates. Never print credentials or signed wires.

[Documentation guide](docs/README.md) identifies current contracts and archived
history. `work/` is optional ignored research, never a checkout/CI prerequisite.
Promote only necessary sanitized requirements/fixtures. Label evidence as observed,
tested, assumed or unresolved; tests are not an audit or unconditional guarantee.
