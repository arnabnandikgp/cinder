//! Durable evidence ingestion. Derived views are diagnostics, never another ledger.
use crate::{Error, client_id, native::*, profile::*};
use cinder_journal::{Backend, Committed, Journal, Protection, model::*};
use cinder_kernel::{
    amounts::*,
    identity::*,
    ledger::{economics::*, *},
    math::DecimalScale,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const MAGIC: &[u8] = b"CINDER-PACIFICA-EVIDENCE-1\0";
const INPUT_REF: &[u8] = b"CINDER-PACIFICA-INPUT-1\0";
/// Maximum exact response body; larger pages must be requested with smaller limits.
pub const MAX_BODY: usize = 16384;
/// Maximum records per response; body and journal limits also apply.
pub const MAX_ROWS: usize = 32;

/// A configured source endpoint/channel, not an arbitrary URL or hostname.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Kind {
    /// GET trades/history.
    Trades,
    /// WS account_trades.
    TradeStream,
    /// GET positions (full replacement).
    Positions,
    /// WS account_positions (full replacement, including empty).
    PositionStream,
    /// GET funding/history; private allocation cut/rate remain unqualified.
    Funding,
    /// GET account; reported balance is not automatically settled cash.
    Account,
    /// GET orders/history; status alone is not terminal completeness.
    Orders,
    /// Explicit loss of stream continuity; no fabricated missing executions.
    Disconnect,
}
/// Exact authenticated transport body plus trusted request routing metadata.
/// Caller excludes credentials/headers and authenticates the upstream connection.
#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    /// Native endpoint or channel.
    pub kind: Kind,
    /// Exact pool account bound in the request/subscription.
    pub account: String,
    /// Request cursor (not response cursor). None starts a new bounded scan.
    pub cursor: Option<String>,
    /// Injected receive time; never taken from the venue body.
    pub received_at: u64,
    /// Exact JSON response bytes. UTF-8 required; no normalization before archive.
    pub body: String,
}
impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Message([PRIVATE])")
    }
}
#[derive(Serialize, Deserialize)]
struct Archive {
    profile: [u8; 32],
    message: Message,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    coverage: Option<Coverage>,
}
/// Trusted complete-history port input, not inferred from REST exhaustion, LI or
/// a cancel acknowledgement. Caller authenticates and retains the certificate.
/// Construction alone does not qualify a live provider (G01/P23).
#[derive(Clone, Serialize, Deserialize)]
pub struct Coverage {
    /// Immutable profile commitment, including source and native account.
    pub profile: [u8; 32],
    /// SHA-256 of this exact response body; a certificate cannot be transplanted.
    pub body: [u8; 32],
    /// Independently qualified source-local causal frontier; not a chain slot.
    pub through: u64,
    /// Nonzero retained certificate commitment, not itself a proof verifier.
    pub evidence: [u8; 32],
}
/// Named qualification/continuity conditions. None of these is silently written off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Gap {
    /// Wrong source, malformed data, unsupported decimals or native event family.
    Unnormalized,
    /// Missing qualified precision or net-of-fee execution semantics.
    FillQualification,
    /// No verified funding boundary, private gross rate and precision convention.
    FundingQualification,
    /// Cursor skipped, repeated, cycled or missing while has_more is true.
    Pagination,
    /// Stream was disconnected; a new snapshot does not certify historical coverage.
    Disconnected,
    /// Same history identity changed native content or attribution metadata.
    ConflictingTrade,
    /// Different economics under equal whole-position-set Last ID.
    ConflictingSnapshot,
    /// A newly discovered older execution could change cost-basis order.
    LateExecution,
}
/// Native quantity plus display entry/funding; not exact ledger basis or accrual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionView {
    /// Exact normalized signed lots.
    pub quantity: QuantityLots,
    /// Preserved rounded native entry string, not converted into exact basis.
    pub displayed_entry: String,
    /// Native cumulative displayed funding, not a settlement boundary.
    pub displayed_funding: String,
    /// Native isolation flag, not a pool isolation guarantee.
    pub isolated: bool,
}
/// Native account metrics deliberately remain separate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountView {
    /// Reported balance before native settlement; not a ledger cash setter.
    pub balance: QuoteAtoms,
    /// Marked native account equity.
    pub equity: QuoteAtoms,
    /// Native spendable margin, not customer withdrawable collateral.
    pub spendable: QuoteAtoms,
    /// Native withdrawal availability, not a user entitlement.
    pub withdrawable: QuoteAtoms,
    /// Pending balance reported separately.
    pub pending: QuoteAtoms,
    /// Native update time, not authoritative event completeness.
    pub updated_at: u64,
}
/// Normalized native order status without a fabricated complete-fill certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderView {
    /// Native order identity, lossless across u64 range.
    pub id: u64,
    /// Total absolute lots.
    pub amount: u64,
    /// Observed absolute cumulative filled lots, not a ledger posting.
    pub filled: u64,
    /// Native lifecycle string (closed does not mean all economic events arrived).
    pub status: String,
    /// Native update time.
    pub updated_at: u64,
}
/// Bounded projection replayed from the same journal; not independently persisted.
#[derive(Clone, Default)]
pub struct View {
    /// Sticky conditions; generic acknowledgement cannot clear them.
    pub gaps: BTreeSet<Gap>,
    /// Latest whole-account snapshot; every configured market includes explicit flat.
    pub positions: Option<(u64, Vec<PositionView>)>,
    /// Latest account metrics, no substitution into the financial ledger.
    pub account: Option<AccountView>,
    /// Native order views, not independent order ownership.
    pub orders: BTreeMap<u64, OrderView>,
    /// Next cursor for each REST scan. End of scan is NOT complete settlement.
    pub cursors: BTreeMap<Kind, Option<String>>,
    seen_cursors: BTreeMap<Kind, BTreeSet<String>>,
    trades: BTreeMap<u64, Vec<u8>>,
    latest_trade: u64,
}
impl View {
    /// Intentionally unavailable until venue causal completeness is qualified.
    /// Neither LI nor has_more=false closes this production evidence gap.
    pub fn complete_history(&self) -> bool {
        false
    }
}
fn decode<T: DeserializeOwned>(body: &str) -> Result<T, Error> {
    serde_json::from_str(body).map_err(|_| Error::Codec)
}
fn page<T: DeserializeOwned>(body: &str) -> Result<Rest<T>, Error> {
    let result: Rest<T> = decode(body)?;
    if !result.success || result.error.is_some() {
        return Err(Error::Codec);
    }
    Ok(result)
}
fn rows<T>(rows: &[T]) -> Result<(), Error> {
    if rows.len() > MAX_ROWS {
        Err(Error::Limit)
    } else {
        Ok(())
    }
}
fn sign(side: &str) -> Result<i64, Error> {
    match side {
        "bid" | "open_long" | "close_short" => Ok(1),
        "ask" | "open_short" | "close_long" => Ok(-1),
        _ => Err(Error::Codec),
    }
}
fn magnitude(profile: &Profile, symbol: &str, amount: &str) -> Result<QuantityLots, Error> {
    let m = profile.mapping(symbol)?;
    let n = i64::try_from(m.size.parse(amount)?).map_err(|_| Error::Codec)?;
    if n < 0 {
        return Err(Error::Codec);
    }
    Ok(QuantityLots::new(
        profile.config.markets[m.market].unit(),
        n,
    ))
}
fn valid_decimal(s: &str) -> Result<(), Error> {
    let places = s.split_once('.').map_or(0, |(_, f)| f.len());
    DecimalScale::new(u8::try_from(places).map_err(|_| Error::Codec)?)?.parse(s)?;
    Ok(())
}
impl View {
    fn pagination<T>(&mut self, m: &Message, p: &Rest<T>) -> Result<(), Error> {
        if m.cursor.is_none() {
            self.seen_cursors.insert(m.kind, BTreeSet::new());
        } else if self.cursors.get(&m.kind) != Some(&m.cursor) {
            return Err(Error::Qualification);
        }
        let more = p.has_more.ok_or(Error::Qualification)?;
        if more != p.next_cursor.is_some() {
            return Err(Error::Qualification);
        }
        if let Some(next) = &p.next_cursor {
            if next.is_empty() || next.len() > 256 || !next.is_ascii() {
                return Err(Error::Codec);
            }
            let seen = self.seen_cursors.entry(m.kind).or_default();
            if seen.len() >= 128 || !seen.insert(next.clone()) {
                return Err(Error::Qualification);
            }
        }
        self.cursors.insert(m.kind, p.next_cursor.clone());
        Ok(())
    }
    fn inspect(
        &mut self,
        profile: &Profile,
        message: &Message,
        state: &State,
    ) -> Result<Vec<Event>, Error> {
        let mut candidate = self.clone();
        match candidate.inspect_page(profile, message, state) {
            Ok(events) => {
                *self = candidate;
                Ok(events)
            }
            Err(error) => {
                self.gaps.extend(candidate.gaps);
                self.gaps.insert(Gap::Unnormalized);
                Err(error)
            }
        }
    }
    fn inspect_page(
        &mut self,
        profile: &Profile,
        message: &Message,
        state: &State,
    ) -> Result<Vec<Event>, Error> {
        if message.account != profile.account {
            return Err(Error::Qualification);
        }
        let mut events = Vec::new();
        match message.kind {
            Kind::Disconnect => {
                self.gaps.insert(Gap::Disconnected);
            }
            Kind::Trades | Kind::TradeStream => {
                let trades = if message.kind == Kind::Trades {
                    let p: Rest<Vec<Trade>> = page(&message.body)?;
                    if self.pagination(message, &p).is_err() {
                        self.gaps.insert(Gap::Pagination);
                    }
                    p.data
                } else {
                    let p: Ws<Vec<Trade>> = decode(&message.body)?;
                    if p.channel != "account_trades" {
                        return Err(Error::Codec);
                    }
                    p.data
                };
                rows(&trades)?;
                // REST often returns newest first. Sorting this bounded batch is
                // not proof that unseen earlier fills do not exist.
                let mut trades = trades;
                trades.sort_by_key(|t| (t.created_at, t.history_id));
                for t in trades {
                    let result = (|| -> Result<Option<Event>, Error> {
                        if t.account.as_ref().is_some_and(|a| a != &profile.account)
                            || (message.kind == Kind::TradeStream && t.account.is_none())
                            || t.instrument_type.is_some_and(|i| i != profile.perp_tag)
                            || t.spot_fee.is_some()
                            || t.cause != "normal"
                            || !matches!(t.event_type.as_str(), "fulfill_maker" | "fulfill_taker")
                            || t.created_at > message.received_at
                        {
                            return Err(Error::Qualification);
                        }
                        let q = magnitude(profile, &t.symbol, &t.amount)?;
                        if q.lots() == 0 {
                            return Err(Error::Codec);
                        }
                        let quantity = QuantityLots::new(
                            q.unit(),
                            q.lots().checked_mul(sign(&t.side)?).ok_or(Error::Limit)?,
                        );
                        let mapping = profile.mapping(&t.symbol)?;
                        let price = PriceTicks::new(
                            q.unit(),
                            u64::try_from(mapping.price.parse(&t.price)?)
                                .map_err(|_| Error::Codec)?,
                        )?;
                        valid_decimal(&t.entry_price)?;
                        let fee = profile.quote(&t.fee)?;
                        let pnl = profile.quote(&t.pnl)?;
                        // LI is delivery metadata, not a completeness cut or dedup key.
                        let _li = t.li;
                        let canonical = serde_json::to_vec(&(
                            t.order_id,
                            &t.client_order_id,
                            &t.symbol,
                            quantity.lots(),
                            price.ticks(),
                            fee.atoms(),
                            pnl.atoms(),
                            &t.entry_price,
                            &t.side,
                            &t.event_type,
                            t.created_at,
                        ))
                        .map_err(|_| Error::Codec)?;
                        if let Some(old) = self.trades.get(&t.history_id) {
                            if *old != canonical {
                                self.gaps.insert(Gap::ConflictingTrade);
                            }
                        } else {
                            if self.trades.len() >= 4096 {
                                return Err(Error::Limit);
                            }
                            if t.created_at < self.latest_trade {
                                self.gaps.insert(Gap::LateExecution);
                                // Retain, but do not insert an older fill into today's
                                // basis order. Explicit ordered reconciliation required.
                                return Ok(None);
                            }
                            self.trades.insert(t.history_id, canonical);
                            self.latest_trade = self.latest_trade.max(t.created_at);
                        }
                        if profile.precision != Level::Qualified
                            || profile.fills != Level::Qualified
                        {
                            self.gaps.insert(Gap::FillQualification);
                            return Ok(None);
                        }
                        let candidates: Vec<_> = state
                            .orders()
                            .iter()
                            .filter_map(|o| o.attempt)
                            .filter(|a| t.client_order_id.as_ref() == Some(&client_id(*a)))
                            .collect();
                        if candidates.len() > 1 {
                            return Err(Error::Qualification);
                        }
                        let target = candidates
                            .first()
                            .copied()
                            .map(FillTarget::Customer)
                            .unwrap_or(FillTarget::Unattributed);
                        let key = EventKey {
                            scope: profile.source,
                            event: EconomicEventId::new(
                                format!("trade:{}", t.history_id).as_bytes(),
                            )?,
                            leg: 0,
                        };
                        Ok(Some(Event {
                            key: RecordKey::Economic(key),
                            policy: profile.config.policy,
                            change: Change::Economics(EconomicChange::Execution {
                                target,
                                quantity,
                                price,
                                fee,
                                pnl: Some(NativePnl::NetOfFee(pnl)),
                            }),
                        }))
                    })();
                    match result {
                        Ok(Some(event)) => events.push(event),
                        Ok(None) => {}
                        Err(_) => {
                            self.gaps.insert(Gap::Unnormalized);
                        }
                    }
                }
            }
            Kind::Positions | Kind::PositionStream => {
                let (last, positions) = if message.kind == Kind::Positions {
                    let p: Rest<Vec<NativePosition>> = page(&message.body)?;
                    (p.last_order_id, p.data)
                } else {
                    let p: Ws<Vec<NativePosition>> = decode(&message.body)?;
                    if p.channel != "account_positions" {
                        return Err(Error::Codec);
                    }
                    (p.li, p.data)
                };
                rows(&positions)?;
                let last = last.ok_or(Error::Qualification)?;
                let mut parsed = profile
                    .config
                    .markets
                    .iter()
                    .map(|m| PositionView {
                        quantity: QuantityLots::new(m.unit(), 0),
                        displayed_entry: "0".into(),
                        displayed_funding: "0".into(),
                        isolated: false,
                    })
                    .collect::<Vec<_>>();
                let mut symbols = BTreeSet::new();
                for p in positions {
                    if !symbols.insert(p.symbol.clone()) {
                        return Err(Error::Codec);
                    }
                    let quantity = magnitude(profile, &p.symbol, &p.amount)?;
                    valid_decimal(&p.entry_price)?;
                    valid_decimal(&p.funding)?;
                    let index = profile.mapping(&p.symbol)?.market;
                    parsed[index] = PositionView {
                        quantity: QuantityLots::new(
                            quantity.unit(),
                            quantity
                                .lots()
                                .checked_mul(sign(&p.side)?)
                                .ok_or(Error::Limit)?,
                        ),
                        displayed_entry: p.entry_price,
                        displayed_funding: p.funding,
                        isolated: p.isolated,
                    };
                }
                match &self.positions {
                    Some((old, values)) if *old == last && *values != parsed => {
                        self.gaps.insert(Gap::ConflictingSnapshot);
                    }
                    Some((old, _)) if *old > last => {}
                    _ => self.positions = Some((last, parsed)),
                }
            }
            Kind::Funding => {
                let p: Rest<Vec<Funding>> = page(&message.body)?;
                rows(&p.data)?;
                if self.pagination(message, &p).is_err() {
                    self.gaps.insert(Gap::Pagination);
                }
                for f in p.data {
                    magnitude(profile, &f.symbol, &f.amount)?;
                    sign(&f.side)?;
                    profile.quote(&f.payout)?;
                    valid_decimal(&f.rate)?;
                    if f.history_id == 0 || f.created_at > message.received_at {
                        return Err(Error::Codec);
                    }
                }
                self.gaps.insert(Gap::FundingQualification);
            }
            Kind::Account => {
                let p: Rest<Account> = page(&message.body)?;
                let a = p.data;
                if a.updated_at > message.received_at {
                    return Err(Error::Codec);
                }
                let next = AccountView {
                    balance: profile.quote(&a.balance)?,
                    equity: profile.quote(&a.account_equity)?,
                    spendable: profile.quote(&a.available_to_spend)?,
                    withdrawable: profile.quote(&a.available_to_withdraw)?,
                    pending: profile.quote(&a.pending_balance)?,
                    updated_at: a.updated_at,
                };
                if self
                    .account
                    .as_ref()
                    .is_none_or(|v| v.updated_at <= next.updated_at)
                {
                    self.account = Some(next);
                }
            }
            Kind::Orders => {
                let p: Rest<Vec<Order>> = page(&message.body)?;
                rows(&p.data)?;
                if self.pagination(message, &p).is_err() {
                    self.gaps.insert(Gap::Pagination);
                }
                for o in p.data {
                    sign(&o.side)?;
                    let _native_cloid = o.client_order_id;
                    let amount = magnitude(profile, &o.symbol, &o.amount)?.magnitude();
                    let filled = magnitude(profile, &o.symbol, &o.filled_amount)?.magnitude();
                    if filled > amount
                        || o.updated_at > message.received_at
                        || !matches!(
                            o.order_status.as_str(),
                            "open" | "partially_filled" | "filled" | "cancelled" | "rejected"
                        )
                    {
                        return Err(Error::Codec);
                    }
                    if self.orders.len() >= 4096 && !self.orders.contains_key(&o.order_id) {
                        return Err(Error::Limit);
                    }
                    let next = OrderView {
                        id: o.order_id,
                        amount,
                        filled,
                        status: o.order_status,
                        updated_at: o.updated_at,
                    };
                    if self
                        .orders
                        .get(&o.order_id)
                        .is_none_or(|v| v.updated_at <= next.updated_at)
                    {
                        self.orders.insert(o.order_id, next);
                    }
                }
            }
        }
        Ok(events)
    }
}
fn archive(
    profile: [u8; 32],
    message: Message,
    coverage: Option<Coverage>,
) -> Result<PrivateBytes, Error> {
    let mut bytes = MAGIC.to_vec();
    bytes.extend(
        serde_json::to_vec(&Archive {
            profile,
            message,
            coverage,
        })
        .map_err(|_| Error::Codec)?,
    );
    Ok(PrivateBytes::new(bytes)?)
}
/// Replay diagnostic views from accepted encrypted-journal evidence only.
/// New/changed profiles cannot silently reinterpret previously accepted history.
pub fn replay<B: Backend, P: Protection>(
    journal: &mut Journal<B, P>,
    profile: &Profile,
) -> Result<View, Error> {
    if journal.configuration() != &profile.config {
        return Err(Error::Qualification);
    }
    journal.verified_state()?;
    let hash = profile.commitment()?;
    let mut view = View::default();
    for tx in journal.transactions() {
        for evidence in &tx.evidence {
            if let Some(bytes) = evidence.as_bytes().strip_prefix(MAGIC) {
                let a: Archive = serde_json::from_slice(bytes).map_err(|_| Error::Codec)?;
                if a.profile != hash {
                    return Err(Error::Qualification);
                }
                if view.inspect(profile, &a.message, journal.state()?).is_err() {
                    view.gaps.insert(Gap::Unnormalized);
                }
            }
        }
    }
    Ok(view)
}
/// Ingest one bounded response atomically with its normalized facts and provenance.
/// Cursor progress is derived only after commit. Unknown storage outcomes cannot
/// advance the caller's view. Source LI never becomes `Input.source_cut`.
pub fn ingest<B: Backend, P: Protection>(
    journal: &mut Journal<B, P>,
    profile: &Profile,
    message: Message,
    id: CommitId,
    at: u64,
) -> Result<Committed, Error> {
    ingest_inner(journal, profile, message, id, at, None)
}
/// Ingest fills with a separately qualified causal certificate. Supply it before
/// first attribution: replay cannot upgrade an earlier unqualified fill's cut.
/// Ordinary ingestion never calls this port automatically.
pub fn ingest_covered<B: Backend, P: Protection>(
    journal: &mut Journal<B, P>,
    profile: &Profile,
    message: Message,
    id: CommitId,
    at: u64,
    coverage: Coverage,
) -> Result<Committed, Error> {
    if coverage.profile != profile.commitment()?
        || coverage.body != <[u8; 32]>::from(Sha256::digest(message.body.as_bytes()))
        || coverage.through == 0
        || coverage.evidence == [0; 32]
        || !matches!(message.kind, Kind::Trades | Kind::TradeStream)
    {
        return Err(Error::Qualification);
    }
    ingest_inner(journal, profile, message, id, at, Some(coverage))
}
fn ingest_inner<B: Backend, P: Protection>(
    journal: &mut Journal<B, P>,
    profile: &Profile,
    message: Message,
    id: CommitId,
    at: u64,
    coverage: Option<Coverage>,
) -> Result<Committed, Error> {
    if message.body.len() > MAX_BODY
        || message.account.len() > 256
        || message.cursor.as_ref().is_some_and(|c| c.len() > 256)
    {
        return Err(Error::Limit);
    }
    if message.received_at > at {
        return Err(Error::Qualification);
    }
    let cut = coverage.as_ref().map(|c| c.through);
    let evidence = archive(profile.commitment()?, message.clone(), coverage)?;
    if let Some(old) = journal.transaction(id) {
        if old.evidence != [evidence.clone()] || old.at != at {
            return Err(Error::Journal(cinder_journal::Error::Conflict));
        }
        return Ok(journal.commit(old.clone())?);
    }
    let mut view = replay(journal, profile)?;
    let old_gaps = view.gaps.clone();
    let result = view.inspect(profile, &message, journal.state()?);
    // Exact body/routing metadata lives once in evidence. Each input points to
    // that immutable archive and its normalized-event ordinal (MAX for a gap).
    let digest = Sha256::digest(evidence.as_bytes());
    let reference = |ordinal: u32| -> Result<PrivateBytes, Error> {
        let mut bytes = INPUT_REF.to_vec();
        bytes.extend_from_slice(&digest);
        bytes.extend_from_slice(&ordinal.to_be_bytes());
        Ok(PrivateBytes::new(bytes)?)
    };
    let mut inputs = Vec::new();
    let mut unnormalized = false;
    match result {
        Ok(events) => {
            for (ordinal, event) in events.into_iter().enumerate() {
                inputs.push(Input {
                    source: profile.source,
                    source_cut: cut,
                    authority_epoch: profile.revision,
                    observed_at: message.received_at,
                    raw: reference(u32::try_from(ordinal).map_err(|_| Error::Limit)?)?,
                    event: Some(event),
                });
            }
        }
        Err(_) => unnormalized = true,
    }
    // Known gaps are mirrored into the authoritative journal gate, not just UI state.
    if unnormalized || !view.gaps.is_subset(&old_gaps) {
        inputs.push(Input {
            source: profile.source,
            source_cut: None,
            authority_epoch: profile.revision,
            observed_at: message.received_at,
            raw: reference(u32::MAX)?,
            event: None,
        });
    }
    let tx = Transaction {
        id,
        expected: journal.head(),
        at,
        evidence: vec![evidence],
        inputs,
        order_observations: vec![],
        funds_observations: vec![],
        controls: vec![],
    };
    Ok(journal.commit(tx)?)
}
