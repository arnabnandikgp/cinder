# ADR 0031 — Journal-owned backend onboarding

Date: 2026-10-15. Status: approved backend-only scope; implementation in P25.
No frontend, deployment, native-credit investigation or live-run authority.

## Contract

The existing attested confidential channel carries a separate bounded,
wallet-signed onboarding envelope. Before an account exists, authentication binds
network/deployment, economic policy, wallet, fresh channel transcript and short
deadline. The wallet signs for itself; a Cinder agent cannot register its owner's
account. Account identity is SHA-256 of `CINDER-ACCOUNT-1\0`, network, deployment
and wallet. There is one account per wallet/deployment in this pool composition.
This is an identity derivation, not a privacy claim about public deposits.

The feature is explicitly opt-in through the measured configuration. Its immutable
policy binds bounded account capacity and the single configured classic-SPL
custody route. The server derives the wallet's canonical mint ATA as its recipient;
requests cannot change owners, payout destinations, routes or economic policy.
This is a new-history, zero-configured-customer composition. Existing
configured-account mode remains available and never silently enables onboarding;
it is not an automatic migration of existing wallets, liabilities or paid basis.
No production admission, coverage or fee policy is selected here.

Registration atomically appends an immutable owner/recipient binding, an empty
kernel book and initial authority epoch to the same accepted journal. It creates
no assets, cash, margin, positions or trading permission. Genesis configuration
and release commitments remain immutable; replay reconstructs additional
participants. All-user risk, projections, funds and recovery use this combined
inventory. Frozen/recovery mode refuses new registrations. Existing owners can
reconnect and rediscover their current epoch without resetting grants or counters.
Their authenticated original-deposit hints remain recordable during stale or
faulted risk admission, freeze and recovery (including dispatch seal). Hints are
observation metadata, not risk admission or recovery-payment authority. They
advance the accepted head, invalidating an older prepared cut without changing
financial books; source qualification and all signing/payment gates still apply.
Duplicate and concurrent requests resolve against accepted state, not a signup
database. Unknown/stale writes remain unavailable until reconciled. Engine
revision changes require a new history or separately designed migration.
The journal head advances for registration, but the financial ledger version
does not: adding an empty book changes no equity, exposure or existing financial
cut. Risk includes the new zero book; its first real financial event uses the
ordinary versioned accounting transitions. Numerical limits remain bounded
by the kernel's market/customer cell budget as well as enrollment capacity.

V1's complete immutable recovery-key registry seals enrollment. New
registrations are refused after that registry is committed, rather than creating
a liability missing from recovery delivery. Existing owners can still rediscover
their accounts. Incremental key enrollment/rotation is a separate design, not
silently inferred from wallet authentication. Recovery preparation must cover all
accepted participants and ordinary/recovery paid counters still cannot reset.

## Custody preparation and funding

Discovery returns the private account/current epoch and configured custody
domain/pool/program/config/venue/route/mint/precision/epoch, exact maximum deposit
batch and registered recipient. SDK helpers create unsigned, owner-paid registration/open/deposit/refund
instructions using the generated Anchor ABI. The caller supplies an explicit
provider, source token account, exact amount, operation and slot deadline.
Nothing signs, sends, pays rent or selects an RPC automatically. Chain state must
be checked before including initialization instructions; a retry never recreates
an existing counter or batch. Preparation is not confirmation.

An owner may report an original deposit operation/signature through the private
channel. This is a bounded, durable discovery hint, never a financial receipt.
The existing finalized-original transaction/code/route/account qualifiers decide
whether pending inventory exists. Sweep and qualified venue-credit rules remain
ADR 0030. The owner-facing ingress read retains pending/prepared/exposed/failed/
refunded/funding/credited/returned; neither registration nor a reported signature
creates available collateral. Unswept refund stays owner-signed on chain.
Discovery lists this owner's unqualified reported operations separately from
qualified ingress rows. A report is not a promise that a transaction executed or
will become credit. The retained report bound is 128 per history; terminally
ineligible original transactions are not repeatedly fetched in the current boot.
Hints and negative-cache entries are owner/operation-scoped: a public signature
reported for the wrong owner cannot reserve or suppress its rightful deposit.
The mint recipient ATA must exist before a refund/payout; preparing an escrow does
not secretly create/fund another wallet account or pay its rent.

## Qualification

Fresh-wallet SDK/API tests cover exact retries, restart, concurrent/stale writes,
wrong domain/policy/channel/signer, weak keys, expiry, capacity, owner isolation,
agent non-escalation and immutable recipients. Compiled local SBF exercises
customer registration, escrow opening, exact deposit and refund with disposable
local accounts only. Existing ordinary/recovery counters and old-route tests
remain required. Browser/HTTP/WebSocket are backend test carriers, not frontend
integration. Changed-image AWS/native financial tests and public availability
remain separate gates.

References: [custody contract](0030-venue-directed-escrow.md),
[implementation scope](../implementation/PLAN.md#p25),
[private API](0018-private-api.md).

Primary addressing references checked 2026-10-15:
[Solana PDA derivation](https://solana.com/docs/core/pda) and
[canonical associated token account seeds](https://www.solana-program.com/docs/associated-token-account).
Runtime derivation is crosschecked against the pinned Solana client in compiled
local SBF acceptance; these references are not financial or live-venue evidence.
