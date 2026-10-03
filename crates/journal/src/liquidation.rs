//! Policy-bounded private liquidation and exceptional house unwind. These use
//! the existing order lifecycle, shared reservations and authoritative ledger.
use crate::{
    Error,
    model::*,
    orders::{Intent, Order, TimeInForce},
    wire::{Reader, Writer},
};
use cinder_kernel::{
    amounts::*,
    identity::*,
    ledger::{close::CloseChange, evidence::Disposition, *},
    math::{Rounding, mul_div},
};
use sha2::{Digest, Sha256};

/// Qualified depth/price/time envelope, not a calibrated live market guarantee.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limit {
    /// Exact market/precision.
    pub market: MarketUnit,
    /// Maximum lots in one bounded attempt, also capped by current position.
    pub maximum_lots: u64,
    /// Inclusive native price floor.
    pub minimum: PriceTicks,
    /// Inclusive native price ceiling.
    pub maximum: PriceTicks,
    /// Nonnegative quote atoms per executed lot.
    pub fee_per_lot: QuoteAtoms,
    /// Additional house stress support per lot beyond price-range/fee/margin bound.
    pub additional_per_lot: QuoteAtoms,
}
/// Trusted administrator installation; availability/depth qualification is external.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    /// Immutable revision; changed limits require an increase.
    pub revision: PolicyVersion,
    /// Verified operator authority epoch.
    pub authority_epoch: u64,
    /// Last time these depth/price/capital bounds may authorize dispatch.
    pub valid_until: u64,
    /// At most one limit per configured market; missing markets are disabled.
    pub limits: Vec<Limit>,
}
/// Distinct source of authority and financial owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Operator-assisted wind-down of an existing customer position, including a
    /// healthy one, only under the explicit durable recovery permission.
    RecoveryClose,
    /// Previously customer-authorized private reduce-only intent.
    PrivateClose,
    /// Only a maintenance-breached customer; not a healthy-customer pool bail-out.
    Liquidation,
    /// Only reduction of already-existing exceptional house inventory.
    HouseUnwind,
}
/// An exact operator instruction bound to current policy and financial state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    /// Customer for liquidation; original incident identity for house unwind.
    pub attempt: AttemptKey,
    /// No operator path for ordinary private orders.
    pub kind: Kind,
    /// Market to reduce. Size is deterministically min(position, depth limit).
    pub market: MarketUnit,
    /// Exact financial state used to derive size and maintenance breach.
    pub expected_version: u64,
    /// Immutable qualified limit revision.
    pub policy: PolicyVersion,
    /// Authenticated operator epoch.
    pub authority_epoch: u64,
    /// Last dispatch time; not evidence that an exposed order ceased to exist.
    pub expires_at: u64,
}
impl Proposal {
    /// Trusted authentication-port binding, not a signature verifier.
    pub fn digest(&self) -> Result<[u8; 32], Error> {
        let mut w = Writer::new(60);
        encode_proposal(&mut w, self);
        Ok(Sha256::digest(w.finish()?).into())
    }
}
/// Controls never insert a hypothetical execution into the real ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Authenticated deployment policy installation.
    Install {
        /// Exact financial cut.
        expected_version: u64,
        /// Explicit qualified bounds, never supplied by a public customer.
        policy: Box<Policy>,
    },
    /// Prepare one bounded IOC and reserve all resources before exposure.
    Prepare {
        /// Exact operator instruction.
        proposal: Box<Proposal>,
        /// Authentication-port result under the configured operator epoch.
        authenticated_digest: [u8; 32],
    },
}
/// Metadata only; positions/cash remain exclusively in Ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Close {
    /// Existing order request.
    pub request: RequestKey,
    /// Qualified authority/purpose.
    pub kind: Kind,
    /// Policy at admission; epoch changes fence future dispatch, not escaped fills.
    pub policy: PolicyVersion,
    /// Operator epoch, independent of customer grant.
    pub authority_epoch: u64,
    /// Original funded house limit; actual costs may exceed it after a shock.
    pub budget: QuoteAtoms,
    /// Known exceptional execution; ordinary new risk remains contained.
    pub contained: bool,
}
impl State {
    pub(crate) fn unexplained_order_fault(&self) -> bool {
        self.orders.iter().any(|o| {
            o.faulted
                && !self
                    .closes
                    .iter()
                    .any(|c| c.request == o.intent.request && c.contained)
                && !self
                    .restorations
                    .iter()
                    .any(|r| r.request == o.intent.request && r.contained)
        })
    }
    pub(crate) fn order_authority(&self, request: RequestKey, epoch: u64) -> bool {
        if self.restorations.iter().any(|r| r.request == request) {
            return self
                .liquidation
                .as_ref()
                .is_some_and(|p| p.authority_epoch == epoch);
        }
        if self
            .closes
            .iter()
            .any(|c| c.request == request && c.kind != Kind::PrivateClose)
        {
            self.liquidation
                .as_ref()
                .is_some_and(|p| p.authority_epoch == epoch)
        } else {
            self.authority(request.account) == Some(epoch)
        }
    }
    /// Read-only bounded close metadata.
    pub fn closes(&self) -> &[Close] {
        &self.closes
    }
    pub(crate) fn close_contained(&self) -> bool {
        self.closes.iter().any(|c| c.contained)
    }
    pub(crate) fn close_limit(&self, market: MarketUnit) -> Result<&Limit, ControlError> {
        let p = self.liquidation.as_ref().ok_or(ControlError::Unqualified)?;
        if self.now >= p.valid_until {
            return Err(ControlError::Expired);
        }
        p.limits
            .iter()
            .find(|l| l.market == market)
            .ok_or(ControlError::Unqualified)
    }
    fn close_budget(&self, intent: &Intent) -> Result<QuoteAtoms, ControlError> {
        let limit = self.close_limit(intent.quantity.unit())?;
        if intent.quantity.lots().unsigned_abs() > limit.maximum_lots
            || intent.minimum.ticks() < limit.minimum.ticks()
            || intent.maximum.ticks() > limit.maximum.ticks()
            || intent.maximum_fee_per_lot.atoms() > limit.fee_per_lot.atoms()
            || intent.expires_at
                > self
                    .liquidation
                    .as_ref()
                    .ok_or(ControlError::Unqualified)?
                    .valid_until
        {
            return Err(ControlError::Invalid);
        }
        let rule = self
            .risk
            .as_ref()
            .and_then(|p| p.markets.iter().find(|m| m.market == limit.market))
            .ok_or(ControlError::Unqualified)?;
        let market = self
            .config
            .markets
            .iter()
            .find(|m| m.unit() == limit.market)
            .ok_or(ControlError::Invalid)?;
        let (n, d) = market.conversion();
        let q = i128::from(intent.quantity.lots().unsigned_abs());
        let value = |ticks: u64| {
            q.checked_mul(i128::from(ticks))
                .and_then(|v| mul_div(v, n, d, Rounding::Ceil).ok())
                .ok_or(ControlError::Capacity)
        };
        let margin = mul_div(
            value(limit.maximum.ticks())?,
            10000,
            rule.maximum_leverage,
            Rounding::Ceil,
        )
        .map_err(|_| ControlError::Capacity)?;
        let cost = limit
            .fee_per_lot
            .atoms()
            .checked_mul(2)
            .and_then(|v| v.checked_add(limit.additional_per_lot.atoms()))
            .and_then(|v| v.checked_mul(q))
            .ok_or(ControlError::Capacity)?;
        Ok(QuoteAtoms::new(
            self.config.quote,
            margin
                .checked_add(value(limit.maximum.ticks() - limit.minimum.ticks())?)
                .and_then(|v| v.checked_add(cost))
                .ok_or(ControlError::Capacity)?,
        ))
    }
    /// Minimum funded support for an already-bounded private close; useful for
    /// constructing its ordinary P07 reservation without guessing an amount.
    pub fn close_reserve(&self, intent: &Intent) -> Result<QuoteAtoms, ControlError> {
        self.close_budget(intent)
    }
    pub(crate) fn attach_private_close(&mut self, request: RequestKey) -> Result<(), ControlError> {
        let order = self
            .orders
            .iter()
            .find(|o| o.intent.request == request)
            .ok_or(ControlError::Invalid)?;
        if !order.intent.reduce_only || self.liquidation.is_none() {
            return Ok(());
        }
        let budget = self.close_budget(&order.intent)?;
        if self.closes.iter().any(|c| c.request == request) {
            return Err(ControlError::Invalid);
        }
        let hold = self
            .holds
            .iter_mut()
            .find(|h| h.request == request && h.active)
            .ok_or(ControlError::Invalid)?;
        if let Some(r) = hold
            .reservations
            .iter_mut()
            .find(|r| r.resource == Resource::House)
        {
            if r.amount.atoms() < budget.atoms() {
                r.amount = budget
            }
        } else {
            hold.reservations.push(Reservation {
                resource: Resource::House,
                amount: budget,
            })
        }
        let p = self.liquidation.as_ref().ok_or(ControlError::Unqualified)?;
        self.closes.push(Close {
            request,
            kind: Kind::PrivateClose,
            policy: p.revision,
            authority_epoch: p.authority_epoch,
            budget,
            contained: false,
        });
        self.all_capacity()
    }
    pub(crate) fn liquidation_action(&mut self, action: &Action) -> Result<(), ControlError> {
        match action {
            Action::Install {
                expected_version,
                policy: p,
            } => {
                if *expected_version != self.ledger.version()
                    || p.authority_epoch == 0
                    || p.valid_until <= self.now
                    || p.limits.is_empty()
                    || p.limits.len() > self.config.markets.len()
                    || self.liquidation.as_ref().is_some_and(|old| {
                        p.revision < old.revision
                            || p.authority_epoch < old.authority_epoch
                            || p.revision == old.revision && p.as_ref() != old
                    })
                    || p.limits.iter().enumerate().any(|(i, l)| {
                        l.maximum_lots == 0
                            || l.maximum_lots > i64::MAX as u64
                            || l.minimum.unit() != l.market
                            || l.maximum.unit() != l.market
                            || l.minimum.ticks() > l.maximum.ticks()
                            || l.fee_per_lot.unit() != self.config.quote
                            || l.fee_per_lot.atoms() < 0
                            || l.additional_per_lot.unit() != self.config.quote
                            || l.additional_per_lot.atoms() < 0
                            || !self.config.markets.iter().any(|m| m.unit() == l.market)
                            || p.limits[..i].iter().any(|old| old.market == l.market)
                    })
                {
                    return Err(ControlError::Invalid);
                }
                self.liquidation = Some(p.as_ref().clone());
            }
            Action::Prepare {
                proposal: p,
                authenticated_digest,
            } => {
                self.request(p.attempt.request)?;
                let policy = self.liquidation.as_ref().ok_or(ControlError::Unqualified)?;
                if p.kind == Kind::PrivateClose
                    || p.expected_version != self.ledger.version()
                    || p.policy != policy.revision
                    || p.authority_epoch != policy.authority_epoch
                    || p.digest().map_err(|_| ControlError::Invalid)? != *authenticated_digest
                    || p.expires_at > policy.valid_until
                    || p.expires_at <= self.now
                    || self.orders.len() >= crate::wire::MAX_ITEMS
                    || self.holds.len() >= crate::wire::MAX_ITEMS
                    || self.holds.iter().any(|h| h.request == p.attempt.request)
                    || self
                        .selections
                        .iter()
                        .any(|s| s.request == p.attempt.request)
                {
                    return Err(ControlError::Invalid);
                }
                let limit = self.close_limit(p.market)?.clone();
                if p.kind == Kind::RecoveryClose && self.recovery_epoch() != Some(p.authority_epoch)
                {
                    return Err(ControlError::Unqualified);
                }
                let report = self.risk_report()?;
                if p.kind == Kind::Liquidation
                    && report
                        .current
                        .customers
                        .iter()
                        .find(|c| c.account == p.attempt.request.account)
                        .is_none_or(|c| c.equity.atoms() >= c.maintenance.atoms())
                {
                    return Err(ControlError::Invalid);
                }
                let owner = if p.kind == Kind::HouseUnwind {
                    Owner::House
                } else {
                    Owner::Customer(p.attempt.request.account)
                };
                if p.kind == Kind::HouseUnwind
                    && !self
                        .ledger
                        .closes()
                        .iter()
                        .any(|b| b.excess > 0 && b.quantity.unit() == p.market)
                    && !self
                        .ledger
                        .reductions()
                        .iter()
                        .any(|r| r.excess > 0 && r.quantity.unit() == p.market)
                {
                    return Err(ControlError::Unqualified);
                }
                if self.orders.iter().any(|o| {
                    !o.complete()
                        && o.intent.quantity.unit() == p.market
                        && (p.kind == Kind::HouseUnwind
                            && self.closes.iter().any(|c| {
                                c.request == o.intent.request && c.kind == Kind::HouseUnwind
                            })
                            || matches!(p.kind, Kind::Liquidation | Kind::RecoveryClose)
                                && o.intent.request.account == p.attempt.request.account)
                }) {
                    return Err(ControlError::Unqualified);
                }
                let current = self
                    .ledger
                    .book(owner)
                    .map_err(|_| ControlError::Invalid)?
                    .positions()
                    .iter()
                    .find(|x| x.quantity().unit() == p.market)
                    .ok_or(ControlError::Invalid)?
                    .quantity()
                    .lots();
                if current == 0 {
                    return Err(ControlError::Invalid);
                }
                let lots = i64::try_from(current.unsigned_abs().min(limit.maximum_lots))
                    .map_err(|_| ControlError::Capacity)?;
                let intent = Intent {
                    request: p.attempt.request,
                    time_in_force: TimeInForce::ImmediateOrCancel,
                    quantity: QuantityLots::new(p.market, if current > 0 { -lots } else { lots }),
                    minimum: limit.minimum,
                    maximum: limit.maximum,
                    maximum_fee_per_lot: limit.fee_per_lot,
                    reduce_only: true,
                    policy: self.config.policy,
                    authority_epoch: p.authority_epoch,
                    expires_at: p.expires_at,
                };
                let budget = self.close_budget(&intent)?;
                self.holds.push(Hold {
                    request: intent.request,
                    reservations: vec![Reservation {
                        resource: Resource::House,
                        amount: budget,
                    }],
                    active: true,
                });
                self.closes.push(Close {
                    request: intent.request,
                    kind: p.kind,
                    policy: p.policy,
                    authority_epoch: p.authority_epoch,
                    budget,
                    contained: false,
                });
                self.orders.push(Order {
                    abandoned: false,
                    intent: intent.clone(),
                    attempt: Some(p.attempt),
                    filled: QuantityLots::new(p.market, 0),
                    executions: vec![],
                    acknowledged: false,
                    cancel_acknowledged: false,
                    unknown: false,
                    terminal: None,
                    faulted: false,
                    bound_violated: false,
                });
                let mut w = Writer::new(61);
                w.item(&p.attempt);
                crate::orders::encode_intent(&mut w, &intent);
                w.byte(u8::from(p.kind == Kind::HouseUnwind));
                self.prepare_attempt(
                    p.attempt,
                    PrivateBytes::new(w.finish().map_err(|_| ControlError::Invalid)?)
                        .map_err(|_| ControlError::Invalid)?,
                    p.authority_epoch,
                    p.expires_at,
                    AttemptKind::Emergency,
                )?;
                let e = Event {
                    key: RecordKey::Attempt(p.attempt),
                    policy: self.config.policy,
                    change: Change::Close(CloseChange::Bind {
                        house: p.kind == Kind::HouseUnwind,
                        quantity: intent.quantity,
                    }),
                };
                let next = self
                    .ledger
                    .ingest(&e, self.now)
                    .map_err(|_| ControlError::Invalid)?;
                if next.disposition != Disposition::Applied {
                    return Err(ControlError::Invalid);
                }
                self.ledger = next.state;
                self.emergency_gate()?;
            }
        }
        Ok(())
    }
    pub(crate) fn emergency_gate(&self) -> Result<(), ControlError> {
        let report = self.risk_report()?;
        let f = &report.current.flags;
        // A customer MM breach permits its bounded exit, not new unrelated risk.
        // Insolvency/unsupported native margin or unfunded house support still
        // requires stronger recovery authority; this is not guaranteed liquidity.
        if f.insolvent
            || f.illiquid
            || f.native_initial
            || f.native_maintenance
            || f.capital
            || f.concentration
            || report.current.free_capital.atoms() < report.target.atoms()
            || report.paths.iter().flat_map(|p| &p.prefixes).any(|s| {
                s.flags.insolvent
                    || s.flags.illiquid
                    || s.flags.native_maintenance
                    || s.flags.capital
                    || s.flags.concentration
            })
        {
            return Err(ControlError::Capacity);
        }
        Ok(())
    }
    pub(crate) fn close_exposure(&self, a: &Attempt) -> Result<(), ControlError> {
        let Some(c) = self.closes.iter().find(|c| c.request == a.key.request) else {
            return Ok(());
        };
        let policy = self.liquidation.as_ref().ok_or(ControlError::Unqualified)?;
        if a.kind != AttemptKind::Cancel
            && (c.policy != policy.revision
                || c.authority_epoch != policy.authority_epoch
                || policy.valid_until <= self.now
                || c.contained)
        {
            return Err(ControlError::Unqualified);
        }
        if a.kind == AttemptKind::Emergency {
            if c.kind == Kind::RecoveryClose && self.recovery_epoch() != Some(c.authority_epoch) {
                return Err(ControlError::Unqualified);
            }
            let order = self
                .orders
                .iter()
                .find(|o| o.intent.request == c.request)
                .ok_or(ControlError::Invalid)?;
            let owner = if c.kind == Kind::HouseUnwind {
                Owner::House
            } else {
                Owner::Customer(c.request.account)
            };
            let current = self
                .ledger
                .book(owner)
                .map_err(|_| ControlError::Invalid)?
                .positions()
                .iter()
                .find(|p| p.quantity().unit() == order.intent.quantity.unit())
                .ok_or(ControlError::Invalid)?
                .quantity()
                .lots();
            if current.signum() == order.intent.quantity.lots().signum()
                || current.unsigned_abs() < order.intent.quantity.lots().unsigned_abs()
            {
                return Err(ControlError::Invalid);
            }
            if c.kind == Kind::Liquidation
                && self
                    .risk_report()?
                    .current
                    .customers
                    .iter()
                    .find(|r| r.account == c.request.account)
                    .is_none_or(|r| r.equity.atoms() >= r.maintenance.atoms())
            {
                return Err(ControlError::Invalid);
            }
            self.emergency_gate()?;
        }
        Ok(())
    }
}

fn kind(w: &mut Writer, k: Kind) {
    w.byte(match k {
        Kind::PrivateClose => 0,
        Kind::Liquidation => 1,
        Kind::HouseUnwind => 2,
        Kind::RecoveryClose => 3,
    })
}
fn read_kind(r: &mut Reader<'_>) -> Result<Kind, Error> {
    match r.byte()? {
        0 => Ok(Kind::PrivateClose),
        1 => Ok(Kind::Liquidation),
        2 => Ok(Kind::HouseUnwind),
        3 => Ok(Kind::RecoveryClose),
        _ => Err(Error::Codec),
    }
}
fn encode_proposal(w: &mut Writer, p: &Proposal) {
    w.item(&p.attempt);
    kind(w, p.kind);
    w.item(&QuantityLots::new(p.market, 0));
    w.u64(p.expected_version);
    w.raw(&p.policy.get().to_be_bytes());
    w.u64(p.authority_epoch);
    w.u64(p.expires_at)
}
fn read_market(r: &mut Reader<'_>) -> Result<MarketUnit, Error> {
    let q: QuantityLots = r.item()?;
    if q.lots() != 0 {
        Err(Error::Codec)
    } else {
        Ok(q.unit())
    }
}
fn read_policy_version(r: &mut Reader<'_>) -> Result<PolicyVersion, Error> {
    PolicyVersion::new(u32::from_be_bytes(r.array()?)).map_err(|_| Error::Codec)
}
pub(crate) fn encode_policy(w: &mut Writer, p: &Policy) {
    w.raw(&p.revision.get().to_be_bytes());
    w.u64(p.authority_epoch);
    w.u64(p.valid_until);
    w.count(p.limits.len());
    for l in &p.limits {
        w.item(&QuantityLots::new(l.market, 0));
        w.u64(l.maximum_lots);
        w.item(&l.minimum);
        w.item(&l.maximum);
        w.item(&l.fee_per_lot);
        w.item(&l.additional_per_lot)
    }
}
fn decode_policy(r: &mut Reader<'_>) -> Result<Policy, Error> {
    let revision = read_policy_version(r)?;
    let authority_epoch = r.u64()?;
    let valid_until = r.u64()?;
    let mut limits = vec![];
    for _ in 0..r.count()? {
        limits.push(Limit {
            market: read_market(r)?,
            maximum_lots: r.u64()?,
            minimum: r.item()?,
            maximum: r.item()?,
            fee_per_lot: r.item()?,
            additional_per_lot: r.item()?,
        })
    }
    Ok(Policy {
        revision,
        authority_epoch,
        valid_until,
        limits,
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
            encode_policy(w, policy)
        }
        Action::Prepare {
            proposal,
            authenticated_digest,
        } => {
            w.byte(1);
            encode_proposal(w, proposal);
            w.raw(authenticated_digest)
        }
    }
}
pub(crate) fn decode_action(r: &mut Reader<'_>) -> Result<Action, Error> {
    match r.byte()? {
        0 => Ok(Action::Install {
            expected_version: r.u64()?,
            policy: Box::new(decode_policy(r)?),
        }),
        1 => Ok(Action::Prepare {
            proposal: Box::new(Proposal {
                attempt: r.item()?,
                kind: read_kind(r)?,
                market: read_market(r)?,
                expected_version: r.u64()?,
                policy: read_policy_version(r)?,
                authority_epoch: r.u64()?,
                expires_at: r.u64()?,
            }),
            authenticated_digest: r.array()?,
        }),
        _ => Err(Error::Codec),
    }
}
pub(crate) fn encode_close(w: &mut Writer, c: &Close) {
    w.item(&c.request);
    kind(w, c.kind);
    w.raw(&c.policy.get().to_be_bytes());
    w.u64(c.authority_epoch);
    w.item(&c.budget);
    w.byte(u8::from(c.contained))
}
