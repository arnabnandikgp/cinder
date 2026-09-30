//! RF1 economics over the same books; RF2 membership/schedule never comes from
//! an operator-supplied allocation manifest. Native facts and declaration differ.
use super::*;
use crate::{
    math::{Rounding, mul_div},
    quota::{self, Plan, Weight},
};

/// One frozen private reduction, with cumulative rounding and a permanent void bit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Private customer.
    pub owner: AccountId,
    /// Original reduced magnitude.
    pub amount: u64,
    /// Removed signed entry basis, not replacement price.
    pub basis: BasisAtoms,
    /// Realized result on that original reduction, excluding fee.
    pub realized: QuoteAtoms,
    /// Original ADL fee allocated once.
    pub fee: QuoteAtoms,
    /// Actually restored private quantity, never a promised/unfilled quantity.
    pub restored: u64,
    /// A changed private intent voids all future awards, never reallocates them.
    pub void: bool,
}
/// One native forced event and its immutable private cut. Not a second asset book.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reduction {
    /// Native event identity, disjoint from liquidation and ordinary execution.
    pub incident: EventKey,
    /// Actual signed native reduction.
    pub quantity: QuantityLots,
    /// Native forced execution price.
    pub price: PriceTicks,
    /// Actual fee; unsupported rebates remain observed but not auto-declared.
    pub fee: QuoteAtoms,
    /// Source supplied the exact prior inventory cut and no mixed house exposure.
    pub qualified: bool,
    cut: Vec<(AccountId, Book)>,
    /// Deterministically compiled immutable schedule; absent before declaration.
    pub plan: Option<Plan>,
    /// Frozen economic allocation.
    pub rows: Vec<Row>,
    /// One admitted replacement attempt; no automatic retries.
    pub attempt: Option<AttemptKey>,
    /// Admitted prefix of the full frozen schedule.
    pub target: u64,
    /// Consumed eligible schedule slots, including voided customer slots.
    pub consumed: u64,
    /// All actual replacement execution magnitudes, even violations.
    pub received: u64,
    /// Cumulative positive house cash costs, never reset by a favorable fill.
    pub spent: QuoteAtoms,
    /// Additional actual house exposure from void/late/oversize fills.
    pub excess: u64,
}
/// Pure normalized facts and internal allocation controls. Authentication,
/// pre-exposure funding and source-time qualification are journal obligations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum RestorationChange {
    /// Source-qualified receipt before journal ownership classification. Direct
    /// kernel application is forbidden; it is not an eligibility assertion.
    Receipt {
        /// Original replacement attempt.
        attempt: AttemptKey,
        /// Actual signed execution.
        quantity: QuantityLots,
        /// Actual price.
        price: PriceTicks,
        /// Actual fee.
        fee: QuoteAtoms,
        /// Native report.
        pnl: Option<NativePnl>,
        /// Source execution time, not delayed receive time.
        executed_at: u64,
    },
    /// Always books the real native fill to suspense first, including stale cuts.
    Observe {
        /// Exact prior private inventory version qualified by the source adapter.
        cut: u64,
        /// Actual signed forced execution.
        quantity: QuantityLots,
        /// Actual price.
        price: PriceTicks,
        /// Actual signed fee.
        fee: QuoteAtoms,
        /// Optional qualified native report.
        pnl: Option<NativePnl>,
    },
    /// Request-keyed fixed proportional declaration; no allocation input.
    Declare {
        /// Previously observed native event.
        incident: EventKey,
    },
    /// Attempt-keyed route, only after funded journal admission.
    Bind {
        /// Declared forced event.
        incident: EventKey,
        /// Positive prefix bounded by the original reduction.
        target: u64,
    },
    /// Authenticated changed private intent (the key supplies its owner).
    Void {
        /// Affected exact market.
        market: MarketUnit,
    },
    /// Actual source execution. Already qualified economics do not vanish when
    /// a later capital shock would refuse a new order.
    Execution {
        /// Original durable replacement route.
        attempt: AttemptKey,
        /// Actual signed replacement size.
        quantity: QuantityLots,
        /// Actual price.
        price: PriceTicks,
        /// Actual fee, all charged to house for this restoration.
        fee: QuoteAtoms,
        /// Qualified native report.
        pnl: Option<NativePnl>,
        /// Source time/bounds/history eligibility, not current capital health.
        eligible: bool,
    },
}
fn piece(total: i128, done: u64, count: u64, size: u64) -> Result<i128, Error> {
    mul_div(
        total,
        done.checked_add(count).ok_or(Error::Overflow)?,
        size,
        Rounding::TowardZero,
    )?
    .checked_sub(mul_div(total, done, size, Rounding::TowardZero)?)
    .ok_or(Error::Overflow)
}
impl Ledger {
    /// Read-only incident accounting; no balance can be supplied by a caller.
    pub fn reductions(&self) -> &[Reduction] {
        &self.reductions
    }
    pub(super) fn void_restorations(&mut self, owner: AccountId, market: MarketUnit) {
        for r in &mut self.reductions {
            if r.quantity.unit() == market {
                for row in &mut r.rows {
                    if row.owner == owner {
                        row.void = true;
                    }
                }
            }
        }
    }
    pub(super) fn apply_restoration(
        &mut self,
        key: &RecordKey,
        change: &RestorationChange,
    ) -> Result<(), LedgerError> {
        match change {
            RestorationChange::Receipt { .. } => return Err(LedgerError::Attribution),
            RestorationChange::Observe {
                cut,
                quantity,
                price,
                fee,
                pnl,
            } => {
                let source = self.source(key, Location::Venue)?;
                let index = self.market_index(quantity.unit())?;
                let current = self.venue.positions[index].quantity().lots();
                let qualified = *cut == self.version
                    && quantity.lots() != 0
                    && current.signum() != quantity.lots().signum()
                    && quantity.lots().unsigned_abs() <= current.unsigned_abs()
                    && self.house.positions[index].quantity().lots() == 0
                    && self
                        .suspense
                        .positions
                        .iter()
                        .all(|p| p.quantity().lots() == 0)
                    && self.unresolved.is_empty()
                    && self.customers.iter().all(|(_, b)| b.funding.atoms() == 0)
                    && self.venue.funding.atoms() == 0
                    && fee.atoms() >= 0;
                let cut = self.customers.clone();
                self.apply_economics(
                    key,
                    &EconomicChange::Execution {
                        target: FillTarget::Unattributed,
                        quantity: *quantity,
                        price: *price,
                        fee: *fee,
                        pnl: *pnl,
                    },
                )?;
                self.reductions.push(Reduction {
                    incident: source,
                    quantity: *quantity,
                    price: *price,
                    fee: *fee,
                    qualified,
                    cut,
                    plan: None,
                    rows: Vec::new(),
                    attempt: None,
                    target: 0,
                    consumed: 0,
                    received: 0,
                    spent: QuoteAtoms::new(self.config.quote, 0),
                    excess: 0,
                });
            }
            RestorationChange::Declare { incident } => {
                if !matches!(key, RecordKey::Request(_)) {
                    return Err(LedgerError::Attribution);
                }
                let ri = self
                    .reductions
                    .iter()
                    .position(|r| r.incident == *incident)
                    .ok_or(LedgerError::UnknownIdentity)?;
                let r = self.reductions[ri].clone();
                if !r.qualified
                    || r.plan.is_some()
                    || r.cut != self.customers
                    || self.unresolved.len() != 1
                    || self.unresolved[0] != *incident
                    || self.issues().iter().any(|i| i.open)
                {
                    return Err(LedgerError::StaleCut);
                }
                let index = self.market_index(r.quantity.unit())?;
                let market = self.config.markets[index];
                let side = -r.quantity.lots().signum();
                let weights: Vec<Weight> = self
                    .customers
                    .iter()
                    .filter_map(|(id, b)| {
                        let q = b.positions[index].quantity().lots();
                        (q.signum() == side).then_some(Weight {
                            owner: *id,
                            amount: q.unsigned_abs(),
                        })
                    })
                    .collect();
                let total = r.quantity.lots().unsigned_abs();
                let allocation =
                    quota::apportion(&weights, total).map_err(|_| LedgerError::Restricted)?;
                let plan = Plan::compile(&allocation).map_err(|_| LedgerError::Restricted)?;
                let fees = quota::apportion(
                    &allocation,
                    u64::try_from(r.fee.atoms()).map_err(|_| Error::Overflow)?,
                )
                .map_err(|_| LedgerError::Restricted)?;
                let native_value = market.notional(r.quantity, r.price)?;
                let mut offset = 0;
                let mut rows = Vec::new();
                for row in allocation {
                    let q = QuantityLots::new(
                        market.unit(),
                        i64::try_from(i128::from(row.amount) * i128::from(-side))
                            .map_err(|_| Error::Overflow)?,
                    );
                    let value = QuoteAtoms::new(
                        self.config.quote,
                        piece(native_value.atoms(), offset, row.amount, total)?,
                    );
                    let fee = QuoteAtoms::new(
                        self.config.quote,
                        fees.iter()
                            .find(|f| f.owner == row.owner)
                            .map(|f| i128::from(f.amount))
                            .unwrap_or(0),
                    );
                    let b = self.book_mut(Owner::Customer(row.owner))?;
                    let before = b.positions[index].basis();
                    let fill = b.positions[index].fill_value(market, q, value)?;
                    b.positions[index] = fill.position;
                    b.cash = b.cash.checked_add(fill.realized)?.checked_sub(fee)?;
                    rows.push(Row {
                        owner: row.owner,
                        amount: row.amount,
                        basis: before.checked_sub(fill.position.basis())?,
                        realized: fill.realized,
                        fee,
                        restored: 0,
                        void: false,
                    });
                    offset += row.amount;
                }
                let reverse = QuantityLots::new(
                    market.unit(),
                    r.quantity.lots().checked_neg().ok_or(Error::Overflow)?,
                );
                self.suspense.execute(index, market, reverse, r.price)?;
                self.suspense.cash = self.suspense.cash.checked_add(r.fee)?;
                self.unresolved.retain(|k| k != incident);
                self.reductions[ri].rows = rows;
                self.reductions[ri].plan = Some(plan);
            }
            RestorationChange::Bind { incident, target } => {
                let RecordKey::Attempt(a) = key else {
                    return Err(LedgerError::Attribution);
                };
                let r = self
                    .reductions
                    .iter_mut()
                    .find(|r| r.incident == *incident)
                    .ok_or(LedgerError::UnknownIdentity)?;
                if r.attempt.is_some()
                    || r.plan.is_none()
                    || *target == 0
                    || *target > r.quantity.lots().unsigned_abs()
                    || r.rows.iter().all(|r| r.void)
                {
                    return Err(LedgerError::Attribution);
                }
                r.attempt = Some(*a);
                r.target = *target;
            }
            RestorationChange::Void { market } => {
                let RecordKey::Request(r) = key else {
                    return Err(LedgerError::Attribution);
                };
                self.market_index(*market)?;
                self.void_restorations(r.account, *market);
            }
            RestorationChange::Execution {
                attempt,
                quantity,
                price,
                fee,
                pnl,
                eligible,
            } => {
                let source = self.source(key, Location::Venue)?;
                let ri = self
                    .reductions
                    .iter()
                    .position(|r| r.attempt == Some(*attempt))
                    .ok_or(LedgerError::UnknownIdentity)?;
                let mut r = self.reductions[ri].clone();
                r.quantity.unit().require(quantity.unit())?;
                let index = self.market_index(quantity.unit())?;
                let market = self.config.markets[index];
                let side = -r.quantity.lots().signum();
                if quantity.lots() == 0 {
                    return Err(Error::InvalidSign.into());
                }
                let total = quantity.lots().unsigned_abs();
                let count = if *eligible && quantity.lots().signum() == side {
                    total.min(r.target - r.consumed)
                } else {
                    0
                };
                let plan = r.plan.as_ref().ok_or(LedgerError::Attribution)?;
                let before = plan
                    .prefix(r.consumed)
                    .map_err(|_| LedgerError::Restricted)?;
                let after = plan
                    .prefix(r.consumed + count)
                    .map_err(|_| LedgerError::Restricted)?;
                let mut restored = 0u64;
                let mut oldvalue = QuoteAtoms::new(self.config.quote, 0);
                let mut refund = oldvalue;
                for (i, row) in r.rows.iter_mut().enumerate() {
                    let n = after[i] - before[i];
                    if row.void || n == 0 {
                        continue;
                    }
                    let basis = BasisAtoms::new(
                        market.unit(),
                        piece(row.basis.atoms(), row.restored, n, row.amount)?,
                    );
                    let realized = QuoteAtoms::new(
                        self.config.quote,
                        piece(row.realized.atoms(), row.restored, n, row.amount)?,
                    );
                    let fees = QuoteAtoms::new(
                        self.config.quote,
                        piece(row.fee.atoms(), row.restored, n, row.amount)?,
                    );
                    let delta = QuantityLots::new(
                        market.unit(),
                        i64::try_from(i128::from(n) * i128::from(side))
                            .map_err(|_| Error::Overflow)?,
                    );
                    let b = self.book_mut(Owner::Customer(row.owner))?;
                    b.positions[index] = Position::new(
                        b.positions[index].quantity().checked_add(delta)?,
                        b.positions[index].basis().checked_add(basis)?,
                    )?;
                    b.cash = b.cash.checked_sub(realized)?.checked_add(fees)?;
                    oldvalue = oldvalue
                        .checked_add(QuoteAtoms::new(self.config.quote, basis.atoms()))?
                        .checked_add(realized)?;
                    refund = refund.checked_add(fees)?;
                    row.restored += n;
                    restored += n;
                }
                let value = market.notional(*quantity, *price)?;
                let (private_value, house_value) =
                    BasisAtoms::new(market.unit(), value.atoms()).split(restored, total)?;
                let house_before = self.house.cash;
                if total > restored {
                    let q = QuantityLots::new(
                        market.unit(),
                        i64::try_from(
                            i128::from(total - restored) * i128::from(quantity.lots().signum()),
                        )
                        .map_err(|_| Error::Overflow)?,
                    );
                    let fill = self.house.positions[index].fill_value(
                        market,
                        q,
                        QuoteAtoms::new(self.config.quote, house_value.atoms()),
                    )?;
                    self.house.positions[index] = fill.position;
                    self.house.cash = self.house.cash.checked_add(fill.realized)?;
                }
                self.house.cash = self
                    .house
                    .cash
                    .checked_sub(*fee)?
                    .checked_sub(QuoteAtoms::new(self.config.quote, private_value.atoms()))?
                    .checked_add(oldvalue)?
                    .checked_sub(refund)?;
                let expected = self.venue.execute(index, market, *quantity, *price)?;
                let reported = pnl.map(|p| p.gross(*fee)).transpose()?.unwrap_or(expected);
                let diff = reported.checked_sub(expected)?;
                self.venue.cash = self.venue.cash.checked_sub(*fee)?.checked_add(diff)?;
                self.suspense.cash = self.suspense.cash.checked_add(diff)?;
                self.set_issue(
                    key.clone(),
                    IssueKind::NativePnl,
                    diff.atoms() != 0,
                    Some(diff),
                );
                self.execution_reports.push(ExecutionReport {
                    key: source,
                    expected,
                    reported,
                    fee: *fee,
                });
                let delta = self.house.cash.checked_sub(house_before)?;
                if self.protection.active {
                    self.protection.reserve = self.protection.reserve.checked_add(delta)?;
                }
                r.spent = r.spent.checked_add(QuoteAtoms::new(
                    self.config.quote,
                    delta.atoms().checked_neg().ok_or(Error::Overflow)?.max(0),
                ))?;
                r.consumed += count;
                r.received = r.received.checked_add(total).ok_or(Error::Overflow)?;
                r.excess = r
                    .excess
                    .checked_add(total - restored)
                    .ok_or(Error::Overflow)?;
                self.reductions[ri] = r;
            }
        }
        Ok(())
    }
}
