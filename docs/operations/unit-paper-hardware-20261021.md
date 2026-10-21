# Fresh paper / Nitro attempt — 2026-10-21

**Failed before allocation and listener startup; safely closed.** This is an
exact-source failure receipt, not an SDK trade, successful financial lifecycle
or qualification of the subsequent local correction. [Run scope](../implementation/P40-PAPER-RUN-PROPOSAL.md),
[paper contract](../architecture/0044-paper-bootstrap.md).

## Frozen artifact and setup

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
contract tests. No source from this correction has been deployed by this receipt.
