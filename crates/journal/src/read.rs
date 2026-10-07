//! Accepted immutable read publication. No backend, keys, append, dispatch or
//! independently editable ledger. A fresh read witness is required per ticket.
use crate::{
    Accepted, Backend, Error, Head, Journal, Protection,
    model::{Receipt, State, Transaction},
    projection::BookChange,
    replicated::{Anchor, Stream},
};
use cinder_kernel::{
    identity::AccountId,
    ledger::{Config, Owner},
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Instant,
};

/// Global per-journal preparation cap, not a customer/financial policy limit.
pub const MAX_READS: usize = 4;

/// Current stream/writer/API identity installed only after qualified startup.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Binding {
    /// Exact independently configured witness stream, including domain.
    pub stream: Stream,
    /// Loaded writer fencing epoch, not customer authorization epoch.
    pub epoch: u64,
    /// Governed API configuration fingerprint, not a solvency proof.
    pub api: [u8; 32],
}
impl std::fmt::Debug for Binding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReadBinding([PRIVATE])")
    }
}
/// One-shot fresh strong-read port. Implementations may share immutable secret
/// credentials, but must not serialize network I/O or reuse a prior response.
/// This interface does not narrow the credential's actual native/IAM authority.
pub trait Witness: Send + Sync {
    /// Authenticate this exact stream's current complete head and writer epoch.
    fn read(&self, stream: Stream) -> Result<Anchor, Error>;
}
/// Read-only accepted provenance used by the SAME API projection as the writer.
/// Sealed: callers cannot inject a host-decoded financial source.
pub trait Source: sealed::Sealed {
    /// Trusted immutable configuration.
    fn configuration(&self) -> &Config;
    /// Exact accepted state, not a freshness authority or spend permit.
    fn state(&self) -> Result<&State, Error>;
    /// Exact opaque head of the accepted source.
    fn head(&self) -> Head;
    /// Private retained provenance; never returned by a customer wire interface.
    fn transactions_with_receipts(&self) -> impl Iterator<Item = (&Transaction, &Receipt)>;
    /// Replay-derived changes for an already-authorized private account.
    fn customer_book_history(&self, account: AccountId) -> Result<Vec<BookChange>, Error>;
}
mod sealed {
    pub trait Sealed {}
}
impl<B: Backend, P: Protection> sealed::Sealed for Journal<B, P> {}
impl<B: Backend, P: Protection> Source for Journal<B, P> {
    fn configuration(&self) -> &Config {
        self.configuration()
    }
    fn state(&self) -> Result<&State, Error> {
        self.state()
    }
    fn head(&self) -> Head {
        self.head()
    }
    fn transactions_with_receipts(&self) -> impl Iterator<Item = (&Transaction, &Receipt)> {
        self.transactions_with_receipts()
    }
    fn customer_book_history(&self, account: AccountId) -> Result<Vec<BookChange>, Error> {
        self.customer_book_history(account)
    }
}
pub(super) struct View {
    binding: Binding,
    config: Config,
    state: Arc<State>,
    head: Head,
    history: Vec<Arc<Accepted>>,
}
impl View {
    fn knows(&self, head: Head) -> bool {
        if head == self.head {
            return true;
        }
        if head.sequence == 0 {
            return self.history.first().is_some_and(|a| a.tx.expected == head);
        }
        usize::try_from(head.sequence - 1)
            .ok()
            .and_then(|n| self.history.get(n))
            .is_some_and(|a| a.head == head)
    }
}
struct Published {
    generation: u64,
    current: Option<Arc<View>>,
    pending: Option<Head>,
    preparing: bool,
}
struct Inner {
    published: Mutex<Published>,
    fenced: AtomicBool,
    readers: AtomicUsize,
}
impl Inner {
    fn locked(&self) -> Result<std::sync::MutexGuard<'_, Published>, Failure> {
        self.published.lock().map_err(|_| {
            self.fenced.store(true, Ordering::SeqCst);
            Failure::Fenced
        })
    }
    fn fence(&self) {
        self.fenced.store(true, Ordering::SeqCst);
        let old = self.published.lock().ok().and_then(|mut p| {
            p.pending = None;
            p.preparing = false;
            p.current.take()
        });
        // Full retained data can be the last reference: drop outside the latch.
        drop(old);
    }
}
pub(crate) struct PanicGuard {
    inner: Arc<Inner>,
}
impl Drop for PanicGuard {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.inner.fence();
        }
    }
}
/// Read side of the one journal's publication. Clone shares a bounded admission
/// budget, not witness evidence, state copies or writer authority.
#[derive(Clone)]
pub struct Reader {
    inner: Arc<Inner>,
}
/// Normal contention/races are distinct from failed/fenced authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// Global read-preparation slots are occupied.
    Busy,
    /// A known accepted/in-flight local generation changed; no stale output.
    Raced,
    /// Unknown head, lost authority, poison, clock/witness failure or boot fence.
    Fenced,
}
/// Bounded unreleased snapshot. Not Clone: retains one admission slot until drop
/// or release, and cannot be constructed from host data.
pub struct Ticket {
    inner: Arc<Inner>,
    view: Arc<View>,
    generation: u64,
}
impl Drop for Ticket {
    fn drop(&mut self) {
        self.inner.readers.fetch_sub(1, Ordering::SeqCst);
    }
}
/// A ticket with its OWN matching witness observation. Not Clone and not reusable
/// as a token for subsequent requests/polls. Release consumes it.
pub struct Verified {
    ticket: Ticket,
}
impl sealed::Sealed for Verified {}
impl Source for Verified {
    fn configuration(&self) -> &Config {
        &self.ticket.view.config
    }
    fn state(&self) -> Result<&State, Error> {
        Ok(&self.ticket.view.state)
    }
    fn head(&self) -> Head {
        self.ticket.view.head
    }
    fn transactions_with_receipts(&self) -> impl Iterator<Item = (&Transaction, &Receipt)> {
        self.ticket.view.history.iter().map(|a| (&a.tx, &a.receipt))
    }
    fn customer_book_history(&self, account: AccountId) -> Result<Vec<BookChange>, Error> {
        self.state()?
            .ledger()
            .book(Owner::Customer(account))
            .map_err(|_| Error::Invalid)?;
        Ok(self
            .ticket
            .view
            .history
            .iter()
            .flat_map(|a| &a.book_changes)
            .filter(|(id, _)| *id == account)
            .map(|(_, c)| c.clone())
            .collect())
    }
}
impl Ticket {
    /// Governed binding of this captured accepted publication.
    pub fn binding(&self) -> Binding {
        self.view.binding
    }
    /// Configuration only, for signature/session validation BEFORE witness I/O.
    pub fn configuration(&self) -> &Config {
        &self.view.config
    }
}
impl Verified {
    /// Governed binding, checked by the API against its loaded fingerprint.
    pub fn binding(&self) -> Binding {
        self.ticket.binding()
    }
    /// Final short metadata gate AFTER qualified time/auth/projection work.
    /// `until` is a conservative processing deadline, never a wall-clock source.
    /// No I/O, NSM, signature verification, encoding or socket write under latch.
    pub fn release(self, until: Instant) -> Result<(), Failure> {
        self.release_inner(until, None)
    }
    /// Check the owning runtime's actual sticky stop flag under the same short
    /// metadata gate. No callback or remote work is admitted under the latch.
    pub fn release_if_running(self, until: Instant, stopped: &AtomicBool) -> Result<(), Failure> {
        self.release_inner(until, Some(stopped))
    }
    fn release_inner(self, until: Instant, stopped: Option<&AtomicBool>) -> Result<(), Failure> {
        let p = self.ticket.inner.locked()?;
        if self.ticket.inner.fenced.load(Ordering::SeqCst)
            || stopped.is_some_and(|s| s.load(Ordering::SeqCst))
        {
            return Err(Failure::Fenced);
        }
        if Instant::now() >= until {
            return Err(Failure::Raced);
        }
        if p.generation != self.ticket.generation
            || p.current
                .as_ref()
                .is_none_or(|v| !Arc::ptr_eq(v, &self.ticket.view))
        {
            return Err(Failure::Raced);
        }
        Ok(())
    }
}
impl Reader {
    /// Ownership only, not a freshness check or release permit. A runtime must
    /// not consume another boot's otherwise similarly configured candidate.
    pub fn owns(&self, verified: &Verified) -> bool {
        Arc::ptr_eq(&self.inner, &verified.ticket.inner)
    }
    /// Capture without holding any writer or remote-I/O lock.
    pub fn capture(&self) -> Result<Ticket, Failure> {
        if self.inner.fenced.load(Ordering::SeqCst) {
            return Err(Failure::Fenced);
        }
        self.inner
            .readers
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
                (n < MAX_READS).then_some(n + 1)
            })
            .map_err(|_| Failure::Busy)?;
        let captured = (|| {
            let p = self.inner.locked()?;
            if self.inner.fenced.load(Ordering::SeqCst) {
                return Err(Failure::Fenced);
            }
            Ok(Ticket {
                inner: self.inner.clone(),
                view: p.current.as_ref().map(Arc::clone).ok_or(Failure::Fenced)?,
                generation: p.generation,
            })
        })();
        if captured.is_err() {
            self.inner.readers.fetch_sub(1, Ordering::SeqCst);
        }
        captured
    }
    /// Each invocation issues a new authenticated witness read, outside latches.
    /// Known local acceptance/publication races refuse without killing the writer.
    pub fn verify(&self, ticket: Ticket, witness: &dyn Witness) -> Result<Verified, Failure> {
        let _panic = PanicGuard {
            inner: self.inner.clone(),
        };
        if !Arc::ptr_eq(&self.inner, &ticket.inner) {
            return Err(Failure::Fenced);
        }
        let anchor = match witness.read(ticket.view.binding.stream) {
            Ok(a) => a,
            Err(_) => {
                self.inner.fence();
                return Err(Failure::Fenced);
            }
        };
        if anchor.epoch != ticket.view.binding.epoch || anchor.head.is_none() {
            self.inner.fence();
            return Err(Failure::Fenced);
        }
        let (generation, current, pending) = {
            let p = self.inner.locked()?;
            if self.inner.fenced.load(Ordering::SeqCst) {
                return Err(Failure::Fenced);
            }
            (p.generation, p.current.as_ref().map(Arc::clone), p.pending)
        };
        let head = anchor.head.ok_or(Failure::Fenced)?;
        if head != ticket.view.head {
            if head.sequence > ticket.view.head.sequence
                && (pending == Some(head) || current.as_ref().is_some_and(|v| v.knows(head)))
            {
                return Err(Failure::Raced);
            }
            self.inner.fence();
            return Err(Failure::Fenced);
        }
        if generation != ticket.generation {
            return Err(Failure::Raced);
        }
        Ok(Verified { ticket })
    }
    /// Sticky publication failure, for the owning runtime's boot fence/health.
    pub fn fenced(&self) -> bool {
        self.inner.fenced.load(Ordering::SeqCst)
    }
    /// Owning boot failure invalidates every outstanding and future read ticket.
    pub fn close(&self) {
        self.inner.fence();
    }
}
pub(crate) struct Publication {
    inner: Arc<Inner>,
    binding: Binding,
}
pub(crate) struct Writing {
    inner: Arc<Inner>,
    binding: Binding,
    armed: bool,
}
impl Drop for Writing {
    fn drop(&mut self) {
        if self.armed {
            self.inner.fence();
        }
    }
}
impl Publication {
    pub(crate) fn checking(&self) -> PanicGuard {
        PanicGuard {
            inner: self.inner.clone(),
        }
    }
    pub(crate) fn fenced(&self) -> bool {
        self.inner.fenced.load(Ordering::SeqCst)
    }
    pub(crate) fn enter(&self) -> Result<Writing, Error> {
        let mut p = self.inner.locked().map_err(|_| Error::Poisoned)?;
        if self.fenced() || p.preparing || p.current.is_none() {
            return Err(Error::Poisoned);
        }
        p.preparing = true;
        Ok(Writing {
            inner: self.inner.clone(),
            binding: self.binding,
            armed: true,
        })
    }
}
impl Writing {
    pub(crate) fn binding(&self) -> Binding {
        self.binding
    }
    pub(crate) fn propose(&self, next: Head) -> Result<(), Error> {
        let mut p = self.inner.locked().map_err(|_| Error::Poisoned)?;
        if self.inner.fenced.load(Ordering::SeqCst)
            || !p.preparing
            || p.pending.is_some()
            || p.current
                .as_ref()
                .is_none_or(|v| v.head.sequence.checked_add(1) != Some(next.sequence))
        {
            return Err(Error::Poisoned);
        }
        p.pending = Some(next);
        Ok(())
    }
    pub(crate) fn cancel_known(mut self) -> Result<(), Error> {
        let mut p = self.inner.locked().map_err(|_| Error::Poisoned)?;
        p.pending = None;
        p.preparing = false;
        self.armed = false;
        Ok(())
    }
    pub(super) fn publish(mut self, view: View) -> Result<(), Error> {
        let view = Arc::new(view);
        let old = {
            let mut p = self.inner.locked().map_err(|_| Error::Poisoned)?;
            if self.inner.fenced.load(Ordering::SeqCst) || p.pending != Some(view.head) {
                return Err(Error::Poisoned);
            }
            p.generation = p.generation.checked_add(1).ok_or(Error::Limit)?;
            p.pending = None;
            p.preparing = false;
            let old = p.current.replace(view);
            self.armed = false;
            old
        };
        drop(old);
        Ok(())
    }
}

impl<B: Backend, P: Protection> Journal<B, P> {
    /// Attach one read boot AFTER governed API/controller initialization. Performs
    /// the existing writer freshness check; no host snapshot or new authority.
    /// Reconciliation cannot revive a fenced reader: reopen a new qualified boot.
    pub fn attach_reader(&mut self, binding: Binding) -> Result<Reader, Error> {
        if self.publication.is_some()
            || binding.stream.domain != self.config.domain
            || binding.stream.id == [0; 32]
            || binding.epoch == 0
            || binding.api == [0; 32]
        {
            return Err(Error::Invalid);
        }
        self.verified_state()?;
        let inner = Arc::new(Inner {
            published: Mutex::new(Published {
                generation: 1,
                current: Some(Arc::new(self.read_view(binding))),
                pending: None,
                preparing: false,
            }),
            fenced: AtomicBool::new(false),
            readers: AtomicUsize::new(0),
        });
        self.publication = Some(Publication {
            inner: inner.clone(),
            binding,
        });
        Ok(Reader { inner })
    }
    pub(super) fn read_view(&self, binding: Binding) -> View {
        View {
            binding,
            config: self.config.clone(),
            state: self.state.clone(),
            head: self.head,
            history: self.history.clone(),
        }
    }
    pub(super) fn close_publication(&self) {
        if let Some(p) = &self.publication {
            p.inner.fence();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn poisoned_publication_latch_is_a_sticky_fence_not_retryable_contention() {
        let inner = Arc::new(Inner {
            published: Mutex::new(Published {
                generation: 1,
                current: None,
                pending: None,
                preparing: false,
            }),
            fenced: AtomicBool::new(false),
            readers: AtomicUsize::new(0),
        });
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _lock = inner.published.lock().unwrap();
                panic!("synthetic latch poison");
            }))
            .is_err()
        );
        let reader = Reader { inner };
        assert!(matches!(reader.capture(), Err(Failure::Fenced)));
        assert!(reader.fenced());
        assert_eq!(reader.inner.readers.load(Ordering::SeqCst), 0);
    }
}
