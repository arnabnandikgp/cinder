//! One opaque CAS history for the entire deployment; no per-unit backends.
use super::*;
use crate::{Backend, Frame, MAX_HISTORY_BYTES, MAX_RECORDS, Protection, RecordContext};

struct Accepted {
    tx: Transaction,
    bytes: model::PrivateBytes,
    receipt: Receipt,
    head: Head,
}

/// Durable result. No exposure field: this foundation cannot dispatch/sign.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Committed {
    /// Original durable transaction head, possibly older on exact retry.
    pub head: Head,
    /// Replay-derived shared or unit result.
    pub receipt: Receipt,
    /// Exact retained retry, after a current-head freshness check.
    pub duplicate: bool,
}

/// Explicit opt-in new-history controller. Not attached to shipping Runtime.
pub struct Journal<B: Backend, P: Protection> {
    backend: B,
    protection: P,
    state: State,
    head: Head,
    history: Vec<Accepted>,
    opaque_bytes: usize,
    poisoned: bool,
}
impl<B: Backend, P: Protection> fmt::Debug for Journal<B, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnitJournal")
            .field("sequence", &self.head.sequence)
            .field("poisoned", &self.poisoned)
            .finish_non_exhaustive()
    }
}
impl<B: Backend, P: Protection> Journal<B, P> {
    /// Create empty books only; a nonempty backend is never overwritten.
    pub fn create(mut backend: B, protection: P, config: Configuration) -> Result<Self, Error> {
        let state = State::new(config)?;
        if !backend.load()?.is_empty() {
            return Err(Error::Conflict);
        }
        let mut w = codec::writer(codec::GENESIS);
        w.blob(&codec::configuration(&state.config)?);
        let opaque = protection.seal(
            RecordContext {
                domain: state.config.domain,
                sequence: 0,
                previous: [0; 32],
            },
            &w.finish()?,
        )?;
        let frame = Frame::new(0, [0; 32], opaque);
        frame.validate()?;
        backend.append(None, &frame)?;
        Ok(Self {
            backend,
            protection,
            state,
            head: frame.head,
            history: vec![],
            opaque_bytes: frame.opaque.as_bytes().len(),
            poisoned: false,
        })
    }
    /// Replay against independently trusted expected genesis. Never auto-create.
    pub fn open(backend: B, protection: P, config: Configuration) -> Result<Self, Error> {
        let mut s = Self {
            backend,
            protection,
            state: State::new(config)?,
            head: Head::default(),
            history: vec![],
            opaque_bytes: 0,
            poisoned: true,
        };
        s.reload()?;
        Ok(s)
    }
    /// Cached accepted view, unavailable on this handle after uncertain storage.
    /// This is not an independent freshness check or signing authority.
    pub fn state(&self) -> Result<&State, Error> {
        if self.poisoned {
            Err(Error::Poisoned)
        } else {
            Ok(&self.state)
        }
    }
    /// Revalidate the shared backend/witness, fencing all units on any failure.
    pub fn verified_state(&mut self) -> Result<&State, Error> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        self.poisoned = true;
        self.backend.check_current(self.head)?;
        self.poisoned = false;
        self.state()
    }
    /// Last accepted local head, not a fresh authority check.
    pub fn head(&self) -> Head {
        self.head
    }
    /// Trusted immutable registry; never native metadata learned from storage.
    pub fn configuration(&self) -> &Configuration {
        &self.state.config
    }
    /// Authorized private retained history. Fails closed if the handle is fenced.
    pub fn transactions(&self) -> Result<impl Iterator<Item = (&Transaction, &Receipt)>, Error> {
        self.state()?;
        Ok(self.history.iter().map(|r| (&r.tx, &r.receipt)))
    }
    /// Explicit bounded reconciliation; failure leaves the whole handle fenced.
    pub fn reload(&mut self) -> Result<(), Error> {
        self.poisoned = true;
        let frames = self.backend.load()?;
        if frames.is_empty() || frames.len() > MAX_RECORDS {
            return Err(Error::Codec);
        }
        let mut state = State::new(self.state.config.clone())?;
        let expected_config = codec::configuration(&state.config)?;
        let mut head = Head::default();
        let mut history: Vec<Accepted> = vec![];
        let mut total = 0_usize;
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
            let plaintext = self.protection.open(
                RecordContext {
                    domain: state.config.domain,
                    sequence: n as u64,
                    previous: head.hash,
                },
                &frame.opaque,
            )?;
            let mut r = codec::reader(
                plaintext.as_bytes(),
                if n == 0 {
                    codec::GENESIS
                } else {
                    codec::RECORD
                },
            )?;
            if n == 0 {
                if r.blob()? != expected_config {
                    return Err(Error::Conflict);
                }
            } else {
                let bytes = model::PrivateBytes::new(r.blob()?.to_vec())?;
                let tx = codec::decode_transaction(bytes.as_bytes())?;
                if tx.expected != head || history.iter().any(|a| a.tx.id == tx.id) {
                    return Err(Error::Codec);
                }
                let (next, receipt) = state.advance(&tx)?;
                if r.blob()? != codec::receipt(&receipt)?
                    || r.array::<32>()? != codec::state(&next)?
                {
                    return Err(Error::Codec);
                }
                state = next;
                history.push(Accepted {
                    tx,
                    bytes,
                    receipt,
                    head: frame.head,
                });
            }
            r.done()?;
            head = frame.head;
        }
        // Loading authenticated bytes alone need not establish present authority.
        self.backend.check_current(head)?;
        self.state = state;
        self.head = head;
        self.history = history;
        self.opaque_bytes = total;
        self.poisoned = false;
        Ok(())
    }
    /// Compute, seal and CAS one shared transition. No external I/O/action callback.
    /// Unknown append outcomes require explicit replay, never automatic resend.
    pub fn commit(&mut self, tx: Transaction) -> Result<Committed, Error> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        self.poisoned = true;
        self.backend.check_current(self.head)?;
        self.poisoned = false;
        let bytes = model::PrivateBytes::new(codec::transaction(&tx)?)?;
        if let Some(old) = self.history.iter().find(|r| r.tx.id == tx.id) {
            if old.bytes != bytes {
                return Err(Error::Conflict);
            }
            return Ok(Committed {
                head: old.head,
                receipt: old.receipt.clone(),
                duplicate: true,
            });
        }
        if tx.expected != self.head {
            return Err(Error::Stale);
        }
        if self.history.len() + 1 >= MAX_RECORDS {
            return Err(Error::Limit);
        }
        let (state, receipt) = self.state.advance(&tx)?;
        let mut w = codec::writer(codec::RECORD);
        w.blob(bytes.as_bytes());
        w.blob(&codec::receipt(&receipt)?);
        w.raw(&codec::state(&state)?);
        let sequence = self.head.sequence.checked_add(1).ok_or(Error::Limit)?;
        let opaque = self.protection.seal(
            RecordContext {
                domain: state.config.domain,
                sequence,
                previous: self.head.hash,
            },
            &w.finish()?,
        )?;
        let frame = Frame::new(sequence, self.head.hash, opaque);
        frame.validate()?;
        let total = self
            .opaque_bytes
            .checked_add(frame.opaque.as_bytes().len())
            .filter(|n| *n <= MAX_HISTORY_BYTES)
            .ok_or(Error::Limit)?;
        self.poisoned = true;
        if let Err(e) = self.backend.append(Some(self.head), &frame) {
            if matches!(e, Error::Stale | Error::Busy) {
                self.poisoned = false;
            }
            return Err(e);
        }
        self.state = state;
        self.head = frame.head;
        self.opaque_bytes = total;
        self.history.push(Accepted {
            tx,
            bytes,
            receipt: receipt.clone(),
            head: frame.head,
        });
        self.poisoned = false;
        Ok(Committed {
            head: frame.head,
            receipt,
            duplicate: false,
        })
    }
}
