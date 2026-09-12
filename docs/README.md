# Cinder spec pack

Copy this folder to the implementation repo as `docs/`.
Copy `AGENTS.md` again to the **repo root**.
Paste `SEED-PROMPT.md` as the first Grok CLI message.

Vendor dumps are in `reference/` (`mb-docs.md`, `phoenix-docs.md`). Keep them at `docs/reference/` in the repo.

| File | Use |
|---|---|
| `AGENTS.md` | Standing orders (also at repo root) |
| `SEED-PROMPT.md` | First CLI message |
| `00-status-and-phases.md` | Planning status |
| `01-decisions.md` | Locked choices |
| `02-architecture.md` | System picture |
| `03-phase4-freeze.md` | Accounts / ixs — code source of truth |
| `04-phoenix-adapter.md` | Rise / Ember / hedge |
| `05-per-local-dev.md` | QFS, mb-stack, Surfpool |
| `06-implementation-handoff.md` | Tracker + stage specs |
| `07-context-for-builders.md` | Why the rules exist |
| `08-funding-allocation.md` | Agreed funding overlay: two-phase health/cash, `Book.funding_epoch` |
| `09-liquidation-liveness.md` | Agreed: Cinder-book liq scanner, I1-safe flatten, PER 10ms plan |
| `10-open-gaps.md` | Backlog of missing overlay pieces (not freeze; plan before building) |
| `reference/mb-docs.md` | MagicBlock dump, API confusion only |
| `reference/phoenix-docs.md` | Phoenix / Rise dump, API confusion only |
