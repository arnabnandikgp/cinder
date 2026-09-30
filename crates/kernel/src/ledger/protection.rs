//! House-funded protection metadata and postings in the existing ledger.
//! Recognition/authority/coverage evidence is a trusted controller obligation.
use super::*;

/// Existing loss versus a new separately owed Cinder obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Confirmed flat customer debt; recognition creates no new positive entitlement.
    Deficit,
    /// Owed operational remediation, credited once even when capital is insufficient.
    Remediation,
}
/// One immutable loss episode. Collection rights are metadata with zero asset value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    /// Exact private owner/domain/episode identity.
    pub id: RequestKey,
    /// Economic category, not a catch-all venue guarantee.
    pub kind: Kind,
    /// Retained cause/evidence commitment; controller must qualify its substance.
    pub cause: [u8; 32],
    /// Original recognized obligation/loss; never overwritten after depletion.
    pub amount: QuoteAtoms,
    /// Remaining earmarked deficit coverage, not new assets.
    pub committed: QuoteAtoms,
    /// Existing debt transferred to house once.
    pub absorbed: QuoteAtoms,
    /// Actual receipt applied to the still-negative, unabsorbed balance.
    pub recovered: QuoteAtoms,
    /// Separately owed remediation offsetting this unabsorbed debt.
    pub offset: QuoteAtoms,
    /// Actual later receipt replenishing the capital that absorbed this debt.
    pub replenished: QuoteAtoms,
}
impl Claim {
    /// Remaining deficit; remediation already exists in ordinary customer cash.
    pub fn remaining(&self) -> Result<QuoteAtoms, Error> {
        if self.kind == Kind::Remediation {
            return Ok(QuoteAtoms::new(self.amount.unit(), 0));
        }
        self.amount
            .checked_sub(self.absorbed)?
            .checked_sub(self.recovered)?
            .checked_sub(self.offset)
    }
    /// Historical absorbed collection right, never included in backing.
    pub fn collection(&self) -> Result<QuoteAtoms, Error> {
        self.absorbed.checked_sub(self.replenished)
    }
}
/// Explicit allocation supplied under a separately authenticated priority decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allocation {
    /// Previously recognized and committed claim.
    pub claim: RequestKey,
    /// Exact positive absorption amount.
    pub amount: QuoteAtoms,
}
/// Pure already-authorized financial transitions, not a public control API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtectionChange {
    /// Alter nominal house designation only; never mint cash or customer collateral.
    Designate {
        /// Signed change. A reduction cannot outrun recognized obligations.
        delta: QuoteAtoms,
    },
    /// Retain a qualified obligation even if it exceeds available protection.
    Recognize {
        /// Request record key supplies owner/claim ID.
        kind: Kind,
        /// Retained qualified evidence commitment, unique per owner/category.
        cause: [u8; 32],
        /// Original exact loss/obligation.
        amount: QuoteAtoms,
    },
    /// Earmark existing designation; does not change backing or settle the claim.
    Commit {
        /// Original recognized deficit.
        claim: RequestKey,
        /// Positive additional commitment.
        amount: QuoteAtoms,
    },
    /// Reclassify committed deficits as house loss, atomically across the approved set.
    Absorb {
        /// No implicit first-arrival or operator-selected priority inside the kernel.
        allocations: Vec<Allocation>,
    },
}
/// Nominal designation/claims, not a second ledger or separately additive fund assets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Protection {
    /// False until this financial contract is explicitly activated.
    pub active: bool,
    /// Signed nominal remaining designation; obligations can overdraw it.
    pub reserve: QuoteAtoms,
    /// Original and resolved episodes, including unpaid/uncovered remainder.
    pub claims: Vec<Claim>,
    /// Cumulative absorbed capital, never reset by gains, recovery or policy revision.
    pub absorbed_total: QuoteAtoms,
    /// Receipts arriving during an unqualified debt episode; assets are still recorded.
    pub receipt_faults: Vec<EventKey>,
}
impl Protection {
    pub(super) fn new(unit: AssetUnit) -> Self {
        Self {
            active: false,
            reserve: QuoteAtoms::new(unit, 0),
            claims: Vec::new(),
            absorbed_total: QuoteAtoms::new(unit, 0),
            receipt_faults: Vec::new(),
        }
    }
    /// Total remaining earmarks. Never subtract this again from positive-claim backing.
    pub fn committed(&self) -> Result<QuoteAtoms, Error> {
        self.claims
            .iter()
            .try_fold(QuoteAtoms::new(self.reserve.unit(), 0), |sum, c| {
                sum.checked_add(c.committed)
            })
    }
    /// Recognized deficit not yet earmarked; remains visible, not silently forgiven.
    pub fn unfunded(&self) -> Result<QuoteAtoms, Error> {
        self.claims
            .iter()
            .try_fold(QuoteAtoms::new(self.reserve.unit(), 0), |sum, c| {
                sum.checked_add(c.remaining()?.checked_sub(c.committed)?)
            })
    }
    /// Zero-valued historical collection right for this exact customer.
    pub fn collection(&self, id: AccountId) -> Result<QuoteAtoms, Error> {
        self.claims
            .iter()
            .filter(|c| c.id.account == id)
            .try_fold(QuoteAtoms::new(self.reserve.unit(), 0), |sum, c| {
                sum.checked_add(c.collection()?)
            })
    }
}

impl Ledger {
    /// Protection metadata is part of this ledger and its replay, never added to assets.
    pub fn protection(&self) -> &Protection {
        &self.protection
    }
    /// Named reconciliation blockers. A reopened/default episode cannot be silently
    /// absorbed from a nonflat balance or relabeled as a new episode.
    pub fn unresolved_protection(&self) -> Result<bool, LedgerError> {
        if !self.protection.receipt_faults.is_empty() {
            return Ok(true);
        }
        for (id, b) in &self.customers {
            let claims = self
                .protection
                .claims
                .iter()
                .filter(|c| c.id.account == *id)
                .try_fold(QuoteAtoms::new(self.config.quote, 0), |sum, c| {
                    sum.checked_add(c.remaining()?)
                })?;
            if claims.atoms() > 0
                && (b.positions.iter().any(|p| p.quantity().lots() != 0)
                    || b.funding.atoms() != 0
                    || claims.atoms() > b.cash.atoms().checked_neg().ok_or(Error::Overflow)?.max(0))
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn flat_claim_owner(&self, id: AccountId) -> Result<(), LedgerError> {
        let b = self.book(Owner::Customer(id))?;
        if b.funding.atoms() != 0 || b.positions.iter().any(|p| p.quantity().lots() != 0) {
            return Err(LedgerError::Restricted);
        }
        Ok(())
    }
    fn claim_index(&self, id: RequestKey) -> Result<usize, LedgerError> {
        self.protection
            .claims
            .iter()
            .position(|c| c.id == id)
            .ok_or(LedgerError::UnknownIdentity)
    }
    pub(super) fn apply_protection(
        &mut self,
        key: &RecordKey,
        change: &ProtectionChange,
    ) -> Result<(), LedgerError> {
        let RecordKey::Request(request) = key else {
            return Err(LedgerError::Attribution);
        };
        self.book(Owner::Customer(request.account))?;
        self.protection.active = true;
        let zero = QuoteAtoms::new(self.config.quote, 0);
        match change {
            ProtectionChange::Designate { delta } => {
                delta.unit().require(self.config.quote)?;
                if delta.atoms() == 0 {
                    return Err(Error::InvalidSign.into());
                }
                let next = self.protection.reserve.checked_add(*delta)?;
                if delta.atoms() > 0 && next.atoms() > self.house.cash.atoms().max(0)
                    || delta.atoms() < 0
                        && (next.atoms() < self.protection.committed()?.atoms()
                            || self.protection.unfunded()?.atoms() > 0
                            || self.unresolved_protection()?)
                {
                    return Err(LedgerError::Restricted);
                }
                self.protection.reserve = next;
            }
            ProtectionChange::Recognize {
                kind,
                cause,
                amount,
            } => {
                amount.unit().require(self.config.quote)?;
                if amount.atoms() <= 0
                    || *cause == [0; 32]
                    || self.protection.claims.iter().any(|c| {
                        c.id == *request
                            || c.id.account == request.account
                                && c.kind == *kind
                                && c.cause == *cause
                    })
                {
                    return Err(LedgerError::Configuration);
                }
                if *kind == Kind::Deficit {
                    self.flat_claim_owner(request.account)?;
                    let existing = self
                        .protection
                        .claims
                        .iter()
                        .filter(|c| c.id.account == request.account)
                        .try_fold(zero, |sum, c| sum.checked_add(c.remaining()?))?;
                    let debt = self
                        .book(Owner::Customer(request.account))?
                        .cash
                        .checked_neg()?;
                    if amount.checked_add(existing)?.atoms() > debt.atoms() {
                        return Err(LedgerError::Restricted);
                    }
                } else {
                    // New owed remediation is a liability even with exhausted capital.
                    let before = self.book(Owner::Customer(request.account))?.cash;
                    let b = self.book_mut(Owner::Customer(request.account))?;
                    b.cash = b.cash.checked_add(*amount)?;
                    self.house.cash = self.house.cash.checked_sub(*amount)?;
                    self.protection.reserve = self.protection.reserve.checked_sub(*amount)?;
                    let offset = QuoteAtoms::new(
                        self.config.quote,
                        amount
                            .atoms()
                            .min(before.atoms().checked_neg().ok_or(Error::Overflow)?.max(0)),
                    );
                    self.reduce_deficits(request.account, offset, false)?;
                }
                self.protection.claims.push(Claim {
                    id: *request,
                    kind: *kind,
                    cause: *cause,
                    amount: *amount,
                    committed: zero,
                    absorbed: zero,
                    recovered: zero,
                    offset: zero,
                    replenished: zero,
                });
            }
            ProtectionChange::Commit { claim, amount } => {
                amount.unit().require(self.config.quote)?;
                let index = self.claim_index(*claim)?;
                let c = &self.protection.claims[index];
                self.flat_claim_owner(claim.account)?;
                if amount.atoms() <= 0
                    || c.kind != Kind::Deficit
                    || c.committed.checked_add(*amount)?.atoms() > c.remaining()?.atoms()
                    || self.protection.committed()?.checked_add(*amount)?.atoms()
                        > self.protection.reserve.atoms()
                    || self.unresolved_protection()?
                {
                    return Err(LedgerError::Restricted);
                }
                self.protection.claims[index].committed = c.committed.checked_add(*amount)?;
            }
            ProtectionChange::Absorb { allocations } => {
                if allocations.is_empty() || self.unresolved_protection()? {
                    return Err(LedgerError::Restricted);
                }
                let mut total = zero;
                for (j, a) in allocations.iter().enumerate() {
                    let c = &self.protection.claims[self.claim_index(a.claim)?];
                    a.amount.unit().require(self.config.quote)?;
                    self.flat_claim_owner(a.claim.account)?;
                    if a.amount.atoms() <= 0
                        || a.amount.atoms() > c.committed.atoms()
                        || allocations[..j].iter().any(|old| old.claim == a.claim)
                    {
                        return Err(LedgerError::Restricted);
                    }
                    total = total.checked_add(a.amount)?;
                }
                if total.atoms() > self.protection.reserve.atoms()
                    || total.atoms() > self.house.cash.atoms()
                {
                    return Err(LedgerError::Restricted);
                }
                for a in allocations {
                    let index = self.claim_index(a.claim)?;
                    let b = self.book_mut(Owner::Customer(a.claim.account))?;
                    if a.amount.atoms() > b.cash.checked_neg()?.atoms() {
                        return Err(LedgerError::Restricted);
                    }
                    b.cash = b.cash.checked_add(a.amount)?;
                    self.house.cash = self.house.cash.checked_sub(a.amount)?;
                    let c = &mut self.protection.claims[index];
                    c.committed = c.committed.checked_sub(a.amount)?;
                    c.absorbed = c.absorbed.checked_add(a.amount)?;
                    self.protection.reserve = self.protection.reserve.checked_sub(a.amount)?;
                }
                self.protection.absorbed_total =
                    self.protection.absorbed_total.checked_add(total)?;
            }
        }
        Ok(())
    }
    // Same debtor, same house resource: chronological bookkeeping is not a priority
    // choice between different customers or distinct capital contributors.
    fn reduce_deficits(
        &mut self,
        id: AccountId,
        amount: QuoteAtoms,
        recovery: bool,
    ) -> Result<(), LedgerError> {
        let mut left = amount;
        for c in self
            .protection
            .claims
            .iter_mut()
            .filter(|c| c.id.account == id && c.kind == Kind::Deficit)
        {
            let take = QuoteAtoms::new(self.config.quote, left.atoms().min(c.remaining()?.atoms()));
            if recovery {
                c.recovered = c.recovered.checked_add(take)?;
            } else {
                c.offset = c.offset.checked_add(take)?;
            }
            c.committed = QuoteAtoms::new(
                self.config.quote,
                c.committed.atoms().min(c.remaining()?.atoms()),
            );
            left = left.checked_sub(take)?;
        }
        Ok(())
    }
    /// Called after a real customer receipt has already credited assets/cash. It
    /// cannot manufacture an asset by valuing an unpaid historical collection right.
    pub(super) fn protection_receipt(
        &mut self,
        id: AccountId,
        amount: QuoteAtoms,
        key: &EventKey,
    ) -> Result<(), LedgerError> {
        let outstanding = self
            .protection
            .claims
            .iter()
            .filter(|c| c.id.account == id)
            .try_fold(QuoteAtoms::new(self.config.quote, 0), |sum, c| {
                sum.checked_add(c.remaining()?)?
                    .checked_add(c.collection()?)
            })?;
        if outstanding.atoms() == 0 {
            return Ok(());
        }
        let b = self.book(Owner::Customer(id))?;
        if b.funding.atoms() != 0 || b.positions.iter().any(|p| p.quantity().lots() != 0) {
            self.protection.receipt_faults.push(key.clone());
            return Ok(());
        }
        let before = b.cash.checked_sub(amount)?;
        let unpaid = QuoteAtoms::new(
            self.config.quote,
            amount
                .atoms()
                .min(before.atoms().checked_neg().ok_or(Error::Overflow)?.max(0)),
        );
        self.reduce_deficits(id, unpaid, true)?;
        let mut left = amount.checked_sub(unpaid)?;
        for c in self
            .protection
            .claims
            .iter_mut()
            .filter(|c| c.id.account == id && c.kind == Kind::Deficit)
        {
            let take =
                QuoteAtoms::new(self.config.quote, left.atoms().min(c.collection()?.atoms()));
            c.replenished = c.replenished.checked_add(take)?;
            left = left.checked_sub(take)?;
        }
        let replenished = amount.checked_sub(unpaid)?.checked_sub(left)?;
        let b = self.book_mut(Owner::Customer(id))?;
        b.cash = b.cash.checked_sub(replenished)?;
        self.house.cash = self.house.cash.checked_add(replenished)?;
        self.protection.reserve = self.protection.reserve.checked_add(replenished)?;
        Ok(())
    }
}
