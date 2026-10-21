//! Closed fresh-account PAPER experiment. This is a named test assumption, not
//! a general native financial frontier, customer deposit or recovery certificate.
//! Only the separately measured paper release installs this trusted policy.
use crate::{
    Error, Network,
    execution::Gateway,
    financial::{self, Target, account::Metric},
    observation::{Kind, MAGIC as OBSERVATION},
    reads,
};
use cinder_journal::{
    collateral,
    controller::Controller,
    funds,
    model::*,
    orders::{Intent, TimeInForce},
    risk,
};
use cinder_kernel::{
    amounts::*,
    identity::*,
    ledger::{evidence::*, *},
    position::Position,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const BOOTSTRAP: &[u8] = b"CINDER-BULK-PAPER-BOOTSTRAP-1\0";
/// Reject promotion of a paper history into an ordinary release, or rebinding
/// it to another experiment. A changed measured manifest does not erase the
/// economic purpose of already accepted cash and reconciliation assumptions.
pub fn history(policy: Option<&Policy>, j: &impl Controller) -> Result<(), Error> {
    for tx in j.transactions() {
        for e in &tx.evidence {
            if let Some(body) = e.as_bytes().strip_prefix(BOOTSTRAP)
                && policy.is_none_or(|p| !p.commitment().is_ok_and(|h| body == h))
            {
                return Err(Error::Unqualified);
            }
        }
    }
    Ok(())
}
pub(crate) fn active(j: &impl Controller) -> bool {
    j.transactions_with_receipts().any(|(t, r)| {
        r.controls.is_none()
            && t.evidence.iter().any(|e| {
                e.as_bytes()
                    .strip_prefix(BOOTSTRAP)
                    .is_some_and(|b| b.len() == 32)
            })
            && t.inputs.len() == 3
            && r.inputs
                .iter()
                .all(|i| *i == InputResult::Normalized(Disposition::Applied))
    })
}
/// Private measured experiment policy. No default, public setter or mainnet mode.
/// Original setup/quote artifacts must be retained by trusted preparation; their
/// hashes are approvals of those originals, not native authenticity certificates.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// Purpose/version must be exactly `closed-fresh-paper-1`.
    pub purpose: String,
    /// Fresh run nonce; never reused with another account/history.
    pub run: [u8; 32],
    /// Exact execution unit.
    pub unit: [u8; 32],
    /// Fresh exclusive native account, never an existing/customer wallet.
    pub account: [u8; 32],
    /// One disposable customer's owner verification key.
    pub customer: [u8; 32],
    /// Exact native profile, binding account, grids, quote and environment.
    pub profile: [u8; 32],
    /// Exact paper quote asset identity and precision revision.
    pub asset: [u8; 32],
    /// Quote denomination revision, not decimal places.
    pub precision: u32,
    /// Original fresh setup/faucet and reference-quote artifact commitments.
    pub setup: [u8; 32],
    /// Retained original trusted-preparation quote artifact digest.
    pub quote: [u8; 32],
    /// Trusted quote observation/start and fixed expiry (no boot renewal).
    pub observed_at: u64,
    /// Absolute deadline; restarting cannot renew it.
    pub expires_at: u64,
    /// At most $20, allocated FROM the observed paper cash, not added to it.
    pub customer_atoms: u64,
    /// At most $20 cumulative native limit notional, including cleanup reserve.
    pub notional_atoms: u64,
    /// Explicit test-house free buffer, in the same quote atoms.
    pub buffer_atoms: u64,
    /// Trusted prepared test mark and finite lower/upper scenario ticks.
    /// These are not a production oracle, calibrated capital or all-path guarantee.
    pub reference_ticks: u64,
    /// Finite lower shock, not a guaranteed price floor.
    pub lower_ticks: u64,
    /// Finite upper shock, not a guaranteed price ceiling.
    pub upper_ticks: u64,
    /// Upper cleanup price, conservatively reserved for every opening lot.
    pub cleanup_ticks: u64,
}
impl std::fmt::Debug for Policy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PaperPolicy([PRIVATE])")
    }
}
impl Policy {
    /// Immutable private policy identity, also bound by the measured capsule.
    pub fn commitment(&self) -> Result<[u8; 32], Error> {
        let body = serde_json::to_vec(self).map_err(|_| Error::Invalid)?;
        Ok(Sha256::digest([BOOTSTRAP, &body].concat()).into())
    }
    pub(crate) fn id(&self, tag: u8) -> Result<CommitId, Error> {
        CommitId::new(Sha256::digest([self.commitment()?.as_slice(), &[tag]].concat()).into())
            .map_err(|_| Error::Invalid)
    }
    /// Pure exact binding. Does not authenticate setup, observe cash or enable orders.
    pub fn validate(
        &self,
        gateway: &Gateway,
        d: &cinder_journal::units::Descriptor,
    ) -> Result<(), Error> {
        let (config, p, policy) = gateway.paper_binding();
        let scale = 10_u64
            .checked_pow(u32::from(p.places))
            .ok_or(Error::Limit)?;
        let ceiling = scale.checked_mul(20).ok_or(Error::Limit)?;
        if self.purpose != "closed-fresh-paper-1"
            || self.run == [0; 32]
            || self.setup == [0; 32]
            || self.quote == [0; 32]
            || self.customer == [0; 32]
            || self.customer == self.account
            || self.customer == d.funding_wallet
            || d.id.bytes() != self.unit
            || d.environment != Network::Testnet.commitment()
            || d.native_account != self.account
            || p.scope.network != Network::Testnet
            || p.scope.account() != self.account
            || p.target != Target::Master
            || p.commitment() != self.profile
            || p.quote.asset.bytes() != self.asset
            || p.quote.precision.get() != self.precision
            || p.places > 8
            || policy.unit != d.id
            || !gateway.financial_observations()
            || &d.ledger != config
            || d.ledger.markets.len() != 1
            || d.ledger.sources.len() != 3
            || [Location::Vault, Location::Broker, Location::Venue]
                .iter()
                .any(|location| {
                    d.ledger
                        .sources
                        .iter()
                        .filter(|s| s.location == *location)
                        .count()
                        != 1
                })
            || !d
                .ledger
                .sources
                .iter()
                .any(|s| s.scope == p.scope.source && s.location == Location::Venue)
            || self.customer_atoms == 0
            || self.customer_atoms > ceiling
            || self.notional_atoms == 0
            || self.notional_atoms > ceiling
            || self.buffer_atoms < self.notional_atoms
            || self.observed_at == 0
            || self.expires_at <= self.observed_at
            || self.expires_at - self.observed_at > 1_800_000
            || self.lower_ticks == 0
            || self.lower_ticks >= self.reference_ticks
            || self.reference_ticks >= self.upper_ticks
            || self.cleanup_ticks < self.reference_ticks
            || self.cleanup_ticks > self.upper_ticks
        {
            return Err(Error::Unqualified);
        }
        Ok(())
    }
    pub(crate) fn time(&self, at: u64) -> Result<(), Error> {
        if at < self.observed_at || at >= self.expires_at {
            Err(Error::Unqualified)
        } else {
            Ok(())
        }
    }
    /// Additional test-only history-wide budget. ACK/cancel/expiry never refunds
    /// the original possibly executed opening quantity or its reserved cleanup.
    pub fn order(&self, state: &State, intent: &Intent) -> Result<(), Error> {
        self.time(state.logical_time())?;
        let c = state.configuration();
        let account = cinder_journal::onboarding::account(c.domain, self.customer)
            .map_err(|_| Error::Invalid)?;
        let market = *c.markets.first().ok_or(Error::Invalid)?;
        if c.markets.len() != 1
            || c.quote.asset.bytes() != self.asset
            || c.quote.precision.get() != self.precision
            || intent.request.account != account
            || intent.quantity.unit() != market.unit()
            || intent.expires_at > self.expires_at
            || intent.minimum.ticks() < self.lower_ticks
            || intent.maximum.ticks() > self.cleanup_ticks
        {
            return Err(Error::Unqualified);
        }
        let mut opening = 0_i64;
        let mut closed = 0_u64;
        let mut spent = 0_i128;
        let mut cleanup_started = false;
        let mut count = 0;
        for order in state
            .orders()
            .iter()
            .map(|o| &o.intent)
            .chain(std::iter::once(intent))
        {
            count += 1;
            if count > 5 {
                return Err(Error::Limit);
            }
            let q = order.quantity.lots();
            if q == 0 || order.request.account != account {
                return Err(Error::Unqualified);
            }
            spent = spent
                .checked_add(
                    market
                        .notional(
                            QuantityLots::new(
                                market.unit(),
                                i64::try_from(q.unsigned_abs()).map_err(|_| Error::Limit)?,
                            ),
                            order.maximum,
                        )
                        .map_err(|_| Error::Invalid)?
                        .atoms(),
                )
                .ok_or(Error::Limit)?;
            if order.reduce_only {
                cleanup_started = true;
                if opening == 0
                    || q.signum() == opening.signum()
                    || order.time_in_force != TimeInForce::ImmediateOrCancel
                {
                    return Err(Error::Unqualified);
                }
                closed = closed.checked_add(q.unsigned_abs()).ok_or(Error::Limit)?;
                if closed > opening.unsigned_abs() {
                    return Err(Error::Limit);
                }
            } else {
                if cleanup_started || count > 4 || opening != 0 && q.signum() != opening.signum() {
                    return Err(Error::Unqualified);
                }
                opening = opening.checked_add(q).ok_or(Error::Limit)?;
            }
        }
        let remaining = opening
            .unsigned_abs()
            .checked_sub(closed)
            .ok_or(Error::Limit)?;
        let reserve = market
            .notional(
                QuantityLots::new(
                    market.unit(),
                    i64::try_from(remaining).map_err(|_| Error::Limit)?,
                ),
                PriceTicks::new(market.unit(), self.cleanup_ticks).map_err(|_| Error::Invalid)?,
            )
            .map_err(|_| Error::Invalid)?
            .atoms();
        if spent.checked_add(reserve).ok_or(Error::Limit)? > i128::from(self.notional_atoms) {
            return Err(Error::Limit);
        }
        Ok(())
    }
}

fn retained<'a>(
    g: &Gateway,
    j: &'a impl Controller,
    id: CommitId,
    kind: Kind,
    now: u64,
) -> Result<(&'a [u8], u64), Error> {
    let tx = j.transaction(id).ok_or(Error::Unqualified)?;
    let prefix = [
        OBSERVATION,
        g.paper_binding().1.commitment().as_slice(),
        &[kind as u8],
    ]
    .concat();
    let mut records = tx
        .evidence
        .iter()
        .filter_map(|e| e.as_bytes().strip_prefix(prefix.as_slice()));
    let record = records.next().ok_or(Error::Unqualified)?;
    if records.next().is_some()
        || record.len() < 8
        || j.transaction_receipt(id)
            .is_none_or(|r| r.controls.is_some())
    {
        return Err(Error::Unqualified);
    }
    let time = u64::from_be_bytes(record[..8].try_into().map_err(|_| Error::Invalid)?);
    let body = &record[8..];
    if time > now || now - time > 60_000 || !g.read_succeeded(tx, Sha256::digest(body).into())? {
        return Err(Error::Unqualified);
    }
    Ok((body, time))
}

/// Observe exactly four original families, then atomically allocate the same
/// cash and install the bounded test policy. A persisted request without its
/// response is never resent. Retry after uncertainty requires protected reopen;
/// a complete original bootstrap is recognized without posting again.
pub fn initialize(
    p: &Policy,
    g: &Gateway,
    j: &mut impl Controller,
    transport: &mut impl reads::Transport,
    now: impl Fn() -> Result<u64, Error>,
) -> Result<(), Error> {
    p.validate(g, j.descriptor().ok_or(Error::Unqualified)?)?;
    history(Some(p), j)?;
    p.time(now()?)?;
    j.verified_state().map_err(|_| Error::Unavailable)?;
    let marker = [BOOTSTRAP, p.commitment()?.as_slice()].concat();
    let commit = p.id(20)?;
    if let Some(tx) = j.transaction(commit) {
        return if tx.evidence.first().is_some_and(|e| e.as_bytes() == marker)
            && j.transaction_receipt(commit).is_some_and(|r| {
                r.controls.is_none()
                    && r.inputs
                        .iter()
                        .all(|r| *r == InputResult::Normalized(Disposition::Applied))
            }) {
            Ok(())
        } else {
            Err(Error::Conflict)
        };
    }
    let state = j.state().map_err(|_| Error::Unavailable)?;
    if state.ledger().version() != 0
        || !state.ledger().events().is_empty()
        || !state.orders().is_empty()
        || !state.attempts().is_empty()
        || state.has_joined_risk()
        || state.registrations().len() != 1
        || state.registrations()[0].wallet != p.customer
    {
        return Err(Error::Unqualified);
    }
    let families = [Kind::Account, Kind::Trades, Kind::Funding, Kind::Risk];
    for (i, kind) in families.iter().enumerate() {
        let request = p.id(i as u8 * 2)?;
        let evidence = p.id(i as u8 * 2 + 1)?;
        if j.transaction(evidence).is_some() {
            continue;
        }
        if j.transaction(request).is_some() {
            return Err(Error::Unqualified);
        }
        let at = now()?;
        p.time(at)?;
        let (r, c) = reads::prepare(
            g,
            j,
            reads::Poll {
                kind: *kind,
                reservation: request,
                evidence,
                range: None,
                continuation: None,
                limit: 128,
                at,
                cleanup: false,
            },
        )?;
        let reply = transport.query(r);
        let at = now()?;
        p.time(at)?;
        if matches!(kind, Kind::Funding | Kind::Risk) {
            reads::complete_fresh_paper_history(p, g, j, c, reply, at)?;
        } else {
            reads::complete(g, j, c, reply, at)?;
        }
    }
    let at = now()?;
    p.time(at)?;
    j.verified_state().map_err(|_| Error::Unavailable)?;
    let (_, profile, native_policy) = g.paper_binding();
    let (body, _) = retained(g, j, p.id(1)?, Kind::Account, at)?;
    let snapshot = financial::account::decode_account(profile, body)?;
    let envelope: Vec<crate::raw::Object> = crate::raw::decode(body)?;
    let account: crate::raw::Object = envelope.first().ok_or(Error::Invalid)?.get("fullAccount")?;
    let agents: Vec<String> = account.get("authorizedAgentWallets")?;
    if agents != vec![bs58::encode(g.agent_key()).into_string()] {
        return Err(Error::Unqualified);
    }
    let cash = snapshot
        .local
        .get(Metric::Total)
        .ok_or(Error::Unqualified)?;
    if cash.atoms() < i128::from(p.customer_atoms).checked_add(i128::from(p.buffer_atoms)).ok_or(Error::Limit)?
        || snapshot.local.get(Metric::Available) != Some(cash)
        || [Metric::Maintenance, Metric::Notional, Metric::RealizedPnl, Metric::UnrealizedPnl, Metric::Fees, Metric::Funding]
            .iter().any(|m| snapshot.local.get(*m).is_none_or(|v| v.atoms() != 0))
        || !snapshot.positions.is_empty() || !snapshot.held.is_empty() || !snapshot.open_orders.is_empty()
        // The observed fullAccount schema omits these optional inventories.
        // Absence stays unknown in the decoder. Only the approved exclusive
        // fresh-account/setup assumption closes that gap for this experiment.
        || snapshot.pending_on_fill.as_ref().is_some_and(|v| !v.is_empty())
        || snapshot.children.as_ref().is_some_and(|v| !v.is_empty())
        || snapshot.locks.as_ref().is_some_and(|v| !v.is_empty())
    {
        return Err(Error::Unqualified);
    }
    let (body, _) = retained(g, j, p.id(3)?, Kind::Trades, at)?;
    let trades = crate::fills::rest(
        &profile.scope,
        &profile
            .mappings
            .iter()
            .map(|m| m.wire.clone())
            .collect::<Vec<_>>(),
        body,
    )?;
    let (body, _) = retained(g, j, p.id(5)?, Kind::Funding, at)?;
    let funding = financial::history::funding(profile, body)?;
    let (body, _) = retained(g, j, p.id(7)?, Kind::Risk, at)?;
    let risk_page = financial::history::risk(profile, body)?;
    // These checks are only inputs to the approved closed-account assumption.
    // They do NOT manufacture a global native frontier or a terminal witness.
    if !trades.fills.is_empty()
        || !trades.unmapped.is_empty()
        || trades.next_cursor.is_some()
        || trades.start_slot != 0
        || trades.min_available_slot.is_some_and(|s| s > 0)
        || trades.coverage == crate::fills::Coverage::Partial
        || [&funding.cut, &risk_page.cut].iter().any(|c| {
            c.next_cursor.is_some()
                || c.start_slot != 0
                || c.min_available_slot.is_some_and(|s| s > 0)
                || c.coverage == crate::fills::Coverage::Partial
        })
        || !funding.rows.is_empty()
        || !risk_page.rows.is_empty()
    {
        return Err(Error::Unqualified);
    }
    let config = j.configuration().clone();
    let scope = profile.scope.source;
    let customer = cinder_journal::onboarding::account(config.domain, p.customer)
        .map_err(|_| Error::Invalid)?;
    let event_id = EconomicEventId::new(&p.commitment()?).map_err(|_| Error::Invalid)?;
    let event_key = |leg| EventKey {
        scope,
        event: event_id.clone(),
        leg,
    };
    let event = |leg, change| Event {
        key: RecordKey::Economic(event_key(leg)),
        policy: config.policy,
        change,
    };
    let amount = |n| QuoteAtoms::new(config.quote, n);
    let raw = PrivateBytes::new(marker.clone()).map_err(|_| Error::Limit)?;
    let inputs = vec![
        event(
            0,
            Change::Receipt {
                owner: Owner::Customer(customer),
                location: Location::Venue,
                amount: amount(i128::from(p.customer_atoms)),
            },
        ),
        event(
            1,
            Change::Receipt {
                owner: Owner::House,
                location: Location::Venue,
                amount: amount(cash.atoms() - i128::from(p.customer_atoms)),
            },
        ),
        event(
            2,
            Change::Reconcile(NativeCheck {
                expected_version: 2,
                cash: Some(cash),
                funding: Some(amount(0)),
                positions: Some(
                    config
                        .markets
                        .iter()
                        .map(|m| Position::flat(m.unit()))
                        .collect(),
                ),
                // Qualified ONLY under this policy's explicitly approved paper assumption.
                complete: true,
                resolves: vec![],
            }),
        ),
    ]
    .into_iter()
    .map(|event| Input {
        source: scope,
        source_cut: None,
        authority_epoch: native_policy.revision,
        observed_at: at,
        raw: raw.clone(),
        event: Some(event),
    })
    .collect();
    // Literal empty bootstrap histories were retained as evidence, not turned
    // into unresolved financial events and "resolved" with an invented cut.
    // Any actually contained input still blocks the allocation. Ordinary raw
    // resolution and ordinary funding/risk ingestion remain unchanged.
    if !j
        .state()
        .map_err(|_| Error::Unavailable)?
        .raw_inputs()
        .is_empty()
    {
        return Err(Error::Unqualified);
    }
    let mut controls = vec![];
    let market = config.markets[0].unit();
    let price = |ticks| PriceTicks::new(market, ticks).map_err(|_| Error::Invalid);
    controls.push(Control::Collateral(collateral::Cut {
        expected_version: 3,
        policy: collateral::Policy {
            revision: config.policy,
            markets: vec![collateral::MarginRule {
                market,
                private_bps: 10_000,
                native_bps: 10_000,
            }],
            evidence: EvidencePolicy {
                max_mark_age: 1_800_000,
                max_issue_age: 60_000,
                max_check_age: 1_800_000,
            },
        },
        marks: vec![MarkObservation {
            price: price(p.reference_ticks)?,
            evidence: event_key(3),
            observed_at: p.observed_at,
            valid_until: p.expires_at,
            qualified: true,
        }],
    }));
    controls.push(Control::Risk(risk::Action::Install {
        expected_version: 3,
        policy: Box::new(risk::Policy {
            revision: config.policy,
            markets: vec![risk::MarketRule {
                market,
                maximum_leverage: 10_000,
                maintenance_bps: 5_000,
                native_maintenance_bps: 5_000,
                gross_limit: amount(i128::from(p.notional_atoms)),
                net_limit: amount(i128::from(p.notional_atoms)),
            }],
            buffer: amount(i128::from(p.buffer_atoms)),
            horizon_ms: 1_000,
            valid_until: p.expires_at,
            paths: [p.lower_ticks, p.upper_ticks]
                .into_iter()
                .enumerate()
                .map(|(i, ticks)| {
                    Ok(risk::Path {
                        id: Sha256::digest([p.commitment()?.as_slice(), &[30 + i as u8]].concat())
                            .into(),
                        steps: vec![risk::Step {
                            after_ms: 1_000,
                            marks: vec![price(ticks)?],
                            events: vec![],
                            liquidity: [Location::Vault, Location::Broker, Location::Venue]
                                .into_iter()
                                .map(|location| risk::Liquidity {
                                    location,
                                    accessible_bps: if location == Location::Venue {
                                        10_000
                                    } else {
                                        0
                                    },
                                    due: amount(0),
                                })
                                .collect(),
                        }],
                    })
                })
                .collect::<Result<_, Error>>()?,
        }),
    }));
    controls.push(Control::Funds(funds::Action::NativeCreditReady(true)));
    let tx = Transaction {
        id: commit,
        expected: j.head(),
        at,
        evidence: vec![PrivateBytes::new(marker).map_err(|_| Error::Limit)?],
        inputs,
        order_observations: vec![],
        funds_observations: vec![],
        controls,
    };
    let preview = j.preview(&tx).map_err(|_| Error::Unavailable)?;
    if preview.controls.is_some()
        || preview
            .inputs
            .iter()
            .any(|r| *r != InputResult::Normalized(Disposition::Applied))
    {
        return Err(Error::Unqualified);
    }
    p.time(now()?)?;
    let result = j.commit(tx).map_err(|_| Error::Unavailable)?;
    if result.duplicate || result.receipt != preview {
        return Err(Error::Unavailable);
    }
    Ok(())
}
