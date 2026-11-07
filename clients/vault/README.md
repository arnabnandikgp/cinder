# Unsigned vault client

`src/index.ts` wraps exact u64/identity encoding, canonical PDA derivation and the
typed Anchor **1.2.0** program interface. Supply an explicit provider and deployment
IDL. It does not choose a wallet/RPC, load keys, sign or send transactions.
Use `.methods.<instruction>(...).accountsStrict(...).instruction()` to construct
an instruction; the custody controller owns durable intent and actual signing.

The tracked IDL/type are generated from the Rust source and checked by
`node scripts/check-vault.mjs`. Never edit them by hand. Use `bigint` amounts,
counters and slots; display numbers are not valid SDK monetary inputs. Customer
deposit receipts have their own owner-bound namespace. Funds/return/payout receipts
share a permanent operator namespace across authority rotation.

The tests deliberately sign/submit only to the check-owned offline localhost
Surfpool, using fresh in-memory identities. That fixture is not a production
wallet implementation. [Vault contract](../../docs/architecture/0015-solana-vault.md).

`src/escrow.ts` builds immutable venue-route registration, owner escrow opening,
exact batch deposit, full sweep and owner refund instructions. An unswept batch
is refundable to its owner; it is not trading collateral. Only a qualified sweep
handoff is recognized in the journal, and native credit remains a separate gate.
`src/funding.ts` accepts explicit v2 sweep contracts with canonical original batch
metadata; legacy rails retain v1 and reject sweep metadata. Later ordinary and
recovery payouts use the PDA payout vault and user-wallet recipient, never the
auto-swept ingress escrow. See [ADR 0030](../../docs/architecture/0030-venue-directed-escrow.md).

`src/recovery.ts` packages already-final positive claims, hashes the domain-bound
statement, constructs ordered Merkle paths and encodes Anchor recovery arguments.
It is not an entitlement calculator, full-solvency verifier or automatic activation
service. Authenticate the actual active statement/configuration and paid-counter
basis before using inclusion results. Use the generated `stageRecovery`,
`activateRecovery` and `claimRecovery` methods with explicit strict accounts;
normal and recovery payments share lifetime counters.

`src/claims.ts` adds independent P21 verification: owner-authorized dedicated
encryption keys, governed manifest signatures, GET-only ciphertext failover,
recipient-local OAEP/GCM decryption and strict live custody/counter/root checks.
Use `verifyManifest` → `retrieveKit` → `decryptKit` → `independentClaim`; supply
authenticated chain observations and independently governed readers explicitly.
`independentClaim` only builds an unsigned instruction; the owner wallet signs it.
No Cinder API, native venue credential or ambient RPC/wallet is selected. Back up
the dedicated decryption key and private opaque locator before depending on recovery;
this slice implements neither key rotation nor lost-key reissue. Package hosts and
live source qualification remain release gates, not defaults hidden in the SDK.
See [recovery contract](../../docs/architecture/0017-recovery-claims.md).
