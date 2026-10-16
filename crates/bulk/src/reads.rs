//! One unsigned account query with shared durable budget and retained original
//! routing. No automatic polling/retry or claim of complete economic coverage.
use crate::{
    Error, Network,
    execution::{Gateway, Reply},
    fills,
    observation::{self, Kind, Message},
};
use cinder_journal::{
    Committed,
    controller::Controller,
    model::{CommitId, PrivateBytes},
};
use sha2::{Digest, Sha256};

/// Explicit bounded native read. A continuation refers to accepted evidence,
/// never a caller-fabricated cursor, offset or timestamp.
pub struct Poll {
    /// REST family only; TradeStream is not an HTTP query.
    pub kind: Kind,
    /// Distinct durable request/budget identity.
    pub reservation: CommitId,
    /// Distinct durable original-response ingestion identity.
    pub evidence: CommitId,
    /// Explicit first-page inclusive native slots, not a causal certificate.
    pub range: Option<(u64, u64)>,
    /// Previous accepted observation carrying this family's nextCursor.
    pub continuation: Option<CommitId>,
    /// Bounded history limit, 1..128; never native's 5000-row default.
    pub limit: u16,
    /// Trusted request time; service overwrites with its qualified clock.
    pub at: u64,
    /// Internal cleanup scheduler only, never customer-controlled capacity.
    pub cleanup: bool,
}
/// One-use egress request; fields cannot be changed after budget commitment.
pub struct Request {
    query: Query,
    at: u64,
    expires_at: u64,
}
/// Immutable allowlisted account body, not a native signing/transfer capability.
pub struct Query {
    body: PrivateBytes,
    network: Network,
    expires_at: u64,
}
impl Query {
    /// Keep the original dispatch deadline through TLS setup and final write.
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }
    /// Actual native signing environment, not the Solana cluster.
    pub fn network(&self) -> Network {
        self.network
    }
    /// Documented unsigned POST account endpoint.
    pub fn path(&self) -> &'static str {
        "/api/v1/account"
    }
    /// Private immutable query body; exclude from parent logs.
    pub fn body(&self) -> &[u8] {
        self.body.as_bytes()
    }
}
impl Request {
    /// Consume at current trusted time before socket/write. No refund on failure.
    pub fn consume(self, now: u64) -> Result<Query, Error> {
        if now < self.at || now >= self.expires_at {
            return Err(Error::Unqualified);
        }
        Ok(self.query)
    }
}
/// Correlation with the exact persisted original query, not a resend permit.
pub struct Completion {
    poll: Poll,
    marker: PrivateBytes,
}
macro_rules! redacted {($($t:ty),*)=>{$(impl std::fmt::Debug for $t {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {f.write_str("[PRIVATE BULK READ]")}
})*};}
redacted!(Poll, Request, Query, Completion);
/// One-shot authenticated private egress. Account queries have no signatures.
pub trait Transport {
    /// Consume once; no retry, redirect, arbitrary URL or ambient credentials.
    fn query(&mut self, request: Request) -> Reply;
}
/// Persist query and budget before any network request. Selected account/network
/// come exclusively from the gateway; customer input cannot override them.
pub fn prepare(
    gateway: &Gateway,
    j: &mut impl Controller,
    poll: Poll,
) -> Result<(Request, Completion), Error> {
    let (profile, _) = gateway.observation_binding(j)?;
    if poll.reservation == poll.evidence
        || !(1..=fills::MAX_ROWS as u16).contains(&poll.limit)
        || poll.continuation.is_some() && poll.range.is_some()
        || poll.range.is_some_and(|(a, b)| a > b)
    {
        return Err(Error::Invalid);
    }
    let name = match poll.kind {
        Kind::Trades => "fills",
        Kind::Account => "fullAccount",
        Kind::Funding => "fundingHistory",
        Kind::Risk => "riskHistory",
        Kind::TradeStream => return Err(Error::Unsupported),
    };
    if matches!(poll.kind, Kind::Account) && (poll.range.is_some() || poll.continuation.is_some()) {
        return Err(Error::Invalid);
    }
    let mut body =
        serde_json::json!({"type":name,"user":bs58::encode(profile.scope.account()).into_string()});
    if !matches!(poll.kind, Kind::Account) {
        body["limit"] = serde_json::json!(poll.limit);
        if let Some(id) = poll.continuation {
            body["cursor"] =
                serde_json::json!(observation::continuation(gateway, j, poll.kind, id)?);
        }
        if let Some((a, b)) = poll.range {
            body["startSlot"] = serde_json::json!(a);
            body["endSlot"] = serde_json::json!(b);
        }
    }
    let network = profile.scope.network;
    let body = PrivateBytes::new(serde_json::to_vec(&body).map_err(|_| Error::Invalid)?)
        .map_err(|_| Error::Limit)?;
    let (marker, expires_at) = gateway.reserve_read(
        j,
        poll.reservation,
        poll.at,
        poll.kind as u8,
        &body,
        poll.cleanup,
    )?;
    Ok((
        Request {
            query: Query {
                body,
                network,
                expires_at,
            },
            at: poll.at,
            expires_at,
        },
        Completion { poll, marker },
    ))
}
/// Atomically retain original response linkage, actual normalized facts and any
/// HTTP cooldown. Unknown/failed responses never become successful empty pages.
pub fn complete(
    gateway: &Gateway,
    j: &mut impl Controller,
    c: Completion,
    reply: Reply,
    at: u64,
) -> Result<Committed, Error> {
    let (profile, _) = gateway.observation_binding(j)?;
    j.verified_state().map_err(|_| Error::Unavailable)?;
    let original = j
        .transaction(c.poll.reservation)
        .ok_or(Error::Unqualified)?;
    if !original.evidence.iter().any(|e| e == &c.marker)
        || j.transaction_receipt(c.poll.reservation)
            .is_none_or(|r| r.controls.is_some())
    {
        return Err(Error::Unqualified);
    }
    let (body, received_at, code, normalize, cooldown) = match reply {
        Reply::Unknown => (
            PrivateBytes::new(b"transport outcome unknown".to_vec()).map_err(|_| Error::Limit)?,
            at,
            None,
            false,
            None,
        ),
        Reply::Response {
            status,
            body,
            received_at,
            retry_after_ms,
        } => {
            if received_at < c.poll.at || received_at > at {
                return Err(Error::Invalid);
            }
            let cooldown = if status == 429 {
                Some(received_at.saturating_add(retry_after_ms.unwrap_or(60000).clamp(1, 3600000)))
            } else {
                None
            };
            (body, received_at, Some(status), status == 200, cooldown)
        }
    };
    // HTTP success alone cannot certify the original requested scan. Retain a
    // mismatched/malformed page as unresolved, without a successful cursor.
    let normalize = normalize
        && (!matches!(c.poll.kind, Kind::Trades)
            || fills::rest(
                &profile.scope,
                &profile
                    .mappings
                    .iter()
                    .map(|m| m.wire.clone())
                    .collect::<Vec<_>>(),
                body.as_bytes(),
            )
            .is_ok_and(|p| c.poll.range.is_none_or(|r| r == (p.start_slot, p.end_slot))));
    let hash: [u8; 32] = Sha256::digest(body.as_bytes()).into();
    let result_code = if code == Some(200) && !normalize {
        None
    } else {
        code
    };
    let extra = gateway.read_result(c.poll.reservation, hash, result_code, cooldown)?;
    observation::ingest_with(
        gateway,
        j,
        Message {
            kind: c.poll.kind,
            body,
            received_at,
        },
        c.poll.evidence,
        at,
        normalize,
        extra,
    )
}
