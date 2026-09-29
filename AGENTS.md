# Cinder TEE contributor guide

This worktree is the clean private-broker product, not the Phoenix/PER proof of
concept. Pacifica is the first venue; AWS Nitro is the TEE target. Do not import
legacy code, account invariants, toolchain pins or website work by default.

## Resume safely

1. Confirm worktree, branch, `git status` and current user instructions. Preserve
   unrelated changes and ignored local research; never stage the whole directory.
2. Read [TRACKER](docs/implementation/TRACKER.md), the selected phase of
   [PLAN](docs/implementation/PLAN.md), and [BASELINE](docs/implementation/BASELINE.md).
3. Consult [EVIDENCE](docs/implementation/EVIDENCE.md). Local `work/` sources are
   optional provenance, not a prerequisite a fresh checkout must magically have.
   Promote needed sanitized contracts/tests in the owning phase. If a necessary
   detail is missing, report it; do not invent it or import a historical venue rule.
4. Implement one bounded phase on its own branch. Keep its scope, dependencies,
   test evidence, deviations and exact next step current in the tracker.
5. Run offline checks before pushing. Do not call a phase closed until its
   acceptance criteria pass, relevant reviews are resolved and its PR is merged.

## Non-negotiable boundaries

- One authoritative financial state and journal. Research projections are not
  separate ledgers whose assets can be added together.
- Private individual entitlements over pooled execution; no internal crossing,
  per-user native accounts by default, or hidden customer-loss mutualization.
- Exact units, signed basis, explicit rounding and checked arithmetic. No
  floating-point money, invented precision or uncollectible debt counted as cash.
- Persist operation/attempt identity before signature exposure. Commit postings,
  consumed-event keys and reservations atomically. A timeout is not a rejection.
- Real adverse events must be recorded even when admission would reject them.
  A cancel acknowledgement is not proof all fills have arrived.
- Separate customer assets, house capital, coverage commitments and liquidity.
  Never count reserve designation as new assets or loss absorption as a second loss.
- RF1/RF2 economics and their limitations are approved; do not reopen them casually.
  Numerical limits, final coverage/priority and production proof targets are not
  approved merely because the accounting examples pass.
- Recovery is operator-assisted. No heartbeat-only payout, stale claim root,
  competing ordinary payout path or assertion that a Merkle proof proves solvency.
- Encrypted external persistence needs authenticated freshness. Nitro has no
  durable private disk. Host plaintext must not carry private customer data or keys.
- Native account authority is a capability boundary. An app's trading-only policy
  cannot narrow a venue credential that natively permits money movement.

## Permissions, evidence and reviews

- Offline fixtures are the default. Live venue tests, devnet funding/deployment,
  AWS use and mainnet/customer funds are distinct approval gates. Check the
  current authorized manifest; historical experiment permission is not blanket
  authority for implementation deployments. Never print credentials/signed wires.
- Stop for a material new economic/security decision; record it in the tracker
  with alternatives. Do not expand scope to make a milestone appear complete.
- A review/check request does not authorize fixes, deployment, merge or messaging.
  Respect the user's current request and available tool permissions.
- No automatic subagent delegation. Use a review agent only when explicitly
  authorized; preserve focused findings and their disposition.
- Label evidence as documented, observed, tested, derived, assumed or unresolved.
  Passing tests are not a machine proof, independent audit or mainnet qualification.
- Normal PR checks must work offline without `work/`, venue APIs or RPC secrets.
  Linux/native/hardware tests belong in the phases that actually need them.
- Default TEE trunk: `product/tee-v1`; never target legacy `main` accidentally.
  Follow the shallow, bottom-up stack process in PLAN. No automatic merges.

## Workspace checks

Use Rust 1.97.1 from `rust-toolchain.toml` and Node 24.21.0 from `.node-version`.
See [development setup](docs/development.md) and
[ADR 0001](docs/architecture/0001-workspace.md). Do not install/replace global
toolchains implicitly or inherit the legacy Phoenix versions.

```sh
node scripts/check.mjs
node scripts/check.mjs --properties
git diff --check
```

The full runner already includes the property tests; the focused command is for
iteration, not an additional proof. Cargo commands are locked/offline after
toolchain installation. Financial conformance remains P02/P03 onward.

Routine PRs use pinned local macOS checks and hosted Linux CI; a local Linux
container run is not a pre-PR requirement. If CI fails, inspect its logs, test
the proposed fix locally and allow two evidence-based fix-and-push attempts
before falling back to local Linux reproduction if the failure persists. Do not
make speculative pushes just to consume that allowance. Explicit Linux/SBF/Nitro
qualification required by an owning phase remains a separate acceptance gate.

Keep the kernel dependency-free and `no_std`; no I/O, clocks, venue SDKs or test
doubles may leak into it. Extend the dependency guard only with an explicit
architecture decision. Test-support is not a production service or durable store.
For Linux checks, export only intended tracked/staged source, mount it read-only
with networking disabled, and write build outputs to container-local temporary
storage. Never mount ignored research or wallet/cloud directories. Use Apple
containers if a Linux test is needed; follow the available container skill.
