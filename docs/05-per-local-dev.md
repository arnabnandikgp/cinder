# PER auth and local cluster

## Auth (two layers)

1. **EphemeralPermission** on the ER (program `ACLseoPoyC3cBqoUtkbjZ4aDrkurZW86v19pXz2XQnp1`).
   Members + flags on each private PDA. Source of truth.
2. **TEE/QFS token** from `getAuthToken(url, pubkey, signChallenge)`.
   Proves the connection owns that pubkey. Endpoint: `url?token=...`.
   Default local expiry: `--token-expiry-days 180`.

Do **not** keep a per-user token table on Cinder servers.
Persist: user pubkey, ledger PDA, flags. Adapter holds **one** operator token.

Flags: `AUTHORITY`, `TX_LOGS`, `TX_BALANCES`, `TX_MESSAGE`, `ACCOUNT_SIGNATURES`.
User: view flags only. Adapter: AUTHORITY + view. Book: adapter only.

Delegate the data PDA on L1 first (`DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh`).
Permission Create/Update/Close run on the ER, PDA-signed, PDA pays rent — pre-fund at init.

`verifyTeeRpcIntegrity` (Phala PCCS) on real TEE URLs only. Skip on local QFS.

## Endpoints

| Env | Base | ER | Privacy ingress |
|---|---|---|---|
| Local mb-stack / Surfpool | :8899 / :8900 | :7799 / :7800 | QFS :6699 / :6700 |
| Devnet TEE | Solana devnet | — | `https://devnet-tee.magicblock.app` |
| Mainnet TEE | Solana mainnet | — | `https://mainnet-tee.magicblock.app` |

Local ER identity: `mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev`.
Never delegate local accounts to `MTEW…` / regional public ids.

Clients testing privacy must use **QFS 6699**, not ER 7799.

## Stage C — `mb-stack`

```bash
npm i -g @magicblock-labs/ephemeral-validator@latest
mb-stack --reset
# 8899 base, 7799 ER, 6699 QFS
anchor build && anchor deploy --provider.cluster localnet
```

Manual equivalent: `mb-test-validator`, `ephemeral-validator --lifecycle ephemeral --remotes http://127.0.0.1:8899 ...`, `query-filtering-service --listen-addr 127.0.0.1:6699 --ephemeral-url http://127.0.0.1:7799 --token-expiry-days 180 --add-cors-headers`.

Prove here: two users, permissions, token filter, place/ack without Phoenix.

Reference: `magicblock-engine-examples/private-counter`.

## Stage A — Surfpool fork + ER + QFS

```bash
curl -sL https://run.surfpool.run/ | bash
surfpool start --rpc-url https://api.mainnet-beta.solana.com
# still localhost:8899 / 8900
ephemeral-validator --lifecycle ephemeral \
  --remotes http://127.0.0.1:8899 --remotes ws://127.0.0.1:8900 \
  --listen 127.0.0.1:7799 --reset
query-filtering-service ... # same as above
```

Clone/preload USDC, Ember, Phoenix programs + exchange accounts, Cinder program.
Adapter: Rise against :8899. ER ixs against :6699?token=operator.

Forked oracles/marks/withdraw queue may be stale. Good enough for a local win.

## Magic Actions (later)

Use for `write_reserve_root` and `settle_user_withdraw` after vault PDA exists.
Bind `escrow_auth` to vault PDA; require injected `escrow` signer.
Do not attach `PlaceMarketOrder`. Action failure reverts the commit.

Need delegated fee payer + `magic_fee_vault` before 10 commits / for live commit fees after 25.

## Session keys

Out of first win. They are not a replacement for `getAuthToken`.
