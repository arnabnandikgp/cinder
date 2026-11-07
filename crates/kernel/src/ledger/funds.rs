//! Partial movement accounting in the same ledger. Source qualification, signing,
//! admission and native finality are controller/adapter duties, not algebra claims.
use super::*;

/// Intended or actually evidenced endpoint in this quote pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    /// Another Cinder custody location; no new customer credit.
    Location(Location),
    /// Exact recipient key in the configured network, not a display alias.
    Recipient([u8; 32]),
}

/// Immutable admitted movement. Authorizing it posts no cash or customer claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mandate {
    /// Unique dispatched attempt and private operation identity.
    pub attempt: AttemptKey,
    /// Physical debit location.
    pub source: Location,
    /// Internal transfer or customer payout beneficiary.
    pub destination: Destination,
    /// Maximum authorized recipient amount, excluding rail fees.
    pub net: QuoteAtoms,
    /// Maximum authorized fee; actual overruns belong to house, not unrelated users.
    pub maximum_fee: QuoteAtoms,
    /// House or this operation's customer; suspense cannot authorize a fee.
    pub fee_payer: Owner,
}

/// Actual financial stage. Partial outcomes remain separate economic events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leg {
    /// Source debit creates transit, not a completed payment.
    Debit,
    /// Gross settlement including its fee, with evidenced actual destination.
    Arrive(Destination),
    /// Gross return to source, less its actual fee.
    Return,
    /// Qualified unrecoverable transit loss, not an unexplained balancing plug.
    Impair,
}

/// Primitive actions consumed by the existing economic identity/replay machinery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FundsChange {
    /// Register already admitted exact semantics under its Attempt record key.
    Authorize(Mandate),
    /// Qualified actual receipt under an Economic record key, even after freezes.
    Observe {
        /// Previously registered immutable movement.
        attempt: AttemptKey,
        /// Actual stage, independent of the other legs' delivery order.
        leg: Leg,
        /// Positive gross atoms, including any fee charged within this stage.
        amount: QuoteAtoms,
        /// Nonnegative fee, at most gross amount; zero for debit/impairment.
        fee: QuoteAtoms,
    },
}

/// Financial movement totals. No stored external equity or independent user book.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movement {
    /// Original immutable authorization.
    pub mandate: Mandate,
    /// Gross evidenced source debits.
    pub debit: QuoteAtoms,
    /// Gross amounts evidenced as settled, returned or impaired.
    pub settled: QuoteAtoms,
    /// Gross destination arrivals.
    pub arrived: QuoteAtoms,
    /// Gross source returns.
    pub returned: QuoteAtoms,
    /// Evidenced impairment, charged to house.
    pub impaired: QuoteAtoms,
    /// Actual fees, not a projected fee cap.
    pub fees: QuoteAtoms,
    /// Authorized fees already charged to customer, bounded by the mandate.
    pub customer_fees: QuoteAtoms,
    /// Beneficiary settlement applied against the intended customer's claim.
    pub paid: QuoteAtoms,
    /// Net external payments including wrong-recipient or excess payments.
    pub sent: QuoteAtoms,
    /// Permanent route/bounds violation; later receipts do not hide it.
    pub faulted: bool,
    /// Applied economic legs, retained for exact terminal-history qualification.
    pub receipts: Vec<EventKey>,
}
impl Movement {
    /// Positive outstanding transfer receivable; not spendable cash.
    pub fn transit(&self) -> Result<QuoteAtoms, Error> {
        Ok(QuoteAtoms::new(
            self.debit.unit(),
            self.debit.checked_sub(self.settled)?.atoms().max(0),
        ))
    }
    /// Contra-balance for an observed settlement before its debit; not real debt.
    pub fn unpaired(&self) -> Result<QuoteAtoms, Error> {
        Ok(QuoteAtoms::new(
            self.debit.unit(),
            self.settled.checked_sub(self.debit)?.atoms().max(0),
        ))
    }
    /// Maximum source capability not yet consumed. Terminal release is separate.
    pub fn source_remaining(&self) -> Result<QuoteAtoms, Error> {
        let cap = self.mandate.net.checked_add(self.mandate.maximum_fee)?;
        Ok(QuoteAtoms::new(
            cap.unit(),
            cap.checked_sub(self.debit)?.atoms().max(0),
        ))
    }
}

impl Ledger {
    /// All partial movement state; read-only and included in the same asset bridge.
    pub fn movements(&self) -> &[Movement] {
        &self.movements
    }

    /// Bounds/route faults or settlements missing their debit prohibit dependent
    /// risk. Ordinary matched transit is illiquid, not inherently conflicting.
    pub fn unresolved_funds(&self) -> usize {
        self.movements
            .iter()
            .filter(|m| m.faulted || m.settled.atoms() > m.debit.atoms())
            .count()
    }

    /// Confirmed normal beneficiary payments only, for later recovery paid counters.
    /// Rail fees and house overpayment are deliberately not beneficiary payments.
    pub fn paid(&self, customer: AccountId) -> Result<QuoteAtoms, LedgerError> {
        self.book(Owner::Customer(customer))?;
        self.movements
            .iter()
            .filter(|m| m.mandate.attempt.request.account == customer)
            .try_fold(QuoteAtoms::new(self.config.quote, 0), |a, m| {
                Ok(a.checked_add(m.paid)?)
            })
    }

    /// Total unmatched observed settlement. Subtract once from assets until the
    /// debit arrives; do not call it customer/third-party capital or a liability.
    pub fn unpaired(&self) -> Result<QuoteAtoms, LedgerError> {
        self.movements
            .iter()
            .try_fold(QuoteAtoms::new(self.config.quote, 0), |a, m| {
                Ok(a.checked_add(m.unpaired()?)?)
            })
    }

    pub(super) fn apply_funds(
        &mut self,
        key: &RecordKey,
        change: &FundsChange,
    ) -> Result<(), LedgerError> {
        match change {
            FundsChange::Authorize(m) => {
                if key != &RecordKey::Attempt(m.attempt)
                    || m.attempt.request.domain != self.config.domain
                    || self
                        .movements
                        .iter()
                        .any(|old| old.mandate.attempt == m.attempt)
                    || m.net.unit() != self.config.quote
                    || m.maximum_fee.unit() != self.config.quote
                    || m.net.atoms() <= 0
                    || m.maximum_fee.atoms() < 0
                    || m.destination == Destination::Location(m.source)
                    || m.destination == Destination::Recipient([0; 32])
                    || !matches!(m.fee_payer, Owner::House | Owner::Customer(_))
                    || matches!(m.fee_payer, Owner::Customer(id) if id != m.attempt.request.account)
                    || matches!(m.destination, Destination::Location(_))
                        && m.fee_payer != Owner::House
                {
                    return Err(LedgerError::Configuration);
                }
                self.book(Owner::Customer(m.attempt.request.account))?;
                m.net.checked_add(m.maximum_fee)?;
                let zero = QuoteAtoms::new(self.config.quote, 0);
                self.movements.push(Movement {
                    mandate: m.clone(),
                    debit: zero,
                    settled: zero,
                    arrived: zero,
                    returned: zero,
                    impaired: zero,
                    fees: zero,
                    customer_fees: zero,
                    paid: zero,
                    sent: zero,
                    faulted: false,
                    receipts: Vec::new(),
                });
            }
            FundsChange::Observe {
                attempt,
                leg,
                amount,
                fee,
            } => {
                let index = self
                    .movements
                    .iter()
                    .position(|m| m.mandate.attempt == *attempt)
                    .ok_or(LedgerError::UnknownIdentity)?;
                let mut m = self.movements[index].clone();
                amount.unit().require(self.config.quote)?;
                fee.unit().require(self.config.quote)?;
                if amount.atoms() <= 0
                    || fee.atoms() < 0
                    || fee.atoms() > amount.atoms()
                    || matches!(leg, Leg::Debit | Leg::Impair) && fee.atoms() != 0
                {
                    return Err(Error::InvalidSign.into());
                }
                let source = match leg {
                    Leg::Arrive(Destination::Location(l)) => *l,
                    _ => m.mandate.source,
                };
                let economic = self.source(key, source)?;
                if *leg == Leg::Debit {
                    self.move_cash(source, amount.checked_neg()?)?;
                    m.debit = m.debit.checked_add(*amount)?;
                } else {
                    m.settled = m.settled.checked_add(*amount)?;
                    m.fees = m.fees.checked_add(*fee)?;
                    let intended = !matches!(leg, Leg::Arrive(destination) if *destination != m.mandate.destination);
                    if !intended {
                        m.faulted = true;
                    }
                    let customer_fee =
                        if intended && matches!(m.mandate.fee_payer, Owner::Customer(_)) {
                            fee.atoms().min(
                                m.mandate
                                    .maximum_fee
                                    .checked_sub(m.customer_fees)?
                                    .atoms()
                                    .max(0),
                            )
                        } else {
                            0
                        };
                    if customer_fee > 0 {
                        let charged = QuoteAtoms::new(self.config.quote, customer_fee);
                        let book = self.book_mut(m.mandate.fee_payer)?;
                        book.cash = book.cash.checked_sub(charged)?;
                        m.customer_fees = m
                            .customer_fees
                            .checked_add(QuoteAtoms::new(self.config.quote, customer_fee))?;
                    }
                    let house_fee =
                        fee.checked_sub(QuoteAtoms::new(self.config.quote, customer_fee))?;
                    self.house.cash = self.house.cash.checked_sub(house_fee)?;
                    let net = amount.checked_sub(*fee)?;
                    match leg {
                        Leg::Arrive(destination) => {
                            m.arrived = m.arrived.checked_add(*amount)?;
                            match destination {
                                Destination::Location(location) => {
                                    self.move_cash(*location, net)?
                                }
                                Destination::Recipient(_) => {
                                    m.sent = m.sent.checked_add(net)?;
                                    let authorized = if intended {
                                        m.mandate
                                            .net
                                            .checked_sub(m.paid)?
                                            .atoms()
                                            .max(0)
                                            .min(net.atoms())
                                    } else {
                                        0
                                    };
                                    let paid = QuoteAtoms::new(self.config.quote, authorized);
                                    let book =
                                        self.book_mut(Owner::Customer(attempt.request.account))?;
                                    book.cash = book.cash.checked_sub(paid)?;
                                    m.paid = m.paid.checked_add(paid)?;
                                    self.house.cash =
                                        self.house.cash.checked_sub(net.checked_sub(paid)?)?;
                                }
                            }
                        }
                        Leg::Return => {
                            m.returned = m.returned.checked_add(*amount)?;
                            self.move_cash(m.mandate.source, net)?;
                        }
                        Leg::Impair => {
                            m.impaired = m.impaired.checked_add(*amount)?;
                            self.house.cash = self.house.cash.checked_sub(*amount)?;
                        }
                        Leg::Debit => unreachable!(),
                    }
                }
                let cap = m.mandate.net.checked_add(m.mandate.maximum_fee)?;
                m.faulted |= m.debit.atoms() > cap.atoms()
                    || m.settled.atoms() > cap.atoms()
                    || m.fees.atoms() > m.mandate.maximum_fee.atoms()
                    || m.sent.atoms() > m.mandate.net.atoms();
                m.receipts.push(economic);
                self.movements[index] = m;
            }
        }
        Ok(())
    }
}
