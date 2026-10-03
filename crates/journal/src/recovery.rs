//! Private, read-only final-cut projection from the accepted financial journal.
//! This is neither a fenced recovery session nor authority to publish or redeem
//! a root. Native fencing, chain counters/backing and independent package delivery
//! must be qualified separately before staging/activation.
use crate::{
    Backend, Error, Head, Journal, Protection,
    model::{Attempt, AttemptKind, ControlError, InputResult, Resource, State},
    wire::{Reader, Writer},
};
use cinder_kernel::{
    amounts::QuoteAtoms,
    identity::{AccountId, EventKey},
    ledger::{Book, Change, Config, Location, Owner, evidence::Disposition},
};

/// Trusted operator cutover port, not a customer command or self-authenticating
/// hash. Before Begin, verify independent writer takeover, custody freeze and
/// native capability revocation/closure. Expiry/loaded-key revocation alone is
/// insufficient. Retain that authenticated evidence under `fence` in the journal.
#[derive(Clone, PartialEq, Eq)]
pub enum Action {
    /// Freeze ordinary paths and permit only bounded recovery closes/returns.
    Begin {
        /// Exact reviewed financial state.
        expected_version: u64,
        /// Distinct authenticated recovery-operator authority.
        authority_epoch: u64,
        /// Trusted dispatch deadline; expiry does not settle escaped operations.
        valid_until: u64,
        /// Retained, externally verified cutover evidence binding; not proof itself.
        fence: [u8; 32],
    },
    /// Irreversibly stop recovery dispatch after all effects are resolved. Facts
    /// continue to be retained and change the head, invalidating prepared roots.
    Seal {
        /// Exact settled financial state; chain/package checks remain separate.
        expected_version: u64,
    },
}
impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RecoveryAction([PRIVATE])")
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Permission {
    pub authority_epoch: u64,
    pub valid_until: u64,
    pub fence: [u8; 32],
    pub sealed: bool,
}
impl State {
    /// Recovery dispatch is permanently sealed; ordinary paths stay frozen.
    pub fn recovery_sealed(&self) -> bool {
        self.recovery.as_ref().is_some_and(|p| p.sealed)
    }
    /// Current recovery operator epoch; None includes expired/sealed permissions.
    pub fn recovery_epoch(&self) -> Option<u64> {
        self.recovery
            .as_ref()
            .filter(|p| !p.sealed && self.now < p.valid_until)
            .map(|p| p.authority_epoch)
    }
    pub(crate) fn recovery_action(&mut self, action: &Action) -> Result<(), ControlError> {
        match action {
            Action::Begin {
                expected_version,
                authority_epoch,
                valid_until,
                fence,
            } => {
                if *expected_version != self.ledger.version()
                    || *authority_epoch == 0
                    || *valid_until <= self.now
                    || *fence == [0; 32]
                    || self.recovery.is_some()
                {
                    return Err(ControlError::Invalid);
                }
                self.frozen = true;
                self.recovery = Some(Permission {
                    authority_epoch: *authority_epoch,
                    valid_until: *valid_until,
                    fence: *fence,
                    sealed: false,
                });
            }
            Action::Seal { expected_version } => {
                if *expected_version != self.ledger.version()
                    || self.recovery.is_none()
                    || self.recovery_sealed()
                    || self.holds.iter().any(|h| h.active)
                    || self.orders.iter().any(|o| !o.complete())
                    || self.funds.iter().any(|o| !o.terminal || o.faulted)
                    || self.raw_unresolved != 0
                    || self.ledger.unresolved_attribution() != 0
                    || self.ledger.unresolved_funds() != 0
                    || self.ledger.issues().iter().any(|i| i.open)
                    || !self.recovery_flat_returns()
                    || self.ledger.venue().cash().atoms() != 0
                    || self.ledger.broker().atoms() != 0
                    || self
                        .ledger
                        .in_transit()
                        .map_err(|_| ControlError::Capacity)?
                        .atoms()
                        != 0
                {
                    return Err(ControlError::Unqualified);
                }
                self.recovery.as_mut().ok_or(ControlError::Invalid)?.sealed = true;
            }
        }
        Ok(())
    }
    pub(crate) fn recovery_exposure(&self, attempt: &Attempt) -> bool {
        let Some(epoch) = self.recovery_epoch() else {
            return false;
        };
        attempt.authority_epoch == epoch
            && match attempt.kind {
                AttemptKind::Funds => self
                    .funds
                    .iter()
                    .any(|o| o.recovery && o.attempt == Some(attempt.key)),
                AttemptKind::Emergency => self.closes.iter().any(|c| {
                    c.request == attempt.key.request
                        && matches!(
                            c.kind,
                            crate::liquidation::Kind::RecoveryClose
                                | crate::liquidation::Kind::HouseUnwind
                        )
                }),
                _ => false,
            }
    }
    pub(crate) fn recovery_flat_returns(&self) -> bool {
        self.frozen
            && self.recovery.is_some()
            && flat(self.ledger.venue())
            && self
                .config
                .customers
                .iter()
                .copied()
                .map(Owner::Customer)
                .chain([Owner::House, Owner::Suspense])
                .all(|o| self.ledger.book(o).is_ok_and(flat))
    }
    // After gross settlement, returning existing physical assets need not meet
    // new-risk capital targets. Still enforce each shared source reservation and
    // conservative house fee capacity. No customer claim or debt becomes capital.
    pub(crate) fn recovery_return_capacity(&self) -> Result<(), ControlError> {
        let positive = self
            .config
            .customers
            .iter()
            .copied()
            .map(Owner::Customer)
            .chain([Owner::Suspense])
            .try_fold(0_i128, |sum, o| {
                sum.checked_add(
                    self.ledger
                        .book(o)
                        .map_err(|_| ControlError::Invalid)?
                        .cash()
                        .atoms()
                        .max(0),
                )
                .ok_or(ControlError::Capacity)
            })?;
        let assets = self
            .ledger
            .vault()
            .checked_add(self.ledger.broker())
            .and_then(|x| x.checked_add(self.ledger.venue().cash()))
            .map_err(|_| ControlError::Capacity)?
            .atoms();
        for h in self.holds.iter().filter(|h| h.active) {
            for r in &h.reservations {
                let cap = match r.resource {
                    Resource::Location(Location::Vault) => self.ledger.vault().atoms(),
                    Resource::Location(Location::Broker) => self.ledger.broker().atoms(),
                    Resource::Location(Location::Venue) => self.ledger.venue().cash().atoms(),
                    Resource::Customer(a) => self
                        .ledger
                        .book(Owner::Customer(a))
                        .map_err(|_| ControlError::Invalid)?
                        .cash()
                        .atoms()
                        .max(0),
                    Resource::House => self
                        .ledger
                        .book(Owner::House)
                        .map_err(|_| ControlError::Invalid)?
                        .cash()
                        .atoms()
                        .min(assets.checked_sub(positive).ok_or(ControlError::Capacity)?)
                        .max(0),
                };
                if self
                    .reserved(r.resource)
                    .map_err(|_| ControlError::Capacity)?
                    .atoms()
                    > cap
                {
                    return Err(ControlError::Capacity);
                }
            }
        }
        Ok(())
    }
}
pub(crate) fn encode_action(w: &mut Writer, a: &Action) {
    match a {
        Action::Begin {
            expected_version,
            authority_epoch,
            valid_until,
            fence,
        } => {
            w.byte(0);
            w.u64(*expected_version);
            w.u64(*authority_epoch);
            w.u64(*valid_until);
            w.raw(fence);
        }
        Action::Seal { expected_version } => {
            w.byte(1);
            w.u64(*expected_version);
        }
    }
}
pub(crate) fn decode_action(r: &mut Reader<'_>) -> Result<Action, Error> {
    match r.byte()? {
        0 => Ok(Action::Begin {
            expected_version: r.u64()?,
            authority_epoch: r.u64()?,
            valid_until: r.u64()?,
            fence: r.array()?,
        }),
        1 => Ok(Action::Seal {
            expected_version: r.u64()?,
        }),
        _ => Err(Error::Codec),
    }
}

/// Secret-safe refusal; no private payload or account identity is returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutError {
    /// Storage/freshness authority failed; preserve the journal's poisoned state.
    Journal(Error),
    /// Caller did not review the exact current journal head.
    Stale,
    /// Ordinary journal exposure/admission has not been durably frozen.
    NotFrozen,
    /// A hold, order or funds operation is still live or faulted.
    OpenCommitments,
    /// Raw, attribution, protection or reconciliation evidence is unresolved.
    UnresolvedEvidence,
    /// Gross private/native positions or a funding boundary remain unsettled.
    NotSettled,
    /// Venue, broker or in-flight assets are not returned vault cash.
    AssetsNotReturned,
    /// Missing/incomplete reconciliation or financial activity after that check.
    IncompleteCheck,
    /// Positive customer and suspense claims exceed recorded vault cash.
    Unbacked,
    /// Exact aggregation cannot be represented; never clamp or wrap it.
    Arithmetic,
}

/// One customer's already-net entitlement and lifetime ordinary payment history.
/// Fees/losses/payouts have already changed cash; do not subtract `paid` again.
#[derive(Clone, PartialEq, Eq)]
pub struct AccountCut {
    account: AccountId,
    cash: QuoteAtoms,
    paid: QuoteAtoms,
}
impl std::fmt::Debug for AccountCut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AccountCut([PRIVATE])")
    }
}
impl AccountCut {
    /// Private configured identity, not a public wallet or recipient mapping.
    pub fn account(&self) -> AccountId {
        self.account
    }
    /// Signed settled cash, including debt that is not a collectible backing asset.
    pub fn cash(&self) -> QuoteAtoms {
        self.cash
    }
    /// Actual beneficiary payments only; excludes fees and source debits in transit.
    pub fn paid(&self) -> QuoteAtoms {
        self.paid
    }
    /// Positive remaining entitlement, already net of settled ordinary payments.
    pub fn payable(&self) -> QuoteAtoms {
        QuoteAtoms::new(self.cash.unit(), self.cash.atoms().max(0))
    }
}

/// Exact financial projection only. Includes every configured customer, even a
/// zero or negative account; positive claims cannot be netted against their debt.
#[derive(Clone, PartialEq, Eq)]
pub struct Cut {
    head: Head,
    configuration: Config,
    check: EventKey,
    accounts: Vec<AccountCut>,
    total: QuoteAtoms,
    deficits: QuoteAtoms,
    vault: QuoteAtoms,
    suspense: QuoteAtoms,
    house: QuoteAtoms,
}
impl std::fmt::Debug for Cut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RecoveryCut([PRIVATE])")
    }
}
impl Cut {
    /// Accepted financial/control tail, not a native capability-fencing certificate.
    pub fn head(&self) -> Head {
        self.head
    }
    /// Trusted immutable journal domain, pool, units and complete customer inventory.
    pub fn configuration(&self) -> &Config {
        &self.configuration
    }
    /// Exact final native reconciliation input; not merely the latest timestamp.
    pub fn check(&self) -> &EventKey {
        &self.check
    }
    /// All customers in immutable configuration order, not a caller-selected subset.
    pub fn accounts(&self) -> &[AccountCut] {
        &self.accounts
    }
    /// Sum of positive unpaid customer claims only.
    pub fn total(&self) -> QuoteAtoms {
        self.total
    }
    /// Retained uncollectible customer debts; never added to vault backing.
    pub fn deficits(&self) -> QuoteAtoms {
        self.deficits
    }
    /// Recorded returned vault cash, not an authenticated current SPL observation.
    pub fn vault(&self) -> QuoteAtoms {
        self.vault
    }
    /// Other unresolved ownership liabilities remain separate, never customer capital.
    pub fn suspense(&self) -> QuoteAtoms {
        self.suspense
    }
    /// Existing house cash; reserve designation is not another asset.
    pub fn house(&self) -> QuoteAtoms {
        self.house
    }
}

fn flat(book: &Book) -> bool {
    book.funding().atoms() == 0
        && book
            .positions()
            .iter()
            .all(|p| p.quantity().lots() == 0 && p.basis().atoms() == 0)
}

impl<B: Backend, P: Protection> Journal<B, P> {
    /// Derive a settled cut only after checking the backend's current authority.
    /// Plain SQLite has no independent witness. A later commit invalidates this
    /// projection; this function does not seal the tail or revoke escaped keys.
    pub fn recovery_cut(&mut self, expected: Head, check: EventKey) -> Result<Cut, CutError> {
        self.verified_state().map_err(CutError::Journal)?;
        if expected != self.head {
            return Err(CutError::Stale);
        }
        let s = self.state().map_err(CutError::Journal)?;
        let l = s.ledger();
        if !s.frozen() {
            return Err(CutError::NotFrozen);
        }
        if s.holds().iter().any(|h| h.active)
            || s.orders().iter().any(|o| !o.complete())
            || s.funds().iter().any(|o| !o.terminal || o.faulted)
            || s.attempts().iter().any(|a| match a.kind {
                AttemptKind::Funds => !s
                    .funds()
                    .iter()
                    .any(|o| o.attempt == Some(a.key) && o.terminal && !o.faulted),
                AttemptKind::Order
                | AttemptKind::Cancel
                | AttemptKind::Emergency
                | AttemptKind::Restoration => !s
                    .orders()
                    .iter()
                    .any(|o| o.intent.request == a.key.request && o.complete()),
                // Generic attempts have no qualified terminal lifecycle. A released
                // hold or policy expiry cannot manufacture closure evidence.
                AttemptKind::Generic => true,
            })
            || l.movements().iter().any(|m| {
                !s.funds()
                    .iter()
                    .any(|o| o.attempt == Some(m.mandate.attempt) && o.terminal && !o.faulted)
            })
        {
            return Err(CutError::OpenCommitments);
        }
        if s.unresolved_raw() != 0
            || l.unresolved_attribution() != 0
            || l.unresolved_funds() != 0
            || l.issues().iter().any(|i| i.open)
            || l.unresolved_protection()
                .map_err(|_| CutError::Arithmetic)?
        {
            return Err(CutError::UnresolvedEvidence);
        }
        l.check_bridge().map_err(|_| CutError::UnresolvedEvidence)?;
        for owner in self
            .configuration()
            .customers
            .iter()
            .copied()
            .map(Owner::Customer)
            .chain([Owner::House, Owner::Suspense])
        {
            if !flat(l.book(owner).map_err(|_| CutError::UnresolvedEvidence)?)
                || l.funding_views(owner)
                    .iter()
                    .any(|f| !f.settled || f.payment.is_none())
            {
                return Err(CutError::NotSettled);
            }
        }
        if !flat(l.venue()) {
            return Err(CutError::NotSettled);
        }
        if l.venue().cash().atoms() != 0
            || l.broker().atoms() != 0
            || l.in_transit().map_err(|_| CutError::Arithmetic)?.atoms() != 0
        {
            return Err(CutError::AssetsNotReturned);
        }
        // A final check must follow every financial posting. Control-only freeze
        // or terminal release may follow it without inventing a newer native cut.
        let qualified = self.history.iter().any(|r| {
            r.tx.inputs
                .iter()
                .zip(&r.receipt.inputs)
                .any(|(i, result)| {
                    i.source_cut.is_some()
                        && i.authority_epoch != 0
                        && i.source == check.scope
                        && *result == InputResult::Normalized(Disposition::Applied)
                        && i.event.as_ref().is_some_and(|e| {
                            e.key == cinder_kernel::identity::RecordKey::Economic(check.clone())
                                && matches!(&e.change, Change::Reconcile(c)
                                if c.complete
                                && c.expected_version.checked_add(1) == Some(l.version())
                                && c.cash == Some(l.venue().cash())
                                && c.funding == Some(l.venue().funding())
                                && c.positions.as_deref() == Some(l.venue().positions()))
                        })
                })
        });
        if !qualified {
            return Err(CutError::IncompleteCheck);
        }
        let zero = QuoteAtoms::new(self.configuration().quote, 0);
        let mut total = zero;
        let mut deficits = zero;
        let mut accounts = Vec::with_capacity(self.configuration().customers.len());
        for account in &self.configuration().customers {
            let cash = l
                .book(Owner::Customer(*account))
                .map_err(|_| CutError::UnresolvedEvidence)?
                .cash();
            if cash.atoms() > 0 {
                total = total.checked_add(cash).map_err(|_| CutError::Arithmetic)?;
            } else {
                deficits = deficits
                    .checked_sub(cash)
                    .map_err(|_| CutError::Arithmetic)?;
            }
            accounts.push(AccountCut {
                account: *account,
                cash,
                paid: l.paid(*account).map_err(|_| CutError::Arithmetic)?,
            });
        }
        let suspense = l
            .book(Owner::Suspense)
            .map_err(|_| CutError::UnresolvedEvidence)?
            .cash();
        let other_claims = QuoteAtoms::new(zero.unit(), suspense.atoms().max(0));
        if l.vault().atoms()
            < total
                .checked_add(other_claims)
                .map_err(|_| CutError::Arithmetic)?
                .atoms()
        {
            return Err(CutError::Unbacked);
        }
        Ok(Cut {
            head: expected,
            configuration: self.configuration().clone(),
            check,
            accounts,
            total,
            deficits,
            vault: l.vault(),
            suspense,
            house: l
                .book(Owner::House)
                .map_err(|_| CutError::UnresolvedEvidence)?
                .cash(),
        })
    }
}
