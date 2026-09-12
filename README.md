# Cinder

Private perp prime broker for Phoenix on Solana. User books on MagicBlock PER; Phoenix sees Cinder net only.

## Demo (record cluster)

```bash
./scripts/stack-c.sh          # other terminal — base :8899, ER :7799, QFS :6699
./scripts/cinder-demo.sh      # isolation → offsetting fills → Alice close with PnL
```

Requires Anchor 1.0.2, Solana 3.1.x, Node 24. Spec: `docs/`. Named gaps: `docs/10-open-gaps.md`.
