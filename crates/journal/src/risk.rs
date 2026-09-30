//! Bounded joined admission over the authoritative ledger, not another asset book.
//! Scenario coverage/calibration and authenticated policy installation are trusted
//! deployment obligations. Finite path acceptance is not a solvency guarantee.
use crate::{
    Error,
    model::*,
    orders::Approval,
    wire::{self, Reader, Writer},
};
use cinder_kernel::{
    amounts::*,
    identity::*,
    ledger::{economics::EconomicChange, evidence::Disposition, *},
    math::{Rounding, mul_div},
    position::Market,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

type Result<T> = std::result::Result<T, ControlError>;
const SCALE: u64 = 10_000;
const MAX_PATHS: usize = 32;
const MAX_STEPS: usize = 64;

/// Versioned market risk limits; numbers are explicit inputs, not venue defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketRule {
    /// Exact market/precision identity.
    pub market: MarketUnit,
    /// Maximum private leverage scaled by 10,000 (25,000 is 2.5x).
    pub maximum_leverage: u64,
    /// Private maintenance fraction in basis points.
    pub maintenance_bps: u32,
    /// Native maintenance fraction; native initial margin comes from collateral policy.
    pub native_maintenance_bps: u32,
    /// Maximum private gross marked notional, including independent pending outcomes.
    pub gross_limit: QuoteAtoms,
    /// Maximum native absolute marked notional, including independent pending outcomes.
    pub net_limit: QuoteAtoms,
}
/// Conservative location availability at a specific scenario prefix/deadline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Liquidity {
    /// Exact location in this pool/asset. No cross-location reuse.
    pub location: Location,
    /// Fraction of derived cash accessible by this deadline, 0..=10,000.
    pub accessible_bps: u32,
    /// Additional obligations due here, excluding existing shared reservations.
    pub due: QuoteAtoms,
}
/// One hypothetical prefix replayed through the same ledger transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// Monotone elapsed milliseconds inside the configured closeout horizon.
    pub after_ms: u64,
    /// Complete hypothetical price vector; not an observation or a live mark setter.
    pub marks: Vec<PriceTicks>,
    /// Explicit hypothetical executions. No receipts, future fees/rebates or fundraising.
    pub events: Vec<Event>,
    /// Vault/native and, when configured, broker locations exactly once.
    pub liquidity: Vec<Liquidity>,
}
/// Explicit finite path. Each prefix, not just its ending, constrains capital.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    /// Stable unique policy identity, not a display label.
    pub id: [u8; 32],
    /// Bounded sequence of shocks, execution costs and deadline observations.
    pub steps: Vec<Step>,
}
/// Trusted risk configuration. This type does not authenticate its installer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    /// Strictly monotone revision for changed configuration.
    pub revision: PolicyVersion,
    /// Complete exact market rules.
    pub markets: Vec<MarketRule>,
    /// Required free capital after the worst modeled prefix.
    pub buffer: QuoteAtoms,
    /// Maximum modeled closeout duration, not an automatic cancellation timeout.
    pub horizon_ms: u64,
    /// Policy/scenario qualification expiry; stale numerical coverage refuses admission.
    pub valid_until: u64,
    /// Explicit nonempty finite scenarios; empty does not mean safe.
    pub paths: Vec<Path>,
}
/// Authenticated private preference; does not resize a position or native leverage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// One-shot customer/deployment identity.
    pub request: RequestKey,
    /// Exact private market.
    pub market: MarketUnit,
    /// Scaled chosen leverage, within the current cap.
    pub leverage: u64,
    /// Bound risk-policy revision.
    pub policy: PolicyVersion,
    /// Current private authority epoch.
    pub authority_epoch: u64,
    /// Expiry in the injected time domain.
    pub expires_at: u64,
}
impl Selection {
    /// Canonical digest for the P18 authentication port, not a signature verifier.
    pub fn digest(&self) -> std::result::Result<[u8; 32], Error> {
        let mut w = Writer::new(40);
        encode_selection(&mut w, self);
        Ok(Sha256::digest(w.finish()?).into())
    }
}
/// Atomic risk controls. Actual financial observations are always ingested first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Record a qualified policy even if it causes restrictions, not hidden liquidation.
    Install {
        /// Exact current financial version; journal head additionally covers all holds.
        expected_version: u64,
        /// Trusted configuration, never a customer-selected bypass.
        policy: Box<Policy>,
    },
    /// Change private initial-margin preference only if the joined result is admissible.
    Select {
        /// Bound request.
        selection: Box<Selection>,
        /// Verified authentication-port result.
        approval: Approval,
    },
}
/// Distinct flags: a buffer breach is not itself insolvency or missed payment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Flags {
    /// Positive customer claims exceed marked backing.
    pub insolvent: bool,
    /// A location cannot meet commitments/deadline demands.
    pub illiquid: bool,
    /// Private initial margin/commitments fail.
    pub private_initial: bool,
    /// Actual private maintenance breach; not triggered just by pending orders.
    pub private_maintenance: bool,
    /// Native initial-margin envelope fails.
    pub native_initial: bool,
    /// Native maintenance envelope fails.
    pub native_maintenance: bool,
    /// Free capital falls below configured target.
    pub capital: bool,
    /// Gross or net concentration envelope fails.
    pub concentration: bool,
}
/// One owner dimension at the same cut; all values are derived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Customer {
    /// Private identity.
    pub account: AccountId,
    /// Conservative current equity, excluding positive unsettled funding.
    pub equity: QuoteAtoms,
    /// Current private initial margin under selected/capped leverage.
    pub initial: QuoteAtoms,
    /// Current maintenance, independent of leverage preference.
    pub maintenance: QuoteAtoms,
    /// Worst pending position margin plus adverse execution/fee cost.
    pub outcome_requirement: QuoteAtoms,
    /// Shared non-order commitments, e.g. pending payouts.
    pub other_held: QuoteAtoms,
    /// Free after max(explicit order hold, derived pending incremental requirement).
    pub free: QuoteAtoms,
}
/// Market concentration does not net private long and short exposure together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Concentration {
    /// Exact market.
    pub market: MarketUnit,
    /// Upper bound on private gross notional.
    pub gross: QuoteAtoms,
    /// Upper bound on native absolute notional.
    pub native: QuoteAtoms,
}
/// One instantaneous derived state, including the reachable pending interval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Explicit independent failures.
    pub flags: Flags,
    /// Private margin dimensions.
    pub customers: Vec<Customer>,
    /// Gross/net concentration dimensions.
    pub concentration: Vec<Concentration>,
    /// Native conservative marked equity before pending adverse execution cost.
    pub native_equity: QuoteAtoms,
    /// Native pending-inclusive initial requirement.
    pub native_requirement: QuoteAtoms,
    /// Eligible free house capital after existing commitments and pending deficit bound.
    pub free_capital: QuoteAtoms,
    /// Available vault cash minus shared source commitments and due demands.
    pub vault_free: QuoteAtoms,
    /// Available native cash/free collateral minus source commitments and due demands.
    pub venue_free: QuoteAtoms,
    /// Available broker-wallet tokens, never native margin or vault payout liquidity.
    pub broker_free: QuoteAtoms,
}
/// Every tested prefix remains inspectable; no success-only summary hides failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    /// Scenario identity.
    pub path: [u8; 32],
    /// Start and every subsequent prefix.
    pub prefixes: Vec<Snapshot>,
}
/// Joined report at the caller's current immutable journal state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Current state, independently distinguished from hypothetical distress.
    pub current: Snapshot,
    /// Minimum eligible free capital across all prefixes, including the start.
    pub minimum_free: QuoteAtoms,
    /// Buffer plus peak erosion relative to current eligible free capital.
    pub target: QuoteAtoms,
    /// Replayable finite scenario results.
    pub paths: Vec<Trace>,
    /// All current and prospective admission requirements passed; not future safety proof.
    pub admissible: bool,
}

fn add(a: i128, b: i128) -> Result<i128> {
    a.checked_add(b).ok_or(ControlError::Capacity)
}
fn sub(a: i128, b: i128) -> Result<i128> {
    a.checked_sub(b).ok_or(ControlError::Capacity)
}
fn abs(a: i128) -> Result<i128> {
    a.checked_abs().ok_or(ControlError::Capacity)
}
fn ratio(a: i128, n: u64, d: u64) -> Result<i128> {
    mul_div(a, n, d, Rounding::Ceil).map_err(|_| ControlError::Capacity)
}
fn value(m: Market, q: i128, p: PriceTicks) -> Result<i128> {
    let (n, d) = m.conversion();
    mul_div(
        q.checked_mul(i128::from(p.ticks()))
            .ok_or(ControlError::Capacity)?,
        n,
        d,
        Rounding::Ceil,
    )
    .map_err(|_| ControlError::Capacity)
}

// Build once per report. Retained identities still prevent replay, but released
// history must not multiply the work of every customer/scenario prefix.
struct ActiveRisk<'a> {
    requests: BTreeSet<(NetworkId, DeploymentId, AccountId, RequestId)>,
    holds: Vec<&'a Hold>,
    order_holds: Vec<&'a Hold>,
    orders: Vec<&'a crate::orders::Order>,
}
fn request_index(k: RequestKey) -> (NetworkId, DeploymentId, AccountId, RequestId) {
    (k.domain.network, k.domain.deployment, k.account, k.request)
}
impl<'a> ActiveRisk<'a> {
    fn new(s: &'a State) -> Self {
        let order_ids: BTreeSet<_> = s
            .orders
            .iter()
            .map(|o| request_index(o.intent.request))
            .collect();
        let holds: Vec<_> = s.holds.iter().filter(|h| h.active).collect();
        let hold_ids: BTreeSet<_> = holds.iter().map(|h| request_index(h.request)).collect();
        let order_holds = holds
            .iter()
            .copied()
            .filter(|h| order_ids.contains(&request_index(h.request)))
            .collect();
        let orders = s
            .orders
            .iter()
            .filter(|o| o.remaining() != 0 && hold_ids.contains(&request_index(o.intent.request)))
            .collect();
        Self {
            requests: hold_ids,
            holds,
            order_holds,
            orders,
        }
    }
    fn held(&self, r: Resource) -> Result<i128> {
        self.holds
            .iter()
            .flat_map(|h| &h.reservations)
            .filter(|h| h.resource == r)
            .try_fold(0, |a, h| add(a, h.amount.atoms()))
    }
    fn order_hold(&self, r: Resource) -> Result<i128> {
        self.order_holds
            .iter()
            .flat_map(|h| &h.reservations)
            .filter(|h| h.resource == r)
            .try_fold(0, |a, h| add(a, h.amount.atoms()))
    }
}

impl State {
    pub(crate) fn risk_action(&mut self, a: &Action) -> Result<()> {
        match a {
            Action::Install {
                expected_version,
                policy,
            } => {
                if *expected_version != self.ledger.version()
                    || self.risk.as_ref().is_some_and(|old| {
                        policy.revision < old.revision
                            || policy.revision == old.revision && old != policy.as_ref()
                    })
                {
                    return Err(ControlError::Invalid);
                }
                self.validate_risk(policy)?;
                self.risk = Some((**policy).clone());
            }
            Action::Select {
                selection: s,
                approval,
            } => {
                self.request(s.request)?;
                let p = self.risk.as_ref().ok_or(ControlError::Unqualified)?;
                let m = p
                    .markets
                    .iter()
                    .find(|m| m.market == s.market)
                    .ok_or(ControlError::Invalid)?;
                if self.selections.len() >= wire::MAX_ITEMS
                    || self.selections.iter().any(|old| old.request == s.request)
                    || self.holds.iter().any(|h| h.request == s.request)
                    || s.policy != p.revision
                    || s.leverage < SCALE
                    || s.leverage > m.maximum_leverage
                    || self.authority(s.request.account) != Some(s.authority_epoch)
                    || approval.account != s.request.account
                    || approval.authority_epoch != s.authority_epoch
                    || approval.intent_hash != s.digest().map_err(|_| ControlError::Invalid)?
                {
                    return Err(ControlError::Invalid);
                }
                if s.expires_at <= self.now {
                    return Err(ControlError::Expired);
                }
                self.selections.push((**s).clone());
                self.risk_gate()?;
            }
        }
        Ok(())
    }
    fn validate_risk(&self, p: &Policy) -> Result<()> {
        let c = self.collateral.as_ref().ok_or(ControlError::Unqualified)?;
        if p.valid_until < self.now {
            return Err(ControlError::Unqualified);
        }
        if p.buffer.unit() != self.config.quote
            || p.buffer.atoms() < 0
            || p.horizon_ms == 0
            || p.markets.len() != self.config.markets.len()
            || p.paths.is_empty()
            || p.paths.len() > MAX_PATHS
        {
            return Err(ControlError::Invalid);
        }
        for (i, r) in p.markets.iter().enumerate() {
            let old = c
                .policy
                .markets
                .iter()
                .find(|m| m.market == r.market)
                .ok_or(ControlError::Invalid)?;
            if p.markets[..i].iter().any(|m| m.market == r.market)
                || r.maximum_leverage < SCALE
                || r.maintenance_bps == 0
                || u128::from(r.maintenance_bps) * u128::from(r.maximum_leverage) >= 100_000_000
                || r.native_maintenance_bps == 0
                || r.native_maintenance_bps >= old.native_bps
                || r.gross_limit.unit() != self.config.quote
                || r.gross_limit.atoms() < 0
                || r.net_limit.unit() != self.config.quote
                || r.net_limit.atoms() < 0
            {
                return Err(ControlError::Invalid);
            }
        }
        for (i, path) in p.paths.iter().enumerate() {
            if path.id == [0; 32]
                || p.paths[..i].iter().any(|old| old.id == path.id)
                || path.steps.is_empty()
                || path.steps.len() > MAX_STEPS
            {
                return Err(ControlError::Invalid);
            }
            let mut at = 0;
            for step in &path.steps {
                if step.after_ms < at
                    || step.after_ms > p.horizon_ms
                    || step.events.len() > MAX_STEPS
                    || step.marks.len() != self.config.markets.len()
                    || step.liquidity.len()
                        != 2 + usize::from(
                            self.config
                                .sources
                                .iter()
                                .any(|s| s.location == Location::Broker),
                        )
                {
                    return Err(ControlError::Invalid);
                }
                at = step.after_ms;
                for (j, mark) in step.marks.iter().enumerate() {
                    if !self.config.markets.iter().any(|m| m.unit() == mark.unit())
                        || step.marks[..j].iter().any(|m| m.unit() == mark.unit())
                    {
                        return Err(ControlError::Invalid);
                    }
                }
                for (j, l) in step.liquidity.iter().enumerate() {
                    if l.accessible_bps > 10_000
                        || l.location == Location::Broker
                            && !self
                                .config
                                .sources
                                .iter()
                                .any(|s| s.location == Location::Broker)
                        || l.due.unit() != self.config.quote
                        || l.due.atoms() < 0
                        || step.liquidity[..j]
                            .iter()
                            .any(|old| old.location == l.location)
                    {
                        return Err(ControlError::Invalid);
                    }
                }
                for e in &step.events {
                    // Scenarios execute existing economics; cannot inject future capital,
                    // hoped-for rebates, fees earned from customers or rewritten snapshots.
                    let key_ok = match &e.change {
                        Change::Fill { .. } => matches!(e.key, RecordKey::Economic(_)),
                        Change::Restoration(
                            cinder_kernel::ledger::restoration::RestorationChange::Observe {
                                fee,
                                ..
                            }
                            | cinder_kernel::ledger::restoration::RestorationChange::Execution {
                                fee,
                                ..
                            },
                        ) if fee.atoms() >= 0 => matches!(e.key, RecordKey::Economic(_)),
                        Change::Restoration(
                            cinder_kernel::ledger::restoration::RestorationChange::Declare {
                                ..
                            }
                            | cinder_kernel::ledger::restoration::RestorationChange::Void { .. },
                        ) => matches!(e.key, RecordKey::Request(_)),
                        Change::Restoration(
                            cinder_kernel::ledger::restoration::RestorationChange::Bind { .. },
                        ) => matches!(e.key, RecordKey::Attempt(_)),
                        Change::Close(cinder_kernel::ledger::close::CloseChange::Bind {
                            ..
                        }) => matches!(e.key, RecordKey::Attempt(_)),
                        Change::Close(cinder_kernel::ledger::close::CloseChange::Execution {
                            fee,
                            ..
                        }) if fee.atoms() >= 0 => matches!(e.key, RecordKey::Economic(_)),
                        Change::Protection(
                            cinder_kernel::ledger::protection::ProtectionChange::Recognize {
                                ..
                            }
                            | cinder_kernel::ledger::protection::ProtectionChange::Commit { .. }
                            | cinder_kernel::ledger::protection::ProtectionChange::Absorb { .. },
                        ) => matches!(e.key, RecordKey::Request(_)),
                        Change::Economics(EconomicChange::Execution { fee, .. })
                            if fee.atoms() >= 0 =>
                        {
                            matches!(e.key, RecordKey::Economic(_))
                        }
                        _ => return Err(ControlError::Invalid),
                    };
                    if !key_ok
                        || e.key.require_domain(self.config.domain).is_err()
                        || e.policy != self.config.policy
                    {
                        return Err(ControlError::Invalid);
                    }
                }
            }
        }
        Ok(())
    }
    fn leverage(&self, id: AccountId, r: &MarketRule) -> u64 {
        self.selections
            .iter()
            .rev()
            .find(|s| s.request.account == id && s.market == r.market)
            .map_or(r.maximum_leverage, |s| s.leverage.min(r.maximum_leverage))
    }
    /// Bounds all independent partial outcomes by their signed inventory interval
    /// and total adverse execution cost. Favorable fills cannot finance another order.
    fn pending(
        &self,
        active: &ActiveRisk<'_>,
        owner: Option<AccountId>,
        market: Market,
        mark: PriceTicks,
    ) -> Result<(i128, i128, i128)> {
        let (mut buys, mut sells, mut cost) = (0, 0, 0);
        for o in active.orders.iter().filter(|o| {
            o.intent.quantity.unit() == market.unit()
                && owner.is_none_or(|id| {
                    id == o.intent.request.account
                        && !self
                            .restorations
                            .iter()
                            .any(|r| r.request == o.intent.request)
                        && !self.closes.iter().any(|c| {
                            c.request == o.intent.request
                                && c.kind == crate::liquidation::Kind::HouseUnwind
                        })
                })
        }) {
            let q = i128::from(o.remaining());
            if q == 0 {
                continue;
            }
            // Conservative rational valuation: ceil positive magnitude/cost. This
            // bounds all representable fills, without assuming one atomic-lot price is exact.
            let edge = if o.intent.quantity.lots() > 0 {
                buys = add(buys, q)?;
                o.intent.maximum.ticks().saturating_sub(mark.ticks())
            } else {
                sells = add(sells, q)?;
                mark.ticks().saturating_sub(o.intent.minimum.ticks())
            };
            let (n, d) = market.conversion();
            let loss = mul_div(
                q.checked_mul(i128::from(edge))
                    .ok_or(ControlError::Capacity)?,
                n,
                d,
                Rounding::Ceil,
            )
            .map_err(|_| ControlError::Capacity)?;
            cost = add(
                cost,
                add(
                    loss,
                    q.checked_mul(o.intent.maximum_fee_per_lot.atoms())
                        .ok_or(ControlError::Capacity)?,
                )?,
            )?;
        }
        if let Some(owner) = owner {
            for r in self.ledger.reductions().iter().filter(|r| {
                r.quantity.unit() == market.unit()
                    && r.attempt
                        .is_some_and(|a| active.requests.contains(&request_index(a.request)))
            }) {
                let Some(row) = r.rows.iter().find(|r| r.owner == owner && !r.void) else {
                    continue;
                };
                // Independent row maxima deliberately overbound correlated quota
                // outcomes; never assume a favorable partial-fill order.
                let q = i128::from((row.amount - row.restored).min(r.target - r.consumed));
                let edge = if r.quantity.lots() < 0 {
                    buys = add(buys, q)?;
                    r.price.ticks().saturating_sub(mark.ticks())
                } else {
                    sells = add(sells, q)?;
                    mark.ticks().saturating_sub(r.price.ticks())
                };
                let (n, d) = market.conversion();
                let loss = mul_div(
                    q.checked_mul(i128::from(edge))
                        .ok_or(ControlError::Capacity)?,
                    n,
                    d,
                    Rounding::Ceil,
                )
                .map_err(|_| ControlError::Capacity)?;
                if q > 0 {
                    cost = add(cost, add(loss, 2)?)?;
                }
            }
        }
        Ok((buys, sells, cost))
    }
    fn book_equity(&self, b: &Book, marks: &[PriceTicks]) -> Result<i128> {
        let mut e = add(b.cash().atoms(), b.funding().atoms().min(0))?;
        for (m, pos) in self.config.markets.iter().zip(b.positions()) {
            let mark = *marks
                .iter()
                .find(|p| p.unit() == m.unit())
                .ok_or(ControlError::Unqualified)?;
            e = add(
                e,
                pos.unrealized(*m, mark)
                    .map_err(|_| ControlError::Capacity)?
                    .atoms(),
            )?;
        }
        Ok(e)
    }
    fn risk_snapshot(
        &self,
        active: &ActiveRisk<'_>,
        ledger: &Ledger,
        marks: &[PriceTicks],
        liquidity: &[Liquidity],
    ) -> Result<Snapshot> {
        let policy = self.risk.as_ref().ok_or(ControlError::Unqualified)?;
        let collateral = self.collateral.as_ref().ok_or(ControlError::Unqualified)?;
        let d = ledger
            .diagnostics(marks)
            .map_err(|_| ControlError::Unqualified)?;
        let atom = |n| QuoteAtoms::new(self.config.quote, n);
        let mut flags = Flags {
            insolvent: d.shortfall.atoms() > 0,
            ..Flags::default()
        };
        let mut rows = vec![];
        let mut gross = vec![0; self.config.markets.len()];
        let mut pending_deficit = 0;
        for id in &self.config.customers {
            let b = ledger
                .book(Owner::Customer(*id))
                .map_err(|_| ControlError::Invalid)?;
            let equity = self.book_equity(b, marks)?;
            let (mut initial, mut maintenance, mut outcome, mut costs) = (0, 0, 0, 0);
            for (j, (m, pos)) in self.config.markets.iter().zip(b.positions()).enumerate() {
                let mark = *marks
                    .iter()
                    .find(|p| p.unit() == m.unit())
                    .ok_or(ControlError::Unqualified)?;
                let r = policy
                    .markets
                    .iter()
                    .find(|r| r.market == m.unit())
                    .ok_or(ControlError::Unqualified)?;
                let (buys, sells, cost) = self.pending(active, Some(*id), *m, mark)?;
                let q = i128::from(pos.quantity().lots());
                i64::try_from(add(q, buys)?).map_err(|_| ControlError::Capacity)?;
                i64::try_from(sub(q, sells)?).map_err(|_| ControlError::Capacity)?;
                let worst = abs(add(q, buys)?)?.max(abs(sub(q, sells)?)?);
                let notional = value(*m, abs(q)?, mark)?;
                let bound = value(*m, worst, mark)?;
                gross[j] = add(gross[j], bound)?;
                initial = add(initial, ratio(notional, SCALE, self.leverage(*id, r))?)?;
                maintenance = add(
                    maintenance,
                    ratio(notional, u64::from(r.maintenance_bps), SCALE)?,
                )?;
                outcome = add(outcome, ratio(bound, SCALE, self.leverage(*id, r))?)?;
                costs = add(costs, cost)?;
            }
            outcome = add(outcome, costs)?;
            let resource = Resource::Customer(*id);
            let other = sub(active.held(resource)?, active.order_hold(resource)?)?;
            let requirement = outcome.max(add(initial, active.order_hold(resource)?)?);
            let free = sub(sub(equity, other)?, requirement)?;
            flags.private_initial |= free < 0;
            flags.private_maintenance |= equity < maintenance;
            let lower = sub(equity, costs)?;
            pending_deficit = add(
                pending_deficit,
                sub(sub(0, lower)?.max(0), sub(0, equity)?.max(0))?.max(0),
            )?;
            rows.push(Customer {
                account: *id,
                equity: atom(equity),
                initial: atom(initial),
                maintenance: atom(maintenance),
                outcome_requirement: atom(outcome),
                other_held: atom(other),
                free: atom(free),
            });
        }
        let native_equity = self.book_equity(ledger.venue(), marks)?;
        let (
            mut native_initial,
            mut native_maintenance,
            mut current_native_initial,
            mut native_cost,
            mut house_initial,
        ) = (0, 0, 0, 0, 0);
        let house = ledger
            .book(Owner::House)
            .map_err(|_| ControlError::Invalid)?;
        let mut concentration = vec![];
        for (j, (m, pos)) in self
            .config
            .markets
            .iter()
            .zip(ledger.venue().positions())
            .enumerate()
        {
            let mark = *marks
                .iter()
                .find(|p| p.unit() == m.unit())
                .ok_or(ControlError::Unqualified)?;
            let r = policy
                .markets
                .iter()
                .find(|r| r.market == m.unit())
                .ok_or(ControlError::Unqualified)?;
            let im = collateral
                .policy
                .markets
                .iter()
                .find(|r| r.market == m.unit())
                .ok_or(ControlError::Unqualified)?
                .native_bps;
            let (buys, sells, cost) = self.pending(active, None, *m, mark)?;
            let q = i128::from(pos.quantity().lots());
            i64::try_from(add(q, buys)?).map_err(|_| ControlError::Capacity)?;
            i64::try_from(sub(q, sells)?).map_err(|_| ControlError::Capacity)?;
            let bound = value(*m, abs(add(q, buys)?)?.max(abs(sub(q, sells)?)?), mark)?;
            native_initial = add(native_initial, ratio(bound, u64::from(im), SCALE)?)?;
            current_native_initial = add(
                current_native_initial,
                ratio(value(*m, abs(q)?, mark)?, u64::from(im), SCALE)?,
            )?;
            native_maintenance = add(
                native_maintenance,
                ratio(bound, u64::from(r.native_maintenance_bps), SCALE)?,
            )?;
            house_initial = add(
                house_initial,
                ratio(
                    value(
                        *m,
                        abs(i128::from(house.positions()[j].quantity().lots()))?,
                        mark,
                    )?,
                    SCALE,
                    r.maximum_leverage,
                )?,
            )?;
            native_cost = add(native_cost, cost)?;
            flags.concentration |= gross[j] > r.gross_limit.atoms() || bound > r.net_limit.atoms();
            concentration.push(Concentration {
                market: m.unit(),
                gross: atom(gross[j]),
                native: atom(bound),
            });
        }
        let resource = Resource::Location(Location::Venue);
        let other_native = sub(active.held(resource)?, active.order_hold(resource)?)?;
        let native_requirement = add(native_initial, native_cost)?
            .max(add(current_native_initial, active.order_hold(resource)?)?);
        flags.native_initial = native_equity < add(native_requirement, other_native)?;
        flags.native_maintenance = native_equity < add(native_maintenance, native_cost)?;
        let house_free = sub(self.book_equity(house, marks)?, house_initial)?;
        let backing = sub(
            sub(
                d.backing_margin.atoms(),
                ledger
                    .in_transit()
                    .map_err(|_| ControlError::Capacity)?
                    .atoms(),
            )?,
            ledger.venue().funding().atoms().max(0),
        )?;
        let designated = if ledger.protection().active {
            ledger
                .protection()
                .reserve
                .checked_sub(
                    ledger
                        .protection()
                        .committed()
                        .map_err(|_| ControlError::Capacity)?,
                )
                .map_err(|_| ControlError::Capacity)?
                .atoms()
        } else {
            house_free
        };
        let free_capital = sub(
            sub(house_free.min(backing).min(designated), pending_deficit)?,
            active.held(Resource::House)?,
        )?;
        flags.capital = free_capital < policy.buffer.atoms();
        let vault_free = sub(
            ledger.vault().atoms(),
            active.held(Resource::Location(Location::Vault))?,
        )?;
        let venue_access = sub(native_equity, native_requirement)?
            .min(ledger.venue().cash().atoms())
            .max(0);
        let broker_held = active.held(Resource::Location(Location::Broker))?;
        let mut location_free = [
            vault_free,
            sub(venue_access, other_native)?,
            sub(ledger.broker().atoms(), broker_held)?,
        ];
        for l in liquidity {
            let (index, available, held) = match l.location {
                Location::Vault => (
                    0,
                    ledger.vault().atoms(),
                    active.held(Resource::Location(Location::Vault))?,
                ),
                Location::Venue => (1, venue_access, other_native),
                Location::Broker => (2, ledger.broker().atoms(), broker_held),
            };
            let accessible = mul_div(
                available.max(0),
                u64::from(l.accessible_bps),
                SCALE,
                Rounding::Floor,
            )
            .map_err(|_| ControlError::Capacity)?;
            location_free[index] = sub(sub(accessible, held)?, l.due.atoms())?;
        }
        flags.illiquid = location_free.iter().any(|v| *v < 0);
        Ok(Snapshot {
            flags,
            customers: rows,
            concentration,
            native_equity: atom(native_equity),
            native_requirement: atom(native_requirement),
            free_capital: atom(free_capital),
            vault_free: atom(location_free[0]),
            venue_free: atom(location_free[1]),
            broker_free: atom(location_free[2]),
        })
    }
    /// Whether joined risk policy is installed. This is not a qualification result;
    /// admission still evaluates the complete proposed state at its current time.
    pub fn has_joined_risk(&self) -> bool {
        self.risk.is_some()
    }
    /// Recompute current/pending margin and every configured stress prefix under
    /// this exact journal state. Errors mean unqualified, never an empty safe report.
    pub fn risk_report(&self) -> Result<Report> {
        let policy = self.risk.as_ref().ok_or(ControlError::Unqualified)?;
        self.validate_risk(policy)?;
        let c = self.collateral.as_ref().ok_or(ControlError::Unqualified)?;
        if self.raw_unresolved > 0
            || self.unexplained_order_fault()
            || self.funds.iter().any(|o| o.faulted)
        {
            return Err(ControlError::Unqualified);
        }
        self.ledger
            .qualified_diagnostics(&c.marks, self.now, c.policy.evidence)
            .map_err(|_| ControlError::Unqualified)?;
        let active = ActiveRisk::new(self);
        // Bound index construction (log2(MAX_ITEMS) < 32), then the actual
        // active scans, including reservations, selections and market lookups.
        // Historical order/hold counts occur only in the one-time index cost.
        let preparation = (self.orders.len() + self.holds.len()).saturating_mul(32);
        let reservations: usize = active.holds.iter().map(|h| h.reservations.len()).sum();
        let work = (self.config.customers.len() + 3)
            .saturating_mul(self.config.markets.len())
            .saturating_mul(
                active
                    .orders
                    .len()
                    .saturating_mul(self.closes.len() + self.restorations.len() + 1)
                    + self.ledger.reductions().len().saturating_mul(97)
                    + 4 * reservations
                    + 2 * self.selections.len()
                    + 4 * self.config.markets.len()
                    + 1,
            )
            .saturating_mul(
                1 + policy
                    .paths
                    .iter()
                    .map(|p| p.steps.iter().map(|s| s.events.len() + 1).sum::<usize>())
                    .sum::<usize>(),
            )
            .saturating_add(preparation);
        if work > 1_000_000 {
            return Err(ControlError::Unqualified);
        }
        let scheduler_work = policy
            .paths
            .iter()
            .flat_map(|p| &p.steps)
            .flat_map(|s| &s.events)
            .fold(0usize, |n, e| {
                n.saturating_add(match &e.change {
                    Change::Restoration(
                        cinder_kernel::ledger::restoration::RestorationChange::Declare { .. },
                    ) => 2_000_000,
                    Change::Restoration(
                        cinder_kernel::ledger::restoration::RestorationChange::Execution { .. },
                    ) => 131_072,
                    _ => 0,
                })
            });
        if work.saturating_add(scheduler_work) > 8_000_000 {
            return Err(ControlError::Unqualified);
        }
        let current = self.risk_snapshot(
            &active,
            &self.ledger,
            &c.marks.iter().map(|m| m.price).collect::<Vec<_>>(),
            &[],
        )?;
        let mut minimum = current.free_capital.atoms();
        let f = &current.flags;
        let mut admissible = !self.frozen
            && !self.close_contained()
            && !self.restorations.iter().any(|r| r.contained)
            && self.protection_ready().is_ok()
            && !f.insolvent
            && !f.illiquid
            && !f.private_initial
            && !f.private_maintenance
            && !f.native_initial
            && !f.native_maintenance
            && !f.capital
            && !f.concentration;
        let mut paths = vec![];
        for path in &policy.paths {
            let mut ledger = self.ledger.clone();
            let mut prefixes = vec![current.clone()];
            for step in &path.steps {
                let at = self
                    .now
                    .checked_add(step.after_ms)
                    .ok_or(ControlError::Invalid)?;
                // A grouped step cannot hide distress between its events. Price
                // shock first, then every actual transition gets a separate prefix.
                let mut snapshots =
                    vec![self.risk_snapshot(&active, &ledger, &step.marks, &step.liquidity)?];
                for e in &step.events {
                    let result = ledger
                        .ingest(e, at)
                        .map_err(|_| ControlError::Unqualified)?;
                    if result.disposition != Disposition::Applied
                        || result.state.issues().iter().any(|i| i.open)
                    {
                        return Err(ControlError::Unqualified);
                    }
                    ledger = result.state;
                    snapshots.push(self.risk_snapshot(
                        &active,
                        &ledger,
                        &step.marks,
                        &step.liquidity,
                    )?);
                }
                for snapshot in snapshots {
                    minimum = minimum.min(snapshot.free_capital.atoms());
                    let f = &snapshot.flags;
                    admissible &= !f.insolvent
                        && !f.illiquid
                        && !f.native_maintenance
                        && !f.capital
                        && !f.concentration;
                    prefixes.push(snapshot);
                }
            }
            paths.push(Trace {
                path: path.id,
                prefixes,
            });
        }
        let target = add(
            policy.buffer.atoms(),
            sub(current.free_capital.atoms(), minimum)?.max(0),
        )?;
        Ok(Report {
            current,
            minimum_free: QuoteAtoms::new(self.config.quote, minimum),
            target: QuoteAtoms::new(self.config.quote, target),
            paths,
            admissible,
        })
    }
    pub(crate) fn risk_gate(&self) -> Result<()> {
        if self.close_contained() || self.restorations.iter().any(|r| r.contained) {
            return Err(ControlError::Unqualified);
        }
        self.protection_ready()?;
        if self.risk_report()?.admissible {
            Ok(())
        } else {
            Err(ControlError::Capacity)
        }
    }
    /// Recheck the final control cut, including policy changes later in the same
    /// proposal. Cleanup/configuration alone may record a restricted state.
    pub(crate) fn risk_controls(&self, controls: &[Control]) -> Result<()> {
        // Depth qualification constrains order admission, not unrelated free
        // collateral payouts. Recheck even if policy changes follow acceptance.
        if self.liquidation.is_some() {
            for c in controls {
                if let Control::Order(crate::orders::Action::Accept { intent, .. }) = c {
                    self.close_limit(intent.quantity.unit())?;
                }
            }
        }
        if self.risk.is_none() {
            return Ok(());
        }
        let admission =
            controls.iter().any(|c| match c {
                Control::Collateral(_)
                | Control::Risk(Action::Install { .. })
                | Control::Release(_) => false,
                Control::Protection(crate::protection::Action::Apply { decision, .. }) => {
                    matches!(
                        decision.change,
                        cinder_kernel::ledger::protection::ProtectionChange::Designate { delta }
                            if delta.atoms() < 0
                    )
                }
                Control::Protection(_) => false,
                Control::Liquidation(_) => false,
                Control::Restoration(a) => matches!(a, crate::restoration::Action::Prepare { .. }),
                Control::Order(orders) => !matches!(
                    orders,
                    crate::orders::Action::AdvanceAuthority { .. }
                        | crate::orders::Action::PrepareCancel { .. }
                        | crate::orders::Action::Release { .. }
                ),
                Control::Funds(f) => matches!(
                    f,
                    crate::funds::Action::Accept { .. } | crate::funds::Action::Prepare { .. }
                ),
                Control::Expose(k) => self.attempts.iter().find(|a| a.key == *k).is_none_or(|a| {
                    !matches!(a.kind, AttemptKind::Cancel | AttemptKind::Emergency)
                }),
                _ => true,
            });
        if admission {
            self.risk_gate()?;
        }
        let emergency = controls.iter().any(|c| match c {
            Control::Liquidation(crate::liquidation::Action::Prepare { .. }) => true,
            Control::Expose(k) => self
                .attempts
                .iter()
                .any(|a| a.key == *k && a.kind == AttemptKind::Emergency),
            _ => false,
        });
        if emergency {
            // Recheck the final joined state/policy. Ordinary mutations in the
            // same proposal still must pass the ordinary gate above.
            self.emergency_gate()?;
        }
        Ok(())
    }
}

pub(crate) fn encode_selection(w: &mut Writer, s: &Selection) {
    w.item(&s.request);
    w.item(&QuantityLots::new(s.market, 0));
    w.u64(s.leverage);
    w.raw(&s.policy.get().to_be_bytes());
    w.u64(s.authority_epoch);
    w.u64(s.expires_at);
}
fn decode_selection(r: &mut Reader<'_>) -> std::result::Result<Selection, Error> {
    let request = r.item()?;
    let q: QuantityLots = r.item()?;
    if q.lots() != 0 {
        return Err(Error::Codec);
    }
    Ok(Selection {
        request,
        market: q.unit(),
        leverage: r.u64()?,
        policy: PolicyVersion::new(u32::from_be_bytes(r.array()?)).map_err(|_| Error::Codec)?,
        authority_epoch: r.u64()?,
        expires_at: r.u64()?,
    })
}
pub(crate) fn encode_policy(w: &mut Writer, p: &Policy) {
    w.raw(&p.revision.get().to_be_bytes());
    w.item(&p.buffer);
    w.u64(p.horizon_ms);
    w.u64(p.valid_until);
    w.count(p.markets.len());
    for m in &p.markets {
        w.item(&QuantityLots::new(m.market, 0));
        w.u64(m.maximum_leverage);
        w.raw(&m.maintenance_bps.to_be_bytes());
        w.raw(&m.native_maintenance_bps.to_be_bytes());
        w.item(&m.gross_limit);
        w.item(&m.net_limit);
    }
    w.count(p.paths.len());
    for p in &p.paths {
        w.raw(&p.id);
        w.count(p.steps.len());
        for s in &p.steps {
            w.u64(s.after_ms);
            w.count(s.marks.len());
            for m in &s.marks {
                w.item(m)
            }
            w.count(s.events.len());
            for e in &s.events {
                match wire::encode_event(e) {
                    Ok(bytes) => w.blob(&bytes),
                    Err(_) => w.invalid(),
                }
            }
            w.count(s.liquidity.len());
            for l in &s.liquidity {
                wire::location(w, l.location);
                w.raw(&l.accessible_bps.to_be_bytes());
                w.item(&l.due);
            }
        }
    }
}
fn decode_policy(r: &mut Reader<'_>) -> std::result::Result<Policy, Error> {
    let revision = PolicyVersion::new(u32::from_be_bytes(r.array()?)).map_err(|_| Error::Codec)?;
    let buffer = r.item()?;
    let horizon_ms = r.u64()?;
    let valid_until = r.u64()?;
    let mut markets = vec![];
    for _ in 0..r.count()? {
        let q: QuantityLots = r.item()?;
        if q.lots() != 0 {
            return Err(Error::Codec);
        }
        markets.push(MarketRule {
            market: q.unit(),
            maximum_leverage: r.u64()?,
            maintenance_bps: u32::from_be_bytes(r.array()?),
            native_maintenance_bps: u32::from_be_bytes(r.array()?),
            gross_limit: r.item()?,
            net_limit: r.item()?,
        });
    }
    let mut paths = vec![];
    for _ in 0..r.count()? {
        let id = r.array()?;
        let mut steps = vec![];
        for _ in 0..r.count()? {
            let after_ms = r.u64()?;
            let mut marks = vec![];
            for _ in 0..r.count()? {
                marks.push(r.item()?);
            }
            let mut events = vec![];
            for _ in 0..r.count()? {
                events.push(wire::decode_event(r.blob()?)?);
            }
            let mut liquidity = vec![];
            for _ in 0..r.count()? {
                liquidity.push(Liquidity {
                    location: wire::read_location(r)?,
                    accessible_bps: u32::from_be_bytes(r.array()?),
                    due: r.item()?,
                });
            }
            steps.push(Step {
                after_ms,
                marks,
                events,
                liquidity,
            });
        }
        paths.push(Path { id, steps });
    }
    Ok(Policy {
        revision,
        markets,
        buffer,
        horizon_ms,
        valid_until,
        paths,
    })
}
pub(crate) fn encode_action(w: &mut Writer, a: &Action) {
    match a {
        Action::Install {
            expected_version,
            policy,
        } => {
            w.byte(0);
            w.u64(*expected_version);
            encode_policy(w, policy);
        }
        Action::Select {
            selection,
            approval,
        } => {
            w.byte(1);
            encode_selection(w, selection);
            w.raw(&approval.account.bytes());
            w.raw(&approval.intent_hash);
            w.u64(approval.authority_epoch);
        }
    }
}
pub(crate) fn decode_action(r: &mut Reader<'_>) -> std::result::Result<Action, Error> {
    Ok(match r.byte()? {
        0 => Action::Install {
            expected_version: r.u64()?,
            policy: Box::new(decode_policy(r)?),
        },
        1 => Action::Select {
            selection: Box::new(decode_selection(r)?),
            approval: Approval {
                account: AccountId::new(r.array()?).map_err(|_| Error::Codec)?,
                intent_hash: r.array()?,
                authority_epoch: r.u64()?,
            },
        },
        _ => return Err(Error::Codec),
    })
}
