# Two-unit deployment assembly

Scope: P38's opt-in manifest version 7. This is the real enclave entrypoint and
protected unit journal, not P36's private-pipe fixture. It is **not yet a native
trading/funding deployment or a changed-image Nitro qualification receipt**.
P39 joins workers/SDK acceptance; P40 qualifies the changed image and supported
paper lifecycle. No frontend integration is included.

## Public image contract

Use the existing default-feature `cinder-enclave`, optionally `--web` for the
attested Noise carrier. It reads only `/etc/cinder/manifest.cbor`. Version 7
selects the unit private schema before any listener opens. Legacy versions 1–6
select only their existing schema. Bad configuration fails; it never retries a
different schema or rewrites an old journal as unit history.

The public `units` policy requires distinct Pacifica/BULK unit IDs, canonical
registry digest, separate BULK port and DER root/hash. Pacifica uses the existing
venue port/root. Native origins are fixed testnet origins in the loaded egress
implementations; the parent cannot supply an alternate URL. Ports cannot alias
ingress, bootstrap, replica, witness or KMS routes. No chain, capture or demo
deposit policy is accepted. All three financial/native-read gates must be false.
Use bounded retained packs and a finite boot lease, at most one hour.

## Private trusted preparation

`cinder-prepare-release /absolute/new/private-directory < private-input.cbor`
performs no AWS, Solana or venue calls. The CBOR input uses these fields:

| Field | Actual content |
| --- | --- |
| `manifest` | Public version-7 endpoints/slots/history/units policy. Nonzero registry/application placeholders are replaced by derived values. |
| `configuration` | Global secret random 32-byte `privacy_nonce`, `owners`, `auth_ms`, `grant_ms`; the closed unit schema, not the legacy Pacifica configuration. |
| `pacifica_configuration` | `units::deployment::Pacifica`: canonical empty ledger bytes, inactive custody descriptor, exact source/grids/profile, testnet/Unknown execution policy and agent epoch. |
| `bulk_configuration` | `units::deployment::Bulk`: canonical empty ledger bytes, inactive custody descriptor, exact native account/source/grids/market indices, evidence/credits and agent epoch. |
| `storage` | Fresh 32-byte storage key. |
| `trading` | Separate 32-byte Pacifica agent seed. |
| `bulk_trading` | Separate 32-byte BULK agent seed. |
| `witness` | Parent-unknown scoped temporary witness credential. |

Omit `broker` and `funds`: version 7 has no money-moving key. Provide exactly
seven unique KMS slots: `Configuration`, `PacificaConfiguration`,
`BulkConfiguration`, `Storage`, `Trading`, `BulkTrading`, `Witness`. Labels in
encryption contexts/output filenames come from `Role::name()`. Trading keys
cannot alias storage, each other, either native owner/funding wallet or a managed
customer wallet. Existing legacy role labels/codes/CBOR encoding remain stable.

Preparation derives registry/application and slot body hashes from the actual
private objects. It validates **every** wrapped plaintext fits 4096 bytes before
creating output. Oversized profiles/inventory must be redesigned explicitly;
never truncate, put plaintext in the image, or bypass capsule validation.
Output remains 0700 directory/0600 files. Only the public finalized manifest goes
in the image; `.plain` files remain on the trusted operator machine. KMS Encrypt
and policy/PCR provisioning are separate controlled steps; send only resulting
ciphertext plus parent-scoped temporary credentials through bootstrap.

Generate `privacy_nonce` freshly with trusted cryptographic randomness; do not use
a published/test constant or disclose it. It blinds the owner configuration's
public body hash against guessed wallet/recipient inventories. It grants no
signing or money authority and is retained only inside its encrypted capsule.

## Managed accounts and history

This assembly supports an explicit private inventory of 1–16 managed owner
wallets, with a token recipient for each unit. Accounts derive from the existing
deployment/wallet rule. Existing trusted Enroll creates **empty** books in both
units; normal API commands still require real owner/agent signatures. No native
balance, credit, risk mark or capital is invented. No public onboarding/deposit
discovery is exposed because the BULK funds controller remains unqualified.

One new empty unit history uses the existing RecordCipher, both S3 replicas and
independent DynamoDB fresh-head witness. Existing unit revision-6 histories can
reopen only under identical registry and valid owner/recipient inventory; legacy
histories cannot be imported. Restart resumes incomplete enrollment without
resetting grants, shared revoke-all epochs, operations or financial state.
Wrong keys/generation, future journal time, recipient substitutions and extra
unmanaged owners are refused before publication.

The runtime retains actual read-only native controllers and fixed TLS egress
contracts, with no callable financial worker. BULK Unknown observations may be
archived raw but cannot post normalized fills; neither read-only gateway can
sign. The inactive supervisor performs only boot lease/fence checks, not remote
polls. External stop, clock expiry/failure and failed fresh publication checks
fence the boot; a later healthy response cannot revive it.

## Qualification boundary

### Version 9: quarantined paper experiment

Only a separately approved fresh paper run uses version 9 and private
`configuration.paper`. See [ADR 0044](../architecture/0044-paper-bootstrap.md)
and the [run proposal](../archive/qualification/P40-PAPER-RUN-PROPOSAL.md) for exact
fields, test assumptions, evidence, capital and cleanup bounds. Use the same
typed `cinder-prepare-release` validator; the read-only `qualification-units`
converter deliberately continues to produce version 8 with `paper` omitted.
No fixture input or existing account is a valid substitute. Protected paper
history cannot later be opened by an ordinary release, even with a changed
measured manifest. Fixed paper expiry caps the actual boot/native/read clock.

### Version 8: explicit bounded workers

Version 8 selects the same private schema with mandatory `units.worker`
(`poll_ms`, `pacifica_reads`, `bulk_reads`, `pacifica_dispatches`,
`bulk_dispatches`). Read limits are per boot (0–32); dispatch limits are
history-wide (0–16, including cancels); interval is 1,000–60,000 ms. Set public
native-read/trading gates to exactly match nonzero allocations. Funding remains
false. For a read-only release both dispatch limits are zero.

Supply explicit Pacifica `fills` and BULK `qualification` in their private
capsules. Unknown read-only interpretation is not signing authority. Any nonzero
dispatch allocation requires the actual corresponding Qualified controller.
Declare all three distinct Vault/Broker/Venue sources, selecting Venue for the
native source. Empty managed accounts start not-ready with no collateral/risk
facts; this constructor cannot import fixture money or grant native credit.

The same seven keys remain separate; no broker/funds key is loaded. The actual
leased clock gates native TLS, including stop and boot expiry. One bounded
supervisor releases the writer before network I/O and rejoins original-unit
completions. Shared history preserves exposure/nonce/credit and global customer
epochs. See [ADR 0043](../architecture/0043-unit-workers.md) for polling limits,
uncertainty, no-resend semantics and offline acceptance.

The existing package CI compiles the default-feature entrypoint, checks its
ordinary-host refusal and reproducible Linux ELF. Host-side Rust tests cover
actual loaded objects, signed unit requests, restart, preparation and fences
with explicit synthetic storage/time/witness ports. Carrier construction tests
check the same actual Handler is accepted by both transports; complete SDK/
carrier/native composition is P39, not claimed from constructor acceptance.
None is a new AWS measurement or live trade/funding receipt.

Use the existing [Nitro runbook](nitro-qualification.md),
[native proposal](../archive/qualification/UNIT-QUALIFICATION-PROPOSAL.md) and
[ADR 0042](../architecture/0042-unit-deployment.md) when preparing a separately
approved changed-image run. Unsupported BULK funding/return and Pacifica's
deferred missing-credit investigation remain named independent gaps.
