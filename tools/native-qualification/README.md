# Native funding semantics qualification

This **local diagnostic tool**, not an enclave component or financial provider,
implements the approved [P23 scope](../../docs/implementation/P23-NATIVE-QUALIFICATION.md).
It uses the existing vault client's locked legacy codecs at an explicit boundary;
no packages, shipping dependencies or program deployments are added. There is no
ambient live mode, mainnet/AWS path, automatic economic retry or live restart.

## Separate read-only schema observations

`observe.mjs` is a reusable **GET-only** diagnostic, separate from the funding
driver and its closed invocations. Supply an authorized disposable test account
address, a distinct public-only absence probe (no private key or funding), and a
new `work/experiments/p23-native-schemas/observe-NAME` directory:

```sh
node tools/native-qualification/observe.mjs PUBLIC_TEST_ACCOUNT PUBLIC_ABSENCE_PROBE work/experiments/p23-native-schemas/observe-NAME
```

It reads the fixed testnet account/deposit-history/balance-history/settings/loan/
positions/orders routes: at most 12 GETs, 12-second cadence, 12-second request
deadline, three-minute total lifetime and 8-KiB bodies. It stops on HTTP 429,
upstream failure or unavailable/malformed replies, with no retry. It cannot read
wallets, sign requests, change settings, use RPC, move funds or create AWS resources.
Native TLS verification stays enabled; redirects and ambient transport overrides
refuse. Source/TLS/scope metadata and pre-I/O request/raw reply records are retained
in new owner-only files. Existing directories refuse rather than reset a budget.
Stdout contains bounded field/type summaries and hashes, not raw values/signatures.
These sequential public observations are not atomic setup or financial certificates.

The [2026-10-08 receipt](../../docs/implementation/P23-NATIVE-SEMANTICS-RECEIPT.md#separate-current-testnet-schema-observation)
records actual established/absent-account results. Sanitized shapes in
`crates/pacifica/tests/fixtures/demo-native-shapes.json` replace signatures and
timestamps; they are offline regression inputs, not reusable native authority.
No signing-key generation or financial invocation is part of this command.

## Offline checks and preparation

Use pinned Node 24.21.0. Normal contract CI discovers these dependency-free tests:

```sh
node --test scripts/native-qualification.test.mjs
node scripts/check.mjs --group=contracts
```

The actual driver sequence is tested through independent fake ports. They are
not native/chain evidence. `bindings.mjs` promotes the public deposit/faucet
ABI from [Pacifica's official SDK](https://github.com/pacifica-fi/pacifica-mcp/blob/4748feca93efe2b2a5a0c95993e40f68d0ca4338/src/idl/pacifica_solana.json):
full IDL SHA256 `a31bb37868338e189fead472c1481954ed876844e83529e2e68b5ef0dbf427a1`.
No historical key, plaintext controller, prototype deployment or amount/time
matcher is imported. The native program address and historical loader metadata
are explicit, revalidated before funding and before every chain submission.
The first preflight streams at most 4 MiB of executable bytes through <=64-KiB
RPC slices, then rechecks the loader header. Later sends require the same loader
slot/authority. This is not verified source equivalence or an audited native image.
All route identifiers must parse as canonical public keys. An independent fixture
pins the [official loader-v3 ID](https://solana.com/docs/core/programs/program-deployment)
and derives ProgramData rather than copying the implementation's owner constant.
The withdrawal diagnostic checks a single-recipient legacy `batch_withdraw`
instruction against that IDL: program, account order/privileges, recipient,
integer net amount and batch nonce. The HTTP ACK's `batch_nonce` matches the
instruction's batch nonce, **not** its separate `withdraw_id`. Multi-recipient,
versioned or extra-instruction payments are outside this diagnostic's scope.
The caller still owns signature, finality and token-effect verification; the
ABI observation is not a financial completion certificate or lost-ACK lookup.

Once the scope is approved, offline preparation creates **two fresh disposable
test identities** in a new private namespace. It reads only the configured CLI
sponsor's address/configuration at preparation time, not its secret. It copies
only the supplied devnet RPC URL to private configuration; no public key/secret
from an old experiment is used as a test identity.
CLI display padding is normalized before sealing. An absolute, existing,
owner-only regular signer file is required before identities are created;
metadata validation does not read its secret. Execution rechecks the exact
sealed locators before starting and never silently changes a sealed path.

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

`MAPPINGS.md` also states the sealed diagnostic observation contract: documented
single-object transfer messages, perp balance history and observed pending balance.
An empty history page or zero pending balance does not prove complete operations.
The probe does not assume unqualified perp withdrawal-history/pending semantics
or substitute unrelated spot endpoints. Those perp routes are listed in official
MCP documentation/client code; their existence does not establish original UUID
lookup, final native debit or no-later-effect semantics.

Changing source/configuration requires a new reviewed seal, not reuse of an old
approval. On 2026-10-08 the user authorized small test-only corrections/reseals
within unchanged approved devnet/testnet limits without another permission
request. Record that authority on each new seal; it does not authorize economic
retries, uncertain replacements, increased exposure, AWS/mainnet/customer assets,
or custody/security-policy changes. Once started, the owned lock remains and a
second invocation refuses.
Counter/head history survives restart; a torn journal requires manual analysis.
Every original native POST retains its exposure before I/O. Transport, body,
rate-limit or audit-storage failures retain an unknown reply and prevent a second
submission under that identity. A separately retained reply withheld at the
callback is labeled loss injection, not genuinely missing native evidence.
Private files are exclusive, owner-only and fsynced before exposure. They are
**not** Nitro-protected or rollback-proof; this tool must never handle customers.
JavaScript/SDK key objects do not guarantee secure erasure. Its private key files
and remaining disposable devnet gas/token-account rent are retained for manual
inspection, not automatically deleted or reclaimed.

## Fixed scenario and honest outcomes

### Separately approved staged test-capital scenario

Explicitly pass `staged-bootstrap` as the third preparation argument to select
`cinder-native-staged-bootstrap-v1`. An explicitly supplied owner-only JSON
transport configuration with exactly `rpc` and `wallet` fields can supply the
devnet RPC locator; its old wallet value, test identities and approvals are not
imported. The sponsor is resolved anew from current Solana CLI configuration.

```sh
node tools/native-qualification/prepare.mjs \
  work/experiments/p23-native-semantics/run-staged-NAME \
  /absolute/private/transport-config.json staged-bootstrap
```

Review both `REVIEW.md` and `STAGED-REVIEW.md`; the latter replaces the legacy
timing/loss-injection bullets for this distinct manifest. It deposits only
20 faucet USDP of test/house capital, observes positive idle account/loan-cache
initialization (at most four read pairs), then submits **one** lending-disable
request. Sequential settings/loan/account/positions/orders must show disabled
lending, zero debt/interest, no spot exposure or margin overrides and no orders
or positions. Original-signature deposit history plus the full, nonpending
balance page corroborate the original credit; neither grants a financial cut.

The explicit staging window is five minutes, not the legacy 120-second
exception. Total HTTP/RPC/WSS, funds and 20-minute invocation limits are unchanged.
The original withdrawal ACK is retained, not withheld. Original batch/instruction/
finalized recipient checks precede return of confirmed net; closeout requires an
empty disabled/no-debt baseline. Failed setup allows only the planned original
withdrawal containment. There is no signed-setting retry, deposit/withdrawal
resend, customer capital, trading, AWS, customer credit or shipping activation.
Unknown economic execution stops. A fresh, separately bound approval is required;
neither an old seal nor historical bootstrap success authorizes this scenario.

`finish-staging.mjs prepare|run ORIGINAL_STAGED_DIRECTORY` supports a separately
sealed **settings-only** continuation if the closed staged run returned the exact
finalized net, has zero broker tokens/pending balance and never exposed a lending
toggle. It verifies the parent's hash-linked journal and original recipient/token
effects before reading the broker key. Up to ten HTTP calls and one first toggle
share the original remaining 200-HTTP / 20-minute budget; no RPC or financial
action is available. A separate `settings-followup/approval.json` must name the
new scope hash and expire within that original deadline. The new child lock and
exclusive pre-I/O artifacts prohibit restart. The parent's result and journal
stay unchanged. A successful empty-account readback does **not** retroactively
pass the earlier pre-withdrawal staging check or grant shipping setup authority.

### Persistent credit investigation

The separately approved persistent credit investigation can instead prepare
`staged-reliable` or `staged-direct`. They use a distinct sealed delivery schema:
bounded `maxRetries:5` forwarding of the same original signed wire, 60 status polls
and twelve initialization pairs/ten minutes, under the same overall request and
20-minute caps. `staged-direct` mints directly to the broker (official helper
control) and creates an empty owner return ATA. Review `DELIVERY-REVIEW.md` as the
authoritative variant override. Never restart/rewrite historical manifests or
infer credit failure from an unlanded transaction. Independent repeat-round
permission applies only when the current user explicitly grants it; financial
and shipping qualification remain separate.

### Legacy reply-loss scenario

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
