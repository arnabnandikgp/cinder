# Locked decisions

Do not reopen these during implementation. Change only with an explicit amendment.

## Product

| ID | Decision |
|---|---|
| Name | Cinder |
| What | Private perp prime broker for Phoenix on Solana |
| Reference | HyperLink on Hyperliquid: pooled venue account + private internal ledger + public edges |
| Privacy (Q1-A) | Attribution privacy only. Phoenix sees Cinder’s net trader. Users do not see each other. Deposit/withdraw edges on L1 are public. |
| Netting (Q2-C) | Windowed residual hedge is the v1 *shape*. First local win is window = 0. Not a dark pool. Phoenix already nets on one cross PDA. |
| Keys (Q3-A) | Phoenix `authority` = vault PDA (keypair stand-in until CPI vault exists). Hot `position_authority` = adapter/TEE key. |
| Surface (Q4-A) | API-first. Existing Cinder TUI is a different product. |
| Access gates (Q5) | No HyperLink-style invite graph. Phoenix no-referral register only. |
| Geo / ToS (Q6) | Deferred. Phoenix + MagicBlock restrictions exist; do not design around them now. |
| Exit (Q7-A) | Design target: L1 exit without operator. First win: write `ReserveRoot`, stub `escape_withdraw`. Named gap, not silent CEX. |
| Q8 | Name is Cinder. |

## Architecture defaults (D1–D8)

| ID | Decision |
|---|---|
| D1 | One Phoenix cross PDA `(portfolio=0, subaccount=0)` only. No isolated children. Skip `isolatedOnly` markets. |
| D2 | Window = 0. Book updates on Phoenix **ack**, not on `place_order`. Windowed residual is out of the implementation tracker. |
| D3 | Cinder IM/MM = 1.25× Phoenix on the *user* book. User leverage cap `min(10x, Phoenix first-tier max)`. Liquidate users before Phoenix can liquidate the pool. Pool buffer 20% of Phoenix IM(residual), floor 50 USDC. |
| D4 | Vault program PDA is Phoenix authority. Keypair stand-in accepted until that CPI exists. |
| D5 | Two cash buckets: vault ATA = free user cash; Phoenix trader = residual IM + buffer. |
| D6 | Full local cluster is the win, not a thin hosted POC. |
| D7 | PER is the v1 privacy box. Stage C (`mb-stack`) before Surfpool Phoenix. |
| D8 | No Cinder invite system. |

## PER / local (Q9–Q12)

| ID | Decision |
|---|---|
| Q9-C | User *and* adapter are members on each `UserLedger`. User: view flags only. Adapter: `AUTHORITY` + view. Do not store user TEE/QFS tokens on the server. |
| Q10 | Stage C = `mb-stack`. Stage A = Surfpool forked from **mainnet** + local ER + QFS. |
| Q11 | L1 money movement is adapter cranks until vault PDA exists. Then Magic Action `settle_user_withdraw`. Then collateral actions. Never attach Phoenix orders to a commit. |
| Q12 | No session keys in the first local win. |

## Phoenix constraints (facts, not choices)

- Cross account: `max_positions` 32–128 (use 128), 64 bids + 64 asks per market.
- Isolated child: `max_positions = 1`, public, kills Q1-A. Do not use.
- Extra `pda_index` / portfolios: not activatable under current gating.
- One market on one trader PDA = one net position. Opposing users net at the venue automatically.
- `position_authority` cannot withdraw or delegate.
- No-referral register allows off-curve `traderAuthority` (PDA).
- Trader state is public by authority pubkey (Phoenix API + chain).
- Ember wraps Solana USDC 1:1. Global withdraw queue exists.
- Some markets are `isolatedOnly`. Out of scope.
- Active Trader Buffer / on-chain trader PDA leak pool inventory. Intended.
