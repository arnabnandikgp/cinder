# Operator runtime

`cinder-adapter` is the decision core. `cinder-operator` owns the private
SQLite journal and serialized recovery, with authenticated QFS and official
Phoenix/Rise RPC ports.

The `recover` command runs one startup/recovery pass. It can sign guarded
private fill/failure acknowledgements and operator-down updates; it **does not
submit new Phoenix orders**. Before exiting it confirms OPERATOR_DOWN on both
L1 Config and private Book. Successful recovery is not permission to trade or a
production-readiness claim.

`execute` explicitly opts into one serialized execution/recovery pass. It
requires both `solvency_policy` and `execution_policy`; there are no trading
defaults. It submits at most one IOC against a freshly reconciled/admitted
snapshot. Like `recover`, it confirms both OPERATOR_DOWN gates before exiting.
Run another pass to observe the finalized outcome, apply its guarded private
acknowledgement, and reconcile. A partial IOC's remainder is restored privately,
never automatically submitted as another trade.

Execution policy specifies `quote_headroom_bps`, complete per-market
`market_quote_limits_usdc` and `market_fee_limits_usdc` maps, and
`max_admission_lots`. The frozen quote ceiling limits perp notional, not margin
or a spot-token payment. The fee ceiling is an actual atomic execution bound,
not a guessed venue fee rate; only confirmed actual fees are charged privately.
Every integer partial-fill quantity is admitted at the worst allowed price and
full fee ceiling. The exhaustive evaluator refuses intents above its explicit
quantity limit (maximum 4096) or 65,536 quantity/user/scenario evaluations;
it does not sample or split them. Choose smaller intent sizes for this
experimental runtime. Resting orders and automatic order splitting are unsupported.

Nonzero funding permits dispatch only after the authenticated allocation epoch
has completed for the entire registry and matches the current native generation.
An accumulator change inside an IOC rolls back the whole transaction.

## Maintenance decision boundary

`run` explicitly enables the single-writer autonomous service. It requires
`maintenance_policy`, `execution_policy`, and `solvency_policy`; missing policy,
fee provisioning, stale views, incomplete registry or unresolved financial
receipts keep entries down. `maintain` performs one maintenance pass and exits
with both gates closed. `recover` and `execute` retain their one-pass semantics.
Timer decisions and feed notifications never grant financial permission.

`MaintenanceProgress::plan` coalesces wakeups into a bounded priority list:
order recovery first, followed by feed refresh and restrictive halt mirroring,
funding accrual/fold, liquidation scanning, expiry, collateral, heartbeat, and
reserve publication. Process at most one mutating maintenance intent before
reobserving/replanning. Completion timestamps advance only after successful
work, not when a timer wakes. They are Unix milliseconds, never ER/L1 slots.
Future/backwards clocks fail closed. Stale marks still wake liquidation queue
draining, but cannot be used to classify a new queue. A timer alone cannot mint
`Book.last_scan_ms`; heartbeat eligibility requires a completed live scan.
Funding accrual and fold intents require authoritative venue observations;
waiting 24 hours is not settlement evidence.

`OperatorRuntime::maintenance_health` performs a coherent, authenticated private
book read around the official Rise view. It returns aggregate counts only.
The private scanner projects confirmed lots, sums IM/MM across the entire user
book, counts cash/debt once, includes unsettled funding, discounts positive
uPnL through the shared risk engine, and keeps losses in full. Users are ranked
by signed MM deficit with deterministic identity tie-breaking. Pool safety or
zero net venue exposure cannot hide an unhealthy private user. Assessment does
not submit a liquidation, write a heartbeat, or clear either ownership gate.

`build_reserve_snapshot` requires all registered ledgers, matching identities,
aligned funding epochs, no pending OIDs, and matching private net lots/Book
residuals. Outstanding withdrawals remain unsupported pending their outbox.
Checked aggregate totals include bad debt. Publication must additionally prove
fresh complete venue I1/I2, resolve financial outboxes, and recheck snapshot
coherence; this encoding helper does not supply those proofs.

The versioned canonical list is:

```text
ASCII("cinder:cash-claims:v1") || schema_version:u8 || user_count:u32LE
then rows sorted by raw user pubkey bytes:
  pubkey:32 || free:u64LE || reserved:u64LE || bad_debt:u64LE || n_pos:u8
  then (asset_id:u16LE || confirmed_lots:i64LE), sorted by asset_id
```

`root = SHA256(list)`. This preserves the frozen claim-row fields; it is a cash
claim commitment, not a Merkle escape proof or a raw-equity/solvency assertion.
`book_hash = SHA256(ASCII("cinder:book:v1") || full Anchor Book account bytes)`,
including the discriminator. Private claim rows are never public logs or
journal payloads. The publication intent is due after 20 acknowledged fills or
30 seconds of dirty state since the last publication (first unpublished ACK
when no prior publication exists), accelerated on liquidation/debt incidents.
Neither counter completion nor elapsed time proves publication finality.

Native public WebSocket subscriptions coalesce wake hints, reconnect after EOF,
and fall back to authoritative HTTP replay. The service targets a best-effort
50-ms whole-user scan, not a latency guarantee. Fresh scans authorize 1-second
heartbeats; stale snapshots cannot manufacture liveness. Mutations close both
gates, drain the sole outbox, then reobserve. Reconciled quiet read-only passes
keep entries usable between mutations. SIGINT/SIGTERM drain the current pass
and confirm both operator-down gates before releasing the pool lock.

Private MM breaches trigger bounded liquidation even without a new user order.
Unsafe native tiers prioritize MM failures, then under-IM users. Liquidations
cannot increase or flip confirmed exposure, cancel uncertain native intents, or
bypass invariant/venue-breach halts. Every integer partial fill is evaluated;
native tier/MM-surplus fences bound closing-cost degradation. Existing deficits
are not recapitalized, and confirmed fees/losses still become explicit debt.
Administrative/debt halts are restrictive atomic ORs and are never automatically
cleared. Unsafe-pool incident rearming remains an explicit operator decision.

Periodic collateral repair moves existing vault cash through the native PDA
bridge to Phoenix IM plus 20%, with a 50-USDC floor. It uses its own immutable
receipt identity, never a fabricated user order. Unknown deposits block further
mutations; Book changes only after confirmed native evidence. Automatic excess
collateral pulls are deliberately disabled.

### Funding accumulator checkpoints and epoch journal

Funding-rate maintenance is separate from the custody top-up outbox. The
runtime can capture a coherent, read-only `FundingCheckpoint`, using official
Phoenix cumulative quote-lots-per-base-lot rates and native update timestamps
in **seconds**. For the supported six-decimal quote mint, one quote lot is one
micro-USDC. With unchanged confirmed inventory, the private allocation is
`-(new_accumulator - old_accumulator) * confirmed_base_lots`: positive rate
deltas charge longs and credit shorts. Checked integer arithmetic has no
rounding step. Gross user allocations sum to the pooled net funding, including
fully offset private long/short books.

Initializing the durable baseline requires both ownership gates down, a
complete registered inventory, a never-traded flat book, zero native/private
funding, and matching backing. Existing exposure or trading history cannot
silently adopt the latest accumulator as its baseline. A saved nonce/basis/Book
inventory commitment detects position changes even if endpoint lots match.
ACK rebases preserve the old rates only when every intervening filled operation
has its persisted atomic generation barrier. A later update is then allocated
to the confirmed post-ACK inventory. Flat new-user incorporation has its own
registry barrier. Unexplained changes and unfenced historical gaps are refused.

`FundingEpochPlan` currently supports the quiet-inventory accrual path only.
The journal freezes source/target checkpoints and hashes of the exact Book
bump and every user's allocation before signing. Actual private instruction
bodies remain in memory. It permits one uncertain signed write at a time;
the Book bump must have its matching successful receipt before any allocation
attempt. A timeout, missing transaction, or expired blockhash cannot authorize
another signature. Only the exact failed receipt permits a replacement.
Receipt decoding joins the signer, program, Config, scope, epoch, instruction
hash, signature, and positive slot; callers must obtain that receipt through
authenticated QFS, not treat arbitrary JSON as chain proof.

Restart reconstruction produces the same immutable instruction bodies, even
when some users already reached the target epoch. The checkpoint advances
atomically only after successful receipts for **all** users, including flat
users, plus a coherent aligned private snapshot. A newer native update during
this process does not skip the frozen target: it becomes the next funding
epoch. Active maintenance blocks new order intents, custody top-ups, native
dispatch, and registry expansion; recovery leaves entries down rather than
misclassifying partial allocation as an I2 incident. Startup validates the
checkpoint/history chain and unresolved-write ordering.

`accrue` performs one authenticated funding pass. Position-changing ACKs carry
the old baseline only across persisted atomic generation fences; unexplained
inventory changes fail closed. A new flat registered user joins through an
explicit immutable registry barrier, never by resetting prior users' rates.
Funding folds require exact allocated generations, observed native settlement
on every asset, and fresh complete basis-aware I1/I2. Positive pool equity or
collateral deposits alone are not settlement evidence. Fold and allocation
attempts persist exact signatures before sending; absent/expired receipts stay
unknown rather than being signed again. Completion alone cannot reopen entries.

Reserve publication is a guarded, finalized L1 aggregate write, independent of
native orders and Magic Actions. Its WAL binds expected epoch, complete claim
root, debt-aware totals, Book commitment and acknowledged-fill count. Root
receipts are drained before subsequent maintenance. The service verifies the
delegated SOL fee balance and validator-owned Magic fee vault before long-lived
PER operation. Provisioning is separate from user USDC and native collateral.

One-pass `recover`/`execute` also drain a daemon's outstanding maintenance
receipt before discovering new orders. An unknown outcome keeps both gates
closed and reports unresolved maintenance; it never authorizes a replacement.

## Bounded native execution

Each native transaction is `[compute budget, before fence, Phoenix IOC, after
fence]`, independent of PER commits/Magic Actions. The first fence hashes the
public Config/native account snapshot, checks age and the orderbook fee counter,
and binds the adjacent IOC. Both fences require an identical opposite-phase
peer around that IOC; the compact trailing instruction commits the full preceding
guard body. Removing or changing it is rejected. Both phases fence all active
native funding generations, including membership changes.
The trailing fence caps the actual native taker
fee-counter increase and checks authoritative Hawkeye health plus confirmed
posted cash (20% IM buffer / 50 USDC floor). A failing fence rolls the entire
IOC back. Signed transactions are simulated, their exact signature/expiry is
written ahead, and only then sent once. Legacy transactions exceeding Solana's
1232-byte packet limit are refused; account lists are never truncated.

Admission reserves native close losses and the fee allowance in the posted-cash
requirement. A shortfall creates the immutable funding outbox before any send;
the atomic vault-PDA bridge and confirmed Book synchronization must finish in
separate passes. The next IOC always needs a new native/private admission.
If prices change enough to need another deposit after that operation's sole
funding attempt, execution stays halted rather than silently changing its
immutable amount or retrying an uncertain transfer.

## Configuration and commands

Build/deploy the updated Cinder programs in your chosen test environment first.
The guarded acknowledgement and adapter-owned operator-down instructions are
additive; existing accounts do not require a layout migration for this change.

Copy [the configuration example](operator-config.example.json), replace its
placeholder pooled trader, and verify every deployment/account pin against the
selected venue. The example contains public production deployment addresses,
not a ready-to-run devnet configuration. Only known matching production/beta
Rise program/global-config pairs are accepted.

Use an L1 RPC where both the configured Cinder deployment and Rise venue exist
(or a suitable local fork), not a mixture of Cinder devnet and Phoenix mainnet.
The index/buffer arrays must include all accounts required by the official
Hawkeye view in its expected order; the example lists header pins only.
Incomplete paging fails simulation rather than producing an empty exposure.

Configuration contains public account pins, market mappings, key-file and lock
paths, and **names** of RPC environment variables. Keep actual RPC URLs/API keys
in your secret environment; never commit them. HTTPS is required except for
loopback HTTP in local tests. Existing bearer-token URL parameters are rejected:
the runtime obtains its own operator token. Before signing, it validates QFS's
exact `Login to Query Filtering Service` challenge domain, timestamp, and
operator public key. Challenges older than five minutes or more than 30 seconds
in the future are rejected; arbitrary server-supplied messages are never signed.

The key file must be a Solana JSON keypair, owner-only (0600 on Unix), owned by
the running operator, and neither a symlink nor a multiply linked file.
Config.adapter and the pooled trader's owner/position authority must authorize
that operator. No user key or user QFS token is needed for recovery.

```bash
cargo build --locked -p cinder-operator

# Journal-only commands: no RPC requests.
target/debug/cinder-operator init .cinder-operator/journal.sqlite
target/debug/cinder-operator status .cinder-operator/journal.sqlite

# Set the named CINDER_L1_RPC and CINDER_QFS_RPC variables through your
# secret environment, then run one authenticated recovery pass.
target/debug/cinder-operator recover operator-config.json .cinder-operator/journal.sqlite

# Opt in to ONE bounded execution/recovery pass, only after configuring policies.
target/debug/cinder-operator execute operator-config.json .cinder-operator/journal.sqlite

# Funding/maintenance one-pass commands also close both gates on exit.
target/debug/cinder-operator accrue operator-config.json .cinder-operator/journal.sqlite
target/debug/cinder-operator maintain operator-config.json .cinder-operator/journal.sqlite

# Explicit continuous operation, after fee provisioning and policy review.
target/debug/cinder-operator run operator-config.json .cinder-operator/journal.sqlite
```

For continuous operation, replace the example's null maintenance policy with
explicit values (these are illustrative, not production recommendations):

```json
{
  "liquidation_slippage_bps": 100,
  "liquidation_deadline_slots": 32,
  "native_ws_env": "CINDER_PHOENIX_WS",
  "fee_balance_index": 0,
  "minimum_fee_balance_lamports": 1000000,
  "minimum_magic_vault_lamports": 1000000
}
```

Set that WebSocket environment variable to a public native `wss://` endpoint
(plaintext is accepted only for localhost). Never supply a user QFS token.
Fee-balance index 255 is reserved for Magic Actions and is refused here.
The fee provisioning script defaults to emitting **unsigned** instructions:

```bash
node -r ts-node/register/transpile-only scripts/provision-per-fees.ts --balance-lamports=10000000
```

Review the instructions first. `--send` explicitly submits once with the
operator key named by `CINDER_OPERATOR_KEYPAIR`; an uncertain send must be
reconciled by its printed signature before retrying. `--validator-vault` is only
for a missing validator vault and requires its actual matching
`CINDER_VALIDATOR_KEYPAIR`. Existing validator fee infrastructure belongs to
the PER operator—do not initialize or redelegate it blindly. The service checks
L1 delegation records and authenticated PER account ownership/balances.

Recover exits 0 only when no operations remain unresolved and reconciliation
succeeds; 3 means recovery remains gated, 1 means a runtime error, and 2 indicates
invalid CLI usage. Output contains aggregate counts and typed error/reason codes,
not credentials, private balances, user identities, or raw receipts.

## Durable causality and acknowledgements

The journal uses schema version 9 and migrates older supported journals in
place, preserving their recovery state. A journal upgraded to version 9 cannot
be reopened by an older binary. Back up the private journal with SQLite's
WAL-aware procedure before upgrading; do not copy only the main database file.

A client order ID is not globally unique. The journal binds the user, ledger,
placement nonce, client ID, and order kind to a deterministic global venue ID.
The full hash is retained and truncated-ID collisions are rejected. Runtime
binding also pins the Cinder deployments, operator, pooled trader, venue, and
market mappings; a different deployment cannot silently adopt the journal.

Startup compares a durable user registry against L1 initialized/delegated user
identities and the complete operator-authorized QFS scan. It reads private Book
and ledgers together, then rechecks the registry. Missing or redacted accounts
halt recovery rather than becoming an empty pool.

The execution boundary can atomically persist a prepared native IOC's signature
and blockhash expiry with its submission intent before broadcast. One native
transaction identity is allowed per order; unknown replies and expired
blockhashes alone do not authorize a fresh IOC. Preflight failures before this
boundary retain the unsent prepared intent. Transaction bodies and credentials
are not persisted. Only the explicitly configured `execute` port enables native
dispatch; `recover` remains recovery-only.

An exclusively owned pristine prepared journal row (no submission intent,
signature or fill) is causal proof that this operator has not submitted it.
An empty venue-history result is not such proof. Read failures keep that
pristine state intact but block dispatch until actual reads succeed. Signed or
ambiguous attempts are never reset to unsent because a lookup is empty.

A structured Solana preflight rejection (`-32002` with a non-null transaction
error and no result) proves that particular broadcast was rejected before
execution and permits failure acknowledgement. Transport failures, rate limits,
malformed replies and node-health errors remain ambiguous. A crash before that
negative proof is journaled still requires recovery of the persisted signature.
Finalized failed transactions without Phoenix packet logs are checked against
the exact persisted signature and bounded native instruction/accounts instead
of manufacturing logs or authorizing another IOC.

An expired, conclusively unsent order may cancel only its unsigned funding
outbox. Cancellation has its own audit timestamp; it is not a fabricated failed
transaction. Signed or uncertain funding remains unresolved, and confirmed
funding must synchronize Book before the order is restored.

An `ExecutionBudget` freezes the maximum native quote lots (executed perp
notional, **not margin or spot-sale proceeds**) and the execution-fee allowance
separately from the user's base-sized request. `record_execution_budget` must
run before funding or native signing. Exact replays are idempotent; a changed
budget conflicts even after restart. Migration retains absent budgets for
legacy operations and does not alter their already-signed packets. The new
native builder refuses operations without a budget; absence is not an unlimited
execution default.

Both buys and sells retain their base-quantity cap, price protection and expiry,
and additionally set Phoenix's `num_quote_lots`. This is a ceiling on matched
notional; native buy-side fee adjustment can further reduce the available
matching budget. It is not a target margin deposit or a guaranteed full fill.
For example, an IOC selling 10 SOL-equivalent perp units with a 1,200 USDC
notional ceiling may match all 10 at 110, but only approximately 9.23 at 130,
subject to lot rounding. A closing order can consequently leave a position
partially open. Only the actual terminal fill is acknowledged; its unfilled
tentative remainder is restored, never automatically dispatched again.

Budget amounts/headroom must come from explicit execution policy, not guessed
production defaults. `ExecutionBudget::taker_fee_allowance` supplies checked,
conservative arithmetic for a separately authenticated upper-bound fee rate:
`ceil(quote_ceiling × rate_micro / 1_000_000) + max_base_lots - 1` for a nonzero
rate, or zero for a zero rate. The additional term covers per-match upward
rounding, even if a single IOC reaches many makers/splines. That helper does
**not** authenticate market fees, trader overrides, additional charges or rate
changes before execution; those remain admission obligations. Network fees in
SOL are separate from USDC trading fees. The persisted fee allowance belongs
in the confirmed-cash collateral target; it does not replace user/pool margin
and backing/stress checks. The execution command uses the explicit absolute
fee allowance and an atomic native counter fence, so fee overrides or changes
cannot charge more than that allowance in a successful IOC transaction.

Recovery requires the exact persisted quote ceiling in native packet evidence,
including `None` for legacy uncapped packets. A proved notional or fee-allowance
breach still retains and acknowledges its real fill. It durably keeps
OPERATOR_DOWN set with `ExecutionBudgetExceeded`, including after the operation
becomes terminal or the operator restarts; successful acknowledgement is not
permission to clear an execution incident. There is no automatic incident reset.

The vault's `fund_phoenix` instruction signs as the canonical vault-authority
PDA and atomically converts USDC through the pinned Ember program, then posts
PhUSD into the one pooled Phoenix cross trader. It does not route funds through
an operator wallet. Its `init` funding receipt prevents the same funding ID
from debiting the vault twice; failure of either CPI rolls back the entire
bridge and receipt. `delegate_phoenix_trader` delegates position authority to
the configured operator without transferring custody.

The private funding outbox records immutable intent before signing and the
exact transaction signature before broadcast. One unresolved pool deposit
blocks new venue sends and reopening the operator gate. An authenticated,
finalized, canonical vault receipt proves the atomic deposit; absence, timeout
and blockhash expiry do not prove failure or authorize another deposit.
Recovery authenticates either that receipt or the exact finalized failed
funding transaction. A proved failed deposit atomically rejects its unsent
hedge in the journal; normal failure acknowledgement then restores its private
tentative position without trading.

A successful deposit remains unresolved until fresh authoritative native cash
is synchronized into Book. The exact signed Book assignment, amount and native
observation slot are journaled before broadcast. Unknown outcomes cannot be
re-signed, even after expiry. A proved terminal assignment may be replaced only
after fresh observation; completion requires its authenticated ER receipt and
a fresh matching Book/native cash read. ER and L1 slots are not compared as
one clock. The coordinator discards the pre-funding snapshot rather than
dispatching an IOC in the same recovery pass. Older receipt-only version-4
records retain the gate until this synchronization is completed.

The production recovery hooks and bounded execution ports are implemented. New
native IOC and funding preparation/broadcast remain disabled in the recovery
command; only the explicit execution mode enables them. A pending deposit is never
available cash. Signed transaction bodies and credentials remain in memory;
restart observes the journaled identity, never blindly submits a fresh deposit.

The native funding proof is opt-in and requires a fresh, disposable Surfpool
mainnet fork. This sandbox globally disables signature verification so the
external Phoenix onboarder can be simulated. Cinder calls still carry real
disposable operator signatures and use actual vault-PDA CPIs, but this is not
a proof of cryptographic signature rejection. Never use this sandbox
configuration for production.

```bash
NO_DNA=1 anchor build --program-name cinder_vault --ignore-keys
NO_DNA=1 anchor build --program-name cinder_ledger --ignore-keys
# Start a fresh fork on these otherwise-unused ports; the datasource is read-only.
NO_DNA=1 surfpool start --rpc-url "$SURFPOOL_RPC_URL" --port 8989 --ws-port 8990 \
  --host 127.0.0.1 --airdrop-amount 0 --db :memory: --no-deploy --ci \
  --skip-signature-verification
# In another terminal:
CINDER_R5_FORK=1 PROVIDER_ENDPOINT=http://127.0.0.1:8989 \
  yarn ts-mocha -p tsconfig.json -t 180000 tests/runtime-funding-fork.test.ts
```

The test refuses non-local transaction endpoints. It verifies atomic native
funding, replay rejection, both conversion- and deposit-failure rollback, and
authoritative Hawkeye posted cash. It also executes a real multi-price native
IOC reversal (including spline liquidity), decodes its receipt through the
pinned Rust production parser, and checks aggregate taker basis, realized cash,
funding settlement and actual fees against Hawkeye. Those facts are passed to
the real Cinder guarded acknowledgement on the fork, verifying I1, basis-aware
I2, margin, fee accrual, actual filled quantity and replay rejection.
Private starting accounts are synthetic fork fixtures: this is actual program
accounting coverage, **not PER/QFS privacy or full live-admission proof**.
Read-only native simulations additionally verify that Phoenix quote budgets
limit IOC fills in both directions, using the exact production-builder
instructions derived from persisted journal budgets. This verifies native cap
enforcement, not production fee authentication or full PER/QFS admission.

Atomic native simulations additionally exercise the before/after execution
fences in both directions. An over-budget actual fee causes the trailing fence
to reject, and a submitted failing transaction leaves trader, orderbook and
index/buffer data unchanged. This verifies rollback, not merely an off-chain
estimate. For the full operator/PER test stack, explicitly opt in:

```bash
CINDER_R5_FORK=1 bash scripts/test-runtime-fork.sh
```

Set `CINDER_R5_ACCOUNT_LATENCY_MS=300` to additionally exercise admission under
delayed local account reads; the default is no injected latency.

That stack loads the installed MagicBlock local-test binaries/public fixtures
on the localhost Phoenix fork. Without a live oracle crank, the fixture refreshes
the real SOL mark and populated component observation slots/oracle timestamps,
preserving prices, weights, validity rules and economic metadata. It also marks
the local exchange active and clears its restart acknowledgement slot; this is
not evidence of upstream venue availability. Native execution still recomputes
and validates its mark normally. Surfpool 1.5's post-delegation owner index
requires a test-only union of two actual registry scans: the fixture freezes
only its user address list, then rereads those accounts and filters current
ownership on every scan. Native financial accounts, Hawkeye returns and
transaction evidence are not stubbed.
Fork onboarding alone skips the unavailable external onboarder signature.

The private proof runs full fill, quote-capped partial fill, partial close,
reversal, zero fill and expiry through the real Rust `execute` command and
delegated PER accounts behind QFS. It kills the operator after successful
funding, native IOC submission and private acknowledgement but before each
reply arrives. Restarts must converge without another IOC, duplicate funding
or repeated fees/ACKs. Every scenario checks native/private/Book inventory,
basis-aware raw-equity reconciliation and actual native fees. Anonymous QFS
must not reveal the user ledger. Starting collateral/deposit credits are
disclosed synthetic fixtures, not proof of production deposit/withdrawal flows.

Run the separate multi-price/native-rollback proof on another fresh stack:

```bash
CINDER_R5_FORK=1 CINDER_R5_PRIVATE=0 bash scripts/test-runtime-fork.sh
```

For autonomous maintenance coverage, run:

```bash
CINDER_R5_FORK=1 CINDER_R6_MAINTENANCE=1 bash scripts/test-runtime-fork.sh
```

This runs the real `run` daemon through 24 accelerated observed hourly funding
generations, all-user epoch alignment, native settlement and a crash during the
private cash fold. It also crashes after an accepted heartbeat to verify the
daemon-to-one-shot handoff drains the exact receipt before a new order.
A quiet-book funding shock triggers a bounded liquidation
without a user order or manual liquidation call; the unaffected flat user's
cash remains unchanged. The test also verifies delegated fee readiness,
guarded root publication after liquidation, and OPERATOR_DOWN on shutdown.
Funding-feed and oracle-clock changes are disclosed localhost-only native
account fixtures, not mocked Hawkeye or private financial RPC results. This is
an integration/restart test, not a 24-hour wall-clock soak or public deployment.

CI runs the autonomous private mode and the native-only mode in
`Phoenix integration`, using the existing private
`SURFPOOL_RPC_URL` secret as a read-only datasource. Normal Rust unit tests do
not contact Phoenix or a public RPC. Each mode intentionally skips the other's
fixture because their private Book accounts share the protocol's canonical PDA.

Private transaction history joins each pending order to its exact signed
placement, immutable bounds, and nonce, including after client-ID reuse.
Missing history or nonce gaps leave the operation unresolved. OpenOid itself
does not store a placement nonce: this identity remains an attestation by the
trusted adapter, not a new trustless on-chain proof.

Before private acknowledgement submission:

1. Persist the authoritative venue fill/failure facts.
2. Read the exact pending placement and calculate fresh margin through R3's
   shared risk boundary.
3. Recheck the private ledger after public risk I/O.
4. Sign a guarded acknowledgement and durably store its signature, expiry, and
   observed nonce **before** sending it.
5. Confirm its exact guarded receipt and economic facts before finalizing.

The on-chain guard checks immutable order fields, pending/liquidation kind,
ledger nonce, and a SHA-256 hash of the whole observed ledger. Funding, another
acknowledgement, or a deposit can change ledger state without advancing nonce;
the hash prevents an old risk quote from applying across that race.

All prepared acknowledgement attempts survive restart. Lost responses cause
receipt/status inspection, not automatic resubmission. Status-only/redacted
receipts cannot prove economic acknowledgement. Expiry is not sufficient:
retry also requires every prior attempt to be unable to apply and the exact
original placement to remain pending.

## Venue evidence and reconciliation

Rise recovery reads finalized pooled-trader transactions and the official SDK's
native packet/event encodings, not REST absence or WebSocket delivery. It checks
the configured operator, trader, native market, global client ID, side, exact
IOC quantity, price bound, deadline, fill totals, fees, and slot context.
Malformed or conflicting evidence halts recovery. A successful terminal IOC
with zero fills is failure evidence; a missing lookup is not.

Terminal partial IOCs acknowledge only their actual filled lots, quote amount,
and fee. The acknowledgement restores the unfilled tentative lots and calculates
margin for the resulting position. Open or unknown outcomes remain unresolved,
even if their observed quantity equals the requested quantity; the remainder
is never resubmitted or separately failure-acknowledged. A
confirmed bound violation remains a fill fact to acknowledge and halt over;
recovery must not erase real exposure by treating that fill as a rejection.
This command neither dispatches nor cancels resting orders.

The unsigned `build_bounded_ioc` builder takes a prepared, budgeted operation
and uses the pinned native delegated
market-order entrypoint with the single operator as authority/primary position
authority. It retains the exact price, base quantity, quote ceiling, slot deadline and derived
global client ID, allows partial fills, aborts self-trades and never cancels
resting orders or opens an isolated account. It preserves complete index/buffer
account lists and pins program, log authority and global configuration to the
configured deployment, independent of `PHOENIX_ENV`. Native `BaseLots` and
`Ticks` decoder bounds are checked before construction; the SDK builder alone
does not check those limits.

`IocMarketAccounts::from_asset_map` checks the configured map address, owner
and native layout, requires an unambiguous symbol/asset-ID mapping, rejects
isolated-only markets and derives the spline PDA for the configured deployment.
A market binding cannot be reused after changing deployment or asset-map pins.
Construction is not admission, freshness or signing-authority proof. The
execution port revalidates a coherent native account view, verifies the
operator's authority, obtain fresh user/pool/backing and collateral
proof, then simulates and persists the exact signature before broadcast. Neither
this helper nor the recovery command signs or sends a new venue IOC.

Reconciliation compares confirmed private positions (tentative positions minus
pending deltas), acknowledged Book, and venue exposure. Cash includes explicit
user debt and unsettled user/pool funding. A deferred withdrawal is unsupported
until the money-movement outbox exists; cash-in-flight is never invented as zero
over such a withdrawal.

Private user entry bases and the net Phoenix entry basis can differ. Opposing
users may retain open positions while closing the pooled venue position,
realizing venue PnL before either user realizes PnL. With equal net lots the
common marked position value cancels, so raw-equity reconciliation is exactly:

```text
signed_user_cash + user_unsettled_funding
  == vault + venue_cash + pool_unsettled_funding + genuine_cash_in_flight
     + sum(private_signed_entry_quote) - venue_signed_entry_quote
```

Private basis comes from confirmed positions, not tentative lots or a rounded
average entry price. Phoenix basis is the negative of its authoritative Hawkeye
virtual quote position, read in the same bracketed snapshot as exposure and
collateral. Missing/noncanonical individual basis or overflow fails closed;
there is no dust tolerance or invented funding/cash-in-flight correction. Risk
PnL haircuts are separate and must not be applied to this accounting identity.
Tests using the pinned official Rise math cover profitable and loss-making
netted closes, averaged entries, partial closes, reversals and exact fees.

Adding to a user's position accumulates exact signed quote value, preserving
weighted-average entry basis without rounding a stored average price. Reductions
realize proportional basis; reversals close the old basis and start a new one.
Different users keep their own bases. Integer division truncates toward zero,
and the remaining exact basis retains the rounding residual, preserving cash
minus basis through later fills and final close.

Uniform-price SDK fills match the private accounting helper for entries,
reductions and reversals. This does **not** establish parity for one order
matching at several prices: collapsing sequential SDK fills can change realized
cash and remaining basis, even while raw equity reconciles. Regression tests
retain both the multi-price reversal and split-close rounding counterexamples.
The opt-in native fork proof distinguishes multi-price matching and crossing
zero, and confirms that the actual Phoenix taker update matches Cinder's
aggregate acknowledgement calculation. This does not assume that separately
submitted orders share one averaging update. New dispatch additionally requires
the exhaustive partial-path admission and atomic fee/collateral fences.

This changes reconciliation, not user cash or the reserve-claim layout. Reserve
root totals still describe signed cash claims, not immediate withdrawable venue
cash or discounted risk equity; publication must use a successfully reconciled
inventory snapshot. Flat, settled inventory reduces this identity to cash-only
I2. Replay-safe user withdrawals remain R7 work.

Current pooled exposure and collateral come from official Hawkeye simulations
over global trader index/active-buffer state, **not only the cold trader account**.
Public account bytes are bracketed around these views, and private ledger/Book
fingerprints are rechecked afterward. Marks and trader observations use actual
venue slot times; local fetch time cannot freshen an old oracle. Stale,
future-dated, incomplete, unsupported, or changing observations fail closed.

Cinder asset IDs are explicitly mapped to native Rise IDs (for example, Cinder
SOL 1 versus native SOL 0); symbols must match venue metadata. Vault USDC and
Phoenix's canonical collateral mint are pinned separately, both six-decimal.
The native-amount conversion follows the official Ember/Rise SDK boundary;
this is not a general valuation or stablecoin-peg guarantee.

## Solvency and admission

Reconciliation is an accounting identity, not proof that payable user claims
are backed or that cash is immediately available. Recovery separately checks:

- Each user's complete confirmed private book: sum SDK-backed position IM,
  apply each market's positive-PnL haircut, count losses and unsettled funding
  in full, and enforce Cinder's whole-book leverage limit. Tentative closes
  cannot hide confirmed positions.
- The authoritative pooled-trader health from Hawkeye.
- Backing: vault cash plus native cash, raw venue position PnL and unsettled
  funding, compared with **the sum of positive individual raw user equities**.
  Negative user equity never funds another user's claim. Existing bad debt
  keeps recovery gated even if positive claims happen to be fully covered.

Missing health evidence, arithmetic overflow, insufficient backing or user
margin leaves OPERATOR_DOWN set. Confirmed fills are still acknowledged and
their real economic effects recorded; health checks gate further risk, not
recognition of trades that already happened.

An optional `solvency_policy` configuration adds explicit portfolio/market
gross-notional caps and stress scenarios. There are no trading defaults; the
configuration example leaves it `null` for recovery-only use. A policy has:

- `max_gross_notional_usdc`: a positive portfolio gross cap in micro-USDC.
- `market_gross_limits_usdc`: a positive cap for every configured Cinder asset
  ID, including markets whose venue net exposure is zero.
- `scenarios`: 2–64 complete `mark_factors_bps` maps and a positive
  `close_cost_bps` allowance. Factors use 10_000 for unchanged prices, must be
  within 1–100_000, and collectively include an up and down move for every
  configured market. Missing markets are rejected, not assumed unchanged.

Each scenario recomputes individual raw equities at stressed integer marks,
reserves each user's estimated close cost against their claim, and deducts
**all** those costs from backing—including costs of defaulting users. Negative
remaining claims are floored at zero, never treated as collectible assets.
Scenarios therefore detect an unfunded winner even when offsetting private
positions leave Phoenix flat. This bounded model holds current funding fixed;
it does not prove safety against every price path, funding change, outage,
stablecoin depeg or cost beyond the configured allowance. Scenario/cap selection
needs market-specific review; structural validation is not calibration.

Configured failing caps/stress backing keep recovery gated. Without a policy,
recovery can establish only current health. Dispatch additionally requires a
fresh, explicit post-intent admission check for user risk, pool risk, confirmed
collateral and partial-fill paths. That port defaults to denial; the concrete
runtime implements it only with explicit execution policy. Neither a configured passing stress
report nor successful recovery alone authorizes native dispatch.

The admission port returns the actual private-ledger, native-trader and mark
observation times used in its checks. After admission I/O, the coordinator
checks those newer observations against the unchanged two-second TTLs, rather
than requiring the earlier startup snapshot to remain fresh through every
budget and funding check. Stale/future observations or a backwards clock still
block dispatch; the atomic native snapshot and freshness guards remain required.

The 20% Phoenix IM buffer / 50 USDC floor is a collateral-availability target,
not insurance capital. A vault-to-transit transfer does not fund Phoenix. The
pinned, PDA-authorized deposit path, durable collateral outbox and confirmation
are still required before a hedge. No external capital, debt recovery or
socialization term is invented to make either reconciliation or stress pass.

`CollateralRequirement::for_hedge` computes
`max(ceil(Phoenix_IM × 1.2), 50 USDC) + verified_execution_cost_allowance` using
checked wide arithmetic. `shortfall_usdc` accepts only confirmed native cash:
vault cash, transit tokens, pending deposits and unrealized gains cannot satisfy
the posted-cash requirement. The allowance is reserved after applying the floor
so execution costs cannot consume that minimum. This arithmetic helper is not
a collateral submission/recovery port or permission to dispatch.

OPERATOR_DOWN now blocks **new private withdrawal requests** on either Config
or Book, in addition to entries. Previously authorized payment/completion
recovery is separate; replay-safe money movement belongs to R7. The autonomous
supervisor remains experimental. Production operational hardening and an
explicit deficit policy remain prerequisites to real user funds.

## Ownership, gates, and storage

Startup confirms OPERATOR_DOWN before recovery. Both Config and Book updates
modify only that bit, preserving administrative, bad-debt, venue-breach, and
other economic halt reasons. The recovery-only process closes both gates again
before releasing ownership, even after successful reconciliation. This also
keeps new withdrawal requests blocked after the recovery-only process exits.

Use one host, one exclusive operator key, and one shared private
`pool_lock_directory` for every process controlling the same trader. The
pool-scoped lease is independent of the journal path and operator credential.
Different local journal paths cannot concurrently acquire the same pool lease
in that directory. Separate directories/hosts are **not** fenced; replicas and
manual/foreign activity on the pooled trader are unsupported. Abrupt process
death is not a chain-level liveness watchdog; that belongs to later maintenance.

SQLite uses schema 9. Supported schemas 1–8 upgrade through ordered,
transactional migrations: schema 2 adds runtime binding, durable users and
prepared ACK attempts; 3 adds native submission attempts; 4 adds the funding
outbox; 5 adds funding failure and Book synchronization evidence; 6 adds
immutable execution budgets; 7 adds unsigned funding cancellation timestamps;
and 8 adds funding-rate checkpoints, immutable epoch/write hashes, and exact
signed maintenance attempts; 9 adds serialized maintenance writes, inventory
anchors and order/registry funding barriers. Upgrades never invent a funding-rate baseline.
Existing recovery state is preserved, and legacy operations do not acquire
invented budgets or signatures. An unbound legacy journal with historical
side effects cannot be adopted silently because its pre-send discipline was
not established. Future schemas, unsafe paths, bad permissions, and linked
database/sidecar files are rejected.

The journal is private metadata. Never persist tokens, signing keys, raw
account payloads, signed transaction bodies, or logs in it. Use restricted,
encrypted local storage. The default `.cinder-operator/` directory is ignored
by Git. Back up SQLite consistently with its WAL; never delete a live WAL.
See [SQLite WAL handling](https://www.sqlite.org/wal.html).

## Verification and limits

```bash
cargo test --locked --workspace
cargo clippy --locked -p cinder-operator --all-targets --no-deps -- -D warnings
./scripts/test-ledger.sh
```

The operator tests cover the durable crash boundaries, lost replies, fact
deduplication, competing owners, WAL reopening after abrupt exit, credential
files, and foreign/stale/incomplete evidence. Local-validator tests cover
guarded replay and concurrent-write rejection, authorization, and halt-bit
preservation.

The real-QFS privacy suite kills and restarts the Rust operator before ack
signing and after fill/failure ack submission, reuses a client ID, and checks
convergence without duplicate fees/acknowledgements. QFS/Anchor execution is
real and local; its public Rise accounts/views/receipts are deterministic,
SDK-encoded fixtures with synthetic collateral. **It submits no Phoenix trade
and is not live-venue integration proof.** The other privacy checks verify
operator visibility and unrelated-user account/history redaction.

This experimental runtime is bounded to 99 users per coherent private read,
32 markets, 64 index/buffer accounts, and 10,000 history rows per scan. Pooled
history and raw receipts are reused within each recovery pass, then discarded;
the receipt cache is capped at 64 MiB of serialized JSON. Exceeding limits halts
instead of truncating. Missing archival history requires restoring
trusted RPC access; it cannot be worked around by assuming rejection.

Native SOL collateral, queued venue withdrawals, spline/conditional orders,
isolated trader/market state, and negative venue cash collateral are unsupported
and halt recovery. Pool-deficit resolution is a separate design, not an implicit
haircut or recapitalization performed by this command.

The whole-account read brackets are deliberately conservative: unrelated venue
activity or slow RPC reads can prevent a stable, fresh snapshot. Recovery stays
gated in that case; do not override this check to enable live execution.

Replay-safe money movements and further operational hardening remain R7 work.
Do not use this prototype with real user funds.
