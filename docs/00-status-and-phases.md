# Cinder planning status

Last updated: 2026-09-12

This file is the map of the planning session. Implementation happens in a git repo on your machine, not in the Grok sandbox.

## Phase status

| Phase | Intent | Status |
|---|---|---|
| 0 Frame / invariants | Privacy, keys, non-goals | Done. Encoded in `01-decisions.md`. |
| 1 HyperLink reconstruction | How the reference broker works | Done in chat. Not required to re-read to implement. Summary lives in `02-architecture.md`. |
| 2 Phoenix venue + accounts | PDA, authority split, caps, publicity | Done. Caps and topology frozen. |
| 2b Phoenix execution / POC slice | Ember, register, orders, recon | Done as design. See `04-phoenix-adapter.md`. POC is not a product ceiling. |
| 3 PER mapping | Accounts on ER vs L1, QFS auth, Magic Actions, local cluster | Done. See `05-per-local-dev.md` and `02-architecture.md`. |
| 4 Architecture freeze | Layouts, ixs, halt/escape | Done. `03-phase4-freeze.md` is source of truth for code. |
| 5 Implementation design / repo | Workspace layout, tool versions, test stages | Specified as handoff in `06-implementation-handoff.md`. Not executed. |
| 6 Dev practice | Audits, attestation, observability | Explicitly later. Do not block stage C. |

## What is left in *this* planning chat

Nothing required before you can start stage C (`mb-stack` ledger + QFS ACL).

Optional follow-ups only if you come back:

- Byte-level Anchor account sizes / realloc math
- Exact Rise lot conversion helpers per market
- Magic Action `settle_user_withdraw` account list once vault PDA exists
- Window>0 matcher design (v1, not first local win)
- Escape-withdraw merkle proof format (Q7, after reserve root is writing)

Do not reopen D1–D12 or Phase 4 layouts without a written change.

## Success bar

A full loop on **Surfpool (mainnet fork) + local ER + QFS** is a win:

two users, private ledgers, QFS token filter, residual hedge on a shared Phoenix trader PDA, net visible on Phoenix, gross visible only on ER.
