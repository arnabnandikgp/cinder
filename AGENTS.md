# Cinder contributor guide

Cinder is an experimental private perp prime broker for Phoenix on Solana. Its
user-level state is private to MagicBlock PER, while Phoenix receives only the
aggregated Cinder position. Start with the public documentation in `docs/`.

## Protocol invariants

- Cinder uses one Phoenix cross trader; do not introduce isolated trader
  accounts.
- Phoenix sees Cinder's net exposure only. Per-user positions belong in ER
  `UserLedger` accounts.
- `place_order` is tentative. Update `Book` only after
  `ack_phoenix_fill`; restore tentative state through `ack_phoenix_fail`.
- Keep residual netting immediate (`window = 0`) unless the protocol design is
  explicitly changed.
- Never persist user QFS tokens in the adapter. The adapter uses one operator
  credential.
- Phoenix orders are independent of Magic Actions and commits.

## Toolchain

| Tool | Version |
| --- | --- |
| Anchor CLI and `anchor-lang` | 1.0.2 |
| `ephemeral-rollups-sdk` | 0.16.2 |
| Solana / Agave | 3.1.9 |
| Rust | 1.89.0 |
| Node | 24.x |

Use exact Cargo pins for Anchor and the ER SDK. The local privacy endpoint is
QFS at `http://127.0.0.1:6699`; raw ER at `:7799` is not a privacy test.

## Useful commands

```bash
yarn build
./scripts/test-ledger.sh
./scripts/stack-local.sh
yarn demo
```
