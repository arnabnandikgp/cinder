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
