# Operator recovery

`cinder-adapter` is the decision core. `cinder-operator` owns the private
SQLite journal and serialized recovery, with authenticated QFS and official
Phoenix/Rise RPC ports.

The `recover` command runs one startup/recovery pass. It can sign guarded
private fill/failure acknowledgements and operator-down updates; it **does not
submit new Phoenix orders**. Before exiting it confirms OPERATOR_DOWN on both
L1 Config and private Book. Successful recovery is not permission to trade or a
production-readiness claim. Autonomous execution and maintenance come later.

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
```

Recover exits 0 only when no operations remain unresolved and reconciliation
succeeds; 3 means recovery remains gated, 1 means a runtime error, and 2 indicates
invalid CLI usage. Output contains aggregate counts and typed error/reason codes,
not credentials, private balances, user identities, or raw receipts.

## Durable causality and acknowledgements

A client order ID is not globally unique. The journal binds the user, ledger,
placement nonce, client ID, and order kind to a deterministic global venue ID.
The full hash is retained and truncated-ID collisions are rejected. Runtime
binding also pins the Cinder deployments, operator, pooled trader, venue, and
market mappings; a different deployment cannot silently adopt the journal.

Startup compares a durable user registry against L1 initialized/delegated user
identities and the complete operator-authorized QFS scan. It reads private Book
and ledgers together, then rechecks the registry. Missing or redacted accounts
halt recovery rather than becoming an empty pool.

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

Partial fills remain unresolved for live execution/remainder handling. A
confirmed bound violation remains a fill fact to acknowledge and halt over;
recovery must not erase real exposure by treating that fill as a rejection.
This command neither dispatches nor cancels resting orders.

Reconciliation compares confirmed private positions (tentative positions minus
pending deltas), acknowledged Book, and venue exposure. Cash includes explicit
user debt and unsettled user/pool funding. A deferred withdrawal is unsupported
until the money-movement outbox exists; cash-in-flight is never invented as zero
over such a withdrawal.

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

## Ownership, gates, and storage

Startup confirms OPERATOR_DOWN before recovery. Both Config and Book updates
modify only that bit, preserving administrative, bad-debt, venue-breach, and
other economic halt reasons. The recovery-only process closes both gates again
before releasing ownership, even after successful reconciliation.

Use one host, one exclusive operator key, and one shared private
`pool_lock_directory` for every process controlling the same trader. The
pool-scoped lease is independent of the journal path and operator credential.
Different local journal paths cannot concurrently acquire the same pool lease
in that directory. Separate directories/hosts are **not** fenced; replicas and
manual/foreign activity on the pooled trader are unsupported. Abrupt process
death is not a chain-level liveness watchdog; that belongs to later maintenance.

SQLite schema 2 adds runtime binding, durable users, and prepared ack attempts.
Schema 1 upgrades transactionally. An unbound legacy journal with historical
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

Next work is live bounded IOC dispatch, partial/fee/remainder handling and
collateral management (R5), autonomous funding/liquidation/heartbeat/root/halt
loops (R6), then replay-safe money movements and operational hardening (R7).
Do not use this prototype with real user funds.
