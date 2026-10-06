//! Atomic private persistence. No native transport, signer or deployed witness.

pub mod collateral;
pub mod encrypted;
pub mod funds;
pub mod liquidation;
pub mod model;
pub mod orders;
pub mod projection;
pub mod protection;
pub mod raw;
pub mod recovery;
pub mod replicated;
pub mod restoration;
pub mod risk;
pub mod sqlite;
pub mod wire;
use cinder_kernel::{identity::Domain, ledger::Config};
use model::*;
use sha2::{Digest, Sha256};
use std::fmt;

// Order classification/authority/lifecycle changes replay semantics.
const ENGINE_REVISION: u16 = 21;

/// Public storage sequencing metadata, never customer identity or balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Head {
    /// Genesis is sequence zero; every atomic transaction advances once.
    pub sequence: u64,
    /// Hash of the opaque chained frame. Not an authenticated freshness witness.
    pub hash: [u8; 32],
}

/// Secret-safe journal failure. Never includes raw payloads, SQL bindings or paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Bounds exceeded; do not truncate private evidence or loop indefinitely.
    Limit,
    /// Malformed, ambiguous or corrupt bytes.
    Codec,
    /// Unsupported schema/engine revision; explicit migration is required.
    Version,
    /// Supplied record protection could not seal/open or verify its context.
    Protection,
    /// Storage is unavailable or commit outcome uncertain. Reopen/reconcile first.
    Storage,
    /// Another writer advanced the exact expected head; no implicit retries.
    Stale,
    /// Same transaction identity with different content.
    Conflict,
    /// Proposed envelope/operation violates the storage/controller contract.
    Invalid,
    /// This handle cannot expose actions after an uncertain commit.
    Poisoned,
    /// A bounded lock wait expired before any write; caller may explicitly reload/retry.
    Busy,
}

/// Bound protected history; exceeding it is containment, never pruning evidence.
pub const MAX_RECORDS: usize = 4096;
/// Total opaque replay bytes allowed per local journal.
pub const MAX_HISTORY_BYTES: usize = 64 * 1024 * 1024;

/// Context which P06 protection must authenticate with each protected record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordContext {
    /// Trusted expected deployment, not learned from an untrusted database alone.
    pub domain: Domain,
    /// Exact journal position; zero is genesis.
    pub sequence: u64,
    /// Previous opaque record commitment.
    pub previous: [u8; 32],
}
/// Mandatory trusted-runtime protection boundary. No plaintext implementation is
/// shipped. P06 must supply qualified authenticated encryption/key handling;
/// merely implementing this trait does not constitute cryptographic assurance.
pub trait Protection {
    /// Seal private canonical bytes, binding the full context.
    fn seal(&self, context: RecordContext, plaintext: &[u8]) -> Result<PrivateBytes, Error>;
    /// Authenticate/decrypt before parsing or releasing any private projection.
    fn open(&self, context: RecordContext, opaque: &PrivateBytes) -> Result<PrivateBytes, Error>;
}

/// Opaque persisted frame. Public chaining detects accidental changes, not a
/// malicious host rewriting/rolling back the whole journal and trusted head.
#[derive(Clone, PartialEq, Eq)]
pub struct Frame {
    /// Exact resulting sequence and opaque-body commitment.
    pub head: Head,
    /// Prior frame commitment, zero only at genesis.
    pub previous: [u8; 32],
    /// Protected body; backend never receives a normalized Event or signing preimage.
    pub opaque: PrivateBytes,
}
impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Frame")
            .field("sequence", &self.head.sequence)
            .finish_non_exhaustive()
    }
}
impl Frame {
    fn hash(sequence: u64, previous: [u8; 32], bytes: &[u8]) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"CINDER-OPAQUE-FRAME-1\0");
        h.update(sequence.to_be_bytes());
        h.update(previous);
        h.update((bytes.len() as u64).to_be_bytes());
        h.update(bytes);
        h.finalize().into()
    }
    fn new(sequence: u64, previous: [u8; 32], opaque: PrivateBytes) -> Self {
        Self {
            head: Head {
                sequence,
                hash: Self::hash(sequence, previous, opaque.as_bytes()),
            },
            previous,
            opaque,
        }
    }
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.opaque.as_bytes().is_empty()
            || self.opaque.as_bytes().len() > wire::MAX_RECORD
            || self.head.hash
                != Self::hash(self.head.sequence, self.previous, self.opaque.as_bytes())
        {
            Err(Error::Codec)
        } else {
            Ok(())
        }
    }
}
/// Atomic opaque storage contract. A successful append must durably bind body and
/// head together. Unknown commit outcomes must not be returned as known failures.
pub trait Backend {
    /// One consistent, bounded snapshot. No automatic creation/repair on read failure.
    fn load(&mut self) -> Result<Vec<Frame>, Error>;
    /// CAS append; None expects an empty, explicitly created store. No implicit retries.
    fn append(&mut self, expected: Option<Head>, frame: &Frame) -> Result<(), Error>;
    /// Revalidate authority/freshness before serving a durable retry. Plain P05
    /// storage has no independent witness; replicated storage overrides this.
    fn check_current(&mut self, _expected: Head) -> Result<(), Error> {
        Ok(())
    }
}

/// A one-shot local action delivery following a confirmed durable exposure record.
/// Not Clone; not authority, risk approval or exactly-once external execution.
pub struct Exposure {
    key: cinder_kernel::identity::AttemptKey,
    message: PrivateBytes,
}
impl fmt::Debug for Exposure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Exposure([PRIVATE])")
    }
}
impl Exposure {
    /// Consume the local delivery. Signer/controller must still enforce policy and
    /// capability fencing; never regenerate this delivery from a replayed receipt.
    pub fn into_message(self) -> (cinder_kernel::identity::AttemptKey, PrivateBytes) {
        (self.key, self.message)
    }
}
/// Confirmed result. Exact transaction retries return no second exposure delivery.
#[derive(Debug)]
pub struct Committed {
    /// Durable head of this transaction, not necessarily the newest head on retry.
    pub head: Head,
    /// Reproducible accounting/control result.
    pub receipt: Receipt,
    /// True for a previously committed identical transaction.
    pub duplicate: bool,
    /// Newly committed possible-exposure actions only; never populated after an error.
    pub exposures: Vec<Exposure>,
}
struct Accepted {
    tx: Transaction,
    bytes: Vec<u8>,
    receipt: Receipt,
    head: Head,
    book_changes: Vec<(cinder_kernel::identity::AccountId, projection::BookChange)>,
}

/// Private-runtime coordinator around an opaque CAS backend. Default construction
/// requires a Protection implementation; no production key/cipher is selected here.
pub struct Journal<B: Backend, P: Protection> {
    backend: B,
    protection: P,
    config: Config,
    state: State,
    head: Head,
    history: Vec<Accepted>,
    opaque_bytes: usize,
    poisoned: bool,
}
impl<B: Backend, P: Protection> fmt::Debug for Journal<B, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Journal")
            .field("sequence", &self.head.sequence)
            .field("poisoned", &self.poisoned)
            .finish_non_exhaustive()
    }
}
impl<B: Backend, P: Protection> Journal<B, P> {
    /// Explicit new genesis. Existing/nonempty data is never overwritten.
    pub fn create(mut backend: B, protection: P, config: Config) -> Result<Self, Error> {
        let state = State::new(config.clone())?;
        if !backend.load()?.is_empty() {
            return Err(Error::Conflict);
        }
        let mut w = wire::Writer::new(10);
        w.raw(&ENGINE_REVISION.to_be_bytes());
        w.blob(&wire::encode_config(&config)?);
        let context = RecordContext {
            domain: config.domain,
            sequence: 0,
            previous: [0; 32],
        };
        let opaque = protection.seal(context, &w.finish()?)?;
        let frame = Frame::new(0, [0; 32], opaque);
        frame.validate()?;
        backend.append(None, &frame)?;
        Ok(Self {
            backend,
            protection,
            config,
            state,
            head: frame.head,
            history: Vec::new(),
            opaque_bytes: frame.opaque.as_bytes().len(),
            poisoned: false,
        })
    }
    /// Replay against an independently supplied expected configuration. Missing,
    /// unsupported, corrupt or failed-protection data never becomes a fresh account.
    pub fn open(backend: B, protection: P, config: Config) -> Result<Self, Error> {
        let state = State::new(config.clone())?;
        let mut s = Self {
            backend,
            protection,
            config,
            state,
            head: Head::default(),
            history: Vec::new(),
            opaque_bytes: 0,
            poisoned: true,
        };
        s.reload()?;
        Ok(s)
    }
    /// Current private projection; disabled after uncertain storage outcome.
    pub fn state(&self) -> Result<&State, Error> {
        if self.poisoned {
            Err(Error::Poisoned)
        } else {
            Ok(&self.state)
        }
    }
    /// Check the independently witnessed current head/epoch before returning a
    /// view. A cached `state()` is not present writer authority. Read freshness is
    /// linearized at this check; it does not revoke already escaped capabilities.
    pub fn verified_state(&mut self) -> Result<&State, Error> {
        if let Err(error) = self.backend.check_current(self.head) {
            self.poisoned = true;
            return Err(error);
        }
        self.state()
    }
    /// Last locally verified head; a stale value is not an independent freshness witness.
    pub fn head(&self) -> Head {
        self.head
    }
    /// Trusted immutable genesis configuration; not a native metadata refresh.
    pub fn configuration(&self) -> &Config {
        &self.config
    }
    /// Exact retained private transaction for authorized audit, including raw evidence.
    pub fn transaction(&self, id: CommitId) -> Option<&Transaction> {
        self.history.iter().find(|r| r.tx.id == id).map(|r| &r.tx)
    }
    /// Replayed outcome for a retained transaction, including rejected controls.
    /// Private adapter provenance must not treat failed controls as authorization.
    pub fn transaction_receipt(&self, id: CommitId) -> Option<&Receipt> {
        self.history
            .iter()
            .find(|r| r.tx.id == id)
            .map(|r| &r.receipt)
    }
    /// Private accepted history for deterministic adapter projections. Revalidate
    /// freshness first; this read view grants no exposure or economic authority.
    pub fn transactions(&self) -> impl Iterator<Item = &Transaction> {
        self.history.iter().map(|r| &r.tx)
    }
    /// Accepted history paired with its exact replayed outcome in one traversal.
    /// This is retained private provenance, not a present freshness check.
    pub fn transactions_with_receipts(
        &self,
    ) -> impl Iterator<Item = (&Transaction, &model::Receipt)> {
        self.history.iter().map(|r| (&r.tx, &r.receipt))
    }
    /// Bounded explicit reload/reconciliation after contention or an unknown reply.
    /// P06 must add independent freshness; this detects no full valid-history rollback.
    pub fn reload(&mut self) -> Result<(), Error> {
        self.poisoned = true;
        let frames = self.backend.load()?;
        if frames.is_empty() || frames.len() > MAX_RECORDS {
            return Err(Error::Codec);
        }
        let mut total = 0_usize;
        let mut head = Head::default();
        let mut history: Vec<Accepted> = Vec::new();
        let mut state = State::new(self.config.clone())?;
        for (n, frame) in frames.iter().enumerate() {
            total = total
                .checked_add(frame.opaque.as_bytes().len())
                .ok_or(Error::Limit)?;
            if total > MAX_HISTORY_BYTES {
                return Err(Error::Limit);
            }
            frame.validate()?;
            if frame.head.sequence != n as u64 || frame.previous != head.hash {
                return Err(Error::Codec);
            }
            let context = RecordContext {
                domain: self.config.domain,
                sequence: n as u64,
                previous: head.hash,
            };
            let bytes = self.protection.open(context, &frame.opaque)?;
            let mut r = wire::Reader::new(bytes.as_bytes(), if n == 0 { 10 } else { 11 })?;
            if r.array::<2>()? != ENGINE_REVISION.to_be_bytes() {
                return Err(Error::Version);
            }
            if n == 0 {
                if wire::decode_config(r.blob()?)? != self.config {
                    return Err(Error::Conflict);
                }
            } else {
                let tx_bytes = r.blob()?.to_vec();
                let tx = wire::decode_transaction(&tx_bytes)?;
                if tx.expected != head || history.iter().any(|a| a.tx.id == tx.id) {
                    return Err(Error::Codec);
                }
                let (next, receipt) = state.advance(&tx)?;
                if r.blob()? != wire::receipt(&receipt)?
                    || r.array::<32>()? != wire::state_commitment(&next)?
                {
                    return Err(Error::Codec);
                }
                let book_changes = projection::capture(&state, &next, &self.config, tx.at)?;
                state = next;
                history.push(Accepted {
                    tx,
                    bytes: tx_bytes,
                    receipt,
                    head: frame.head,
                    book_changes,
                });
            }
            r.done()?;
            head = frame.head;
        }
        self.state = state;
        self.head = head;
        self.history = history;
        self.opaque_bytes = total;
        self.poisoned = false;
        Ok(())
    }
    /// Compute, protect and atomically append one transition. There is no network
    /// dispatch callback before/during commit and no automatic resend after errors.
    pub fn commit(&mut self, tx: Transaction) -> Result<Committed, Error> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        if let Err(error) = self.backend.check_current(self.head) {
            self.poisoned = true;
            return Err(error);
        }
        let bytes = wire::encode_transaction(&tx)?;
        if let Some(old) = self.history.iter().find(|a| a.tx.id == tx.id) {
            if old.bytes != bytes {
                return Err(Error::Conflict);
            }
            return Ok(Committed {
                head: old.head,
                receipt: old.receipt.clone(),
                duplicate: true,
                exposures: Vec::new(),
            });
        }
        if tx.expected != self.head {
            return Err(Error::Stale);
        }
        if self.history.len() + 1 >= MAX_RECORDS {
            return Err(Error::Limit);
        }
        let (state, receipt) = self.state.advance(&tx)?;
        let book_changes = projection::capture(&self.state, &state, &self.config, tx.at)?;
        let mut w = wire::Writer::new(11);
        w.raw(&ENGINE_REVISION.to_be_bytes());
        w.blob(&bytes);
        w.blob(&wire::receipt(&receipt)?);
        w.raw(&wire::state_commitment(&state)?);
        let next = self.head.sequence.checked_add(1).ok_or(Error::Limit)?;
        let context = RecordContext {
            domain: self.config.domain,
            sequence: next,
            previous: self.head.hash,
        };
        let opaque = self.protection.seal(context, &w.finish()?)?;
        let frame = Frame::new(next, self.head.hash, opaque);
        frame.validate()?;
        let opaque_bytes = self
            .opaque_bytes
            .checked_add(frame.opaque.as_bytes().len())
            .filter(|n| *n <= MAX_HISTORY_BYTES)
            .ok_or(Error::Limit)?;
        let mut exposures = Vec::new();
        if receipt.controls.is_none() {
            for c in &tx.controls {
                if let Control::Expose(k) = c {
                    let a = state
                        .attempts
                        .iter()
                        .find(|a| a.key == *k)
                        .ok_or(Error::Invalid)?;
                    exposures.push(Exposure {
                        key: *k,
                        message: a.message.clone(),
                    });
                }
            }
        }
        if let Err(e) = self.backend.append(Some(self.head), &frame) {
            if !matches!(e, Error::Stale | Error::Busy) {
                self.poisoned = true;
            }
            return Err(e);
        }
        self.state = state;
        self.head = frame.head;
        self.opaque_bytes = opaque_bytes;
        self.history.push(Accepted {
            tx,
            bytes,
            receipt: receipt.clone(),
            head: self.head,
            book_changes,
        });
        Ok(Committed {
            head: self.head,
            receipt,
            duplicate: false,
            exposures,
        })
    }
}
