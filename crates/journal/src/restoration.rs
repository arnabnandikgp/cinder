//! One qualified native risk unit, fixed RF2 attribution and funded RF1 recovery.
//! No native subaccount isolation or automatic live ADL classification is claimed.
use crate::{
    Error,
    model::*,
    orders::{Intent, Order, TimeInForce},
    wire::{Reader, Writer},
};
use cinder_kernel::{
    amounts::*,
    identity::*,
    ledger::{evidence::Disposition, restoration::RestorationChange as R, *},
    math::{Rounding, mul_div},
};
use sha2::{Digest, Sha256};

/// Exact administrator-authorized bounded replacement. Limits come from the
/// explicitly installed liquidation/depth policy, not arbitrary public inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    /// Durable operation/attempt; its private account is an incident label only.
    pub attempt: AttemptKey,
    /// Previously declared native forced-event identity.
    pub incident: EventKey,
    /// Positive frozen-schedule prefix to attempt.
    pub target: u64,
    /// Exact financial cut reviewed for admission.
    pub expected_version: u64,
    /// Immutable depth/price/fee revision approved by the operator.
    pub limit_revision: PolicyVersion,
    /// Immutable lifetime-cap revision approved by the operator.
    pub cap_revision: PolicyVersion,
    /// Current trusted operator authority epoch.
    pub authority_epoch: u64,
    /// Qualified source-time execution deadline.
    pub expires_at: u64,
}
impl Proposal {
    /// Binding expected from the authentication port, not a signature verifier.
    pub fn digest(&self) -> Result<[u8; 32], Error> {
        let mut w = Writer::new(70);
        encode_proposal(&mut w, self);
        Ok(Sha256::digest(w.finish()?).into())
    }
}
/// Lifecycle mutations. Real external effects enter as retained observations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Trusted administrator sets an explicit lifetime close/replacement cost
    /// cap. A new revision does not erase prior spend. Zero disables new support.
    Install {
        /// Exact financial state.
        expected_version: u64,
        /// Strictly increasing immutable policy revision.
        revision: PolicyVersion,
        /// Nonnegative nominal positive-cost limit; not calibrated by fixtures.
        lifetime_limit: QuoteAtoms,
    },
    /// Deterministic allocation of an already qualified cut, no operator weights.
    Declare {
        /// Unique internal declaration identity.
        request: RequestKey,
        /// Exact current financial cut.
        expected_version: u64,
        /// Observed forced execution.
        incident: EventKey,
    },
    /// Reserve capital and persist the exact bounded IOC before signature exposure.
    Prepare {
        /// Exact authenticated operator intent.
        proposal: Box<Proposal>,
        /// Trusted authentication-port binding, not an untrusted bearer field.
        authenticated_digest: [u8; 32],
    },
    /// A private user declines future restoration even if already fully reduced.
    Void {
        /// Authenticated customer and idempotency identity.
        request: RequestKey,
        /// Affected instrument.
        market: MarketUnit,
        /// Current private authorization epoch.
        authority_epoch: u64,
        /// Exact `void_digest` under the authenticated customer epoch.
        authenticated_digest: [u8; 32],
    },
}
/// Exact private-authentication binding for rejecting future replacement awards.
pub fn void_digest(request: RequestKey, market: MarketUnit, epoch: u64) -> Result<[u8; 32], Error> {
    let mut w = Writer::new(71);
    w.item(&request);
    w.item(&QuantityLots::new(market, 0));
    w.u64(epoch);
    Ok(Sha256::digest(w.finish()?).into())
}
/// Controller metadata; all economic quantities still live in the kernel ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replacement {
    /// Existing ordinary lifecycle request.
    pub request: RequestKey,
    /// Fixed source incident.
    pub incident: EventKey,
    /// Qualified depth-policy revision at preparation.
    pub revision: PolicyVersion,
    /// Lifetime-cap revision at admission.
    pub cap_revision: PolicyVersion,
    /// Original funded loss/cost envelope (not a guarantee under arbitrary shocks).
    pub budget: QuoteAtoms,
    /// Known actual exception; never cleared just because house later becomes flat.
    pub contained: bool,
}
impl State {
    /// Immutable replacement metadata, separate from economic ownership.
    pub fn restorations(&self) -> &[Replacement] {
        &self.restorations
    }
    pub(crate) fn restoration_event(
        &mut self,
        key: RecordKey,
        change: R,
    ) -> Result<(), ControlError> {
        let e = Event {
            key,
            policy: self.config.policy,
            change: Change::Restoration(change),
        };
        let next = self
            .ledger
            .ingest(&e, self.now)
            .map_err(|_| ControlError::Invalid)?;
        if next.disposition != Disposition::Applied {
            return Err(ControlError::Unqualified);
        }
        self.ledger = next.state;
        Ok(())
    }
    pub(crate) fn restoration_action(&mut self, a: &Action) -> Result<(), ControlError> {
        match a {
            Action::Install {
                expected_version,
                revision,
                lifetime_limit,
            } => {
                if *expected_version != self.ledger.version()
                    || lifetime_limit.unit() != self.config.quote
                    || lifetime_limit.atoms() < 0
                    || self
                        .restoration_limit
                        .as_ref()
                        .is_some_and(|(r, _)| *revision <= *r)
                {
                    return Err(ControlError::Invalid);
                }
                self.restoration_limit = Some((*revision, *lifetime_limit));
            }
            Action::Declare {
                request,
                expected_version,
                incident,
            } => {
                self.request(*request)?;
                if *expected_version != self.ledger.version()
                    || self.orders.iter().any(|o| !o.complete())
                    || self.raw_unresolved != 0
                {
                    return Err(ControlError::Unqualified);
                }
                self.restoration_event(
                    RecordKey::Request(*request),
                    R::Declare {
                        incident: incident.clone(),
                    },
                )?;
            }
            Action::Void {
                request,
                market,
                authority_epoch,
                authenticated_digest,
            } => {
                self.request(*request)?;
                if self.authority(request.account) != Some(*authority_epoch)
                    || void_digest(*request, *market, *authority_epoch)
                        .map_err(|_| ControlError::Invalid)?
                        != *authenticated_digest
                {
                    return Err(ControlError::Invalid);
                }
                self.restoration_event(RecordKey::Request(*request), R::Void { market: *market })?;
            }
            Action::Prepare {
                proposal: p,
                authenticated_digest,
            } => {
                self.request(p.attempt.request)?;
                if p.expected_version != self.ledger.version()
                    || p.digest().map_err(|_| ControlError::Invalid)? != *authenticated_digest
                    || self.frozen
                    || self.raw_unresolved != 0
                    || self.orders.iter().any(|o| !o.complete())
                    || self.restorations.iter().any(|r| r.contained)
                    || self.restorations.len() >= crate::wire::MAX_ITEMS
                {
                    return Err(ControlError::Unqualified);
                }
                self.risk_gate()?;
                let r = self
                    .ledger
                    .reductions()
                    .iter()
                    .find(|r| r.incident == p.incident)
                    .ok_or(ControlError::Invalid)?
                    .clone();
                if r.plan.is_none()
                    || r.attempt.is_some()
                    || r.rows.iter().any(|r| r.void)
                    || p.target == 0
                    || p.target > r.quantity.lots().unsigned_abs()
                    || self
                        .ledger
                        .protection()
                        .claims
                        .iter()
                        .any(|c| r.rows.iter().any(|r| r.owner == c.id.account))
                {
                    return Err(ControlError::Unqualified);
                }
                let limit = self.close_limit(r.quantity.unit())?.clone();
                let policy = self.liquidation.as_ref().ok_or(ControlError::Unqualified)?;
                let revision = policy.revision;
                if p.limit_revision != revision
                    || p.authority_epoch != policy.authority_epoch
                    || p.expires_at <= self.now
                    || p.expires_at > policy.valid_until
                    || p.target > limit.maximum_lots
                {
                    return Err(ControlError::Expired);
                }
                let side = -r.quantity.lots().signum();
                let qty = i64::try_from(i128::from(p.target) * i128::from(side))
                    .map_err(|_| ControlError::Capacity)?;
                let market = *self
                    .config
                    .markets
                    .iter()
                    .find(|m| m.unit() == limit.market)
                    .ok_or(ControlError::Invalid)?;
                let (n, d) = market.conversion();
                let gap = if side > 0 {
                    limit.maximum.ticks().saturating_sub(r.price.ticks())
                } else {
                    r.price.ticks().saturating_sub(limit.minimum.ticks())
                };
                let gap = mul_div(i128::from(p.target) * i128::from(gap), n, d, Rounding::Ceil)
                    .map_err(|_| ControlError::Capacity)?;
                // Full original fee is a conservative refund bound even for a
                // partial target. Two atoms per owner cover split rounding.
                let budget = gap
                    .checked_add(r.fee.atoms())
                    .and_then(|x| {
                        limit
                            .fee_per_lot
                            .atoms()
                            .checked_add(limit.additional_per_lot.atoms())
                            .and_then(|f| f.checked_mul(i128::from(p.target)))
                            .and_then(|v| x.checked_add(v))
                    })
                    .and_then(|x| x.checked_add(2 * r.rows.len() as i128))
                    .ok_or(ControlError::Capacity)?;
                let budget = QuoteAtoms::new(self.config.quote, budget.max(1));
                let (cap_revision, cap) =
                    self.restoration_limit.ok_or(ControlError::Unqualified)?;
                if p.cap_revision != cap_revision {
                    return Err(ControlError::Invalid);
                }
                if self
                    .restoration_committed_cost()?
                    .checked_add(budget.atoms())
                    .is_none_or(|n| n > cap.atoms())
                {
                    return Err(ControlError::Capacity);
                }
                self.control(&Control::Reserve {
                    request: p.attempt.request,
                    reservations: vec![Reservation {
                        resource: Resource::House,
                        amount: budget,
                    }],
                })?;
                let intent = Intent {
                    request: p.attempt.request,
                    quantity: QuantityLots::new(limit.market, qty),
                    minimum: limit.minimum,
                    maximum: limit.maximum,
                    maximum_fee_per_lot: limit.fee_per_lot,
                    reduce_only: false,
                    time_in_force: TimeInForce::ImmediateOrCancel,
                    policy: self.config.policy,
                    authority_epoch: p.authority_epoch,
                    expires_at: p.expires_at,
                };
                self.restorations.push(Replacement {
                    request: p.attempt.request,
                    incident: p.incident.clone(),
                    revision,
                    cap_revision,
                    budget,
                    contained: false,
                });
                self.orders.push(Order {
                    abandoned: false,
                    intent: intent.clone(),
                    attempt: Some(p.attempt),
                    filled: QuantityLots::new(limit.market, 0),
                    executions: vec![],
                    acknowledged: false,
                    cancel_acknowledged: false,
                    unknown: false,
                    terminal: None,
                    faulted: false,
                    bound_violated: false,
                });
                let mut w = Writer::new(72);
                encode_proposal(&mut w, p);
                crate::orders::encode_intent(&mut w, &intent);
                self.prepare_attempt(
                    p.attempt,
                    PrivateBytes::new(w.finish().map_err(|_| ControlError::Invalid)?)
                        .map_err(|_| ControlError::Invalid)?,
                    p.authority_epoch,
                    p.expires_at,
                    AttemptKind::Restoration,
                )?;
                self.restoration_event(
                    RecordKey::Attempt(p.attempt),
                    R::Bind {
                        incident: p.incident.clone(),
                        target: p.target,
                    },
                )?;
                self.all_capacity()?;
            }
        }
        Ok(())
    }
    pub(crate) fn restoration_exposure(&self, a: &Attempt) -> Result<(), ControlError> {
        if a.kind != AttemptKind::Restoration {
            return Ok(());
        }
        let m = self
            .restorations
            .iter()
            .find(|r| r.request == a.key.request)
            .ok_or(ControlError::Invalid)?;
        let policy = self.liquidation.as_ref().ok_or(ControlError::Unqualified)?;
        let (cap_revision, cap) = self.restoration_limit.ok_or(ControlError::Unqualified)?;
        if cap_revision != m.cap_revision || self.restoration_committed_cost()? > cap.atoms() {
            return Err(ControlError::Capacity);
        }
        let r = self
            .ledger
            .reductions()
            .iter()
            .find(|r| r.incident == m.incident)
            .ok_or(ControlError::Invalid)?;
        if policy.revision != m.revision
            || policy.authority_epoch != a.authority_epoch
            || policy.valid_until <= self.now
            || m.contained
            || r.rows.iter().any(|r| r.void)
        {
            return Err(ControlError::Unqualified);
        }
        Ok(())
    }
    fn restoration_committed_cost(&self) -> Result<i128, ControlError> {
        let spent = self
            .ledger
            .reductions()
            .iter()
            .map(|r| r.spent.atoms())
            .chain(self.ledger.closes().iter().map(|r| r.spent.atoms()));
        let held = self
            .holds
            .iter()
            .filter(|h| {
                h.active
                    && (self.restorations.iter().any(|r| r.request == h.request)
                        || self.closes.iter().any(|r| r.request == h.request))
            })
            .flat_map(|h| &h.reservations)
            .filter(|r| r.resource == Resource::House)
            .map(|r| r.amount.atoms());
        spent
            .chain(held)
            .try_fold(0i128, |a, b| a.checked_add(b).ok_or(ControlError::Capacity))
    }
}
fn encode_proposal(w: &mut Writer, p: &Proposal) {
    w.item(&p.attempt);
    w.item(&p.incident);
    w.u64(p.target);
    w.u64(p.expected_version);
    w.raw(&p.limit_revision.get().to_be_bytes());
    w.raw(&p.cap_revision.get().to_be_bytes());
    w.u64(p.authority_epoch);
    w.u64(p.expires_at);
}
pub(crate) fn encode_action(w: &mut Writer, a: &Action) {
    match a {
        Action::Install {
            expected_version,
            revision,
            lifetime_limit,
        } => {
            w.byte(3);
            w.u64(*expected_version);
            w.raw(&revision.get().to_be_bytes());
            w.item(lifetime_limit);
        }
        Action::Declare {
            request,
            expected_version,
            incident,
        } => {
            w.byte(0);
            w.item(request);
            w.u64(*expected_version);
            w.item(incident);
        }
        Action::Prepare {
            proposal,
            authenticated_digest,
        } => {
            w.byte(1);
            encode_proposal(w, proposal);
            w.raw(authenticated_digest);
        }
        Action::Void {
            request,
            market,
            authority_epoch,
            authenticated_digest,
        } => {
            w.byte(2);
            w.item(request);
            w.item(&QuantityLots::new(*market, 0));
            w.u64(*authority_epoch);
            w.raw(authenticated_digest);
        }
    }
}
pub(crate) fn decode_action(r: &mut Reader<'_>) -> Result<Action, Error> {
    Ok(match r.byte()? {
        3 => Action::Install {
            expected_version: r.u64()?,
            revision: PolicyVersion::new(u32::from_be_bytes(r.array()?))
                .map_err(|_| Error::Codec)?,
            lifetime_limit: r.item()?,
        },
        0 => Action::Declare {
            request: r.item()?,
            expected_version: r.u64()?,
            incident: r.item()?,
        },
        1 => Action::Prepare {
            proposal: Box::new(Proposal {
                attempt: r.item()?,
                incident: r.item()?,
                target: r.u64()?,
                expected_version: r.u64()?,
                limit_revision: PolicyVersion::new(u32::from_be_bytes(r.array()?))
                    .map_err(|_| Error::Codec)?,
                cap_revision: PolicyVersion::new(u32::from_be_bytes(r.array()?))
                    .map_err(|_| Error::Codec)?,
                authority_epoch: r.u64()?,
                expires_at: r.u64()?,
            }),
            authenticated_digest: r.array()?,
        },
        2 => {
            let request = r.item()?;
            let q: QuantityLots = r.item()?;
            if q.lots() != 0 {
                return Err(Error::Codec);
            }
            Action::Void {
                request,
                market: q.unit(),
                authority_epoch: r.u64()?,
                authenticated_digest: r.array()?,
            }
        }
        _ => return Err(Error::Codec),
    })
}
pub(crate) fn encode_change(w: &mut Writer, c: &R) {
    match c {
        R::Observe {
            cut,
            quantity,
            price,
            fee,
            pnl,
        } => {
            w.byte(0);
            w.u64(*cut);
            w.item(quantity);
            w.item(price);
            w.item(fee);
            w.option(pnl, |w, p| crate::wire::pnl(w, *p));
        }
        R::Declare { incident } => {
            w.byte(1);
            w.item(incident);
        }
        R::Bind { incident, target } => {
            w.byte(2);
            w.item(incident);
            w.u64(*target);
        }
        R::Void { market } => {
            w.byte(3);
            w.item(&QuantityLots::new(*market, 0));
        }
        R::Execution {
            attempt,
            quantity,
            price,
            fee,
            pnl,
            eligible,
        } => {
            w.byte(4);
            w.item(attempt);
            w.item(quantity);
            w.item(price);
            w.item(fee);
            w.option(pnl, |w, p| crate::wire::pnl(w, *p));
            w.byte(u8::from(*eligible));
        }
        R::Receipt {
            attempt,
            quantity,
            price,
            fee,
            pnl,
            executed_at,
        } => {
            w.byte(5);
            w.item(attempt);
            w.item(quantity);
            w.item(price);
            w.item(fee);
            w.option(pnl, |w, p| crate::wire::pnl(w, *p));
            w.u64(*executed_at);
        }
    }
}
pub(crate) fn decode_change(r: &mut Reader<'_>) -> Result<R, Error> {
    Ok(match r.byte()? {
        0 => R::Observe {
            cut: r.u64()?,
            quantity: r.item()?,
            price: r.item()?,
            fee: r.item()?,
            pnl: r.option(crate::wire::read_pnl)?,
        },
        1 => R::Declare {
            incident: r.item()?,
        },
        2 => R::Bind {
            incident: r.item()?,
            target: r.u64()?,
        },
        3 => {
            let q: QuantityLots = r.item()?;
            if q.lots() != 0 {
                return Err(Error::Codec);
            }
            R::Void { market: q.unit() }
        }
        4 => R::Execution {
            attempt: r.item()?,
            quantity: r.item()?,
            price: r.item()?,
            fee: r.item()?,
            pnl: r.option(crate::wire::read_pnl)?,
            eligible: r.bool()?,
        },
        5 => R::Receipt {
            attempt: r.item()?,
            quantity: r.item()?,
            price: r.item()?,
            fee: r.item()?,
            pnl: r.option(crate::wire::read_pnl)?,
            executed_at: r.u64()?,
        },
        _ => return Err(Error::Codec),
    })
}
pub(crate) fn encode_state(w: &mut Writer, s: &State) {
    w.option(&s.restoration_limit, |w, (revision, limit)| {
        w.raw(&revision.get().to_be_bytes());
        w.item(limit);
    });
    w.count(s.restorations.len());
    for r in &s.restorations {
        w.item(&r.request);
        w.item(&r.incident);
        w.raw(&r.revision.get().to_be_bytes());
        w.raw(&r.cap_revision.get().to_be_bytes());
        w.item(&r.budget);
        w.byte(u8::from(r.contained));
    }
    w.count(s.ledger.reductions().len());
    for r in s.ledger.reductions() {
        w.item(&r.incident);
        w.item(&r.quantity);
        w.item(&r.price);
        w.item(&r.fee);
        w.byte(u8::from(r.qualified));
        w.option(&r.attempt, |w, a| w.item(a));
        w.u64(r.target);
        w.u64(r.consumed);
        w.u64(r.received);
        w.u64(r.excess);
        w.item(&r.spent);
        w.count(r.rows.len());
        for row in &r.rows {
            w.raw(&row.owner.bytes());
            w.u64(row.amount);
            w.item(&row.basis);
            w.item(&row.realized);
            w.item(&row.fee);
            w.u64(row.restored);
            w.byte(u8::from(row.void));
        }
        // The plan and private cut are deterministic derivatives of the fully
        // committed event history and pinned engine revision, not imported state.
    }
}
