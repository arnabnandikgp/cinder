# Fresh paper / Nitro attempts — 2026-10-21

**Both runs safely closed; assembled financial lifecycle still incomplete.**
Run 1 failed before allocation. The separately approved corrected run passed
actual SDK opening/fill attribution, but Cinder refused its close. The last
section records that partial result and separately authorized test teardown;
neither run qualifies a full financial lifecycle. [Run scope](../implementation/P40-PAPER-RUN-PROPOSAL.md),
[paper contract](../architecture/0044-paper-bootstrap.md).

## Run 1 — frozen artifact and setup

- Source: `79f238a4534a5fd1b0c5f4d292b48301b2272f76`; all eight hosted jobs passed.
- Rust 1.97.1; default-feature, source-only, network-disabled ARM64 build,
  rebuilt ELF-identically. Tool image:
  `sha256:33107aa751a9d69fb65acb26474fe3954dd30f8580b4150b4a63daf58bac434a`.
- ELF: `81b590393ad662cd361fa7d3c601af19a60d3ae89e3fcd99fec7723136f50879`.
- Public manifest SHA-256:
  `b5bc8eb186c55c7cf8c2f8b0579a62c025174a2503a27211e57f9405b99f8e0f`.
- Domain-separated manifest digest:
  `74ba158c148ddabc15b69c82c6cc96cefafedbb5a7671a12bd2ff7b7be618635`.
- EIF: `711a0841563d946b4b21cb9a722c8472e0de53283b565bc1e82fd0a6f112e779`.
- PCR0: `5bbcb5d52e6d1ea9251c5af50accf86d8d5fd4dcd416e4b58e58b736e98e876eb593bbd59b468e3ce2a1a994b0d4194c`.
- PCR1: `3b4a7e1b5f13c5a1000b3ed32ef8995ee13e9876329f9bc72650b918329ef9cf4e2e4d1e1e37375dab0ba56ba0974d03`.
- PCR2: `9775c6bdeadc74a78900f4d689abcdfb7b27a0cec09dcdce8f37cd6e55b22d4750f4ef3f06a267bbdf8dd769ef8f70c9`.

One `c6g.large` in us-east-1, instance `i-016ecd7063fd3b0ed`, one non-debug boot.
Nitro CLI 1.5.0, Docker 25.0.16, Amazon Linux glibc 2.34. Independent EIF
inspection checked CRC/version/entrypoint/measurements, and the downloaded image
digest was checked locally. All seven KMS policies and the parent inline policy
were read back with exact PCR/context/tag bindings. Actual parent plaintext-KMS
decrypt and witness-read probes were denied. The enclave released all seven
roles, initialized protected history, and retained four authenticated native
responses. Private role plaintext and owner seeds never reached the parent.

The local VPN alternated between two observed egress addresses. SSH ingress
was corrected from one /32 to those two /32s, port 22 only; no open CIDR or public
application listener was introduced. This changed no enclave artifact or policy.

## Exact failing input

Fresh faucet and sole-agent registration succeeded. Account cash and available
margin were both 1,000 paper USD, with zero positions/orders/fees/funding/PnL.
All four enclave bootstrap queries returned HTTP 200. Each history family
returned this literal shape:

```json
{"data":[],"page":{"asOfSlot":0,"coverage":"unknown","endSlot":0,"hasMore":false,"nextCursor":null,"startSlot":0}}
```

`paper::initialize` incorrectly required a positive funding/risk `end_slot`.
Its old implementation also created generic unresolved raw inputs for these
empty histories and tried to resolve them through a positive source cut. A
fresh zero-slot account cannot supply that cut. The generic resolver's safety
check must not be bypassed or satisfied by inventing slot 1 or a timestamp.

Startup therefore refused before posting the proposed 20/980 split and before
exposing a listener. Two fresh read-only SDK connection attempts failed; no
order intent, native dispatch, allocation or customer economic event occurred.
Node/Chrome authorization, actual SDK open/fill/close, same-image SDK restart
and live post-fence refusal are **not qualified by this attempt**.

## Retention, usage and closure

Trusted offline audit authenticated all 11 accepted frames and verified equal
replicas: 22 ciphertext objects, 185,830 bytes. It found the four HTTP 200
originals and zero financial input events. The stopped history's witness was
independently fenced from epoch 1 to 2, preserving sequence 10/hash; this is not
a live SDK fencing test because the application had already exited.

Native totals: three signed setup/cleanup requests (one faucet, one register,
one revoke), two operator account reads, three public reads, four enclave
account/history reads, **zero orders**. Final account cash remains 1,000 with
zero positions/open orders/PnL/fees/funding. The original revocation ACK confirms
removal of the sole agent; the final account omits the agent inventory field,
which is recorded as omitted, not silently decoded as an explicit empty array.

Host ran approximately 20.5 minutes. At the checked $0.068/hour rate, compute
alone is approximately $0.023; this is not a final all-in invoice. Cleanup was
independently verified at 11:16:32 UTC: instance terminated, EBS/buckets/table/
roles/SSH/security group absent; seven KMS keys PendingDeletion with seven-day
scheduling. Preexisting user policies were preserved. The $5/four-hour/eight-boot
limits were not exceeded. No real funds, transfers, RPC or Pacifica activity.

Ignored `work/experiments/p40-paper/run-1/` retains original private native
requests/responses, the exact artifact/measurements, public failure receipt,
encrypted replicas, trusted audit, and resource/request/cleanup ledger. Tools
remain in its sibling `tools/`; neither is a checkout or CI prerequisite.
Subsequent source corrections need a newly measured, separately scoped run.

## Local correction, not hardware qualification

Implementation `a7b1263c6fd265b2cb4e5b37b0fefb0045dbc6b8` first reproduces the
zero-slot refusal, then uses policy-bound evidence-only retention before the
one-shot paper allocation. It does not relax generic raw resolution. Ordinary
empty funding/risk ingestion still remains unresolved, including on an active
paper book. Partial/continuing/nonempty/malformed pages and positive retention
floors refuse allocation. The joined signed-handler/native-adapter tests now
use the observed zero-slot startup shape, not synthetic slot 100.

Local gates: 74 BULK tests, 26 optimized unit-runtime tests, full optimized
all-feature workspace 834 passed / 10 allowed ignored / zero failed, strict
all-target/all-feature workspace Clippy, formatting, whitespace and 97 repository
contract tests. Those local checks alone are not hardware qualification; the
separately approved corrected-image run is recorded below.

## Run 2 — corrected bootstrap, actual SDK open/fill, close refused

Frozen source: `e72484f9521e86cd8bb38bc013f8e31d0f6d4e71`. All eight
hosted jobs passed before launch. CodeRabbit reviewed that exact successor with
no actionable findings. This is separate authority/evidence from run 1: fresh
owner, agent, customer, capsules, storage keys, cloud resource instances and
history. Exact existing policy templates were reverified; only previously deleted
resource **names**, not identities or history, were reused with `CinderRun=2`.

- Source-only, network-disabled default-feature ARM64 build, rebuilt
  ELF-identically using the same pinned tool image as run 1.
- ELF: `a1907e2787b5a915e0c60c93be4391fcf7515fbaa5c88d3c5e25a61e529fecbe`.
- Public manifest SHA-256:
  `4d8963a5fc065ac25b3440b3a8d8d1d4ee39a509d99c100d41b2cb09537a46fb`.
- EIF: `c059126ee37053badea2d44f58aad1e0e0cbdba793cebc097ad46dad8b846ed9`.
- PCR0: `f9c111769ee8fe2992060a53f5d54002ead7c4f17e568a05b154f3441b6741a7800ef7e05c3e177c2a1a2c2571618b69`.
- PCR1: `3b4a7e1b5f13c5a1000b3ed32ef8995ee13e9876329f9bc72650b918329ef9cf4e2e4d1e1e37375dab0ba56ba0974d03`.
- PCR2: `4d28f74da4a1593a5e605ebbdef1f3ab0a0fb192510f77690ceeb3e3cc13f0315c208589abee226e81d16bdda53e3607`.

One `c6g.large`, `i-0464d76017e14eadd`, two non-debug boots of that exact image.
Independent EIF inspection, exact PCR/context/tag policy readback and actual
parent plaintext-KMS/witness refusals passed. All seven roles were released.
The corrected zero-slot bootstrap succeeded: observed 1,000 paper USD was split
once into customer 20 / house 980, not inflated to 1,020. Pacifica stayed empty,
with zero native connections or financial activity.

| Criterion | Observed result |
| --- | --- |
| Confidential SDK/private access and initialization | Passed on actual Nitro; customer/house split matches the same native backing |
| SDK order → enclave/native agent → actual fill | Passed: one 0.00003 BTC buy at 94,500, attributed as 3,000 lots to the original customer/unit |
| Cinder opposite-side close | Refused with `unavailable`; lookup returned not-found. No native close was dispatched through Cinder |
| Same-image protected restart | Existing customer cash and filled position survived without reallocation; this is partial restart evidence, not completion of the entire planned receipt/admin suite |
| Independent exact-head witness fence | Epoch 1→2 preserved sequence 48/hash; actual SDK access to both units refused |
| Encrypted persistence | 98 objects / 1,511,032 bytes, identical replicas, all 49 frames authenticated offline |
| Full Node/Chrome grant/revoke and final cash bridge | Not completed in this run; no pass inferred from older images or local fixtures |

### Exact active-account incompatibility

The retained original account bodies were decrypted only on the trusted local
operator machine, then passed **unchanged** to the frozen implementation's
`financial::account::decode_account` using the actual prepared profile.
Three empty/pre-trade account responses decode; six subsequent active-account
responses return `Invalid`. Reported `availableMargin`, `executionImpact`,
`transferableBalance`, `marginUsed`, `bufferedMargin` and `notional` contain
nonzero fractions finer than the configured eight-decimal quote scale. Four
later responses also contain inexact, nonzero cumulative funding.

`observation::Kind::Account` therefore retains unresolved input rather than
treating these reports as qualified account evidence. Separately, the active
paper policy requires zero funding; even a representable nonzero funding report
would remain contained. Increasing a network timeout, restarting or restoring
access does not repair either financial dependency. A lost local SSH tunnel
also caused two earlier cleanup connection failures **before any new intent**;
once restored, the actual SDK close still refused financially. Do not confuse
that transport failure with the reproduced decoder/containment result.

No amount was rounded away, native balance delta booked as a settlement,
unresolved input erased, funding cut invented, or admission guard bypassed.
The earlier joined fixture used exact integer account metrics and did not cover
these actual active-account reports. Next work must separate reported valuation
precision from exact settlement amounts and establish funding reconciliation;
it must preserve the original raw bytes and explicit policy boundaries.

### Separate paper teardown, not a passing Cinder close

The enclave was fenced and stopped first. The original agent was revoked before
the 30-minute financial window expired. The user subsequently approved a separate
five-minute **teardown-only** extension: re-register the same disposable agent,
one opposite IOC close of the confirmed 0.00003 BTC long, then revoke it again.
The local tool retained each original identity/wire before exposure, rechecked
the exact position immediately before closing, and never resent a request.
Only the agent signed the close; the owner signed agent management, not trading.

All three additional signed requests returned HTTP 200. The close filled exactly
0.00003 BTC at 42,000; final native state is flat with no resting orders, and
revocation is acknowledged. Total native limit notional is 4.095 paper USD
(2.835 opening + 1.260 close), below the original 20 USD cap. Final reported
native cash is 998.424968; realized PnL -1.575, fees zero, cumulative funding
-0.000031115. These are original native reports, **not** a reconciled final
Cinder ledger or a successful shipping fallback. No final Cinder cash bridge
was claimed or manufactured after fencing.

Totals: seven signed native requests, including the one enclave opening and
three separately approved teardown signatures; seven operator account reads;
three public reads; 20 enclave native account/history responses. Actual native
TLS connection count 21 includes the opening. Parent metadata counts are S3
293/194, witness 1,321, KMS 14 and Pacifica zero. The offline archive has ten
financial input records; that count is not ten independent fills or proof of
terminal reconciliation.

### Verified closure and reproducible next action

Host launched 11:42:39 UTC; termination was verified by 12:27:07 UTC. At
12:31:10 UTC independent teardown checks confirmed instance termination and no
remaining test EBS, buckets, witness table, runtime roles, security group or SSH
key. All seven **run-2** KMS keys are PendingDeletion with seven-day scheduling;
preexisting administrator policies and run-1 pending-deletion keys were not
altered. Local opaque relay/tunnel listeners are stopped. Ciphertext evidence
and private diagnostic originals were preserved before deleting cloud copies.
No mainnet/customer assets, transfer, Solana RPC or Pacifica action occurred.
AWS billing is delayed; this is resource/usage evidence, not a final invoice.

Ignored `work/experiments/p40-paper2/run-2/` retains the frozen artifacts,
original native/SDK requests, ciphertext, exact-byte diagnostic receipts,
separate teardown extension and cleanup ledger. Its sibling `tools/` retains
preparation, archive/audit/diagnosis and bounded teardown scripts for review;
the consumed invocation must not be replayed. No code was changed mid-run.

**P40 remains in progress.** First resolve the active-account observation and
funding cases locally, then review the bounded financial policy and remaining
SDK close/cash-bridge/admin/restart acceptance. A new measured run needs its own
authority; neither existing one-host approval permits another host. No stack
merge, third-venue work or repository sanitation was performed.
