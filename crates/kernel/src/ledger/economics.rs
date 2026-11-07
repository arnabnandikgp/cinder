//! Funding cut/recognition/settlement and execution-cost normalization.
//! All source qualification and policy values are explicit inputs, not venue defaults.

use super::*;
use crate::math::{Rounding, mul_div};

/// Native PnL convention, qualified per source revision. Fee is positive for a
/// charge and negative for a rebate; neither convention is guessed from its sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePnl {
    /// Realized PnL before the separately reported fee.
    Gross(QuoteAtoms),
    /// Realized PnL already reduced by the signed reported fee.
    NetOfFee(QuoteAtoms),
}
impl NativePnl {
    /// Recover gross realized PnL once. Does not determine private cost basis.
    pub fn gross(self, fee: QuoteAtoms) -> Result<QuoteAtoms, Error> {
        match self {
            Self::Gross(value) => {
                value.unit().require(fee.unit())?;
                Ok(value)
            }
            Self::NetOfFee(value) => value.checked_add(fee),
        }
    }
}

/// Qualified signed quote-atom payout per long lot, as a rational. A positive
/// value credits longs; shorts receive the opposite sign before rounding.
/// Rate/mark/unit normalization is an adapter obligation, not performed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FundingRate {
    /// Signed numerator in the configured quote atoms per lot.
    pub numerator: QuoteAtoms,
    /// Strictly positive rational denominator.
    pub denominator: u64,
    /// Explicit qualified rounding policy; no implicit venue default.
    pub rounding: Rounding,
}
impl FundingRate {
    fn payment(self, lots: i64) -> Result<QuoteAtoms, Error> {
        // Flip the rounding direction before negation for signed short exposure.
        let rounding = if lots < 0 {
            match self.rounding {
                Rounding::Floor => Rounding::Ceil,
                Rounding::Ceil => Rounding::Floor,
                other => other,
            }
        } else {
            self.rounding
        };
        let value = mul_div(
            self.numerator.atoms(),
            lots.unsigned_abs(),
            self.denominator,
            rounding,
        )?;
        let value = if lots < 0 {
            value.checked_neg().ok_or(Error::Overflow)?
        } else {
            value
        };
        Ok(QuoteAtoms::new(self.numerator.unit(), value))
    }
}

/// Pure normalized financial actions. Native execution/source finality and customer
/// authority are qualified outside the kernel; no variant dispatches an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EconomicChange {
    /// Actual fill plus signed native fee/rebate, charged to its execution owner.
    Execution {
        /// Pre-recorded allocation route.
        target: FillTarget,
        /// Executed, not requested, lots.
        quantity: QuantityLots,
        /// Exact execution price.
        price: PriceTicks,
        /// Actual fee (>0) or rebate (<0); never projected future savings.
        fee: QuoteAtoms,
        /// If the source reports PnL, its qualified convention must be explicit.
        pnl: Option<NativePnl>,
    },
    /// An authoritative correction of the PnL report, not another execution or fee.
    CorrectExecutionPnl {
        /// Original economic fill; correction has a distinct source identity.
        execution: EventKey,
        /// Revised native PnL using the original fee. Fee corrections are not inferred.
        pnl: NativePnl,
    },
    /// Explicitly authorized separate Cinder charge: customer → house, no venue debit.
    BrokerFee {
        /// Bound authenticated customer request; matching request key required.
        customer: AccountId,
        /// Positive policy-approved amount; fee schedule calibration remains G02.
        amount: QuoteAtoms,
    },
    /// Freeze the eligible private/native inventory at a qualified causal cut.
    FundingBoundary {
        /// Configured market.
        market: MarketUnit,
        /// Must exactly match this state, not an approximate timestamp.
        expected_version: u64,
    },
    /// Add missing qualified inputs to a frozen boundary. Known native funding is
    /// initially held in suspense if the gross private rate is unavailable.
    FundingInputs {
        /// Original boundary identity; it cannot be recaptured after position changes.
        boundary: EventKey,
        /// None means unknown, not zero; previously known values cannot be overwritten.
        rate: Option<FundingRate>,
        /// Actual native signed funding, including explicit qualified zero.
        native: Option<QuoteAtoms>,
    },
    /// Cash settlement of one boundary only. Never sweeps unrelated open accruals.
    FundingSettlement {
        /// Frozen boundary identity.
        boundary: EventKey,
        /// Actual signed cash payment; can arrive before private rate evidence.
        native: QuoteAtoms,
    },
    /// Qualified correction to native funding only, preserving original allocations.
    CorrectFundingNative {
        /// Boundary being corrected.
        boundary: EventKey,
        /// Revised native accrual/cash amount according to settlement state.
        native: QuoteAtoms,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FundingRecord {
    key: EventKey,
    market: MarketUnit,
    inventory: Vec<(Owner, i64)>,
    native_lots: i64,
    rate: Option<FundingRate>,
    native: Option<QuoteAtoms>,
    posted: Vec<(Owner, QuoteAtoms)>,
    settled: bool,
    rounding_residual: QuoteAtoms,
}

/// Private projection of one owner's frozen funding, never other inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FundingView {
    /// Internal provenance; never expose native identities through the API.
    pub boundary: EventKey,
    /// Configured market.
    pub market: MarketUnit,
    /// Actual frozen eligible lots.
    pub lots: i64,
    /// Recognized allocation, including qualified zero; None is unknown.
    pub payment: Option<QuoteAtoms>,
    /// Allocation settled to cash, not a second earning.
    pub settled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExecutionReport {
    pub(super) key: EventKey,
    pub(super) expected: QuoteAtoms,
    pub(super) reported: QuoteAtoms,
    pub(super) fee: QuoteAtoms,
}

impl Ledger {
    /// Owner-scoped immutable facts for the trusted private API.
    pub fn funding_views(&self, owner: Owner) -> Vec<FundingView> {
        self.funding_records
            .iter()
            .filter_map(|r| {
                let (_, lots) = r.inventory.iter().find(|(o, _)| *o == owner)?;
                Some(FundingView {
                    boundary: r.key.clone(),
                    market: r.market,
                    lots: *lots,
                    payment: r.posted.iter().find(|(o, _)| *o == owner).map(|(_, p)| *p),
                    settled: r.settled,
                })
            })
            .collect()
    }
    pub(super) fn apply_economics(
        &mut self,
        key: &RecordKey,
        change: &EconomicChange,
    ) -> Result<(), LedgerError> {
        if let EconomicChange::BrokerFee { customer, amount } = change {
            let RecordKey::Request(request) = key else {
                return Err(LedgerError::Attribution);
            };
            if request.account != *customer {
                return Err(LedgerError::Attribution);
            }
            amount.unit().require(self.config.quote)?;
            if amount.atoms() <= 0 {
                return Err(Error::InvalidSign.into());
            }
            let user = self.book_mut(Owner::Customer(*customer))?;
            user.cash = user.cash.checked_sub(*amount)?;
            self.house.cash = self.house.cash.checked_add(*amount)?;
            return Ok(());
        }
        let source = self.source(key, Location::Venue)?;
        match change {
            EconomicChange::Execution {
                target,
                quantity,
                price,
                fee,
                pnl,
            } => {
                fee.unit().require(self.config.quote)?;
                let (owner, expected) = self.execute_fill(key, *target, *quantity, *price)?;
                let book = self.book_mut(owner)?;
                book.cash = book.cash.checked_sub(*fee)?;
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
            }
            EconomicChange::CorrectExecutionPnl { execution, pnl } => {
                let index = self
                    .execution_reports
                    .iter()
                    .position(|r| r.key == *execution)
                    .ok_or(LedgerError::UnknownIdentity)?;
                let old = self.execution_reports[index].clone();
                let reported = pnl.gross(old.fee)?;
                let delta = reported.checked_sub(old.reported)?;
                self.venue.cash = self.venue.cash.checked_add(delta)?;
                self.suspense.cash = self.suspense.cash.checked_add(delta)?;
                self.execution_reports[index].reported = reported;
                let difference = reported.checked_sub(old.expected)?;
                self.set_issue(
                    RecordKey::Economic(execution.clone()),
                    IssueKind::NativePnl,
                    difference.atoms() != 0,
                    Some(difference),
                );
            }
            EconomicChange::FundingBoundary {
                market,
                expected_version,
            } => {
                if *expected_version != self.version {
                    return Err(LedgerError::StaleCut);
                }
                let index = self.market_index(*market)?;
                let mut inventory: Vec<_> = self
                    .customers
                    .iter()
                    .map(|(id, book)| {
                        (
                            Owner::Customer(*id),
                            book.positions[index].quantity().lots(),
                        )
                    })
                    .collect();
                inventory.push((Owner::House, self.house.positions[index].quantity().lots()));
                inventory.push((
                    Owner::Suspense,
                    self.suspense.positions[index].quantity().lots(),
                ));
                self.funding_records.push(FundingRecord {
                    key: source,
                    market: *market,
                    inventory,
                    native_lots: self.venue.positions[index].quantity().lots(),
                    rate: None,
                    native: None,
                    posted: Vec::new(),
                    settled: false,
                    rounding_residual: QuoteAtoms::new(self.config.quote, 0),
                });
                self.set_issue(key.clone(), IssueKind::FundingInputs, true, None);
            }
            EconomicChange::FundingInputs {
                boundary,
                rate,
                native,
            } => {
                let index = self.funding_index(boundary)?;
                let mut record = self.funding_records[index].clone();
                if rate.is_none() && native.is_none() {
                    return Err(LedgerError::Evidence);
                }
                if let Some(rate) = rate {
                    rate.numerator.unit().require(self.config.quote)?;
                    if rate.denominator == 0 {
                        return Err(Error::DivisionByZero.into());
                    }
                    if record.rate.is_some_and(|old| old != *rate) {
                        return Err(LedgerError::FundingState);
                    }
                    record.rate = Some(*rate);
                }
                if let Some(native) = native {
                    native.unit().require(self.config.quote)?;
                    if record.native.is_some_and(|old| old != *native) {
                        return Err(LedgerError::FundingState);
                    }
                    record.native = Some(*native);
                }
                self.update_funding(index, record)?;
            }
            EconomicChange::FundingSettlement { boundary, native } => {
                let index = self.funding_index(boundary)?;
                let mut record = self.funding_records[index].clone();
                if record.settled {
                    return Err(LedgerError::FundingState);
                }
                native.unit().require(self.config.quote)?;
                record.native = Some(*native);
                record.settled = true;
                self.update_funding(index, record)?;
            }
            EconomicChange::CorrectFundingNative { boundary, native } => {
                let index = self.funding_index(boundary)?;
                let mut record = self.funding_records[index].clone();
                if record.native.is_none() {
                    return Err(LedgerError::FundingState);
                }
                native.unit().require(self.config.quote)?;
                record.native = Some(*native);
                self.update_funding(index, record)?;
            }
            EconomicChange::BrokerFee { .. } => {
                unreachable!("handled before native source validation")
            }
        }
        Ok(())
    }
    fn funding_index(&self, key: &EventKey) -> Result<usize, LedgerError> {
        self.funding_records
            .iter()
            .position(|r| r.key == *key)
            .ok_or(LedgerError::UnknownIdentity)
    }
    fn post_funding(book: &mut Book, amount: QuoteAtoms, settled: bool) -> Result<(), LedgerError> {
        if settled {
            book.cash = book.cash.checked_add(amount)?;
        } else {
            book.funding = book.funding.checked_add(amount)?;
        }
        Ok(())
    }
    fn remove_funding(
        book: &mut Book,
        amount: QuoteAtoms,
        settled: bool,
    ) -> Result<(), LedgerError> {
        // Direct subtraction also supports i128::MIN without an impossible negation.
        if settled {
            book.cash = book.cash.checked_sub(amount)?;
        } else {
            book.funding = book.funding.checked_sub(amount)?;
        }
        Ok(())
    }
    fn update_funding(&mut self, index: usize, mut next: FundingRecord) -> Result<(), LedgerError> {
        let old = self.funding_records[index].clone();
        next.posted.clear();
        let zero = QuoteAtoms::new(self.config.quote, 0);
        let mut mismatch = None;
        if let Some(native) = next.native {
            if let Some(rate) = next.rate {
                let mut total = zero;
                for (owner, lots) in &next.inventory {
                    let value = rate.payment(*lots)?;
                    total = total.checked_add(value)?;
                    next.posted.push((*owner, value));
                }
                let expected = rate.payment(next.native_lots)?;
                // Only the mechanically derived per-owner-vs-net rounding difference
                // belongs to house. An unexplained actual difference goes to suspense.
                next.rounding_residual = expected.checked_sub(total)?;
                let house = next
                    .posted
                    .iter_mut()
                    .find(|(o, _)| *o == Owner::House)
                    .ok_or(LedgerError::Bridge)?;
                house.1 = house.1.checked_add(next.rounding_residual)?;
                let difference = native.checked_sub(expected)?;
                let suspense = next
                    .posted
                    .iter_mut()
                    .find(|(o, _)| *o == Owner::Suspense)
                    .ok_or(LedgerError::Bridge)?;
                suspense.1 = suspense.1.checked_add(difference)?;
                mismatch = Some(difference);
            } else {
                next.posted.push((Owner::Suspense, native));
            }
        }
        // Replace exactly this boundary's postings; other boundaries stay untouched.
        // Removing old entries and adding new entries happens inside one pure proposal.
        for (owner, value) in &old.posted {
            Self::remove_funding(self.book_mut(*owner)?, *value, old.settled)?;
        }
        if let Some(value) = old.native {
            Self::remove_funding(&mut self.venue, value, old.settled)?;
        }
        for (owner, value) in &next.posted {
            Self::post_funding(self.book_mut(*owner)?, *value, next.settled)?;
        }
        if let Some(value) = next.native {
            Self::post_funding(&mut self.venue, value, next.settled)?;
        }
        let key = RecordKey::Economic(next.key.clone());
        self.set_issue(
            key.clone(),
            IssueKind::FundingInputs,
            next.rate.is_none() || next.native.is_none(),
            None,
        );
        self.set_issue(
            key,
            IssueKind::FundingDifference,
            mismatch.is_some_and(|v| v.atoms() != 0),
            mismatch,
        );
        self.funding_records[index] = next;
        Ok(())
    }
    /// Auditable derived rounding residue for a qualified boundary, never an
    /// arbitrary native reconciliation difference assigned to the house.
    pub fn funding_rounding_residual(
        &self,
        boundary: &EventKey,
    ) -> Result<QuoteAtoms, LedgerError> {
        let record = &self.funding_records[self.funding_index(boundary)?];
        if record.rate.is_none() || record.native.is_none() {
            return Err(LedgerError::Evidence);
        }
        Ok(record.rounding_residual)
    }
}
