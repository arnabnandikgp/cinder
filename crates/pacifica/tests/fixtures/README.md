# Sanitized numeric regression

`observed-trades-sanitized.json` transcribes six rows from the historical P3 run-02
trade-history capture (0256). Prices, quantities, entry values, fees, PnL and
separate reversal legs are preserved. History/order IDs, client identifiers and
timestamps are replaced; no wallet, headers, keys or bearer credentials are copied.
This is **not** a new live recording or the entire eight-fill P3 run. Its use with
a synthetic initial balance does not reproduce that run's final account balance.
Provenance: `work/venues/pacifica/experiments/p3-execution/run-02/evidence/0256-get-_trades_history-response.json`.
The test does not need the ignored original file. Primary schemas and unresolved
precision evidence are documented in `docs/venues/pacifica.md`.
