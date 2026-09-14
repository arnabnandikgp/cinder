# Development

## Toolchain

| Tool | Version |
| --- | --- |
| Node | 24.x |
| Rust | 1.89.0 |
| Solana / Agave | 3.1.9 |
| Anchor CLI and `anchor-lang` | 1.0.2 |
| MagicBlock ER SDK (Rust) | 0.16.2 |
| MagicBlock ER client (TypeScript) | 0.14.3 |

Install dependencies with `yarn install --frozen-lockfile`, then build with
`yarn build`.

## Local services

`./scripts/stack-local.sh` starts an isolated local environment:

| Service | Endpoint |
| --- | --- |
| Solana base validator | `http://127.0.0.1:8899` |
| Ephemeral rollup | `http://127.0.0.1:7799` |
| Query Filtering Service | `http://127.0.0.1:6699` |

The script uses separate temporary ledgers for the base validator and the ER
so a new invocation starts from a clean state. Stop the stack with `Ctrl-C`.

## Tests

```bash
./scripts/test-ledger.sh
./scripts/test-collateral.sh

# Require the local stack in another terminal.
./scripts/test-privacy.sh
./scripts/test-netting.sh
```

The repository CI also exercises Phoenix integration against a Surfpool fork.
Those tests depend on a reachable mainnet RPC and are therefore more suitable
for CI than a quick local check.

To run that fork locally, supply your own authenticated mainnet RPC URL without
committing it:

```bash
SURFPOOL_RPC_URL="https://your-mainnet-rpc.example" ./scripts/stack-fork.sh
```

CI reads the URL from the `SURFPOOL_RPC_URL` GitHub Actions secret.
