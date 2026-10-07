# Next P23 financial-evidence qualification

Prepared 2026-10-07; scope approved 2026-10-08. **The test bounds and bootstrap
exception are approved. Two approved exact invocations stopped before financial
activity: a signer-locator defect, then a malformed loader constant. The corrected
replacement requires fresh exact approval.** See the
[invocation receipt](P23-NATIVE-SEMANTICS-RECEIPT.md). This scopes
the existing G01/G05 gates; it adds no phase or product
promise. [Financial map](P23-FINANCIAL-GATES.md),
[native contract](P23-NATIVE-EVIDENCE.md),
[measured capture](../architecture/0028-measured-native-capture.md).

## Purpose and order

Qualify the native facts required by the Controller **before** enabling shipping
funding. Reuse the historical funding-round-trip scenario and instruction/signature
assertions, not its keys, balances, deployment, weak amount/time matcher or old
approval. The changed shipping image remains separately unqualified.

| Step | Evidence to retain | Acceptance / containment |
| --- | --- | --- |
| Setup | Fresh account baseline; authenticated settings, debt/interest and disable-lending result/readback | Null/default/missing cache is not ready. Record whether safe pre-deposit setup is possible; never construct `Setup.complete` from one successful GET. |
| Deposit | One original persisted signature, finalized exact native-program/mint/recipient effect, account-transfer observation and related native history | Join by the original signature, not amount/time. Establish operation-specific total/fees and any real causal evidence; missing evidence leaves transit/holds unresolved. |
| Withdrawal | One original persisted UUID and request; retained successful native ACK/batch, pending/confirmed transfer, independent finalized broker token receipt | Gross, fee and net must reconcile. Establish operation-specific terminality and provenance. A batch, ACK or wallet delta alone cannot create a native cut. |
| Lost native reply | Discard the response before controller acceptance, preserving a separately identified private audit trace; inspect native reconciliation interfaces | No second POST. A private audit copy can explain/contain the experiment but is not a durable controller ACK or a shipping recovery capability. If native UUID/batch correlation cannot be reconstructed, report the named blocker. |
| Closeout | Actual venue/broker/owner assets, fees, debt, interest, pending transfers, positions, orders and agents | Reconcile every original operation. An unresolved movement is reported and retained, not declared cleaned up or final. No customer credit or recovery activation in this semantics probe. |

Finite observations alone cannot prove that an arbitrary future effect is
impossible. Provider acceptance must identify the supporting native protocol or
documented idempotency/finality contract and the assumptions it needs. If those
are absent, bring the smallest required policy/interface change for review rather
than assign an arbitrary timestamp, counter or Solana slot as a native frontier.

## Approved scope (2026-10-08)

| Boundary | Approved ceiling |
| --- | --- |
| Environment | Pacifica paper/testnet and Solana devnet only; fresh disposable identities, no existing venue wallets, mainnet or customer assets |
| Quote exposure | At most 20 faucet USDP in total; at most 2 USDP total native fees; verify current mint, units, route and fee before any submission |
| Devnet sponsor | At most 0.10 devnet SOL from the configured Solana CLI wallet, only after explicit approval and cluster verification; no new Cinder program deployment in this semantics probe |
| Actions | One native deposit and one withdrawal; bounded original chain funding/return transfers; at most six disable-lending requests, each separately persisted and only following definite no-effect rejection evidence. Expiry alone is insufficient. No orders, agents, loans or subaccounts |
| Bootstrap exception | If pre-deposit disable is rejected, permit one qualification-only initial deposit, then disable/read back within 120 seconds of submission. No trading/readiness while default lending may be enabled. On failure, contain/reconcile under the original identities rather than continue testing |
| Bounds | 20-minute hard deadline from the first native action; at most 200 native HTTP, 400 RPC and eight finite WSS connections, at most 30 seconds/256 messages/1 MiB payload each; include cleanup calls in the budgets |
| AWS | No AWS resources or spend in this native-semantics probe. It is not evidence of enclave execution, parent confidentiality or shipping financial activation |
| Unknown outcome | Persist exact request/wire before exposure; no automatic POST retry, re-signing, new UUID, replacement operation or unapproved rescue transfer |

These are approved test ceilings, not measured costs or production limits. The
120-second bootstrap exception was approved explicitly for this probe, not
inherited from M1 and not adopted as the shipping deposit policy. The source-bound
[one-shot tool](../../tools/native-qualification/README.md) still refuses live
requests without approval of its exact run seal.

## Before execution

1. Review the relevant existing experimental runner and replace historical
   correlation/assumption paths with explicit unresolved outcomes. Run its bounded
   offline scenarios; do not inject synthetic certificates into the shipping code.
2. Seal exact fresh source/tool/lock hashes, account/role identities, native
   origins/CA, devnet genesis/RPC, quote mint/precision/program/vault, sponsor and
   budgets in the G05 execution record. Keep keys, signed wires and private traces
   out of Git. A fresh namespace alone is not approval.
3. Obtain approval of that exact execution record, including the bootstrap
   exception. Stop if a prerequisite fails or a cap/assumption changes.
4. Publish the sanitized native-semantics receipt and disposition of each gate.
   Implement only justified trusted providers, then their prepared-I/O-completion
   wiring and narrow measured financial policy. The current manifest still
   rejects funding/trading; do not bypass it with a diagnostic helper.
5. Prepare the separate current-source Nitro/devnet full vault round-trip run,
   then bounded order/funding and fenced recovery qualification. Its deployment,
   cloud, financial and cleanup authority must be separately sealed and approved.

The local probe narrows uncertainty without paying for another exploratory AWS
session. It does **not** replace P23's actual measured workflow acceptance. No
native request, new program deployment or funds transfer has occurred in either
stopped invocation; the second made three read-only devnet RPC calls. Preparing
the replacement and passing offline tests do not
establish native finality, lost-reply recovery or a changed-image hardware pass.
