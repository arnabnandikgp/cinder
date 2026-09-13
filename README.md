<p align="center">
  <img src="assets/cinder-header.png" alt="Cinder" width="620" />
</p>

# Cinder

**Private perp prime brokerage for [Phoenix](https://www.phoenix.trade/) on
Solana.** Cinder keeps individual positions in MagicBlock private ephemeral
rollups (PER) and routes only the aggregate residual to Phoenix.

> Experimental software. It is a protocol prototype, not production-ready
> custody or trading infrastructure.

## Why Cinder

- **Private user books:** individual balances, positions, and order state live
  on a private rollup.
- **Public net execution:** Phoenix sees one Cinder trader and Cinder's net
  exposure, not every underlying user.
- **Explicit execution state:** orders are tentative until a Phoenix fill is
  acknowledged; failed venue orders unwind through a dedicated failure path.
- **Verifiable local privacy:** QFS token tests verify that users can read only
  the accounts they are permitted to access.

## Architecture

```text
Users ── QFS ──► MagicBlock ER user ledgers ──► Cinder adapter ──► Phoenix
                       private state                 net residual       L1 venue
```

The on-chain vault holds shared collateral and configuration on Solana L1. The
ledger program maintains private user state on the ER. The adapter is the
operator process that reconciles the net residual with Phoenix.

Read [the architecture guide](docs/architecture.md) for the account model and
execution lifecycle, and [the security model](docs/security-model.md) for the
protocol boundaries.

## Quick start

Prerequisites: Node 24, Rust 1.89, Solana/Agave 3.1.9, and Anchor 1.0.2.

```bash
yarn install --frozen-lockfile
yarn build

# Terminal 1: base validator, ephemeral rollup, and QFS
./scripts/stack-local.sh

# Terminal 2: privacy and netting walkthrough
yarn demo
```

The demo deploys the Cinder programs to an empty local stack, verifies QFS
isolation, applies offsetting fills, and closes a position with PnL.

## Website demo

The repository also includes a product site and terminal walkthrough. It uses
live, read-only Phoenix market data alongside deterministic Cinder execution
scenarios; it deliberately does not connect a wallet or route live orders.

```bash
yarn --cwd web install
yarn web:dev
```

The landing page is available at `/`; the interactive terminal is at
`/terminal`. Build it with `yarn web:build`.

## Repository map

| Path | Purpose |
| --- | --- |
| `programs/cinder_vault` | L1 configuration, collateral vault, and reserve state |
| `programs/cinder_ledger` | ER user ledgers, order state, permissions, and net book |
| `crates/cinder-adapter` | Operator logic and Phoenix integration boundary |
| `crates/cinder-common` | Shared protocol constants and pure accounting helpers |
| `web` | Product site and deterministic terminal walkthrough |
| `tests` | Ledger, privacy, netting, collateral, and venue integration tests |
| `docs` | Public protocol, security, and development documentation |

## Development

```bash
./scripts/test-ledger.sh       # local Anchor ledger tests
./scripts/test-privacy.sh      # QFS / ER privacy tests (stack required)
./scripts/test-netting.sh      # two-user netting test (stack required)
cargo test --workspace         # Rust unit tests
```

See [development notes](docs/development.md) for local endpoints, test setup,
and toolchain details.

## License

MIT
