# BULK native qualification: current scope and funding questions

Reviewed 2026-10-20. Target: Pacifica's present **bounded experimental integration
level**, not production completeness. P33–P36 implementation is merged; current
native trading and supported funds rails are separate evidence targets. Existing
financial policy, exact accounting, recovery and confidentiality rules stand.

## Current primary-source refresh

- Official client HEAD: `5012611bd0fd36e12523cd15bac763f3a600b1e9`, committed
  2026-10-18. [Comparison with Cinder's pin](https://github.com/Bulk-trade/bulk-client/compare/56f6676611a0c05dbd187d406dced26168516070...5012611bd0fd36e12523cd15bac763f3a600b1e9)
  contains one commit: administrative pricing/provider-fee actions, risk config
  DTOs and related CLI changes. The inspected raw-signing diff appends admin
  cases, not changes to the existing limit/cancel branches. No account-history
  or Solana deposit/withdrawal builder/document file appears in the change list.
  This is a source comparison, not a native execution result or automatic pin bump.
- Official keychain HEAD remains
  `ec7212cf1c1269b4dce4dce23c69a310a73b9766` (2026-10-04).
  No package/native binary was installed or executed from upstream.
- [API introduction](https://docs.bulk.trade/api-reference/introduction) still
  distinguishes paper HTTP `https://exchange-api.bulk.trade/api/v1` from mainnet
  and requires the matching signing domain. The
  [faucet](https://docs.bulk.trade/api-reference/requestFaucet) supplies paper
  account funds; its example amount is not a confirmed allocation or an SPL mint.
- [Exchange info](https://docs.bulk.trade/api-reference/getExchangeInfo) describes
  size increment, decimal width, resting tick and minimum notional separately.
  Its illustrative minimum is not a current testnet observation. Preserve raw
  numbers and qualify actual market values before choosing a trade size.
- [Agent documentation](https://docs.bulk.trade/api-reference/manageAgentWallet)
  describes master agents covering children and independently scoped subaccount
  agents. Actual powers/revocation must be tested; app permissions do not narrow
  native key powers. Historical September probes do not qualify this release.

## First stage: unsigned metadata only

[The preflight](../../tools/bulk-qualification/README.md) is source-bound, requires
current exact approval and makes at most two fixed-origin GETs. It retains request
identity before I/O, original response bytes before interpretation and a named
non-success result for origin/rate-limit/transport/schema problems. No retry,
fallback host, credential, key, faucet or signed action exists in this tool.
CI uses injected responses, never network. Successful metadata means only that
those public shapes were observed, not that a trading policy can be activated.

The paper lifecycle follows only after usable current metadata and an approved
scenario with reserved cleanup. The original proposal's two order attempts cannot
cover a resting probe plus an IOC open plus an IOC close. Do not silently add a
third order, raise the notional budget or leave an opening trade without a close
allowance. Finalize the counted opening/cleanup actions against actual minima.

<a id="observed-metadata-receipt--2026-10-20"></a>
### Observed metadata receipt — 2026-10-20

The user separately approved **two unsigned GETs only**. Preflight source
`b8cac32` ran once from 12:26:51.781 to 12:26:52.477 UTC; both returned HTTP 200.
Private original artifacts are retained in
`work/venues/bulk/run-20261020.TG1ntr/preflight`. They are optional ignored run
evidence, not a checkout/CI dependency. Manifest commitment:
`ca4d5e7cfcc33b0253e92c4e5579065257b8fce6dfb9d8da3c3a7c400434d0f3`.
Source commitment:
`1bca26fa43df1b2eeec83294498817c2ab3a3407f196444d8f6c7ac739a5b0b1`.
The run terminated `metadata-observed` with two requests; its expired approval
must not be reused or treated as permission for a signed paper run.

| Fixed paper-origin path | Status | Original bytes | SHA-256 |
| --- | --- | --- | --- |
| `/api/v1/exchangeInfo` | 200 | 5,809 | `822eaf7f215d93d84d812abd3cd451ca04239814e0bac160d0de3c7870c97748` |
| `/api/v1/feeState` | 200 | 1,416 | `d535319c5abe9b33cd0b0005eb9a861e03efa9f50d40ff5793f55e22d5bf80d2` |

Original response hashes/lengths were independently rechecked. Exchange info
lists 17 markets, all `TRADING`. BTC-USD reports `minNotional: 1.0`,
`tickSize: 0.01`, `sizeDecimals: 8`, and `sizeIncrement: 1e-8`
(exact decimal `0.00000001`). Its advertised native maximum leverage is 40;
that does not select Cinder's leverage policy. Native TIFs include GTC, IOC, ALO,
ALO_SLIDE and ALO_JOIN; this observation does not expand Cinder's supported TIFs.
Revalidate actual prices/grids before a separately approved signed run; these
values are a dated observation, not permanent configuration or execution proof.

Fee state retains exact wide `stamp: 1792499212510152669`, slot `10475828`,
`globalPolicyActive: false`, zero active instrument overrides, no scheduled
policies and zero reported settled fills. This public response does **not**
establish an account's execution fees, a financial completeness frontier,
zero-fee trading, fee-tier savings or mainnet policy. Original numeric bytes
remain authoritative; no floating-point financial values were posted.

No wallets, signatures, account setup, faucet, orders, transfers, Solana RPC or
AWS were used. Earlier 502 responses remain historical; this success establishes
only these two observations. Next: approve the corrected paper-account lifecycle
and its counted cleanup allowance in [proposal A](../implementation/UNIT-QUALIFICATION-PROPOSAL.md#a-native-paper-account-capability-before-hardware).

## Funding/return investigation: facts versus remaining questions

The [official retained builder contract](https://github.com/Bulk-trade/bulk-client/blob/5012611bd0fd36e12523cd15bac763f3a600b1e9/docs/solana-deposit-withdraw-intent.md)
describes owner-signed classic-SPL deposit opcode 2 and withdrawal-intent opcode 4.
Deposit transfers tokens to the configured mint vault; venue credit follows bridge
processing. The withdrawal intent transfers no tokens: margin locking and actual
settlement happen later. CLI defaults point to mainnet and are not testnet pins.
Neither a dry run nor a finalized intent establishes a credited/paid operation.

The public client refresh has not established a usable **test** bridge for us.
No Solana RPC, wallet, deposit, withdrawal or financial venue request was made in
this source investigation. The precise questions to resolve are:

1. Which Solana cluster, program, classic-SPL mint and initialized vault feed the
   paper environment, if any? Is paper faucet collateral independent of bridge
   settlement? Do not infer a devnet route from an HTTP testnet URL.
2. Which native evidence links an original finalized deposit signature to its
   exact credited account/asset/amount/fee and duplicate handling?
3. For the original withdrawal intent, which query/event identifies pending,
   failed or paid status and the actual beneficiary token transfer? How is that
   original recovered after a lost HTTP/event response without issuing a second?

These are targeted integration questions, not prerequisites for unrelated paper
order tests. Do not build a bridge controller against guessed settings. If no
supported rail/evidence is available, retain a paper-only trading boundary and
keep Cinder's BULK funding/returns disabled. Pacifica's missing-credit work stays
deferred; no weaker funding or recovery contract is approved by this document.
