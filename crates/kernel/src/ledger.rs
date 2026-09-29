//! One in-memory economic state per configured quote pool. Pure proposals only:
//! caller authentication, source qualification and durable atomic commit are not
//! implemented here. There is no setter for reported equity or a second asset book.

use crate::{Error, Transition, amounts::*, identity::*, position::*};
use alloc::vec::Vec;

/// An entitlement owner, not a physical custody location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    /// Private customer claim.
    Customer(AccountId),
    /// Cinder's own capital/exposure.
    House,
    /// Observed but not yet attributed economics; never silently house income.
    Suspense,
}

/// The initial pool's two location classes; native cash is not a token balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Location {
    /// Nonnegative physical quote assets under the designated custody route.
    Vault,
    /// Signed native venue cash, excluding position PnL.
    Venue,
}

/// Trusted source routing, qualified by the eventual ingestion adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Source {
    /// Exact deployment/venue/account/semantic namespace.
    pub scope: EventScope,
    /// Which location this source can evidence; not inferred from event payloads.
    pub location: Location,
}

/// Explicit single-quote, single-native-account scope. Additional pools must not
/// be summed into cross-margin without the later joined risk/ownership contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Cinder deployment.
    pub domain: Domain,
    /// Approved economic policy revision.
    pub policy: PolicyVersion,
    /// Quote atom definition shared by these linear markets and locations.
    pub quote: AssetUnit,
    /// Initial native venue.
    pub venue: VenueId,
    /// Initial pooled native account.
    pub venue_account: VenueAccountId,
    /// Allowed source namespaces; no automatic acceptance of new namespaces.
    pub sources: Vec<Source>,
    /// Explicit qualified market conversions.
    pub markets: Vec<Market>,
    /// Initial private identities; runtime onboarding is a later controller.
    pub customers: Vec<AccountId>,
}

/// Fill direction of a pre-recorded customer execution route, not a risk limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Positive lots.
    Buy,
    /// Negative lots.
    Sell,
}
impl Side {
    fn matches(self, lots: i64) -> bool {
        match self {
            Self::Buy => lots > 0,
            Self::Sell => lots < 0,
        }
    }
}

/// Actual fill ownership; a customer is derived from its recorded attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillTarget {
    /// No caller-selected customer field can override this stored route.
    Customer(AttemptKey),
    /// Explicit house-owned execution. This API is not permission to trade.
    House,
    /// Qualified external fact whose private attribution is unresolved.
    Unattributed,
}

/// Phase-local transition schema. No outbound actions or risk admission are exposed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// Record an already authorized route under an Attempt record key. P07 adds
    /// reservations/lifecycle; this records no fill and grants no signing authority.
    BindExecution {
        /// Configured market.
        market: MarketUnit,
        /// Direction expected for this attempt.
        side: Side,
    },
    /// Qualified final receipt, with exactly one asset and one entitlement credit.
    Receipt {
        /// Explicit attribution; use suspense when unknown.
        owner: Owner,
        /// Evidenced destination.
        location: Location,
        /// Positive exact quote atoms.
        amount: QuoteAtoms,
    },
    /// Fee-exclusive actual native fill, whether or not current risk is acceptable.
    Fill {
        /// Pre-recorded owner or explicit house/suspense.
        target: FillTarget,
        /// Actual signed executed quantity; not order size.
        quantity: QuantityLots,
        /// Qualified exact execution price.
        price: PriceTicks,
    },
    /// Full source debit creates one transfer receivable. Its economic event key
    /// is the transfer identity; later lifecycle/partial receipts belong to P08.
    TransferDebit {
        /// Evidenced source.
        source: Location,
        /// Intended other location.
        destination: Location,
        /// Positive amount, fee-free for this bounded phase.
        amount: QuoteAtoms,
    },
    /// Qualified full arrival consumes the named receivable without new user credit.
    TransferArrival {
        /// Exact source-debit economic identity, not the arrival's own event key.
        debit: EventKey,
    },
}

/// Exact normalized semantic event. Transport metadata does not define equality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Attempt for bindings; economic execution/receipt identity otherwise.
    pub key: RecordKey,
    /// Must match configured policy; unknown revisions are never guessed.
    pub policy: PolicyVersion,
    /// Economic payload retained for exact duplicate/conflict comparison.
    pub change: Change,
}

/// Ledger rejection. A failed proposal leaves the input state unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerError {
    /// Checked arithmetic or primitive scope/precision failure.
    Primitive(Error),
    /// Duplicate configuration identity, missing scope or impossible configuration.
    Configuration,
    /// Unconfigured private owner/market or missing execution route.
    UnknownIdentity,
    /// Key kind, source location, market or execution owner routing is wrong.
    Attribution,
    /// Same key with a different economic payload.
    Conflict,
    /// Physical source lacks the stated assets; do not fabricate negative tokens.
    PhysicalShortfall,
    /// Transfer is unknown, already arrived, or has an invalid route.
    Transfer,
    /// Structural exposure or cash/basis bridge failed (not a solvency failure).
    Bridge,
    /// Missing, duplicate or unconfigured valuation market.
    Valuation,
}
impl From<Error> for LedgerError {
    fn from(value: Error) -> Self {
        Self::Primitive(value)
    }
}

/// Signed entitlement/native book. Fields are read-only outside transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Book {
    cash: QuoteAtoms,
    positions: Vec<Position>,
}
impl Book {
    /// Settled signed cash, not physical tokens or withdrawable collateral.
    pub fn cash(&self) -> QuoteAtoms {
        self.cash
    }
    /// Immutable position projection.
    pub fn positions(&self) -> &[Position] {
        &self.positions
    }
    fn empty(config: &Config) -> Self {
        Self {
            cash: QuoteAtoms::new(config.quote, 0),
            positions: config
                .markets
                .iter()
                .map(|m| Position::flat(m.unit()))
                .collect(),
        }
    }
    fn execute(
        &mut self,
        index: usize,
        market: Market,
        q: QuantityLots,
        p: PriceTicks,
    ) -> Result<(), LedgerError> {
        let fill = self.positions[index].fill(market, q, p)?;
        self.cash = self.cash.checked_add(fill.realized)?;
        self.positions[index] = fill.position;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Binding {
    attempt: AttemptKey,
    market: MarketUnit,
    side: Side,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Transfer {
    debit: EventKey,
    destination: Location,
    amount: QuoteAtoms,
    arrived: bool,
}

/// Sole phase-local state. Cloning produces a proposal, not another asset owner.
/// The in-memory event list is not a production journal or an unbounded-service design.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ledger {
    config: Config,
    customers: Vec<(AccountId, Book)>,
    house: Book,
    suspense: Book,
    venue: Book,
    vault: QuoteAtoms,
    transfers: Vec<Transfer>,
    bindings: Vec<Binding>,
    events: Vec<Event>,
    unresolved: Vec<EventKey>,
    version: u64,
}

/// Read-only diagnostic at caller-supplied qualified marks. Not an admission verdict,
/// a proof of authentic assets, a liquidity report, or a claim of future solvency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostics {
    /// Signed external value: vault + transit + native cash + native marked PnL.
    pub net_assets: QuoteAtoms,
    /// Individually positive customer entitlements only.
    pub customer_claims: QuoteAtoms,
    /// Customer deficits, assigned zero collectible asset value.
    pub customer_deficits: QuoteAtoms,
    /// Signed house equity; not automatically free protection capital.
    pub house_equity: QuoteAtoms,
    /// Signed unattributed equity; never silently absorbed by the house.
    pub suspense_equity: QuoteAtoms,
    /// N minus positive customer claims and positive suspense obligations.
    pub backing_margin: QuoteAtoms,
    /// Positive uncovered amount, without erasing the original claims.
    pub shortfall: QuoteAtoms,
    /// Unresolved attribution prevents interpreting a zero shortfall as assurance.
    pub unresolved_events: usize,
}

impl Ledger {
    /// Construct an empty, explicitly scoped pool. No opening equity injection exists.
    pub fn new(config: Config) -> Result<Self, LedgerError> {
        if config.markets.is_empty() || config.sources.is_empty() {
            return Err(LedgerError::Configuration);
        }
        for (i, m) in config.markets.iter().enumerate() {
            m.unit().quote.require(config.quote)?;
            if config.markets[..i]
                .iter()
                .any(|other| other.unit().market == m.unit().market)
            {
                return Err(LedgerError::Configuration);
            }
        }
        for (i, id) in config.customers.iter().enumerate() {
            if config.customers[..i].contains(id) {
                return Err(LedgerError::Configuration);
            }
        }
        for (i, source) in config.sources.iter().enumerate() {
            if source.scope.domain != config.domain
                || config.sources[..i].iter().any(|s| s.scope == source.scope)
                || (source.location == Location::Venue
                    && (source.scope.venue != config.venue
                        || source.scope.account != config.venue_account))
            {
                return Err(LedgerError::Configuration);
            }
        }
        Ok(Self {
            customers: config
                .customers
                .iter()
                .map(|id| (*id, Book::empty(&config)))
                .collect(),
            house: Book::empty(&config),
            suspense: Book::empty(&config),
            venue: Book::empty(&config),
            vault: QuoteAtoms::new(config.quote, 0),
            transfers: Vec::new(),
            bindings: Vec::new(),
            events: Vec::new(),
            unresolved: Vec::new(),
            version: 0,
            config,
        })
    }
    /// Monotone successful distinct transition count, not a durable CAS by itself.
    pub fn version(&self) -> u64 {
        self.version
    }
    /// Immutable entitlement projection; cannot be used as a second asset input.
    pub fn book(&self, owner: Owner) -> Result<&Book, LedgerError> {
        match owner {
            Owner::House => Ok(&self.house),
            Owner::Suspense => Ok(&self.suspense),
            Owner::Customer(id) => self
                .customers
                .iter()
                .find(|(i, _)| *i == id)
                .map(|(_, b)| b)
                .ok_or(LedgerError::UnknownIdentity),
        }
    }
    fn book_mut(&mut self, owner: Owner) -> Result<&mut Book, LedgerError> {
        match owner {
            Owner::House => Ok(&mut self.house),
            Owner::Suspense => Ok(&mut self.suspense),
            Owner::Customer(id) => self
                .customers
                .iter_mut()
                .find(|(i, _)| *i == id)
                .map(|(_, b)| b)
                .ok_or(LedgerError::UnknownIdentity),
        }
    }
    /// Native signed cash and net positions, excluding the physical vault.
    pub fn venue(&self) -> &Book {
        &self.venue
    }
    /// Nonnegative physical assets; not additive to a native total-equity snapshot.
    pub fn vault(&self) -> QuoteAtoms {
        self.vault
    }
    /// Outstanding full-transfer receivables, counted once and not spendable cash.
    pub fn in_transit(&self) -> Result<QuoteAtoms, LedgerError> {
        self.transfers
            .iter()
            .filter(|t| !t.arrived)
            .try_fold(QuoteAtoms::new(self.config.quote, 0), |a, t| {
                Ok(a.checked_add(t.amount)?)
            })
    }
    /// Accepted normalized provenance; raw/rejected evidence retention belongs to P05.
    pub fn events(&self) -> &[Event] {
        &self.events
    }
    fn market_index(&self, unit: MarketUnit) -> Result<usize, LedgerError> {
        let index = self
            .config
            .markets
            .iter()
            .position(|m| m.unit().market == unit.market)
            .ok_or(LedgerError::UnknownIdentity)?;
        unit.require(self.config.markets[index].unit())?;
        Ok(index)
    }
    fn source(&self, key: &RecordKey, location: Location) -> Result<EventKey, LedgerError> {
        let RecordKey::Economic(key) = key else {
            return Err(LedgerError::Attribution);
        };
        if !self
            .config
            .sources
            .iter()
            .any(|s| s.scope == key.scope && s.location == location)
        {
            return Err(LedgerError::Attribution);
        }
        Ok(key.clone())
    }
    fn move_cash(&mut self, location: Location, amount: QuoteAtoms) -> Result<(), LedgerError> {
        match location {
            Location::Vault => {
                let next = self.vault.checked_add(amount)?;
                if next.atoms() < 0 {
                    return Err(LedgerError::PhysicalShortfall);
                }
                self.vault = next;
            }
            Location::Venue => self.venue.cash = self.venue.cash.checked_add(amount)?,
        }
        Ok(())
    }
    /// Compute a full proposed transition. Equal duplicate is a no-op; changed
    /// payload conflicts. No capital/admission filter suppresses adverse fills.
    pub fn apply(&self, event: &Event) -> Result<Self, LedgerError> {
        event.key.require_domain(self.config.domain)?;
        event.policy.require(self.config.policy)?;
        if let Some(old) = self.events.iter().find(|old| old.key == event.key) {
            return if old == event {
                Ok(self.clone())
            } else {
                Err(LedgerError::Conflict)
            };
        }
        let mut next = self.clone();
        next.apply_distinct(event)?;
        next.check_bridge()?;
        next.version = self.version.checked_add(1).ok_or(Error::Overflow)?;
        next.events.push(event.clone());
        Ok(next)
    }
    fn apply_distinct(&mut self, event: &Event) -> Result<(), LedgerError> {
        match &event.change {
            Change::BindExecution { market, side } => {
                let RecordKey::Attempt(attempt) = event.key else {
                    return Err(LedgerError::Attribution);
                };
                self.book(Owner::Customer(attempt.request.account))?;
                self.market_index(*market)?;
                self.bindings.push(Binding {
                    attempt,
                    market: *market,
                    side: *side,
                });
            }
            Change::Receipt {
                owner,
                location,
                amount,
            } => {
                let source = self.source(&event.key, *location)?;
                amount.unit().require(self.config.quote)?;
                if amount.atoms() <= 0 {
                    return Err(Error::InvalidSign.into());
                }
                let book = self.book_mut(*owner)?;
                book.cash = book.cash.checked_add(*amount)?;
                self.move_cash(*location, *amount)?;
                if *owner == Owner::Suspense {
                    self.unresolved.push(source);
                }
            }
            Change::Fill {
                target,
                quantity,
                price,
            } => {
                let source = self.source(&event.key, Location::Venue)?;
                let index = self.market_index(quantity.unit())?;
                let market = self.config.markets[index];
                let owner = match target {
                    FillTarget::Customer(attempt) => {
                        let binding = self
                            .bindings
                            .iter()
                            .find(|b| b.attempt == *attempt)
                            .ok_or(LedgerError::UnknownIdentity)?;
                        if binding.market != quantity.unit()
                            || !binding.side.matches(quantity.lots())
                        {
                            return Err(LedgerError::Attribution);
                        }
                        Owner::Customer(binding.attempt.request.account)
                    }
                    FillTarget::House => Owner::House,
                    FillTarget::Unattributed => Owner::Suspense,
                };
                self.book_mut(owner)?
                    .execute(index, market, *quantity, *price)?;
                self.venue.execute(index, market, *quantity, *price)?;
                if owner == Owner::Suspense {
                    self.unresolved.push(source);
                }
            }
            Change::TransferDebit {
                source,
                destination,
                amount,
            } => {
                let debit = self.source(&event.key, *source)?;
                amount.unit().require(self.config.quote)?;
                if source == destination {
                    return Err(LedgerError::Transfer);
                }
                if amount.atoms() <= 0 {
                    return Err(Error::InvalidSign.into());
                }
                self.move_cash(*source, amount.checked_neg()?)?;
                self.transfers.push(Transfer {
                    debit,
                    destination: *destination,
                    amount: *amount,
                    arrived: false,
                });
            }
            Change::TransferArrival { debit } => {
                let index = self
                    .transfers
                    .iter()
                    .position(|t| t.debit == *debit && !t.arrived)
                    .ok_or(LedgerError::Transfer)?;
                let transfer = self.transfers[index].clone();
                self.source(&event.key, transfer.destination)?;
                self.move_cash(transfer.destination, transfer.amount)?;
                self.transfers[index].arrived = true;
            }
        }
        Ok(())
    }
    /// Exact structural identities, independent of any mark or positive backing.
    /// Signed native cash minus native basis equals total private cash minus basis
    /// after including the vault and transit exactly once.
    pub fn check_bridge(&self) -> Result<(), LedgerError> {
        let books = || {
            self.customers
                .iter()
                .map(|(_, b)| b)
                .chain([&self.house, &self.suspense])
        };
        for (index, _) in self.config.markets.iter().enumerate() {
            let sum = books().try_fold(0_i128, |sum, b| {
                sum.checked_add(i128::from(b.positions[index].quantity().lots()))
                    .ok_or(Error::Overflow)
            })?;
            if sum != i128::from(self.venue.positions[index].quantity().lots()) {
                return Err(LedgerError::Bridge);
            }
        }
        let intercept = |book: &Book| -> Result<i128, Error> {
            book.positions.iter().try_fold(book.cash.atoms(), |sum, p| {
                sum.checked_sub(p.basis().atoms()).ok_or(Error::Overflow)
            })
        };
        let native = self
            .vault
            .atoms()
            .checked_add(self.in_transit()?.atoms())
            .and_then(|n| n.checked_add(self.venue.cash.atoms()))
            .ok_or(Error::Overflow)?;
        let external = self.venue.positions.iter().try_fold(native, |n, p| {
            n.checked_sub(p.basis().atoms()).ok_or(Error::Overflow)
        })?;
        let internal = books().try_fold(0_i128, |n, b| {
            n.checked_add(intercept(b)?).ok_or(Error::Overflow)
        })?;
        if external != internal {
            return Err(LedgerError::Bridge);
        }
        Ok(())
    }
    /// Marked equity for one owner. Marks must be complete, unique and correctly
    /// versioned; freshness/authenticity remain caller obligations until P04/P13.
    pub fn equity(&self, owner: Owner, marks: &[PriceTicks]) -> Result<QuoteAtoms, LedgerError> {
        self.validate_marks(marks)?;
        self.book_equity(self.book(owner)?, marks)
    }
    fn validate_marks(&self, marks: &[PriceTicks]) -> Result<(), LedgerError> {
        if marks.len() != self.config.markets.len() {
            return Err(LedgerError::Valuation);
        }
        for (i, mark) in marks.iter().enumerate() {
            self.market_index(mark.unit())?;
            if marks[..i]
                .iter()
                .any(|p| p.unit().market == mark.unit().market)
            {
                return Err(LedgerError::Valuation);
            }
        }
        Ok(())
    }
    fn book_equity(&self, book: &Book, marks: &[PriceTicks]) -> Result<QuoteAtoms, LedgerError> {
        marks.iter().try_fold(book.cash, |equity, mark| {
            let index = self.market_index(mark.unit())?;
            Ok(equity.checked_add(
                book.positions[index].unrealized(self.config.markets[index], *mark)?,
            )?)
        })
    }
    /// Claims and shortfall at a common valuation cut; no negative debt collectibility.
    pub fn diagnostics(&self, marks: &[PriceTicks]) -> Result<Diagnostics, LedgerError> {
        self.validate_marks(marks)?;
        self.check_bridge()?;
        let net_assets = self
            .vault
            .checked_add(self.in_transit()?)?
            .checked_add(self.book_equity(&self.venue, marks)?)?;
        let mut claims = QuoteAtoms::new(self.config.quote, 0);
        let mut deficits = claims;
        for (_, book) in &self.customers {
            let equity = self.book_equity(book, marks)?;
            if equity.atoms() >= 0 {
                claims = claims.checked_add(equity)?;
            } else {
                deficits = deficits.checked_sub(equity)?;
            }
        }
        let suspense_equity = self.book_equity(&self.suspense, marks)?;
        let backing_margin = net_assets
            .checked_sub(claims)?
            .checked_sub(QuoteAtoms::new(
                self.config.quote,
                suspense_equity.atoms().max(0),
            ))?;
        let shortfall = if backing_margin.atoms() < 0 {
            backing_margin.checked_neg()?
        } else {
            QuoteAtoms::new(self.config.quote, 0)
        };
        Ok(Diagnostics {
            net_assets,
            customer_claims: claims,
            customer_deficits: deficits,
            house_equity: self.book_equity(&self.house, marks)?,
            suspense_equity,
            backing_margin,
            shortfall,
            unresolved_events: self.unresolved.len(),
        })
    }
}

/// Pure transition adapter for generic controllers; does not commit or dispatch.
pub struct LedgerTransition;
impl Transition for LedgerTransition {
    type State = Ledger;
    type Event = Event;
    type Error = LedgerError;
    fn apply(state: &Ledger, event: &Event) -> Result<Ledger, LedgerError> {
        state.apply(event)
    }
}
