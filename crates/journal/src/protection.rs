//! Explicit house-coverage controls. No production coverage or priority is chosen
//! here: the deployment must qualify and authenticate the exact allocation plan.
use crate::{
    Error,
    model::*,
    wire::{self, Reader, Writer},
};
use cinder_kernel::{
    amounts::*,
    identity::*,
    ledger::{evidence::Disposition, protection::*, *},
};
use sha2::{Digest, Sha256};

/// Trusted deployment configuration; zero caps disable discretionary absorption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    /// Changed policy requires a higher revision, without resetting spent capital.
    pub revision: PolicyVersion,
    /// Verified administrator authorization epoch, not a customer trading grant.
    pub authority_epoch: u64,
    /// Qualification expiry in the injected clock domain.
    pub valid_until: u64,
    /// Global lifetime absorption ceiling. Not a per-account Sybil defense.
    pub lifetime_limit: QuoteAtoms,
    /// Additional lifetime ceiling per private account.
    pub customer_limit: QuoteAtoms,
    /// Commitment to separately approved coverage and competing-claim priority.
    /// A hash is not proof of approval; the trusted deployment authenticates it.
    pub coverage: [u8; 32],
}
/// Exact controller decision, never a public caller-selected capital authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    /// One-shot identity/domain (account is bookkeeping, not the administrator).
    pub request: RequestKey,
    /// Financial cut in addition to the encompassing journal CAS head.
    pub expected_version: u64,
    /// Coverage revision and authenticated administrator epoch.
    pub policy: PolicyVersion,
    /// Must match the active policy authority.
    pub authority_epoch: u64,
    /// Must be within policy qualification lifetime.
    pub expires_at: u64,
    /// Designation, commitment or exact multi-claim absorption. Owed recognition
    /// is an input fact, so failed controls cannot erase it.
    pub change: ProtectionChange,
}
impl Decision {
    /// Binding for the trusted administration authentication port, not a signature.
    pub fn digest(&self) -> Result<[u8; 32], Error> {
        let mut w = Writer::new(50);
        encode_decision(&mut w, self);
        Ok(Sha256::digest(w.finish()?).into())
    }
}
/// Trusted controls; actual obligation inputs precede this all-or-none proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Authenticated deployment installation; not reachable by a customer API.
    Install {
        /// Exact financial cut at installation.
        expected_version: u64,
        /// Authenticated deployment policy, not caller-selected limits.
        policy: Box<Policy>,
    },
    /// Authentication port must verify the exact digest under the configured
    /// administrator epoch. Merely supplying these bytes is NOT authentication.
    Apply {
        /// Exact policy-bound instruction.
        decision: Box<Decision>,
        /// Output of the administrator authentication port.
        authenticated_digest: [u8; 32],
    },
}
impl State {
    /// Active explicit coverage limits, distinct from nominal fund designation.
    pub fn protection_policy(&self) -> Option<&Policy> {
        self.protection_policy.as_ref()
    }
    pub(crate) fn protection_action(&mut self, action: &Action) -> Result<(), ControlError> {
        match action {
            Action::Install {
                expected_version,
                policy,
            } => {
                if *expected_version != self.ledger.version()
                    || policy.authority_epoch == 0
                    || policy.valid_until < self.now
                    || policy.coverage == [0; 32]
                    || policy.lifetime_limit.unit() != self.config.quote
                    || policy.customer_limit.unit() != self.config.quote
                    || policy.lifetime_limit.atoms() < 0
                    || policy.customer_limit.atoms() < 0
                    || policy.customer_limit.atoms() > policy.lifetime_limit.atoms()
                    || self.protection_policy.as_ref().is_some_and(|old| {
                        policy.revision < old.revision
                            || policy.authority_epoch < old.authority_epoch
                            || policy.revision == old.revision && policy.as_ref() != old
                    })
                {
                    return Err(ControlError::Invalid);
                }
                self.protection_policy = Some(policy.as_ref().clone());
            }
            Action::Apply {
                decision: d,
                authenticated_digest,
            } => {
                self.request(d.request)?;
                let policy = self
                    .protection_policy
                    .as_ref()
                    .ok_or(ControlError::Unqualified)?;
                if d.expected_version != self.ledger.version()
                    || d.policy != policy.revision
                    || d.authority_epoch != policy.authority_epoch
                    || d.digest().map_err(|_| ControlError::Invalid)? != *authenticated_digest
                    || self.holds.iter().any(|h| h.request == d.request)
                    || self.selections.iter().any(|s| s.request == d.request)
                {
                    return Err(ControlError::Invalid);
                }
                if self.now > d.expires_at || d.expires_at > policy.valid_until {
                    return Err(ControlError::Expired);
                }
                match &d.change {
                    ProtectionChange::Recognize { .. } => return Err(ControlError::Invalid),
                    ProtectionChange::Absorb { allocations } => {
                        let p = self.ledger.protection();
                        let mut total = p.absorbed_total;
                        for row in allocations {
                            self.claim_owner_quiet(row.claim.account)?;
                            total = total
                                .checked_add(row.amount)
                                .map_err(|_| ControlError::Capacity)?;
                            let spent = p
                                .claims
                                .iter()
                                .filter(|c| c.id.account == row.claim.account)
                                .try_fold(QuoteAtoms::new(self.config.quote, 0), |sum, c| {
                                    sum.checked_add(c.absorbed)
                                })
                                .map_err(|_| ControlError::Capacity)?;
                            let proposed = allocations
                                .iter()
                                .filter(|a| a.claim.account == row.claim.account)
                                .try_fold(spent, |sum, a| sum.checked_add(a.amount))
                                .map_err(|_| ControlError::Capacity)?;
                            if proposed.atoms() > policy.customer_limit.atoms() {
                                return Err(ControlError::Capacity);
                            }
                        }
                        if total.atoms() > policy.lifetime_limit.atoms() {
                            return Err(ControlError::Capacity);
                        }
                    }
                    ProtectionChange::Commit { claim, .. } => {
                        self.claim_owner_quiet(claim.account)?
                    }
                    _ => {}
                }
                let e = Event {
                    key: RecordKey::Request(d.request),
                    policy: self.config.policy,
                    change: Change::Protection(d.change.clone()),
                };
                let next = self
                    .ledger
                    .ingest(&e, self.now)
                    .map_err(|_| ControlError::Invalid)?;
                if next.disposition != Disposition::Applied {
                    return Err(ControlError::Invalid);
                }
                self.ledger = next.state;
                if matches!(d.change,ProtectionChange::Designate{delta} if delta.atoms()<0) {
                    // Releasing designation is not a native house payout rail.
                    // It must still leave the joined capital envelope satisfied.
                    self.risk_gate()?;
                }
            }
        }
        Ok(())
    }
    pub(crate) fn protection_ready(&self) -> Result<(), ControlError> {
        let p = self.ledger.protection();
        if !p.active {
            return Ok(());
        }
        let policy = self
            .protection_policy
            .as_ref()
            .ok_or(ControlError::Unqualified)?;
        if self.risk.is_none()
            || policy.valid_until < self.now
            || p.unfunded().map_err(|_| ControlError::Capacity)?.atoms() > 0
            || self
                .ledger
                .unresolved_protection()
                .map_err(|_| ControlError::Unqualified)?
        {
            return Err(ControlError::Unqualified);
        }
        Ok(())
    }
    fn claim_owner_quiet(&self, id: AccountId) -> Result<(), ControlError> {
        if self.ledger.reductions().iter().any(|r| {
            r.rows
                .iter()
                .any(|row| row.owner == id && !row.void && row.restored < row.amount)
                && r.attempt.is_some_and(|a| {
                    self.orders
                        .iter()
                        .any(|o| o.attempt == Some(a) && !o.complete())
                })
        }) {
            return Err(ControlError::Unqualified);
        }
        // A flat snapshot is not a completed close if another order or funds
        // movement can still change this debtor's balance after absorption.
        if self
            .orders
            .iter()
            .any(|o| o.intent.request.account == id && !o.complete())
            || self
                .funds
                .iter()
                .any(|o| o.intent.request.account == id && !o.terminal)
            || self.holds.iter().any(|h| {
                h.active
                    && h.reservations
                        .iter()
                        .any(|r| r.resource == Resource::Customer(id))
            })
        {
            return Err(ControlError::Unqualified);
        }
        Ok(())
    }
}
pub(crate) fn encode_policy(w: &mut Writer, p: &Policy) {
    w.raw(&p.revision.get().to_be_bytes());
    w.u64(p.authority_epoch);
    w.u64(p.valid_until);
    w.item(&p.lifetime_limit);
    w.item(&p.customer_limit);
    w.raw(&p.coverage);
}
fn decode_policy(r: &mut Reader<'_>) -> Result<Policy, Error> {
    Ok(Policy {
        revision: PolicyVersion::new(u32::from_be_bytes(r.array()?)).map_err(|_| Error::Codec)?,
        authority_epoch: r.u64()?,
        valid_until: r.u64()?,
        lifetime_limit: r.item()?,
        customer_limit: r.item()?,
        coverage: r.array()?,
    })
}
fn encode_decision(w: &mut Writer, d: &Decision) {
    w.item(&d.request);
    w.u64(d.expected_version);
    w.raw(&d.policy.get().to_be_bytes());
    w.u64(d.authority_epoch);
    w.u64(d.expires_at);
    wire::encode_protection(w, &d.change);
}
fn decode_decision(r: &mut Reader<'_>) -> Result<Decision, Error> {
    Ok(Decision {
        request: r.item()?,
        expected_version: r.u64()?,
        policy: PolicyVersion::new(u32::from_be_bytes(r.array()?)).map_err(|_| Error::Codec)?,
        authority_epoch: r.u64()?,
        expires_at: r.u64()?,
        change: wire::decode_protection(r)?,
    })
}
pub(crate) fn encode_action(w: &mut Writer, a: &Action) {
    match a {
        Action::Install {
            expected_version,
            policy,
        } => {
            w.byte(0);
            w.u64(*expected_version);
            encode_policy(w, policy)
        }
        Action::Apply {
            decision,
            authenticated_digest,
        } => {
            w.byte(1);
            encode_decision(w, decision);
            w.raw(authenticated_digest)
        }
    }
}
pub(crate) fn decode_action(r: &mut Reader<'_>) -> Result<Action, Error> {
    match r.byte()? {
        0 => Ok(Action::Install {
            expected_version: r.u64()?,
            policy: Box::new(decode_policy(r)?),
        }),
        1 => Ok(Action::Apply {
            decision: Box::new(decode_decision(r)?),
            authenticated_digest: r.array()?,
        }),
        _ => Err(Error::Codec),
    }
}
