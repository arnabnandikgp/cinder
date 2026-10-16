//! Opt-in v2 scope preflight and separately bounded durable `history` capture.
//! Opt-in financial admission is separate from capture. No listener/signing/dispatch.

pub mod admission;
pub mod history;

use crate::{
    ConfidentialChannel, Error, Response, authorization,
    wire::{self, Command, Grant, Reader, Writer},
};
use cinder_journal::{Backend, Head, Protection, model::PrivateBytes, units};
use cinder_kernel::identity::*;
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};
use std::fmt;
use zeroize::Zeroizing;

const MAGIC: &[u8] = b"CINDER-API\0\x00\x02";
const INTENT: &[u8] = b"CINDER-API-INTENT\0\x00\x02";
const REPLY: &[u8] = b"CINDER-API-REPLY\0\x00\x02";

/// Distinct v2 signed request; the embedded v1-shaped fields are not signed v1.
#[derive(Clone, PartialEq, Eq)]
pub struct Request {
    /// Immutable explicit execution unit, signed rather than inferred from a URL.
    pub unit: UnitId,
    /// Existing command/identity/auth fields. Signature must cover `message()` here.
    pub body: wire::Request,
}
impl Request {
    /// Version-separated wallet/agent preimage, including the unit.
    pub fn message(&self) -> Zeroizing<Vec<u8>> {
        let mut w = Writer::new(MAGIC);
        self.body.economic_prefix(&mut w);
        w.raw(&self.unit.bytes());
        w.u64(self.body.epoch);
        w.raw(&self.body.signer);
        w.raw(&self.body.session);
        w.u64(self.body.expires_at);
        self.body.command.encode(&mut w);
        w.0
    }
    /// Stable economic identity binds unit and financial deadlines, not auth refresh.
    pub fn digest(&self) -> [u8; 32] {
        let mut w = Writer::new(INTENT);
        self.body.economic_prefix(&mut w);
        w.raw(&self.unit.bytes());
        self.body.command.encode(&mut w);
        Sha256::digest(&*w.0).into()
    }
    /// Full private v2 wire; the existing bound includes unit and signature.
    pub fn encode(&self) -> Result<PrivateBytes, Error> {
        let mut bytes = self.message();
        bytes.extend_from_slice(&self.body.signature);
        if bytes.len() > wire::MAX_REQUEST {
            return Err(Error::Invalid);
        }
        PrivateBytes::new(bytes.to_vec()).map_err(|_| Error::Invalid)
    }
    /// Strict v2-only bounded parsing. No legacy/default-unit fallback.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > wire::MAX_REQUEST {
            return Err(Error::Invalid);
        }
        let mut r = Reader::new(bytes, MAGIC)?;
        let domain = Domain {
            network: NetworkId::new(r.array()?).map_err(|_| Error::Invalid)?,
            deployment: DeploymentId::new(r.array()?).map_err(|_| Error::Invalid)?,
        };
        let account = AccountId::new(r.array()?).map_err(|_| Error::Invalid)?;
        let id = RequestId::new(r.array()?).map_err(|_| Error::Invalid)?;
        let policy =
            PolicyVersion::new(u32::from_be_bytes(r.array()?)).map_err(|_| Error::Invalid)?;
        let unit = UnitId::new(r.array()?).map_err(|_| Error::Invalid)?;
        let body = wire::Request {
            domain,
            account,
            id,
            policy,
            epoch: r.u64()?,
            signer: r.array()?,
            session: r.array()?,
            expires_at: r.u64()?,
            command: Command::decode(&mut r)?,
            signature: r.array()?,
        };
        r.done()?;
        let req = Self { unit, body };
        if req.encode()?.as_bytes() != bytes {
            return Err(Error::Invalid);
        }
        Ok(req)
    }
}

/// Complete unit/customer/global-epoch grant scope, never a native capability.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Scope {
    /// Network and deployment of the authoritative journal.
    pub domain: Domain,
    /// Only this unit, regardless of reused market symbols or identities.
    pub unit: UnitId,
    /// Original private owner.
    pub account: AccountId,
    /// Shared revoke-all epoch at acceptance/use.
    pub epoch: u64,
}

/// Trusted accepted-history result, not a request field or owner signature alone.
#[derive(Clone, PartialEq, Eq)]
pub struct GrantCut {
    /// Must equal the exact currently verified shared head.
    pub head: Head,
    /// Original immutable scope; lookup hints cannot override it.
    pub scope: Scope,
    /// Actual accepted grant, not a newly submitted proposal.
    pub grant: Grant,
    /// Journal acceptance time of that grant.
    pub accepted_at: u64,
    /// Accepted order usage at `head`. Admission checks this after operation dedupe.
    pub used_orders: u64,
}

/// Pure in-enclave immutable accepted-history port. Never call parent/network
/// lookups here or use an independently mutable grants database. Runtime wiring
/// must supply a qualified implementation before enabling agent operations.
pub trait Grants {
    /// Resolve at this exact head; the returned cut must repeat all scope fields.
    fn lookup(&self, scope: Scope, signer: [u8; 32], head: Head)
    -> Result<Option<GrantCut>, Error>;
}
/// Explicit owner-only configuration; no implicit fabricated agent grant.
#[derive(Debug, Default)]
pub struct NoGrants;
impl Grants for NoGrants {
    fn lookup(&self, _: Scope, _: [u8; 32], _: Head) -> Result<Option<GrantCut>, Error> {
        Ok(None)
    }
}

/// Signature principal, not financial admission or native authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Principal {
    /// The immutable registered owner wallet.
    Owner,
    /// A currently scoped accepted Cinder grant.
    Agent,
}

/// Read-only preflight result. Not a signing/exposure permit: the later controller
/// must perform idempotency, original cancel ownership, quota, risk and atomic
/// head/epoch rechecks. Keeping the request private prevents post-check mutation.
pub struct Checked {
    request: Request,
    head: Head,
    principal: Principal,
    grant: Option<GrantCut>,
}
impl Checked {
    /// Exact authenticated v2 request; no mutable access.
    pub fn request(&self) -> &Request {
        &self.request
    }
    /// Shared head at preflight, not perpetual freshness authority.
    pub fn head(&self) -> Head {
        self.head
    }
    /// Original signature principal.
    pub fn principal(&self) -> Principal {
        self.principal
    }
    /// Accepted cut for later post-dedupe budget checks, absent for the owner.
    pub fn grant(&self) -> Option<&GrantCut> {
        self.grant.as_ref()
    }
}

/// Trusted request/grant lifetime limits. These are release policy, not calibrated
/// leverage/capital parameters, and grant no durability authority by themselves.
pub struct Authorizer {
    maximum_auth_lifetime: u64,
    maximum_grant_lifetime: u64,
}
impl Authorizer {
    /// Require explicit nonzero limits; no production defaults.
    pub fn new(maximum_auth_lifetime: u64, maximum_grant_lifetime: u64) -> Result<Self, Error> {
        if maximum_auth_lifetime == 0 || maximum_grant_lifetime == 0 {
            return Err(Error::Invalid);
        }
        Ok(Self {
            maximum_auth_lifetime,
            maximum_grant_lifetime,
        })
    }
    /// Authenticate before any operation lookup. This performs no journal append
    /// or financial admission. A freshness failure fences all units through the
    /// existing journal; the grant callback is pure trusted accepted-state data.
    pub fn check<B: Backend, P: Protection>(
        &self,
        journal: &mut units::Journal<B, P>,
        channel: &impl ConfidentialChannel,
        bytes: &PrivateBytes,
        grants: &impl Grants,
        now: u64,
    ) -> Result<Checked, Error> {
        self.check_inner(journal, channel, bytes, grants, now, true)
    }
    // Durable capture validates a *new* owner-grant proposal after dedupe, so
    // an expired original grant can still be recovered by its authenticated owner.
    fn check_inner<B: Backend, P: Protection>(
        &self,
        journal: &mut units::Journal<B, P>,
        channel: &impl ConfidentialChannel,
        bytes: &PrivateBytes,
        grants: &impl Grants,
        now: u64,
        validate_owner_grant: bool,
    ) -> Result<Checked, Error> {
        let request = Request::decode(bytes.as_bytes())?;
        let req = &request.body;
        let config = journal.configuration();
        let descriptor = config
            .units()
            .iter()
            .find(|d| d.id == request.unit)
            .ok_or(Error::Unauthorized)?;
        if channel.domain() != req.domain
            || req.domain != config.domain()
            || descriptor.ledger.policy != req.policy
            || channel.binding() == [0; 32]
            || req.session != channel.binding()
            || now >= channel.expires_at()
            || now >= req.expires_at
            || req.expires_at > channel.expires_at()
            || req.expires_at.saturating_sub(now) > self.maximum_auth_lifetime
        {
            return Err(Error::Unauthorized);
        }
        VerifyingKey::from_bytes(&req.signer)
            .map_err(|_| Error::Unauthorized)?
            .verify_strict(&request.message(), &Signature::from_bytes(&req.signature))
            .map_err(|_| Error::Unauthorized)?;
        let head = journal.head();
        let state = journal.verified_state().map_err(|_| Error::Unavailable)?;
        if now < state.logical_time() {
            return Err(Error::Unavailable);
        }
        let owner = state
            .customers()
            .iter()
            .find(|c| c.account == req.account)
            .ok_or(Error::Unauthorized)?;
        let component = state.unit(request.unit).ok_or(Error::Unauthorized)?;
        if req.epoch == 0
            || owner.epoch != req.epoch
            || component.authority_epoch(req.account) != Some(req.epoch)
        {
            return Err(Error::Unauthorized);
        }
        let descriptor = state
            .configuration()
            .units()
            .iter()
            .find(|d| d.id == request.unit)
            .ok_or(Error::Unauthorized)?;
        let is_owner = req.signer == owner.wallet;
        let scope = Scope {
            domain: req.domain,
            unit: request.unit,
            account: req.account,
            epoch: req.epoch,
        };
        let grant = if is_owner {
            if validate_owner_grant && let Command::Grant(grant) = &req.command {
                self.validate_grant(grant, owner.wallet, &descriptor.ledger, now)?;
            }
            None
        } else {
            let cut = grants
                .lookup(scope, req.signer, head)?
                .ok_or(Error::Unauthorized)?;
            if cut.scope != scope || cut.grant.key != req.signer {
                return Err(Error::Unauthorized);
            }
            if cut.head != head
                || cut.accepted_at > state.logical_time()
                || cut.used_orders > cut.grant.maximum_orders
            {
                return Err(Error::Unavailable);
            }
            self.validate_grant(
                &cut.grant,
                owner.wallet,
                &descriptor.ledger,
                cut.accepted_at,
            )
            .map_err(|_| Error::Unavailable)?;
            authorization::permissions(&req.command, &cut.grant, now)?;
            Some(cut)
        };
        Ok(Checked {
            request,
            head,
            principal: if is_owner {
                Principal::Owner
            } else {
                Principal::Agent
            },
            grant,
        })
    }
    fn validate_grant(
        &self,
        g: &Grant,
        owner: [u8; 32],
        config: &cinder_kernel::ledger::Config,
        at: u64,
    ) -> Result<(), Error> {
        let key = VerifyingKey::from_bytes(&g.key).map_err(|_| Error::Invalid)?;
        if key.is_weak()
            || g.key == owner
            || g.methods == 0
            || g.methods & !7 != 0
            || g.maximum_lots == 0
            || g.maximum_fee < 0
            || g.maximum_orders == 0
            || g.expires_at <= at
            || g.expires_at.saturating_sub(at) > self.maximum_grant_lifetime
            || !config.markets.iter().any(|m| m.unit().market == g.market)
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}

/// Wrap an unchanged private response in an explicit bounded v2 unit envelope.
/// Only the established confidential channel authenticates this server output.
pub fn encode_response(unit: UnitId, response: &Response) -> Result<PrivateBytes, Error> {
    wrap(unit, response.encode()?)
}
/// Equally private/redacted v2 error; never emit an outer plaintext native error.
pub fn encode_error(unit: UnitId, error: Error) -> Result<PrivateBytes, Error> {
    wrap(unit, error.encode_private()?)
}
fn wrap(unit: UnitId, body: PrivateBytes) -> Result<PrivateBytes, Error> {
    if body.as_bytes().len() > cinder_journal::wire::MAX_RECORD - REPLY.len() - 32 {
        return Err(Error::Unavailable);
    }
    let mut w = Writer::new(REPLY);
    w.raw(&unit.bytes());
    w.raw(body.as_bytes());
    PrivateBytes::new(w.0.to_vec()).map_err(|_| Error::Unavailable)
}

macro_rules! private {
    ($($t:ty),*) => { $(impl fmt::Debug for $t {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("[PRIVATE UNIT API]") }
    })* };
}
private!(Request, Scope, GrantCut, Checked, Authorizer);
