# Native funding semantics qualification

This **local diagnostic tool**, not an enclave component or financial provider,
implements the approved [P23 scope](../../docs/implementation/P23-NATIVE-QUALIFICATION.md).
It uses the existing vault client's locked legacy codecs at an explicit boundary;
no packages, shipping dependencies or program deployments are added. There is no
ambient live mode, mainnet/AWS path, automatic economic retry or live restart.

## Offline checks and preparation

Use pinned Node 24.21.0. Normal contract CI discovers these dependency-free tests:

```sh
node --test scripts/native-qualification.test.mjs
node scripts/check.mjs --group=contracts
```

The actual driver sequence is tested through independent fake ports. They are
not native/chain evidence. `bindings.mjs` promotes only the public deposit/faucet
ABI from [Pacifica's official SDK](https://github.com/pacifica-fi/pacifica-mcp/blob/4748feca93efe2b2a5a0c95993e40f68d0ca4338/src/idl/pacifica_solana.json):
full IDL SHA256 `a31bb37868338e189fead472c1481954ed876844e83529e2e68b5ef0dbf427a1`.
No historical key, plaintext controller, prototype deployment or amount/time
matcher is imported. The native program address and historical loader metadata
are explicit, revalidated before funding and before every chain submission.
The first preflight streams at most 4 MiB of executable bytes through <=64-KiB
RPC slices, then rechecks the loader header. Later sends require the same loader
slot/authority. This is not verified source equivalence or an audited native image.

Once the scope is approved, offline preparation creates **two fresh disposable
test identities** in a new private namespace. It reads only the configured CLI
sponsor's address/configuration at preparation time, not its secret. It copies
only the supplied devnet RPC URL to private configuration; no public key/secret
from an old experiment is used as a test identity.

```sh
node tools/native-qualification/prepare.mjs \
  work/experiments/p23-native-semantics/run-01 /absolute/private/devnet-rpc.txt
```

Review that directory's `REVIEW.md` and `manifest.json`. Source, lock, TLS roots,
roles, original withdrawal UUID, routes, sponsor transfers and budgets are bound
by its seal. `approval.json` must separately name that **exact** seal, carry
`approved:true`, a meaningful approval reference and a millisecond `not_after`
less than 24 hours ahead. Preparation never creates approval. No live request is
made until the approved runner is explicitly invoked:

```sh
node tools/native-qualification/run.mjs \
  work/experiments/p23-native-semantics/run-01
```

Changing source/configuration requires a new reviewed seal, not reuse of an old
approval. Once started, the owned lock remains and a second invocation refuses.
Counter/head history survives restart; a torn journal requires manual analysis.
Private files are exclusive, owner-only and fsynced before exposure. They are
**not** Nitro-protected or rollback-proof; this tool must never handle customers.
JavaScript/SDK key objects do not guarantee secure erasure. Its private key files
and remaining disposable devnet gas/token-account rent are retained for manual
inspection, not automatically deleted or reclaimed.

## Fixed scenario and honest outcomes

1. Pin devnet genesis; verify native loader metadata, mint and vault layout; reject
   nonempty fresh identities. Require an authentic empty account/history baseline.
2. Attempt lending disable before funding. An unknown response stops. The exact
   historical fresh-account 422 rejection is an explicitly scoped diagnostic
   bootstrap allowance, not a general financial no-later-effect certificate.
3. Sponsor 0.035/0.025 devnet SOL to owner/broker; mint 20 faucet USDP, allocate
   20 to broker, then submit **one** original native deposit. Keep exact wire,
   signature, simulations and final receipts. No Cinder vault is deployed.
4. For the approved exception, disable/read back within 120 seconds of deposit
   exposure; require actual debt/interest observations. Missing cache/default/null
   is unresolved. Failure permits only original withdrawal containment, not study
   trading, another deposit or customer credit.
5. Submit **one** original UUID withdrawal. Retain its response in a separately
   marked private diagnostic audit trace, but withhold it from controller ACK
   acceptance. This is callback-level loss injection, not a packet-drop or process
   crash claim. Never promote that audit ACK as shipping reconciliation.
6. Observe its batch-linked transfer and independently finalized exact recipient
   token delta; return only the reconciled net to the fresh owner. No rescue send
   on uncertainty. Retain fees, actual residual assets/debt/interest and pending
   state; unresolved closeout is not reported as clean.

All native HTTP calls share a conservative 12-second cadence; 429 stops rather
than retries. HTTP bodies are capped while reading. WSS windows are finite,
authenticated through the sealed Node roots, compression/redirects disabled,
with bounded frames/messages/bytes; there is no continuous feed. Native/RPC/WSS
counters, original signatures/UUIDs and a hard 20-minute deadline include cleanup;
study requests reserve 40 HTTP / 80 RPC / two WSS slots for containment.

The native withdrawal API has no signed max-fee parameter. Its documented USDC
fee is $1; **2 USDP is a receipt acceptance ceiling, not a fee guarantee enforced
by our request**. An unexpected fee stops the qualification and remains in the
actual ledger/evidence. The exact run review must expose this dependency rather
than advertise an enforceable fee cap.

The API schema does not establish an operation-complete native frontier or a
UUID lookup recovering an ACK lost before controller persistence. The driver
therefore reports `source_cut:null`, `financial_completion:false`, and unresolved
lost-native-reply reconciliation even when assets return correctly. Exact
signature/batch observations and payment are facts, not permission to erase a
hold or fabricate a credit/cut. A bounded pass cannot prove no future effect.

## Remaining gates

This tool narrows G01/G05 uncertainty. It does not qualify source authentication
inside Nitro, setup atomicity, funding providers, journal-safe financial I/O,
production recovery, native trading/funding, or a full vault/customer round trip.
Missing capabilities go into a sanitized receipt and the smallest policy/API
question for review. Current shipping funding/trading gates remain disabled.

Retain private originals locally; publish only sanitized aggregate results and
approved public provenance. No RPC key, signature/wire, keypair, raw trace,
private operation ID or test wallet journal belongs in a PR or CI log.
