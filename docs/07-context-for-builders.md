# Context the freeze does not repeat

Use this when a change looks tempting. It is not a second spec.

## What Cinder is copying from HyperLink

HyperLink is a prime broker, not a venue. Users deposit to HyperLink contracts. An AWS Nitro enclave keeps the private book. All venue flow goes through one pooled Hyperliquid account. Users sign Hyperliquid-shaped messages; the enclave attributes fills internally. Chain sees deposits, withdrawals, and the pool’s inventory.

Cinder’s equivalents:

| HyperLink | Cinder |
|---|---|
| Nitro enclave | PER + QFS/TEE |
| Pooled HL account | One Phoenix cross PDA |
| Agent key | `position_authority` |
| Master / withdrawal key | Vault authority (PDA later, keypair now) |
| Signed private reads | `getAuthToken` + EphemeralPermission |
| Always-exitable on-chain | `ReserveRoot` now; escape ix later |

We are not copying HyperLink’s invite graph, staking-for-fee-tier, or TWAP UI.

## Cross vs isolated — how we chose

Phoenix offers two account types under one authority:

| | Cross PDA `(0,0)` | Isolated child |
|---|---|---|
| Positions | up to 128 | exactly 1 |
| Resting orders | 64 bid + 64 ask per market | same cap, one market |
| Margin | shared across markets on that PDA | that position only |
| On-chain visibility | one public trader | **another** public trader |
| Extra portfolios | `pda_index` > 0 is gated; treat as unavailable | each child is its own public PDA |

The tempting idea was “isolated children = private subaccounts.” That is false on Phoenix. Isolated accounts are first-class on-chain traders. Using one child per user would publish every Cinder customer as a separate Phoenix book. That destroys Q1-A (attribution privacy) for no venue benefit.

A second tempting idea was several cross PDAs (multi-portfolio) to cap blast radius. Phoenix does not currently activate extra `pda_index` values for a builder the way a CEX opens subaccounts. Even if it did, each extra PDA is another public residual.

So the locked shape is **one cross PDA**. Phoenix already nets on that account: Alice +1 SOL and Bob −1 SOL look like flat to the world. That *is* the privacy. Isolated legs would prevent that netting and leak identity.

`isolatedOnly` markets cannot live on the cross PDA. Skip them. Do not open an isolated child “just for that market.”

## Privacy vs internal risk (the trade we accepted)

Pooling everyone on one cross account is how we hide who is behind a fill. It is also how one user’s loser can threaten the pool.

Phoenix IM/MM and liquidations run on the **residual**, not on the sum of absolute user sizes. If Alice is +10 and Bob is −9, Phoenix sees +1 and charges IM on +1. That is good for capital and for privacy. If Alice is +10 and Bob is 0, Phoenix sees +10. If Alice blows through Cinder’s books and we are slow, Phoenix can liquidate **Cinder**, which socializes Alice onto every other user.

We do not “solve” that by splitting isolated children. We solve it with a **stricter internal overlay**, same idea as HyperLink sitting on Hyperliquid:

1. **User IM/MM = 1.25× Phoenix** on that user’s own size (`USER_IM_MULT_BPS` / `USER_MM_MULT_BPS` = 12500). Alice is margined as if she were a standalone Phoenix account, haircut 25%. The pool’s venue IM can still be smaller because of netting.
2. **Hard user leverage cap** below the venue: `MAX_USER_LEVERAGE = 10` when Phoenix’s first size tier is ~15x. Effective cap is `min(10, phoenix_first_tier_max_leverage)`. Never allow a user more leverage than Phoenix would give that size, and usually less. This is the HyperLink “buffer under the venue max” move.
3. **Liquidate users at Cinder MM**, before Phoenix reaches `Cancellable` / `Liquidatable` on the pool.
4. **Phoenix buffer**: posted collateral ≥ Phoenix IM(residual) × 1.20, floor 50 USDC. Absorbs mark noise so a tick does not immediately threaten the shared account.
5. **Reject** a new residual that would leave Phoenix health worse than `Safe`, even if the user passed Cinder IM.

What we are *not* doing: taking venue leverage 1:1 and hoping netting saves us. Netting is a privacy and capital side-effect, not a risk model.

Leverage-tier warning: Phoenix tiers are **size-based on the pooled residual**. A large residual can knock the whole PDA into a worse tier. Cinder must treat intended residual size (Book + pending), not only the user’s ticket, when checking whether the hedge is allowed.

## Operational numbers (do not freelance)

| Knob | Value | Role |
|---|---|---|
| User IM | 1.25 × Phoenix IM(user) | Internal haircut |
| User MM | 1.25 × Phoenix MM(user) | Internal liq line |
| User max leverage | 10x, and never above Phoenix first-tier max | HyperLink-style cap |
| Pool buffer | 20% of Phoenix IM(residual), floor 50 USDC | Shared-account slack |
| First win orders | market / IOC only | No 64-slot book usage |
| First win markets | 1 allowlisted asset | Stay inside 128-position cap trivially |

## Why Book moves on ack, not on place

If `Book.residual` jumped at `place_order` and Phoenix rejected, I1 (`Book == Phoenix`) would be false until a compensating ix. Window=0 therefore:

1. User ledger tentatively applies the delta and parks a pending oid.
2. Adapter computes `intended = Book + pending` and decides whether Phoenix can take it.
3. Phoenix market/IOC.
4. `ack_phoenix_fill` commits Book + fees; `ack_phoenix_fail` reverts the user tentative.

Window>0 later only changes when step 3 fires (batch leftover). Do not change account layout for that.

## Why Magic Actions must not send Phoenix orders

A Magic Action runs on L1 after an ER commit. If the action fails, **the commit reverts**. Phoenix can reject for margin, queue, or slippage. Gluing user fill and venue hedge into one commit would roll back a good private fill because the venue blinked.

Hedge is an adapter L1 tx. ER and Phoenix are correlated, not atomic.

Magic Actions *are* for: writing `ReserveRoot`, later paying withdraws from the vault PDA via the injected escrow signer.

## Why QFS tokens are not a Cinder user table

`getAuthToken` proves a wallet to the TEE/QFS. QFS then filters accounts using **on-chain members**. The ACL is `EphemeralPermission`. Persisting user tokens on the adapter is persisting a read credential for their private PDA.

Adapter is a member on every user ledger so it can liquidate without the user’s client. That is honest-operator local-win trust, same class as HyperLink’s enclave seeing all books.

## What is public no matter what we do

- Vault USDC ATA balances
- Phoenix trader state for Cinder’s authority (`GET /v1/trader/state/{authority}` is unauthenticated)
- Residual size, leverage tier, liquidations of the pool
- Deposit and withdraw txs

Privacy is **attribution**: observers should not map a Phoenix fill to Alice vs Bob.

## Name collision

`cinder.trading` and `github.com/cosmic-markets/cinder` are an open-source Phoenix TUI. Unrelated. Do not import that crate as this product.

## First win vs later

Local win = Surfpool mainnet fork + local ER + QFS, two users, net-vs-gross demo.

Later: vault PDA as Phoenix authority, Magic Action withdraw, attested TEE URL, escape-from-root, window>0, session keys.

Offline exit is a **named gap** until escape exists. Do not advertise CEX-style “always withdraw on-chain” before that ix works.
