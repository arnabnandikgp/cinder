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

# Durable journal and crash/restart tests; no RPC or QFS stack required.
cargo test --locked -p cinder-operator
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

The venue, funding and runtime fork tests share a localhost-only Phoenix
fixture. It activates the local exchange and clears the copied mainnet restart
acknowledgement, since Surfpool has a different restart lifecycle and no venue
admin crank. Economic parameters and account bindings are preserved. This
tests native integration, not the upstream exchange's current availability.

The extended runtime acceptance test starts a separate disposable Phoenix fork
and a real PER/QFS stack, then runs the autonomous operator:

```bash
CINDER_R5_FORK=1 CINDER_R6_MAINTENANCE=1 ./scripts/test-runtime-fork.sh
```

It exercises private trading and exact-signature crash recovery, 24 observed
hourly funding generations, all-user allocation, actual native settlement,
restart during a private cash fold, quiet-book liquidation, guarded reserve
publication, delegated fee readiness, and graceful shutdown. The 24 generations
are replayed in an accelerated test; this is not a 24-hour soak test.

The fork has no live oracle/funding cranks. Local-only fixtures refresh frozen
native price-component clocks and update the native funding accumulator. All
Hawkeye reads, order execution, settlement, private allocation and custody
transfers use actual programs and accounts, not financial RPC stubs. Disposable
accounts receive local genesis funding. No transactions go to public networks.
