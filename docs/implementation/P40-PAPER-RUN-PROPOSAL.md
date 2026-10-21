# P40 fresh assembled-paper / Nitro run proposal

Status: **not approved for live execution**. The user approved implementation of
the [paper-only accounting assumption](P40-PAPER-GATE.md), not a new AWS session.
Finish offline review/checks and pin the final committed source before requesting
this run. No resource, wallet, faucet or native request is created by this document.

## Combined scope

One fresh, exclusively controlled BULK paper owner and one separate native agent;
one disposable Cinder customer. Original owner setup, faucet and revocation stay
on the trusted operator machine; only the agent enters the enclave. No existing
wallet, mainnet/customer funds, Solana RPC, bridge, transfer, withdrawal or payout.
Pacifica stays read-only; do not reopen missing-credit investigations.

Use one fresh Nitro host in `us-east-1`, profile `cinder_new`, **$5 / four hours /
eight boots maximum**, cleanup afterward. This combines the reviewed startup
corrections, version-9 paper bootstrap and actual SDK/native lifecycle into one
image/session. Old P40 resources, identities, policies, budgets and PCRs are closed
historical evidence, not authority for this run. Verify current IAM identity,
prices/quota and fresh exact resource policies before launch.

Fresh local permission templates and administrator instructions are in ignored
`work/experiments/p40-paper/permissions/README.md`, with `runtime-boundary.json`
and `operator-permissions.json` alongside it. Only after approval, create
`CinderP40PaperRuntimeBoundary` without attaching it to the user and create/attach
`CinderP40PaperOperator` to `cinder_new`. Read back and compare the actual policies
before use. These local files have not created or attached any AWS policy.

## Frozen inputs before key release

Local implementation receipt: `2f565223f4310a211ef1671f65414eb9fb38ca9c`.
This identifies the checked code, not a qualified image or final live-run source.
After review, freeze the exact deployment commit and retain any documentation-only
successors separately; requalify application changes rather than reusing an old ELF.

- Exact reviewed code commit, locked dependencies, pinned Node/Rust, clean
  source-only default-feature ARM64 package and reproducible enclave ELF.
- Fresh run namespace/resource ledger, seven distinct KMS roles, two ciphertext
  stores, independent witness and fresh storage generation/key. No private role
  plaintext or owner keys on parent/image; inspect the actual EIF/PCRs.
- Original new account/faucet/agent records, native market grids/minima and recent
  quote captured through authenticated official testnet endpoints. Retain originals;
  bind setup/quote digests and the actual native profile into the private policy.
- Typed version-9 preparation through `cinder-prepare-release`, one managed owner,
  Pacifica Unknown/zero dispatches, BULK qualified agent, three-location inventory,
  at most 20 USD test allocation FROM actual faucet backing, explicit finite risk
  settings, fixed 30-minute paper expiry and all wrapped plaintexts <=4096 bytes.
- The local joined test uses tiny synthetic prices. Never copy those prices,
  fixed seeds, roots, account IDs, balances or fixture preparation into this run.

The native setup/quote capture and final public manifest must be staged late
enough that initial private observations and the fixed expiry remain valid.
Do not renew expired policy or accept stale evidence to salvage a boot.

## Finite native budgets

| Resource | Maximum / treatment |
| --- | --- |
| Native environment | Only `https://exchange-api.bulk.trade/api/v1` testnet |
| Financial activity window | 30 minutes; all native paper actions/cleanup within the fixed window |
| Faucet | One original attempt; use actual credited amount, not an assumed 1,000 |
| Signed requests | Eight total: faucet, one agent registration, up to five order/cancel requests and agent revocation |
| Order attempts | Five history-wide, including native cancellation and cleanup |
| Cumulative limit notional | 20 USD including separately reserved cleanup; actual initial trade should be much smaller |
| Unsigned account reads | 80 total, including setup, four bootstrap reads and up to eight worker reads per permitted boot |
| Public metadata/quote reads | Four, retained, no account secrets |
| Worker | 30-second poll interval, at most eight reads/boot; zero Pacifica dispatches; five BULK dispatches/history |
| Gateway credits | Explicit allocation sufficient for four bootstrap reads + 8×8 reads + five dispatches + cancellation reserve; no budget reset on restart |

The external run ledger must refuse reboot/action if its remaining request or
time allowance cannot cover the planned work and cleanup. These maxima are not
targets. Prefer two healthy boots; do not keep polling or repeat originals after
unknown outcomes. If current minima/grids cannot fit admission and cleanup,
stop without opening exposure; do not enlarge the notional or capital allocation.

## Acceptance sequence

1. Verify fresh funding/account/agent setup and public market metadata. Freeze the
   actual private policy, source/artifacts, public manifest and inspected measurements.
2. Boot without debug; qualify attestation/KMS, private TLS egress and the protected
   shared journal. Four original native reads establish only the approved paper cut.
   Confirm `native F = customer C + house (F-C)` and Pacifica remains empty.
3. Through actual encrypted UnitClient, read both units and exercise scoped access.
   Place one tiny bounded order, wait for retained native fills and correct original
   customer attribution, then an affordable opposite-side reduce-only IOC close.
   Use fewer than five requests whenever possible; preserve cleanup before opening.
4. Verify native/customer quantity flat, actual cash/fees/PnL bridge, duplicates
   once-only, original operation lookup and still-uncertain holds honestly. Do not
   fabricate terminal history or call this a native funding/recovery qualification.
5. Same exact image/capsules/history restart: no reallocation, original receipt and
   shared epochs preserved. Exercise Node/Chrome encrypted access; revoke Cinder
   grants after trading. Finally fence through the independent witness and verify
   both units refuse. Complete native agent revocation within its separate allowance.
6. Collect/authenticate encrypted replicas, exact source/ELF/manifest/EIF/PCR receipts,
   native originals, request/resource usage, final known exposure and all unknowns.
   Terminate host and verify all owned resource cleanup; retain evidence, not secrets
   in tracked files. Report failed/unobserved criteria rather than filling them in.

Abort on unqualified setup, expiry, funding/risk/position discrepancies, original
request uncertainty, lost freshness, changed source/measurements or insufficient
cleanup allowance. An uncertain order is not resent. A failed live criterion
remains open; later policy/image/run changes require a new bounded proposal.
