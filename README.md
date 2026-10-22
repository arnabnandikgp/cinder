# Cinder

Solana-native prime brokerage for perpetual futures: individual trading accounts,
shared venue execution, and confidential account processing in AWS Nitro Enclaves.

This repository contains Cinder's Rust backend, private TypeScript SDK and Anchor
custody/recovery program. The trading terminal, marketing website and public
Mintlify documentation are maintained separately.

## How it works

Customers authorize Cinder orders rather than giving clients control over a pooled
venue account. The enclave authenticates requests, checks margin and risk, records
the operation, and signs permitted native actions. Actual venue fills update each
customer's private balance and positions through one authoritative journal.

- **Private accounts:** balances, positions, orders and history are processed
  inside the enclave; the parent relays encrypted traffic.
- **Venue-specific execution:** Pacifica and opt-in BULK adapters share common
  accounting and API machinery without sharing collateral across venues.
- **Program-controlled custody:** identified Solana escrow batches are swept
  through an allowlisted funding route; normal and recovery payouts share payment
  protection and end at the registered wallet.
- **Verified client access:** the SDK checks enclave attestation and uses
  encrypted HTTP/WebSocket channels, owner signatures and scoped agents.

## Project status

Cinder is experimental software, not a customer-funds release. The financial
engine, custody/recovery program, confidential SDK and joined offline workflows
are implemented. Bounded AWS Nitro and native testnet runs provide evidence for
their exact source and scope. The latest BULK run passed an encrypted SDK opening
and fill attribution, but its close remains blocked by native decimal/funding
handling. Pacifica's missing-credit investigation remains deferred.

See [current status and remaining gates](docs/status.md). A green test suite or
merged implementation is not production qualification.

## Explore the project

| Read | Purpose |
| --- | --- |
| [Documentation guide](docs/README.md) | Find the right explanation or debugging entrypoint |
| [Architecture](docs/architecture.md) | Runtime, accounting, execution, custody and recovery flows |
| [Financial contract](docs/implementation/BASELINE.md) | Ownership, equations, risk policy and unresolved financial gates |
| [Development](docs/development.md) | Pinned toolchains, offline tests and CI coverage |
| [Private SDK](clients/private/README.md) | Client authentication, channels and operation semantics |
| [Contributor guide](AGENTS.md) | Repository boundaries and implementation safeguards |

## Local development

Use Rust **1.97.1**, Node **24.21.0**, a C compiler and OpenSSL development
libraries. After installing those prerequisites:

```sh
cargo fetch --locked
npm ci --ignore-scripts --prefix clients/private
node scripts/check.mjs
```

Dependency hydration requires network access; ordinary checks then use locked,
offline dependencies and local fixtures, not wallets, AWS or venue accounts.
For a quick documentation/repository check, run
`node scripts/check.mjs --group=contracts`. Browser and compiled-Solana checks
have their own prerequisites in [development](docs/development.md).

The separate `programs/` workspace pins Anchor **1.2.0**. Its SBF/Surfpool checks
run with `node scripts/check-vault.mjs`; see [program setup](programs/README.md).

Original research and private experiment artifacts remain in ignored `work/`.
They are not checkout or CI dependencies. Sanitized historical decisions and
qualification receipts are indexed in the [archive](docs/archive/README.md).
