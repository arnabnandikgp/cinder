//! Single native fills split between a still-valid private close and house
//! exceptions. Admission/authority are controller obligations, not kernel inputs.
use super::*;
use crate::math::{Rounding, mul_div};

/// Immutable route; no speculative house order is authorized by this value alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// Exact durably admitted attempt.
    pub attempt: AttemptKey,
    /// Customer close versus exceptional house unwind.
    pub house: bool,
    /// Originally authorized signed quantity.
    pub quantity: QuantityLots,
    /// All actual executed magnitude, including policy-violating executions.
    pub received: u64,
    /// Actual private allocation magnitude.
    pub customer_filled: u64,
    /// Actual additional house quantity from customer-close excess.
    pub excess: u64,
    /// Cumulative positive realized house costs, never reduced by later gains.
    pub spent: QuoteAtoms,
}
/// Normalized private route/actual economics, not a dispatch API.
#[derive(Debug, Clone, PartialEq, Eq)]
// Keep the normalized inline event shape used by the existing Change envelope;
// its economics variant is already this size. Avoid a new heap allocation per fill.
#[allow(clippy::large_enum_variant)]
pub enum CloseChange {
    /// Exact attempt key supplies the customer/incident identity.
    Bind {
        /// True only for a funded exceptional house unwind.
        house: bool,
        /// Bounded originally admitted quantity.
        quantity: QuantityLots,
    },
    /// Actual source execution posts once even if it exceeds all prior limits.
    Execution {
        /// Original recorded route.
        attempt: AttemptKey,
        /// Actual signed size, not the requested amount.
        quantity: QuantityLots,
        /// Exact native price.
        price: PriceTicks,
        /// Actual signed fee/rebate.
        fee: QuoteAtoms,
        /// Qualified native PnL convention, if supplied.
        pnl: Option<NativePnl>,
        /// False for a contradicted terminal/policy route: retain provisional
        /// house economics, not an invented customer entitlement.
        customer_allowed: bool,
    },
}
impl Ledger {
    /// Durable close routes and actual allocations; no additional asset book.
    pub fn closes(&self) -> &[Binding] {
        &self.closes
    }
    pub(super) fn apply_close(
        &mut self,
        key: &RecordKey,
        change: &CloseChange,
    ) -> Result<(), LedgerError> {
        match change {
            CloseChange::Bind { house, quantity } => {
                let RecordKey::Attempt(attempt) = key else {
                    return Err(LedgerError::Attribution);
                };
                self.book(Owner::Customer(attempt.request.account))?;
                self.market_index(quantity.unit())?;
                if quantity.lots() == 0
                    || self.closes.iter().any(|b| b.attempt == *attempt)
                    || self.bindings.iter().any(|b| b.attempt == *attempt)
                {
                    return Err(LedgerError::Attribution);
                }
                self.closes.push(Binding {
                    attempt: *attempt,
                    house: *house,
                    quantity: *quantity,
                    received: 0,
                    customer_filled: 0,
                    excess: 0,
                    spent: QuoteAtoms::new(self.config.quote, 0),
                });
            }
            CloseChange::Execution {
                attempt,
                quantity,
                price,
                fee,
                pnl,
                customer_allowed,
            } => {
                let source = self.source(key, Location::Venue)?;
                let index = self.market_index(quantity.unit())?;
                fee.unit().require(self.config.quote)?;
                let route = self
                    .closes
                    .iter()
                    .position(|b| b.attempt == *attempt)
                    .ok_or(LedgerError::UnknownIdentity)?;
                let binding = self.closes[route].clone();
                binding.quantity.unit().require(quantity.unit())?;
                if quantity.lots() == 0 {
                    return Err(Error::InvalidSign.into());
                }
                let owner = Owner::Customer(attempt.request.account);
                let current = self.book(owner)?.positions[index].quantity().lots();
                let total = quantity.lots().unsigned_abs();
                let customer = if !binding.house
                    && *customer_allowed
                    && quantity.lots().signum() == binding.quantity.lots().signum()
                    && current.signum() != quantity.lots().signum()
                {
                    total.min(current.unsigned_abs()).min(
                        binding
                            .quantity
                            .lots()
                            .unsigned_abs()
                            .saturating_sub(binding.received),
                    )
                } else {
                    0
                };
                let market = self.config.markets[index];
                let native_value = market.notional(*quantity, *price)?;
                let (private_value, house_value) =
                    BasisAtoms::new(quantity.unit(), native_value.atoms())
                        .split(customer, total)?;
                let customer_fee = QuoteAtoms::new(
                    self.config.quote,
                    mul_div(fee.atoms(), customer, total, Rounding::TowardZero)?,
                );
                let house_fee = fee.checked_sub(customer_fee)?;
                let signed = |n: u64| -> Result<QuantityLots, Error> {
                    Ok(QuantityLots::new(
                        quantity.unit(),
                        i64::try_from(i128::from(n) * i128::from(quantity.lots().signum()))
                            .map_err(|_| Error::Overflow)?,
                    ))
                };
                let house_before = self.house.cash;
                if customer > 0 {
                    let b = self.book_mut(owner)?;
                    let fill = b.positions[index].fill_value(
                        market,
                        signed(customer)?,
                        QuoteAtoms::new(market.unit().quote, private_value.atoms()),
                    )?;
                    b.positions[index] = fill.position;
                    b.cash = b
                        .cash
                        .checked_add(fill.realized)?
                        .checked_sub(customer_fee)?;
                }
                if total > customer {
                    let fill = self.house.positions[index].fill_value(
                        market,
                        signed(total - customer)?,
                        QuoteAtoms::new(market.unit().quote, house_value.atoms()),
                    )?;
                    self.house.positions[index] = fill.position;
                    self.house.cash = self.house.cash.checked_add(fill.realized)?;
                }
                self.house.cash = self.house.cash.checked_sub(house_fee)?;
                let expected = self.venue.execute(index, market, *quantity, *price)?;
                self.venue.cash = self.venue.cash.checked_sub(*fee)?;
                let reported = pnl.map(|p| p.gross(*fee)).transpose()?.unwrap_or(expected);
                let difference = reported.checked_sub(expected)?;
                self.venue.cash = self.venue.cash.checked_add(difference)?;
                self.suspense.cash = self.suspense.cash.checked_add(difference)?;
                self.set_issue(
                    key.clone(),
                    IssueKind::NativePnl,
                    difference.atoms() != 0,
                    Some(difference),
                );
                self.execution_reports.push(ExecutionReport {
                    key: source,
                    expected,
                    reported,
                    fee: *fee,
                });
                let house_delta = self.house.cash.checked_sub(house_before)?;
                if self.protection.active {
                    self.protection.reserve = self.protection.reserve.checked_add(house_delta)?;
                }
                let b = &mut self.closes[route];
                b.received = b.received.checked_add(total).ok_or(Error::Overflow)?;
                b.customer_filled = b
                    .customer_filled
                    .checked_add(customer)
                    .ok_or(Error::Overflow)?;
                if !b.house {
                    b.excess = b
                        .excess
                        .checked_add(total - customer)
                        .ok_or(Error::Overflow)?;
                }
                b.spent = b.spent.checked_add(QuoteAtoms::new(
                    self.config.quote,
                    house_delta
                        .atoms()
                        .checked_neg()
                        .ok_or(Error::Overflow)?
                        .max(0),
                ))?;
            }
        }
        Ok(())
    }
}
