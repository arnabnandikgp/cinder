# ADR 0042: Explicit measured two-unit deployment assembly

Status: implemented for offline assembly; native workers and changed-image Nitro
qualification remain separate gates (P39/P40).

## Decision

Manifest version 7 selects the existing unit Runtime/Handler on the real enclave
entrypoint. Versions 1–6 retain the legacy private schema and journal. No fallback,
conversion or state import joins them. Both existing TLS and attested Noise
carriers receive the same unit handler; the parent remains an opaque relay.

The public unit policy pins two distinct unit IDs, the canonical registry digest
and a separate fixed-origin BULK testnet tunnel/trust anchor. Pacifica continues
to use its explicit testnet egress. One shared journal uses the existing retained
S3 packs, RecordCipher and independent DynamoDB witness. Host objects cannot
select the history format, reset an epoch or substitute a witness credential.

Seven separate KMS purposes are required: global configuration, Pacifica
configuration, BULK configuration, storage, Pacifica trading, BULK trading and
witness. Existing role codes 1–6 do not change. Configurations are split because
KMS Encrypt plaintext is bounded; each wrapped plaintext must fit 4096 bytes.
There is no unrestricted master, broker-owner key or Solana funds key in v7.
The global private configuration includes a fresh secret random 32-byte nonce:
its public body hash must not become an oracle for guessed owner inventories.
This is blinding, not another signing authority; zero/missing nonce is refused.
Agent seeds must be distinct from each other/storage, and neither agent public
key may equal either native owner/funding wallet or a managed customer's wallet.

The application component is derived from actual parsed unit configurations,
API authorization policy, read-only gateways, egress and history limits. The
complete manifest additionally binds replica/witness/key-release routes and
slot body hashes. Trusted preparation derives the registry/application; the
parent cannot provide those as authority. Preparation validates every wrapped
plaintext before creating private output files.

## Deliberately inactive capabilities

Version 7 refuses trading, funding and native polling. Pacifica's existing
read-only constructor and BULK's new testnet/Unknown constructor cannot sign or
post normalized financial fills. Configured custody identifiers are inactive
registry inventory, not qualified funds controllers or discoverable deposits.

Use an explicit private managed inventory (at most 16 owners) to initialize
empty accounts via the existing trusted Enroll transition. Every ordinary API
command still authenticates its actual owner/agent signature. Public custody
onboarding remains unavailable: do not bypass open_with_custody or supply a
fixture controller to advertise a BULK deposit route. A restart verifies the
same owners/recipients, resumes incomplete enrollment and preserves epochs,
grants, operations and balances. It never reseeds collateral or risk readiness.

Trusted preparation rejects duplicate wallets and duplicate token recipients
within either unit before journal I/O; cross-unit address reuse is not a
same-unit collision. The finite boot clock/stop guard starts before journal
create/replay and is checked after freshness reads and immediately before each
enrollment commit. Expiry/stop leaves only the already accepted prefix; a later
authorized boot validates/resumes that prefix without reset or renewed grants.

This bounded assembly lets encrypted service and key/storage qualification
proceed without pretending an unsupported funding rail exists. P39 must add
explicit measured native-worker/paper policy before activation; P40 must qualify
that changed image. Offline constructors and old image receipts do not qualify
the new deployment, native financial round trips or customer-funds safety.
