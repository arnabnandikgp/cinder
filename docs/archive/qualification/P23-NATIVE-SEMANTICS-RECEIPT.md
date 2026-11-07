# Native-semantics invocation receipt

Recorded 2026-10-08. Includes an earlier actual bounded native deposit/payment
round trip and the later persistent investigation below. Neither establishes
shipping financial completeness or a Nitro/P23 pass. Earlier stopped invocations
remain retained failures, not relabeled passes.

<a id="persistent-credit-investigation"></a>
## Persistent multi-deposit investigation — recent credit still absent

The latest user explicitly authorized continued independent devnet/testnet
transfers and checks in one run until actual credit behavior or another cause was
established. This supersedes the earlier one-additional-attempt stopping rule;
it does not change customer funding, mainnet, AWS or shipping financial gates.
The diagnostic now uses a separately sealed `cinder-native-staged-delivery-v1`:
`maxRetries:5` permits bounded RPC forwarding of the **same original wire**,
sixty status polls and twelve account/loan pairs within ten minutes. There is
one `sendTransaction` call per original, no re-signing or replacement operation.
The 20-minute/200-HTTP/400-RPC/eight-WSS invocation caps and 12-second native
request cadence remain. Historical manifests retain their earlier exact limits.
The focused offline suite passes **36/36** before execution and again afterward.

Three new fresh-identity cases ran sequentially. A/C use owner faucet → broker;
B mints directly to the broker, matching the official funding helper. B creates
an empty owner return ATA, not an invented owner allocation. Each case uses only
20 faucet USDP. All five chain actions simulated and finalized successfully;
each native original has an independently verified **20,000,000-atom broker
debit, equal native-vault credit and broker-bound `DepositEvent`**.

| Case | Native finalized block time (UTC) | Slot / event nonce | Account initialization | Later credit comparison |
| --- | --- | --- | --- | --- |
| A, owner → broker | 14:57:08 | 508854102 / 3888 | All twelve pairs absent | Account/loan 404; histories empty after 25 min 11 s |
| B, direct broker faucet | 15:03:42 | 508855757 / 3889 | All twelve pairs absent | Account/loan 404; histories empty after 19 min 25 s |
| C, owner → broker | 15:10:39 | 508857501 / 3890 | All twelve pairs absent | Account/loan 404; histories empty after 13 min 15 s |
| Earlier original | 13:15:42 | 508828590 / 3886 | Historical bounded checks absent | Still account/loan 404; histories empty after 2 h 5 min 48 s |

Each new main case consumed **76 RPC / 27 HTTP / two finite WSS**, with five
simulations and five original exposures, no native POST, settings mutation or
withdrawal exposure. The original setup wait is approximately five minutes;
ten minutes is the sealed maximum, not a claimed ten-minute observation. Later
reads are separate, read-only scopes that do not reopen a stopped invocation.
No 429 was bypassed, missing account was treated as zero, or chain debit promoted
to native credit. The unchanged native executable hash is
`4806b3b684357596851a3436874ff8fb28fbea0ea0ddf5dae6c725fc6a78e901`;
this is byte identity, not verified source equivalence.

### Independent controls

A fixed public-program comparison decoded recent finalized deposit events using
22 read-only RPC calls. A subsequent six-GET comparison selected three distinct
public depositor accounts; no third-party balances or wallets are published here.

| Native deposit control | On-chain time (UTC) | Native API observation |
| --- | --- | --- |
| Nonce 3883, 10,000 USDP | 10:52:51 | Account 200; one deposit-history record matching the original signature |
| Nonce 3885, 10,000 USDP | 13:09:54 | Account 404; successful empty deposit history after over two hours |
| Nonce 3887, 500 USDP | 13:16:46 | Account 200, but successful empty deposit history; no original match after over two hours |

An existing account's HTTP 200 alone is not proof the new 500-USDP deposit was
credited. These observations show that missing recent credits are not confined
to our 20-USDP originals or owner-to-broker route, and that an older original's
history can still be retrieved. They do **not** establish an exact outage start,
a global API failure, a proven backend cause or that future credit cannot arrive.
Raw observations and public identifiers remain owner-only in ignored research.

The final independent 20-GET comparison ran **15:20:53.677–15:24:42.628 UTC**;
the older established control returned account/loan 200, zero balance and its
one deposit/two balance-history rows. It is a public observation, not permission
to reuse its keys, balance or setup. The current original/A/B/C results are in
the table above. The public-event scope SHA256 is
`686cff720ad8c21ea5e316d5ad0c5852a1d126b4cdaa1aa6f31c809e6601192e`,
result `7d1f6fb95769b00de6f8e6651bd05cf97a4df126ec3cd146124464c1aaf1d7f5`.
The six-GET control scope SHA256 is
`172b26004c3f1a4ac0d1d02a319cc8718052ad9fdec29b1642631ec115ec2bd2`,
result `944bffd717201729ed12be499bc48b47df3c3d05bdb2687bb973a62a46c60a63`.
The final original/control comparison scope SHA256 is
`7625953bf9b2248138471f61981613f758ffbeb74ca71fb24dfcfb1ca6e16448`,
result `a2aea93a16a35545569e1b969a6300a948c5627fcbdf1a61314eeefb16b552b9`.
These bind observations, not native completeness certificates.

### Immutable receipts and residuals

| Case | Stopped journal / receipt SHA256 |
| --- | --- |
| A | 248 rows; head `5c8728c1d9843c744625eb2de94d8e7772af7a6846931644e280faf61b4dfe1a`; receipt `dcb8ac5db1b8127138cd7bb5665dab79b1a39cae8a0feac02770180f2c8ba721` |
| B | 249 rows; head `fff961643903ea062f01d2ef266dfdb3d03e0f19ae69731682e495317899deb3`; receipt `fe3bb5db463be05648eeb78ae01671b67bb3dfea109bae25ac02ef8f274a455c` |
| C | 248 rows; head `9d4b1c6c5d7b3d04be6e15d7c31ef66256d8ad672635d703c82f503ff697e8b8`; receipt `c39e66cf04d1ef3b852b3036d961d560f687da6e96ddc856cbf62adef94b7366` |

The corresponding seals are retained in `run-credit-a-20261008`,
`run-credit-b-20261008` and `run-credit-c-20261008`; exact run times/public
transaction IDs are in the ignored support packet and reusable offline inventory.
New sponsor debit is **0.18003 devnet SOL** in total; unreclaimed gas and ATA rent
remain in the disposable accounts. The four finalized originals together leave
**80 USDP at the venue with credit/return unresolved**. The earlier unlanded
`run-fund2-init` is separate: its last verified **20 USDP remains in its broker**,
not another successful native-vault deposit. No asset is relabeled returned.

**Disposition:** repeated finalized deposits across both funding paths have not
produced native credit, including after later follow-up. A current venue
ingestion/credit delay or failure is the leading explanation, not a proven indexer
diagnosis. The former chain-delivery confound is eliminated for A/B/C. Further
identical deposits would duplicate that evidence; trace the retained originals
with the venue and/or observe their later original-linked credit. No customer
top-up, trading, new vault/program or AWS resource was started. Strong native
setup/credit readiness and P23 financial acceptance remain unqualified. All
scripts, approvals, original wires, receipts and raw responses are retained;
the support packet is prepared locally and **not sent**.
Final offline contracts run passes **79/79** across six discovered suites,
workspace boundaries and the 27-phase/54-document foundation/link checks.
Whitespace is clean. Original fund1/fund2 stopped heads are independently
verified unchanged; no generated secret or raw signed wire is git-tracked.

<a id="independent-second-initialization"></a>
## Independent second initialization diagnostic — stopped at chain delivery

The user explicitly authorized another independent 20-faucet-USDP round if the
original was still uncredited. A new six-GET scope,
`reconcile-20261008-1430`, checked the original from
**14:30:45.151–14:31:46.421 UTC**: account/loan 404, empty deposit/balance histories,
no original match, default lending null; earlier public control account 200.
Its scope SHA256 is
`e95db7da9f4b914f99f31706f66eb1ced0e9450ffb8221ff3deae7a9ca1899d4`;
result SHA256 is
`d70c9541aa1586fee498a8e36a1ce6dcb976ff3ddab2315de577d0748884dff7`.
No signer, RPC, mutation or original-run restart was involved in those GETs.

The separately sealed `run-fund2-init` began **14:34:18.619 UTC**, seal
`589f927374bee1cb2e0dc00512dca28281e2d529f401e4d7dab89577c1f59df9`.
It uses fresh disposable owner/broker identities and one additional 20-faucet-USDP
staged deposit; no original operation is resent or replaced. Existing 0.10-SOL,
20-minute/200-HTTP/400-RPC/eight-WSS limits and the four-pair/five-minute staging
bound remain. One retained-ACK withdrawal and confirmed-net return are the only
planned financial containment, subject to actual prerequisites. No AWS, customer
funding, trading or shipping gate is enabled. Fresh focused regressions passed
35/35 before execution; those tests do not establish live deposit delivery.

The original 20 USDP remains a separate unresolved residual. Success here would
not establish why it was missing, a native completeness certificate or a Nitro
funding pass. Preserve both invocation histories and do not silently retry either.

### Actual outcome and read-only containment

The main invocation stopped **14:36:26.413 UTC**. Its five actions all simulated
successfully; sponsor-to-owner/broker, faucet mint and owner-to-broker allocation
finalized. The native deposit was signed/exposed once and accepted by the RPC,
but all **30 original-signature status polls were null**. No finalized native
deposit receipt, initialization, settings mutation or withdrawal was reached.
The independently decoded signed instruction still matches the exact official
deposit ABI, broker signer and 20-USDP amount. Signature verification passes;
the pre-exposure finalized height left **145 valid blocks**. The native executable
hash remains the same `4806b3b684357596851a3436874ff8fb28fbea0ea0ddf5dae6c725fc6a78e901`.
These observations do not establish why the transaction did not land.

A separate read-only child ran **14:39:20.318–14:40:48.637 UTC**, within the
parent's remaining deadline/request limits. It used five RPC reads and eight
native GETs, read no signer and performed no send or mutation. The original
status and finalized transaction remained absent; finalized block height
**496085059** exceeded last valid height **496084157**. Owner quote tokens were
zero; the broker retained the full **20,000,000 atoms**. Both original and new
native accounts remained 404 with no matching deposit/history rows. The first
original signature still had finalized status. No native credit was inferred.

| Evidence | Actual disposition |
| --- | --- |
| Main stopped journal | 236 rows; SHA256 head `686aad8e2aa0712529f9fcf5f5046cafd051b8f47e77bd5dc0a39ade5024fbff` |
| Read-only child scope | SHA256 `5c2138f4c17eb4ab5bd2b1d7ab199309a80377e949f9c7b370ee593fa2f3256f` |
| Read-only child result | SHA256 `0d1755faa2e50a53e9012fb756938d2ce8a31cfee0c748c4e9f1f920c692694c` |
| Retained helper | SHA256 `69f7662a479d73782ab4659c5a0d332351c59a60531e33c85e17fbabdf3c91cf`; fixed original observation only |
| Main plus child requests | 108 RPC, ten native HTTP, one finite WSS |
| New sponsor debit | 0.06001 devnet SOL; unused gas/token-account rent retained |
| First residual | 20 USDP at the native vault, original credit/return unresolved |
| Second residual | 20 USDP still in its disposable broker token account, not returned |
| Withdrawals / settings writes / economic retries | 0 / 0 / 0 |
| New vault/program / customer top-up / AWS / trading | None |

The first parent's 208-row stopped head remains exactly unchanged. The second
is **not a second successful venue deposit**, nor proof of a second ingestion
failure. Observed expiry/absence is not promoted into the financial controller's
strong no-later-effect contract or permission for an automatic rescue/re-sign.
The public [Solana sendTransaction contract](https://solana.com/docs/rpc/http/sendtransaction)
explicitly distinguishes RPC acceptance from cluster processing/confirmation.
The current driver uses `maxRetries:0`; investigate bounded original-wire
forwarding in a separately reviewed future scope, not by changing either closed
invocation. All original request/wire/receipt files and raw replies remain ignored.
The helper is saved under `work/experiments/p23-live/funding-tools/observe-fund2.mjs`.
No stronger setup, customer/Nitro funding pass or clean-return claim follows.

<a id="original-credit-follow-up"></a>
## Earlier GET-only reconciliation still finds no original credit

After the stopped initialization below, the user authorized investigation
("I see, go for it"). `reconcile-20261008-1340` is a distinct read-only scope,
not a restart or extension of the financial invocation. It ran from
**13:39:26.467–13:40:27.289 UTC**, within its own three-minute/six-request limit,
12-second cadence and 8-KiB body bound. All six authenticated-source GET replies
were retained before interpretation; no signer or RPC configuration was read.

| GET | Result |
| --- | --- |
| Original account | 404, unsuccessful envelope with null data; no balance inferred |
| Original deposit history | 200, successful empty array; no original-signature match |
| Original balance history, with trades included | 200, successful empty array |
| Original loan | 404, null data; no zero debt inferred |
| Original settings | 200, default lending null; no disabled-lending readiness |
| Earlier established public control account | 200; no prior authority adopted |

Server `Date` headers agree with observation times. No cache-age/cache-status
header was present; this does not prove absence of backend caching. The original
20 initialization USDP remain deposited with credit/return unresolved. No extra
RPC, WSS, withdrawal, settings write, economic retry, funding, deployment or AWS
action was performed. The funding parent's verified head remains unchanged.

Fresh scope SHA256:
`21b5a663bd9508f3f04a272d8794d37f7db77dcf51301e9b410a4b5f0bb72059`.
Result SHA256:
`1320727f485446258f7b76ae8e7a2bebe2e61089b48f5f29708f0f1969dc9a6c`.
Read-only helper SHA256:
`da49fd146de58021fb789fab6f0e7a713432276543f10c25ae1ac39d35c297c3`.
These bind observations, not a financial certificate or renewed live authority.

### Checked explanations and remaining uncertainty

Independent offline decoding of both the earlier successful staged original and
this unresolved original confirms the same discriminator/account ABI, exact
20-USDP allocation from a distinct faucet owner to the broker, and broker-bound
`DepositEvent`. Event nonces are respectively **3880** and **3886**. The earlier
successful flow also did not mint directly to the broker. This comparison does
not require another deposit or import old keys/readiness into the new account.

Two live public source reads confirm the current official MCP head is still
`4748feca93efe2b2a5a0c95993e40f68d0ca4338`; its IDL SHA256 remains
`a31bb37868338e189fead472c1481954ed876844e83529e2e68b5ef0dbf427a1`.
The program, ten account definitions, discriminator and `u64` amount agree with
the original. The [official funding helper](https://github.com/pacifica-fi/pacifica-mcp/blob/4748feca93efe2b2a5a0c95993e40f68d0ca4338/scripts/fund-account.ts)
performs mint/deposit/balance polling; the checked official tools do not prescribe
another account-registration POST. [General deposit documentation](https://docs.pacifica.fi/trading-on-pacifica/deposits-and-withdrawals)
lists a $10 minimum; this 20-USDP original is not below it. That general statement
does not separately qualify a current testnet acceptance rule.

The [snapshot-delay FAQ](https://docs.pacifica.fi/api-documentation/api/api-faq/delayed-account_positions)
concerns order/position snapshot channels. It does not establish that a missing
deposit credit is harmless or authorize using an empty cache as ready collateral.

**Disposition:** no local deposit-route mismatch or documented extra
initialization step was found. Missing native ingestion/credit is the current
observation; its exact root cause remains unresolved and requires venue tracing
or later original-linked credit evidence. The updated local support note contains
only public testnet identities/signature and is **not sent**. No messaging channel
or recipient was selected, no economic resend is allowed, and no AWS/customer
funding was started. All private originals remain ignored and untouched.

<a id="fresh-funding-initialization"></a>
## Fresh funding initialization stopped before AWS

The user approved one fresh funding-only Nitro host ($5/four hours/eight boots),
one 20-USDP customer top-up and up to 20 separate faucet USDP for idle-account
initialization, including separately recorded test-only return after fencing.
The reply was "Yes, run within those bounds". This receipt covers **only the
initialization prerequisite**, not the customer vault or an AWS invocation.

The fresh, one-shot staged subrun `run-fund1-init` has seal
`c41cd2c92ced26415daf90a9c3dc230d7553d344c9b83609c9d206c01bf8e88b`.
It began at **13:14:49.226 UTC**, with a 20-minute original deadline and the
existing four-pair/five-minute initialization bound. The previous returned test
accounts/keys/approvals were not adopted. Each financial send was simulated,
persisted and exposed once; economic retries remain zero.

| Evidence | Actual result |
| --- | --- |
| Native deposit | Original successful finalized devnet slot **508828590**; exact **20,000,000** broker quote-atom debit and Pacifica vault credit |
| Original ABI | One deposit instruction, exact official discriminator/amount/accounts and original broker signer; independently decoded offline from the finalized receipt |
| Native event | One successful native CPI `DepositEvent`: original broker, 20,000,000 atoms, matching block timestamp and deposit nonce **3886** |
| Native initialization | Four bounded account/loan GET pairs returned 404; no missing-cache zero or complete setup accepted |
| Follow-up histories | Separate 12-GET shape observer, three-GET residual reconciliation and three-GET credit follow-up; original deposit and balance histories empty |
| Last read | **13:29:48.704 UTC**: account 404, no deposit-history rows/original match, no balance-history rows |
| Comparison | One previously established test account returned 200; its authority was not imported |
| Retained request totals | **79 RPC / 29 native HTTP / two finite WSS**, within original caps |
| Native withdrawals / settings writes / resends | **0 / 0 / 0**; absent withdrawal prerequisites stopped the main runner before signing |
| Sponsor debit | **0.06001 devnet SOL** including sponsor transaction fees; unused test gas and token-account rent retained |
| Residual | **20 initialization USDP deposited; native credit and return unresolved**; owner and broker quote ATAs zero |
| Customer top-up / new vault / AWS / trading | None; customer top-up cap unused and no paid host launched |

The original executable hash remains
`4806b3b684357596851a3436874ff8fb28fbea0ea0ddf5dae6c725fc6a78e901`,
with pinned loader slot/authority unchanged. This is not source equivalence.
The finalized receipt SHA256 is
`dc1ac27d313cd045e58eab11911adf17fc2a143e0577be727b8ef9b55f9a75b2`;
the independently verified 208-row stopped journal head is
`fddc7a137a872ba521d9629f9d3fb62a6e639a78334073b7c74c1dbab2ab372b`.
The separate read-only children preserve this parent and cannot restart it.
The native event decode uses the pinned public IDL's exact discriminator and
72-byte CPI payload layout; it is evidence of the program event, not an API
credit or a stronger completeness certificate. All observations were within
the original window. That window has now expired; no subsequent live action
or query is authorized by reopening its locked invocation.
Private addresses, signature/wires, keys, raw replies and RPC configuration stay
in ignored owner-only research. No archive, asset or key was removed.

Current primary [official deposit tooling](https://github.com/pacifica-fi/pacifica-mcp/blob/main/src/tools/deposit.ts)
and [network route plumbing](https://github.com/pacifica-fi/pacifica-mcp/blob/main/src/tools/onchain.ts)
still describe this depositor/central-state-vault route. The actual instruction
matches the previously pinned official ABI. These observations support a
missing native-credit prerequisite, **not a proven indexer root cause** or a
guarantee that credit will never arrive. A successful chain debit alone is not
an eligible demo credit; unknown funds cannot be returned speculatively.

**Next:** investigate the original with bounded read-only evidence or venue
support. Do not resend the deposit or start AWS/customer funding while setup is
absent. Any later authorized return/settings continuation must retain originals,
once-only exposure and its own finite scope; the stopped runner and expired
deadline cannot be reopened. P23 remains in progress. The earlier actual round
trip below is historical and does not substitute for this fresh prerequisite.

## Approved original invocation

The user explicitly approved `run-final-01` at seal
`2dbcf59f6272128e08eec9f34c113af3967549025e162ed76362d2eeff736fcc`.
The one-shot runner started once and stopped while loading the configured sponsor.
Offline preparation had retained a trailing display space in Solana CLI's
`Keypair Path` value, producing a nonexistent filename. The correct file exists
and has the required owner-only permissions; neither the RPC nor Pacifica was
responsible for this failure.

Read-only inspection of the original durable journal establishes:

| Evidence | Actual result |
| --- | --- |
| Journal | One `start` record; no consumed request or exposed-operation records |
| RPC / native HTTP / WSS calls | 0 / 0 / 0 |
| SOL funding, faucet mint, native deposit and withdrawal | Not submitted |
| Funds moved or transaction fees incurred | 0 SOL; 0 USDP |
| Simulation, native setup, payment and completeness | Not reached; unqualified |
| AWS / program deployment / shipping gate changes | None |

The original lock, approval, identities and private artifacts remain untouched.
No restart, second submission, replacement UUID, rescue transfer or cleanup
transaction was performed. Secret files and request payloads are not published.

## Local correction and replacement preparation

Offline preparation now trims CLI display padding, requires one absolute locator
and validates file type, ownership, permissions and size **without reading the
sponsor secret** before creating fresh identities. Execution validates the exact
sealed locators before locking; it does not silently trim or amend them. A later
signer-decoding failure is durably classified before network I/O.

Verification on pinned Node 24.21.0 passes:

- 15 native tool regressions, including display whitespace, missing/relative/
  ambiguous paths, permissive files and symlink refusal.
- All 58 contract/inventory checks.
- All nine existing/new vault funding/ABI checks and vault SDK type checking.

`run-final-02` is prepared offline with corrected source/configuration and fresh
disposable identities at seal
`160e00d1a5a7286f3dbabeda1aecdf5ccd9123b600eeae319548bfeeb8d1e59f`.
Its scope is unchanged: 20 faucet USDP, 0.035/0.025 devnet SOL funding within the
0.10 SOL sponsor cap, one original deposit/withdrawal and no AWS. The 2-USDP native
fee ceiling remains receipt acceptance, not a signed maximum fee.
That replacement subsequently received explicit exact-record approval; its actual
invocation is recorded below. The preparation above was not itself a live pass.

Native setup, original deposit/withdrawal/payment evidence, lost-reply recovery
and causal completeness remain unqualified. No source cut, customer credit or
financial completion certificate is inferred from this stopped invocation.

## Second approved invocation: loader preflight refusal

The user explicitly approved `run-final-02` at seal
`160e00d1a5a7286f3dbabeda1aecdf5ccd9123b600eeae319548bfeeb8d1e59f`.
Source checkpoint `90b0e00a46fec8cd27da9fdbc997983de12d105d` started once and
performed three read-only devnet RPC requests: genesis, native Program account
and its 45-byte ProgramData header. All three returned HTTP 200. The verified
genesis, program-data pointer, deployment slot `376257391` and upgrade authority
match the pinned expectation. Funding/simulation/native requests were not reached.

The diagnostic constant for the loader was **invalid**: it omitted three trailing
`1` characters. The RPC owner matches the canonical
[`BPFLoaderUpgradeab1e11111111111111111111111`](https://solana.com/docs/core/programs/program-deployment).
This is our local constant/fixture defect, not observed native deployment drift.
The old offline fixture copied its owner from the same incorrect constant, so it
could not detect the typo. This evidence is retained rather than relabeled a pass.

| Evidence | Actual result |
| --- | --- |
| RPC / native HTTP / WSS calls | 3 / 0 / 0 |
| Simulations / financial submissions | 0 / 0 |
| Funds moved or transaction fees incurred | 0 SOL; 0 USDP |
| Native setup / credit / payment / complete cut | Not reached; unqualified |
| AWS, shipping financial changes or retries | None |

The stopped lock, hash-linked journal, original approval and raw private RPC
receipts are preserved. No second invocation, signature exposure or rescue took
place. Offline replay of those retained headers passes the corrected validator;
it is not a fresh read, executable/source-equivalence proof or full preflight pass.

## Broader local boundary correction before another invocation

Route constants now parse as canonical 32-byte public keys during module loading.
The independent ABI test pins the official loader instead of copying `ROUTE`,
derives the ProgramData PDA through that loader and rejects the old typo.

Review against current Pacifica primary documentation also corrected two local
probe mappings before any assets could be committed:

- [`account_transfers`](https://docs.pacifica.fi/api-documentation/api/websocket/subscriptions/account-transfers)
  carries a single event object. The tool retains it rather than silently ignoring
  it; malformed event envelopes refuse. Bounded array fixtures remain diagnostic
  compatibility inputs, not a documented replay/completeness guarantee.
- [`account/balance/history`](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-balance-history)
  is the documented perp balance-history route. Baseline still requires an
  explicit successful empty final page; missing/404 history is not empty. Planned
  subsequent observation windows use that route and account info, not assumed
  perp withdrawal-history/pending paths or unrelated spot endpoints. Closeout
  checks actual `pending_balance`; it explicitly leaves operation completeness
  false. No UUID correlation, source cut or no-later-effect certificate is invented.

These corrections affect only the local diagnostic tool/tests. Shipping Rust
providers, source-cut/credit/payment gates, resources and economic policies are
unchanged. Pinned checks pass 17 native-tool cases, all 60 contracts/inventory
cases, all nine vault codec cases and SDK type checking. No Rust/SBF/Nitro rerun
or hosted new-head CI success is claimed by those local checks.

The offline-only replacement is `run-final-03`, seal
`ade4499008899fc2fc8c8d43b41df8a3836b7bed2a990bc9670935a56597c604`.
Its `REVIEW.md` and `MAPPINGS.md` describe the exact fresh roles, source bindings
and corrected observation contract. Amounts, devnet fee payers, bootstrap bounds,
one original deposit/withdrawal, fee-acceptance limitation and no-AWS scope remain
unchanged. That preparation had no approval, journal, lock or live activity;
neither prior approval was reused. The user subsequently approved this exact
replacement, as recorded below.

## Third approved invocation: signer-buffer aliasing refusal

The user explicitly approved `run-final-03` at seal
`ade4499008899fc2fc8c8d43b41df8a3836b7bed2a990bc9670935a56597c604`.
Source checkpoint `19a6953181695371a97ce7880c710bf6151a9d01` started once.
The devnet deployment/executable, mint/vault and fresh-identity checks passed.
Pacifica returned an explicit absent account and a successful empty balance
history page. These observations are not a complete financial source cut.

The first lending-disable POST returned HTTP 400 with
`signature_verification_failed`, before funding. Offline verification against
the configured broker public key fails. The signing preimage matches the
[documented canonical format](https://docs.pacifica.fi/api-documentation/api/signing/implementation);
the local defect is buffer ownership: `Keypair.fromSecretKey` retains its input,
which the loader immediately cleared. The stored public key remained correct,
but the retained signing bytes were zeroed. Verifying against that same private
key concealed the wrong identity. This is not a native signing-policy failure.

| Evidence | Actual result |
| --- | --- |
| RPC / native HTTP / WSS | 19 / 3 / 0 |
| Native signed requests | One explicitly rejected lending-setting request |
| Simulations / chain submissions / deposits / withdrawals | 0 / 0 / 0 / 0 |
| Funds moved / transaction fees | 0 SOL; 0 USDP |
| Settings readiness, customer credit, payment and complete cut | Unqualified |
| AWS, program deployment or shipping gate changes | None |

The original private approval, wire, response, journal and lock are preserved.
No original economic operation was exposed and no used record was restarted.

## Corrected signer boundary and fourth invocation

The diagnostic now gives the SDK a separately owned copy before clearing its
parse buffer, and verifies native signatures against the expected public
identity. A deterministic unfunded regression reproduces the old aliasing fault,
requires its rejection and verifies the corrected native and chain signatures.
Wrong identities and malformed key arrays also refuse. JS/SDK objects still
offer no secure-erasure guarantee; this is not a Nitro or customer-key loader.
All 17 core/driver cases, 60 contract/inventory cases and ten vault funding/ABI/
signer cases pass; vault type checking passes. No Rust/SBF/Nitro rerun is claimed.

The user granted standing authority on 2026-10-08 for small test-only source/
configuration corrections and reseals within unchanged devnet/testnet limits.
It does not authorize economic retries, uncertain replacement operations,
increased exposure, AWS/mainnet/customer assets or custody/security-policy changes.
Future agents must record applicable authority on a corrected seal, without
repeating a permission question for a minor test-only correction.

Under that authority, `run-final-04` has fresh disposable roles and seal
`77421841139e81672e3ea6f4eeeaa8b2adf8be7e695b4c804148c16b70990b6a`.
Its reviewed amounts, fee-acceptance limitation, bootstrap bounds, request budgets
and no-AWS scope are unchanged. Its source-bundle SHA256 is
`be4fdf99ca24059d44fc96128becea2578c48fec941d2c1b7fb57b6339f1d5da`.
The one-shot runner completed in 253,308 ms with `closeout-unresolved`, because
lending disable was rejected after deposit as well as before it. The original
result is preserved; the later settings-only check does not rewrite it.

| Evidence | Actual result |
| --- | --- |
| Main invocation RPC / native HTTP / finite WSS | 96 / 16 / 4; within approved caps |
| Controlled chain sends | Six originals, each simulated and finalized; no resend |
| Faucet and native deposit | 20 USDP; exact finalized broker debit and venue-vault credit |
| Native deposit observation | Matches the original finalized signature; no customer credit inferred |
| Native withdrawal | One original UUID, successful audit ACK, batch-linked confirmed event and independently finalized broker receipt |
| Gross / native fee / net | 20 / 1 / 19 USDP |
| Net return | Exact finalized 19 USDP back to the fresh owner; broker token balance zero |
| Last venue observations | Balance/pending balance zero, no orders/positions, borrowed/pending interest zero |
| Sponsor debit including its transaction fees | 0.06001 devnet SOL, below 0.10 SOL cap; unused test gas and account rent retained |
| Main invocation lending/bootstrap | Both disable requests returned the exact HTTP-422 rejection; setup unresolved, final setting null |
| Lost-reply experiment | Response withheld before controller acceptance; private audit retained; not actual packet loss or a process crash |
| Complete native frontier / UUID recovery / shipping credit | Unqualified; `source_cut:null`, `financial_completion:false`, no activation |
| AWS, program deployment, trading/agents/loans/recovery | None |

The streamed public native executable SHA256 is
`4806b3b684357596851a3436874ff8fb28fbea0ea0ddf5dae6c725fc6a78e901`;
the loader slot/authority were unchanged. This is byte-identity observation,
not verified-source equivalence or an audited native-program claim.
The original hash-linked journal was independently replay-verified. Private
requests, replies, captures, simulations, original wires, signatures and keys
remain in the ignored private directory; none is published or deleted.

## Bounded settings-only follow-up on the empty original broker

The original scope permits bounded disable attempts after definite setting
rejections. With all net funds returned, a separately retained settings-only
helper used the same original disposable broker, original remaining deadline/
request budgets and standing test-only authority. Its code digest, parent journal
head and scope were durably recorded before I/O. Its own lock prevents a second
invocation. It does not restart the economic runner or alter its original result.

The check used four HTTP calls: explicit empty-account read, one new
`disabled:true` request, settings readback and loan readback. It passed
`observed-disabled-no-debt`: actual setting true, borrowed and pending interest
zero. No RPC/WSS, new funding, economic POST or chain transaction was performed.
Combined fourth-run counts are 96 RPC / 20 HTTP / four WSS and three setting
attempts, within the original ceilings; all earlier records remain intact.

This establishes that disable/readback works on the established empty account.
The earlier account absence followed by successful deposit indexing and this
late successful setting is **consistent with an initialization/readiness delay**;
it does not prove the precise native failure cause, atomic bootstrap or safe
customer funding. The original 120-second bootstrap gate remains unpassed.

## Read-only instruction check during the workstreams-first continuation

On 2026-10-08, a strict offline diagnostic independently decoded the retained
fourth-run finalized payment's public instruction ABI against the pinned official
IDL. The successful audit ACK's batch nonce equals the `batch_withdraw` batch
nonce, **not** its separate per-recipient `withdraw_id`. The one-recipient legacy
instruction contains the expected broker, 19,000,000 net quote atoms, program,
account order and signer/writable privileges. Its net amount agrees with the
original 20-USDP request and 1-USDP advertised fee.

This adds no live request, changes no original archive, and does not convert the
audit ACK into a controller-accepted reply. It supports the positively identified
ACK/batch/payment path, not recovery of a genuinely missing native ACK, native
debit completeness or a final financial certificate. The diagnostic refuses
versioned/multi-recipient/extra-instruction layouts rather than claiming support.
Independent synthetic ABI tests include malformed routes, bytes, permissions,
amounts and nonce identity; all five native ABI/signer tests pass. The large
per-recipient `withdraw_id` fixture is retained as an integer string, not rounded
through JavaScript `Number`.

The same continuation fixes thrown native transport/body/rate-limit/audit failures
to retain original exposure and unknown reply before stopping. The no-resend
regressions pass without re-executing any closed invocation. Shipping diagnostics
also stop polling currently unqualified perp withdrawal-history/pending paths
before credit reservation; legacy archive tags remain readable. The subsequent
[source investigation](P23-NATIVE-EVIDENCE.md#completion-contract-audit-2026-10-08)
corrects the earlier undocumented label: official MCP tools/client list them,
and retained historical responses worked. These are local
corrections, not fresh native or hardware qualification.

<a id="separate-current-testnet-schema-observation"></a>

## Separate current testnet schema observation

On 2026-10-08, under the user's approved schema/readiness continuation, the new
GET-only `tools/native-qualification/observe.mjs` completed a separate observation
at `work/experiments/p23-native-schemas/observe-20261008-01`. It did **not** reopen
the fourth financial invocation. The established account is that invocation's
already-returned disposable broker; the absence probe is a random public-only
32-byte address, with no private key, ownership assertion or funds.

The retained observation ran from `05:16:43.993Z` to `05:18:56.822Z` (132,829 ms):
12 testnet GETs, 12-second cadence, 8-KiB response bound and fixed Node 24.21.0
TLS roots. There were no authorization headers, settings writes, RPC/WSS requests,
wallet/secret reads, chain sends, financial activity or AWS resources. Three
earlier exploratory GETs used the same public broker metadata without artifact
writes; they returned consistent deposit/balance/settings shapes. Total current
live HTTP observations are **15**, not 12. Neither observation sequence changes
the old economic invocation's budgets/results.

| Current observed endpoint / account | Result and permissible conclusion |
| --- | --- |
| Established deposit history | HTTP 200; one `amount`, `transaction_id`, `created_at` row; exact original retained signature and 20-USDP gross match. Final page omits `next_cursor`, with `has_more:false`. This is a current native history read of a **historical** deposit, not new chain finality or a new credit. |
| Established balance history, `include_trades=true` | HTTP 200; latest `withdraw` has amount -20, balance/pending zero; older `deposit` has amount/balance 20, pending zero. An old matching deposit is not currently available collateral. |
| Established settings / loan | HTTP 200; explicit lending-disabled true, empty margin/spot overrides; zero borrowed/pending interest and empty spot balances. Additional settings fields (`can_rfq`, `can_create_covered_exit`, `privacy_mode`) exist. These are component observations, not atomic `Setup.complete`. |
| Established account / positions / orders | HTTP 200; cash, pending, interest, margin and position/order counts zero; separate orders/positions arrays empty. The account is established and currently idle; no fresh-account bootstrap was tested. |
| Absence-probe histories | HTTP 200; empty final pages. Successful empty history does not establish account readiness. |
| Absence-probe settings | HTTP 200; lending null and empty overrides. Success/default cross settings do not establish initialization or disabled lending. |
| Absence-probe account / loan | HTTP 404; `success:false`, null data, code 404, error and error ID. Missing loan cache is not zero debt. |

Rechecked primary [balance-history](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-balance-history),
[settings](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-settings)
and [loan](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-loan-info)
references support the field meanings; the read-only official
[MCP tools](https://docs.pacifica.fi/api-documentation/api/mcp/tools) identify deposit
history. They do not add atomicity, no-later-effect or fresh-bootstrap guarantees.

Bounded private scope SHA256:
`6fcdcc6cf20a1a0422af73b103a9eab33747002ee3d7f8c646e6efdf50931b1b`.
Observer source SHA256:
`409515439f1bd337c5fb81d1f3e4ea8fea30498b916db9340ae0f15961563118`.
Deposit-body SHA256:
`3d4b4bb21f998f29688f188dae70d962f09ed3dee1875d65b1c7636a68560c13`.
Balance-body SHA256:
`11e55103975d80c987a3a88af4f84d7dbc25e82536e9af9194e2e9a90abc915a`.
These identify retained observations, not economic completeness or a deployment
approval. Private addresses/signatures and raw originals are not promoted.

Owning regressions promote only sanitized response **shapes**, replacing the
signature/time values. A withdrawal-containing current page keeps the original
hold; a deliberately synthetic pre-withdrawal prefix exercises first-credit
parsing without claiming a new live pass. The setup decoder now rejects a
conflicting top-level code even when `success:true`. Default lending, missing
loan fields/cache and conflicting error/code observations still refuse readiness.

**Distinction at the read-only handoff:** authenticated current formats and established-account
predicates are observed outside Nitro. A fresh initialized broker, shipping setup
provider, actual eligible customer deposit and changed-image measured financial
path remain unqualified. The recommended next test stages initialization with
faucet/test capital only, disables lending and verifies an empty baseline before
customer-vault funding. That separate bootstrap choice/run seal was presented for
user approval; it is not silently inherited from the old 120-second exception.

<a id="separate-staged-bootstrap"></a>
## Separate fresh test-capital staging and empty-account follow-up

The user explicitly approved this separate test on 2026-10-08 ("you have my
permission, go ahead"). `run-staged-20261008-01` uses two fresh disposable test
identities, the current configured CLI sponsor, only 20 faucet USDP and at most
0.10 devnet SOL, no AWS/customer/mainnet assets. No old test identity, approval,
120-second exception or lost-reply injection is reused.

Manifest seal: `b1d79fe8edc424e5443e6bef6cc65064c0ed4228bbe966d23f37c5e20709ed20`.
Executed source-bundle SHA256:
`5e1597404504d3fbd01e182f60628f0ab7a04ec975d06726c0e4394c6b9e960a`.
The separately selected staged schema allows at most four initialization read
pairs and a five-minute test-capital setup window; all monetary, HTTP/RPC/WSS and
20-minute total limits are unchanged. Original wires are simulated/persisted
before one exposure; the original withdrawal ACK is retained, not withheld.

The main invocation ran **05:41:18.373–05:45:03.536 UTC**, 225,163 ms:

| Evidence | Actual result |
| --- | --- |
| Main RPC / HTTP / finite WSS | 95 / 14 / 3, within caps |
| Controlled chain sends | Six originals, simulated and finalized; no resend |
| Faucet / deposit / fee / returned net | 20 / 20 / 1 / 19 USDP |
| Original native deposit | Exact finalized broker debit and venue-vault credit; matching original-signature transfer observation |
| Withdrawal/payment | One original UUID and retained successful ACK, batch-linked observation, exact native instruction and independently finalized recipient token effect |
| Owner / broker token balances | 19 / 0 USDP after finalized net return |
| Sponsor debit including its transaction fees | 0.06001 devnet SOL; unused test gas and token-account rent retained |
| Initialization replies | Successful idle 20-USDP account and zero-debt loan cache, not 404/missing-cache zeros |
| Main staging result | **Unresolved**, not a pass: the diagnostic mistakenly expected `equity` instead of `account_equity`, so no lending toggle was exposed |
| AWS / customer funds / trading / agent mutations / borrowing | None |

The field error belongs to Cinder's diagnostic, not a new venue-capability
failure. [Current account documentation](https://docs.pacifica.fi/api-documentation/api/rest-api/account/get-account-info)
and the retained actual response both name `account_equity`. The corrected parser
accepts the initial account/loan replies at their **original receive time** in an
offline replay, but this does not rewrite the original failed result. The
promoted account-shape regression refuses the invented `equity` alias and checks
stop orders, spot collateral and spot value as well as ordinary exposure.

All test net funds returned before the correction was used for a separate
**settings-only** continuation. Reusable `finish-staging.mjs` verifies the closed
parent's hash-linked journal, exact finalized owner/broker return effects, no
earlier lending-toggle exposure and remaining original time/HTTP budgets. Its
separate source/TLS/scope approval and child lock permit only ten HTTP calls and
one first lending toggle—no RPC, economic POST, chain send, new funding or retry.
The parent's result and journal are unchanged; final parent head remains
`dfd8306e0cda14efae56200973f215cb04a4c72abf9674c934e6ba118269bdb6`.

Follow-up scope seal:
`d2d0c4ef757a9ba419e4735a48ce88db00f5cbaa6970f880d945a9631f2b540d`.
Follow-up source-bundle SHA256:
`f5d8403baffeac8f3d01a9bc1e74602b0237cd1701e251269940b459af9ec68b`.
Its **ten HTTP calls passed**, giving **24 HTTP total** with unchanged 95 RPC /
three WSS / monetary exposure. The original total deadline remains applicable.

- Before the one setting request: established empty native account and positive
  loan-cache observations, zero borrowing/interest and no spot/order/position
  exposure. No missing-cache zero was accepted.
- Native POST succeeded; separate settings readback gives
  `auto_lend_disabled:true`. Subsequent loan/account/positions/orders observations
  show zero cash/pending/debt/interest/margin, no stop orders, spot assets or
  margin overrides and empty order/position arrays.
- Deposit history contains the exact original full 20-USDP signature; latest
  balance history is the completed withdrawal with zero balance/pending, **not**
  an eligible new customer credit.
- Result: `observed-empty-disabled-no-debt`. These are sequential Node-TLS
  observations outside Nitro, not an atomic `Setup.complete`, a native financial
  frontier, lost-ACK recovery or a shipping financial activation.

**Honest qualification boundary:** initialization, the original asset round trip,
and empty-account lending-disable/readback are observed on fresh identities.
The originally requested **disable-before-withdrawal** sequence did not pass and
is not retroactively qualified. A shipping provider and a fresh measured customer
vault/native-credit workflow remain open. Do not promote these local test keys or
withdrawn signature into a shipping release or new credit. Raw replies, original
wires/signatures, keys, RPC configuration and both approvals remain ignored and
owner-only locally; no asset/key/archive is deleted or rewritten.

Current offline verification: native diagnostic suite **35 passes**; vault
codec/layout suite **5 passes**; owning Pacifica funding suite **45 passes** plus
one existing named killed-child-worker ignore, and evidence suite **5 passes**.
The invocation's initial offline suite passed 33 tests; the field-shape/follow-up
eligibility fixes are separately tested after the failed main staging check.
All use pinned macOS Node 24.21.0 / Rust 1.97.1 and locked/offline Cargo with
`CARGO_INCREMENTAL=0`. Final `node scripts/check.mjs --group=contracts` passes
**78** discovered tests, the **54-document** foundation/link check and dependency
guard. `cargo fmt --all -- --check` / `git diff --check` pass; private RPC/key/reply
artifacts remain Git-ignored. No fresh Nitro/browser/SBF/ARM/full-workspace/hosted-CI pass,
commit, push, stack merge or phase closure is claimed.

## Next disposition

1. Account setup: connect the authenticated settings/loan/account observations to
   a reviewed shipping provider with exact broker/epoch/receive-time/complete
   setup semantics before customer funding. Fresh empty-account readiness now has
   actual evidence, but the disable-before-withdrawal staging sequence remains
   unqualified. Do not inherit this late success as an atomic policy or import
   test identities. The service also still needs original funding-route creation.
2. Native evidence: the actual signature/event/payment facts narrow the provider
   contract, but do not supply complete native causal coverage or recover an ACK
   genuinely unavailable to the controller. Final page/zero pending balance is
   not a no-later-effect certificate. Bring that named G01/interface or policy
   gap for review before shipping credit/payment activation.
3. Preserve the returned test assets, gas, private originals and reusable helper.
   No further financial invocation or changed-image AWS qualification is implied.
   P23 stays in progress; these facts do not qualify full vault/user payouts,
   trading/funding, recovery or the measured shipping financial workflow.
