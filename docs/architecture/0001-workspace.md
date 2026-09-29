# ADR 0001: deterministic Rust kernel and explicit I/O boundaries

Date: 2026-09-30. Status: merged in P01 ([PR #24](https://github.com/arnabnandikgp/cinder/pull/24)).
Scope: workspace/tooling, not financial policy, runtime crypto selection or deployment.
[Plan](../implementation/PLAN.md#p01), [baseline](../implementation/BASELINE.md),
[development checks](../development.md).

## Decision and rationale

Use Rust for the accounting/runtime foundation, pinned to **1.97.1**, edition 2024.
The dependency-free kernel is `no_std`, with unsafe code forbidden and explicit
state/event inputs. Rust supports exact checked integer arithmetic, typed ownership
boundaries and native Linux builds for the eventual enclave. These properties help
implementation; they do not prove economic correctness, privacy or durable commit.
The chosen installed stable patch includes the upstream compiler correction; it
is a reproducible baseline, not a promise always to track latest stable.

Use **Node 24.21.0** only for repository checks now; it is pinned independently of
the financial runtime. Later SDK/browser decisions belong to P18/P19. Keep one
Cargo workspace/lockfile, explicit member paths, exact local dependency versions,
no third-party Rust/JS dependencies and no build scripts at this stage. Commit the
lockfile and forbid publication of these scaffold crates. No new license grant is
assumed. Toolchain updates require a tested PR, not a moving `stable`/`latest` pin.

## Packages and permitted edges

| Package | Role | Dependencies |
| --- | --- | --- |
| `crates/kernel` / `cinder-kernel` | Pure transition seam; P02/P03 add actual types and financial behavior | None, including no I/O/SDK/runtime or test framework |
| `crates/ports` / `cinder-ports` | Clock, journal and venue test seams; no implementations | None; no dependency back into controllers or adapters |
| `crates/test-support` / `cinder-test-support` | Manual time, in-memory commit faults, scripted venue replies and harness checks | Ports normally; kernel only as a dev dependency |

The dependency guard checks all Cargo dependency kinds/targets and unexpected
packages/build scripts. Source checks protect the declared `no_std` boundary and
obvious explicit `std` escapes; these are review guardrails, not a security sandbox
or a proof against malicious future source. Future kernel features must remain
explicit and deterministic. Test-support must not become a production dependency.

All Rust source denies unsafe code and missing public documentation. Release builds
keep overflow checks enabled, but **checked arithmetic and typed errors remain
mandatory** for future money operations. A release panic is not a correct rejection.

The initial ports are intentionally synchronous and generic. They are test seams,
not finalized storage or SDK APIs. P05 adds atomic batch/version/hold/consumed-key
semantics and typed uncertain commit outcomes; P07/P14 add authorized exposure,
native attempts and reconciliation. Do not wrap a real database/venue in these
minimal traits and call it production-ready. No generic event bus, database,
async runtime or crypto implementation is selected prematurely.

## Error and evidence boundaries

- Kernel rejection computes no externally committed state. Returning a next state
  is not durability or dispatch permission.
- Store errors can occur before commit or after commit with a lost reply. The fake
  distinguishes them; a real adapter must supply qualified outcome evidence.
- Venue reply is not a fill; transport error is not rejection. The fake can accept
  without replying, reject, or exhaust its script. Exhaustion never falls back to
  a real network. Test introspection is not a capability given to controllers.
- Fake time has caller-defined logical ticks, including backwards changes. It is
  not authoritative venue time, finality, cryptographic freshness or a global order.
- Current property checks exhaust all 65,536 u8 counter/input pairs and 243 five-step
  fault schedules. They exercise the harness, not Cinder accounting. P02/P03 must
  port actual financial cases before claiming financial implementation coverage.

## Alternatives and reuse

The CJS studies remain valuable exact-integer references, not production ledgers.
Using them wholesale would preserve separate projections and in-memory assumptions.
A TypeScript financial service could use BigInt, but would require a separate
native boundary and additional discipline around wire decimals and runtime state.
Go is viable for services, but does not improve reuse of the Rust custody/Nitro
experiments enough to justify a second initial implementation language.

Catalog starters such as `anchor-by-example`, `program-examples` and frontend
templates do not fit the whole broker/enclave architecture. Do not clone a DEX or
website scaffold or pull in default AMM/LP economics. Reuse the scoped M1/M2 and
financial tests per the [evidence register](../implementation/EVIDENCE.md), and
consider a narrowly selected program example at P15 instead. Existing scaffolding,
DeFi and Apple-container skills support this work; later Solana inspection via a
read-only RPC/MCP is an opt-in qualification tool, not a dependency of local tests.
No tool/skill installs or external telemetry are required for this workspace.

## Deferred targets and gates

| Target | Owner | Why not claimed now |
| --- | --- | --- |
| Linux native | P01 | Verify actual builds/tests on Linux; a successful macOS run alone is insufficient |
| Solana SBF and Anchor/native program selection | P15/P17 | No program in P01; bundled SBF compiler compatibility needs its own pin and local instruction tests, not inherited Phoenix versions |
| Enclave image/NSM/KMS/TLS and witness | P19/P20 | Linux library builds are not Nitro qualification; no hardware/crypto key policy selected here |
| Public SDK/browser verification | P18/P19 | No browser runtime, transport ABI or wallet encryption choice in this PR |
| Live native venue actions | P13/P14/P23 | Fake ports grant no custody authority, fee precision or native parity |

Repository integration follow-up: propose required TEE documentation/workspace
checks, scoped CodeRabbit review for `product/tee-v1` and `tee/*` bases, and exclusion
of TEE branches from the legacy Vercel project. Changing hosted protection/review/
deployment policy needs authorization; do not add a dummy website to silence it.
Ordinary branch pushes create existing previews under existing repository settings;
no new deployment target is being configured by this ADR.

Sources checked 2026-09-30: [Rust 1.97.1 release](https://github.com/rust-lang/rust/releases/tag/1.97.1),
[Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html),
[Cargo test/offline flags](https://doc.rust-lang.org/cargo/commands/cargo-test.html),
[Node 24 release archive](https://nodejs.org/en/download/archive/v24),
[Node release checksums](https://nodejs.org/dist/v24.21.0/SHASUMS256.txt).
