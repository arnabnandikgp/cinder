//! Transaction-linked testnet deposit confirmation under an explicit venue-
//! bookkeeping assumption. No fabricated native cut, retry of money movement,
//! stronger completion certificate, production activation or recovery assurance.
use super::*;
use crate::{execution::Origin, observation::MAX_ROWS, reads};

const POLL: &[u8] = b"CINDER-DEMO-DEPOSIT-POLL-1\0";
const RESPONSE: &[u8] = b"CINDER-DEMO-DEPOSIT-RESPONSE-1\0";
/// Maximum private HTTP response, before JSON journal encoding overhead.
pub const MAX_BODY: usize = 8192;

/// Governed assumption for the initial idle test account, not native attestation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialSetup {
    /// Explicit approval revision; zero never opts in.
    pub revision: u64,
    /// The run controls ALL wallet/agent authority, with no external writers or
    /// uncertain setup actions. REST responses do not establish this assertion.
    pub exclusive_control: bool,
}
/// Explicit finite testnet policy; no default or mainnet counterpart exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// Nonzero approval revision, bound into every retained request/confirmation.
    pub revision: u64,
    /// Total GET attempts, including missing/malformed replies and page reads.
    pub maximum_reads: u16,
    /// Maximum pages in one deposit-history scan.
    pub maximum_pages: u8,
    /// Minimum gap between requests in milliseconds; no tight polling loops.
    pub interval_ms: u64,
    /// Maximum exponential delay after unavailable/negative observations.
    pub maximum_backoff_ms: u64,
    /// Maximum elapsed time from the original exposed deposit plan.
    pub lifetime_ms: u64,
    /// Separately approved initial setup assumption. Omission preserves the
    /// original strong setup requirement and its serialized policy commitment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_setup: Option<InitialSetup>,
}
impl Policy {
    /// Validate finite policy bounds without touching a journal or external port.
    pub fn validate(&self) -> Result<(), Error> {
        if self.revision == 0
            || !(2..=64).contains(&self.maximum_reads)
            || !(1..=8).contains(&self.maximum_pages)
            || !(1_000..=30_000).contains(&self.interval_ms)
            || self.maximum_backoff_ms < self.interval_ms
            || self.maximum_backoff_ms > 30_000
            || self.lifetime_ms < self.interval_ms
            || self.lifetime_ms > 600_000
            || self
                .initial_setup
                .as_ref()
                .is_some_and(|s| s.revision == 0 || !s.exclusive_control || self.maximum_reads < 3)
        {
            return Err(Error::Qualification);
        }
        Ok(())
    }
    /// Bind the actual loaded testnet controllers, not a caller-supplied digest.
    pub fn release_commitment(&self, c: &Controller, g: &Gateway) -> Result<[u8; 32], Error> {
        self.validate()?;
        let (profile, execution) = g.read_binding();
        if execution.origin != Origin::Testnet
            || profile.environment != Origin::Testnet.url()
            || c.profile.environment != Origin::Testnet.url()
            || profile.commitment()? != c.profile.commitment()?
            || profile.precision != Level::Qualified
        {
            return Err(Error::Qualification);
        }
        let mut h = Sha256::new();
        h.update(b"CINDER-DEMO-DEPOSIT-POLICY-1\0");
        h.update(c.contract);
        h.update(g.release_commitment(&profile.config)?);
        h.update(serde_json::to_vec(self).map_err(|_| Error::Codec)?);
        Ok(h.finalize().into())
    }
}
/// Trusted scheduler identities, not caller-selected routes or account overrides.
pub struct Poll {
    /// Original durable deposit attempt; never a replacement signature.
    pub attempt: AttemptKey,
    /// Durable read-spend identity.
    pub reservation: CommitId,
    /// Durable response identity, distinct from the spend.
    pub evidence: CommitId,
    /// Trusted injected monotone time, milliseconds.
    pub at: u64,
}
impl std::fmt::Debug for Poll {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DemoDepositPoll([PRIVATE])")
    }
}
/// Safe scheduler outcome; waiting/exhaustion never releases a money-movement hold.
pub enum Step {
    /// Wait until the retained polling cadence/backoff permits another GET.
    Waiting,
    /// The finite budget/deadline was consumed; operator reconciliation required.
    Exhausted,
    /// The original deposit is already durably demo-confirmed; no further GET.
    Confirmed,
    /// One already-reserved request and its separate single-use completion token.
    Request(reads::Request, Box<reads::Completion>),
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Endpoint {
    Deposit,
    Balance,
}
impl Endpoint {
    pub(crate) fn path(self) -> &'static str {
        match self {
            Self::Deposit => "/api/v1/account/deposit/history",
            Self::Balance => "/api/v1/account/balance/history",
        }
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Metadata {
    pub policy: [u8; 32],
    pub attempt: Vec<u8>,
    pub reservation: [u8; 32],
    pub evidence: [u8; 32],
    pub endpoint: Endpoint,
    pub cursor: Option<String>,
    pub at: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Archive {
    metadata: Metadata,
    status: u16,
    received_at: u64,
    body: Vec<u8>,
}
pub(crate) fn marker(m: &Metadata) -> Result<PrivateBytes, Error> {
    encode(POLL, m)
}
fn encode<T: Serialize>(prefix: &[u8], value: &T) -> Result<PrivateBytes, Error> {
    let mut bytes = prefix.to_vec();
    bytes.extend(serde_json::to_vec(value).map_err(|_| Error::Codec)?);
    Ok(PrivateBytes::new(bytes)?)
}
pub(crate) fn archive(
    m: &Metadata,
    status: u16,
    received_at: u64,
    body: &[u8],
) -> Result<PrivateBytes, Error> {
    if body.len() > MAX_BODY || received_at < m.at {
        return Err(Error::Qualification);
    }
    encode(
        RESPONSE,
        &Archive {
            metadata: m.clone(),
            status,
            received_at,
            body: body.to_vec(),
        },
    )
}
#[cfg(test)]
mod bounds_tests {
    use super::*;
    #[test]
    fn largest_demo_archive_fits_the_measured_record_allowance_and_oversize_refuses() {
        let metadata = Metadata {
            policy: [255; 32],
            attempt: vec![255; 160],
            reservation: [255; 32],
            evidence: [255; 32],
            endpoint: Endpoint::Deposit,
            cursor: Some("x".repeat(256)),
            at: u64::MAX,
        };
        assert!(
            archive(&metadata, 200, u64::MAX, &vec![255; MAX_BODY])
                .unwrap()
                .as_bytes()
                .len()
                < MAX_BODY * 4 + 8192
        );
        assert!(archive(&metadata, 200, u64::MAX, &vec![255; MAX_BODY + 1]).is_err());
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Page<T> {
    success: bool,
    data: Vec<T>,
    error: Option<String>,
    code: Option<i64>,
    next_cursor: Option<String>,
    has_more: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Deposit {
    amount: String,
    transaction_id: String,
    created_at: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Balance {
    amount: String,
    balance: String,
    pending_balance: String,
    event_type: String,
    created_at: u64,
}
fn page<T: for<'de> Deserialize<'de>>(a: &Archive) -> Result<Page<T>, Error> {
    if a.status != 200 || a.body.is_empty() || a.body.len() > MAX_BODY {
        return Err(Error::Qualification);
    }
    let p: Page<T> = serde_json::from_slice(&a.body).map_err(|_| Error::Codec)?;
    if !p.success
        || p.error.is_some()
        || p.code.is_some()
        || p.data.len() > MAX_ROWS
        || p.has_more != p.next_cursor.is_some()
        || p.next_cursor.as_ref().is_some_and(|v| {
            v.is_empty() || v.len() > 256 || !v.bytes().all(|b| (33..=126).contains(&b))
        })
    {
        return Err(Error::Qualification);
    }
    Ok(p)
}
struct Scan {
    requests: Vec<Metadata>,
    replies: Vec<Archive>,
}
fn scan<B: Backend, P: Protection>(
    j: &mut Journal<B, P>,
    attempt: AttemptKey,
    binding: [u8; 32],
) -> Result<Scan, Error> {
    j.verified_state()?;
    let attempt = attempt.encode();
    let mut requests = vec![];
    let mut replies = vec![];
    for tx in j.transactions() {
        for raw in &tx.evidence {
            if let Some(bytes) = raw.as_bytes().strip_prefix(POLL) {
                let m: Metadata = serde_json::from_slice(bytes).map_err(|_| Error::Codec)?;
                if m.attempt != attempt {
                    continue;
                }
                if m.policy != binding || m.reservation != tx.id.bytes() || m.at != tx.at {
                    return Err(Error::Qualification);
                }
                requests.push(m);
            } else if let Some(bytes) = raw.as_bytes().strip_prefix(RESPONSE) {
                let a: Archive = serde_json::from_slice(bytes).map_err(|_| Error::Codec)?;
                if a.metadata.attempt != attempt {
                    continue;
                }
                if a.metadata.policy != binding
                    || a.metadata.evidence != tx.id.bytes()
                    || a.received_at < a.metadata.at
                    || a.received_at > tx.at
                    || !requests.contains(&a.metadata)
                {
                    return Err(Error::Qualification);
                }
                replies.push(a);
            }
        }
    }
    Ok(Scan { requests, replies })
}
fn deposit_match(
    c: &Controller,
    scan: &Scan,
    signature: [u8; 64],
    gross: u64,
    start: u64,
) -> Result<Option<u64>, Error> {
    let mut found = None;
    for a in scan
        .replies
        .iter()
        .filter(|a| a.metadata.endpoint == Endpoint::Deposit && a.status == 200)
    {
        let Ok(p) = page::<Deposit>(a) else {
            continue;
        };
        for row in p.data {
            let Ok(decoded) = bs58::decode(&row.transaction_id).into_vec() else {
                return Err(Error::Codec);
            };
            let tx: [u8; 64] = decoded.try_into().map_err(|_| Error::Codec)?;
            if tx == [0; 64] || bs58::encode(tx).into_string() != row.transaction_id {
                return Err(Error::Codec);
            }
            if tx != signature {
                continue;
            }
            if c.profile.quote(&row.amount)?.atoms() != i128::from(gross)
                || row.created_at < start
                || row.created_at > a.received_at
                || found.is_some_and(|old| old != row.created_at)
            {
                return Err(Error::Qualification);
            }
            found = Some(row.created_at);
        }
    }
    Ok(found)
}
fn available(c: &Controller, scan: &Scan, gross: u64, deposited_at: u64) -> Result<bool, Error> {
    let Some(a) = scan
        .replies
        .iter()
        .rev()
        .find(|a| a.metadata.endpoint == Endpoint::Balance)
    else {
        return Ok(false);
    };
    available_reply(c, a, gross, deposited_at)
}
fn available_reply(
    c: &Controller,
    a: &Archive,
    gross: u64,
    deposited_at: u64,
) -> Result<bool, Error> {
    let Ok(p) = page::<Balance>(a) else {
        return Ok(false);
    };
    let Some(latest) = p.data.iter().max_by_key(|r| r.created_at) else {
        return Ok(false);
    };
    if p.data
        .iter()
        .filter(|r| r.created_at == latest.created_at)
        .count()
        != 1
        || latest.created_at < deposited_at
        || latest.created_at > a.received_at
        || !matches!(latest.event_type.as_str(), "deposit" | "deposit_release")
    {
        return Ok(false);
    }
    Ok(
        c.profile.quote(&latest.amount)?.atoms() == i128::from(gross)
            && c.profile.quote(&latest.balance)?.atoms() == i128::from(gross)
            && c.profile.quote(&latest.pending_balance)?.atoms() == 0,
    )
}
impl Controller {
    /// Select the unique existing original deposit for the measured scheduler.
    /// Missing setup/plan/finality waits without I/O; conflicting history refuses.
    /// This never creates an intent, exposes a transfer or grants signing authority.
    pub fn demo_deposit_candidate<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        g: &Gateway,
        policy: &Policy,
        at: u64,
    ) -> Result<Option<AttemptKey>, Error> {
        let binding = policy.release_commitment(self, g)?;
        let history = self.history(j, at)?;
        let s = j.state()?;
        if s.frozen() {
            return Err(Error::Qualification);
        }
        let mut candidates = s.funds().iter().filter(|o| {
            o.demo.is_some()
                || !o.terminal
                    && (o.intent.source == Location::Venue
                        || o.intent.destination == Destination::Location(Location::Venue))
        });
        let Some(o) = candidates.next() else {
            return Ok(None);
        };
        if candidates.next().is_some() || o.faulted {
            return Err(Error::Qualification);
        }
        if let Some(demo) = &o.demo {
            return if demo.policy == binding {
                Ok(None)
            } else {
                Err(Error::Qualification)
            };
        }
        if o.intent.source != Location::Broker
            || o.intent.destination != Destination::Location(Location::Venue)
        {
            return Err(Error::Qualification);
        }
        let Some(attempt) = o.attempt else {
            return Ok(None);
        };
        if !self.demo_plan_ready(&history, policy, binding, attempt, at)?
            || !history.plans.iter().any(|p| p.attempt == attempt.encode())
            || !history
                .chains
                .iter()
                .any(|(a, _, ok)| a == &attempt.encode() && *ok)
        {
            return Ok(None);
        }
        self.demo_original(j, policy, binding, attempt, at)?;
        Ok(Some(attempt))
    }
    fn demo_original<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        policy: &Policy,
        binding: [u8; 32],
        attempt: AttemptKey,
        at: u64,
    ) -> Result<(Plan, [u8; 64]), Error> {
        let history = self.history(j, at)?;
        let plan = self.existing(j, attempt, at)?;
        // Deliberately bounded first-credit demo: no pre-existing native cash,
        // positions, funding, live orders or competing native money movement.
        let s = j.state()?;
        if plan.rail != Rail::Deposit
            || !self.demo_plan_ready(&history, policy, binding, attempt, at)?
            || s.frozen()
            || s.unresolved_raw() != 0
            || s.ledger().venue().cash().atoms() != 0
            || s.ledger().venue().funding().atoms() != 0
            || s.ledger()
                .venue()
                .positions()
                .iter()
                .any(|p| p.quantity().lots() != 0 || p.basis().atoms() != 0)
            || s.orders().iter().any(|o| !o.complete())
            || s.funds().iter().any(|o| {
                o.demo.is_some()
                    || o.attempt == Some(attempt) && (o.terminal || o.faulted || o.proof.is_some())
                    || o.attempt != Some(attempt)
                        && !o.terminal
                        && (o.intent.source == Location::Venue
                            || o.intent.destination == Destination::Location(Location::Venue))
            })
        {
            return Err(Error::Qualification);
        }
        let (_, signature, _) = history
            .chains
            .iter()
            .find(|(a, _, ok)| a == &plan.attempt && *ok)
            .ok_or(Error::Qualification)?;
        Ok((
            plan,
            signature.as_slice().try_into().map_err(|_| Error::Codec)?,
        ))
    }
    /// Prepare one bounded read; no loop, sleep, socket or money-movement retry.
    /// All cadence/page/budget state is rebuilt from the same durable journal.
    pub fn prepare_demo_deposit_poll<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        g: &Gateway,
        policy: &Policy,
        p: Poll,
    ) -> Result<Step, Error> {
        let binding = policy.release_commitment(self, g)?;
        if let Some(o) = j
            .verified_state()?
            .funds()
            .iter()
            .find(|o| o.attempt == Some(p.attempt) && o.demo.is_some())
        {
            return if !o.faulted && o.demo.as_ref().is_some_and(|d| d.policy == binding) {
                Ok(Step::Confirmed)
            } else {
                Err(Error::Qualification)
            };
        }
        let (plan, signature) = self.demo_original(j, policy, binding, p.attempt, p.at)?;
        let scan = scan(j, p.attempt, binding)?;
        if p.at
            >= plan
                .at
                .checked_add(policy.lifetime_ms)
                .ok_or(Error::Limit)?
            || scan.requests.len() >= usize::from(policy.maximum_reads)
        {
            return Ok(Step::Exhausted);
        }
        let mut endpoint = Endpoint::Deposit;
        let mut cursor = None;
        let deposited_at = deposit_match(self, &scan, signature, plan.gross, plan.at)?;
        if deposited_at.is_some() {
            endpoint = Endpoint::Balance;
        }
        if let Some(last) = scan.requests.last() {
            let reply = scan.replies.iter().find(|a| a.metadata == *last);
            let failures = scan
                .requests
                .iter()
                .rev()
                .take_while(|m| {
                    !scan.replies.iter().any(|a| {
                        if a.metadata != **m {
                            return false;
                        }
                        match m.endpoint {
                            Endpoint::Deposit => page::<Deposit>(a).is_ok_and(|p| {
                                p.has_more
                                    || p.data.iter().any(|r| {
                                        r.transaction_id == bs58::encode(signature).into_string()
                                    })
                            }),
                            Endpoint::Balance => deposited_at.is_some_and(|at| {
                                available_reply(self, a, plan.gross, at).unwrap_or(false)
                            }),
                        }
                    })
                })
                .count()
                .min(8);
            let delay = policy
                .interval_ms
                .saturating_mul(1_u64 << failures)
                .min(policy.maximum_backoff_ms);
            let base = reply.map_or(last.at, |a| a.received_at);
            if p.at < base.checked_add(delay).ok_or(Error::Limit)? {
                return Ok(Step::Waiting);
            }
            if endpoint == Endpoint::Deposit
                && last.endpoint == Endpoint::Deposit
                && let Some(page) = reply.and_then(|a| page::<Deposit>(a).ok())
                && page.has_more
            {
                let round = scan
                    .requests
                    .iter()
                    .rev()
                    .take_while(|m| m.cursor.is_some())
                    .count()
                    + 1;
                if round >= usize::from(policy.maximum_pages) {
                    return Ok(Step::Exhausted);
                }
                if scan
                    .requests
                    .iter()
                    .rev()
                    .take(round)
                    .any(|m| m.cursor == page.next_cursor)
                {
                    return Err(Error::Qualification);
                }
                cursor = page.next_cursor;
            }
        }
        let metadata = Metadata {
            policy: binding,
            attempt: p.attempt.encode(),
            reservation: p.reservation.bytes(),
            evidence: p.evidence.bytes(),
            endpoint,
            cursor,
            at: p.at,
        };
        let (request, completion) = reads::prepare_demo(j, g, metadata)?;
        Ok(Step::Request(request, Box::new(completion)))
    }
    /// Atomically post the full original credit and its explicit weaker closure.
    /// Only source-authenticated retained GET evidence is eligible at this port.
    /// No balance-only attribution, arbitrary cut, readiness or recovery grant.
    pub fn confirm_demo_deposit<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        g: &Gateway,
        policy: &Policy,
        attempt: AttemptKey,
        id: CommitId,
        at: u64,
    ) -> Result<bool, Error> {
        let binding = policy.release_commitment(self, g)?;
        if let Some(o) = j
            .verified_state()?
            .funds()
            .iter()
            .find(|o| o.attempt == Some(attempt) && o.demo.is_some())
        {
            return if !o.faulted && o.demo.as_ref().is_some_and(|d| d.policy == binding) {
                Ok(true)
            } else {
                Err(Error::Qualification)
            };
        }
        let (plan, signature) = self.demo_original(j, policy, binding, attempt, at)?;
        let scan = scan(j, attempt, binding)?;
        if at
            >= plan
                .at
                .checked_add(policy.lifetime_ms)
                .ok_or(Error::Limit)?
        {
            return Ok(false);
        }
        let Some(deposited_at) = deposit_match(self, &scan, signature, plan.gross, plan.at)? else {
            return Ok(false);
        };
        if !available(self, &scan, plan.gross, deposited_at)? {
            return Ok(false);
        }
        let debit = EventKey {
            scope: self.scope(Location::Broker)?,
            event: EconomicEventId::new(&signature)?,
            leg: 0,
        };
        let credit = EventKey {
            scope: self.profile.source,
            event: EconomicEventId::new(&[b"demo-deposit:".as_slice(), &signature].concat())?,
            leg: 1,
        };
        let confirmation = funds::DemoDeposit {
            attempt,
            policy: binding,
            debit,
            credit: credit.clone(),
            amount: QuoteAtoms::new(self.profile.config.quote, i128::from(plan.gross)),
        };
        let raw = encode(
            b"CINDER-DEMO-DEPOSIT-CONFIRMATION-1\0",
            &(
                binding,
                signature.to_vec(),
                scan.requests.iter().map(|m| m.evidence).collect::<Vec<_>>(),
            ),
        )?;
        let input = Input {
            source: self.profile.source,
            source_cut: None,
            authority_epoch: self.profile.revision,
            observed_at: at,
            raw,
            event: Some(Event {
                key: RecordKey::Economic(credit),
                policy: self.profile.config.policy,
                change: Change::Funds(FundsChange::Observe {
                    attempt,
                    leg: Leg::Arrive(Destination::Location(Location::Venue)),
                    amount: confirmation.amount,
                    fee: QuoteAtoms::new(self.profile.config.quote, 0),
                }),
            }),
        };
        let committed = j.commit(Transaction {
            id,
            expected: j.head(),
            at,
            evidence: vec![],
            inputs: vec![input],
            order_observations: vec![],
            funds_observations: vec![],
            controls: vec![Control::Funds(funds::Action::ConfirmDemoDeposit(Box::new(
                confirmation,
            )))],
        })?;
        checked(committed)?;
        Ok(true)
    }
}
