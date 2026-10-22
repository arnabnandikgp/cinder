# BULK candidate qualification map

Assessed 2026-10-15. **Candidate for second-venue qualification, not an enabled
adapter.** No BULK API, RPC, wallet, faucet or cloud invocation occurred in this
assessment. Pacifica's missing-credit work remains deferred. The current custody
model is [venue-directed escrow](../architecture/0030-venue-directed-escrow.md);
the proposed implementation sequence is [SECOND-VENUE](../archive/qualification/SECOND-VENUE.md).

Subsequent P27 [codec foundation](../architecture/0032-bulk-codec-foundation.md)
adds offline integer/wire/fill tests, not activation. The separately approved
October 15 public-testnet probe made three unsigned metadata GETs; all returned
HTTP 502 Cloudflare origin errors. No usable current schema or financial evidence
was obtained. This limited observation is not proof of a global venue outage.

P28 adds [financial observations](../architecture/0033-bulk-financial-observations.md)
for account/positions, actual funding payments, fee-tier quotes and risk history.
Exact configured units, missing data, unsupported rows and native side/reason
information remain explicit; none is a ledger posting, credit certificate or
active financial provider. Its [two-unit contract](../architecture/0034-execution-unit-boundary.md)
is proposed for review before Runtime/journal/API changes. The same source pins
apply; no new native, RPC, wallet, faucet or cloud run occurred in this slice.

## Evidence and source pins

October 17 implementation update: P33–P36 add the opt-in shared-unit service,
agent-only signing/read budgets, exact ordinary fill/fee postings and existing
unit custody/recovery composition. [Joined acceptance](../implementation/UNIT-ACCEPTANCE.md)
uses explicit synthetic native/collateral ports, not live qualification. BULK
funding/returns and unsupported causal coverage remain disabled/unresolved.
October 16's separately approved two unsigned public metadata GETs also returned
origin 502; no private financial/RPC/AWS activity or new capability was established.
The earlier assessment paragraphs retain their dated scope, not current code status.

- Official client: [`56f6676611a0c05dbd187d406dced26168516070`](https://github.com/Bulk-trade/bulk-client/tree/56f6676611a0c05dbd187d406dced26168516070),
  committed 2026-10-14. Compared with historical research pin
  `2df898d7af1cf0625649896293a3137dc1d6de43`.
- Official signing library: [`ec7212cf1c1269b4dce4dce23c69a310a73b9766`](https://github.com/Bulk-trade/bulk-keychain/tree/ec7212cf1c1269b4dce4dce23c69a310a73b9766),
  committed 2026-10-04. Public source was inspected, not installed/executed.
- Official documentation was captured on 2026-10-15. The
  [changelog](https://docs.bulk.trade/api-reference/changelog) describes a scheduled
  October 4 upgrade; elapsed time and published schemas do not establish deployed
  behavior. Current market grids must not inherit September captures.

Full captures, selected SHA-256s, old/new comparisons and detailed findings remain
ignored under `work/venues/bulk/reassessment-2026-10-15/`; checkout and CI do not
depend on them. This file promotes conclusions and public references, not keys,
old signed requests or live qualification. Client source is Apache-2.0 and keychain
MIT per pinned manifests/licenses; future dependency adoption needs explicit review.

## Fit against the present product

The former PDA-to-native-HTTP requirement is no longer the integration bar.
Cinder already uses a venue-recognized key in its funding route. That makes BULK
a plausible candidate without waiting for Solana program composability, **not**
a proof that its bridge, key controls or recovery satisfy our contract.

| ID | Current primary-source evidence | Cinder treatment / remaining qualification |
| --- | --- | --- |
| BV-01 authentication | [API overview](https://docs.bulk.trade/api-reference/introduction): account queries unsigned; state changes Ed25519-signed | Bind environment/account/key/operation to accepted history; no bearer-token assumption or customer access to pooled authority |
| BV-02 signature and identities | [Signing](https://docs.bulk.trade/api-reference/signing) binds actions, u64 nonce, account and network domain; [order IDs](https://docs.bulk.trade/api-reference/orderIds) are precomputable | Persist exact bytes/nonce/derived ID before exposure; timeout/absent lookup is not rejection or resend authority |
| BV-03 delegation | [Agents](https://docs.bulk.trade/api-reference/manageAgentWallet) can be scoped to master or subaccount; master agents cover children | Do not call a native agent trading-only. [Transfer architecture](https://docs.bulk.trade/architecture/transfers) permits master agents for internal transfers; [API prose](https://docs.bulk.trade/api-reference/transfer) is narrower. Test actual powers, revocation and isolation |
| BV-04 custody ingress | Pinned [Solana builder contract](https://github.com/Bulk-trade/bulk-client/blob/56f6676611a0c05dbd187d406dced26168516070/docs/solana-deposit-withdraw-intent.md) describes key-owned token deposit into a mint vault | Bind cluster/program/mint/source/amount/original signature. No supported devnet bridge or native credit completion has been established here |
| BV-05 return | Same SDK describes a withdrawal **intent**, followed by separate native/chain processing | Finalized intent is not finalized payment. Need beneficiary, amount, once-only behavior and recovery from a lost response; no shipping completion from ACK alone |
| BV-06 accounting | [Account queries](https://docs.bulk.trade/api-reference/getAccount) distinguish local margin from optional parent-plus-isolated `agg*` totals | Never add local + aggregate, include sibling subaccounts implicitly, or equate available margin with withdrawable funds. Reconcile physical cash/claims independently |
| BV-07 history | Same account schema has bounded pages, coverage/retention metadata and activity rows | Coverage was already in historical research. It is not a global finality certificate. Activity rows do not document an original Solana signature/action hash; amount/balance matching alone cannot discharge B05/B07 |
| BV-08 fees/cohorts | [Fees](https://docs.bulk.trade/bulk-exchange/fees) aggregate completed-day volume across a main/subaccount tree; [subaccounts](https://docs.bulk.trade/bulk-exchange/sub-accounts) document independent native risk | Attractive for pooled fee economics with future risk cohorts; not one account per user. Qualify isolation/current fee policy and omnibus permission before enabling. Do not promise a specific rate or rebate |
| BV-09 funding | [Funding](https://docs.bulk.trade/bulk-exchange/funding) is hourly, using positions at its boundary | Verify precise rates, boundary quantities, rounding and settlement, including gross private positions when native net is zero |
| BV-10 forced events | [Liquidation](https://docs.bulk.trade/bulk-exchange/Liquidations) and [ADL](https://docs.bulk.trade/bulk-exchange/ADL) distinguish selective reductions, collateral shortfall and execution underfill | Preserve real price/reason/quantity and all cash adjustments once; no automatic reuse of Pacifica settlement classification. RF1/RF2 remain Cinder policy, not a clone of BULK's ranking |

### Wire precision is not financial precision

The published market schema now uses `sizeDecimals` / `sizeIncrement`; contract
overview scaling and signing scaling are not interchangeable. The pinned
[client serializer](https://github.com/Bulk-trade/bulk-client/blob/56f6676611a0c05dbd187d406dced26168516070/crates/api-rust/src/msgs/mod.rs)
and [keychain encoder](https://github.com/Bulk-trade/bulk-keychain/blob/ec7212cf1c1269b4dce4dce23c69a310a73b9766/crates/bulk-keychain/src/sdk_compat.rs)
round scaled signing values at 1e8 and use floating convenience types. Cinder must
parse original decimal lexemes into checked integers, validate exact market grids
and test canonical encoding independently. No f64 enters the financial kernel.
Wide IDs/nonces/timestamps cannot pass through a JavaScript Number.

The new client fills decoder can synthesize `tradeId` from slot/sequence when
missing; that does not improve native evidence. Open-order responses now unwrap
`openOrder`. Both are relevant decoder details, not a reason to import a whole SDK.

### Bridge and environment limits

Official testnet HTTP is `https://exchange-api.bulk.trade/api/v1` and WebSocket
`wss://exchange-ws1.bulk.trade`. A BULK signature domain is not a Solana RPC
selection. The SDK CLI defaults to mainnet bridge settings; do not run it against
ambient configuration. Paper faucet funds prove neither a Solana token deposit
nor a usable return rail.

The Solana funding builder/docs did not change between the historical and current
client pins. Their renewed relevance comes from Cinder's changed custody model,
not a newly discovered supported bridge. Security prose about per-user custody
and SDK per-mint vault construction need program-level verification, not inference.

## Go / defer boundary

**Proceed:** source-pinned exact wire/observation qualification and a narrowly
designed two-venue ownership boundary. Existing core accounting, protection,
RF1/RF2 and operator-assisted recovery are retained.

**Require evidence before enabling:** exact native grid and fees, causal history,
actual signer powers, currently supported bridge cluster/program/mint, original
credit attribution, settled return and beneficiary payment. Unavailable evidence
disables the dependent capability, not unrelated local work.

**Defer:** automatic routing/rebalancing, cross-venue margin, spot, triggers/TP-SL,
per-user native accounts, production/native isolation claims, changed-image Nitro
and public/frontend integration. No promise that BULK solves Pacifica's credit gap.

The first useful acceptance target is a manual-venue SDK path with isolated unit
accounting, not a universal adapter framework. A real second-venue financial path
eventually needs original deposit → credit → order/fill/funding → close → settled
return → protected wallet payout. A paper-only pass must stay labeled paper-only.
