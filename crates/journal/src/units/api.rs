//! Trusted API-controller records under the one unit journal. This layer checks
//! ownership/atomicity, not the application wire signature. The API interpreter
//! must validate the retained wire and its typed header before using a grant.
use super::*;

/// Financial capture and administrative completion are different contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Original intent retained; this record alone implies no financial effects.
    Recorded,
    /// Owner's accepted grant; application semantics live in the retained wire.
    Grant,
    /// Owner's global revoke-all, atomically advancing all unit epochs once.
    Revoke,
}

/// Original qualified signed mutation. Construct only inside the trusted API
/// controller; raw caller data is not authorization to append this record.
#[derive(Clone, PartialEq, Eq)]
pub struct Record {
    /// Immutable execution unit.
    pub unit: UnitId,
    /// Globally unique owner/request identity.
    pub request: RequestKey,
    /// Signed economic policy of the unit.
    pub policy: PolicyVersion,
    /// V2 economic digest, not a foundation reservation commitment.
    pub digest: [u8; 32],
    /// Original authenticated global epoch, before a possible revoke effect.
    pub epoch: u64,
    /// Original authenticated signer. Only the owner can grant/revoke.
    pub signer: [u8; 32],
    /// Immutable loaded API/registry/limits commitment, pinned by first capture.
    pub contract: [u8; 32],
    /// Recorded-only financial or completed administrative effect.
    pub effect: Effect,
    /// Original canonical signed v2 request, max 1,024 bytes, retained privately.
    pub wire: model::PrivateBytes,
}
impl fmt::Debug for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiRecord([PRIVATE])")
    }
}

/// Immutable state-derived accepted record. Fields cannot be supplied to mutate
/// journal state; constructing a copy is not evidence of accepted history.
#[derive(Clone, PartialEq, Eq)]
pub struct Accepted {
    /// Original immutable record.
    pub record: Record,
    /// Original trusted journal capture time, not retry time.
    pub at: u64,
}
impl fmt::Debug for Accepted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiAccepted([PRIVATE])")
    }
}

/// Typed financial command family, qualified against the original wire by API.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Original bounded order, with holds computed by the trusted admission port.
    Order,
    /// Original admitted order and the signed cancellation attempt identity.
    Cancel {
        /// Original admitted order in the same customer/unit.
        target: RequestKey,
        /// Signed identity of this cancellation attempt.
        attempt: AttemptId,
    },
    /// Owner's payout request, not a completed beneficiary payment.
    Payout,
    /// Owner's private initial-margin preference, not native account leverage.
    Leverage,
}
impl fmt::Debug for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKind([PRIVATE])")
    }
}
/// Trusted API-controller proposal; not a network DTO or native authorization.
#[derive(Clone, PartialEq, Eq)]
pub struct Admission {
    /// Exact original record; retries cannot replace it with refreshed auth.
    pub record: Record,
    /// Original command family and cancellation target.
    pub kind: Kind,
    /// One financial control, exact outer id/head/time; observations remain real.
    pub proposal: model::Transaction,
}
impl fmt::Debug for Admission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiAdmission([PRIVATE])")
    }
}
/// Immutable decided operation; refusal is terminal for this request too.
#[derive(Clone, PartialEq, Eq)]
pub struct Decision {
    /// Original globally owned request.
    pub request: RequestKey,
    /// Original decision time, not retry time.
    pub at: u64,
    /// Replayed financial result, never native exposure or completion.
    pub receipt: model::Receipt,
}
impl fmt::Debug for Decision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiDecision([PRIVATE])")
    }
}

impl State {
    pub(super) fn admit_api(&mut self, tx: &Transaction, a: &Admission) -> Result<Receipt, Error> {
        let r = &a.record;
        let p = &a.proposal;
        if r.effect != Effect::Recorded
            || p.id != tx.id
            || p.expected != tx.expected
            || p.at != tx.at
            || self.decisions.iter().any(|d| d.request == r.request)
            || self.decisions.len() >= wire::MAX_ITEMS
            || !self
                .customers
                .iter()
                .any(|c| c.account == r.request.account && c.epoch == r.epoch)
        {
            return Err(Error::Invalid);
        }
        if let Some(old) = self.api.iter().find(|old| old.record.request == r.request) {
            if old.record != *r {
                return Err(Error::Conflict);
            }
        } else {
            self.capture_api(tx, r)?;
        }
        let valid = match (a.kind, p.controls.as_slice()) {
            (Kind::Order, [model::Control::Order(orders::Action::Accept { intent, .. })]) => {
                intent.request == r.request
                    && intent.authority_epoch == r.epoch
                    && intent.policy == r.policy
            }
            (
                Kind::Payout,
                [model::Control::Funds(crate::funds::Action::Accept { intent, .. })],
            ) => {
                intent.request == r.request
                    && intent.authority_epoch == r.epoch
                    && intent.policy == r.policy
            }
            (
                Kind::Leverage,
                [model::Control::Risk(crate::risk::Action::Select { selection, .. })],
            ) => {
                selection.request == r.request
                    && selection.authority_epoch == r.epoch
                    && selection.policy == r.policy
            }
            (Kind::Cancel { target, attempt }, [control]) => {
                let target_owned = target.domain == r.request.domain
                    && target.account == r.request.account
                    && self
                        .decisions
                        .iter()
                        .any(|d| d.request == target && d.receipt.controls.is_none())
                    && self
                        .api
                        .iter()
                        .any(|old| old.record.request == target && old.record.unit == r.unit)
                    && self.unit(r.unit).is_some_and(|s| {
                        s.orders()
                            .iter()
                            .any(|o| o.intent.request == target && !o.complete())
                    });
                target_owned
                    && match control {
                        model::Control::Release(key) => *key == target,
                        model::Control::Order(orders::Action::PrepareCancel {
                            attempt: key,
                            authority_epoch,
                            ..
                        }) => {
                            key.request == target
                                && key.attempt == attempt
                                && *authority_epoch == r.epoch
                        }
                        _ => false,
                    }
            }
            _ => false,
        };
        let financial = !matches!(a.kind, Kind::Cancel { .. });
        let qualified = !financial || self.unit(r.unit).is_some_and(|s| s.has_joined_risk());
        let mut facts = p.clone();
        if !valid || !qualified {
            facts.controls.clear();
        }
        let index = self
            .units
            .iter()
            .position(|(u, _)| *u == r.unit)
            .ok_or(Error::Invalid)?;
        let (next, mut receipt) = self.units[index]
            .1
            .advance_with_scope(&facts, |i| self.input_owned(r.unit, i))?;
        if !valid || !qualified {
            receipt.controls = Some(if valid {
                model::ControlError::Unqualified
            } else {
                model::ControlError::Invalid
            });
        }
        self.units[index].1 = next;
        self.decisions.push(Decision {
            request: r.request,
            at: tx.at,
            receipt: receipt.clone(),
        });
        Ok(Receipt::Admitted {
            unit: r.unit,
            receipt,
        })
    }
}

impl State {
    pub(super) fn capture_api(&mut self, tx: &Transaction, record: &Record) -> Result<(), Error> {
        let owner = self
            .customers
            .iter()
            .find(|c| c.account == record.request.account)
            .ok_or(Error::Invalid)?;
        let descriptor = self
            .config
            .units
            .iter()
            .find(|u| u.id == record.unit)
            .ok_or(Error::Invalid)?;
        if record.request.domain != self.config.domain
            || record.policy != descriptor.ledger.policy
            || record.epoch == 0
            || record.epoch != owner.epoch
            || record.signer == [0; 32]
            || record.contract == [0; 32]
            || record.digest == [0; 32]
            || record.wire.as_bytes().is_empty()
            || record.wire.as_bytes().len() > 1024
        {
            return Err(Error::Invalid);
        }
        if self.api.len() >= wire::MAX_ITEMS {
            return Err(Error::Limit);
        }
        if self.api.iter().any(|a| a.record.request == record.request)
            || self.route(record.request).is_some()
            || self
                .api
                .first()
                .is_some_and(|a| a.record.contract != record.contract)
        {
            return Err(Error::Conflict);
        }
        if record.effect != Effect::Recorded && record.signer != owner.wallet {
            return Err(Error::Invalid);
        }
        if record.effect == Effect::Revoke {
            self.advance_authority(
                tx,
                record.request.account,
                record.epoch.checked_add(1).ok_or(Error::Limit)?,
            )?;
        }
        self.api.push(Accepted {
            record: record.clone(),
            at: tx.at,
        });
        Ok(())
    }
}
