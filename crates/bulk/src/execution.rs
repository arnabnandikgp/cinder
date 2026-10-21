//! Agent-only signing after shared durable exposure. No HTTP client, key loader,
//! automatic nonce, master-key fallback, registration, transfer or live permission.
use crate::{
    Error, Network,
    financial::Profile,
    wire::{Action, TimeInForce},
};
use cinder_journal::{
    controller::Controller,
    model::*,
    orders::{self, Status},
};
use cinder_kernel::{codec::Canonical, identity::*, ledger::Config};
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

const MAGIC: &[u8] = b"CINDER-BULK-EXECUTION-1\0";
const WINDOW: u64 = 60_000;
/// Independently governed qualification; construction does not establish it.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Level {
    /// Unsupported/unverified; cannot construct an active signer.
    Unknown,
    /// Bound evidence qualified for the exact release/environment (synthetic in tests).
    Qualified,
}
/// No default active policy or inferred upstream rate limit.
#[derive(Clone)]
pub struct Policy {
    /// Exact immutable execution unit; legacy/default-unit use is refused.
    pub unit: UnitId,
    /// Nonzero source/interpretation authority revision.
    pub revision: u64,
    /// Independently reviewed evidence commitment, not a host permission boolean.
    pub evidence: [u8; 32],
    /// Native signing/agent/fill/fee semantics qualification for this profile.
    pub qualification: Level,
    /// Local one-shot egress lifetime, 1..30000 ms. Native raw signatures do not
    /// contain this deadline and an already escaped request is not revoked by it.
    pub expiry_ms: u64,
    /// This unit's allocation from a separately qualified shared egress/IP
    /// allowance per 60000ms. Never clone a whole venue limit into each unit.
    pub credits: u32,
    /// Capacity unavailable to new orders, preserved for scoped cancellation.
    pub cleanup_reserve: u32,
}
/// Already prepared attempt and explicit injected native nonce/clock.
#[derive(Clone, Copy)]
pub struct Dispatch {
    /// Original durable private attempt, not a raw native order.
    pub attempt: AttemptKey,
    /// New common journal commit; exact retry never yields a second delivery.
    pub commit: CommitId,
    /// Explicit native u64 nonce, strictly increasing across this account history.
    pub nonce: u64,
    /// Trusted current milliseconds; never parsed from a host request.
    pub at: u64,
}
/// One-shot signed private native request. Must remain inside enclave TLS egress.
pub struct Outbound {
    network: Network,
    body: PrivateBytes,
    expires_at: u64,
}
impl Outbound {
    /// Exact bound signing network. No URL may be supplied by the response.
    pub fn network(&self) -> Network {
        self.network
    }
    /// Fixed documented unified transaction path.
    pub fn path(&self) -> &'static str {
        "/api/v1/order"
    }
    /// Private signed body, never a parent relay/log payload.
    pub fn body(&self) -> &[u8] {
        self.body.as_bytes()
    }
    /// Local egress deadline, not an upstream signature-revocation guarantee.
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }
}
/// Non-clone response correlation, not authority to sign/resend or settle fills.
pub struct Completion {
    dispatch: Dispatch,
    plan: Plan,
    evidence: PrivateBytes,
}
/// Native ACK only; it never posts a position or certifies terminal history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Scoped native acceptance, actual executions still outstanding.
    Acknowledged,
    /// Scoped cancel ACK; late fills and holds remain.
    CancelAcknowledged,
    /// Timeout/error/malformed/ambiguous result; no resend authority.
    Unknown,
}
/// One bounded authenticated response supplied by private TLS egress.
pub enum Reply {
    /// The original may have escaped; preserve its exposure and holds.
    Unknown,
    /// Actual native response; caller retains no credential headers in the body.
    Response {
        /// Actual HTTP code.
        status: u16,
        /// Exact bounded body.
        body: PrivateBytes,
        /// Trusted receive milliseconds, not a native timestamp.
        received_at: u64,
        /// Optional bounded cooldown duration from authenticated response headers.
        retry_after_ms: Option<u64>,
    },
}
/// Private one-shot authenticated egress. No retries, redirects or parent plaintext.
pub trait Transport {
    /// Consume the signed request once, checking its local expiry before writing.
    fn post(&mut self, request: Outbound) -> Reply;
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Plan {
    attempt: Vec<u8>,
    original: Vec<u8>,
    abstract_action: Vec<u8>,
    message: Vec<u8>,
    action: serde_json::Value,
    order: [u8; 32],
    nonce: u64,
    agent: [u8; 32],
    epoch: u64,
    cancel: bool,
}
#[derive(Serialize, Deserialize)]
struct Archive {
    contract: [u8; 32],
    record: Record,
}
#[derive(Serialize, Deserialize)]
enum Record {
    Read {
        request: Vec<u8>,
        kind: u8,
        cleanup: bool,
    },
    ReadResponse {
        reservation: [u8; 32],
        body: [u8; 32],
        code: Option<u16>,
    },
    Epoch {
        epoch: u64,
        agent: Option<[u8; 32]>,
    },
    Expose(Plan),
    Response {
        exposure: [u8; 32],
        code: Option<u16>,
    },
    Cooldown {
        until: u64,
    },
}
struct History {
    epoch: u64,
    agent: Option<[u8; 32]>,
    nonce: u64,
    used: u32,
    blocked: u64,
    plans: Vec<Plan>,
}
/// Loaded native agent with immutable financial/source/qualification bindings.
/// Its actual upstream permissions must be separately established; a local
/// trading policy cannot narrow a credential that also has funds powers.
pub struct Gateway {
    config: Config,
    profile: Profile,
    policy: Policy,
    key: SigningKey,
    epoch: u64,
    contract: [u8; 32],
}
macro_rules! redacted { ($($t:ty),*)=>{$(impl std::fmt::Debug for $t {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { f.write_str("[PRIVATE BULK EXECUTION]") }
})*}; }
redacted!(
    Policy, Dispatch, Outbound, Completion, Reply, Gateway, Level
);
impl Gateway {
    /// Explicit qualified profile and enclave-owned agent seed. No file/env or
    /// owner-key loading. A master key is always refused, never used as fallback.
    pub fn new(
        config: Config,
        profile: Profile,
        policy: Policy,
        seed: Zeroizing<[u8; 32]>,
        epoch: u64,
    ) -> Result<Self, Error> {
        if policy.qualification != Level::Qualified {
            return Err(Error::Unqualified);
        }
        Self::construct(config, profile, policy, seed, epoch)
    }
    /// Testnet assembly/read capture only. Unknown interpretation may archive raw
    /// observations but can neither expose a signature nor post financial fills.
    pub fn new_read_only(
        config: Config,
        profile: Profile,
        policy: Policy,
        seed: Zeroizing<[u8; 32]>,
        epoch: u64,
    ) -> Result<Self, Error> {
        if policy.qualification != Level::Unknown || profile.scope.network != Network::Testnet {
            return Err(Error::Unqualified);
        }
        Self::construct(config, profile, policy, seed, epoch)
    }
    fn construct(
        config: Config,
        profile: Profile,
        policy: Policy,
        seed: Zeroizing<[u8; 32]>,
        epoch: u64,
    ) -> Result<Self, Error> {
        let key = SigningKey::from_bytes(&seed);
        if epoch == 0
            || policy.revision == 0
            || policy.evidence == [0; 32]
            || !(1..=30000).contains(&policy.expiry_ms)
            || policy.cleanup_reserve == 0
            || policy.credits <= policy.cleanup_reserve
            || key.verifying_key().to_bytes() == profile.scope.account()
            || matches!(profile.target,crate::financial::Target::SubAccount {parent} if parent==key.verifying_key().to_bytes())
            || config.quote != profile.quote
            || config.markets.len() != profile.mappings.len()
            || config.markets.is_empty()
            || profile.scope.source.domain != config.domain
            || profile.scope.source.venue != config.venue
            || profile.scope.source.account != config.venue_account
            || !config
                .sources
                .iter()
                .any(|s| s.scope == profile.scope.source)
        {
            return Err(Error::Unqualified);
        }
        for native in &profile.mappings {
            let market = config
                .markets
                .iter()
                .find(|m| m.unit() == native.unit)
                .ok_or(Error::Unqualified)?;
            let (n, d) = market.conversion();
            let left = u128::from(native.wire.size_step)
                .checked_mul(u128::from(native.wire.price_step))
                .and_then(|v| v.checked_mul(10_u128.checked_pow(u32::from(profile.places))?))
                .and_then(|v| v.checked_mul(u128::from(d)))
                .ok_or(Error::Limit)?;
            let right = u128::from(n)
                .checked_mul(100_000_000_u128.pow(2))
                .ok_or(Error::Limit)?;
            if left != right {
                return Err(Error::Unqualified);
            }
        }
        let mut hash = Sha256::new();
        hash.update(MAGIC);
        hash.update(profile.commitment());
        hash.update(cinder_journal::wire::encode_config(&config).map_err(|_| Error::Invalid)?);
        hash.update(policy.unit.bytes());
        hash.update(policy.revision.to_be_bytes());
        hash.update(policy.evidence);
        hash.update([policy.qualification as u8]);
        hash.update(policy.expiry_ms.to_be_bytes());
        hash.update(policy.credits.to_be_bytes());
        hash.update(policy.cleanup_reserve.to_be_bytes());
        Ok(Self {
            config,
            profile,
            policy,
            key,
            epoch,
            contract: hash.finalize().into(),
        })
    }
    /// Public agent only; never exports the seed or arbitrary signing service.
    pub fn agent_key(&self) -> [u8; 32] {
        self.key.verifying_key().to_bytes()
    }
    // Immutable binding only; paper initialization cannot mutate a loaded
    // gateway's profile/policy without rebuilding its release commitment.
    pub(crate) fn paper_binding(&self) -> (&Config, &Profile, &Policy) {
        (&self.config, &self.profile, &self.policy)
    }
    pub(crate) fn observation_binding(
        &self,
        j: &impl Controller,
    ) -> Result<(&Profile, u64), Error> {
        self.binding(j)?;
        Ok((&self.profile, self.policy.revision))
    }
    pub(crate) fn financial_observations(&self) -> bool {
        self.policy.qualification == Level::Qualified
    }
    /// Fresh durable capacity preflight only; no reservation/refund or signing.
    pub fn read_available(
        &self,
        j: &mut impl Controller,
        at: u64,
        cleanup: bool,
    ) -> Result<bool, Error> {
        let h = self.history(j, at)?;
        let limit = if cleanup {
            self.policy.credits
        } else {
            self.policy.credits - self.policy.cleanup_reserve
        };
        Ok(at >= h.blocked && h.used < limit)
    }
    /// Durable nonce floor for an explicit trusted scheduler. Never resets on
    /// restart/rotation; this does not allocate a nonce or expose a signature.
    pub fn last_nonce(&self, j: &mut impl Controller, at: u64) -> Result<u64, Error> {
        Ok(self.history(j, at)?.nonce)
    }
    /// Validate the exact planned action and exposure without accepting it.
    /// A later dispatch still performs every check and the durable commit.
    pub fn dispatch_available(&self, j: &mut impl Controller, d: Dispatch) -> Result<bool, Error> {
        if !self.financial_observations() {
            return Err(Error::Unqualified);
        }
        let h = self.history(j, d.at)?;
        self.active(&h)?;
        let plan = match self.plan_at(j, &h, d, false) {
            Ok(p) => p,
            Err(Error::Unqualified) => return Ok(false),
            Err(e) => return Err(e),
        };
        if !self.read_available(j, d.at, plan.cancel)? {
            return Ok(false);
        }
        let mut tx = self.transaction(j, d.commit, d.at, Record::Expose(plan))?;
        tx.controls.push(Control::Expose(d.attempt));
        Ok(j.preview(&tx)
            .map_err(|_| Error::Unavailable)?
            .controls
            .is_none())
    }
    pub(crate) fn attribution(
        &self,
        j: &mut impl Controller,
        order: [u8; 32],
        at: u64,
    ) -> Result<Option<AttemptKey>, Error> {
        let h = self.history(j, at)?;
        h.plans
            .iter()
            .find(|p| !p.cancel && p.order == order)
            .map(|p| AttemptKey::decode(&p.attempt).map_err(|_| Error::Invalid))
            .transpose()
    }
    /// Exact loaded release component; independently included in measured config.
    pub fn release_commitment(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"CINDER-BULK-LOADED-1\0");
        h.update(self.contract);
        h.update(self.epoch.to_be_bytes());
        h.update(self.agent_key());
        h.finalize().into()
    }
    fn binding(&self, j: &impl Controller) -> Result<(), Error> {
        let d = j.descriptor().ok_or(Error::Unqualified)?;
        if j.configuration() != &self.config
            || d.id != self.policy.unit
            || d.native_account != self.profile.scope.account()
            || d.environment != self.profile.scope.network.commitment()
        {
            return Err(Error::Unqualified);
        }
        Ok(())
    }
    fn archive(&self, record: Record) -> Result<PrivateBytes, Error> {
        let mut bytes = MAGIC.to_vec();
        bytes.extend(
            serde_json::to_vec(&Archive {
                contract: self.contract,
                record,
            })
            .map_err(|_| Error::Invalid)?,
        );
        PrivateBytes::new(bytes).map_err(|_| Error::Limit)
    }
    fn history(&self, j: &mut impl Controller, at: u64) -> Result<History, Error> {
        self.binding(j)?;
        j.verified_state().map_err(|_| Error::Unavailable)?;
        let mut h = History {
            epoch: 0,
            agent: None,
            nonce: 0,
            used: 0,
            blocked: 0,
            plans: vec![],
        };
        for (tx, receipt) in j.transactions_with_receipts() {
            if tx.at > at {
                return Err(Error::Unqualified);
            }
            for e in &tx.evidence {
                let Some(body) = e.as_bytes().strip_prefix(MAGIC) else {
                    continue;
                };
                let a: Archive = serde_json::from_slice(body).map_err(|_| Error::Invalid)?;
                if a.contract != self.contract {
                    return Err(Error::Unqualified);
                }
                match a.record {
                    Record::Epoch { epoch, agent } => {
                        if epoch <= h.epoch || !tx.controls.is_empty() || receipt.controls.is_some()
                        {
                            return Err(Error::Unqualified);
                        }
                        h.epoch = epoch;
                        h.agent = agent;
                    }
                    Record::Expose(plan) if receipt.controls.is_none() => {
                        if plan.nonce <= h.nonce
                            || plan.epoch != h.epoch
                            || Some(plan.agent) != h.agent
                        {
                            return Err(Error::Unqualified);
                        }
                        let attempt =
                            AttemptKey::decode(&plan.attempt).map_err(|_| Error::Invalid)?;
                        if tx.controls.as_slice() != [Control::Expose(attempt)]
                            || self.plan_at(
                                j,
                                &h,
                                Dispatch {
                                    attempt,
                                    commit: tx.id,
                                    nonce: plan.nonce,
                                    at: tx.at,
                                },
                                true,
                            )? != plan
                        {
                            return Err(Error::Unqualified);
                        }
                        h.nonce = plan.nonce;
                        if at - tx.at < WINDOW + self.policy.expiry_ms {
                            h.used = h.used.checked_add(1).ok_or(Error::Limit)?;
                        }
                        h.plans.push(plan);
                    }
                    Record::Cooldown { until } => h.blocked = h.blocked.max(until),
                    Record::Read { .. } => {
                        if !tx.controls.is_empty() || receipt.controls.is_some() {
                            return Err(Error::Unqualified);
                        }
                        if at - tx.at < WINDOW + self.policy.expiry_ms {
                            h.used = h.used.checked_add(1).ok_or(Error::Limit)?;
                        }
                    }
                    Record::Expose(_) | Record::Response { .. } | Record::ReadResponse { .. } => (),
                }
            }
        }
        Ok(h)
    }
    fn active(&self, h: &History) -> Result<(), Error> {
        if h.epoch != self.epoch || h.agent != Some(self.agent_key()) {
            Err(Error::Unqualified)
        } else {
            Ok(())
        }
    }
    fn transaction(
        &self,
        j: &impl Controller,
        id: CommitId,
        at: u64,
        record: Record,
    ) -> Result<Transaction, Error> {
        Ok(Transaction {
            id,
            expected: j.head(),
            at,
            evidence: vec![self.archive(record)?],
            inputs: vec![],
            order_observations: vec![],
            funds_observations: vec![],
            controls: vec![],
        })
    }
    /// Operator-qualified local activation/rotation. This is NOT an upstream
    /// agent registration or live-run approval; old nonces/credits never reset.
    pub fn initialize(&self, j: &mut impl Controller, id: CommitId, at: u64) -> Result<(), Error> {
        let h = self.history(j, at)?;
        if h.epoch == self.epoch {
            return self.active(&h);
        }
        if h.epoch >= self.epoch || j.transaction(id).is_some() {
            return Err(Error::Unqualified);
        }
        let tx = self.transaction(
            j,
            id,
            at,
            Record::Epoch {
                epoch: self.epoch,
                agent: Some(self.agent_key()),
            },
        )?;
        j.commit(tx).map_err(|_| Error::Unavailable)?;
        Ok(())
    }
    /// Fence the local key across replay. An already escaped signature/agent
    /// must also be revoked upstream through separately approved authority.
    pub fn deactivate(&self, j: &mut impl Controller, id: CommitId, at: u64) -> Result<(), Error> {
        self.active(&self.history(j, at)?)?;
        let tx = self.transaction(
            j,
            id,
            at,
            Record::Epoch {
                epoch: self.epoch.checked_add(1).ok_or(Error::Limit)?,
                agent: None,
            },
        )?;
        j.commit(tx).map_err(|_| Error::Unavailable)?;
        Ok(())
    }
    pub(crate) fn reserve_read(
        &self,
        j: &mut impl Controller,
        id: CommitId,
        at: u64,
        kind: u8,
        body: &PrivateBytes,
        cleanup: bool,
    ) -> Result<(PrivateBytes, u64), Error> {
        if j.transaction(id).is_some() {
            return Err(Error::Unqualified);
        }
        let h = self.history(j, at)?;
        let cap = if cleanup {
            self.policy.credits
        } else {
            self.policy.credits - self.policy.cleanup_reserve
        };
        if at < h.blocked || h.used >= cap {
            return Err(Error::Limit);
        }
        let tx = self.transaction(
            j,
            id,
            at,
            Record::Read {
                request: body.as_bytes().to_vec(),
                kind,
                cleanup,
            },
        )?;
        let marker = tx.evidence[0].clone();
        let committed = j.commit(tx).map_err(|_| Error::Unavailable)?;
        if committed.duplicate || committed.receipt.controls.is_some() {
            return Err(Error::Unqualified);
        }
        Ok((
            marker,
            at.checked_add(self.policy.expiry_ms).ok_or(Error::Limit)?,
        ))
    }
    pub(crate) fn read_result(
        &self,
        id: CommitId,
        body: [u8; 32],
        code: Option<u16>,
        cooldown: Option<u64>,
    ) -> Result<Vec<PrivateBytes>, Error> {
        let mut evidence = vec![self.archive(Record::ReadResponse {
            reservation: id.bytes(),
            body,
            code,
        })?];
        if let Some(until) = cooldown {
            evidence.push(self.archive(Record::Cooldown { until })?);
        }
        Ok(evidence)
    }
    pub(crate) fn read_succeeded(
        &self,
        tx: &Transaction,
        body_hash: [u8; 32],
    ) -> Result<bool, Error> {
        let mut successes = 0;
        for e in &tx.evidence {
            if let Some(bytes) = e.as_bytes().strip_prefix(MAGIC) {
                let a: Archive = serde_json::from_slice(bytes).map_err(|_| Error::Invalid)?;
                if a.contract != self.contract {
                    return Err(Error::Unqualified);
                }
                if let Record::ReadResponse {
                    body,
                    code: Some(200),
                    ..
                } = a.record
                    && body == body_hash
                {
                    successes += 1;
                }
            }
        }
        Ok(successes == 1)
    }
    fn plan_at(
        &self,
        j: &impl Controller,
        h: &History,
        d: Dispatch,
        exposed: bool,
    ) -> Result<Plan, Error> {
        let state = j.state().map_err(|_| Error::Unavailable)?;
        let attempt = state
            .attempts()
            .iter()
            .find(|a| {
                a.key == d.attempt
                    && a.possibly_exposed == exposed
                    && (!exposed || a.exposed_at == Some(d.at))
            })
            .ok_or(Error::Unqualified)?;
        if d.nonce == 0 || d.nonce <= h.nonce || d.at >= attempt.expires_at {
            return Err(Error::Unqualified);
        }
        let order = state
            .orders()
            .iter()
            .find(|o| o.intent.request == d.attempt.request)
            .ok_or(Error::Unqualified)?;
        let m = self
            .profile
            .mappings
            .iter()
            .find(|m| m.unit == order.intent.quantity.unit())
            .ok_or(Error::Unqualified)?;
        let (action, original, cancel) = match attempt.kind {
            AttemptKind::Order => {
                let i = &order.intent;
                let buy = i.quantity.lots() > 0;
                let price = if buy {
                    i.maximum.ticks()
                } else {
                    i.minimum.ticks()
                };
                let tif = match i.time_in_force {
                    orders::TimeInForce::GoodTilCancelled => TimeInForce::Gtc,
                    orders::TimeInForce::AddLiquidityOnly => TimeInForce::Alo,
                    orders::TimeInForce::ImmediateOrCancel => TimeInForce::Ioc,
                };
                (
                    Action::limit(
                        &m.wire,
                        buy,
                        &format_grid(price, m.wire.price_step)?,
                        &format_grid(i.quantity.magnitude(), m.wire.size_step)?,
                        tif,
                    )?,
                    d.attempt,
                    false,
                )
            }
            AttemptKind::Cancel => {
                let original = order.attempt.ok_or(Error::Unqualified)?;
                let old = h
                    .plans
                    .iter()
                    .find(|p| p.attempt == original.encode() && !p.cancel)
                    .ok_or(Error::Unqualified)?;
                (Action::cancel(m.wire.symbol(), old.order)?, original, true)
            }
            _ => return Err(Error::Unsupported),
        };
        let native_order = if cancel {
            h.plans
                .iter()
                .find(|p| p.attempt == original.encode() && !p.cancel)
                .ok_or(Error::Unqualified)?
                .order
        } else {
            action.order_id(&self.profile.scope, d.nonce)?
        };
        Ok(Plan {
            attempt: d.attempt.encode(),
            original: original.encode(),
            abstract_action: attempt.message.as_bytes().to_vec(),
            message: action.message(&self.profile.scope, d.nonce)?,
            action: serde_json::from_slice(&action.json()?).map_err(|_| Error::Invalid)?,
            order: native_order,
            nonce: d.nonce,
            agent: h.agent.ok_or(Error::Unqualified)?,
            epoch: h.epoch,
            cancel,
        })
    }
    /// Commit exact plan/nonce/spend and possible exposure BEFORE signing. No
    /// request is returned after unknown storage, a duplicate or stale authority.
    pub fn prepare_dispatch(
        &self,
        j: &mut impl Controller,
        d: Dispatch,
    ) -> Result<(Outbound, Completion), Error> {
        if !self.financial_observations() || j.transaction(d.commit).is_some() {
            return Err(Error::Unqualified);
        }
        let h = self.history(j, d.at)?;
        self.active(&h)?;
        let plan = self.plan_at(j, &h, d, false)?;
        let cap = if plan.cancel {
            self.policy.credits
        } else {
            self.policy.credits - self.policy.cleanup_reserve
        };
        if d.at < h.blocked || h.used >= cap {
            return Err(Error::Limit);
        }
        let mut tx = self.transaction(j, d.commit, d.at, Record::Expose(plan.clone()))?;
        tx.controls.push(Control::Expose(d.attempt));
        let evidence = tx.evidence[0].clone();
        let committed = j.commit(tx).map_err(|_| Error::Unavailable)?;
        if committed.duplicate
            || committed.receipt.controls.is_some()
            || committed.exposures.len() != 1
        {
            return Err(Error::Unqualified);
        }
        let (key, message) = committed
            .exposures
            .into_iter()
            .next()
            .ok_or(Error::Unqualified)?
            .into_message();
        if key != d.attempt || message.as_bytes() != plan.abstract_action {
            return Err(Error::Unqualified);
        }
        self.active(&self.history(j, d.at)?)?;
        let signature = self.key.sign(&plan.message);
        let body=serde_json::to_vec(&serde_json::json!({"actions":[plan.action.clone()],"nonce":plan.nonce.to_string(),
            "account":bs58::encode(self.profile.scope.account()).into_string(),"signer":bs58::encode(self.agent_key()).into_string(),"signature":bs58::encode(signature.to_bytes()).into_string()})).map_err(|_|Error::Invalid)?;
        let attempt = j
            .state()
            .map_err(|_| Error::Unavailable)?
            .attempts()
            .iter()
            .find(|a| a.key == d.attempt)
            .ok_or(Error::Unqualified)?;
        let expires_at =
            d.at.checked_add(self.policy.expiry_ms)
                .ok_or(Error::Limit)?
                .min(attempt.expires_at);
        Ok((
            Outbound {
                network: self.profile.scope.network,
                body: PrivateBytes::new(body).map_err(|_| Error::Limit)?,
                expires_at,
            },
            Completion {
                dispatch: d,
                plan,
                evidence,
            },
        ))
    }
    /// Retain a real late response even after local key rotation. ACK is not a
    /// fill/terminal frontier; timeouts, 429 and malformed replies retain holds.
    pub fn complete_dispatch(
        &self,
        j: &mut impl Controller,
        c: Completion,
        reply: Reply,
        at: u64,
    ) -> Result<Outcome, Error> {
        self.history(j, at)?;
        let original = j.transaction(c.dispatch.commit).ok_or(Error::Unqualified)?;
        if !original.evidence.iter().any(|e| e == &c.evidence)
            || j.transaction_receipt(c.dispatch.commit)
                .is_none_or(|r| r.controls.is_some())
        {
            return Err(Error::Unqualified);
        }
        let (outcome, raw, received, code, cooldown) = match reply {
            Reply::Unknown => (
                Outcome::Unknown,
                PrivateBytes::new(b"transport outcome unknown".to_vec())
                    .map_err(|_| Error::Limit)?,
                c.dispatch.at,
                None,
                None,
            ),
            Reply::Response {
                status,
                body,
                received_at,
                retry_after_ms,
            } => {
                if body.as_bytes().len() > 64 * 1024
                    || received_at < c.dispatch.at
                    || received_at > at
                {
                    return Err(Error::Invalid);
                }
                let outcome = if status == 200 {
                    acknowledged(body.as_bytes(), &c.plan)
                } else {
                    Outcome::Unknown
                };
                let until = if status == 429 {
                    Some(
                        received_at
                            .saturating_add(retry_after_ms.unwrap_or(WINDOW).clamp(1, 3_600_000)),
                    )
                } else {
                    None
                };
                (outcome, body, received_at, Some(status), until)
            }
        };
        let mut hash = Sha256::new();
        hash.update(b"CINDER-BULK-REPLY-1\0");
        hash.update(c.dispatch.commit.bytes());
        hash.update([outcome as u8]);
        let id = CommitId::new(hash.finalize().into()).map_err(|_| Error::Invalid)?;
        let mut tx = self.transaction(
            j,
            id,
            at,
            Record::Response {
                exposure: c.dispatch.commit.bytes(),
                code,
            },
        )?;
        if let Some(until) = cooldown {
            tx.evidence.push(self.archive(Record::Cooldown { until })?);
        }
        let status = match outcome {
            Outcome::Acknowledged => Status::Acknowledged,
            Outcome::CancelAcknowledged => Status::CancelAcknowledged,
            Outcome::Unknown => Status::Unknown,
        };
        let mut h = Sha256::new();
        h.update(b"CINDER-BULK-ACK-1\0");
        h.update(c.plan.attempt.clone());
        h.update([outcome as u8]);
        tx.order_observations.push(orders::Observation {
            key: EventKey {
                scope: self.profile.scope.source,
                event: EconomicEventId::new(&h.finalize()).map_err(|_| Error::Invalid)?,
                leg: 0,
            },
            attempt: AttemptKey::decode(&c.plan.original).map_err(|_| Error::Invalid)?,
            status,
            authority_epoch: self.policy.revision,
            observed_at: received,
            raw,
        });
        j.commit(tx).map_err(|_| Error::Unavailable)?;
        Ok(outcome)
    }
}
fn format_grid(value: u64, step: u64) -> Result<String, Error> {
    let n = u128::from(value)
        .checked_mul(u128::from(step))
        .ok_or(Error::Limit)?;
    Ok(format!("{}.{:08}", n / 100_000_000, n % 100_000_000))
}
fn acknowledged(bytes: &[u8], plan: &Plan) -> Outcome {
    let Ok(value) = crate::raw::decode::<serde_json::Value>(bytes) else {
        return Outcome::Unknown;
    };
    if value.get("status").and_then(|v| v.as_str()) != Some("ok")
        || value.pointer("/response/type").and_then(|v| v.as_str()) != Some("order")
    {
        return Outcome::Unknown;
    }
    let Some(rows) = value
        .pointer("/response/data/statuses")
        .and_then(|v| v.as_array())
    else {
        return Outcome::Unknown;
    };
    if rows.is_empty() || rows.len() > 128 {
        return Outcome::Unknown;
    }
    for row in rows {
        let Some(object) = row.as_object() else {
            return Outcome::Unknown;
        };
        if object.len() != 1 {
            return Outcome::Unknown;
        }
        let Some((kind, data)) = object.iter().next() else {
            return Outcome::Unknown;
        };
        if (plan.cancel && kind != "cancelled")
            || (!plan.cancel
                && !["resting", "working", "filled", "partiallyFilled"].contains(&kind.as_str()))
            || data
                .get("oid")
                .and_then(|v| v.as_str())
                .and_then(|s| crate::address(s).ok())
                != Some(plan.order)
        {
            return Outcome::Unknown;
        }
    }
    if plan.cancel {
        Outcome::CancelAcknowledged
    } else {
        Outcome::Acknowledged
    }
}
