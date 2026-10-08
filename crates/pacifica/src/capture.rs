//! Bounded native account-transfer capture. The trusted transport authenticates
//! the server; this module authenticates neither a socket nor causal coverage.
//! All records stay raw in the one journal: no cash, readiness or payout updates.
use crate::{
    Error,
    execution::{Gateway, Origin, ReadPermit},
    observation::MAX_BODY,
    reads::MIN_READ_COST,
};
use cinder_journal::{
    Backend, Committed, Journal, Protection,
    model::{CommitId, Input, PrivateBytes, Transaction},
};
use cinder_kernel::identity::EventScope;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Finite qualification capture, not an automatically reconnecting subscriber.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    /// Absolute duration, including handshake, after the permit is consumed.
    pub maximum_ms: u64,
    /// All peer messages, including controls and unrecognized bodies.
    pub maximum_messages: u32,
    /// Total decoded payload bytes, including control messages.
    pub maximum_bytes: usize,
    /// One decoded message; fragments share this bound.
    pub maximum_message: usize,
}
impl Limits {
    /// Keep the first capture below Pacifica's documented 60-second send-idle
    /// cutoff. Continuous subscription/heartbeat policy is a separate gate.
    pub fn validate(self) -> Result<(), Error> {
        if self.maximum_ms == 0
            || self.maximum_ms > 30_000
            || self.maximum_messages == 0
            || self.maximum_messages > 256
            || self.maximum_message == 0
            || self.maximum_message > MAX_BODY
            || self.maximum_bytes < self.maximum_message
            || self.maximum_bytes > 1_048_576
        {
            return Err(Error::Limit);
        }
        Ok(())
    }
}
/// Trusted scheduler identities, distinct from external event identities.
pub struct Plan {
    /// Shared pool/IP read-credit reservation, durably spent before I/O.
    pub reservation: CommitId,
    /// Durable capture start; never reuse for another connection.
    pub capture: CommitId,
    /// Qualified clock before preparing the request.
    pub at: u64,
    /// Explicit finite resource policy.
    pub limits: Limits,
}
/// Non-clone capability. No signatures, bearer token or customer identifier.
pub struct Request {
    permit: ReadPermit,
    query: Query,
}
/// Bound request consumed only by an enclave-owned trusted transport. Possession
/// is not evidence of TLS authentication: fixture ports can consume it too.
pub struct Query {
    origin: Origin,
    account: String,
    binding: [u8; 32],
    limits: Limits,
    at: u64,
    sequence: u32,
    messages: u32,
    bytes: usize,
    ended: bool,
}
impl Request {
    /// Consume the original reservation immediately before one connection. No
    /// refund, clone or second delivery after handshake failure or disconnect.
    pub fn consume(mut self, now: u64) -> Result<Query, Error> {
        let (_, cost) = self.permit.consume(now)?;
        if cost < MIN_READ_COST {
            return Err(Error::Qualification);
        }
        self.query.at = now;
        Ok(self.query)
    }
}
/// Journal record kind, not an economic event or stream-completeness assertion.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Kind {
    /// TLS/upgrade verified and exact subscribe written once.
    Opened,
    /// Full UTF-8 text; even unknown or malformed JSON is retained.
    Text,
    /// Unexpected binary payload retained without normalizing it.
    Binary,
    /// RFC control bytes, included in resource accounting.
    Ping,
    /// RFC control bytes; not native subscription acknowledgement.
    Pong,
    /// Peer closed; does not cover events omitted before/after the session.
    Closed,
    /// Redacted transport/protocol/clock failure; outcome remains uncertain.
    Interrupted,
    /// Absolute duration or resource budget reached; not terminal coverage.
    Limited,
}
impl Kind {
    fn terminal(self) -> bool {
        matches!(self, Self::Closed | Self::Interrupted | Self::Limited)
    }
    fn message(self) -> bool {
        matches!(
            self,
            Self::Text | Self::Binary | Self::Ping | Self::Pong | Self::Closed
        )
    }
}
/// Private capture fact, created by the trusted transport's consumed query.
/// There is deliberately no conversion into Credit/Setup/Withdrawal certificates.
pub struct Record {
    binding: [u8; 32],
    sequence: u32,
    kind: Kind,
    received_at: u64,
    body: PrivateBytes,
}
impl std::fmt::Debug for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeCaptureRecord([PRIVATE])")
    }
}
impl Query {
    /// Fixed REST environment determines the distinct native WSS origin.
    pub fn origin(&self) -> Origin {
        self.origin
    }
    /// Validated pooled wallet only; no caller-selected subscription scope.
    pub fn account(&self) -> &str {
        &self.account
    }
    /// Exact checked resource bounds.
    pub fn limits(&self) -> Limits {
        self.limits
    }
    /// Last trusted observation, usable only to timestamp an uncertain stop if
    /// the clock itself becomes unavailable. Never timestamp a new body with it.
    pub fn last_at(&self) -> u64 {
        self.at
    }
    /// Trusted transport input. It must authenticate TLS, enforce these limits
    /// before allocation and retain every accepted body. This is not a public API.
    pub fn record(&mut self, kind: Kind, body: Vec<u8>, at: u64) -> Result<Record, Error> {
        let body = PrivateBytes::new(body)?;
        if self.ended
            || at < self.at
            || (self.sequence == 0 && kind != Kind::Opened && !kind.terminal())
            || (self.sequence != 0 && kind == Kind::Opened)
            || body.as_bytes().len() > self.limits.maximum_message
            || (matches!(kind, Kind::Opened | Kind::Interrupted | Kind::Limited)
                && !body.as_bytes().is_empty())
        {
            return Err(Error::Qualification);
        }
        if kind.message() {
            let messages = self.messages.checked_add(1).ok_or(Error::Limit)?;
            let bytes = self
                .bytes
                .checked_add(body.as_bytes().len())
                .ok_or(Error::Limit)?;
            if messages > self.limits.maximum_messages || bytes > self.limits.maximum_bytes {
                return Err(Error::Limit);
            }
            self.messages = messages;
            self.bytes = bytes;
        }
        self.sequence = self.sequence.checked_add(1).ok_or(Error::Limit)?;
        self.ended = kind.terminal();
        self.at = at;
        Ok(Record {
            binding: self.binding,
            sequence: self.sequence,
            kind,
            received_at: at,
            body,
        })
    }
}
/// Writer-owned completion, separate from transport and all signing authority.
pub struct Archive {
    contract: [u8; 32],
    reserved: Transaction,
    prepared: Transaction,
    source: EventScope,
    binding: [u8; 32],
    next: u32,
    at: u64,
    ended: bool,
}
/// Reserve and retain intent under the journal guard; release it before I/O.
/// An uncertain preparation commit must be reopened, not retried with fresh IDs.
pub fn prepare<B: Backend, P: Protection>(
    journal: &mut Journal<B, P>,
    gateway: &Gateway,
    plan: Plan,
) -> Result<(Request, Archive), Error> {
    plan.limits.validate()?;
    let (profile, policy) = gateway.read_binding();
    let account = bs58::decode(&profile.account)
        .into_vec()
        .map_err(|_| Error::Qualification)?;
    if account.len() != 32 || bs58::encode(account).into_string() != profile.account {
        return Err(Error::Qualification);
    }
    if plan.at == 0
        || plan.reservation == plan.capture
        || journal.transaction(plan.capture).is_some()
        || policy.read_cost < MIN_READ_COST
    {
        return Err(Error::Qualification);
    }
    let contract = gateway.release_commitment(journal.configuration())?;
    let binding = Sha256::digest(
        [
            b"CINDER-NATIVE-CAPTURE-1\0".as_slice(),
            &contract,
            &plan.capture.bytes(),
            &serde_json::to_vec(&plan.limits).map_err(|_| Error::Codec)?,
        ]
        .concat(),
    )
    .into();
    let raw = PrivateBytes::new(
        serde_json::to_vec(&serde_json::json!({
            "schema":"cinder-native-capture-start-v1", "binding":binding,
            "account":profile.account, "origin":policy.origin.url(),
            "limits":plan.limits, "reservation":plan.reservation.bytes()
        }))
        .map_err(|_| Error::Codec)?,
    )?;
    let permit = gateway.reserve_read(journal, plan.reservation, plan.at, false)?;
    let reserved = journal
        .transaction(plan.reservation)
        .ok_or(Error::Qualification)?
        .clone();
    let prepared = transaction(journal, plan.capture, profile.source, raw, plan.at);
    journal.commit(prepared.clone())?;
    Ok((
        Request {
            permit,
            query: Query {
                origin: policy.origin,
                account: profile.account.clone(),
                binding,
                limits: plan.limits,
                at: plan.at,
                sequence: 0,
                messages: 0,
                bytes: 0,
                ended: false,
            },
        },
        Archive {
            contract,
            reserved,
            prepared,
            source: profile.source,
            binding,
            next: 1,
            at: plan.at,
            ended: false,
        },
    ))
}
impl Archive {
    /// Rejoin the current authoritative head for EACH result, never during socket
    /// I/O. Raw adverse/unrecognized facts do not disappear after a command race.
    pub fn append<B: Backend, P: Protection>(
        &mut self,
        journal: &mut Journal<B, P>,
        gateway: &Gateway,
        id: CommitId,
        record: Record,
    ) -> Result<Committed, Error> {
        if self.ended
            || record.binding != self.binding
            || record.sequence != self.next
            || record.received_at < self.at
            || journal.transaction(id).is_some()
            || self.contract != gateway.release_commitment(journal.configuration())?
            || journal.transaction(self.reserved.id) != Some(&self.reserved)
            || journal.transaction(self.prepared.id) != Some(&self.prepared)
        {
            return Err(Error::Qualification);
        }
        let raw = PrivateBytes::new(serde_json::to_vec(&serde_json::json!({
            "schema":"cinder-native-capture-record-v1", "binding":self.binding,
            "sequence":record.sequence, "kind":record.kind,
            "received_at":record.received_at,
            "clock_sample":if matches!(record.kind,Kind::Interrupted|Kind::Limited){"last_known"}else{"fresh"},
            "body":record.body.as_bytes()
        })).map_err(|_| Error::Codec)?)?;
        let result = journal.commit(transaction(
            journal,
            id,
            self.source,
            raw,
            record.received_at,
        ))?;
        self.next = self.next.checked_add(1).ok_or(Error::Limit)?;
        self.at = record.received_at;
        self.ended = record.kind.terminal();
        Ok(result)
    }
}
fn transaction<B: Backend, P: Protection>(
    journal: &Journal<B, P>,
    id: CommitId,
    source: EventScope,
    raw: PrivateBytes,
    observed_at: u64,
) -> Transaction {
    Transaction {
        id,
        expected: journal.head(),
        at: observed_at.max(journal.state().map_or(0, |s| s.logical_time())),
        evidence: vec![],
        inputs: vec![Input {
            source,
            source_cut: None,
            authority_epoch: 0,
            observed_at,
            raw,
            event: None,
        }],
        controls: vec![],
        order_observations: vec![],
        funds_observations: vec![],
    }
}
