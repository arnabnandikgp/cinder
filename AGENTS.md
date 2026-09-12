# AGENTS.md — Cinder implementation

You are implementing **Cinder**, a private perp prime broker for **Phoenix** on Solana. Privacy is MagicBlock **PER** (private ephemeral rollups). Venue matching stays on Phoenix L1. This file is standing orders. Re-read it when you drift.

## How to use this repo

1. Product decisions: `docs/01-decisions.md`
2. Picture: `docs/02-architecture.md`
3. Code source of truth: `docs/03-phase4-freeze.md`
4. Phoenix wiring: `docs/04-phoenix-adapter.md`
5. PER / QFS / local cluster: `docs/05-per-local-dev.md`
6. What to build next: `docs/06-implementation-handoff.md` (tracker + stage specs)
7. Why those choices exist: `docs/07-context-for-builders.md`
8. What is still missing (backlog, not freeze): `docs/10-open-gaps.md`
9. Raw vendor dumps when an API is ambiguous:
   - `docs/reference/mb-docs.md` (MagicBlock)
   - `docs/reference/phoenix-docs.md` (Phoenix / Rise)
   - optional: `docs/reference/hl-complete.md` (HyperLink reference only)

If spec and dump disagree on **Cinder policy**, the spec wins. If they disagree on **vendor API shape**, the dump (or live vendor docs/examples) wins, then amend the spec.

## Non-negotiables

- One Phoenix **cross** trader PDA `(pda_index=0, subaccount_index=0)`. Never isolated children. Skip `isolatedOnly` markets.
- Phoenix sees **Cinder net only**. User books live on ER `UserLedger`.
- `place_order` is tentative on the user ledger. **`Book` moves on `ack_phoenix_fill`**, not on place. Fail path is `ack_phoenix_fail`.
- Window = 0. Do not implement windowed netting. Same account layout can absorb it later; it is not a tracker stage.
- Do not store user TEE/QFS tokens on the server. Adapter holds one operator token.
- Do not attach Phoenix orders to Magic Actions / commits.
- Do not implement session keys, Cinder fees, invite graph, stealth eSPL, or working `escape_withdraw` until their stage is Open and previous stages are Closed.
- Name collision: `cinder.trading` / `cosmic-markets/cinder` is an unrelated Phoenix TUI. Do not depend on it.

## Toolchain pin

Match MagicBlock **private-counter (Anchor)** + PER quickstart, not random latest.

| Tool | Pin | Why |
|---|---|---|
| Anchor CLI + `anchor-lang` | **1.0.2** | `private-counter/anchor/Anchor.toml` and PER quickstart |
| `ephemeral-rollups-sdk` | **0.16.2** first (example). **0.17.0** allowed if it builds clean with `features = ["anchor", "access-control"]` | Example uses 0.16.2; crates.io latest 0.17.0 supports `anchor-lang ^1.0` |
| Solana / Agave | **3.1.9** as in PER quickstart (fallback 2.3.13 only if 3.1.9 fights the local validator) | Official PER table |
| Rust | **1.89.0** (PER table) | |
| Node | **24.x** | |
| Local ER identity | `mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev` | Never delegate local PDAs to mainnet/devnet TEE ids |
| Privacy RPC | QFS `http://127.0.0.1:6699` | ER `:7799` is not a privacy test |

```toml
# programs/.../Cargo.toml (sketch)
# Exact `=` pins. Cargo `version = "1.0.2"` is ^1.0.2 and will pull 1.2.x.
anchor-lang = { version = "=1.0.2", features = ["init-if-needed"] }
ephemeral-rollups-sdk = { version = "=0.16.2", features = ["anchor", "access-control"] }
```

Anchor CLI 1.0.2 `anchor test` defaults to Surfpool. Until S5, run `./scripts/test.sh` or `anchor test --validator legacy`.
