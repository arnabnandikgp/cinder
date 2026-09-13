# Architecture

## Components

Cinder has three execution domains:

| Domain | Responsibility |
| --- | --- |
| Solana L1 | Cinder configuration, collateral vault, reserve metadata, and Phoenix venue execution |
| MagicBlock ER | Private `UserLedger` accounts, the aggregate `Book`, and permission data |
| Operator adapter | Reconciles the aggregate residual and interacts with Phoenix |

The vault program owns protocol configuration and collateral state. The ledger
program owns private user state and the aggregate book. Both use program-derived
addresses for their long-lived accounts.

## Privacy boundary

Users access the ER through the Query Filtering Service (QFS), authenticated by
short-lived tokens. A user's token may read that user's ledger but not another
user's ledger. The adapter has an operator credential to access the accounts it
must reconcile. Raw ER access is intentionally not treated as a privacy
endpoint; QFS is the enforced access-control boundary.

## Execution lifecycle

1. A user submits an order to the private ledger. The ledger records a
   tentative position and reserves collateral.
2. The adapter aggregates the resulting residual across users and submits only
   that residual to Phoenix through Cinder's single cross trader.
3. When Phoenix confirms a fill, the adapter calls `ack_phoenix_fill`. This
   finalizes the user position and updates the aggregate `Book`.
4. When Phoenix rejects or does not fill an order, the adapter calls
   `ack_phoenix_fail`, releasing the tentative state.

This distinction prevents the aggregate book from claiming venue exposure that
has not actually filled.

## Netting

For each supported market, the `Book` represents the sum of filled user
positions. Phoenix should observe the same net position. Offsetting user
positions can therefore reduce or eliminate venue exposure while remaining
private to Cinder.
