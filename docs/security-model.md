# Security model

## Core invariants

- Cinder uses a single Phoenix cross trader. It does not create per-user
  Phoenix accounts.
- A venue order is never treated as filled until the ledger receives a fill
  acknowledgement.
- The aggregate book must equal the sum of filled private user positions and
  the position observed at Phoenix.
- User QFS credentials are never stored by the adapter.
- User orders are not attached to Magic Actions or commit transactions.

## Credentials and access

QFS tokens are user-held, short-lived access credentials. The adapter uses an
operator credential for the minimal set of rollup accounts required to
reconcile Cinder's book. A raw ER RPC endpoint is useful for local operations
but does not provide user privacy guarantees.

## Failure behavior

The ledger keeps order state explicit: pending orders are either acknowledged
as fills or acknowledged as failures. Halt flags can block new entries,
withdrawals, or deposits when a safety condition is detected. The adapter is
expected to stop entering new venue risk when reconciliation fails.

The operator journal preserves order causality across restarts. Unknown venue
outcomes halt entries and require reconciliation; a timeout must never be used
as proof that Phoenix did not fill. The journal is private metadata and must be
kept on restricted, encrypted storage without bearer tokens or signing keys.
The runtime uses one operator QFS token in memory. Placement receipts establish
the exact user/ledger/nonce/client-ID identity; guarded acknowledgements also
check a hash of the observed private ledger to reject concurrent-write races.
The placement nonce remains an adapter attestation, not an independently stored
field in each on-chain OID. The honest-operator trust assumption still applies.

A pool-scoped local lease supplements the SQLite lock. All processes controlling
the same pooled trader must share one private lock directory on one host;
independent directories or hosts are not fenced. Exclusive operator signing-key
ownership is required. Recovery is not a distributed execution service.
See [operator recovery](operator.md) for configuration and remaining limitations.

## Scope

This repository is a prototype. It has not received a production security
audit, and its local demonstration environment is not a production deployment
recipe. Do not use it with real user funds.
