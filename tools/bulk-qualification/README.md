# BULK native qualification, not activation

First stage: a fresh **two-request unsigned metadata preflight**, with no wallet,
signature, faucet, order, transfer, Solana RPC or AWS access. This is inside the
larger proposed 12-public-read allowance, not a reason to spend that allowance.
No dependency installation or historical account reuse is needed.

Use pinned Node 24.21.0. `read.mjs --prepare /absolute/new/private-directory`
creates a source-bound manifest without network I/O. Current explicit approval
is recorded separately in private `approval.json` (0600): manifest commitment,
`approved: true`, exact user-reference and `not_after` milliseconds no more than
two minutes ahead. Preparation is not approval. `--run-approved` checks source,
scope and private permissions, retains each request durably before GET, and
archives bounded original response bytes losslessly before interpretation.

Only official paper-testnet `/exchangeInfo` and `/feeState` are allowed. No
redirect, retry, alternate host, cookies or credentials. Failure stops the run;
status 2 means blocked/non-success, not a qualified financial capability.
Never rerun a started directory or copy its approval to a new run.

CI runs `scripts/bulk-qualification.test.mjs` entirely with injected responses;
it does not call a live endpoint. Exact raw metadata is evidence for reviewing
current market rules, not an account balance, precision qualification or risk
readiness. No observed public metadata activates the production Gateway.

The signed paper lifecycle remains a separate stage under
[the proposal](../../docs/archive/qualification/UNIT-QUALIFICATION-PROPOSAL.md). Reserve
an explicit close/cleanup attempt when planning an opening trade: two attempts
cannot promise a resting-order probe **and** an opening IOC **and** a closing IOC.
Fresh signing identities, original native messages/nonces and known cleanup
must be retained before any exposure. Old scratch SDK scripts and faucet funds
do not authorize a live run or qualify Solana funding/returns.

## Disposable paper lifecycle

`paper.mjs` is the separate native test driver. It is **not** an enclave service,
shipping signer, master-key fallback or a Cinder financial journal. Setup uses
only three freshly generated private keys (one owner, two agents) under a new
0700 directory. It never loads configured wallets, existing credentials or an
RPC/AWS client. Disposable setup keys are intentionally local; they are not
evidence that production custody/private keys may leave the enclave.

Build the offline codec bridge with
`cargo build -p cinder-bulk --example paper-codec --locked --offline`. It calls
the actual `wire::Action` and `fills::rest` implementations, has no network or
key input, and supports only BTC-USD ALO/IOC/cancel and fill interpretation.
The driver uses Node's standard Ed25519 implementation and the documented
faucet/agent binary layouts; no upstream package/native addon is installed.

With current user authority, initialize using
`node tools/bulk-qualification/paper.mjs --init /absolute/new/private-directory 'approval reference'`.
Then execute individually named steps using `--step DIRECTORY JSON_SPEC`.
Supported categories are `public` (info/book), `account` (fixed own-account
families), and `signed` (faucet, fixed agent add/remove, agent-only limit, or
cancel bound to a retained original order). No transfers or arbitrary action/URL
exist. The source/binary seal must still match. Limits are 14 signed requests,
24 account reads, 12 public reads, five order attempts and $20 cumulative order
notional including cleanup, in 30 minutes. These are ceilings, not targets.
Read-only offline tests inject responses and never run the CLI against a venue.

The runner enforces the pinned Node version and verified TLS before private-file
creation or any request. Each opening limit declares `cleanupPrice`, an exact
positive limit-price ceiling at least as large as its opening price. Admission
reserves one order/signed-request slot and notional for the sum of **all possibly
filled opening quantities** at the largest declared ceiling. Cancellations never
refund that reserve. Openings must have one side; no new opening is accepted
after cleanup starts. A close uses `cleanup: true`, opposite side, IOC, no more
than that original quantity and no price above the reserved ceiling. Only this
close may consume the reserved capacity. These are planned finite bounds, not a
guarantee of execution through an unbounded price gap. The operator must still
reconcile actual fills/positions before choosing a closing size; the label is
not a venue-native reduce-only guarantee.

Each new action persists its identity, nonce, immutable unsigned body and consumed
capacity before signing. Private signed wires persist before transmission;
bounded raw responses persist before interpretation. A named action cannot be
resent; source changes, duplicate IDs and a second faucet attempt are refused.
Stepwise inspection allows explicit reconciliation/cleanup after a known refusal;
it is not automatic retry or unknown-execution replacement authority. Operators
must reserve closing capacity, inspect semantic statuses (HTTP 200 can contain
`unauthorized signer`), and verify positions/orders/agents after cleanup. A
non-200 diagnostic query must not skip independent cleanup of known exposure.

The [October 20 receipt](../../docs/venues/bulk-native-qualification.md) records
actual opening/closing and cancellation, but also a missing `tradeId` and fee-tier
404. The codec now accepts the SDK-defined absent-ID/native-pair representation;
it never rewrites the original response or replaces an explicit invalid ID.
Do not treat an ACK as a fill or promote this wire test into Gateway qualification.
Private artifacts remain ignored; only
sanitized regression inputs belong in the repository.
