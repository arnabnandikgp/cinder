//! Joined durable transaction semantics; not another asset ledger.

use crate::{
    Error, Head,
    wire::{MAX_ITEMS, MAX_RECORD},
};
use cinder_kernel::{
    amounts::QuoteAtoms,
    identity::*,
    ledger::{evidence::Disposition, *},
};
use std::fmt;
use zeroize::Zeroize;

/// Private bytes: never print raw evidence or attempted signing material in Debug.
#[derive(Clone, PartialEq, Eq)]
pub struct PrivateBytes(Vec<u8>);
impl PrivateBytes {
    /// Retain exact bounded bytes; caller excludes credentials/transport headers.
    pub fn new(mut bytes: Vec<u8>) -> Result<Self, Error> {
        if bytes.len() > MAX_RECORD {
            bytes.zeroize();
            Err(Error::Limit)
        } else {
            Ok(Self(bytes))
        }
    }
    /// Private-runtime access, not a public/logging API.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
impl fmt::Debug for PrivateBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PrivateBytes([REDACTED])")
    }
}
impl Drop for PrivateBytes {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// Independent journal transaction identity; not a request, attempt or venue event ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitId([u8; 32]);
impl CommitId {
    /// Zero is reserved; identities must be retained across an uncertain commit.
    pub fn new(bytes: [u8; 32]) -> Result<Self, Error> {
        if bytes == [0; 32] {
            Err(Error::Invalid)
        } else {
            Ok(Self(bytes))
        }
    }
    /// Exact identifier bytes, scoped by the journal's genesis/domain.
    pub fn bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Raw evidence plus its normalized proposal; source/authority are qualified by P13.
#[derive(Clone, PartialEq, Eq)]
pub struct Input {
    /// Physical source including network/deployment and semantic namespace.
    pub source: EventScope,
    /// Source causal coverage if qualified; None never means complete.
    pub source_cut: Option<u64>,
    /// Nonzero qualified authority revision, not proof of authentication itself.
    pub authority_epoch: u64,
    /// Injected monotone observation time in milliseconds.
    pub observed_at: u64,
    /// Exact body/response evidence, excluding bearer tokens and secret headers.
    pub raw: PrivateBytes,
    /// None retains an unnormalized fact and prevents dependent exposure.
    pub event: Option<Event>,
}
impl fmt::Debug for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Input([PRIVATE])")
    }
}

/// Reserved capacity is not an asset. Full margin/capital envelopes arrive in P09.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resource {
    /// Flat customer's settled cash only; never negative debt or open-position equity.
    Customer(AccountId),
    /// Flat house cash, not customer capital or an insurance-coverage decision.
    House,
    /// Physical vault liquidity or flat native cash, separately from entitlements.
    Location(Location),
}
/// One component of a shared reservation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reservation {
    /// Explicit ownership/location dimension.
    pub resource: Resource,
    /// Positive quote amount; no rounding or implicit netting.
    pub amount: QuoteAtoms,
}
/// One operation's immutable, multi-resource commitment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hold {
    /// Operation identity, never reusable after release.
    pub request: RequestKey,
    /// Unique resource dimensions; simultaneous use is not duplicated capacity.
    pub reservations: Vec<Reservation>,
    /// Released holds remain in the history rather than allowing ID reuse.
    pub active: bool,
}
/// Exact action prepared before potential signature/transport exposure.
#[derive(Clone, PartialEq, Eq)]
pub struct Attempt {
    /// Attempt under the reservation's parent request.
    pub key: AttemptKey,
    /// Exact unsigned signing preimage/action encoding; never a generated secret key.
    pub message: PrivateBytes,
    /// Capability authority epoch qualified by the controller, not a DB lease.
    pub authority_epoch: u64,
    /// Policy expiry in the injected time domain; expiry does not release holds.
    pub expires_at: u64,
    /// Once true, reconciliation is required even if no reply reached the caller.
    pub possibly_exposed: bool,
}
impl fmt::Debug for Attempt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Attempt([PRIVATE])")
    }
}

/// Atomic control mutations; policy/authorization is a separate P07/P09 prerequisite.
#[derive(Clone, PartialEq, Eq)]
pub enum Control {
    /// Acquire unique resources under one compare-and-swap head.
    Reserve {
        /// Parent operation.
        request: RequestKey,
        /// Positive resources; active status is not caller-controlled.
        reservations: Vec<Reservation>,
    },
    /// Cancel an unexposed reservation. Exposed release requires later qualified lifecycle.
    Release(RequestKey),
    /// Persist an immutable exact action, without exposing or signing it.
    Prepare {
        /// Parent/attempt identity.
        key: AttemptKey,
        /// Exact bounded private action.
        message: PrivateBytes,
        /// Qualified epoch.
        authority_epoch: u64,
        /// Policy expiry.
        expires_at: u64,
    },
    /// Durably mark possible exposure before a one-shot local delivery can be returned.
    Expose(AttemptKey),
}
impl fmt::Debug for Control {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Control([PRIVATE])")
    }
}

/// Immutable atomic input. Changed content under an existing CommitId conflicts.
#[derive(Clone, PartialEq, Eq)]
pub struct Transaction {
    /// Journal-level replay identity.
    pub id: CommitId,
    /// Exact sequence/hash this proposal was constructed against.
    pub expected: Head,
    /// Trusted injected transaction time; no clock is read by the model.
    pub at: u64,
    /// Raw and normalized observations, including duplicate/conflicting input.
    pub inputs: Vec<Input>,
    /// All-or-none control subproposal; actual external facts remain recordable.
    pub controls: Vec<Control>,
}
impl fmt::Debug for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Transaction([PRIVATE])")
    }
}

/// Retained input outcome, including rejected outer envelopes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputResult {
    /// A body that cannot yet be normalized; never a fabricated zero observation.
    Unnormalized,
    /// Invalid source/time/authority envelope retained outside the financial kernel.
    EnvelopeRejected,
    /// Exact kernel result, including contained financial rejection or duplicate.
    Normalized(Disposition),
}
/// Deterministic refusal of the entire control subproposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlError {
    /// Identity/scope/amount/operation is invalid.
    Invalid,
    /// The resource lacks unreserved flat settled capacity.
    Capacity,
    /// Open positions, unsettled funding or unresolved evidence need later qualification.
    Unqualified,
    /// The action may have escaped; automatic resend/release is forbidden.
    Exposed,
    /// Action policy expiry has passed, without discharging the obligation.
    Expired,
}
/// Reproducible receipt; not a signing or capital authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    /// One disposition for every input, including corrupt/duplicate evidence.
    pub inputs: Vec<InputResult>,
    /// None means all controls applied; Some means none applied.
    pub controls: Option<ControlError>,
    /// Resulting kernel proposal version, not the journal sequence.
    pub ledger_version: u64,
}

/// One joined replay state. Reservations/attempts own no second set of assets.
#[derive(Clone, PartialEq, Eq)]
pub struct State {
    pub(crate) config: Config,
    pub(crate) ledger: Ledger,
    pub(crate) holds: Vec<Hold>,
    pub(crate) attempts: Vec<Attempt>,
    pub(crate) raw_unresolved: u64,
    pub(crate) now: u64,
}
impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("State([PRIVATE])")
    }
}
impl State {
    pub(crate) fn new(config: Config) -> Result<Self, Error> {
        if config
            .markets
            .len()
            .saturating_mul(config.customers.len().saturating_add(3))
            > 4096
        {
            return Err(Error::Limit);
        }
        Ok(Self {
            ledger: Ledger::new(config.clone()).map_err(|_| Error::Invalid)?,
            config,
            holds: Vec::new(),
            attempts: Vec::new(),
            raw_unresolved: 0,
            now: 0,
        })
    }
    /// Read-only financial projection. No direct mutation can bypass journal consumption.
    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }
    /// Read-only commitments, including released records.
    pub fn holds(&self) -> &[Hold] {
        &self.holds
    }
    /// Prepared/exposed action history, never an instruction to resend on restart.
    pub fn attempts(&self) -> &[Attempt] {
        &self.attempts
    }
    /// Outer evidence needing normalization/qualification; no generic admin reset.
    pub fn unresolved_raw(&self) -> u64 {
        self.raw_unresolved
    }
    /// Sum active holds on this exact resource dimension.
    pub fn reserved(&self, resource: Resource) -> Result<QuoteAtoms, Error> {
        self.holds
            .iter()
            .filter(|h| h.active)
            .flat_map(|h| &h.reservations)
            .filter(|r| r.resource == resource)
            .try_fold(QuoteAtoms::new(self.config.quote, 0), |sum, r| {
                sum.checked_add(r.amount).map_err(|_| Error::Limit)
            })
    }
    fn capacity(&self, resource: Resource) -> Result<QuoteAtoms, ControlError> {
        let book = match resource {
            Resource::Location(Location::Vault) => return Ok(self.ledger.vault()),
            Resource::Location(Location::Venue) => self.ledger.venue(),
            Resource::House => self
                .ledger
                .book(Owner::House)
                .map_err(|_| ControlError::Invalid)?,
            Resource::Customer(id) => self
                .ledger
                .book(Owner::Customer(id))
                .map_err(|_| ControlError::Invalid)?,
        };
        if book.funding().atoms() != 0 || book.positions().iter().any(|p| p.quantity().lots() != 0)
        {
            return Err(ControlError::Unqualified);
        }
        Ok(QuoteAtoms::new(
            self.config.quote,
            book.cash().atoms().max(0),
        ))
    }
    fn request(&self, k: RequestKey) -> Result<(), ControlError> {
        if k.domain != self.config.domain || self.ledger.book(Owner::Customer(k.account)).is_err() {
            Err(ControlError::Invalid)
        } else {
            Ok(())
        }
    }
    fn all_capacity(&self) -> Result<(), ControlError> {
        for r in self
            .holds
            .iter()
            .filter(|h| h.active)
            .flat_map(|h| &h.reservations)
        {
            if self
                .reserved(r.resource)
                .map_err(|_| ControlError::Capacity)?
                .atoms()
                > self.capacity(r.resource)?.atoms()
            {
                return Err(ControlError::Capacity);
            }
        }
        Ok(())
    }
    fn control(&mut self, c: &Control) -> Result<(), ControlError> {
        match c {
            Control::Reserve {
                request,
                reservations,
            } => {
                self.request(*request)?;
                if reservations.is_empty()
                    || reservations.len() > MAX_ITEMS
                    || self.holds.len() >= MAX_ITEMS
                    || self.holds.iter().any(|h| h.request == *request)
                {
                    return Err(ControlError::Invalid);
                }
                for (i, r) in reservations.iter().enumerate() {
                    if r.amount.unit() != self.config.quote
                        || r.amount.atoms() <= 0
                        || reservations[..i]
                            .iter()
                            .any(|old| old.resource == r.resource)
                    {
                        return Err(ControlError::Invalid);
                    }
                    if matches!(r.resource, Resource::Customer(id) if id!=request.account) {
                        return Err(ControlError::Invalid);
                    }
                }
                self.holds.push(Hold {
                    request: *request,
                    reservations: reservations.clone(),
                    active: true,
                });
                self.all_capacity()?;
            }
            Control::Release(request) => {
                self.request(*request)?;
                if self
                    .attempts
                    .iter()
                    .any(|a| a.key.request == *request && a.possibly_exposed)
                {
                    return Err(ControlError::Exposed);
                }
                let h = self
                    .holds
                    .iter_mut()
                    .find(|h| h.request == *request && h.active)
                    .ok_or(ControlError::Invalid)?;
                h.active = false;
            }
            Control::Prepare {
                key,
                message,
                authority_epoch,
                expires_at,
            } => {
                self.request(key.request)?;
                if self.attempts.len() >= MAX_ITEMS
                    || message.as_bytes().is_empty()
                    || message.as_bytes().len() > 65536
                    || *authority_epoch == 0
                    || self.attempts.iter().any(|a| a.key.request == key.request)
                    || !self
                        .holds
                        .iter()
                        .any(|h| h.request == key.request && h.active)
                {
                    return Err(ControlError::Invalid);
                }
                if *expires_at <= self.now {
                    return Err(ControlError::Expired);
                }
                self.attempts.push(Attempt {
                    key: *key,
                    message: message.clone(),
                    authority_epoch: *authority_epoch,
                    expires_at: *expires_at,
                    possibly_exposed: false,
                });
            }
            Control::Expose(key) => {
                if self.raw_unresolved != 0
                    || self.ledger.issues().iter().any(|i| i.open)
                    || self.ledger.unresolved_attribution() != 0
                {
                    return Err(ControlError::Unqualified);
                }
                self.all_capacity()?;
                let a = self
                    .attempts
                    .iter_mut()
                    .find(|a| a.key == *key)
                    .ok_or(ControlError::Invalid)?;
                if a.possibly_exposed {
                    return Err(ControlError::Exposed);
                }
                if a.expires_at <= self.now {
                    return Err(ControlError::Expired);
                }
                if !self
                    .holds
                    .iter()
                    .any(|h| h.request == key.request && h.active)
                {
                    return Err(ControlError::Invalid);
                }
                a.possibly_exposed = true;
            }
        }
        Ok(())
    }
    pub(crate) fn advance(&self, tx: &Transaction) -> Result<(Self, Receipt), Error> {
        let observations = self
            .ledger
            .observations()
            .len()
            .saturating_add(tx.inputs.len());
        let cells = observations.saturating_mul(
            self.config
                .markets
                .len()
                .saturating_mul(2)
                .saturating_add(self.config.customers.len())
                .saturating_add(4),
        );
        if observations > 4096 || cells > 65536 {
            return Err(Error::Limit);
        }
        if tx.at < self.now || tx.inputs.len() > MAX_ITEMS || tx.controls.len() > MAX_ITEMS {
            return Err(Error::Invalid);
        }
        let mut s = self.clone();
        s.now = tx.at;
        let mut inputs = Vec::new();
        for input in &tx.inputs {
            let valid = input.authority_epoch != 0
                && input.observed_at <= tx.at
                && input.source.domain == self.config.domain
                && self.config.sources.iter().any(|s| s.scope == input.source)
                && input.event.as_ref().is_none_or(|e| match &e.key {
                    RecordKey::Economic(k) => k.scope == input.source,
                    _ => e.key.require_domain(self.config.domain).is_ok(),
                });
            let result = if !valid {
                InputResult::EnvelopeRejected
            } else if let Some(e) = &input.event {
                match s.ledger.ingest(e, input.observed_at) {
                    Ok(ingested) => {
                        s.ledger = ingested.state;
                        InputResult::Normalized(ingested.disposition)
                    }
                    Err(_) => InputResult::EnvelopeRejected,
                }
            } else {
                InputResult::Unnormalized
            };
            if matches!(
                result,
                InputResult::Unnormalized | InputResult::EnvelopeRejected
            ) {
                s.raw_unresolved = s.raw_unresolved.checked_add(1).ok_or(Error::Limit)?;
            }
            inputs.push(result);
        }
        let mut candidate = s.clone();
        let mut controls = None;
        // A duplicate/conflicting fact cannot be reused to release new commitments.
        if !tx.controls.is_empty()
            && inputs
                .iter()
                .any(|i| !matches!(i, InputResult::Normalized(Disposition::Applied)))
        {
            controls = Some(ControlError::Unqualified);
        }
        if controls.is_none() {
            for c in &tx.controls {
                if let Err(e) = candidate.control(c) {
                    controls = Some(e);
                    break;
                }
            }
            if controls.is_none() {
                s = candidate;
            }
        }
        let receipt = Receipt {
            inputs,
            controls,
            ledger_version: s.ledger.version(),
        };
        Ok((s, receipt))
    }
}
