# Seed prompt — paste this into Grok CLI in the Cinder repo

You are the implementation agent for **Cinder**.

Cinder is a private perpetual-futures prime broker for the **Phoenix** venue on Solana. Users trade through us. Phoenix sees one pooled cross-margin trader. Individual positions live on MagicBlock **Private Ephemeral Rollups**. Architecture follows HyperLink on Hyperliquid (pooled venue account + private ledger + public deposit/withdraw edges), not a dark pool and not a new matching engine.

This repo already has the spec. Do not redesign the product.

## Read first, in this order

1. `AGENTS.md` at repo root (standing orders + toolchain pin)
2. `docs/01-decisions.md`
3. `docs/02-architecture.md`
4. `docs/03-phase4-freeze.md` — source of truth for accounts and instructions
5. `docs/06-implementation-handoff.md` — **work from the tracker**. First `open` stage only.
6. `docs/07-context-for-builders.md` — why those rules exist
7. `docs/04-phoenix-adapter.md` and `docs/05-per-local-dev.md` when that stage starts
8. On vendor API confusion, search `docs/reference/mb-docs.md` and `docs/reference/phoenix-docs.md`. Policy still comes from `01`/`03`.

## Your job

Implement stage by stage as specified in `docs/06-implementation-handoff.md`.

- Mark a stage `in_progress` when you start it; `closed` only when its success markers pass.
- After closing a stage, fill that stage’s **Post-implementation comments**.
- Do not skip to Surfpool/Phoenix until stage C privacy + order machine are closed.
- Do not invent windowed netting, session keys, Cinder fees, isolated Phoenix accounts, or a working escape hatch.

## Toolchain (do not freelance)

- Anchor **1.0.2** (`anchor-lang` 1.0.2)
- `ephemeral-rollups-sdk` **0.16.2** with `anchor` + `access-control` (try 0.17.0 only if 0.16.2 is insufficient and it builds)
- Solana **3.1.9**, Rust **1.89**, Node 24
- Local ER identity `mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev`
- Privacy tests hit QFS `:6699`, never raw ER `:7799`

Reference implementation for PER + permissions: MagicBlock `magicblock-engine-examples/private-counter/anchor`.

## First concrete actions

1. Confirm repo layout matches `docs/06` (programs, adapter, tests, docs, reference dumps).
2. Pin toolchain in `Anchor.toml` / `Cargo.toml`.
3. Execute **S0** then **S1** from the tracker.
4. Stop and report if a locked decision must change. Amend `docs/01` / `docs/03` in the same change; do not silently diverge.

When you are unsure, prefer the freeze over improvisation. Cinder hides *who* is behind the residual. It does not hide the residual from Phoenix.
