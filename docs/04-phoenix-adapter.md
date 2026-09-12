# Phoenix adapter

Rise SDK: `@ellipsis-labs/rise` / `phoenix-rise`.
HTTP: `https://perp-api.phoenix.trade` on mainnet; on Surfpool use the fork RPC and still the public API only if it matches cloned state — prefer on-chain + WS against the fork.

## Boot (once)

1. Create stand-in authority keypair (later vault PDA).
2. `POST /v1/exchange/build-register-ixs` with `traderAuthority`, `txFeePayer`, `maxPositions: 128`.
3. Sign with fee payer only. `send-register-ixs` (`pda_index=0`, `subaccount_index=0`).
4. `DelegateTrader` → `position_authority` = adapter hot key.
5. Subscribe Rise trader-state for that authority + market stats + `createMarginCalculator`.

No-referral path allows off-curve authorities. Fee payer ≠ Phoenix onboarder.

## Collateral

Deposit helper (Rise TS): create Phoenix ATA, Ember wrap, `DepositFunds`.
Withdraw helper: ATA, approve Ember, USDC ATA, `WithdrawFunds`, Ember unwrap.

Amounts: TS native `1 USDC = 1_000_000n`. Authority must sign deposits/withdraws (stand-in key or vault CPI).

Buffer: posted ≥ Phoenix IM of residual × (1 + buffer_min_bps) and ≥ floor.
Top up before a hedge that would breach. Pull on withdraw if vault ATA is short.
Phoenix global withdraw queue can delay or drop pulls — do not debit the user until USDC is in the vault.

Mints (mainnet):

- Wallet USDC `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`
- Phoenix USDC `PhUsd11YkbjSaWjFncfAAmatntsjx3MgDR9B6g1ks3A`
- Ember `EMBERpYNE6ehWmXymZZS2skiFmCa9V5dp14e1iduM5qy`

## Hedge (window = 0)

First local win: **market / IOC only**. No resting limits.

1. User ER `place_order` succeeds (pending oid).
2. `intended = Book + pending`.
3. Reject if Rise pool post-trade leaves `Safe`, or user failed Cinder IM already on ER.
4. `post_collateral` if needed.
5. `buildMarketOrderPacket` + `placeMarketOrder` / `MarketOrderTicket` with `authority = position_authority`, `trader_account = trader PDA`.
6. Confirm. `ack_phoenix_fill` or `ack_phoenix_fail`.

Phoenix taker fee 3.5 bp assigned to that user on ack. Maker 0.5 bp unused until limits exist.

## Reads

- WS trader-state on vault authority (public pool view — expected).
- Local margin: `marginInputs()` + `createMarginCalculator` or Rust `TraderPortfolio::compute_margin`.
- Hawkeye is on-chain truth; local calc is the pre-trade gate.

## Failures

| Event | Action |
|---|---|
| Phoenix 0 fill / reject | `ack_phoenix_fail` |
| Partial IOC | ack filled size only |
| Dropped tx | retry once if no fill; else fail-ack |
| Queue on pull | user withdraw stays pending |
| Pool Cancellable+ | UNSAFE_POOL; MM first, then IM only until Safe |
| Venue liq/ADL of Cinder | freeze and flatten internally |

## Not in adapter

User TEE tokens. Session keys. Isolated subaccounts. Builder fees. Referral activation (use no-referral register).
