//! Private order lifecycle joined to the authoritative journal. Source completeness
//! and customer signature verification are trusted ports, not inferred from an ACK.

use crate::{
    Error,
    model::*,
    wire::{self, MAX_ITEMS, Reader, Writer},
};
use cinder_kernel::{
    amounts::*,
    identity::*,
    ledger::{economics::EconomicChange, evidence::Disposition, *},
};
use sha2::{Digest, Sha256};

/// Exact native execution policy, part of customer authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeInForce {
    /// Bounded resting limit order.
    GoodTilCancelled,
    /// Add liquidity only; do not silently fall back to taking.
    AddLiquidityOnly,
    /// Bounded immediate execution; unfilled remainder must still be reconciled.
    ImmediateOrCancel,
}

/// Immutable bounded customer instruction; it is not a native pool reduce-only flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intent {
    /// Resting, post-only or bounded immediate execution semantics.
    pub time_in_force: TimeInForce,
    /// Private operation, owner and deployment binding.
    pub request: RequestKey,
    /// Signed total lots. Positive buys, negative sells.
    pub quantity: QuantityLots,
    /// Inclusive execution bounds; even immediate execution must be bounded.
    pub minimum: PriceTicks,
    /// Inclusive upper execution price.
    pub maximum: PriceTicks,
    /// Nonnegative quote atoms per executed lot; rebates are not promised income.
    pub maximum_fee_per_lot: QuoteAtoms,
    /// May reduce this private position, never flip it through zero.
    pub reduce_only: bool,
    /// Approved economic revision.
    pub policy: PolicyVersion,
    /// Customer grant epoch, independent of storage-writer/source authority epochs.
    pub authority_epoch: u64,
    /// Last admissible local dispatch time. Expiry cannot release escaped orders.
    pub expires_at: u64,
}

/// Bound result from the trusted customer-authentication port. It is NOT a
/// signature or a publicly acceptable bearer credential. P18 verifies that port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approval {
    /// Authenticated private account.
    pub account: AccountId,
    /// Domain-separated hash of every intent field.
    pub intent_hash: [u8; 32],
    /// Current verified grant revision.
    pub authority_epoch: u64,
}

impl Intent {
    /// Canonical authorization digest; never hash a display/rounded order.
    pub fn digest(&self) -> Result<[u8; 32], Error> {
        let mut w = Writer::new(20);
        encode_intent(&mut w, self);
        Ok(Sha256::digest(w.finish()?).into())
    }
}

/// Qualified final history witness from an adapter, NOT a cancel response. All
/// listed execution identities must already have posted before releasing a hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Terminal {
    /// Final executed quantity in the intent's signed units (zero allowed).
    pub filled: QuantityLots,
    /// Exact complete economic execution set, not order IDs or REST page IDs.
    pub executions: Vec<EventKey>,
    /// Qualified source coverage; a venue's largest seen ID is not this guarantee.
    pub through: u64,
}

/// Status changes do not post positions, cash or fees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// Venue accepted the request, without proving an execution.
    Acknowledged,
    /// Cancel accepted; fills/history may still be outstanding.
    CancelAcknowledged,
    /// Transport outcome unknown; no automatic rejection/resend.
    Unknown,
    /// Qualified complete execution history, possibly delivered before its fills.
    Terminal(Terminal),
}

/// Raw evidence retained in the encrypted transaction independently of controls.
#[derive(Clone, PartialEq, Eq)]
pub struct Observation {
    /// Scoped immutable native status identity.
    pub key: EventKey,
    /// Original dispatch whose lifecycle this concerns (not a cancel attempt).
    pub attempt: AttemptKey,
    /// Normalized semantics, excluding transport timestamps from duplicate equality.
    pub status: Status,
    /// Qualified source authority, not customer grant epoch.
    pub authority_epoch: u64,
    /// Injected observation time.
    pub observed_at: u64,
    /// Sanitized exact body, excluding credentials.
    pub raw: PrivateBytes,
}
impl std::fmt::Debug for Observation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OrderObservation([PRIVATE])")
    }
}

/// Control proposals are all-or-none with the journal's shared reservations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Trusted authentication controller installs a strictly increasing grant epoch.
    /// This does not revoke capabilities already escaped to a native venue.
    AdvanceAuthority {
        /// Authenticated private owner.
        account: AccountId,
        /// Strictly increasing, nonzero epoch.
        epoch: u64,
    },
    /// Persist authorized immutable intent and all shared resource commitments.
    Accept {
        /// Exact intent.
        intent: Box<Intent>,
        /// Authenticated binding supplied by the private runtime.
        approval: Approval,
        /// Shared customer/location/capital commitments.
        reservations: Vec<Reservation>,
    },
    /// Persist a canonical order dispatch action and its kernel allocation route.
    Prepare {
        /// New native attempt; at most one place attempt per intent in P07.
        attempt: AttemptKey,
    },
    /// Persist an attributed cancel for this order only. No cancel-all capability.
    PrepareCancel {
        /// Distinct cancel attempt under the same private request.
        attempt: AttemptKey,
        /// Current authenticated owner grant epoch.
        authority_epoch: u64,
        /// Independent cleanup dispatch expiry.
        expires_at: u64,
    },
    /// Explicitly release terminal pending commitment after complete applied history.
    /// Filled position risk remains in the ledger and later P09 admission envelope.
    Release {
        /// Original order operation.
        request: RequestKey,
    },
}

/// Immutable ownership plus lifecycle; no separate cash/position ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    /// Bound instruction.
    pub intent: Intent,
    /// Original place attempt, if prepared.
    pub attempt: Option<AttemptKey>,
    /// Signed applied quantity, including adverse fills retained in suspense.
    pub filled: QuantityLots,
    /// Applied execution identities and qualified per-input cuts (None is incomplete).
    pub executions: Vec<(EventKey, Option<u64>)>,
    /// Most recent normalized status flags; ACK is never terminal.
    pub acknowledged: bool,
    /// Cancel ACK alone never changes reserved resources.
    pub cancel_acknowledged: bool,
    /// At least one uncertain transport result was observed.
    pub unknown: bool,
    /// Immutable proposed final-history certificate; contradictions fault the order.
    pub terminal: Option<Terminal>,
    /// Attribution/terminal contradiction blocks new exposure and terminal release.
    pub faulted: bool,
    /// Prior order-bound violation prevents automatic attribution of further fills.
    /// A lifecycle-only contradiction does not erase an independently valid fill.
    pub bound_violated: bool,
}
impl Order {
    /// Remaining order lots, never a spendable-collateral calculation.
    pub fn remaining(&self) -> u64 {
        self.intent
            .quantity
            .lots()
            .unsigned_abs()
            .saturating_sub(self.filled.lots().unsigned_abs())
    }
    /// Complete identity set and exact quantity at the certified causal cut.
    pub fn complete(&self) -> bool {
        !self.faulted
            && self.terminal.as_ref().is_some_and(|t| {
                t.filled == self.filled
                    && t.executions.len() == self.executions.len()
                    && self.executions.iter().all(|(k, cut)| {
                        t.executions.contains(k) && cut.is_some_and(|c| c <= t.through)
                    })
            })
    }
}

impl State {
    /// Bound private order history. Reading does not grant dispatch authority.
    pub fn orders(&self) -> &[Order] {
        &self.orders
    }

    fn authority(&self, account: AccountId) -> Option<u64> {
        self.authorities
            .iter()
            .find(|(a, _)| *a == account)
            .map(|(_, e)| *e)
    }

    pub(crate) fn order_action(&mut self, action: &Action) -> Result<(), ControlError> {
        match action {
            Action::AdvanceAuthority { account, epoch } => {
                if *epoch == 0
                    || self.ledger.book(Owner::Customer(*account)).is_err()
                    || self.authority(*account).is_some_and(|old| *epoch <= old)
                {
                    return Err(ControlError::Invalid);
                }
                if let Some((_, e)) = self.authorities.iter_mut().find(|(a, _)| a == account) {
                    *e = *epoch;
                } else {
                    self.authorities.push((*account, *epoch));
                }
            }
            Action::Accept {
                intent: i,
                approval,
                reservations,
            } => {
                self.request(i.request)?;
                if self.orders.len() >= MAX_ITEMS
                    || self.orders.iter().any(|o| o.intent.request == i.request)
                    || i.quantity.lots() == 0
                    || i.minimum.unit() != i.quantity.unit()
                    || i.maximum.unit() != i.quantity.unit()
                    || i.minimum.ticks() > i.maximum.ticks()
                    || i.maximum_fee_per_lot.unit() != self.config.quote
                    || i.maximum_fee_per_lot.atoms() < 0
                    || i.policy != self.config.policy
                    || !self
                        .config
                        .markets
                        .iter()
                        .any(|m| m.unit() == i.quantity.unit())
                    || self.authority(i.request.account) != Some(i.authority_epoch)
                    || approval.account != i.request.account
                    || approval.authority_epoch != i.authority_epoch
                    || approval.intent_hash != i.digest().map_err(|_| ControlError::Invalid)?
                {
                    return Err(ControlError::Invalid);
                }
                if i.expires_at <= self.now {
                    return Err(ControlError::Expired);
                }
                if !reservations
                    .iter()
                    .any(|r| r.resource == Resource::Customer(i.request.account))
                    || !reservations
                        .iter()
                        .any(|r| r.resource == Resource::Location(Location::Venue))
                {
                    return Err(ControlError::Invalid);
                }
                // P09 replaces flat-only capacity with outcome-aware margin. Do not
                // silently weaken it in order to make a lifecycle test place risk.
                self.control(&Control::Reserve {
                    request: i.request,
                    reservations: reservations.clone(),
                })?;
                self.orders.push(Order {
                    intent: (**i).clone(),
                    attempt: None,
                    filled: QuantityLots::new(i.quantity.unit(), 0),
                    executions: vec![],
                    acknowledged: false,
                    cancel_acknowledged: false,
                    unknown: false,
                    terminal: None,
                    faulted: false,
                    bound_violated: false,
                });
            }
            Action::Prepare { attempt } => {
                let index = self
                    .orders
                    .iter()
                    .position(|o| o.intent.request == attempt.request)
                    .ok_or(ControlError::Invalid)?;
                let order = &self.orders[index];
                if order.attempt.is_some() || order.faulted {
                    return Err(ControlError::Invalid);
                }
                let intent = order.intent.clone();
                let mut w = Writer::new(21);
                w.byte(0);
                w.item(attempt);
                encode_intent(&mut w, &intent);
                self.prepare_attempt(
                    *attempt,
                    PrivateBytes::new(w.finish().map_err(|_| ControlError::Invalid)?)
                        .map_err(|_| ControlError::Invalid)?,
                    intent.authority_epoch,
                    intent.expires_at,
                    AttemptKind::Order,
                )?;
                let e = Event {
                    key: RecordKey::Attempt(*attempt),
                    policy: self.config.policy,
                    change: Change::BindExecution {
                        market: intent.quantity.unit(),
                        side: if intent.quantity.lots() > 0 {
                            Side::Buy
                        } else {
                            Side::Sell
                        },
                    },
                };
                let proposal = self
                    .ledger
                    .ingest(&e, self.now)
                    .map_err(|_| ControlError::Invalid)?;
                if proposal.disposition != Disposition::Applied {
                    return Err(ControlError::Invalid);
                }
                self.ledger = proposal.state;
                self.orders[index].attempt = Some(*attempt);
            }
            Action::PrepareCancel {
                attempt,
                authority_epoch,
                expires_at,
            } => {
                let order = self
                    .orders
                    .iter()
                    .find(|o| o.intent.request == attempt.request)
                    .ok_or(ControlError::Invalid)?;
                let original = order.attempt.ok_or(ControlError::Invalid)?;
                if !self
                    .attempts
                    .iter()
                    .any(|a| a.key == original && a.possibly_exposed)
                    || order.complete()
                    || self.authority(attempt.request.account) != Some(*authority_epoch)
                    || self
                        .attempts
                        .iter()
                        .any(|a| a.key.request == attempt.request && a.kind == AttemptKind::Cancel)
                {
                    return Err(ControlError::Invalid);
                }
                let mut w = Writer::new(21);
                w.byte(1);
                w.item(attempt);
                w.item(&original);
                w.u64(*authority_epoch);
                w.u64(*expires_at);
                self.prepare_attempt(
                    *attempt,
                    PrivateBytes::new(w.finish().map_err(|_| ControlError::Invalid)?)
                        .map_err(|_| ControlError::Invalid)?,
                    *authority_epoch,
                    *expires_at,
                    AttemptKind::Cancel,
                )?;
            }
            Action::Release { request } => {
                let order = self
                    .orders
                    .iter()
                    .find(|o| o.intent.request == *request)
                    .ok_or(ControlError::Invalid)?;
                if !order.complete()
                    || self.raw_unresolved != 0
                    || self.ledger.unresolved_attribution() != 0
                    || self.ledger.issues().iter().any(|i| i.open)
                {
                    return Err(ControlError::Unqualified);
                }
                self.holds
                    .iter_mut()
                    .find(|h| h.request == *request && h.active)
                    .ok_or(ControlError::Invalid)?
                    .active = false;
            }
        }
        Ok(())
    }

    pub(crate) fn prepare_attempt(
        &mut self,
        key: AttemptKey,
        message: PrivateBytes,
        authority_epoch: u64,
        expires_at: u64,
        kind: AttemptKind,
    ) -> Result<(), ControlError> {
        self.request(key.request)?;
        if self.attempts.len() >= MAX_ITEMS
            || message.as_bytes().is_empty()
            || message.as_bytes().len() > 65536
            || authority_epoch == 0
            || self.attempts.iter().any(|a| a.key == key)
            || !self
                .holds
                .iter()
                .any(|h| h.request == key.request && h.active)
        {
            return Err(ControlError::Invalid);
        }
        if expires_at <= self.now {
            return Err(ControlError::Expired);
        }
        self.attempts.push(Attempt {
            key,
            message,
            authority_epoch,
            expires_at,
            possibly_exposed: false,
            kind,
        });
        Ok(())
    }

    pub(crate) fn order_exposure(&self, attempt: &Attempt) -> Result<(), ControlError> {
        if attempt.kind == AttemptKind::Generic {
            return Ok(());
        }
        let order = self
            .orders
            .iter()
            .find(|o| o.intent.request == attempt.key.request)
            .ok_or(ControlError::Invalid)?;
        if self.authority(attempt.key.request.account) != Some(attempt.authority_epoch) {
            return Err(ControlError::Invalid);
        }
        if attempt.kind == AttemptKind::Order
            && (order.faulted
                || order.terminal.is_some()
                || self
                    .order_observations
                    .iter()
                    .any(|o| o.attempt == attempt.key))
        {
            return Err(ControlError::Unqualified);
        }
        if attempt.kind == AttemptKind::Order && order.intent.reduce_only {
            let current = self
                .ledger
                .book(Owner::Customer(attempt.key.request.account))
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
        }
        Ok(())
    }

    pub(crate) fn order_observe(&mut self, o: &Observation) -> Result<bool, Error> {
        let index = self
            .orders
            .iter()
            .position(|order| order.attempt == Some(o.attempt));
        let valid = o.authority_epoch != 0
            && o.observed_at <= self.now
            && self
                .config
                .sources
                .iter()
                .any(|s| s.scope == o.key.scope && s.location == Location::Venue)
            && self
                .attempts
                .iter()
                .any(|a| a.key == o.attempt && a.possibly_exposed)
            && index.is_some();
        let duplicate = self.order_observations.iter().find(|old| old.key == o.key);
        if valid && duplicate.is_some_and(|old| old.attempt == o.attempt && old.status == o.status)
        {
            return Ok(true);
        }
        let conflict = duplicate.is_some();
        if self.order_observations.len() >= MAX_ITEMS {
            return Err(Error::Limit);
        }
        self.order_observations.push(o.clone());
        if !valid || conflict {
            self.raw_unresolved = self.raw_unresolved.checked_add(1).ok_or(Error::Limit)?;
            if let Some(i) = index {
                self.fault_order(i);
            }
            return Ok(false);
        }
        let index = index.ok_or(Error::Invalid)?;
        let order = &mut self.orders[index];
        match &o.status {
            Status::Acknowledged => order.acknowledged = true,
            Status::CancelAcknowledged => order.cancel_acknowledged = true,
            Status::Unknown => order.unknown = true,
            Status::Terminal(t) => {
                let valid = t.filled.unit() == order.intent.quantity.unit()
                    && (t.filled.lots() == 0
                        || t.filled.lots().signum() == order.intent.quantity.lots().signum())
                    && t.filled.lots().unsigned_abs()
                        <= order.intent.quantity.lots().unsigned_abs()
                    && t.executions.len() <= MAX_ITEMS
                    && t.executions
                        .iter()
                        .enumerate()
                        .all(|(i, k)| k.scope == o.key.scope && !t.executions[..i].contains(k))
                    && order.terminal.as_ref().is_none_or(|old| old == t);
                if !valid {
                    self.fault_order(index);
                    return Ok(false);
                }
                order.terminal = Some(t.clone());
                if order
                    .executions
                    .iter()
                    .any(|(k, c)| !t.executions.contains(k) || c.is_none_or(|c| c > t.through))
                    || order.filled.lots().unsigned_abs() > t.filled.lots().unsigned_abs()
                {
                    self.fault_order(index);
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    fn fault_order(&mut self, index: usize) {
        self.orders[index].faulted = true;
        // A contradictory late fact re-encumbers even a previously released hold.
        // This can exceed capacity; actual events are not rejected as new admission.
        if let Some(h) = self
            .holds
            .iter_mut()
            .find(|h| h.request == self.orders[index].intent.request)
        {
            h.active = true;
        }
    }

    /// Classify only new economic fills. Already consumed identities reuse their
    /// original classification, so a REST/WS duplicate cannot change ownership.
    pub(crate) fn order_fill(&mut self, input: &Input, e: &Event) -> Result<Event, Error> {
        let (attempt, q, price, fee) = match &e.change {
            Change::Fill {
                target: FillTarget::Customer(a),
                quantity,
                price,
            } => (*a, *quantity, *price, QuoteAtoms::new(self.config.quote, 0)),
            Change::Economics(EconomicChange::Execution {
                target: FillTarget::Customer(a),
                quantity,
                price,
                fee,
                ..
            }) => (*a, *quantity, *price, *fee),
            _ => return Ok(e.clone()),
        };
        let Some(index) = self.orders.iter().position(|o| o.attempt == Some(attempt)) else {
            return Ok(e.clone());
        };
        if let Some((original, classified)) =
            self.order_fills.iter().find(|(old, _)| old.key == e.key)
        {
            if original == e {
                return Ok(classified.clone());
            }
            self.fault_order(index);
            // Original retained separately; kernel's conflict handling must not
            // disappear just because two rejected customer intents both map to suspense.
            self.raw_unresolved = self.raw_unresolved.checked_add(1).ok_or(Error::Limit)?;
            return Ok(e.clone());
        }
        if self.order_fills.len() >= MAX_ITEMS {
            return Err(Error::Limit);
        }
        let order = &self.orders[index];
        let key = if let RecordKey::Economic(k) = &e.key {
            k.clone()
        } else {
            self.fault_order(index);
            return Ok(e.clone());
        };
        let total = order.filled.lots().checked_add(q.lots());
        let valid = !order.bound_violated
            && e.policy == order.intent.policy
            && q.unit() == order.intent.quantity.unit()
            && price.unit() == q.unit()
            && fee.unit() == self.config.quote
            && q.lots() != 0
            && q.lots().signum() == order.intent.quantity.lots().signum()
            && total
                .is_some_and(|n| n.unsigned_abs() <= order.intent.quantity.lots().unsigned_abs())
            && price.ticks() >= order.intent.minimum.ticks()
            && price.ticks() <= order.intent.maximum.ticks()
            && order
                .intent
                .maximum_fee_per_lot
                .atoms()
                .checked_mul(i128::from(q.lots().unsigned_abs()))
                .is_some_and(|bound| fee.atoms() <= bound)
            && self
                .attempts
                .iter()
                .any(|a| a.key == attempt && a.possibly_exposed);
        let terminal_contradiction = order.terminal.as_ref().is_some_and(|t| {
            !t.executions.contains(&key)
                || input.source_cut.is_none_or(|c| c > t.through)
                || total.is_none_or(|n| n.unsigned_abs() > t.filled.lots().unsigned_abs())
        });
        let current = self
            .ledger
            .book(Owner::Customer(attempt.request.account))
            .map_err(|_| Error::Invalid)?
            .positions()
            .iter()
            .find(|p| p.quantity().unit() == order.intent.quantity.unit())
            .ok_or(Error::Invalid)?
            .quantity()
            .lots();
        let reduce_valid = !order.intent.reduce_only
            || (current.signum() != q.lots().signum()
                && current.unsigned_abs() >= q.lots().unsigned_abs());
        let mut classified = e.clone();
        if terminal_contradiction {
            self.fault_order(index);
        }
        if !valid || !reduce_valid {
            self.fault_order(index);
            self.orders[index].bound_violated = true;
            match &mut classified.change {
                Change::Fill { target, .. }
                | Change::Economics(EconomicChange::Execution { target, .. }) => {
                    *target = FillTarget::Unattributed
                }
                _ => unreachable!(),
            }
        }
        self.order_fills.push((e.clone(), classified.clone()));
        Ok(classified)
    }

    pub(crate) fn order_applied(
        &mut self,
        original: &Event,
        cut: Option<u64>,
    ) -> Result<(), Error> {
        let (attempt, q) = match &original.change {
            Change::Fill {
                target: FillTarget::Customer(a),
                quantity,
                ..
            }
            | Change::Economics(EconomicChange::Execution {
                target: FillTarget::Customer(a),
                quantity,
                ..
            }) => (*a, *quantity),
            _ => return Ok(()),
        };
        if let Some(order) = self.orders.iter_mut().find(|o| o.attempt == Some(attempt)) {
            let RecordKey::Economic(k) = &original.key else {
                return Err(Error::Invalid);
            };
            if q.unit() != order.filled.unit() {
                return Ok(());
            }
            order.filled = order.filled.checked_add(q).map_err(|_| Error::Limit)?;
            order.executions.push((k.clone(), cut));
        }
        Ok(())
    }
}

pub(crate) fn encode_intent(w: &mut Writer, i: &Intent) {
    w.byte(match i.time_in_force {
        TimeInForce::GoodTilCancelled => 0,
        TimeInForce::AddLiquidityOnly => 1,
        TimeInForce::ImmediateOrCancel => 2,
    });
    w.item(&i.request);
    w.item(&i.quantity);
    w.item(&i.minimum);
    w.item(&i.maximum);
    w.item(&i.maximum_fee_per_lot);
    w.byte(u8::from(i.reduce_only));
    w.raw(&i.policy.get().to_be_bytes());
    w.u64(i.authority_epoch);
    w.u64(i.expires_at);
}
pub(crate) fn decode_intent(r: &mut Reader<'_>) -> Result<Intent, Error> {
    Ok(Intent {
        time_in_force: match r.byte()? {
            0 => TimeInForce::GoodTilCancelled,
            1 => TimeInForce::AddLiquidityOnly,
            2 => TimeInForce::ImmediateOrCancel,
            _ => return Err(Error::Codec),
        },
        request: r.item()?,
        quantity: r.item()?,
        minimum: r.item()?,
        maximum: r.item()?,
        maximum_fee_per_lot: r.item()?,
        reduce_only: read_bool(r)?,
        policy: PolicyVersion::new(u32::from_be_bytes(r.array()?)).map_err(|_| Error::Codec)?,
        authority_epoch: r.u64()?,
        expires_at: r.u64()?,
    })
}
pub(crate) fn read_bool(r: &mut Reader<'_>) -> Result<bool, Error> {
    match r.byte()? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::Codec),
    }
}
pub(crate) fn encode_status(w: &mut Writer, s: &Status) {
    match s {
        Status::Acknowledged => w.byte(0),
        Status::CancelAcknowledged => w.byte(1),
        Status::Unknown => w.byte(2),
        Status::Terminal(t) => {
            w.byte(3);
            w.item(&t.filled);
            w.u64(t.through);
            w.count(t.executions.len());
            for k in &t.executions {
                w.item(k);
            }
        }
    }
}
pub(crate) fn decode_status(r: &mut Reader<'_>) -> Result<Status, Error> {
    match r.byte()? {
        0 => Ok(Status::Acknowledged),
        1 => Ok(Status::CancelAcknowledged),
        2 => Ok(Status::Unknown),
        3 => {
            let filled = r.item()?;
            let through = r.u64()?;
            let mut executions = vec![];
            for _ in 0..r.count()? {
                executions.push(r.item()?);
            }
            Ok(Status::Terminal(Terminal {
                filled,
                through,
                executions,
            }))
        }
        _ => Err(Error::Codec),
    }
}
pub(crate) fn encode_action(w: &mut Writer, a: &Action) {
    match a {
        Action::AdvanceAuthority { account, epoch } => {
            w.byte(0);
            w.raw(&account.bytes());
            w.u64(*epoch);
        }
        Action::Accept {
            intent,
            approval,
            reservations,
        } => {
            w.byte(1);
            encode_intent(w, intent);
            w.raw(&approval.account.bytes());
            w.raw(&approval.intent_hash);
            w.u64(approval.authority_epoch);
            wire::reservations(w, reservations);
        }
        Action::Prepare { attempt } => {
            w.byte(2);
            w.item(attempt);
        }
        Action::PrepareCancel {
            attempt,
            authority_epoch,
            expires_at,
        } => {
            w.byte(3);
            w.item(attempt);
            w.u64(*authority_epoch);
            w.u64(*expires_at);
        }
        Action::Release { request } => {
            w.byte(4);
            w.item(request);
        }
    }
}
pub(crate) fn decode_action(r: &mut Reader<'_>) -> Result<Action, Error> {
    match r.byte()? {
        0 => Ok(Action::AdvanceAuthority {
            account: AccountId::new(r.array()?).map_err(|_| Error::Codec)?,
            epoch: r.u64()?,
        }),
        1 => Ok(Action::Accept {
            intent: Box::new(decode_intent(r)?),
            approval: Approval {
                account: AccountId::new(r.array()?).map_err(|_| Error::Codec)?,
                intent_hash: r.array()?,
                authority_epoch: r.u64()?,
            },
            reservations: wire::read_reservations(r)?,
        }),
        2 => Ok(Action::Prepare { attempt: r.item()? }),
        3 => Ok(Action::PrepareCancel {
            attempt: r.item()?,
            authority_epoch: r.u64()?,
            expires_at: r.u64()?,
        }),
        4 => Ok(Action::Release { request: r.item()? }),
        _ => Err(Error::Codec),
    }
}
