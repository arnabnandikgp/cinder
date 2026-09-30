//! Native signing after durable exposure. No HTTP client or key-loading I/O.
use crate::{Error, client_id, observation, profile::*};
use cinder_journal::{
    Backend, Journal, Protection,
    model::*,
    orders::{Observation, Status, TimeInForce},
};
use cinder_kernel::{codec::Canonical, identity::*};
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

const MAGIC: &[u8] = b"CINDER-PACIFICA-EXECUTION-1\0";
const WINDOW: u64 = 60_000;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// Exact egress destination. Redirects and origin substitution are forbidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origin {
    /// Bounded native testnet, only with separate live-test authority.
    Testnet,
    /// Mainnet; construction never grants deployment/customer-fund authority.
    Mainnet,
}
impl Origin {
    /// Pinned origin; the path is independently selected by the action encoder.
    pub fn url(self) -> &'static str {
        match self {
            Self::Testnet => "https://test-api.pacifica.fi",
            Self::Mainnet => "https://api.pacifica.fi",
        }
    }
}
/// Immutable local execution/credit qualification. No production default exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    /// Explicit evidence revision.
    pub revision: u64,
    /// Named test/qualification artifact. Offline evidence is not native permission.
    pub evidence: String,
    /// Exact native signing/method semantics qualification.
    pub execution: Level,
    /// Allowed origin, also bound in the key slot and durable plan.
    pub origin: Origin,
    /// Maximum signed lifetime, no more than 30 seconds in this narrow profile.
    pub expiry_ms: u64,
    /// Shared 60-second allowance in tenths of one API credit.
    pub credits: u32,
    /// Capacity protected from new orders and ordinary reads.
    pub cleanup_reserve: u32,
    /// Qualified worst-case read cost in tenths, including heavy history reads.
    pub read_cost: u32,
}
/// Injected one-shot dispatch identity and time. Not a new order intent.
#[derive(Debug, Clone, Copy)]
pub struct Dispatch {
    /// Already prepared, reserved exact attempt.
    pub attempt: AttemptKey,
    /// New transaction identity; retries do not acquire another signing capability.
    pub commit: CommitId,
    /// Trusted monotone millisecond clock; P20 must qualify the clock boundary.
    pub at: u64,
}
/// Narrow observed response. Never a complete execution history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Create acknowledged; no position or cash changed.
    Acknowledged,
    /// Scoped cancellation acknowledged; holds remain until complete history.
    CancelAcknowledged,
    /// Includes timeouts, 429, malformed bodies and ambiguous errors.
    Unknown,
}
/// Secret-safe trusted transport result. Response bodies exclude credentials.
pub enum Reply {
    /// No known response; transport may already have delivered the action.
    Unknown,
    /// HTTP result from the exact authenticated destination, without redirects.
    Response {
        /// Native HTTP status.
        status: u16,
        /// Exact bounded response body.
        body: PrivateBytes,
        /// Trusted local receive time, not a native response timestamp.
        received_at: u64,
        /// Parsed Retry-After duration, if supplied (bounded before arithmetic).
        retry_after_ms: Option<u64>,
    },
}
impl std::fmt::Debug for Reply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Reply([PRIVATE])")
    }
}
/// Non-clone, one-shot signed request, constructed only after a durable exposure.
/// It must remain inside the confidential runtime and its TLS egress boundary.
pub struct Outbound {
    origin: Origin,
    path: &'static str,
    body: PrivateBytes,
}
impl std::fmt::Debug for Outbound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Outbound([PRIVATE])")
    }
}
impl Outbound {
    /// Only the prequalified origin; never follow a server-supplied redirect.
    pub fn origin(&self) -> Origin {
        self.origin
    }
    /// Exact allowlisted order or scoped-cancel path.
    pub fn path(&self) -> &'static str {
        self.path
    }
    /// Private-runtime-only request bytes; never log or send through host plaintext.
    pub fn body(&self) -> &[u8] {
        self.body.as_bytes()
    }
}
/// Trusted one-shot egress seam. P19/P20 own actual in-enclave TLS/host isolation.
/// Implementations must not retry, redirect, log, or mutate the signed payload.
pub trait Transport {
    /// Send once to the exact bound origin/path. Any uncertainty returns Unknown.
    fn post(&mut self, request: Outbound) -> Reply;
}
/// Non-clone read budget token. It is not authentication or a network capability.
pub struct ReadPermit {
    at: u64,
    cost: u32,
    until: u64,
}
impl ReadPermit {
    /// Consume once at an injected current time, then send immediately. Expired
    /// permits cannot be stockpiled past the accounting window. No refund on loss.
    /// Retain the reservation's CommitId and report HTTP 429 through
    /// `Gateway::record_read_limit` before scheduling more traffic.
    pub fn consume(self, now: u64) -> Result<(u64, u32), Error> {
        if now < self.at || now >= self.until {
            return Err(Error::Qualification);
        }
        Ok((self.at, self.cost))
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Plan {
    attempt: Vec<u8>,
    original: Vec<u8>,
    abstract_action: Vec<u8>,
    authority_epoch: u64,
    key_epoch: u64,
    agent: String,
    account: String,
    origin: Origin,
    path: String,
    preimage: String,
}
#[derive(Serialize, Deserialize)]
enum Record {
    Epoch {
        epoch: u64,
        agent: Option<String>,
    },
    Spend {
        cost: u32,
        cleanup: bool,
        plan: Option<Plan>,
    },
    Cooldown {
        until: u64,
    },
    Response {
        attempt: Vec<u8>,
        status: Option<u16>,
    },
    ReadLimit {
        reservation: [u8; 32],
        received_at: u64,
        retry_after_ms: Option<u64>,
    },
}
#[derive(Serialize, Deserialize)]
struct Archive {
    contract: [u8; 32],
    record: Record,
}
#[derive(Default)]
struct History {
    epoch: u64,
    agent: Option<String>,
    used: u32,
    blocked_until: u64,
}

/// Scoped private signer and native encoder. There is deliberately no public
/// arbitrary-message signing method or fund-moving operation enum.
pub struct Gateway {
    profile: Profile,
    policy: Policy,
    key: SigningKey,
    epoch: u64,
    contract: [u8; 32],
}
impl std::fmt::Debug for Gateway {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Gateway([PRIVATE])")
    }
}
impl Gateway {
    /// Load caller-supplied enclave key material, not a wallet/file/environment.
    /// The seed must be independently generated and never registered for another
    /// native account or network; the native signature cannot enforce that rule.
    pub fn new(
        profile: Profile,
        policy: Policy,
        seed: Zeroizing<[u8; 32]>,
        epoch: u64,
    ) -> Result<Self, Error> {
        let fingerprint = profile.commitment()?;
        if epoch == 0
            || policy.revision == 0
            || policy.evidence.is_empty()
            || policy.evidence.len() > 256
            || !policy.evidence.is_ascii()
            || policy.execution != Level::Qualified
            || profile.fills != Level::Qualified
            || profile.precision != Level::Qualified
            || policy.expiry_ms == 0
            || policy.expiry_ms > 30_000
            || policy.credits == 0
            || policy.cleanup_reserve == 0
            || policy.cleanup_reserve >= policy.credits
            || policy.read_cost == 0
            || policy.read_cost > policy.credits
            || profile.environment != policy.origin.url()
        {
            return Err(Error::Qualification);
        }
        let account = bs58::decode(&profile.account)
            .into_vec()
            .map_err(|_| Error::Qualification)?;
        if account.len() != 32 || bs58::encode(account).into_string() != profile.account {
            return Err(Error::Qualification);
        }
        let mut hash = Sha256::new();
        hash.update(MAGIC);
        hash.update(fingerprint);
        hash.update(serde_json::to_vec(&policy).map_err(|_| Error::Codec)?);
        Ok(Self {
            profile,
            policy,
            key: SigningKey::from_bytes(&seed),
            epoch,
            contract: hash.finalize().into(),
        })
    }
    /// Public agent identity only; no export of seed or arbitrary signing service.
    pub fn agent(&self) -> String {
        bs58::encode(self.key.verifying_key().to_bytes()).into_string()
    }
    fn evidence(&self, record: Record) -> Result<PrivateBytes, Error> {
        let mut bytes = MAGIC.to_vec();
        bytes.extend(
            serde_json::to_vec(&Archive {
                contract: self.contract,
                record,
            })
            .map_err(|_| Error::Codec)?,
        );
        Ok(PrivateBytes::new(bytes)?)
    }
    fn history<B: Backend, P: Protection>(
        &self,
        journal: &mut Journal<B, P>,
        at: u64,
    ) -> Result<History, Error> {
        journal.verified_state()?;
        if journal.configuration() != &self.profile.config {
            return Err(Error::Qualification);
        }
        let mut result = History::default();
        for tx in journal.transactions() {
            if tx.at > at {
                return Err(Error::Qualification);
            }
            for body in &tx.evidence {
                if let Some(bytes) = body.as_bytes().strip_prefix(MAGIC) {
                    let a: Archive = serde_json::from_slice(bytes).map_err(|_| Error::Codec)?;
                    if a.contract != self.contract {
                        return Err(Error::Qualification);
                    }
                    match a.record {
                        Record::Epoch { epoch, agent } => {
                            if epoch <= result.epoch {
                                return Err(Error::Qualification);
                            }
                            result.epoch = epoch;
                            result.agent = agent;
                        }
                        Record::Spend { cost, .. } => {
                            // Hold through the last permitted dispatch instant
                            // plus one venue window; delayed delivery cannot spend
                            // an already expired local credit reservation.
                            if at - tx.at < WINDOW + self.policy.expiry_ms {
                                result.used = result.used.checked_add(cost).ok_or(Error::Limit)?;
                            }
                        }
                        Record::Cooldown { until } => {
                            result.blocked_until = result.blocked_until.max(until)
                        }
                        Record::Response { .. } | Record::ReadLimit { .. } => {}
                    }
                }
            }
        }
        Ok(result)
    }
    fn active(&self, history: &History) -> Result<(), Error> {
        if history.epoch != self.epoch || history.agent.as_ref() != Some(&self.agent()) {
            Err(Error::Qualification)
        } else {
            Ok(())
        }
    }
    fn credit(&self, history: &History, at: u64, cost: u32, cleanup: bool) -> Result<(), Error> {
        self.active(history)?;
        let limit = if cleanup {
            self.policy.credits
        } else {
            self.policy.credits - self.policy.cleanup_reserve
        };
        if at < history.blocked_until || history.used.checked_add(cost).is_none_or(|n| n > limit) {
            Err(Error::Limit)
        } else {
            Ok(())
        }
    }
    fn transaction<B: Backend, P: Protection>(
        &self,
        journal: &Journal<B, P>,
        id: CommitId,
        at: u64,
        record: Record,
    ) -> Result<Transaction, Error> {
        Ok(Transaction {
            id,
            expected: journal.head(),
            at,
            evidence: vec![self.evidence(record)?],
            inputs: vec![],
            order_observations: vec![],
            funds_observations: vec![],
            controls: vec![],
        })
    }
    /// Trusted administrator/key-release port, not a customer/API method. Record
    /// a strictly newer key epoch; rotations do not reset spent API credits.
    pub fn activate<B: Backend, P: Protection>(
        &self,
        journal: &mut Journal<B, P>,
        id: CommitId,
        at: u64,
    ) -> Result<(), Error> {
        let history = self.history(journal, at)?;
        if self.epoch <= history.epoch {
            return Err(Error::Qualification);
        }
        let tx = self.transaction(
            journal,
            id,
            at,
            Record::Epoch {
                epoch: self.epoch,
                agent: Some(self.agent()),
            },
        )?;
        journal.commit(tx)?;
        Ok(())
    }
    /// Trusted administrator port: durably fence this key even across restarts.
    /// Revoking an escaped native credential at the venue is a separate operation.
    pub fn deactivate<B: Backend, P: Protection>(
        &self,
        journal: &mut Journal<B, P>,
        id: CommitId,
        at: u64,
    ) -> Result<(), Error> {
        let history = self.history(journal, at)?;
        self.active(&history)?;
        let tx = self.transaction(
            journal,
            id,
            at,
            Record::Epoch {
                epoch: self.epoch.checked_add(1).ok_or(Error::Limit)?,
                agent: None,
            },
        )?;
        journal.commit(tx)?;
        Ok(())
    }
    /// Reserve worst-case read credits in the same pool/IP budget before I/O.
    /// Only the trusted reconciliation scheduler may set cleanup=true; ordinary
    /// customer reads cannot consume the protected cleanup budget.
    pub fn reserve_read<B: Backend, P: Protection>(
        &self,
        journal: &mut Journal<B, P>,
        id: CommitId,
        at: u64,
        cleanup: bool,
    ) -> Result<ReadPermit, Error> {
        if journal.transaction(id).is_some() {
            return Err(Error::Qualification);
        }
        let history = self.history(journal, at)?;
        self.credit(&history, at, self.policy.read_cost, cleanup)?;
        let tx = self.transaction(
            journal,
            id,
            at,
            Record::Spend {
                cost: self.policy.read_cost,
                cleanup,
                plan: None,
            },
        )?;
        let c = journal.commit(tx)?;
        if c.duplicate {
            return Err(Error::Qualification);
        }
        Ok(ReadPermit {
            at,
            cost: self.policy.read_cost,
            until: at.checked_add(self.policy.expiry_ms).ok_or(Error::Limit)?,
        })
    }
    /// Trusted egress response port for HTTP 429 on an already reserved read.
    /// Call after consuming its permit, before further dispatch. The reservation
    /// binds pool/policy provenance; no active signer is required for a late reply
    /// after rotation. Exact retries are idempotent; changed replies conflict.
    pub fn record_read_limit<B: Backend, P: Protection>(
        &self,
        journal: &mut Journal<B, P>,
        reservation: CommitId,
        received_at: u64,
        retry_after_ms: Option<u64>,
    ) -> Result<(), Error> {
        let at = received_at.max(journal.transactions().map(|t| t.at).max().unwrap_or(0));
        self.history(journal, at)?;
        let source = journal
            .transaction(reservation)
            .ok_or(Error::Qualification)?;
        let [body] = source.evidence.as_slice() else {
            return Err(Error::Qualification);
        };
        let archive: Archive = serde_json::from_slice(
            body.as_bytes()
                .strip_prefix(MAGIC)
                .ok_or(Error::Qualification)?,
        )
        .map_err(|_| Error::Codec)?;
        if archive.contract != self.contract
            || !matches!(archive.record,
                Record::Spend { cost, plan: None, .. } if cost == self.policy.read_cost
            )
        {
            return Err(Error::Qualification);
        }
        let evidence = self.evidence(Record::ReadLimit {
            reservation: reservation.bytes(),
            received_at,
            retry_after_ms,
        })?;
        let mut hash = Sha256::new();
        hash.update(b"CINDER-PACIFICA-READ-LIMIT-1\0");
        hash.update(reservation.bytes());
        let id = CommitId::new(hash.finalize().into())?;
        if let Some(old) = journal.transaction(id) {
            return if old.evidence.first() == Some(&evidence) {
                Ok(())
            } else {
                Err(Error::Journal(cinder_journal::Error::Conflict))
            };
        }
        let mut tx = self.transaction(
            journal,
            id,
            at,
            Record::Cooldown {
                until: cooldown_until(at, retry_after_ms),
            },
        )?;
        tx.evidence.insert(0, evidence);
        journal.commit(tx)?;
        Ok(())
    }
    fn plan(&self, state: &State, dispatch: Dispatch) -> Result<(Plan, u32, bool), Error> {
        let a = state
            .attempts()
            .iter()
            .find(|a| a.key == dispatch.attempt)
            .ok_or(Error::Qualification)?;
        if a.possibly_exposed || a.expires_at <= dispatch.at || dispatch.at > MAX_SAFE_INTEGER {
            return Err(Error::Qualification);
        }
        let order = state
            .orders()
            .iter()
            .find(|o| o.intent.request == a.key.request)
            .ok_or(Error::Qualification)?;
        let original = order.attempt.ok_or(Error::Qualification)?;
        let m = self
            .profile
            .markets
            .iter()
            .find(|m| self.profile.config.markets[m.market].unit() == order.intent.quantity.unit())
            .ok_or(Error::Qualification)?;
        let cleanup = a.kind == AttemptKind::Cancel;
        let (path, kind, data, cost) = if cleanup {
            (
                "/api/v1/orders/cancel",
                "cancel_order",
                json!({"symbol":m.symbol,"client_order_id":client_id(original)}),
                5,
            )
        } else {
            // Emergency and restoration native qualification stays disabled: P13
            // does not yet provide their required native source-time/cut evidence.
            if a.kind != AttemptKind::Order || original != a.key {
                return Err(Error::Qualification);
            }
            let i = &order.intent;
            let (side, limit) = if i.quantity.lots() > 0 {
                ("bid", i.maximum.ticks())
            } else {
                ("ask", i.minimum.ticks())
            };
            let tif = match i.time_in_force {
                TimeInForce::GoodTilCancelled => "GTC",
                TimeInForce::AddLiquidityOnly => "ALO",
                TimeInForce::ImmediateOrCancel => "IOC",
            };
            // Native reduce_only refers to the pooled position, not this customer.
            (
                "/api/v1/orders/create",
                "create_order",
                json!({"symbol":m.symbol,"price":m.price.format(limit)?,"amount":m.size.format(i.quantity.magnitude())?,"side":side,"tif":tif,"reduce_only":false,"client_order_id":client_id(a.key)}),
                10,
            )
        };
        let expiry = self.policy.expiry_ms.min(a.expires_at - dispatch.at);
        if dispatch
            .at
            .checked_add(expiry)
            .is_none_or(|n| n > MAX_SAFE_INTEGER)
        {
            return Err(Error::Qualification);
        }
        let preimage = canonical(data, dispatch.at, expiry, kind)?;
        Ok((
            Plan {
                attempt: a.key.encode(),
                original: original.encode(),
                abstract_action: a.message.as_bytes().to_vec(),
                authority_epoch: a.authority_epoch,
                key_epoch: self.epoch,
                agent: self.agent(),
                account: self.profile.account.clone(),
                origin: self.policy.origin,
                path: path.into(),
                preimage,
            },
            cost,
            cleanup,
        ))
    }
    /// Validate, charge, durably expose, sign and send once. There is deliberately
    /// no automatic retry after a timeout, 429, lost commit or process restart.
    pub fn dispatch<B: Backend, P: Protection, T: Transport>(
        &self,
        journal: &mut Journal<B, P>,
        dispatch: Dispatch,
        transport: &mut T,
    ) -> Result<Outcome, Error> {
        if journal.transaction(dispatch.commit).is_some() {
            return Err(Error::Qualification);
        }
        let history = self.history(journal, dispatch.at)?;
        let (plan, cost, cleanup) = self.plan(journal.state()?, dispatch)?;
        if !cleanup && !observation::replay(journal, &self.profile)?.gaps.is_empty() {
            return Err(Error::Qualification);
        }
        self.credit(&history, dispatch.at, cost, cleanup)?;
        let mut tx = self.transaction(
            journal,
            dispatch.commit,
            dispatch.at,
            Record::Spend {
                cost,
                cleanup,
                plan: Some(plan.clone()),
            },
        )?;
        tx.controls.push(Control::Expose(dispatch.attempt));
        let c = journal.commit(tx)?;
        if c.duplicate || c.receipt.controls.is_some() || c.exposures.len() != 1 {
            return Err(Error::Qualification);
        }
        let (key, abstract_action) = c
            .exposures
            .into_iter()
            .next()
            .ok_or(Error::Qualification)?
            .into_message();
        if key != dispatch.attempt || abstract_action.as_bytes() != plan.abstract_action {
            return Err(Error::Qualification);
        }
        // A competing writer/revocation after preparation must not escape signing.
        self.active(&self.history(journal, dispatch.at)?)?;
        let signature = self.key.sign(plan.preimage.as_bytes());
        let mut signed: Value = serde_json::from_str(&plan.preimage).map_err(|_| Error::Codec)?;
        let mut body = signed["data"]
            .take()
            .as_object()
            .cloned()
            .ok_or(Error::Codec)?;
        body.insert("account".into(), json!(plan.account));
        body.insert("agent_wallet".into(), json!(plan.agent));
        body.insert(
            "signature".into(),
            json!(bs58::encode(signature.to_bytes()).into_string()),
        );
        body.insert("timestamp".into(), signed["timestamp"].clone());
        body.insert("expiry_window".into(), signed["expiry_window"].clone());
        let request = Outbound {
            origin: plan.origin,
            path: if cleanup {
                "/api/v1/orders/cancel"
            } else {
                "/api/v1/orders/create"
            },
            body: PrivateBytes::new(serde_json::to_vec(&body).map_err(|_| Error::Codec)?)?,
        };
        let reply = transport.post(request);
        self.record_reply(journal, dispatch, &plan, reply)
    }
    fn record_reply<B: Backend, P: Protection>(
        &self,
        journal: &mut Journal<B, P>,
        dispatch: Dispatch,
        plan: &Plan,
        reply: Reply,
    ) -> Result<Outcome, Error> {
        let (outcome, raw, at, cooldown, http_status) = match reply {
            Reply::Unknown => (
                Outcome::Unknown,
                PrivateBytes::new(b"transport outcome unknown".to_vec())?,
                dispatch.at,
                None,
                None,
            ),
            Reply::Response {
                status,
                body,
                received_at,
                retry_after_ms,
            } => {
                let malformed =
                    body.as_bytes().len() > observation::MAX_BODY || received_at < dispatch.at;
                let received_at = received_at.max(dispatch.at);
                let body = if malformed {
                    PrivateBytes::new(b"response rejected: bound or clock".to_vec())?
                } else {
                    body
                };
                let outcome = if status == 200 && !malformed {
                    acknowledged(body.as_bytes(), plan.path.ends_with("/cancel"))
                } else {
                    Outcome::Unknown
                };
                let cooldown = if status == 429 {
                    Some(cooldown_until(received_at, retry_after_ms))
                } else {
                    None
                };
                (outcome, body, received_at, cooldown, Some(status))
            }
        };
        let id = reply_id(dispatch.commit)?;
        let mut tx = Transaction {
            id,
            expected: journal.head(),
            at,
            evidence: vec![],
            inputs: vec![],
            order_observations: vec![],
            funds_observations: vec![],
            controls: vec![],
        };
        tx.evidence.push(self.evidence(Record::Response {
            attempt: dispatch.attempt.encode(),
            status: http_status,
        })?);
        if let Some(until) = cooldown {
            tx.evidence.push(self.evidence(Record::Cooldown { until })?);
        }
        tx.order_observations.push(Observation {
            key: EventKey {
                scope: self.profile.source,
                event: EconomicEventId::new(&[b"dispatch:".as_slice(), &id.bytes()].concat())?,
                leg: 0,
            },
            attempt: AttemptKey::decode(&plan.original)?,
            status: match outcome {
                Outcome::Acknowledged => Status::Acknowledged,
                Outcome::CancelAcknowledged => Status::CancelAcknowledged,
                Outcome::Unknown => Status::Unknown,
            },
            authority_epoch: self.profile.revision,
            observed_at: at,
            raw,
        });
        let committed = journal.commit(tx)?;
        if committed.receipt.order_observations != [true] {
            return Err(Error::Qualification);
        }
        Ok(outcome)
    }
}
fn cooldown_until(at: u64, retry_after_ms: Option<u64>) -> u64 {
    // A broken extreme clock must not erase a post-send observation via overflow.
    // Saturation contains future traffic rather than wrapping into an expired limit.
    at.saturating_add(retry_after_ms.unwrap_or(WINDOW).clamp(WINDOW, 3_600_000))
}
fn canonical(data: Value, timestamp: u64, expiry_window: u64, kind: &str) -> Result<String, Error> {
    serde_json::to_string(
        &json!({"data":data,"expiry_window":expiry_window,"timestamp":timestamp,"type":kind}),
    )
    .map_err(|_| Error::Codec)
}
fn reply_id(id: CommitId) -> Result<CommitId, Error> {
    let mut h = Sha256::new();
    h.update(b"CINDER-PACIFICA-REPLY-1\0");
    h.update(id.bytes());
    Ok(CommitId::new(h.finalize().into())?)
}
#[derive(Deserialize)]
struct Ack {
    error: Option<Value>,
    success: Option<bool>,
    order_id: Option<u64>,
    data: Option<AckData>,
}
#[derive(Deserialize)]
struct AckData {
    order_id: u64,
}
fn acknowledged(body: &[u8], cancel: bool) -> Outcome {
    let Ok(a) = serde_json::from_slice::<Ack>(body) else {
        return Outcome::Unknown;
    };
    if a.error.is_some() {
        return Outcome::Unknown;
    }
    if cancel {
        return if a.success == Some(true) {
            Outcome::CancelAcknowledged
        } else {
            Outcome::Unknown
        };
    }
    if a.success == Some(false) {
        return Outcome::Unknown;
    }
    match (a.order_id, a.data.map(|d| d.order_id)) {
        (Some(n), None) | (None, Some(n)) if n > 0 => Outcome::Acknowledged,
        _ => Outcome::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cooldown_clock_overflow_saturates_instead_of_dropping_response() {
        assert_eq!(cooldown_until(u64::MAX - 1, None), u64::MAX);
        assert_eq!(cooldown_until(10, Some(0)), 60_010);
        assert_eq!(cooldown_until(10, Some(u64::MAX)), 3_600_010);
    }
    fn hex<const N: usize>(s: &str) -> [u8; N] {
        (0..N)
            .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
    #[test]
    fn official_canonical_bytes_and_independent_openssl_signature_vector() {
        let data = json!({"symbol":"BTC","price":"100000","amount":"0.1","side":"bid","tif":"GTC","reduce_only":false,"client_order_id":"12345678-1234-1234-1234-123456789abc"});
        let text = canonical(data, 1748970123456, 5000, "create_order").unwrap();
        let expected = r#"{"data":{"amount":"0.1","client_order_id":"12345678-1234-1234-1234-123456789abc","price":"100000","reduce_only":false,"side":"bid","symbol":"BTC","tif":"GTC"},"expiry_window":5000,"timestamp":1748970123456,"type":"create_order"}"#;
        assert_eq!(text, expected);
        // Public RFC8032 vector; never a venue-funded key. Expected request
        // signature independently generated with Node/OpenSSL, not this library.
        let key = SigningKey::from_bytes(&hex::<32>(
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
        ));
        assert_eq!(
            key.verifying_key().to_bytes(),
            hex::<32>("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
        );
        assert_eq!(
            key.sign(b"").to_bytes(),
            hex::<64>(
                "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
            )
        );
        let signature = key.sign(text.as_bytes());
        assert_eq!(
            signature.to_bytes(),
            hex::<64>(
                "140a59bf58220039a9f2d6d65aa16b59b6ab673215e5a4c9b6b84ea074f44e40f4b758048b7b47302401ca01df4b4618dd3ac30b5cbdf0a5e6228c360d919905"
            )
        );
        assert!(
            key.verifying_key()
                .verify_strict(text.replace("GTC", "IOC").as_bytes(), &signature)
                .is_err()
        );
    }
}
