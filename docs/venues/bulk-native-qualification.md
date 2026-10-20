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
