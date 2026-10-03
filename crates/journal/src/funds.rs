//! FIFO funds controller over shared holdings and the kernel's partial movements.
//! Native rail dispatch and qualified no-later-execution evidence remain ports.
use crate::{
    Error,
    model::*,
    orders::Approval,
    wire::{self, MAX_ITEMS, Reader, Writer},
};
use cinder_kernel::{
    amounts::*,
    identity::*,
    ledger::{evidence::Disposition, funds::*, *},
};
use sha2::{Digest, Sha256};

/// Immutable requested payout/transfer. Actual partial dispatch needs explicit consent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intent {
    /// Owner, deployment and durable operation identity.
    pub request: RequestKey,
    /// Supported physical source.
    pub source: Location,
    /// Internal destination or exact authorized beneficiary.
    pub destination: Destination,
    /// Requested net amount; no perp notional is inferred.
    pub net: QuoteAtoms,
    /// Authorized rail-fee cap, separately reserved.
    pub maximum_fee: QuoteAtoms,
    /// House or the authenticated customer; not other customers or suspense.
    pub fee_payer: Owner,
    /// Allows intentionally dispatching less than requested. Actual partial receipts
    /// are always recorded even without consent, then contained as a broken promise.
    pub allow_partial: bool,
    /// Economic policy revision.
    pub policy: PolicyVersion,
    /// Verified customer grant epoch, not a witness lease.
    pub authority_epoch: u64,
    /// Dispatch expiry; elapsed time cannot release an unknown attempt.
    pub expires_at: u64,
}
impl Intent {
    /// Domain-separated exact instruction hash for the trusted authentication port.
    pub fn digest(&self) -> Result<[u8; 32], Error> {
        let mut w = Writer::new(30);
        encode_intent(&mut w, self);
        Ok(Sha256::digest(w.finish()?).into())
    }
}

/// Independently qualified causal coverage for each source of a cross-location move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coverage {
    /// Full source namespace, not merely transport name or venue symbol.
    pub source: EventScope,
    /// Verified coverage in this source's own causal units.
    pub through: u64,
}
/// Full-history/no-later-execution witness supplied by the qualified funding port.
/// A response timeout, old empty page or arbitrary hash is not this proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Terminal {
    /// Exact original attempted movement.
    pub attempt: AttemptKey,
    /// Confirmed total source debit, including fees.
    pub debit: QuoteAtoms,
    /// Confirmed total settlement/return/impairment; must equal debit to close.
    pub settled: QuoteAtoms,
    /// Complete immutable economic leg set.
    pub receipts: Vec<EventKey>,
    /// Separate source cuts; no invented global Solana/venue sequence.
    pub coverage: Vec<Coverage>,
    /// Reference to retained authenticated capability-expiry/fencing evidence.
    /// P16 must validate its substance; nonzero alone is NOT authentication.
    pub no_later_execution: [u8; 32],
}
/// Raw qualified completion evidence, independent of ordinary-control mode.
#[derive(Clone, PartialEq, Eq)]
pub struct Observation {
    /// Immutable certificate identity in a configured trusted source namespace.
    pub key: EventKey,
    /// Bound witness, not an instruction to pay anyone.
    pub terminal: Terminal,
    /// Source qualification epoch.
    pub authority_epoch: u64,
    /// Original injected receipt time.
    pub observed_at: u64,
    /// Exact sanitized evidence body; no credentials.
    pub raw: PrivateBytes,
}
impl std::fmt::Debug for Observation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FundsObservation([PRIVATE])")
    }
}

/// Funds controls, committed atomically with holds and postings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Authenticated recovery operator's return of existing, gross-settled assets.
    /// Only Venue -> Broker or Broker -> Vault; never deposits or beneficiary payouts.
    RecoveryAccept {
        /// Same immutable economic intent; fees must remain house-owned.
        intent: Box<Intent>,
        /// Trusted recovery-operator digest, not a fabricated customer signature.
        approval: Approval,
    },
    /// Persist the requested economics and its customer/fee commitments.
    Accept {
        /// Exact immutable instruction.
        intent: Box<Intent>,
        /// Authenticated scoped digest; verified by P18's port.
        approval: Approval,
    },
    /// Choose an explicitly consented amount, reserve source liquidity, and persist
    /// the exact bounded abstract dispatch before any signing can occur.
    Prepare {
        /// Distinct attempt under the accepted request.
        attempt: AttemptKey,
        /// Full net request unless partial dispatch was allowed.
        net: QuoteAtoms,
    },
    /// Cancel only before capability exposure, retaining identity/history.
    CancelUnexposed(RequestKey),
    /// Close after qualified terminal evidence and exact applied receipt set.
    Finalize(RequestKey),
    /// Restrict ordinary new exposure; receipt accounting and scoped cancels continue.
    Freeze,
    /// Trusted funding-qualification port: fence native risk while collateral or
    /// account setup is uncertain. This does not authorize a transfer or payout.
    NativeCreditReady(bool),
}

/// Queue/lifecycle metadata, not another asset or claim ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    /// Recovery-only return; cannot revive an ordinary prepared capability.
    pub recovery: bool,
    /// Requested economics; vector insertion order is durable queue order.
    pub intent: Intent,
    /// At most one dispatch attempt in this phase.
    pub attempt: Option<AttemptKey>,
    /// Complete qualified history, possibly received before its legs.
    pub proof: Option<Terminal>,
    /// Authorization safely closed or cancelled before exposure.
    pub terminal: bool,
    /// Contradictory/invalid lifecycle evidence; never silently reset.
    pub faulted: bool,
}

impl State {
    /// Durable FIFO operation order for this pool/asset; no caller-side reordering.
    pub fn funds(&self) -> &[Operation] {
        &self.funds
    }
    /// Explicit ordinary-path freeze, independent of receipt ingestion.
    pub fn frozen(&self) -> bool {
        self.frozen
    }

    fn funds_ready(&self, recovery: bool) -> Result<(), ControlError> {
        if recovery {
            if self.recovery_epoch().is_none()
                || !self.recovery_flat_returns()
                || self.raw_unresolved != 0
                || self.funds.iter().any(|o| o.faulted)
                || self.orders.iter().any(|o| !o.complete())
                || self.ledger.unresolved_attribution() != 0
                || self.ledger.unresolved_funds() != 0
                || self.ledger.issues().iter().any(|i| i.open)
            {
                return Err(ControlError::Unqualified);
            }
            return self.recovery_return_capacity();
        }
        self.protection_ready()?;
        if self.frozen
            || self.raw_unresolved != 0
            || self.funds.iter().any(|o| o.faulted)
            || self.orders.iter().any(|o| o.faulted)
            || self.ledger.unresolved_attribution() != 0
            || self.ledger.unresolved_funds() != 0
        {
            return Err(ControlError::Unqualified);
        }
        if self.risk.is_some() {
            self.risk_gate()?;
        } else {
            self.collateral_capacity(Resource::Location(Location::Vault))?;
        }
        Ok(())
    }

    fn funds_turn(&self, index: usize) -> Result<(), ControlError> {
        let o = &self.funds[index];
        if matches!(o.intent.destination, Destination::Recipient(_))
            && self.funds[..index].iter().any(|old| {
                !old.terminal && matches!(old.intent.destination, Destination::Recipient(_))
            })
        {
            return Err(ControlError::Unqualified);
        }
        if self.funds.iter().enumerate().any(|(i, old)| {
            i != index
                && old.intent.source == o.intent.source
                && !old.terminal
                && old.attempt.is_some()
        }) {
            return Err(ControlError::Exposed);
        }
        Ok(())
    }

    pub(crate) fn funds_action(&mut self, action: &Action) -> Result<(), ControlError> {
        match action {
            Action::Freeze => self.frozen = true,
            Action::NativeCreditReady(ready) => self.native_funding_ready = *ready,
            Action::Accept {
                intent: i,
                approval,
            }
            | Action::RecoveryAccept {
                intent: i,
                approval,
            } => {
                let recovery = matches!(action, Action::RecoveryAccept { .. });
                self.funds_ready(recovery)?;
                if recovery
                    && (self.recovery_epoch() != Some(i.authority_epoch)
                        || i.fee_payer != Owner::House
                        || !matches!(
                            (i.source, i.destination),
                            (Location::Venue, Destination::Location(Location::Broker))
                                | (Location::Broker, Destination::Location(Location::Vault))
                        ))
                {
                    return Err(ControlError::Unqualified);
                }
                self.request(i.request)?;
                if self.funds.len() >= MAX_ITEMS
                    || self.funds.iter().any(|o| o.intent.request == i.request)
                    || i.policy != self.config.policy
                    || i.net.unit() != self.config.quote
                    || i.maximum_fee.unit() != self.config.quote
                    || i.net.atoms() <= 0
                    || i.maximum_fee.atoms() < 0
                    || i.destination == Destination::Location(i.source)
                    || i.destination == Destination::Recipient([0; 32])
                    || !matches!(i.fee_payer, Owner::House | Owner::Customer(_))
                    || matches!(i.fee_payer,Owner::Customer(id) if id!=i.request.account)
                    || matches!(i.destination, Destination::Location(_))
                        && i.fee_payer != Owner::House
                    || !recovery && self.authority(i.request.account) != Some(i.authority_epoch)
                    || approval.account != i.request.account
                    || approval.authority_epoch != i.authority_epoch
                    || approval.intent_hash != i.digest().map_err(|_| ControlError::Invalid)?
                {
                    return Err(ControlError::Invalid);
                }
                if i.expires_at <= self.now {
                    return Err(ControlError::Expired);
                }
                let cap = i
                    .net
                    .checked_add(i.maximum_fee)
                    .map_err(|_| ControlError::Invalid)?;
                let mut reservations = vec![];
                if matches!(i.destination, Destination::Recipient(_)) {
                    reservations.push(Reservation {
                        resource: Resource::Customer(i.request.account),
                        amount: if i.fee_payer == Owner::House {
                            i.net
                        } else {
                            cap
                        },
                    });
                } else {
                    reservations.push(Reservation {
                        resource: Resource::Location(i.source),
                        amount: cap,
                    });
                }
                if i.fee_payer == Owner::House && i.maximum_fee.atoms() > 0 {
                    reservations.push(Reservation {
                        resource: Resource::House,
                        amount: i.maximum_fee,
                    });
                }
                self.control(&Control::Reserve {
                    request: i.request,
                    reservations,
                })?;
                self.funds.push(Operation {
                    recovery,
                    intent: (**i).clone(),
                    attempt: None,
                    proof: None,
                    terminal: false,
                    faulted: false,
                });
            }
            Action::Prepare { attempt, net } => {
                let index = self
                    .funds
                    .iter()
                    .position(|o| o.intent.request == attempt.request)
                    .ok_or(ControlError::Invalid)?;
                let recovery = self.funds[index].recovery;
                self.funds_ready(recovery)?;
                self.funds_turn(index)?;
                let o = &self.funds[index];
                let i = o.intent.clone();
                if o.terminal
                    || o.attempt.is_some()
                    || net.unit() != i.net.unit()
                    || net.atoms() <= 0
                    || net.atoms() > i.net.atoms()
                    || !i.allow_partial && *net != i.net
                    || (if recovery {
                        self.recovery_epoch()
                    } else {
                        self.authority(i.request.account)
                    }) != Some(i.authority_epoch)
                {
                    return Err(ControlError::Invalid);
                }
                let cap = net
                    .checked_add(i.maximum_fee)
                    .map_err(|_| ControlError::Capacity)?;
                let hold = self
                    .holds
                    .iter_mut()
                    .find(|h| h.request == i.request && h.active)
                    .ok_or(ControlError::Invalid)?;
                if let Some(r) = hold
                    .reservations
                    .iter_mut()
                    .find(|r| r.resource == Resource::Location(i.source))
                {
                    r.amount = cap;
                } else {
                    hold.reservations.push(Reservation {
                        resource: Resource::Location(i.source),
                        amount: cap,
                    });
                }
                self.all_capacity()?;
                let mandate = Mandate {
                    attempt: *attempt,
                    source: i.source,
                    destination: i.destination,
                    net: *net,
                    maximum_fee: i.maximum_fee,
                    fee_payer: i.fee_payer,
                };
                let mut w = Writer::new(31);
                encode_intent(&mut w, &i);
                wire::mandate(&mut w, &mandate);
                self.prepare_attempt(
                    *attempt,
                    PrivateBytes::new(w.finish().map_err(|_| ControlError::Invalid)?)
                        .map_err(|_| ControlError::Invalid)?,
                    i.authority_epoch,
                    i.expires_at,
                    AttemptKind::Funds,
                )?;
                let e = Event {
                    key: RecordKey::Attempt(*attempt),
                    policy: i.policy,
                    change: Change::Funds(FundsChange::Authorize(mandate)),
                };
                let p = self
                    .ledger
                    .ingest(&e, self.now)
                    .map_err(|_| ControlError::Invalid)?;
                if p.disposition != Disposition::Applied {
                    return Err(ControlError::Invalid);
                }
                self.ledger = p.state;
                self.funds[index].attempt = Some(*attempt);
            }
            Action::CancelUnexposed(request) => {
                let o = self
                    .funds
                    .iter_mut()
                    .find(|o| o.intent.request == *request)
                    .ok_or(ControlError::Invalid)?;
                if o.faulted
                    || o.proof.is_some()
                    || self.ledger.movements().iter().any(|m| {
                        Some(m.mandate.attempt) == o.attempt
                            && (m.faulted || !m.receipts.is_empty())
                    })
                {
                    return Err(ControlError::Unqualified);
                }
                if o.terminal
                    || self
                        .attempts
                        .iter()
                        .any(|a| a.key.request == *request && a.possibly_exposed)
                {
                    return Err(ControlError::Exposed);
                }
                o.terminal = true;
                self.holds
                    .iter_mut()
                    .find(|h| h.request == *request)
                    .ok_or(ControlError::Invalid)?
                    .active = false;
            }
            Action::Finalize(request) => {
                let index = self
                    .funds
                    .iter()
                    .position(|o| o.intent.request == *request)
                    .ok_or(ControlError::Invalid)?;
                if self.funds[index].terminal || !self.funds_complete(index) {
                    return Err(ControlError::Unqualified);
                }
                self.funds[index].terminal = true;
                self.holds
                    .iter_mut()
                    .find(|h| h.request == *request)
                    .ok_or(ControlError::Invalid)?
                    .active = false;
            }
        }
        Ok(())
    }

    pub(crate) fn funds_exposure(&self, a: &Attempt) -> Result<(), ControlError> {
        if a.kind != AttemptKind::Funds {
            return Ok(());
        }
        let index = self
            .funds
            .iter()
            .position(|o| o.attempt == Some(a.key))
            .ok_or(ControlError::Invalid)?;
        let recovery = self.funds[index].recovery;
        self.funds_ready(recovery)?;
        self.funds_turn(index)?;
        if self.funds[index].terminal
            || self.funds[index].proof.is_some()
            || (if recovery {
                self.recovery_epoch()
            } else {
                self.authority(a.key.request.account)
            }) != Some(a.authority_epoch)
        {
            return Err(ControlError::Invalid);
        }
        Ok(())
    }

    fn funds_complete(&self, index: usize) -> bool {
        let o = &self.funds[index];
        let Some(t) = &o.proof else {
            return false;
        };
        let Some(m) = self
            .ledger
            .movements()
            .iter()
            .find(|m| Some(m.mandate.attempt) == o.attempt)
        else {
            return false;
        };
        !o.faulted
            && !m.faulted
            && m.debit == t.debit
            && m.settled == t.settled
            && m.debit == m.settled
            && m.receipts.len() == t.receipts.len()
            && m.receipts.iter().all(|k| {
                t.receipts.contains(k)
                    && self
                        .funds_receipts
                        .iter()
                        .find(|(id, _)| id == k)
                        .is_some_and(|(_, cut)| {
                            cut.is_some_and(|cut| {
                                t.coverage
                                    .iter()
                                    .any(|c| c.source == k.scope && cut <= c.through)
                            })
                        })
            })
    }

    pub(crate) fn funds_observe(&mut self, o: &Observation) -> Result<bool, Error> {
        let index = self
            .funds
            .iter()
            .position(|p| p.attempt == Some(o.terminal.attempt));
        let valid = o.authority_epoch != 0
            && o.observed_at <= self.now
            && self.config.sources.iter().any(|s| s.scope == o.key.scope)
            && self
                .attempts
                .iter()
                .any(|a| a.key == o.terminal.attempt && a.possibly_exposed)
            && index.is_some();
        let old = self.funds_observations.iter().find(|old| old.key == o.key);
        if valid && old.is_some_and(|old| old.terminal == o.terminal) {
            return Ok(true);
        }
        let conflict = old.is_some();
        if self.funds_observations.len() >= MAX_ITEMS {
            return Err(Error::Limit);
        }
        self.funds_observations.push(o.clone());
        if !valid || conflict {
            self.raw_unresolved = self.raw_unresolved.checked_add(1).ok_or(Error::Limit)?;
            if let Some(i) = index {
                self.funds[i].faulted = true;
            }
            return Ok(false);
        }
        let index = index.ok_or(Error::Invalid)?;
        let t = &o.terminal;
        let valid = t.no_later_execution != [0; 32]
            && t.debit.unit() == self.config.quote
            && t.settled.unit() == self.config.quote
            && t.debit.atoms() >= 0
            && t.debit == t.settled
            && t.receipts.len() <= MAX_ITEMS
            && t.coverage.len() <= MAX_ITEMS
            && t.coverage.iter().enumerate().all(|(i, c)| {
                self.config.sources.iter().any(|s| s.scope == c.source)
                    && !t.coverage[..i].iter().any(|old| old.source == c.source)
            })
            && t.receipts.iter().enumerate().all(|(i, k)| {
                !t.receipts[..i].contains(k) && t.coverage.iter().any(|c| c.source == k.scope)
            })
            && self.funds[index].proof.as_ref().is_none_or(|old| old == t);
        if !valid {
            self.funds[index].faulted = true;
            return Ok(false);
        }
        self.funds[index].proof = Some(t.clone());
        self.sync_funds()?;
        Ok(!self.funds[index].faulted)
    }

    pub(crate) fn funds_applied(&mut self, e: &Event, cut: Option<u64>) -> Result<(), Error> {
        if let Change::Funds(FundsChange::Observe { attempt, .. }) = &e.change {
            let RecordKey::Economic(k) = &e.key else {
                return Err(Error::Invalid);
            };
            if self.funds_receipts.len() >= MAX_ITEMS {
                return Err(Error::Limit);
            }
            self.funds_receipts.push((k.clone(), cut));
            if !self
                .attempts
                .iter()
                .any(|a| a.key == *attempt && a.possibly_exposed)
                && let Some(o) = self.funds.iter_mut().find(|o| o.attempt == Some(*attempt))
            {
                o.faulted = true;
            }
        }
        Ok(())
    }

    /// Consume only the commitment corresponding to actual settled/debited amounts.
    /// This does not rescale unrelated order/capital commitments or authorize risk.
    pub(crate) fn sync_funds(&mut self) -> Result<(), Error> {
        for o in &mut self.funds {
            let Some(m) = self
                .ledger
                .movements()
                .iter()
                .find(|m| Some(m.mandate.attempt) == o.attempt)
            else {
                continue;
            };
            o.faulted |= m.faulted
                || o.proof.as_ref().is_some_and(|t| {
                    m.debit.atoms() > t.debit.atoms()
                        || m.settled.atoms() > t.settled.atoms()
                        || m.receipts.iter().any(|k| {
                            !t.receipts.contains(k)
                                || self
                                    .funds_receipts
                                    .iter()
                                    .find(|(id, _)| id == k)
                                    .is_none_or(|(_, cut)| {
                                        cut.is_none_or(|cut| {
                                            !t.coverage
                                                .iter()
                                                .any(|c| c.source == k.scope && cut <= c.through)
                                        })
                                    })
                        })
                });
            // No consent cannot justify ignoring a real partial payment. Its exact
            // accounting survives, while this lifecycle stays contained for review.
            if o.proof.as_ref().is_some_and(|t| {
                m.debit == t.debit && m.settled == t.settled && m.receipts.len() == t.receipts.len()
            }) && m.debit == m.settled
                && !o.intent.allow_partial
                && matches!(o.intent.destination, Destination::Recipient(_))
                && m.paid.atoms() > 0
                && m.paid != o.intent.net
            {
                o.faulted = true;
            }
            let h = self
                .holds
                .iter_mut()
                .find(|h| h.request == o.intent.request)
                .ok_or(Error::Invalid)?;
            if o.faulted {
                h.active = true;
            }
            for r in &mut h.reservations {
                let remaining = match r.resource {
                    Resource::Customer(_) => o.intent.net.checked_sub(m.paid).and_then(|n| {
                        if matches!(o.intent.fee_payer, Owner::Customer(_)) {
                            n.checked_add(o.intent.maximum_fee.checked_sub(m.customer_fees)?)
                        } else {
                            Ok(n)
                        }
                    }),
                    Resource::House => o.intent.maximum_fee.checked_sub(m.fees),
                    Resource::Location(_) => m.source_remaining(),
                }
                .map_err(|_| Error::Limit)?;
                r.amount = QuoteAtoms::new(self.config.quote, remaining.atoms().max(0));
            }
        }
        Ok(())
    }
}

pub(crate) fn encode_intent(w: &mut Writer, i: &Intent) {
    w.item(&i.request);
    wire::location(w, i.source);
    wire::destination(w, i.destination);
    w.item(&i.net);
    w.item(&i.maximum_fee);
    wire::owner(w, i.fee_payer);
    w.byte(u8::from(i.allow_partial));
    w.raw(&i.policy.get().to_be_bytes());
    w.u64(i.authority_epoch);
    w.u64(i.expires_at);
}
pub(crate) fn decode_intent(r: &mut Reader<'_>) -> Result<Intent, Error> {
    Ok(Intent {
        request: r.item()?,
        source: wire::read_location(r)?,
        destination: wire::read_destination(r)?,
        net: r.item()?,
        maximum_fee: r.item()?,
        fee_payer: wire::read_owner(r)?,
        allow_partial: crate::orders::read_bool(r)?,
        policy: PolicyVersion::new(u32::from_be_bytes(r.array()?)).map_err(|_| Error::Codec)?,
        authority_epoch: r.u64()?,
        expires_at: r.u64()?,
    })
}
pub(crate) fn encode_terminal(w: &mut Writer, t: &Terminal) {
    w.item(&t.attempt);
    w.item(&t.debit);
    w.item(&t.settled);
    w.raw(&t.no_later_execution);
    w.count(t.receipts.len());
    for k in &t.receipts {
        w.item(k);
    }
    w.count(t.coverage.len());
    for c in &t.coverage {
        wire::scope(w, c.source);
        w.u64(c.through);
    }
}
pub(crate) fn decode_terminal(r: &mut Reader<'_>) -> Result<Terminal, Error> {
    let attempt = r.item()?;
    let debit = r.item()?;
    let settled = r.item()?;
    let no_later_execution = r.array()?;
    let mut receipts = vec![];
    for _ in 0..r.count()? {
        receipts.push(r.item()?);
    }
    let mut coverage = vec![];
    for _ in 0..r.count()? {
        coverage.push(Coverage {
            source: wire::read_scope(r)?,
            through: r.u64()?,
        });
    }
    Ok(Terminal {
        attempt,
        debit,
        settled,
        receipts,
        coverage,
        no_later_execution,
    })
}
pub(crate) fn encode_action(w: &mut Writer, a: &Action) {
    match a {
        Action::Accept { intent, approval } | Action::RecoveryAccept { intent, approval } => {
            w.byte(if matches!(a, Action::RecoveryAccept { .. }) {
                6
            } else {
                0
            });
            encode_intent(w, intent);
            w.raw(&approval.account.bytes());
            w.raw(&approval.intent_hash);
            w.u64(approval.authority_epoch);
        }
        Action::Prepare { attempt, net } => {
            w.byte(1);
            w.item(attempt);
            w.item(net);
        }
        Action::CancelUnexposed(k) => {
            w.byte(2);
            w.item(k);
        }
        Action::Finalize(k) => {
            w.byte(3);
            w.item(k);
        }
        Action::Freeze => w.byte(4),
        Action::NativeCreditReady(ready) => {
            w.byte(5);
            w.byte(u8::from(*ready));
        }
    }
}
pub(crate) fn decode_action(r: &mut Reader<'_>) -> Result<Action, Error> {
    match r.byte()? {
        0 => Ok(Action::Accept {
            intent: Box::new(decode_intent(r)?),
            approval: Approval {
                account: AccountId::new(r.array()?).map_err(|_| Error::Codec)?,
                intent_hash: r.array()?,
                authority_epoch: r.u64()?,
            },
        }),
        1 => Ok(Action::Prepare {
            attempt: r.item()?,
            net: r.item()?,
        }),
        2 => Ok(Action::CancelUnexposed(r.item()?)),
        3 => Ok(Action::Finalize(r.item()?)),
        4 => Ok(Action::Freeze),
        5 => Ok(Action::NativeCreditReady(r.bool()?)),
        6 => Ok(Action::RecoveryAccept {
            intent: Box::new(decode_intent(r)?),
            approval: Approval {
                account: AccountId::new(r.array()?).map_err(|_| Error::Codec)?,
                intent_hash: r.array()?,
                authority_epoch: r.u64()?,
            },
        }),
        _ => Err(Error::Codec),
    }
}
