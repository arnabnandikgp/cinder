# Pacifica qualification map

Reviewed 2026-09-30. This tracked contract promotes sanitized research conclusions;
ignored local captures are not runtime dependencies. First venue, not a universal
adapter. `Profile` must bind environment, pool, evidence and precision explicitly.

| Topic | Evidence and implementation | Still disabled / separate gate |
| --- | --- | --- |
| VM-01 units/basis | Decimal strings, exact grids and dimensional conversion checks; native rounded entry retained only as a view | Engine precision/mainnet equivalence is not established by display decimals |
| VM-02 execution identity/PnL | History ID dedups REST/WS; distinct reversal legs; durable client-ID attribution; selected historical testnet fills used execution `price/p` and net-of-fee `pnl/n` | Partial/maker/unequal-entry parity not universally qualified; profile requires explicit evidence |
| VM-03 costs | Fee recorded once; signed rebates are not promised savings | Historical flat-account difference **−0.000002 test USDP** remains unexplained, not a customer write-off |
| VM-04 funding | Lossless native amount/rate/payout retained | Logging timestamp does not establish funding cut, gross-private rate, signed rounding or explicit net-flat zero |
| VM-05 risk | Native equity, spendable margin and withdrawal availability are distinct views | Private IM/MM and pooled capital checks stay in P09; no inferred margin isolation or fee aggregation across accounts |
| VM-06 forced events | Unknown liquidation/settlement causes retained with containment | `settlement` covers ADL **or other settlement**; cannot automatically declare P12 ADL |
| VM-07 custody | Historical testnet agent permitted owner-directed withdrawal and parent/child movement | Not natively trading-only; Solana vault/allowlisted funding/recovery in P15–P17/P21, no PDA HTTP-signing inference |
| VM-08 finality | Whole-position-set snapshots, scoped LI, cursor/error provenance; no historical event erased on overlap | No complete causal fill certificate from largest LI/end of pagination; no automatic hold release |
| VM-09 compatibility | Narrow exact method/field mapping, native IDs and explicit unsupported cases | No literal URL-only compatibility promise or imported BULK semantics |

## Primary sources and historical observation

[Trade history](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-trade-history)
and [account trades](https://docs.pacifica.fi/api-documentation/api/websocket/subscriptions/account-trades)
provide the two transport schemas. Current REST prose about `price` and
`entry_price` remains ambiguous; the historical bounded testnet run compared both
feeds and observed execution at `price/p`, with entry basis at `entry_price/o`.
Observed opening PnL already included the displayed fee. This is evidence for
explicitly scoped qualification, not permission to apply that convention everywhere.

[Last ID](https://docs.pacifica.fi/api-documentation/api/last-id) and
[position snapshots](https://docs.pacifica.fi/api-documentation/api/websocket/subscriptions/account-positions)
describe scoped progression and complete snapshot replacement. They do not specify
a global complete economic log. [Funding history](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-funding-history)
does not supply all inputs needed for private allocation.

[Account information](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-info)
distinguishes reported balance, marked equity, margin availability, pending items
and withdrawal availability. Auto-lending, spot collateral and pending interest
cannot be silently included as immediately accessible fund capital.
[Order history](https://docs.pacifica.fi/api-documentation/api/rest-api/orders/get-order-history)
provides lifecycle views, not proof that every fill has been ingested.

Historical P3 research: nine orders/eight executions; seven paired REST/WS fills,
cleanup visible in REST. Fee sum 0.037621, gross PnL −0.030240, expected closing
balance 49.932139 versus observed 49.932137. Hidden fee precision is a hypothesis,
not a resolution. No live test was rerun in P13. Synthetic repository tests exercise
large IDs, exact native shapes, separate reversals, source gaps and a named signed
cash discrepancy. They are not new recordings of live fills.

## Conservative operational treatment

Unknown evidence restricts dependent activity without creating missing cash,
fees or funding. Existing genuine fills remain ingestible despite an admission
failure. New source qualifications or correction rules must be explicit; changing
a profile against old archived history currently rejects and needs a reviewed
migration. No native endpoint silently writes customer equity or clears suspense.
