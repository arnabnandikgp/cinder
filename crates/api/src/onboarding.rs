//! Separate owner-only bootstrap protocol over the existing confidential carrier.
use crate::{wire::Reader, *};
use cinder_journal::onboarding::{self, Action, Binding, Notice};
use ed25519_dalek::Signature;

/// Purpose-separated bootstrap protocol prefix.
pub const MAGIC: &[u8] = b"CINDER-ONBOARD\0\x00\x01";
// Classic SPL Token and Associated Token program addresses, independently checked
// against the existing generated-client SDK in the local acceptance suite.
const TOKEN: [u8; 32] = [
    6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172, 28, 180, 133, 237,
    95, 91, 55, 145, 58, 140, 245, 133, 126, 255, 0, 169,
];
const ATA: [u8; 32] = [
    140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142, 13, 131, 11, 90, 19, 153, 218,
    255, 16, 132, 4, 142, 123, 216, 219, 233, 248, 89,
];

/// Immutable opt-in policy; contains only the already-governed single route.
#[derive(Clone, PartialEq, Eq)]
pub struct Policy {
    /// Governed total participant capacity, not a financial limit.
    pub maximum_accounts: u16,
    /// Custody program deployment seed.
    pub custody_domain: [u8; 32],
    /// Immutable custody pool seed.
    pub pool: [u8; 32],
    /// Configured execution venue identity.
    pub venue: [u8; 32],
    /// Custody executable address.
    pub program: [u8; 32],
    /// Canonical custody configuration PDA.
    pub config: [u8; 32],
    /// Canonical registered venue-route PDA.
    pub route: [u8; 32],
    /// Supported classic-SPL quote mint.
    pub mint: [u8; 32],
    /// Exact mint atom scale.
    pub decimals: u8,
    /// Current authority epoch; never reset by reconnect.
    pub epoch: u64,
    /// Maximum full deposit batch in configured mint atoms, not perp notional.
    pub maximum_deposit: u64,
}
impl std::fmt::Debug for Policy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OnboardingPolicy([PRIVATE])")
    }
}
fn pda(program: [u8; 32], seeds: &[&[u8]]) -> Result<[u8; 32], Error> {
    for bump in (0..=255u8).rev() {
        let mut h = Sha256::new();
        for seed in seeds {
            h.update(seed);
        }
        h.update([bump]);
        h.update(program);
        h.update(b"ProgramDerivedAddress");
        let bytes = h.finalize().into();
        if VerifyingKey::from_bytes(&bytes).is_err() {
            return Ok(bytes);
        }
    }
    Err(Error::Invalid)
}
impl Policy {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.maximum_accounts == 0
            || self.maximum_accounts > 1024
            || self.decimals > 18
            || self.epoch == 0
            || self.maximum_deposit == 0
            || [
                self.custody_domain,
                self.pool,
                self.venue,
                self.program,
                self.config,
                self.route,
                self.mint,
            ]
            .contains(&[0; 32])
            || pda(
                self.program,
                &[
                    b"cinder_vault",
                    &self.custody_domain,
                    &self.pool,
                    &self.mint,
                ],
            )? != self.config
            || pda(self.program, &[b"venue_route", &self.config])? != self.route
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
    pub(crate) fn encode(&self, w: &mut Writer) {
        w.raw(&self.maximum_accounts.to_be_bytes());
        for key in [
            self.custody_domain,
            self.pool,
            self.venue,
            self.program,
            self.config,
            self.route,
            self.mint,
        ] {
            w.raw(&key);
        }
        w.byte(self.decimals);
        w.u64(self.epoch);
        w.u64(self.maximum_deposit);
    }
    /// Canonical classic-SPL recipient; neither the request nor an agent chooses it.
    pub fn recipient(&self, owner: [u8; 32]) -> Result<[u8; 32], Error> {
        pda(ATA, &[&owner, &TOKEN, &self.mint])
    }
}
/// A fresh owner proof, independent of an as-yet unknown private account/epoch.
#[derive(Clone, PartialEq, Eq)]
pub struct Request {
    /// Expected network and deployment.
    pub domain: Domain,
    /// Approved economic policy revision.
    pub policy: PolicyVersion,
    /// Owner Ed25519 wallet verification key.
    pub wallet: [u8; 32],
    /// Fresh verified confidential-channel transcript.
    pub session: [u8; 32],
    /// Short authentication deadline in injected milliseconds.
    pub expires_at: u64,
    /// Exact owner-only bootstrap action.
    pub command: Command,
    /// Original signature; not evidence of execution.
    pub signature: [u8; 64],
}
#[derive(Clone, PartialEq, Eq)]
/// Bounded owner-only bootstrap and original-locator actions.
pub enum Command {
    /// Create once or retrieve the existing owner account.
    Open,
    /// Report original chain identity; no funds are credited by this hint.
    Deposit {
        /// Current private-account authority epoch.
        epoch: u64,
        /// Original exact-batch deposit identity.
        operation: [u8; 32],
        /// Original transaction signature, never regenerated by the server.
        signature: [u8; 64],
    },
}
impl Request {
    /// Canonical wallet-signed purpose/domain/session preimage.
    pub fn message(&self) -> zeroize::Zeroizing<Vec<u8>> {
        let mut w = Writer::new(MAGIC);
        w.raw(&self.domain.network.bytes());
        w.raw(&self.domain.deployment.bytes());
        w.raw(&self.policy.get().to_be_bytes());
        w.raw(&self.wallet);
        w.raw(&self.session);
        w.u64(self.expires_at);
        match self.command {
            Command::Open => w.byte(0),
            Command::Deposit {
                epoch,
                operation,
                signature,
            } => {
                w.byte(1);
                w.u64(epoch);
                w.raw(&operation);
                w.raw(&signature);
            }
        }
        w.0
    }
    /// Bounded private carrier payload, signature included.
    pub fn encode(&self) -> Result<PrivateBytes, Error> {
        PrivateBytes::new([self.message().as_slice(), &self.signature].concat())
            .map_err(|_| Error::Invalid)
    }
    /// Decode strictly, rejecting trailing or oversized payloads.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > 512 {
            return Err(Error::Invalid);
        }
        let mut r = Reader::new(bytes, MAGIC)?;
        let domain = Domain {
            network: NetworkId::new(r.array()?).map_err(|_| Error::Invalid)?,
            deployment: DeploymentId::new(r.array()?).map_err(|_| Error::Invalid)?,
        };
        let policy =
            PolicyVersion::new(u32::from_be_bytes(r.array()?)).map_err(|_| Error::Invalid)?;
        let wallet = r.array()?;
        let session = r.array()?;
        let expires_at = r.u64()?;
        let command = match r.byte()? {
            0 => Command::Open,
            1 => Command::Deposit {
                epoch: r.u64()?,
                operation: r.array()?,
                signature: r.array()?,
            },
            _ => return Err(Error::Invalid),
        };
        let signature = r.array()?;
        r.done()?;
        Ok(Self {
            domain,
            policy,
            wallet,
            session,
            expires_at,
            command,
            signature,
        })
    }
}
/// Private bootstrap response; no pooled balance, native credential or owner list.
#[derive(Clone, PartialEq, Eq)]
pub struct Discovery {
    /// Private deployment-scoped customer identity.
    pub account: AccountId,
    /// Current authority epoch; never reset by reconnect.
    pub epoch: u64,
    /// Canonical owner token recipient.
    pub recipient: [u8; 32],
    /// Loaded immutable custody policy.
    pub custody: Policy,
    /// Owner-reported originals still awaiting qualified inventory. No amount or credit.
    pub pending_deposits: Vec<[u8; 32]>,
}
impl Discovery {
    pub(crate) fn encode(&self, w: &mut Writer) {
        w.raw(&self.account.bytes());
        w.u64(self.epoch);
        w.raw(&self.recipient);
        self.custody.encode(w);
        w.u64(self.pending_deposits.len() as u64);
        for operation in &self.pending_deposits {
            w.raw(operation);
        }
    }
}
impl<A: Admission> Service<A> {
    /// Explicit opt-in constructor, including a zero-customer genesis.
    pub fn new_onboarding(contract: Contract, admission: A, policy: Policy) -> Result<Self, Error> {
        // New-history composition only. Existing configured customers require
        // an explicit migration, not implicit rebinding of their paid basis.
        if !contract.owners.is_empty() {
            return Err(Error::Invalid);
        }
        policy.validate()?;
        Self::construct(contract, admission, Some(policy))
    }
    /// Loaded policy, not a caller-supplied enable flag.
    pub fn onboarding_policy(&self) -> Option<&Policy> {
        self.onboarding.as_ref()
    }
    pub(crate) fn binding(&self, state: &State, account: AccountId) -> Result<OwnerBinding, Error> {
        if let Some(b) = self.contract.owners.iter().find(|b| b.account == account) {
            return Ok(b.clone());
        }
        if self.onboarding.is_none() {
            return Err(Error::Unauthorized);
        }
        let b = state
            .registrations()
            .iter()
            .find(|b| b.account == account)
            .ok_or(Error::Unauthorized)?;
        Ok(OwnerBinding {
            account: b.account,
            wallet: b.wallet,
            tokens: b.tokens,
        })
    }
    pub(crate) fn onboard<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        channel: &impl ConfidentialChannel,
        bytes: &PrivateBytes,
        now: u64,
    ) -> Result<Response, Error> {
        let policy = self.onboarding.as_ref().ok_or(Error::Unauthorized)?;
        let r = Request::decode(bytes.as_bytes())?;
        let key = VerifyingKey::from_bytes(&r.wallet).map_err(|_| Error::Unauthorized)?;
        if key.is_weak()
            || r.domain != j.configuration().domain
            || r.domain != channel.domain()
            || r.policy != j.configuration().policy
            || r.session == [0; 32]
            || r.session != channel.binding()
            || now >= r.expires_at
            || now >= channel.expires_at()
            || r.expires_at > channel.expires_at()
            || r.expires_at.saturating_sub(now) > self.contract.maximum_auth_lifetime
        {
            return Err(Error::Unauthorized);
        }
        key.verify_strict(&r.message(), &Signature::from_bytes(&r.signature))
            .map_err(|_| Error::Unauthorized)?;
        j.verified_state().map_err(|_| Error::Unavailable)?;
        self.records(j)?;
        let account = onboarding::account(r.domain, r.wallet).map_err(|_| Error::Invalid)?;
        let state = j.state().map_err(|_| Error::Unavailable)?;
        if now < state.logical_time() {
            return Err(Error::Unavailable);
        }
        let registered = state.registrations().iter().find(|b| b.account == account);
        let recipient = policy.recipient(r.wallet)?;
        if registered.is_some_and(|b| b.wallet != r.wallet || b.tokens != recipient)
            || self
                .contract
                .owners
                .iter()
                .any(|b| b.wallet == r.wallet || b.account == account || b.tokens == recipient)
        {
            return Err(Error::Conflict);
        }
        let action = match r.command {
            Command::Open if registered.is_none() => {
                // V1 recovery delivery uses a complete immutable key registry.
                // Never silently enroll a liability absent from that inventory.
                if j.transactions().any(|tx| {
                    tx.evidence
                        .iter()
                        .any(|e| e.as_bytes().starts_with(b"CINDER_RECOVERY_KEYS_V1\0"))
                }) {
                    return Err(Error::Unavailable);
                }
                if state.ledger().customer_ids().count() >= usize::from(policy.maximum_accounts) {
                    return Err(Error::Unavailable);
                }
                Some(Action::Register(Binding {
                    account,
                    wallet: r.wallet,
                    tokens: recipient,
                }))
            }
            Command::Open => None,
            Command::Deposit {
                epoch,
                operation,
                signature,
            } => {
                if registered.is_none() || state.authority_epoch(account) != Some(epoch) {
                    return Err(Error::Unauthorized);
                }
                if let Some(old) = state
                    .deposit_notices()
                    .iter()
                    .find(|n| n.account == account && n.operation == operation)
                {
                    if old.signature != signature {
                        return Err(Error::Conflict);
                    }
                    None
                } else {
                    Some(Action::Notice(Notice {
                        account,
                        operation,
                        signature,
                    }))
                }
            }
        };
        if let Some(action) = action {
            let mut identity = Writer::new(b"CINDER-ONBOARD-COMMIT-1\0");
            identity.raw(&self.fingerprint(r.domain, r.policy));
            identity.raw(&account.bytes());
            match &action {
                Action::Register(_) => identity.byte(0),
                Action::Notice(n) => {
                    identity.byte(1);
                    identity.raw(&n.operation);
                }
            }
            let mut tx = empty(
                j,
                CommitId::new(Sha256::digest(&*identity.0).into()).map_err(|_| Error::Invalid)?,
                now,
            );
            let register = matches!(action, Action::Register(_));
            tx.controls.push(Control::Onboarding(action));
            if register {
                tx.controls
                    .push(Control::Order(orders::Action::AdvanceAuthority {
                        account,
                        epoch: 1,
                    }));
            }
            tx.evidence.push(r.encode()?);
            let committed = j.commit(tx).map_err(|_| Error::Unavailable)?;
            if committed.receipt.controls.is_some() {
                return Err(Error::Unavailable);
            }
        }
        let state = j.state().map_err(|_| Error::Unavailable)?;
        Ok(Response::Discovery(Box::new(Discovery {
            account,
            epoch: state.authority_epoch(account).ok_or(Error::Unavailable)?,
            recipient,
            custody: policy.clone(),
            pending_deposits: state
                .deposit_notices()
                .iter()
                .filter(|n| {
                    n.account == account
                        && !state.escrows().iter().any(|e| {
                            e.batch.request.account == account
                                && e.batch.operation == n.operation
                                && e.batch.signature == n.signature
                        })
                })
                .map(|n| n.operation)
                .collect(),
        })))
    }
}
