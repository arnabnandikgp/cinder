//! Authenticated private application boundary over the one protected journal.
//! No listener, HTTP client, wallet loader, native signing or plaintext fallback.
//! The confidential-channel and admission ports are trusted runtime interfaces,
//! not proof of attestation. P19/P20 qualify their actual implementations.
pub mod reads;
pub mod wire;
use cinder_journal::{Backend, Journal, Protection, funds, model::*, orders, risk};
use cinder_kernel::{
    amounts::*,
    identity::*,
    ledger::{funds::Destination, *},
};
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fmt};
use wire::{Command, Request, Writer};

const INIT: &[u8] = b"CINDER-API-CONTRACT-1\0";
const RECORD: &[u8] = b"CINDER-API-RECORD-1\0";

/// Redacted application failures. Never return native bodies, account identifiers,
/// signatures or a more informative error for another customer's operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Malformed/unsupported bounded input.
    Invalid,
    /// Caller/session/domain/epoch/grant is not currently authorized.
    Unauthorized,
    /// Authenticated operation ID does not exist in this private account.
    NotFound,
    /// Authenticated stable ID was already used for different economics.
    Conflict,
    /// Durability/freshness/qualification failed; outcome may be unknown.
    Unavailable,
}
impl Error {
    /// Redacted failure frame. This must travel through the confidential channel,
    /// just like success replies; it is not an outer HTTP/logging payload.
    pub fn encode_private(self) -> Result<PrivateBytes, Self> {
        let mut w = Writer::new(b"CINDER-API-REPLY\0\x00\x01");
        w.byte(2);
        w.byte(match self {
            Self::Invalid => 0,
            Self::Unauthorized => 1,
            Self::NotFound => 2,
            Self::Conflict => 3,
            Self::Unavailable => 4,
        });
        PrivateBytes::new(w.0.to_vec()).map_err(|_| Self::Unavailable)
    }
}
/// Governed onboarding binding; match the P16 beneficiary route exactly.
#[derive(Clone, PartialEq, Eq)]
pub struct OwnerBinding {
    /// Opaque configured private customer identity.
    pub account: AccountId,
    /// Owner wallet Ed25519 key.
    pub wallet: [u8; 32],
    /// Configured owner-controlled quote token account; not a caller-supplied address.
    pub tokens: [u8; 32],
}
/// Immutable API contract. Persisted fingerprint prevents silent owner/config reset.
#[derive(Clone, PartialEq, Eq)]
pub struct Contract {
    /// Every configured customer exactly once; runtime onboarding is not added here.
    pub owners: Vec<OwnerBinding>,
    /// Maximum auth/session duration in injected milliseconds, never an ambient clock.
    pub maximum_auth_lifetime: u64,
    /// Maximum grant duration, independent of short request/session lifetime.
    pub maximum_grant_lifetime: u64,
}
/// Confidential endpoint supplied inside the approved runtime. A binding must be
/// obtained from the established channel, never a request header or host boolean.
/// Implementing this trait alone does not qualify transport security.
pub trait ConfidentialChannel {
    /// Exact independently selected service domain.
    fn domain(&self) -> Domain;
    /// Fresh transcript/challenge/session-key binding.
    fn binding(&self) -> [u8; 32];
    /// Qualified session expiry in the injected clock domain.
    fn expires_at(&self) -> u64;
}
/// Trusted server-side hold computation at this exact immutable state. No client
/// parameter can supply reservations/policy/marks or a raw journal control.
/// The same journal checks the resulting full candidate through P09 admission.
pub trait Admission {
    /// Explicit policy-qualified customer and native location commitments.
    fn order_holds(
        &self,
        state: &State,
        intent: &orders::Intent,
    ) -> Result<Vec<Reservation>, Error>;
}
/// Current durable outcome, not a transport acknowledgement or guaranteed fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Controls rejected atomically; stable ID is retained and cannot retrade.
    Rejected,
    /// Durably accepted, with no possibly exposed attempt.
    Accepted,
    /// Original action was possibly exposed; no fill inferred.
    Dispatched,
    /// Venue acknowledges only, with no fill inferred.
    Acknowledged,
    /// Some actual fills/beneficiary payments arrived; not necessarily complete.
    Partial,
    /// Complete qualified history/settlement, or a purely administrative mutation.
    Complete,
    /// Unknown/contradictory evidence; do not automatically retry/release.
    Unknown,
}
/// Account-private receipt; deliberately no global journal position or native ID.
#[derive(Clone, PartialEq, Eq)]
pub struct Receipt {
    /// Stable private operation identity.
    pub id: RequestId,
    /// Exact economic digest, unchanged by reconnect/auth epoch rotation.
    pub digest: [u8; 32],
    /// Accepted economic policy revision.
    pub policy: PolicyVersion,
    /// Actual current lifecycle outcome.
    pub outcome: Outcome,
    /// Actual signed filled lots, zero for non-order operations.
    pub filled: i64,
    /// Actual beneficiary payments, not a source debit/HTTP ACK.
    pub paid: i128,
    /// Actual customer rail fees (not an order fee prediction).
    pub rail_fees: i128,
    /// Total bounded order/payout fee cap, not an already charged fee.
    pub fee_cap: i128,
    /// Whether any original action may have escaped; completion does not clear this.
    pub possibly_exposed: bool,
    /// Caller explicitly consented to a partial payout dispatch.
    pub allow_partial: bool,
}
/// Exact private position, not a rounded entry-price approximation.
#[derive(Clone, PartialEq, Eq)]
pub struct PositionView {
    /// Market/precision identity from trusted configuration.
    pub market: MarketUnit,
    /// Signed lots.
    pub lots: i64,
    /// Signed remaining quote-atom basis.
    pub basis: i128,
}
/// Snapshot derived only from this customer's authoritative book and operations.
#[derive(Clone, PartialEq, Eq)]
pub struct View {
    /// Owner's current authorization epoch; revoke/reconnect can recover it.
    pub epoch: u64,
    /// Settled cash, not withdrawable balance or equity.
    pub cash: i128,
    /// Recognized unsettled funding, not spendable cash.
    pub funding: i128,
    /// Explicit shared customer holds, not total pending-inclusive margin.
    pub held: i128,
    /// Customer positions only; no pooled/house/suspense book.
    pub positions: Vec<PositionView>,
    /// This customer's retained API mutations, including rejected operation IDs.
    pub operations: Vec<RequestId>,
}
/// Returned only inside the same established confidential channel.
#[derive(Clone, PartialEq, Eq)]
pub enum Response {
    /// Bounded authorized projection of retained state/evidence, not a spend permit.
    Read(reads::Page),
    /// Fresh authenticated snapshot; polling/reconnect always obtains a full snapshot.
    View(View),
    /// Fresh current operation state, including on an exact retry.
    Receipt(Receipt),
}
impl Response {
    /// Canonical confidential reply. No signature/solvency claim is manufactured.
    pub fn encode(&self) -> Result<PrivateBytes, Error> {
        let mut w = Writer::new(b"CINDER-API-REPLY\0\x00\x01");
        match self {
            Self::Read(p) => {
                w.byte(3);
                p.encode(&mut w)?;
            }
            Self::View(v) => {
                w.byte(0);
                w.u64(v.epoch);
                w.i128(v.cash);
                w.i128(v.funding);
                w.i128(v.held);
                w.u64(v.positions.len() as u64);
                for p in &v.positions {
                    w.raw(&p.market.market.bytes());
                    w.raw(&p.market.precision.get().to_be_bytes());
                    w.raw(&p.lots.to_be_bytes());
                    w.i128(p.basis);
                }
                w.u64(v.operations.len() as u64);
                for id in &v.operations {
                    w.raw(&id.bytes());
                }
            }
            Self::Receipt(r) => {
                w.byte(1);
                w.raw(&r.id.bytes());
                w.raw(&r.digest);
                w.raw(&r.policy.get().to_be_bytes());
                w.byte(match r.outcome {
                    Outcome::Rejected => 0,
                    Outcome::Accepted => 1,
                    Outcome::Dispatched => 2,
                    Outcome::Acknowledged => 3,
                    Outcome::Partial => 4,
                    Outcome::Complete => 5,
                    Outcome::Unknown => 6,
                });
                w.raw(&r.filled.to_be_bytes());
                w.i128(r.paid);
                w.i128(r.rail_fees);
                w.i128(r.fee_cap);
                w.byte(u8::from(r.possibly_exposed));
                w.byte(u8::from(r.allow_partial));
            }
        }
        PrivateBytes::new(w.0.to_vec()).map_err(|_| Error::Unavailable)
    }
}
macro_rules! private_debug {
    ($($ty:ty),*) => { $(impl fmt::Debug for $ty {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("[PRIVATE API]") }
    })* };
}
private_debug!(
    OwnerBinding,
    Contract,
    Receipt,
    PositionView,
    View,
    Response
);

struct Record {
    request: Request,
    accepted: bool,
    at: u64,
}
/// Runtime handler has no independently authoritative balances or replay database.
pub struct Service<A> {
    contract: Contract,
    admission: A,
}
impl<A: Admission> Service<A> {
    /// Construct with explicit governed owner bindings and server-side admission port.
    pub fn new(contract: Contract, admission: A) -> Result<Self, Error> {
        if contract.owners.is_empty()
            || contract.owners.len() > 1024
            || contract.maximum_auth_lifetime == 0
            || contract.maximum_grant_lifetime == 0
        {
            return Err(Error::Invalid);
        }
        for (i, b) in contract.owners.iter().enumerate() {
            let key = VerifyingKey::from_bytes(&b.wallet).map_err(|_| Error::Invalid)?;
            if key.is_weak()
                || b.tokens == [0; 32]
                || contract.owners[..i]
                    .iter()
                    .any(|old| old.account == b.account || old.tokens == b.tokens)
            {
                return Err(Error::Invalid);
            }
        }
        Ok(Self {
            contract,
            admission,
        })
    }
    fn fingerprint(&self, domain: Domain, policy: PolicyVersion) -> [u8; 32] {
        let mut w = Writer::new(INIT);
        w.raw(&domain.network.bytes());
        w.raw(&domain.deployment.bytes());
        w.raw(&policy.get().to_be_bytes());
        w.u64(self.contract.maximum_auth_lifetime);
        w.u64(self.contract.maximum_grant_lifetime);
        w.u64(self.contract.owners.len() as u64);
        for b in &self.contract.owners {
            w.raw(&b.account.bytes());
            w.raw(&b.wallet);
            w.raw(&b.tokens);
        }
        Sha256::digest(&*w.0).into()
    }
    /// Commitment to this loaded API contract, using the journal's exact domain
    /// and policy. This is a release-manifest component, not a solvency proof.
    pub fn release_commitment(&self, config: &Config) -> Result<[u8; 32], Error> {
        if config.customers.len() != self.contract.owners.len()
            || self
                .contract
                .owners
                .iter()
                .any(|b| !config.customers.contains(&b.account))
        {
            return Err(Error::Invalid);
        }
        Ok(self.fingerprint(config.domain, config.policy))
    }
    /// Private-runtime configuration access for matching the loaded custody
    /// beneficiaries. This is not a customer API or public owner directory.
    pub fn owner_bindings(&self) -> &[OwnerBinding] {
        &self.contract.owners
    }
    /// One explicit governed installation, committed with initial authority epochs.
    /// A changed contract needs an explicit migration; it cannot overwrite history.
    pub fn initialize<B: Backend, P: Protection>(
        &self,
        journal: &mut Journal<B, P>,
        now: u64,
    ) -> Result<(), Error> {
        journal.verified_state().map_err(|_| Error::Unavailable)?;
        let config = journal.configuration();
        let fingerprint = self.release_commitment(config)?;
        let marker = [INIT, &fingerprint].concat();
        for tx in journal.transactions() {
            for e in &tx.evidence {
                if e.as_bytes().starts_with(INIT) {
                    if e.as_bytes() != marker
                        || journal
                            .transaction_receipt(tx.id)
                            .is_none_or(|r| r.controls.is_some())
                    {
                        return Err(Error::Unavailable);
                    }
                    return Ok(());
                }
            }
        }
        let mut controls = Vec::new();
        for b in &self.contract.owners {
            let epoch = journal
                .state()
                .map_err(|_| Error::Unavailable)?
                .authority_epoch(b.account)
                .unwrap_or(0)
                .checked_add(1)
                .ok_or(Error::Unavailable)?;
            controls.push(Control::Order(orders::Action::AdvanceAuthority {
                account: b.account,
                epoch,
            }));
        }
        let mut tx = empty(
            journal,
            CommitId::new(Sha256::digest(&marker).into()).map_err(|_| Error::Invalid)?,
            now,
        );
        tx.evidence
            .push(PrivateBytes::new(marker).map_err(|_| Error::Invalid)?);
        tx.controls = controls;
        let committed = journal.commit(tx).map_err(|_| Error::Unavailable)?;
        if committed.receipt.controls.is_some() {
            return Err(Error::Unavailable);
        }
        Ok(())
    }
    fn records<B: Backend, P: Protection>(
        &self,
        journal: &Journal<B, P>,
    ) -> Result<Vec<Record>, Error> {
        let fingerprint = self.fingerprint(
            journal.configuration().domain,
            journal.configuration().policy,
        );
        let mut installed = false;
        let mut records = Vec::new();
        let mut seen = BTreeSet::new();
        for tx in journal.transactions() {
            let accepted = journal
                .transaction_receipt(tx.id)
                .ok_or(Error::Unavailable)?
                .controls
                .is_none();
            for e in &tx.evidence {
                if e.as_bytes().starts_with(INIT) {
                    if installed || e.as_bytes() != [INIT, &fingerprint].concat() || !accepted {
                        return Err(Error::Unavailable);
                    }
                    installed = true;
                } else if let Some(tail) = e.as_bytes().strip_prefix(RECORD) {
                    if !installed || tail.get(..32) != Some(fingerprint.as_slice()) {
                        return Err(Error::Unavailable);
                    }
                    let request = Request::decode(tail.get(32..).ok_or(Error::Unavailable)?)
                        .map_err(|_| Error::Unavailable)?;
                    if !seen.insert((request.account.bytes(), request.id.bytes())) {
                        return Err(Error::Unavailable);
                    }
                    records.push(Record {
                        request,
                        accepted,
                        at: tx.at,
                    });
                }
            }
        }
        if !installed {
            return Err(Error::Unavailable);
        }
        Ok(records)
    }
    /// Verify identity, current epoch and grant before any operation dedupe/lookup.
    /// Receive decrypted bytes only from the established trusted channel port.
    pub fn handle<B: Backend, P: Protection>(
        &self,
        journal: &mut Journal<B, P>,
        channel: &impl ConfidentialChannel,
        bytes: &PrivateBytes,
        now: u64,
    ) -> Result<Response, Error> {
        let req = Request::decode(bytes.as_bytes())?;
        let owner = self
            .contract
            .owners
            .iter()
            .find(|b| b.account == req.account)
            .ok_or(Error::Unauthorized)?;
        if channel.domain() != req.domain
            || req.domain != journal.configuration().domain
            || req.policy != journal.configuration().policy
            || channel.binding() == [0; 32]
            || req.session != channel.binding()
            || now >= channel.expires_at()
            || now >= req.expires_at
            || req.expires_at > channel.expires_at()
            || req.expires_at.saturating_sub(now) > self.contract.maximum_auth_lifetime
        {
            return Err(Error::Unauthorized);
        }
        let key = VerifyingKey::from_bytes(&req.signer).map_err(|_| Error::Unauthorized)?;
        key.verify_strict(&req.message(), &Signature::from_bytes(&req.signature))
            .map_err(|_| Error::Unauthorized)?;
        journal.verified_state().map_err(|_| Error::Unavailable)?;
        let state = journal.state().map_err(|_| Error::Unavailable)?;
        if now < state.logical_time() {
            return Err(Error::Unavailable);
        }
        if req.epoch == 0 || state.authority_epoch(req.account) != Some(req.epoch) {
            return Err(Error::Unauthorized);
        }
        let records = self.records(journal)?;
        let is_owner = req.signer == owner.wallet;
        if !is_owner {
            self.authorize_agent(&req, &records, now)?;
        }
        match &req.command {
            Command::Read(query) => {
                return self
                    .read(journal, &req, &records, *query, is_owner, now)
                    .map(Response::Read);
            }
            Command::View => {
                let book = state
                    .ledger()
                    .book(Owner::Customer(req.account))
                    .map_err(|_| Error::Unavailable)?;
                return Ok(Response::View(View {
                    epoch: req.epoch,
                    cash: book.cash().atoms(),
                    funding: book.funding().atoms(),
                    held: state
                        .reserved(Resource::Customer(req.account))
                        .map_err(|_| Error::Unavailable)?
                        .atoms(),
                    positions: book
                        .positions()
                        .iter()
                        .map(|p| PositionView {
                            market: p.quantity().unit(),
                            lots: p.quantity().lots(),
                            basis: p.basis().atoms(),
                        })
                        .collect(),
                    operations: records
                        .iter()
                        .filter(|r| r.request.account == req.account)
                        .map(|r| r.request.id)
                        .collect(),
                }));
            }
            Command::Operation(id) => {
                let record = records
                    .iter()
                    .find(|r| r.request.account == req.account && r.request.id == *id)
                    .ok_or(Error::NotFound)?;
                return self.receipt(state, record).map(Response::Receipt);
            }
            _ => {}
        }
        // No private ID comparison or saved outcome is performed before authorization.
        if let Some(old) = records
            .iter()
            .find(|r| r.request.account == req.account && r.request.id == req.id)
        {
            if !is_owner && old.request.signer != req.signer {
                return Err(Error::Unauthorized);
            }
            if old.request.digest() != req.digest() {
                return Err(Error::Conflict);
            }
            return self.receipt(state, old).map(Response::Receipt);
        }
        let controls = self.controls(journal, &req, owner, &records, is_owner, now)?;
        let mut tx = empty(journal, commit_id(&req)?, now);
        let fingerprint = self.fingerprint(req.domain, req.policy);
        tx.evidence.push(
            PrivateBytes::new([RECORD, &fingerprint, req.encode()?.as_bytes()].concat())
                .map_err(|_| Error::Invalid)?,
        );
        tx.controls = controls;
        let committed = journal.commit(tx).map_err(|_| Error::Unavailable)?;
        let record = Record {
            request: req,
            accepted: committed.receipt.controls.is_none(),
            at: now,
        };
        self.receipt(journal.state().map_err(|_| Error::Unavailable)?, &record)
            .map(Response::Receipt)
    }
    fn authorize_agent(&self, req: &Request, records: &[Record], now: u64) -> Result<(), Error> {
        let grant = records
            .iter()
            .find_map(|r| match &r.request.command {
                Command::Grant(g)
                    if r.accepted
                        && r.request.account == req.account
                        && r.request.epoch == req.epoch
                        && g.key == req.signer =>
                {
                    Some(g)
                }
                _ => None,
            })
            .ok_or(Error::Unauthorized)?;
        if now >= grant.expires_at {
            return Err(Error::Unauthorized);
        }
        match &req.command {
            Command::View | Command::Operation(_) | Command::Read(_)
                if grant.methods & wire::READ != 0 =>
            {
                Ok(())
            }
            Command::Order {
                market,
                lots,
                fee,
                good_until,
                ..
            } if grant.methods & wire::TRADE != 0
                && *market == grant.market
                && lots.unsigned_abs() <= grant.maximum_lots
                && *fee >= 0
                && *fee <= grant.maximum_fee
                && *good_until <= grant.expires_at =>
            {
                Ok(())
            }
            Command::Cancel { good_until, .. }
                if grant.methods & wire::CANCEL != 0 && *good_until <= grant.expires_at =>
            {
                Ok(())
            }
            _ => Err(Error::Unauthorized),
        }
    }
    fn controls<B: Backend, P: Protection>(
        &self,
        journal: &Journal<B, P>,
        req: &Request,
        owner: &OwnerBinding,
        records: &[Record],
        is_owner: bool,
        now: u64,
    ) -> Result<Vec<Control>, Error> {
        let state = journal.state().map_err(|_| Error::Unavailable)?;
        let request = RequestKey {
            domain: req.domain,
            account: req.account,
            request: req.id,
        };
        let unit = |id: MarketId| {
            journal
                .configuration()
                .markets
                .iter()
                .find(|m| m.unit().market == id)
                .map(|m| m.unit())
                .ok_or(Error::Invalid)
        };
        let deadline = |at: u64| {
            if at > now {
                Ok(at)
            } else {
                Err(Error::Invalid)
            }
        };
        let approval = |hash| orders::Approval {
            account: req.account,
            intent_hash: hash,
            authority_epoch: req.epoch,
        };
        Ok(vec![match &req.command {
            Command::Order {
                market,
                lots,
                minimum,
                maximum,
                fee,
                tif,
                reduce_only,
                good_until,
            } => {
                if !is_owner {
                    let g = records
                        .iter()
                        .find_map(|r| match &r.request.command {
                            Command::Grant(g)
                                if r.accepted
                                    && r.request.account == req.account
                                    && r.request.epoch == req.epoch
                                    && g.key == req.signer =>
                            {
                                Some(g)
                            }
                            _ => None,
                        })
                        .ok_or(Error::Unauthorized)?;
                    let used = records
                        .iter()
                        .filter(|r| {
                            r.accepted
                                && r.request.account == req.account
                                && r.request.epoch == req.epoch
                                && r.request.signer == req.signer
                                && matches!(r.request.command, Command::Order { .. })
                        })
                        .count() as u64;
                    if used >= g.maximum_orders {
                        return Err(Error::Unauthorized);
                    }
                }
                let market = unit(*market)?;
                if *lots == 0
                    || *fee < 0
                    || minimum > maximum
                    || fee.checked_mul(i128::from(lots.unsigned_abs())).is_none()
                {
                    return Err(Error::Invalid);
                }
                let intent = orders::Intent {
                    time_in_force: match tif {
                        0 => orders::TimeInForce::GoodTilCancelled,
                        1 => orders::TimeInForce::AddLiquidityOnly,
                        2 => orders::TimeInForce::ImmediateOrCancel,
                        _ => return Err(Error::Invalid),
                    },
                    request,
                    quantity: QuantityLots::new(market, *lots),
                    minimum: PriceTicks::new(market, *minimum).map_err(|_| Error::Invalid)?,
                    maximum: PriceTicks::new(market, *maximum).map_err(|_| Error::Invalid)?,
                    maximum_fee_per_lot: QuoteAtoms::new(market.quote, *fee),
                    reduce_only: *reduce_only,
                    policy: req.policy,
                    authority_epoch: req.epoch,
                    expires_at: deadline(*good_until)?,
                };
                // No legacy flat-cash fallback: P09 must actually be installed.
                if !state.has_joined_risk() {
                    return Err(Error::Unavailable);
                }
                let reservations = self.admission.order_holds(state, &intent)?;
                Control::Order(orders::Action::Accept {
                    approval: approval(intent.digest().map_err(|_| Error::Invalid)?),
                    intent: Box::new(intent),
                    reservations,
                })
            }
            Command::Cancel {
                target,
                attempt,
                good_until,
            } => {
                let target = RequestKey {
                    request: *target,
                    ..request
                };
                let old = records
                    .iter()
                    .find(|r| {
                        r.accepted
                            && r.request.account == req.account
                            && r.request.id == target.request
                            && matches!(r.request.command, Command::Order { .. })
                    })
                    .ok_or(Error::NotFound)?;
                let order = state
                    .orders()
                    .iter()
                    .find(|o| o.intent.request == target)
                    .ok_or(Error::NotFound)?;
                if !is_owner
                    && (old.request.signer != req.signer
                        || !matches!(&old.request.command,
                    Command::Order { market, .. } if records.iter().any(|r| r.accepted && r.request.account == req.account
                        && r.request.epoch == req.epoch && matches!(&r.request.command, Command::Grant(g) if g.key == req.signer && g.market == *market))))
                {
                    return Err(Error::Unauthorized);
                }
                if order.complete() {
                    return Err(Error::Invalid);
                }
                if state
                    .attempts()
                    .iter()
                    .any(|a| a.key.request == target && a.possibly_exposed)
                {
                    Control::Order(orders::Action::PrepareCancel {
                        attempt: AttemptKey {
                            request: target,
                            attempt: *attempt,
                        },
                        authority_epoch: req.epoch,
                        expires_at: deadline(*good_until)?,
                    })
                } else {
                    Control::Release(target)
                }
            }
            Command::Payout {
                net,
                maximum_fee,
                allow_partial,
                good_until,
            } if is_owner => {
                let quote = journal.configuration().quote;
                if *net <= 0 || *maximum_fee < 0 || net.checked_add(*maximum_fee).is_none() {
                    return Err(Error::Invalid);
                }
                let intent = funds::Intent {
                    request,
                    source: Location::Vault,
                    destination: Destination::Recipient(owner.tokens),
                    net: QuoteAtoms::new(quote, *net),
                    maximum_fee: QuoteAtoms::new(quote, *maximum_fee),
                    fee_payer: Owner::Customer(req.account),
                    allow_partial: *allow_partial,
                    policy: req.policy,
                    authority_epoch: req.epoch,
                    expires_at: deadline(*good_until)?,
                };
                Control::Funds(funds::Action::Accept {
                    approval: approval(intent.digest().map_err(|_| Error::Invalid)?),
                    intent: Box::new(intent),
                })
            }
            Command::Grant(g) if is_owner => {
                let key = VerifyingKey::from_bytes(&g.key).map_err(|_| Error::Invalid)?;
                if key.is_weak()
                    || g.key == owner.wallet
                    || g.methods == 0
                    || g.methods & !7 != 0
                    || g.maximum_lots == 0
                    || g.maximum_fee < 0
                    || g.maximum_orders == 0
                    || g.expires_at <= now
                    || g.expires_at.saturating_sub(now) > self.contract.maximum_grant_lifetime
                    || records.iter().any(|r| {
                        r.accepted
                            && r.request.account == req.account
                            && r.request.epoch == req.epoch
                            && matches!(&r.request.command, Command::Grant(old) if old.key == g.key)
                    })
                {
                    return Err(Error::Invalid);
                }
                unit(g.market)?;
                // The grant itself lives atomically in protected evidence, not another DB.
                return Ok(vec![]);
            }
            Command::Revoke if is_owner => Control::Order(orders::Action::AdvanceAuthority {
                account: req.account,
                epoch: req.epoch.checked_add(1).ok_or(Error::Invalid)?,
            }),
            Command::Leverage {
                market,
                leverage,
                good_until,
            } if is_owner => {
                let selection = risk::Selection {
                    request,
                    market: unit(*market)?,
                    leverage: *leverage,
                    policy: req.policy,
                    authority_epoch: req.epoch,
                    expires_at: deadline(*good_until)?,
                };
                Control::Risk(risk::Action::Select {
                    approval: approval(selection.digest().map_err(|_| Error::Invalid)?),
                    selection: Box::new(selection),
                })
            }
            _ => return Err(Error::Unauthorized),
        }])
    }
    fn receipt(&self, state: &State, record: &Record) -> Result<Receipt, Error> {
        let req = &record.request;
        let request = RequestKey {
            domain: req.domain,
            account: req.account,
            request: req.id,
        };
        let mut r = Receipt {
            id: req.id,
            digest: req.digest(),
            policy: req.policy,
            outcome: if record.accepted {
                Outcome::Complete
            } else {
                Outcome::Rejected
            },
            filled: 0,
            paid: 0,
            rail_fees: 0,
            fee_cap: 0,
            possibly_exposed: false,
            allow_partial: false,
        };
        if let Command::Order { lots, fee, .. } = req.command {
            r.fee_cap = fee
                .checked_mul(i128::from(lots.unsigned_abs()))
                .ok_or(Error::Unavailable)?;
            if record.accepted {
                let o = state
                    .orders()
                    .iter()
                    .find(|o| o.intent.request == request)
                    .ok_or(Error::Unavailable)?;
                r.possibly_exposed = state
                    .attempts()
                    .iter()
                    .any(|a| a.key.request == request && a.possibly_exposed);
                r.filled = o.filled.lots();
                r.outcome = if o.faulted || o.unknown {
                    Outcome::Unknown
                } else if o.complete() {
                    Outcome::Complete
                } else if r.filled != 0 {
                    Outcome::Partial
                } else if o.acknowledged {
                    Outcome::Acknowledged
                } else if r.possibly_exposed {
                    Outcome::Dispatched
                } else {
                    Outcome::Accepted
                };
            }
        }
        if let Command::Payout {
            maximum_fee,
            allow_partial,
            ..
        } = req.command
        {
            r.fee_cap = maximum_fee;
            r.allow_partial = allow_partial;
            if record.accepted {
                let o = state
                    .funds()
                    .iter()
                    .find(|o| o.intent.request == request)
                    .ok_or(Error::Unavailable)?;
                r.possibly_exposed = state
                    .attempts()
                    .iter()
                    .any(|a| a.key.request == request && a.possibly_exposed);
                for m in state
                    .ledger()
                    .movements()
                    .iter()
                    .filter(|m| m.mandate.attempt.request == request)
                {
                    r.paid = r
                        .paid
                        .checked_add(m.paid.atoms())
                        .ok_or(Error::Unavailable)?;
                    r.rail_fees = r
                        .rail_fees
                        .checked_add(m.customer_fees.atoms())
                        .ok_or(Error::Unavailable)?;
                }
                r.outcome = if o.faulted {
                    Outcome::Unknown
                } else if o.terminal {
                    Outcome::Complete
                } else if r.paid > 0 {
                    Outcome::Partial
                } else if r.possibly_exposed {
                    Outcome::Dispatched
                } else {
                    Outcome::Accepted
                };
            }
        }
        if let Command::Cancel {
            target, attempt, ..
        } = req.command
            && record.accepted
        {
            let target = RequestKey {
                request: target,
                ..request
            };
            let o = state
                .orders()
                .iter()
                .find(|o| o.intent.request == target)
                .ok_or(Error::Unavailable)?;
            r.filled = o.filled.lots();
            r.possibly_exposed = state.attempts().iter().any(|a| {
                a.key
                    == AttemptKey {
                        request: target,
                        attempt,
                    }
                    && a.possibly_exposed
            });
            r.outcome = if o.faulted || o.unknown {
                Outcome::Unknown
            } else if o.complete() {
                Outcome::Complete
            } else if o.cancel_acknowledged {
                Outcome::Acknowledged
            } else if r.possibly_exposed {
                Outcome::Dispatched
            } else {
                Outcome::Accepted
            };
        }
        Ok(r)
    }
}
fn commit_id(req: &Request) -> Result<CommitId, Error> {
    // Same account/id shares one namespace across ALL mutation methods/content.
    let hash = Sha256::digest(
        [
            b"CINDER-API-COMMIT-1\0".as_slice(),
            &req.domain.network.bytes(),
            &req.domain.deployment.bytes(),
            &req.account.bytes(),
            &req.id.bytes(),
        ]
        .concat(),
    );
    CommitId::new(hash.into()).map_err(|_| Error::Invalid)
}
fn empty<B: Backend, P: Protection>(
    journal: &Journal<B, P>,
    id: CommitId,
    now: u64,
) -> Transaction {
    Transaction {
        id,
        expected: journal.head(),
        at: now,
        evidence: vec![],
        inputs: vec![],
        order_observations: vec![],
        funds_observations: vec![],
        controls: vec![],
    }
}
