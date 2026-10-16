//! Owner bootstrap for the new shared registry. Not a v1 signature wrapper,
//! credit acknowledgment, native signer or independent account database.
use super::*;
use crate::onboarding::{Command, Discovery, Policy};
use cinder_journal::{
    controller::Controller,
    model::{CommitId, Control},
    onboarding as owners,
    read::Source,
};

/// Versioned owner-only signature purpose, separate from API grants and v1.
pub const MAGIC: &[u8] = b"CINDER-UNIT-ONBOARD\0\x00\x01";
/// Original private bootstrap, bound to the entire loaded custody inventory.
pub struct Request {
    /// Selected discovery/deposit-notice unit; enrollment itself is shared.
    pub unit: UnitId,
    /// Independently pinned registry/policy commitment.
    pub contract: [u8; 32],
    /// Existing owner identity/session/action fields; signature covers THIS message.
    pub body: crate::onboarding::Request,
}
impl Request {
    /// Fresh owner signature preimage. A v1 signed request is not reusable.
    pub fn message(&self) -> Zeroizing<Vec<u8>> {
        Zeroizing::new(
            [
                MAGIC,
                &self.unit.bytes(),
                &self.contract,
                &self.body.message(),
            ]
            .concat(),
        )
    }
    /// Bounded encrypted application payload.
    pub fn encode(&self) -> Result<PrivateBytes, Error> {
        PrivateBytes::new([self.message().as_slice(), &self.body.signature].concat())
            .map_err(|_| Error::Invalid)
    }
    /// Strict new-purpose decoder, with no v1 fallback or trailing bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > 640 {
            return Err(Error::Invalid);
        }
        let mut r = Reader::new(bytes, MAGIC)?;
        let unit = UnitId::new(r.array()?).map_err(|_| Error::Invalid)?;
        let contract = r.array()?;
        let body = crate::onboarding::Request::decode(
            bytes.get(MAGIC.len() + 64..).ok_or(Error::Invalid)?,
        )?;
        let request = Self {
            unit,
            contract,
            body,
        };
        if request.encode()?.as_bytes() != bytes {
            return Err(Error::Invalid);
        }
        Ok(request)
    }
}
impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UnitBootstrap([PRIVATE])")
    }
}
/// Loaded canonical custody discovery policies for every unit, not user input.
pub struct Contract {
    configuration: units::Configuration,
    policies: Vec<(UnitId, Policy)>,
    digest: [u8; 32],
}
impl fmt::Debug for Contract {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UnitOnboarding([PRIVATE])")
    }
}
impl Contract {
    /// Validate canonical config/route PDAs, exact registry/config/venue identity
    /// and complete unit inventory. This does not qualify any native funding rail.
    pub fn new(
        configuration: units::Configuration,
        policies: Vec<(UnitId, Policy)>,
    ) -> Result<Self, Error> {
        if policies.len() != configuration.units().len() {
            return Err(Error::Invalid);
        }
        let mut w = Writer::new(b"CINDER-UNIT-CUSTODY-1\0");
        w.raw(&configuration.commitment().map_err(|_| Error::Invalid)?);
        for (unit, (id, policy)) in configuration.units().iter().zip(&policies) {
            policy.validate()?;
            if unit.id != *id
                || policy.config != unit.custody_config
                || policy.payout_vault()? != unit.payout_vault
                || policy.venue != unit.ledger.venue.bytes()
                || policy.custody_domain != configuration.domain().deployment.bytes()
            {
                return Err(Error::Invalid);
            }
            w.raw(&id.bytes());
            policy.encode(&mut w);
        }
        let digest = Sha256::digest(&*w.0).into();
        Ok(Self {
            configuration,
            policies,
            digest,
        })
    }
    /// Pin this value in client/release configuration; never trust a host hint.
    pub fn commitment(&self) -> [u8; 32] {
        self.digest
    }
    fn policy(&self, unit: UnitId) -> Result<&Policy, Error> {
        self.policies
            .iter()
            .find(|(u, _)| *u == unit)
            .map(|(_, p)| p)
            .ok_or(Error::Invalid)
    }
    fn verify(&self, r: &Request, at: u64, maximum: u64) -> Result<(), Error> {
        let key = VerifyingKey::from_bytes(&r.body.wallet).map_err(|_| Error::Unauthorized)?;
        let policy = &self
            .configuration
            .units()
            .iter()
            .find(|u| u.id == r.unit)
            .ok_or(Error::Unauthorized)?
            .ledger;
        if key.is_weak()
            || r.contract != self.digest
            || r.body.domain != self.configuration.domain()
            || r.body.policy != policy.policy
            || r.body.session == [0; 32]
            || at >= r.body.expires_at
            || r.body.expires_at - at > maximum
        {
            return Err(Error::Unauthorized);
        }
        key.verify_strict(&r.message(), &Signature::from_bytes(&r.body.signature))
            .map_err(|_| Error::Unauthorized)
    }
    fn recipients(&self, wallet: [u8; 32]) -> Result<Vec<units::Recipient>, Error> {
        self.policies
            .iter()
            .map(|(unit, p)| {
                Ok(units::Recipient {
                    unit: *unit,
                    tokens: p.recipient(wallet)?,
                })
            })
            .collect()
    }
    pub(super) fn initialize<B: Backend, P: Protection>(
        &self,
        j: &units::Journal<B, P>,
        maximum: u64,
    ) -> Result<(), Error> {
        if j.configuration() != &self.configuration {
            return Err(Error::Conflict);
        }
        for (tx, _) in j.transactions().map_err(|_| Error::Unavailable)? {
            if matches!(tx.change, units::Change::Enroll { .. }) {
                return Err(Error::Unauthorized);
            }
            if let units::Change::EnrollOwner {
                wallet,
                recipients,
                proof,
            } = &tx.change
            {
                let r = Request::decode(proof.as_bytes())?;
                self.verify(&r, tx.at, maximum)?;
                if !matches!(r.body.command, Command::Open)
                    || *wallet != r.body.wallet
                    || *recipients != self.recipients(*wallet)?
                {
                    return Err(Error::Conflict);
                }
            }
        }
        Ok(())
    }
    pub(super) fn handle<B: Backend, P: Protection>(
        &self,
        j: &mut units::Journal<B, P>,
        channel: &impl ConfidentialChannel,
        bytes: &PrivateBytes,
        now: u64,
        maximum: u64,
    ) -> Result<(UnitId, Response), Error> {
        let r = Request::decode(bytes.as_bytes())?;
        self.verify(&r, now, maximum)?;
        if r.body.domain != channel.domain()
            || r.body.session != channel.binding()
            || r.body.expires_at > channel.expires_at()
            || now >= channel.expires_at()
        {
            return Err(Error::Unauthorized);
        }
        j.verified_state().map_err(|_| Error::Unavailable)?;
        if j.configuration() != &self.configuration
            || j.state().map_err(|_| Error::Unavailable)?.logical_time() > now
        {
            return Err(Error::Unavailable);
        }
        let account = owners::account(r.body.domain, r.body.wallet).map_err(|_| Error::Invalid)?;
        let registered = j
            .state()
            .map_err(|_| Error::Unavailable)?
            .customers()
            .iter()
            .find(|c| c.account == account);
        let recipients = self.recipients(r.body.wallet)?;
        if let Some(owner) = registered
            && (owner.wallet != r.body.wallet
                || recipients.iter().any(|recipient| {
                    j.state()
                        .ok()
                        .and_then(|s| s.unit(recipient.unit))
                        .and_then(|s| s.registrations().iter().find(|b| b.account == account))
                        .is_none_or(|b| b.wallet != r.body.wallet || b.tokens != recipient.tokens)
                }))
        {
            return Err(Error::Conflict);
        }
        let mut identity = Writer::new(b"CINDER-UNIT-ONBOARD-COMMIT-1\0");
        identity.raw(&self.digest);
        identity.raw(&account.bytes());
        match r.body.command {
            Command::Open if registered.is_none() => {
                if self.policies.iter().any(|(_, p)| {
                    j.state()
                        .is_ok_and(|s| s.customers().len() >= usize::from(p.maximum_accounts))
                }) {
                    return Err(Error::Unavailable);
                }
                // Never append a new liability after any recovery key inventory.
                if j.transactions().map_err(|_|Error::Unavailable)?.any(|(t,_)|matches!(&t.change,units::Change::Unit {proposal,..} if proposal.evidence.iter().any(|e|e.as_bytes().starts_with(b"CINDER_RECOVERY_KEYS_V1\0")))) {return Err(Error::Unavailable);}
                identity.byte(0);
                j.commit(units::Transaction {
                    id: CommitId::new(Sha256::digest(&*identity.0).into())
                        .map_err(|_| Error::Invalid)?,
                    expected: j.head(),
                    at: now,
                    change: units::Change::EnrollOwner {
                        wallet: r.body.wallet,
                        recipients,
                        proof: r.encode()?,
                    },
                })
                .map_err(|_| Error::Unavailable)?;
            }
            Command::Open => (),
            Command::Deposit {
                epoch,
                operation,
                signature,
            } => {
                if registered.is_none_or(|c| c.wallet != r.body.wallet || c.epoch != epoch) {
                    return Err(Error::Unauthorized);
                }
                let state = j
                    .state()
                    .map_err(|_| Error::Unavailable)?
                    .unit(r.unit)
                    .ok_or(Error::Invalid)?;
                if let Some(old) = state
                    .deposit_notices()
                    .iter()
                    .find(|n| n.account == account && n.operation == operation)
                {
                    if old.signature != signature {
                        return Err(Error::Conflict);
                    }
                } else {
                    identity.byte(1);
                    identity.raw(&r.unit.bytes());
                    identity.raw(&operation);
                    let mut selected = j.select(r.unit).map_err(|_| Error::Unavailable)?;
                    let tx = cinder_journal::model::Transaction {
                        id: CommitId::new(Sha256::digest(&*identity.0).into())
                            .map_err(|_| Error::Invalid)?,
                        expected: selected.head(),
                        at: now,
                        evidence: vec![r.encode()?],
                        inputs: vec![],
                        order_observations: vec![],
                        funds_observations: vec![],
                        controls: vec![Control::Onboarding(owners::Action::Notice(
                            owners::Notice {
                                account,
                                operation,
                                signature,
                            },
                        ))],
                    };
                    if selected
                        .commit(tx)
                        .map_err(|_| Error::Unavailable)?
                        .receipt
                        .controls
                        .is_some()
                    {
                        return Err(Error::Unavailable);
                    }
                }
            }
        }
        let state = j.state().map_err(|_| Error::Unavailable)?;
        let owner = state
            .customers()
            .iter()
            .find(|c| c.account == account)
            .ok_or(Error::Unavailable)?;
        let component = state.unit(r.unit).ok_or(Error::Unavailable)?;
        let policy = self.policy(r.unit)?.clone();
        Ok((
            r.unit,
            Response::Discovery(Box::new(Discovery {
                account,
                epoch: owner.epoch,
                recipient: policy.recipient(r.body.wallet)?,
                custody: policy,
                pending_deposits: component
                    .deposit_notices()
                    .iter()
                    .filter(|n| {
                        n.account == account
                            && !component.escrows().iter().any(|e| {
                                e.batch.request.account == account
                                    && e.batch.operation == n.operation
                                    && e.batch.signature == n.signature
                            })
                    })
                    .map(|n| n.operation)
                    .collect(),
            })),
        ))
    }
}
