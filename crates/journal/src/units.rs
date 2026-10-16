//! Opt-in, new-history unit composition. One accepted state/head, no dispatch.
//!
//! This does not migrate the shipping single-unit journal or authenticate a
//! caller. A trusted controller must qualify every supplied fact/authorization.

pub mod api;
mod codec;
pub(crate) mod store;
pub use store::{Committed, Journal};

use crate::{Error, Head, model, onboarding, orders, wire};
use cinder_kernel::{identity::*, ledger::Config};
use std::fmt;

/// Initial composition bound, not an admission or scalability claim.
pub const MAX_UNITS: usize = 2;

/// Immutable physical/financial ownership. Commitments are not native evidence.
#[derive(Clone, PartialEq, Eq)]
pub struct Descriptor {
    /// Opaque configured unit; cannot be relabeled after genesis.
    pub id: UnitId,
    /// Commitment to the exact native environment, not an inferred URL alias.
    pub environment: [u8; 32],
    /// Exact native pooled account identity within venue/environment.
    pub native_account: [u8; 32],
    /// Unit-exclusive key-controlled quote funding wallet.
    pub funding_wallet: [u8; 32],
    /// Existing-program configuration/paid-counter domain.
    pub custody_config: [u8; 32],
    /// Unit-exclusive program-controlled return/payout vault.
    pub payout_vault: [u8; 32],
    /// Commitment to separate native key-role instances and release policy.
    pub key_policy: [u8; 32],
    /// Existing exact units, source scopes, policy and empty initial books.
    pub ledger: Config,
}
impl fmt::Debug for Descriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Descriptor([PRIVATE])")
    }
}

/// Trusted immutable genesis registry. No active-state/snapshot import.
#[derive(Clone, PartialEq, Eq)]
pub struct Configuration {
    domain: Domain,
    units: Vec<Descriptor>,
}
impl fmt::Debug for Configuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Configuration([PRIVATE])")
    }
}
impl Configuration {
    /// Validate and canonically order one or two empty units. These checks do
    /// not qualify native addresses, fee aggregation or actual margin isolation.
    pub fn new(domain: Domain, mut units: Vec<Descriptor>) -> Result<Self, Error> {
        if units.is_empty() || units.len() > MAX_UNITS {
            return Err(Error::Limit);
        }
        units.sort_by_key(|u| u.id);
        for (i, u) in units.iter().enumerate() {
            if u.ledger.domain != domain
                || !u.ledger.customers.is_empty()
                || u.funding_wallet == u.payout_vault
                || [
                    u.environment,
                    u.native_account,
                    u.funding_wallet,
                    u.custody_config,
                    u.payout_vault,
                    u.key_policy,
                ]
                .contains(&[0; 32])
            {
                return Err(Error::Invalid);
            }
            model::State::new(u.ledger.clone())?;
            for old in &units[..i] {
                if old.id == u.id
                    || (old.ledger.venue == u.ledger.venue
                        && old.environment == u.environment
                        && old.native_account == u.native_account)
                    || old.funding_wallet == u.funding_wallet
                    || old.funding_wallet == u.payout_vault
                    || old.payout_vault == u.funding_wallet
                    || old.custody_config == u.custody_config
                    || old.payout_vault == u.payout_vault
                    || old.key_policy == u.key_policy
                    || old.ledger.sources.iter().any(|a| {
                        u.ledger.sources.iter().any(|b| {
                            a.scope.venue == b.scope.venue && a.scope.account == b.scope.account
                        })
                    })
                {
                    return Err(Error::Conflict);
                }
            }
        }
        let c = Self { domain, units };
        codec::configuration(&c)?;
        Ok(c)
    }
    /// Independently expected deployment scope.
    pub fn domain(&self) -> Domain {
        self.domain
    }
    /// Canonical immutable descriptor inventory.
    pub fn units(&self) -> &[Descriptor] {
        &self.units
    }
    /// Exact canonical registry commitment, not native qualification.
    pub fn commitment(&self) -> Result<[u8; 32], Error> {
        use sha2::{Digest, Sha256};
        Ok(Sha256::digest(codec::configuration(self)?).into())
    }
}

/// Asset-specific recipient for a shared wallet owner.
#[derive(Clone, PartialEq, Eq)]
pub struct Recipient {
    /// Configured unit, exactly once in canonical registry order.
    pub unit: UnitId,
    /// Qualified immutable recipient for this unit's quote asset.
    pub tokens: [u8; 32],
}
impl fmt::Debug for Recipient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Recipient([PRIVATE])")
    }
}

/// Deployment-wide identity and revoke-all epoch, not a combined balance.
#[derive(Clone, PartialEq, Eq)]
pub struct Customer {
    /// Wallet-derived deployment account.
    pub account: AccountId,
    /// Owner verification key; retained only in protected history.
    pub wallet: [u8; 32],
    /// Current shared epoch; advancing it does not cancel existing orders.
    pub epoch: u64,
}
impl fmt::Debug for Customer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Customer([PRIVATE])")
    }
}

/// Durable ownership of a foundation reservation, including after release.
#[derive(Clone, PartialEq, Eq)]
pub struct Route {
    /// Original operation key; globally unique across units.
    pub request: RequestKey,
    /// Immutable owning component.
    pub unit: UnitId,
    /// Canonical reservation commitment, NOT a future signed-order digest.
    pub reservation_digest: [u8; 32],
}
impl fmt::Debug for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Route([PRIVATE])")
    }
}

/// New-version mutation; no legacy implicit/default unit routing.
#[derive(Clone, PartialEq, Eq)]
pub enum Change {
    /// Trusted API controller's authenticated record; never financial admission.
    Api(Box<api::Record>),
    /// Trusted controller's original record and one joined financial decision.
    Admit(Box<api::Admission>),
    /// Add an empty owner to every component, atomically, at epoch one.
    Enroll {
        /// Qualified owner's wallet verification key.
        wallet: [u8; 32],
        /// One recipient per configured unit, in registry order.
        recipients: Vec<Recipient>,
    },
    /// Global revoke-all after caller authentication; no automatic cancellation.
    AdvanceAuthority {
        /// Existing deployment account.
        account: AccountId,
        /// Strictly increasing epoch, never zero or a reset.
        epoch: u64,
    },
    /// Reuse existing fact/controller semantics inside exactly one component.
    Unit {
        /// Explicit immutable unit, bound in canonical transaction bytes.
        unit: UnitId,
        /// Inner id/head/time must exactly equal the outer transaction header.
        proposal: Box<model::Transaction>,
    },
}
impl fmt::Debug for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Change([PRIVATE])")
    }
}

/// One atomic deployment transition, not a native transaction or signature.
#[derive(Clone, PartialEq, Eq)]
pub struct Transaction {
    /// Unique identity in the shared history.
    pub id: model::CommitId,
    /// Exact shared CAS head.
    pub expected: Head,
    /// Trusted injected monotone time; no wall clock in transitions.
    pub at: u64,
    /// Shared mutation or one unit mutation.
    pub change: Change,
}
impl Transaction {
    /// Bind an existing proposal to a unit without inventing a second header.
    pub fn unit(unit: UnitId, proposal: model::Transaction) -> Self {
        Self {
            id: proposal.id,
            expected: proposal.expected,
            at: proposal.at,
            change: Change::Unit {
                unit,
                proposal: Box::new(proposal),
            },
        }
    }
}
impl fmt::Debug for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UnitTransaction([PRIVATE])")
    }
}

/// Replay-verifiable outcome; never an exposure or signing authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Receipt {
    /// A recorded-only intent or administrative record, never financial acceptance.
    Api,
    /// Durable financial decision, including refusal; never exposure authority.
    Admitted {
        /// Original owning unit.
        unit: UnitId,
        /// All-or-none controls; actual facts remain recorded on refusal.
        receipt: model::Receipt,
    },
    /// Successful atomic enrollment or shared authority advance.
    Shared,
    /// Original existing financial/control dispositions in the selected unit.
    Unit {
        /// Original unit; lookup cannot substitute another one.
        unit: UnitId,
        /// Inputs survive refused control subproposals.
        receipt: model::Receipt,
    },
}

/// One accepted financial state composed of unit components, not additive books.
#[derive(Clone, PartialEq, Eq)]
pub struct State {
    config: Configuration,
    units: Vec<(UnitId, model::State)>,
    customers: Vec<Customer>,
    routes: Vec<Route>,
    api: Vec<api::Accepted>,
    decisions: Vec<api::Decision>,
    now: u64,
}
impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DeploymentState([PRIVATE])")
    }
}
impl State {
    fn new(config: Configuration) -> Result<Self, Error> {
        let units = config
            .units
            .iter()
            .map(|u| model::State::new(u.ledger.clone()).map(|s| (u.id, s)))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            config,
            units,
            customers: vec![],
            routes: vec![],
            api: vec![],
            decisions: vec![],
            now: 0,
        })
    }
    /// Immutable registry of this accepted state; not a fresh native metadata cut.
    pub fn configuration(&self) -> &Configuration {
        &self.config
    }
    /// Component view; does not aggregate collateral or check current freshness.
    pub fn unit(&self, id: UnitId) -> Option<&model::State> {
        self.units.iter().find(|(u, _)| *u == id).map(|(_, s)| s)
    }
    /// Shared private owner/epoch inventory.
    pub fn customers(&self) -> &[Customer] {
        &self.customers
    }
    /// Original immutable operation ownership, even after release.
    pub fn route(&self, request: RequestKey) -> Option<&Route> {
        self.routes.iter().find(|r| r.request == request)
    }
    /// Monotone deployment time; inactive component time is not a fresh risk cut.
    pub fn logical_time(&self) -> u64 {
        self.now
    }
    /// Immutable private API records in this accepted state; not fresh authority.
    pub fn api_records(&self) -> &[api::Accepted] {
        &self.api
    }
    /// Immutable financial decisions, not native delivery capabilities.
    pub fn api_decisions(&self) -> &[api::Decision] {
        &self.decisions
    }

    fn advance(&self, tx: &Transaction) -> Result<(Self, Receipt), Error> {
        if tx.at < self.now {
            return Err(Error::Invalid);
        }
        let mut s = self.clone();
        let receipt = match &tx.change {
            Change::Api(record) => {
                s.capture_api(tx, record)?;
                Receipt::Api
            }
            Change::Admit(admission) => s.admit_api(tx, admission)?,
            Change::Enroll { wallet, recipients } => {
                if *wallet == [0; 32]
                    || recipients.len() != s.units.len()
                    || s.customers.len() >= wire::MAX_ITEMS
                    || s.customers.iter().any(|c| c.wallet == *wallet)
                {
                    return Err(Error::Invalid);
                }
                let account = onboarding::account(s.config.domain, *wallet)?;
                for ((unit, state), recipient) in s.units.iter_mut().zip(recipients) {
                    if *unit != recipient.unit {
                        return Err(Error::Invalid);
                    }
                    let inner = shared_proposal(
                        tx,
                        vec![
                            model::Control::Onboarding(onboarding::Action::Register(
                                onboarding::Binding {
                                    account,
                                    wallet: *wallet,
                                    tokens: recipient.tokens,
                                },
                            )),
                            model::Control::Order(orders::Action::AdvanceAuthority {
                                account,
                                epoch: 1,
                            }),
                        ],
                    );
                    let (next, result) = state.advance(&inner)?;
                    if result.controls.is_some() {
                        return Err(Error::Invalid);
                    }
                    *state = next;
                }
                s.customers.push(Customer {
                    account,
                    wallet: *wallet,
                    epoch: 1,
                });
                Receipt::Shared
            }
            Change::AdvanceAuthority { account, epoch } => {
                s.advance_authority(tx, *account, *epoch)?;
                Receipt::Shared
            }
            Change::Unit { unit, proposal } => {
                if proposal.id != tx.id || proposal.expected != tx.expected || proposal.at != tx.at
                {
                    return Err(Error::Invalid);
                }
                let index = s
                    .units
                    .iter()
                    .position(|(u, _)| u == unit)
                    .ok_or(Error::Invalid)?;
                let routes = s.control_routes(*unit, proposal)?;
                let mut facts;
                let inner = if routes.is_none() {
                    facts = (**proposal).clone();
                    facts.controls.clear();
                    &facts
                } else {
                    proposal
                };
                let (next, mut result) = s.units[index]
                    .1
                    .advance_with_scope(inner, |input| s.input_owned(*unit, input))?;
                if let Some(routes) = routes {
                    if result.controls.is_none() {
                        s.routes = routes;
                    }
                } else {
                    result.controls = Some(model::ControlError::Invalid);
                }
                s.units[index].1 = next;
                Receipt::Unit {
                    unit: *unit,
                    receipt: result,
                }
            }
        };
        s.now = tx.at;
        Ok((s, receipt))
    }

    fn input_owned(&self, unit: UnitId, input: &model::Input) -> bool {
        use cinder_kernel::ledger::{
            Change, FillTarget, close::CloseChange, economics::EconomicChange, funds::FundsChange,
            protection::ProtectionChange, restoration::RestorationChange,
        };
        let owns = |request| {
            self.route(request).is_some_and(|r| r.unit == unit)
                || self.decisions.iter().any(|d| {
                    d.request == request
                        && d.receipt.controls.is_none()
                        && self
                            .api
                            .iter()
                            .any(|a| a.record.request == request && a.record.unit == unit)
                })
        };
        let target = |target: &FillTarget| match target {
            FillTarget::Customer(key) => owns(key.request),
            FillTarget::House | FillTarget::Unattributed => true,
        };
        let Some(event) = &input.event else {
            return true;
        };
        if !match &event.key {
            RecordKey::Request(r) => owns(*r),
            RecordKey::Attempt(a) => owns(a.request),
            RecordKey::Economic(_) => true,
        } {
            return false;
        }
        match &event.change {
            Change::Fill { target: t, .. }
            | Change::Economics(EconomicChange::Execution { target: t, .. }) => target(t),
            Change::Funds(FundsChange::Authorize(m)) => owns(m.attempt.request),
            Change::Funds(FundsChange::Observe { attempt, .. })
            | Change::Close(CloseChange::Execution { attempt, .. })
            | Change::Restoration(
                RestorationChange::Receipt { attempt, .. }
                | RestorationChange::Execution { attempt, .. },
            ) => owns(attempt.request),
            Change::Protection(ProtectionChange::Commit { claim, .. }) => owns(*claim),
            Change::Protection(ProtectionChange::Absorb { allocations }) => {
                allocations.iter().all(|a| owns(a.claim))
            }
            _ => true,
        }
    }

    // Tentative routing is published only if every existing controller succeeds.
    fn control_routes(
        &self,
        unit: UnitId,
        tx: &model::Transaction,
    ) -> Result<Option<Vec<Route>>, Error> {
        let mut routes = self.routes.clone();
        for control in &tx.controls {
            match control {
                model::Control::Reserve { request, .. } => {
                    if request.domain != self.config.domain
                        || !self.customers.iter().any(|c| c.account == request.account)
                        || self.api.iter().any(|r| r.record.request == *request)
                    {
                        return Ok(None);
                    }
                    let digest = codec::reservation(control)?;
                    if let Some(old) = routes.iter().find(|r| r.request == *request) {
                        if old.unit != unit || old.reservation_digest != digest {
                            return Ok(None);
                        }
                    } else {
                        if routes.len() >= wire::MAX_ITEMS {
                            return Ok(None);
                        }
                        routes.push(Route {
                            request: *request,
                            unit,
                            reservation_digest: digest,
                        });
                    }
                }
                model::Control::Release(request) => {
                    if !routes
                        .iter()
                        .any(|r| r.unit == unit && r.request == *request)
                    {
                        return Ok(None);
                    }
                }
                model::Control::Prepare {
                    key,
                    authority_epoch,
                    ..
                } => {
                    if !routes
                        .iter()
                        .any(|r| r.unit == unit && r.request == key.request)
                        || !self.customers.iter().any(|c| {
                            c.account == key.request.account && c.epoch == *authority_epoch
                        })
                    {
                        return Ok(None);
                    }
                }
                model::Control::Onboarding(onboarding::Action::Notice(_))
                | model::Control::ResolveRaw(_)
                | model::Control::Collateral(_)
                | model::Control::Risk(crate::risk::Action::Install { .. }) => (),
                _ => return Ok(None),
            }
        }
        Ok(Some(routes))
    }
    fn advance_authority(
        &mut self,
        tx: &Transaction,
        account: AccountId,
        epoch: u64,
    ) -> Result<(), Error> {
        let owner = self
            .customers
            .iter_mut()
            .find(|c| c.account == account)
            .ok_or(Error::Invalid)?;
        if epoch <= owner.epoch {
            return Err(Error::Invalid);
        }
        for (_, state) in &mut self.units {
            let inner = shared_proposal(
                tx,
                vec![model::Control::Order(orders::Action::AdvanceAuthority {
                    account,
                    epoch,
                })],
            );
            let (next, result) = state.advance(&inner)?;
            if result.controls.is_some() {
                return Err(Error::Invalid);
            }
            *state = next;
        }
        owner.epoch = epoch;
        Ok(())
    }
}

fn shared_proposal(tx: &Transaction, controls: Vec<model::Control>) -> model::Transaction {
    model::Transaction {
        id: tx.id,
        expected: tx.expected,
        at: tx.at,
        evidence: vec![],
        inputs: vec![],
        order_observations: vec![],
        funds_observations: vec![],
        controls,
    }
}
