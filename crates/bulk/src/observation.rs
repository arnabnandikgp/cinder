//! Exact authenticated native observations into the SAME selected financial
//! component. Snapshot metrics never set cash; page coverage never releases holds.
use crate::{
    Error,
    execution::Gateway,
    fills::{self, Fill, Reason},
    financial::{self, Profile},
};
use cinder_journal::{
    Committed,
    controller::Controller,
    model::{CommitId, Input, PrivateBytes, Transaction},
};
use cinder_kernel::{
    amounts::{PriceTicks, QuantityLots, QuoteAtoms},
    identity::*,
    ledger::{Change, Event, FillTarget, economics::EconomicChange},
};
use sha2::{Digest, Sha256};

pub(crate) const MAGIC: &[u8] = b"CINDER-BULK-OBSERVATION-1\0";
/// Exact native family selected by the private scheduler, not by response fields.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Kind {
    /// Bounded account fills page, with no inferred complete economic frontier.
    Trades,
    /// One account-topic fill from an authenticated native WebSocket.
    TradeStream,
    /// Reported account/positions; diagnostics, not additive assets.
    Account,
    /// Native payment history; gross private boundary/rate remains separate.
    Funding,
    /// Liquidation/ADL history; cause/economic linkage still needs qualification.
    Risk,
}
/// Sanitized response bytes, not credentials, host assertions or a ledger command.
pub struct Message {
    /// Trusted request/subscription family.
    pub kind: Kind,
    /// Exact bounded private body. Never log or send via parent plaintext.
    pub body: PrivateBytes,
    /// Trusted receive time in milliseconds, never native nanoseconds.
    pub received_at: u64,
}
impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BulkObservation([PRIVATE])")
    }
}
fn reference(hash: [u8; 32], index: u64) -> Result<PrivateBytes, Error> {
    let mut bytes = b"CINDER-BULK-INPUT-1\0".to_vec();
    bytes.extend(hash);
    bytes.extend(index.to_be_bytes());
    PrivateBytes::new(bytes).map_err(|_| Error::Limit)
}
fn fill_event(
    profile: &Profile,
    fill: &Fill,
    target: FillTarget,
    policy: PolicyVersion,
) -> Result<Event, Error> {
    let m = profile
        .mappings
        .iter()
        .find(|m| m.wire == fill.market)
        .ok_or(Error::Unqualified)?;
    if fill.reason != Reason::Trade {
        return Err(Error::Unqualified);
    }
    let quantity = i64::try_from(fill.quantity).map_err(|_| Error::Limit)?;
    let quantity = if fill.buy {
        quantity
    } else {
        quantity.checked_neg().ok_or(Error::Limit)?
    };
    Ok(Event {
        key: RecordKey::Economic(EventKey {
            scope: profile.scope.source,
            event: EconomicEventId::new(&fill.key).map_err(|_| Error::Invalid)?,
            leg: 0,
        }),
        policy,
        change: Change::Economics(EconomicChange::Execution {
            target,
            quantity: QuantityLots::new(m.unit, quantity),
            price: PriceTicks::new(m.unit, fill.price).map_err(|_| Error::Invalid)?,
            fee: QuoteAtoms::new(profile.quote, fill.fee),
            pnl: None,
        }),
    })
}
/// Retain raw evidence once, plus compact immutable references per normalized
/// input. Real executions ingest even with frozen admission or revoked keys.
/// Unsupported funding/forced events remain unresolved rather than synthesized.
pub fn ingest(
    gateway: &Gateway,
    j: &mut impl Controller,
    message: Message,
    id: CommitId,
    at: u64,
) -> Result<Committed, Error> {
    ingest_with(gateway, j, message, id, at, true, vec![])
}
pub(crate) fn ingest_with(
    gateway: &Gateway,
    j: &mut impl Controller,
    message: Message,
    id: CommitId,
    at: u64,
    normalize: bool,
    extra: Vec<PrivateBytes>,
) -> Result<Committed, Error> {
    ingest_record(
        gateway,
        j,
        message,
        id,
        at,
        if normalize {
            Mode::Normalize
        } else {
            Mode::Contain
        },
        extra,
    )
}
enum Mode {
    Normalize,
    Contain,
    RetainFreshPaperHistory,
}
/// Only the policy-bound, pre-allocation paper reader may select this path,
/// after validating a literal empty funding/risk page. It archives the original
/// body and HTTP linkage without manufacturing a raw financial event or cut.
pub(crate) fn retain_fresh_paper_history(
    gateway: &Gateway,
    j: &mut impl Controller,
    message: Message,
    id: CommitId,
    at: u64,
    extra: Vec<PrivateBytes>,
) -> Result<Committed, Error> {
    ingest_record(
        gateway,
        j,
        message,
        id,
        at,
        Mode::RetainFreshPaperHistory,
        extra,
    )
}
fn ingest_record(
    gateway: &Gateway,
    j: &mut impl Controller,
    message: Message,
    id: CommitId,
    at: u64,
    mode: Mode,
    extra: Vec<PrivateBytes>,
) -> Result<Committed, Error> {
    let (profile, revision) = gateway.observation_binding(j)?;
    if message.body.as_bytes().len() > fills::MAX_BODY || message.received_at > at {
        return Err(Error::Limit);
    }
    j.verified_state().map_err(|_| Error::Unavailable)?;
    let digest: [u8; 32] = Sha256::digest(message.body.as_bytes()).into();
    let mut archive = MAGIC.to_vec();
    archive.extend(profile.commitment());
    archive.push(message.kind as u8);
    archive.extend(message.received_at.to_be_bytes());
    archive.extend(message.body.as_bytes());
    let mut tx = Transaction {
        id,
        expected: j.head(),
        at,
        evidence: vec![PrivateBytes::new(archive).map_err(|_| Error::Limit)?],
        inputs: vec![],
        order_observations: vec![],
        funds_observations: vec![],
        controls: vec![],
    };
    tx.evidence.extend(extra);
    let mut fills = Vec::new();
    let unresolved;
    if matches!(mode, Mode::RetainFreshPaperHistory) {
        unresolved = false;
    } else if matches!(mode, Mode::Contain) || !gateway.financial_observations() {
        unresolved = true;
    } else {
        match message.kind {
            Kind::Trades => match fills::rest(
                &profile.scope,
                &profile
                    .mappings
                    .iter()
                    .map(|m| m.wire.clone())
                    .collect::<Vec<_>>(),
                message.body.as_bytes(),
            ) {
                Ok(page) => {
                    fills = page.fills;
                    unresolved = !page.unmapped.is_empty();
                }
                Err(_) => unresolved = true,
            },
            Kind::TradeStream => {
                // Only an already configured symbol can select a mapping. Decode
                // twice at most, not once per configured market.
                if let Ok(body) = crate::raw::decode::<serde_json::Value>(message.body.as_bytes())
                    && let Some(symbol) = body.pointer("/data/symbol").and_then(|v| v.as_str())
                    && let Some(mapping) = profile.mapping(symbol)
                    && let Ok(fill) =
                        fills::stream(&profile.scope, &mapping.wire, message.body.as_bytes())
                {
                    fills.push(fill);
                }
                unresolved = fills.len() != 1;
            }
            Kind::Account => {
                unresolved = !financial::account::decode_account(profile, message.body.as_bytes())
                    .is_ok_and(|s| {
                        s.held.is_empty()
                            && (!crate::paper::active(j)
                                || s.local
                                    .get(financial::account::Metric::Funding)
                                    .is_some_and(|v| v.atoms() == 0))
                    });
            }
            Kind::Funding => {
                // A native net payment does not determine private gross funding or
                // its eligible inventory cut. Keep it pending qualification.
                let _ = financial::history::funding(profile, message.body.as_bytes());
                unresolved = true;
            }
            Kind::Risk => {
                let _ = financial::history::risk(profile, message.body.as_bytes());
                unresolved = true;
            }
        }
    }
    for (index, fill) in fills.iter().enumerate() {
        let target = match gateway.attribution(j, fill.order, at) {
            Ok(Some(attempt)) => FillTarget::Customer(attempt),
            Ok(None) | Err(Error::Unqualified | Error::Invalid | Error::Conflict) => {
                FillTarget::Unattributed
            }
            Err(e) => return Err(e),
        };
        let event = fill_event(profile, fill, target, j.configuration().policy).ok();
        tx.inputs.push(Input {
            source: profile.scope.source,
            source_cut: None,
            authority_epoch: revision,
            observed_at: message.received_at,
            raw: reference(digest, index as u64)?,
            event,
        });
    }
    if unresolved {
        tx.inputs.push(Input {
            source: profile.scope.source,
            source_cut: None,
            authority_epoch: revision,
            observed_at: message.received_at,
            raw: reference(digest, u64::MAX)?,
            event: None,
        });
    }
    j.commit(tx).map_err(|_| Error::Unavailable)
}
pub(crate) fn continuation(
    gateway: &Gateway,
    j: &impl Controller,
    kind: Kind,
    id: CommitId,
) -> Result<String, Error> {
    let (profile, _) = gateway.observation_binding(j)?;
    let tx = j.transaction(id).ok_or(Error::Unqualified)?;
    let mut records = tx
        .evidence
        .iter()
        .filter_map(|e| e.as_bytes().strip_prefix(MAGIC));
    let record = records.next().ok_or(Error::Unqualified)?;
    if records.next().is_some()
        || record.len() < 41
        || record[..32] != profile.commitment()
        || record[32] != kind as u8
    {
        return Err(Error::Unqualified);
    }
    if !gateway.read_succeeded(tx, Sha256::digest(&record[41..]).into())? {
        return Err(Error::Unqualified);
    }
    let body = crate::raw::decode::<serde_json::Value>(&record[41..])?;
    if body.pointer("/page/hasMore").and_then(|v| v.as_bool()) != Some(true) {
        return Err(Error::Unqualified);
    }
    let cursor = body
        .pointer("/page/nextCursor")
        .and_then(|v| v.as_str())
        .ok_or(Error::Invalid)?;
    if cursor.is_empty() || cursor.len() > 1024 {
        return Err(Error::Limit);
    }
    Ok(cursor.into())
}
