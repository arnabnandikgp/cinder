# BULK native qualification, not activation

First stage: a fresh **two-request unsigned metadata preflight**, with no wallet,
signature, faucet, order, transfer, Solana RPC or AWS access. This is inside the
larger proposed 12-public-read allowance, not a reason to spend that allowance.
No dependency installation or historical account reuse is needed.

Use pinned Node 24.21.0. `read.mjs --prepare /absolute/new/private-directory`
creates a source-bound manifest without network I/O. Current explicit approval
is recorded separately in private `approval.json` (0600): manifest commitment,
`approved: true`, exact user-reference and `not_after` milliseconds no more than
two minutes ahead. Preparation is not approval. `--run-approved` checks source,
scope and private permissions, retains each request durably before GET, and
archives bounded original response bytes losslessly before interpretation.

Only official paper-testnet `/exchangeInfo` and `/feeState` are allowed. No
redirect, retry, alternate host, cookies or credentials. Failure stops the run;
status 2 means blocked/non-success, not a qualified financial capability.
Never rerun a started directory or copy its approval to a new run.

CI runs `scripts/bulk-qualification.test.mjs` entirely with injected responses;
it does not call a live endpoint. Exact raw metadata is evidence for reviewing
current market rules, not an account balance, precision qualification or risk
readiness. No observed public metadata activates the production Gateway.

The signed paper lifecycle remains a separate stage under
[the proposal](../../docs/implementation/UNIT-QUALIFICATION-PROPOSAL.md). Reserve
an explicit close/cleanup attempt when planning an opening trade: two attempts
cannot promise a resting-order probe **and** an opening IOC **and** a closing IOC.
Fresh signing identities, original native messages/nonces and known cleanup
must be retained before any exposure. Old scratch SDK scripts and faucet funds
do not authorize a live run or qualify Solana funding/returns.
