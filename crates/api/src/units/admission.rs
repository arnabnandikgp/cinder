//! Offline financial controller. Acceptance is not native exposure or settlement.
use super::{
    history::{History, key, scope},
    *,
};
use cinder_journal::{
    funds,
    model::{self, CommitId, Control},
    orders, risk,
    units::api as retained,
};
use cinder_kernel::{
    amounts::*,
    ledger::{Location, Owner, funds::Destination},
};

/// Original financial decision; no native receipt, pooled ID or signing permit.
#[derive(Clone, PartialEq, Eq)]
pub struct Decided {
    /// Original signed intent, not refreshed retry authentication.
    pub request: Request,
    /// Original decision time.
    pub at: u64,
    /// None means the financial control succeeded; Some is a durable refusal.
    pub refusal: Option<model::ControlError>,
    /// True only for an authenticated lookup of an already decided operation.
    pub duplicate: bool,
}
impl fmt::Debug for Decided {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Decided([PRIVATE])")
    }
}

impl Authorizer {
    /// Authenticate then atomically join the original operation and financial
    /// controls. The hold port is trusted enclave code over the selected component;
    /// callers cannot supply holds. No retry, signer or native I/O occurs here.
    pub fn admit<B: Backend, P: Protection>(
        &self,
        journal: &mut units::Journal<B, P>,
        channel: &impl ConfidentialChannel,
        bytes: &PrivateBytes,
        admission: &impl crate::Admission,
        now: u64,
    ) -> Result<Decided, Error> {
        let history = History::load(journal, self)?;
        self.admit_at(journal, channel, bytes, admission, now, &history)
    }
    pub(super) fn admit_at<B: Backend, P: Protection>(
        &self,
        journal: &mut units::Journal<B, P>,
        channel: &impl ConfidentialChannel,
        bytes: &PrivateBytes,
        admission: &impl crate::Admission,
        now: u64,
        history: &History,
    ) -> Result<Decided, Error> {
        let checked = self.check_inner(journal, channel, bytes, history, now, false)?;
        if checked.head() != history.head {
            return Err(Error::Unavailable);
        }
        let retry = checked.request();
        let original = if let Some(old) = history.find(retry.body.account, retry.body.id) {
            if checked.principal() != Principal::Owner
                && retry.body.signer != old.request.body.signer
            {
                return Err(Error::Unauthorized);
            }
            if retry.unit != old.request.unit || retry.digest() != old.request.digest() {
                return Err(Error::Conflict);
            }
            &old.request
        } else {
            retry
        };
        let state = journal.state().map_err(|_| Error::Unavailable)?;
        if let Some(d) = state
            .api_decisions()
            .iter()
            .find(|d| d.request == key(original))
        {
            return Ok(Decided {
                request: original.clone(),
                at: d.at,
                refusal: d.receipt.controls,
                duplicate: true,
            });
        }
        let owner = state
            .customers()
            .iter()
            .find(|c| c.account == original.body.account)
            .ok_or(Error::Unavailable)?;
        if original.body.epoch != owner.epoch {
            return Err(Error::Unauthorized);
        }
        let descriptor = state
            .configuration()
            .units()
            .iter()
            .find(|d| d.id == original.unit)
            .ok_or(Error::Unavailable)?;
        // An owner recovering an agent's staged request must still honor its
        // original scope/expiry/budget, rather than turn it into an owner order.
        original_authority(original, history, owner.wallet, now)?;
        if !matches!(original.body.command, Command::Cancel { .. })
            && !state
                .unit(original.unit)
                .ok_or(Error::Unavailable)?
                .has_joined_risk()
        {
            return Err(Error::Unavailable);
        }
        let (kind, mut control) = control(original, &descriptor.ledger, now)?;
        let component = state.unit(original.unit).ok_or(Error::Unavailable)?;
        match &mut control {
            Control::Funds(funds::Action::Accept { intent, approval }) => {
                intent.destination = Destination::Recipient(
                    component
                        .registrations()
                        .iter()
                        .find(|r| r.account == original.body.account)
                        .ok_or(Error::Unavailable)?
                        .tokens,
                );
                approval.intent_hash = intent.digest().map_err(|_| Error::Invalid)?;
            }
            Control::Order(orders::Action::Accept {
                intent,
                reservations,
                ..
            }) => {
                if original.body.signer != owner.wallet {
                    let cut = history
                        .lookup(scope(original), original.body.signer, history.head)?
                        .ok_or(Error::Unauthorized)?;
                    if cut.used_orders >= cut.grant.maximum_orders {
                        return Err(Error::Unauthorized);
                    }
                }
                *reservations = admission.order_holds(component, intent)?;
            }
            Control::Release(target) => {
                let old = history
                    .find(target.account, target.request)
                    .filter(|r| r.request.unit == original.unit)
                    .ok_or(Error::NotFound)?;
                let Command::Order { market, .. } = old.request.body.command else {
                    return Err(Error::Invalid);
                };
                if original.body.signer != owner.wallet
                    && (old.request.body.signer != original.body.signer
                        || !history.grants.iter().any(|(s, g)| {
                            *s == scope(original)
                                && g.grant.key == original.body.signer
                                && g.grant.market == market
                        }))
                {
                    return Err(Error::Unauthorized);
                }
                let order = component
                    .orders()
                    .iter()
                    .find(|o| o.intent.request == *target)
                    .ok_or(Error::NotFound)?;
                if order.complete() {
                    return Err(Error::Invalid);
                }
                if component
                    .attempts()
                    .iter()
                    .any(|a| a.key.request == *target && a.possibly_exposed)
                {
                    let Command::Cancel {
                        attempt,
                        good_until,
                        ..
                    } = original.body.command
                    else {
                        return Err(Error::Invalid);
                    };
                    control = Control::Order(orders::Action::PrepareCancel {
                        attempt: AttemptKey {
                            request: *target,
                            attempt,
                        },
                        authority_epoch: original.body.epoch,
                        expires_at: good_until,
                    });
                }
            }
            _ => (),
        }
        let record = if let Some(old) = state
            .api_records()
            .iter()
            .find(|a| a.record.request == key(original))
        {
            old.record.clone()
        } else {
            if history.effect(
                original,
                checked.principal() == Principal::Owner,
                self,
                owner.wallet,
                &descriptor.ledger,
                now,
            )? != retained::Effect::Recorded
            {
                return Err(Error::Invalid);
            }
            retained::Record {
                unit: original.unit,
                request: key(original),
                policy: original.body.policy,
                digest: original.digest(),
                epoch: original.body.epoch,
                signer: original.body.signer,
                contract: history.contract,
                effect: retained::Effect::Recorded,
                wire: original.encode()?,
            }
        };
        let id = decision_id(original)?;
        let proposal = model::Transaction {
            id,
            expected: checked.head(),
            at: now,
            evidence: vec![],
            inputs: vec![],
            order_observations: vec![],
            funds_observations: vec![],
            controls: vec![control],
        };
        let result = journal
            .commit(units::Transaction {
                id,
                expected: checked.head(),
                at: now,
                change: units::Change::Admit(Box::new(retained::Admission {
                    record,
                    kind,
                    proposal,
                })),
            })
            .map_err(|e| {
                if e == cinder_journal::Error::Conflict {
                    Error::Conflict
                } else {
                    Error::Unavailable
                }
            })?;
        let units::Receipt::Admitted { receipt, .. } = result.receipt else {
            return Err(Error::Unavailable);
        };
        Ok(Decided {
            request: original.clone(),
            at: now,
            refusal: receipt.controls,
            duplicate: false,
        })
    }
}

fn original_authority(
    req: &Request,
    history: &History,
    owner: [u8; 32],
    now: u64,
) -> Result<(), Error> {
    if req.body.signer != owner {
        let cut = history
            .lookup(scope(req), req.body.signer, history.head)?
            .ok_or(Error::Unauthorized)?;
        authorization::permissions(&req.body.command, &cut.grant, now)?;
    }
    Ok(())
}
fn decision_id(req: &Request) -> Result<CommitId, Error> {
    let mut w = Writer::new(b"CINDER-UNIT-API-DECISION\0\x00\x01");
    for b in [
        req.body.domain.network.bytes(),
        req.body.domain.deployment.bytes(),
        req.body.account.bytes(),
        req.body.id.bytes(),
    ] {
        w.raw(&b);
    }
    CommitId::new(Sha256::digest(&*w.0).into()).map_err(|_| Error::Invalid)
}

// Pure financial shape, shared by admission and retained-history validation.
// Holds are deliberately computed later by the trusted port, never the wire.
fn control(
    req: &Request,
    config: &cinder_kernel::ledger::Config,
    now: u64,
) -> Result<(retained::Kind, Control), Error> {
    let b = &req.body;
    let market = |id| {
        config
            .markets
            .iter()
            .find(|m| m.unit().market == id)
            .map(|m| m.unit())
            .ok_or(Error::Invalid)
    };
    let approval = |hash| orders::Approval {
        account: b.account,
        intent_hash: hash,
        authority_epoch: b.epoch,
    };
    let deadline = |at| {
        if at > now {
            Ok(at)
        } else {
            Err(Error::Invalid)
        }
    };
    Ok(match &b.command {
        Command::Order {
            market: m,
            lots,
            minimum,
            maximum,
            fee,
            tif,
            reduce_only,
            good_until,
        } => {
            let m = market(*m)?;
            if *lots == 0
                || minimum > maximum
                || *fee < 0
                || fee.checked_mul(i128::from(lots.unsigned_abs())).is_none()
            {
                return Err(Error::Invalid);
            }
            let intent = orders::Intent {
                request: key(req),
                quantity: QuantityLots::new(m, *lots),
                minimum: PriceTicks::new(m, *minimum).map_err(|_| Error::Invalid)?,
                maximum: PriceTicks::new(m, *maximum).map_err(|_| Error::Invalid)?,
                maximum_fee_per_lot: QuoteAtoms::new(m.quote, *fee),
                reduce_only: *reduce_only,
                policy: b.policy,
                authority_epoch: b.epoch,
                expires_at: deadline(*good_until)?,
                time_in_force: match tif {
                    0 => orders::TimeInForce::GoodTilCancelled,
                    1 => orders::TimeInForce::AddLiquidityOnly,
                    2 => orders::TimeInForce::ImmediateOrCancel,
                    _ => return Err(Error::Invalid),
                },
            };
            (
                retained::Kind::Order,
                Control::Order(orders::Action::Accept {
                    approval: approval(intent.digest().map_err(|_| Error::Invalid)?),
                    intent: Box::new(intent),
                    reservations: vec![],
                }),
            )
        }
        Command::Cancel {
            target,
            attempt,
            good_until,
        } => {
            deadline(*good_until)?;
            let target = RequestKey {
                request: *target,
                ..key(req)
            };
            (
                retained::Kind::Cancel {
                    target,
                    attempt: *attempt,
                },
                Control::Release(target),
            )
        }
        Command::Payout {
            net,
            maximum_fee,
            allow_partial,
            good_until,
        } => {
            if *net <= 0 || *maximum_fee < 0 || net.checked_add(*maximum_fee).is_none() {
                return Err(Error::Invalid);
            }
            // Recipient is filled from the selected unit's immutable registration.
            let intent = funds::Intent {
                request: key(req),
                source: Location::Vault,
                destination: Destination::Recipient([0; 32]),
                net: QuoteAtoms::new(config.quote, *net),
                maximum_fee: QuoteAtoms::new(config.quote, *maximum_fee),
                fee_payer: Owner::Customer(b.account),
                allow_partial: *allow_partial,
                policy: b.policy,
                authority_epoch: b.epoch,
                expires_at: deadline(*good_until)?,
            };
            (
                retained::Kind::Payout,
                Control::Funds(funds::Action::Accept {
                    approval: approval(intent.digest().map_err(|_| Error::Invalid)?),
                    intent: Box::new(intent),
                }),
            )
        }
        Command::Leverage {
            market: m,
            leverage,
            good_until,
        } => {
            let selection = risk::Selection {
                request: key(req),
                market: market(*m)?,
                leverage: *leverage,
                policy: b.policy,
                authority_epoch: b.epoch,
                expires_at: deadline(*good_until)?,
            };
            (
                retained::Kind::Leverage,
                Control::Risk(risk::Action::Select {
                    approval: approval(selection.digest().map_err(|_| Error::Invalid)?),
                    selection: Box::new(selection),
                }),
            )
        }
        _ => return Err(Error::Invalid),
    })
}

/// Reinterpret trusted journal joins before they supply grants/financial history.
/// Replay verifies engine decisions; this checks original API semantics too.
pub(super) fn validate<'a>(
    state: &units::State,
    transactions: impl Iterator<Item = (&'a units::Transaction, &'a units::Receipt)>,
    history: &History,
) -> Result<(), Error> {
    for (tx, _) in transactions {
        let units::Change::Admit(a) = &tx.change else {
            continue;
        };
        let original = history
            .find(a.record.request.account, a.record.request.request)
            .ok_or(Error::Unavailable)?;
        let req = &original.request;
        if original.at > tx.at || req.unit != a.record.unit || req.encode()? != a.record.wire {
            return Err(Error::Unavailable);
        }
        let config = &state
            .configuration()
            .units()
            .iter()
            .find(|u| u.id == req.unit)
            .ok_or(Error::Unavailable)?
            .ledger;
        let owner = state
            .customers()
            .iter()
            .find(|c| c.account == req.body.account)
            .ok_or(Error::Unavailable)?;
        original_authority(req, history, owner.wallet, tx.at).map_err(|_| Error::Unavailable)?;
        let (kind, expected) = control(req, config, tx.at).map_err(|_| Error::Unavailable)?;
        if kind != a.kind {
            return Err(Error::Unavailable);
        }
        let valid = match (expected, a.proposal.controls.as_slice()) {
            (
                Control::Order(orders::Action::Accept {
                    intent, approval, ..
                }),
                [
                    Control::Order(orders::Action::Accept {
                        intent: actual,
                        approval: actual_approval,
                        ..
                    }),
                ],
            ) => intent == *actual && approval == *actual_approval,
            (Control::Release(target), [Control::Release(actual)]) => target == *actual,
            (
                Control::Release(target),
                [
                    Control::Order(orders::Action::PrepareCancel {
                        attempt,
                        authority_epoch,
                        expires_at,
                    }),
                ],
            ) => {
                matches!(req.body.command,Command::Cancel { attempt:id,good_until,.. } if attempt.request==target && attempt.attempt==id && *authority_epoch==req.body.epoch && *expires_at==good_until)
            }
            (
                Control::Funds(funds::Action::Accept { intent, .. }),
                [
                    Control::Funds(funds::Action::Accept {
                        intent: actual,
                        approval,
                    }),
                ],
            ) => {
                let mut intent = *intent;
                intent.destination = Destination::Recipient(
                    state
                        .unit(req.unit)
                        .ok_or(Error::Unavailable)?
                        .registrations()
                        .iter()
                        .find(|r| r.account == req.body.account)
                        .ok_or(Error::Unavailable)?
                        .tokens,
                );
                intent == **actual
                    && approval.account == req.body.account
                    && approval.authority_epoch == req.body.epoch
                    && approval.intent_hash == intent.digest().map_err(|_| Error::Unavailable)?
            }
            (expected, [actual]) => expected == *actual,
            _ => false,
        };
        if !valid {
            return Err(Error::Unavailable);
        }
        if let retained::Kind::Cancel { target, .. } = kind {
            let old = history
                .find(target.account, target.request)
                .filter(|r| r.request.unit == req.unit)
                .ok_or(Error::Unavailable)?;
            if !matches!(old.request.body.command,Command::Order{..}) || req.body.signer!=owner.wallet &&
                (old.request.body.signer!=req.body.signer || !history.grants.iter().any(|(s,g)|*s==scope(req)&&g.grant.key==req.body.signer&&matches!(old.request.body.command,Command::Order{market,..} if market==g.grant.market)))
            { return Err(Error::Unavailable); }
        }
    }
    Ok(())
}
