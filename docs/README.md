# Cinder documentation

Cinder is an experimental private perp prime broker for Phoenix on Solana. It
separates private user accounting from public venue execution: user-level state
lives in MagicBlock private ephemeral rollups, while Phoenix receives only
Cinder's aggregate position.

- [Architecture](architecture.md) explains the components, accounts, and order
  lifecycle.
- [Security model](security-model.md) records the privacy and reconciliation
  boundaries.
- [Development](development.md) covers the supported local workflow.
- [Operator recovery](operator.md) explains durable order tracking, restart
  safety, and the current runtime boundary.

The repository also contains detailed tests that serve as executable examples
of the ledger, privacy, and netting behavior.
