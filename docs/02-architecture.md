# Cinder architecture

## One sentence

Users hold private positions on a MagicBlock Private Ephemeral Rollup. All Phoenix trading is one shared cross-margin trader. Only deposits, withdrawals, and Cinder’s net inventory are public on Solana.

## Worlds

```
user wallet
     │
     ├─ QFS :6699?token=user     → ER :7799  → private UserLedger
     │
     └─ Surfpool/L1 :8899        → vault USDC, Ember, Phoenix trader (0,0)
```

Phoenix matching does not run on the ER. Magic Actions must not place Phoenix orders (commit would revert if Phoenix rejects).

## Components

| Piece | Where | Job |
|---|---|---|
| `cinder_vault` | L1 | Config, vault ATA, reserve root, Ember/Phoenix collateral, user USDC out |
| `cinder_ledger` | ER after delegate | UserLedger, Book, FeeAccrual, permissions |
| Adapter | Server process | Operator QFS token, Rise client, residual hedge, cranks, halt |
| QFS | Local 6699 / prod TEE URL | Token-gated filter over ER state |
| Phoenix + Ember | L1 (fork or mainnet) | Venue book, mark, liquidations |

## Mapping from HyperLink

| HyperLink | Cinder |
|---|---|
| Nitro enclave ledger | PER UserLedger + Book |
| Shared HL account | Phoenix trader PDA (0,0) |
| Master key | Vault PDA / stand-in authority |
| Agent key | `position_authority` in adapter |
| Signed private /info | QFS `getAuthToken` + membership |
| HyperEVM contracts | `cinder_vault` + ReserveRoot |
| Always-exitable | ReserveRoot now; escape ix later |
| Pooled fee tier | Not a Phoenix feature; ignore for v0 |
| Browser TWAP | Out of scope |

## Data plane

1. Deposit USDC to vault ATA (public).
2. Adapter `credit_deposit` on that user’s ER ledger (private).
3. User `place_order` on ER. Tentative user position. Pending oid.
4. Adapter computes intended residual = Book + pending, checks pool health, posts collateral if needed, sends Phoenix market order.
5. `ack_phoenix_fill` or `ack_phoenix_fail`.
6. Invariant: after ack, Book residual == Phoenix lots.

## Trust (honest)

Local win trusts the adapter key, QFS-without-TDX, Surfpool, and forked marks.

v1 on real TEE adds TDX attestation, vault PDA authority, Magic Action withdraw with escrow bound to that PDA, and a used reserve root.

PER hides *who* is behind the residual. It does not hide Cinder’s net on Phoenix.
