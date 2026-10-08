# Pre-hardware release package

This is a **qualification build**, not a live brokerage deployment. The measured
version-1 manifest rejects all native activation; version 2 permits bounded
read-only qualification; version 3 additionally binds bounded retained S3 packs.
All three still reject trading and funding. Private SDK auth,
grants and views use the actual API and journal. No fixture funds, synthetic fill
or fake exposure is injected. P22 owns integrated offline financial scenarios;
P23 owns qualified venue/chain workflows. P20 hardware must still prove actual
NSM, vsock, KMS, cloud restore and fencing; an ELF/OCI build is not that proof.

The [runbook](../../docs/operations/nitro-qualification.md) specifies preparation,
permissions, evidence, cost and cleanup. The [ADR](../../docs/architecture/0020-nitro-runtime.md)
defines the current limitations and topology approval boundary.
P23's [chain-port ADR](../../docs/architecture/0024-confidential-chain-ports.md)
specifies the sixth key purpose, versioned preparation, private RPC and fixed
read-only handoff. Its [runbook](../../docs/operations/live-qualification.md)
governs the new live session; P20 permission/measurements cannot be reused.

## Build inputs and separation

Use only an exact committed source export, locked public vendor tree and the
immutable ARM64 Linux check image in `Containerfile`. The manifest is PUBLIC
measured configuration, not a directory containing private provisioning output.
Rootfs has only the default-feature enclave executable, five explicit runtime
libraries, its loader and that manifest. No Rust toolchain, shell, cloud CLI,
fixtures, wallet, `.git`, `work/`, owner directory, secrets or parent credential
belongs in it. OpenSSL's built-in default provider and kernel entropy still need
real measured-kernel observation; adding NSM entropy does not qualify other RNGs.

`package-check.sh` builds all required parent/operator tools **separately** from
the enclave rootfs, records ELF/library hashes, rebuilds the application crate
and compares the two ELF files. It runs standalone in the offline source-only
ARM64 Linux test container; running the whole Linux Rust suite first is not
required. `.github/workflows/nitro-package.yml` runs this check on every product
PR, using a committed source export and locked read-only vendor tree. The check
rejects a network interface, NSM device, research/git directories or stale bundle
outputs. An optional `/build-cache` mount reuses dependency compilation only;
CI's pinned Rust-cache action excludes workspace artifacts before saving. The
application crate is cleaned before the initial build and again for the
reproducibility comparison, so a cached application cannot replace either build.
The cache is keyed separately from host builds by the pinned tool image/vendor
configuration/package script and Cargo/Rust inputs. CI runs as the runner UID
so the cache stays writable without a privileged ownership-repair step.
Copy `/tmp/cinder-bundle` out before removing that container only when
you need the qualification artifacts; routine CI discards them. Parent binaries
are not included in the measured enclave.
The script's ordinary-host boot refusal is not a successful NSM test.

For an OCI build, create a new explicit temporary build context. Populate it only
with root Cargo/toolchain files, `crates/`, public `vendor/`, a Cargo vendor config
whose directory is `/vendor`, `Containerfile`, and the public `manifest.cbor`.
Do not point a builder at the live worktree or trusted preparation directory.
Use the current `container build --help` and a task-specific image name; public
dependency setup must be completed first. This CLI has no build `--network` flag.
The image's RUN uses Cargo offline. Record OCI/ARM64/config hashes and the exact
manifest hash. Repeat builds must be compared, not assumed identical from a tag.

Alternatively add a finalized PUBLIC manifest to the checked rootfs, make a
sorted tar with fixed numeric ownership/time on Linux, and import that into the
Nitro build host with the exact `cinder-enclave` entrypoint. The manifest may only
be finalized once the disposable resource identities/credential generation are
known; changing it changes the measured release and requires new PCR approval.
No ephemeral credential or plaintext role file may be added to the image.

EIF generation uses the pinned, recorded Nitro CLI and its kernel/init tooling
on the approved AWS host, then `nitro-cli describe-eif`. Record actual PCR0/1/2
and EIF SHA-256; do not treat an OCI digest as a PCR. The first accepted hardware
image MUST NOT use debug mode. Ship the SDK's 240-byte Policy (domain, full
manifest digest, actual approved PCRs) independently of the parent/quote.

## Binaries

| Location | Executable | Responsibility |
| --- | --- | --- |
| Enclave | `cinder-enclave` | NSM clock/recipient, KMS release, actual private API, encrypted replicated journal and bounded fresh-state supervisor |
| Parent | `cinder-nitro-relay` | Loopback TCP to fixed enclave CID/port; opaque TLS only |
| Parent | `cinder-cloud-relay` | Fixed manifest AWS endpoint on port 443; opaque TLS only |
| Parent | `cinder-nitro-egress` | Fixed native origin or explicit `solana-devnet`/`helius-devnet` HTTPS route; opaque TLS only, fixed manifest port |
| Parent | `cinder-bootstrap` | One-shot KMS ciphertext + already-parent-known temporary KMS/S3 credential, 60-second bound |
| Independent client | `cinder-verify-quote` | Production AWS-root verifier; independently expected SDK release policy |
| Trusted operator only | `cinder-prepare-release` | Validate actual private configuration/key roles; create 0700/0600 local provisioning files; no AWS calls |

The qualification relay tools stop on stdin closure. A supervisor must hold their
private control pipes open and close them on termination; do not let a systemd
unit with closed stdin repeatedly restart them. The enclave itself uses its fresh
witness/clock and finite boot lease, not parent stdin, for authority fencing.

The image recipe is an explicit build contract, not yet an EIF measurement
receipt. A Debian security update or different toolchain/provider is a new measured
release; never silently resolve a mutable base tag or unpinned library here.
