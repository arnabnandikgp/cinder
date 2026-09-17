# Operator recovery

Cinder's runtime foundation separates decision logic from durable execution.
`cinder-adapter` calculates protocol decisions. `cinder-operator` journals the
order lifecycle and coordinates recovery through injectable venue and private
ledger interfaces.

This is not a live trading service. The binary manages a local journal; it does
not authenticate against QFS, sign transactions, or connect to Phoenix. Test
implementations of the interfaces exercise recovery without external endpoints.

## Durable causality

A user's client order ID is not globally unique. The journal correlates the
user, ledger, nonce, and client order ID with a deterministic global venue ID.
The full hash is retained and a uniqueness constraint rejects truncated-ID
collisions instead of replacing another operation.

The recovery lifecycle is:

```text
durable bounded intent → venue submission → authoritative venue outcome
                                            ├─ fill → private fill acknowledgement
                                            └─ zero fill → private fail acknowledgement
```

Unknown submission outcomes stay unresolved. An elapsed deadline, lost network
response, or process restart is not proof of rejection. Recovery checks the
venue before retrying a submission; it must not accidentally create another
public order for the same private operation.

Fill facts are durable before an acknowledgement is sent. A confirmed fill
cannot switch to a failure path. After a lost acknowledgement response, the
coordinator checks the private ledger before retrying. Confirmation must match
the operation, not merely an older row with the same user-selected client ID.

Partial fills with a live or unknown remainder remain unresolved. Complete
live event aggregation and cancellation handling belong in the venue execution
implementation; recovery does not invent those facts.

## Entry gates

Recovery starts with entries disabled. Unresolved operations, stale observations,
or failed position/cash reconciliation prevent enabling them. Fill recovery
takes priority over failure acknowledgements. Other protocol halt reasons must
not be cleared simply because restart recovery succeeded.

Position reconciliation compares the acknowledged aggregate book with Phoenix
exposure. Cash reconciliation includes explicit debt and unsettled funding.
Production interfaces must obtain complete, fresh authoritative observations;
test fixtures are not a substitute for these observations in deployment.

## Journal handling

SQLite WAL transactions persist operation identity, bounded intent, state, and
minimal acknowledgement/fill correlation facts. The journal contains private
operator metadata even when the individual identifiers are public keys.

- Never store bearer tokens, signing keys, raw private account payloads, or
  transaction logs in the journal.
- Use a dedicated private directory on a local encrypted volume. The default
  repository-local runtime directory, `.cinder-operator/`, is ignored by Git.
- Only one active journal owner is supported. A local lock is not distributed
  leadership or fencing across independent journals.
- Back up SQLite consistently. Do not copy a live database without its WAL or
  delete WAL files manually; committed state may still reside there. See the
  [SQLite WAL documentation](https://www.sqlite.org/wal.html).

## Tests and remaining work

```bash
cargo test --locked -p cinder-operator

# Local journal management only; no network requests or trading.
cargo run --locked -p cinder-operator -- init
cargo run --locked -p cinder-operator -- status
```

Both journal commands require exclusive ownership of the journal. `status`
prints aggregate state counts, never user identities or fill details. Existing
unsafe file permissions, linked journal files/sidecars, and unknown schema
versions are rejected. On Unix, newly created directories are owner-only and
database/lock/sidecar files are restricted to the operator.

Recovery tests reopen an on-disk journal while keeping the simulated venue and
private ledger alive. They exercise the boundaries around submission, persisted
venue outcomes, acknowledgement, and terminal confirmation. A subprocess test
also exits without SQLite cleanup to verify committed WAL recovery and lock
release.

The next integration slice supplies real QFS/Rise connections, private-user
discovery, authoritative history recovery, and startup reconciliation. Subsequent
work adds live bounded IOC execution, fee/partial-fill handling, collateral
management, autonomous maintenance loops, and replay-safe deposits/withdrawals.
This foundation alone is not a production deployment or mainnet-readiness claim.
