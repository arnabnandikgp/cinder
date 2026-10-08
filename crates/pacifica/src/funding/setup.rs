//! Read-only setup adapter over the same durable journal. A successful finite
//! read set is never atomic setup or native coverage. An explicit governed demo
//! policy can qualify INITIAL idle setup only, without native risk readiness.
//! No key, settings POST, clock, socket or automatic retry here.
use super::*;
use crate::reads;

const REQUEST: &[u8] = b"CINDER-NATIVE-SETUP-READ-1\0";
const RESPONSE: &[u8] = b"CINDER-NATIVE-SETUP-REPLY-1\0";

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Endpoint {
    Settings,
    Loan,
    Account,
}
impl Endpoint {
    pub(crate) fn path(self) -> &'static str {
        match self {
            Self::Settings => "/api/v1/account/settings",
            Self::Loan => "/api/v1/account/loan",
            Self::Account => "/api/v1/account",
        }
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Metadata {
    binding: [u8; 32],
    pub(crate) endpoint: Endpoint,
    pub(crate) reservation: [u8; 32],
    pub(crate) evidence: [u8; 32],
    pub(crate) at: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Archive {
    metadata: Metadata,
    status: u16,
    received_at: u64,
    body: Vec<u8>,
}
fn encode<T: Serialize>(prefix: &[u8], value: &T) -> Result<PrivateBytes, Error> {
    let mut bytes = prefix.to_vec();
    bytes.extend(serde_json::to_vec(value).map_err(|_| Error::Codec)?);
    Ok(PrivateBytes::new(bytes)?)
}
pub(crate) fn marker(m: &Metadata) -> Result<PrivateBytes, Error> {
    encode(REQUEST, m)
}
pub(crate) fn archive(
    m: &Metadata,
    status: u16,
    received_at: u64,
    body: &[u8],
) -> Result<PrivateBytes, Error> {
    if body.len() > demo::MAX_BODY || received_at < m.at {
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

/// One injected scheduling cut, with two distinct durable identities.
pub struct Poll {
    /// Original durable GET-spend identity.
    pub reservation: CommitId,
    /// Distinct private response-archive identity.
    pub evidence: CommitId,
    /// Trusted injected current time in milliseconds.
    pub at: u64,
}
/// Parsed private observations only; not an authority or completeness token.
pub struct Observation {
    setup: Setup,
    /// Explicit lending-disabled setting at the observed source.
    pub lending_disabled: bool,
    /// Observed cross-only mapped-market overrides, not calibrated leverage.
    pub compatible_margin: bool,
    /// All reported account balances, margin and exposures are empty.
    pub idle: bool,
    /// Observed debt in the bound profile's quote atoms.
    pub borrowed: u64,
    /// Observed pending interest in the bound profile's quote atoms.
    pub interest: u64,
}
/// Retained demo assumption and exact source-read provenance, not a native cut.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Qualification {
    policy: [u8; 32],
    observed_at: u64,
    valid_until: u64,
    reads: [[u8; 32]; 3],
}
impl std::fmt::Debug for Observation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SetupObservation([PRIVATE])")
    }
}
/// Bounded observation status; none of these grant financial authority.
pub enum Step {
    /// Cadence or the shared read budget currently prevents another request.
    Waiting,
    /// The finite original budget/lifetime has ended; restart does not renew it.
    Exhausted,
    /// The current stronger trusted provider has already supplied fresh setup.
    Qualified,
    /// Explicit demo setup, or its already authorized original deposit. Does
    /// not enable native risk, ordinary strong ingress or another allocation.
    DemoQualified,
    /// Three coherent authenticated replies were retained, not certified complete.
    Observed(Box<Observation>),
    /// One durably charged request, separate from its single-use completion.
    Request(reads::Request, Box<reads::Completion>),
}
struct Scan {
    requests: Vec<Metadata>,
    replies: Vec<Archive>,
}
fn scan<B: Backend, P: Protection>(
    j: &mut Journal<B, P>,
    binding: [u8; 32],
) -> Result<Scan, Error> {
    j.verified_state()?;
    let mut result = Scan {
        requests: vec![],
        replies: vec![],
    };
    for (tx, receipt) in j.transactions_with_receipts() {
        let opaque = opaque_evidence_slot(tx)?;
        for (index, raw) in tx.evidence.iter().enumerate() {
            if opaque == Some(index) {
                continue;
            }
            if let Some(bytes) = raw.as_bytes().strip_prefix(REQUEST) {
                let m: Metadata = serde_json::from_slice(bytes).map_err(|_| Error::Codec)?;
                if m.binding != binding
                    || m.reservation != tx.id.bytes()
                    || m.at != tx.at
                    || receipt.controls.is_some()
                    || result.requests.contains(&m)
                {
                    return Err(Error::Qualification);
                }
                result.requests.push(m);
            } else if let Some(bytes) = raw.as_bytes().strip_prefix(RESPONSE) {
                let a: Archive = serde_json::from_slice(bytes).map_err(|_| Error::Codec)?;
                if a.metadata.binding != binding
                    || a.metadata.evidence != tx.id.bytes()
                    || a.received_at < a.metadata.at
                    || a.received_at > tx.at
                    || a.body.len() > demo::MAX_BODY
                    || !result.requests.contains(&a.metadata)
                    || result.replies.iter().any(|old| old.metadata == a.metadata)
                {
                    return Err(Error::Qualification);
                }
                result.replies.push(a);
            }
        }
    }
    Ok(result)
}

impl Controller {
    fn setup_binding(&self, g: &Gateway, p: &demo::Policy) -> Result<[u8; 32], Error> {
        let mut h = Sha256::new();
        h.update(b"CINDER-NATIVE-SETUP-POLICY-1\0");
        h.update(p.release_commitment(self, g)?);
        Ok(h.finalize().into())
    }
    /// At most the existing explicit demo policy's GET budget/lifetime, in fixed
    /// Settings -> Loan -> Account rounds. Lost replies count, including restart.
    /// Settings are never mutated, and freshness does not imply completeness.
    pub fn prepare_setup_poll<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        g: &Gateway,
        p: &demo::Policy,
        input: Poll,
    ) -> Result<Step, Error> {
        let binding = self.setup_binding(g, p)?;
        let history = self.history(j, input.at)?;
        if !history.bound || j.state()?.frozen() {
            return Err(Error::Qualification);
        }
        let s = scan(j, binding)?;
        if self.ready(&history, input.at) {
            return Ok(Step::Qualified);
        }
        if let Some(ref demo) = history.demo_setup
            && demo.policy == p.release_commitment(self, g)?
            && p.initial_setup.is_some()
            && self.demo_setup_current(&history, demo, input.at, false)
            && (input.at < demo.valid_until
                || history.plans.iter().any(|plan| {
                    plan.rail == Rail::Deposit && plan.demo_setup.as_deref() == Some(demo)
                }))
        {
            return Ok(Step::DemoQualified);
        }
        if let Some(observed) = observation(self, &s, p, input.at)? {
            return Ok(Step::Observed(Box::new(observed)));
        }
        if s.requests.len() >= usize::from(p.maximum_reads)
            || s.requests
                .first()
                .is_some_and(|m| input.at.saturating_sub(m.at) >= p.lifetime_ms)
        {
            return Ok(Step::Exhausted);
        }
        if let Some(last) = s.requests.last() {
            let base = s
                .replies
                .iter()
                .find(|a| a.metadata == *last)
                .map_or(last.at, |a| a.received_at);
            if input.at < base.checked_add(p.interval_ms).ok_or(Error::Limit)? {
                return Ok(Step::Waiting);
            }
        }
        if !g.read_available(j, input.at, false)? {
            return Ok(Step::Waiting);
        }
        let endpoint =
            [Endpoint::Settings, Endpoint::Loan, Endpoint::Account][s.requests.len() % 3];
        let (request, complete) = reads::prepare_setup(
            j,
            g,
            Metadata {
                binding,
                endpoint,
                reservation: input.reservation.bytes(),
                evidence: input.evidence.bytes(),
                at: input.at,
            },
        )?;
        Ok(Step::Request(request, Box::new(complete)))
    }
    /// Feed retained source observations into the controller with complete=false.
    /// Separately retain the approved initial demo assumption when opted in.
    /// Missing cache, stale/malformed data and noncoherent rounds stay pending.
    /// Replaying the same observation does not append a second controller record.
    pub fn observe_setup_reads<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        g: &Gateway,
        p: &demo::Policy,
        id: CommitId,
        at: u64,
    ) -> Result<bool, Error> {
        let binding = self.setup_binding(g, p)?;
        let history = self.history(j, at)?;
        if !history.bound {
            return Err(Error::Qualification);
        }
        let scan = scan(j, binding)?;
        let Some(observed) = observation(self, &scan, p, at)? else {
            return Ok(false);
        };
        let qualify = p.initial_setup.is_some()
            && history.demo_setup.is_none()
            && history.plans.is_empty()
            && observed.idle
            && observed.compatible_margin
            && observed.lending_disabled
            && observed.borrowed == 0
            && observed.interest == 0
            && self.demo_idle(j)?;
        let setup = observed.setup;
        if !qualify && history.setup.as_ref() == Some(&setup)
            || history
                .setup
                .as_ref()
                .is_some_and(|old| old.observed_at > setup.observed_at)
        {
            return Ok(false);
        }
        if qualify {
            let reads = scan.requests[scan.requests.len() - 3..]
                .iter()
                .map(|m| m.evidence)
                .collect::<Vec<_>>()
                .try_into()
                .map_err(|_| Error::Qualification)?;
            let qualified = Qualification {
                policy: p.release_commitment(self, g)?,
                observed_at: setup.observed_at,
                valid_until: scan
                    .requests
                    .first()
                    .ok_or(Error::Qualification)?
                    .at
                    .checked_add(p.lifetime_ms)
                    .ok_or(Error::Limit)?
                    .min(
                        setup
                            .observed_at
                            .checked_add(self.route.setup_max_age)
                            .ok_or(Error::Limit)?,
                    ),
                reads,
            };
            let mut tx = self.tx(j, id, at, Record::Setup(setup))?;
            tx.evidence
                .push(self.evidence(Record::DemoSetup(qualified))?);
            tx.controls
                .push(Control::Funds(funds::Action::NativeCreditReady(false)));
            checked(j.commit(tx)?)?;
        } else {
            self.observe_setup(j, id, at, setup)?;
        }
        Ok(true)
    }
    fn demo_idle<B: Backend, P: Protection>(&self, j: &Journal<B, P>) -> Result<bool, Error> {
        let s = j.state()?;
        Ok(!s.frozen()
            && s.recovery_epoch().is_none()
            && s.unresolved_raw() == 0
            && s.ledger().venue().cash().atoms() == 0
            && s.ledger().venue().funding().atoms() == 0
            && s.ledger()
                .venue()
                .positions()
                .iter()
                .all(|p| p.quantity().lots() == 0 && p.basis().atoms() == 0)
            && s.orders().iter().all(|o| o.complete() && !o.faulted)
            && s.funds().iter().all(|o| !o.faulted && o.demo.is_none())
            && s.attempts().iter().all(|a| !a.possibly_exposed))
    }
    pub(super) fn demo_setup_current(
        &self,
        h: &History,
        demo: &Qualification,
        at: u64,
        fresh: bool,
    ) -> bool {
        let grid = Grid {
            places: self.route.decimals,
            step: 1,
        };
        h.bound
            && h.demo_setup.as_ref() == Some(demo)
            && demo.observed_at <= at
            && (!fresh || at < demo.valid_until)
            && h.setup.as_ref().is_some_and(|s| {
                !s.complete
                    && s.account == self.route.broker
                    && s.observed_at == demo.observed_at
                    && s.lending_disabled
                    && grid.parse(&s.borrowed) == Ok(0)
                    && grid.parse(&s.interest) == Ok(0)
            })
    }
    pub(super) fn demo_plan_ready(
        &self,
        h: &History,
        p: &demo::Policy,
        binding: [u8; 32],
        attempt: AttemptKey,
        at: u64,
    ) -> Result<bool, Error> {
        let Some(plan) = h.plans.iter().find(|p| p.attempt == attempt.encode()) else {
            return Ok(false);
        };
        Ok(if let Some(ref demo) = plan.demo_setup {
            p.initial_setup.is_some()
                && demo.policy == binding
                && self.demo_setup_current(h, demo, at, false)
                && plan.at >= demo.observed_at
                && plan.at < demo.valid_until
        } else {
            self.ready(h, at)
        })
    }
    /// Readiness for a new internally authorized initial ingress intent. This
    /// returns no approval, capability or strong native-readiness certificate.
    pub fn initial_demo_ingress_ready<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        g: &Gateway,
        p: &demo::Policy,
        at: u64,
    ) -> Result<bool, Error> {
        let binding = p.release_commitment(self, g)?;
        let h = self.history(j, at)?;
        let Some(demo) = h.demo_setup.as_ref() else {
            return Ok(false);
        };
        let s = j.state()?;
        Ok(p.initial_setup.is_some()
            && demo.policy == binding
            && self.demo_setup_current(&h, demo, at, true)
            && !s.frozen()
            && s.recovery_epoch().is_none()
            && s.unresolved_raw() == 0
            && s.orders().iter().all(|o| o.complete() && !o.faulted)
            && s.funds()
                .iter()
                .all(|o| o.terminal && !o.faulted && o.demo.is_none())
            && s.ledger().venue().cash().atoms() == 0
            && s.ledger().venue().funding().atoms() == 0
            && s.ledger()
                .venue()
                .positions()
                .iter()
                .all(|p| p.quantity().lots() == 0 && p.basis().atoms() == 0)
            && h.plans.iter().all(|plan| plan.rail == Rail::Release))
    }
    fn demo_ingress_setup<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        g: &Gateway,
        p: &demo::Policy,
        dispatch: Dispatch,
    ) -> Result<(Rail, Qualification), Error> {
        let binding = p.release_commitment(self, g)?;
        let h = self.history(j, dispatch.at)?;
        let demo = h.demo_setup.as_ref().ok_or(Error::Qualification)?;
        // Unlike initial qualification, a settled release may now exist. Any
        // OTHER pending attempt prevents the next leg; it must be reconciled.
        let s = j.state()?;
        let o = s
            .funds()
            .iter()
            .find(|o| o.intent.request == dispatch.attempt.request)
            .ok_or(Error::Qualification)?;
        let rail = match (o.intent.source, o.intent.destination) {
            (Location::Vault, Destination::Location(Location::Broker)) => Rail::Release,
            (Location::Broker, Destination::Location(Location::Venue)) => Rail::Deposit,
            _ => return Err(Error::Qualification),
        };
        if p.initial_setup.is_none()
            || demo.policy != binding
            || !self.demo_setup_current(&h, demo, dispatch.at, true)
            || j.transaction(dispatch.commit).is_some()
            || s.frozen()
            || s.recovery_epoch().is_some()
            || s.unresolved_raw() != 0
            || s.orders().iter().any(|o| !o.complete() || o.faulted)
            || s.ledger().venue().cash().atoms() != 0
            || s.ledger().venue().funding().atoms() != 0
            || s.ledger()
                .venue()
                .positions()
                .iter()
                .any(|p| p.quantity().lots() != 0 || p.basis().atoms() != 0)
            || s.funds().iter().any(|other| {
                other.faulted
                    || other.demo.is_some()
                    || other.intent.request != o.intent.request
                        && !other.terminal
                        && other.attempt.is_some()
            })
            || h.plans.iter().any(|plan| {
                plan.rail == Rail::Deposit
                    || rail == Rail::Release && plan.rail == Rail::Release
                    || !matches!(plan.rail, Rail::Release | Rail::Deposit)
            })
        {
            return Err(Error::Qualification);
        }
        Ok((rail, demo.clone()))
    }
    /// Prepare one accepted initial demo ingress intent. All ordinary journal
    /// authority/risk/liquidity checks still apply; no readiness flag is raised.
    pub fn prepare_demo_ingress<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        g: &Gateway,
        p: &demo::Policy,
        dispatch: Dispatch,
    ) -> Result<Rail, Error> {
        self.demo_ingress_setup(j, g, p, dispatch)?;
        self.prepare_ingress_inner(j, dispatch)
    }
    /// Validate an EXISTING accepted original before chain I/O. This creates no
    /// approval, intent, attempt, signature or capability; exposure rechecks it.
    pub fn validate_demo_ingress<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        g: &Gateway,
        p: &demo::Policy,
        dispatch: Dispatch,
    ) -> Result<Rail, Error> {
        let (rail, demo) = self.demo_ingress_setup(j, g, p, dispatch)?;
        let state = j.state()?;
        let attempt = state
            .attempts()
            .iter()
            .find(|a| a.key == dispatch.attempt)
            .ok_or(Error::Qualification)?;
        if state.authority_epoch(dispatch.attempt.request.account) != Some(attempt.authority_epoch)
        {
            return Err(Error::Qualification);
        }
        // Check the same prepared movement/expiry/amount rules without retaining
        // a plan or claiming any actual chain sequence/slot. Real counters and
        // current financial authority are checked again at capability exposure.
        self.plan_inner(
            j,
            dispatch,
            rail,
            Counters {
                sequence: 0,
                paid: 0,
                recipient_tokens: [0; 32],
                expires_at_slot: 1,
            },
            Some(demo),
        )?;
        Ok(rail)
    }
    /// Persist/expose only the first bounded Release or Deposit, explicitly
    /// tagging its setup assurance. Not an AWS/live activation or send approval.
    pub fn expose_demo_ingress<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        g: &Gateway,
        p: &demo::Policy,
        dispatch: Dispatch,
        counters: Counters,
    ) -> Result<ChainAction, Error> {
        let (rail, demo) = self.demo_ingress_setup(j, g, p, dispatch)?;
        let plan = self.plan_inner(j, dispatch, rail, counters, Some(demo))?;
        self.expose_chain_plan(j, dispatch, plan)
    }
}

#[derive(Deserialize)]
struct Envelope<T> {
    success: bool,
    data: T,
    error: Option<String>,
    code: Option<i64>,
}
#[derive(Deserialize)]
struct Account {
    balance: String,
    account_equity: String,
    available_to_spend: String,
    available_to_withdraw: String,
    pending_balance: String,
    pending_interest: String,
    total_margin_used: String,
    cross_mmr: String,
    spot_collateral: String,
    spot_market_value: String,
    cross_account_equity: Option<String>,
    positions_count: u64,
    orders_count: u64,
    stop_orders_count: u64,
    spot_balances: Vec<serde_json::Value>,
    updated_at: u64,
    error: Option<String>,
    code: Option<i64>,
}
/// Full current account shape, not the historical diagnostic's `equity` alias.
fn idle_account(c: &Controller, body: &[u8], received_at: u64) -> Result<bool, Error> {
    let a: Envelope<Account> = serde_json::from_slice(body).map_err(|_| Error::Codec)?;
    if !a.success
        || a.error.is_some()
        || a.code.is_some()
        || a.data.updated_at == 0
        || a.data.updated_at > received_at
        || a.data.error.is_some()
        || a.data.code.is_some()
    {
        return Err(Error::Qualification);
    }
    let a = a.data;
    let values = [
        &a.balance,
        &a.account_equity,
        &a.available_to_spend,
        &a.available_to_withdraw,
        &a.pending_balance,
        &a.pending_interest,
        &a.total_margin_used,
        &a.cross_mmr,
        &a.spot_collateral,
        &a.spot_market_value,
    ];
    let mut empty = true;
    for value in values {
        empty &= c.profile.quote(value)?.atoms() == 0;
    }
    if let Some(value) = a.cross_account_equity {
        empty &= c.profile.quote(&value)?.atoms() == 0;
    }
    Ok(empty
        && a.positions_count == 0
        && a.orders_count == 0
        && a.stop_orders_count == 0
        && a.spot_balances.is_empty())
}
#[derive(Deserialize)]
struct AuxiliarySettings {
    spot_settings: Vec<serde_json::Value>,
    error: Option<String>,
    code: Option<i64>,
}
#[derive(Deserialize)]
struct AuxiliaryLoan {
    spot_balances: Vec<serde_json::Value>,
    error: Option<String>,
    code: Option<i64>,
}
fn empty_auxiliary(settings: &[u8], loan: &[u8]) -> Result<bool, Error> {
    let s: Envelope<AuxiliarySettings> =
        serde_json::from_slice(settings).map_err(|_| Error::Codec)?;
    let l: Envelope<AuxiliaryLoan> = serde_json::from_slice(loan).map_err(|_| Error::Codec)?;
    if s.data.error.is_some()
        || s.data.code.is_some()
        || l.data.error.is_some()
        || l.data.code.is_some()
    {
        return Err(Error::Qualification);
    }
    Ok(s.data.spot_settings.is_empty() && l.data.spot_balances.is_empty())
}
fn observation(
    c: &Controller,
    s: &Scan,
    p: &demo::Policy,
    at: u64,
) -> Result<Option<Observation>, Error> {
    // Only the latest complete round; do not reuse older good data after a
    // missing reply or a newer contradictory read. Receive age, not an idle
    // account's old native updated_at, bounds this observed snapshot.
    if s.requests.is_empty()
        || !s.requests.len().is_multiple_of(3)
        || s.requests.len() > usize::from(p.maximum_reads)
        || s.requests
            .first()
            .is_some_and(|m| at.saturating_sub(m.at) >= p.lifetime_ms)
    {
        return Ok(None);
    }
    let mut round = vec![];
    for m in &s.requests[s.requests.len() - 3..] {
        let Some(a) = s.replies.iter().find(|a| a.metadata == *m) else {
            return Ok(None);
        };
        if a.status != 200 || a.received_at > at || at - a.received_at > c.route.setup_max_age {
            return Ok(None);
        }
        round.push(a);
    }
    if round.iter().map(|a| a.metadata.endpoint).ne([
        Endpoint::Settings,
        Endpoint::Loan,
        Endpoint::Account,
    ]) || round
        .windows(2)
        .any(|a| a[0].received_at > a[1].metadata.at)
    {
        return Err(Error::Qualification);
    }
    let Ok(parsed) = evidence::setup(&c.profile, &round[0].body, &round[1].body) else {
        return Ok(None);
    };
    if parsed.loan_at > round[1].received_at {
        return Ok(None);
    }
    let Ok(empty_auxiliary) = empty_auxiliary(&round[0].body, &round[1].body) else {
        return Ok(None);
    };
    let Ok(idle) = idle_account(c, &round[2].body, round[2].received_at) else {
        return Ok(None);
    };
    // Retain adverse settings/debt too. Compatibility/idle status cannot turn
    // this into complete=true, even when every observed predicate passes.
    let setup = Setup {
        account: c.route.broker,
        observed_at: round[0].received_at,
        lending_disabled: parsed.lending_disabled,
        borrowed: Grid {
            places: c.route.decimals,
            step: 1,
        }
        .format(parsed.borrowed)?,
        interest: Grid {
            places: c.route.decimals,
            step: 1,
        }
        .format(parsed.interest)?,
        complete: false,
    };
    Ok(Some(Observation {
        setup,
        idle,
        lending_disabled: parsed.lending_disabled,
        compatible_margin: parsed.compatible_margin && empty_auxiliary,
        borrowed: parsed.borrowed,
        interest: parsed.interest,
    }))
}
