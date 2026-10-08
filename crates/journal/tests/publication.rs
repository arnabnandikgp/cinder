//! Accepted-publication races with actual SQLite/AEAD; witness timing is synthetic.
mod support;
use cinder_journal::{
    Backend, Error, Frame, Head, Journal,
    encrypted::RecordCipher,
    read::{Binding, Failure, Reader, Source, Witness},
    replicated::{Anchor, Stream},
    sqlite::SqliteBackend,
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

#[derive(Default)]
struct Shared {
    anchor: Mutex<Option<Head>>,
    mode: AtomicU8,
    reads: AtomicUsize,
    barrier: Mutex<Option<(mpsc::Sender<()>, mpsc::Receiver<()>)>>,
}
impl Shared {
    fn pause(&self) {
        if let Some((entered, resume)) = self.barrier.lock().unwrap().take() {
            entered.send(()).unwrap();
            resume.recv_timeout(Duration::from_secs(5)).unwrap();
        }
    }
}
struct Storage {
    sqlite: SqliteBackend,
    shared: Arc<Shared>,
}
impl Backend for Storage {
    fn load(&mut self) -> Result<Vec<Frame>, Error> {
        self.sqlite.load()
    }
    fn check_current(&mut self, expected: Head) -> Result<(), Error> {
        assert_ne!(
            self.shared.mode.load(Ordering::SeqCst),
            6,
            "synthetic check panic"
        );
        if *self.shared.anchor.lock().unwrap() != Some(expected) {
            return Err(Error::Stale);
        }
        self.sqlite.check_current(expected)
    }
    fn append(&mut self, expected: Option<Head>, frame: &Frame) -> Result<(), Error> {
        let mode = self.shared.mode.load(Ordering::SeqCst);
        if mode == 1 {
            self.shared.pause();
        }
        if mode == 4 {
            return Err(Error::Busy);
        }
        assert_ne!(mode, 5, "synthetic append panic");
        self.sqlite.append(expected, frame)?;
        *self.shared.anchor.lock().unwrap() = Some(frame.head);
        if mode == 2 {
            self.shared.pause();
        }
        if mode == 3 {
            Err(Error::Storage)
        } else {
            Ok(())
        }
    }
}
struct Fresh {
    shared: Arc<Shared>,
    binding: Binding,
}
impl Witness for Fresh {
    fn read(&self, stream: Stream) -> Result<Anchor, Error> {
        if stream != self.binding.stream {
            return Err(Error::Stale);
        }
        self.shared.reads.fetch_add(1, Ordering::SeqCst);
        Ok(Anchor {
            epoch: self.binding.epoch,
            head: *self.shared.anchor.lock().unwrap(),
        })
    }
}
type Store = Journal<Storage, RecordCipher>;
fn fixture() -> (support::Temp, Store, Reader, Fresh) {
    let temp = support::Temp::new();
    let shared = Arc::new(Shared::default());
    let mut store = Journal::create(
        Storage {
            sqlite: SqliteBackend::create(&temp.db).unwrap(),
            shared: shared.clone(),
        },
        RecordCipher::new(Zeroizing::new([99; 32]), 1, [42; 32]).unwrap(),
        support::config(),
    )
    .unwrap();
    let binding = Binding {
        stream: Stream {
            domain: support::config().domain,
            id: [42; 32],
        },
        epoch: 1,
        api: [8; 32],
    };
    let reader = store.attach_reader(binding).unwrap();
    (temp, store, reader, Fresh { shared, binding })
}
fn release(reader: &Reader, witness: &Fresh) -> Result<(), Failure> {
    reader
        .verify(reader.capture()?, witness)?
        .release(Instant::now() + Duration::from_secs(1))
}
#[test]
fn publication_covers_accepted_rejected_and_duplicate_commits_without_extra_history() {
    let (_temp, mut store, reader, witness) = fixture();
    let tx = support::transaction(store.head(), 20, vec![], vec![]);
    let old = reader.verify(reader.capture().unwrap(), &witness).unwrap();
    let accepted = store.commit(tx.clone()).unwrap();
    assert_eq!(
        old.release(Instant::now() + Duration::from_secs(1)),
        Err(Failure::Raced)
    );
    let current = reader.verify(reader.capture().unwrap(), &witness).unwrap();
    assert_eq!(current.head(), accepted.head);
    assert_eq!(current.state().unwrap(), store.state().unwrap());
    assert!(store.commit(tx).unwrap().duplicate);
    assert!(
        current
            .release(Instant::now() + Duration::from_secs(1))
            .is_ok()
    );
    let rejected = store
        .commit(support::transaction(
            store.head(),
            21,
            vec![],
            vec![support::reserve(21, 1000)],
        ))
        .unwrap();
    assert!(rejected.receipt.controls.is_some());
    let v = reader.verify(reader.capture().unwrap(), &witness).unwrap();
    assert_eq!(v.head(), rejected.head);
    assert_eq!(
        v.transactions_with_receipts().last().unwrap().1,
        &rejected.receipt
    );
    assert_eq!(v.state().unwrap(), store.state().unwrap());
    assert_eq!(
        v.customer_book_history(support::user(1)).unwrap(),
        store.customer_book_history(support::user(1)).unwrap()
    );
}
#[test]
fn pre_cas_reads_remain_available_but_post_cas_pre_publication_is_a_known_race() {
    for mode in [1, 2] {
        let (_temp, mut store, reader, witness) = fixture();
        let tx = support::transaction(store.head(), 20, vec![], vec![]);
        let (entered, observed) = mpsc::channel();
        let (resume, wait) = mpsc::channel();
        *witness.shared.barrier.lock().unwrap() = Some((entered, wait));
        witness.shared.mode.store(mode, Ordering::SeqCst);
        std::thread::scope(|scope| {
            // SQLite's all-feature fault hook is intentionally not Send. Keep
            // the writer here; move only the independent reader to a thread.
            let r = &reader;
            let w = &witness;
            let read = scope.spawn(move || {
                observed.recv_timeout(Duration::from_secs(5)).unwrap();
                assert_eq!(
                    release(r, w),
                    if mode == 1 {
                        Ok(())
                    } else {
                        Err(Failure::Raced)
                    }
                );
                assert!(!r.fenced());
                resume.send(()).unwrap();
            });
            store.commit(tx).unwrap();
            read.join().unwrap();
        });
        assert!(release(&reader, &witness).is_ok());
    }
}
#[test]
fn uncertain_append_and_panics_in_append_or_freshness_invalidate_every_ticket() {
    for mode in [3, 5, 6] {
        let (_temp, mut store, reader, witness) = fixture();
        let old = reader.verify(reader.capture().unwrap(), &witness).unwrap();
        witness.shared.mode.store(mode, Ordering::SeqCst);
        let tx = support::transaction(store.head(), 20, vec![], vec![]);
        let attempted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if mode == 6 {
                store.verified_state().map(|_| ())
            } else {
                store.commit(tx).map(|_| ())
            }
        }));
        assert!(attempted.is_err() || attempted.unwrap().is_err());
        assert!(reader.fenced());
        assert_eq!(
            old.release(Instant::now() + Duration::from_secs(1)),
            Err(Failure::Fenced)
        );
        assert!(matches!(reader.capture(), Err(Failure::Fenced)));
        assert!(store.state().is_err());
        witness.shared.mode.store(0, Ordering::SeqCst);
        // Replay is not authority to revive this boot's old Reader.
        let _ = store.reload();
        assert!(reader.fenced());
        assert!(store.attach_reader(witness.binding).is_err());
    }
}
#[test]
fn known_refusal_keeps_old_view_and_each_read_requires_its_own_witness() {
    let (_temp, mut store, reader, witness) = fixture();
    witness.shared.mode.store(4, Ordering::SeqCst);
    assert_eq!(
        store
            .commit(support::transaction(store.head(), 20, vec![], vec![]))
            .unwrap_err(),
        Error::Busy
    );
    assert!(!reader.fenced());
    for _ in 0..3 {
        release(&reader, &witness).unwrap();
    }
    assert_eq!(witness.shared.reads.load(Ordering::SeqCst), 3);
}
#[test]
fn read_slots_are_bounded_shared_history_is_not_copied_and_drop_returns_capacity() {
    let (_temp, mut store, reader, witness) = fixture();
    store
        .commit(support::transaction(store.head(), 20, vec![], vec![]))
        .unwrap();
    let v = reader.verify(reader.capture().unwrap(), &witness).unwrap();
    assert!(std::ptr::eq(v.state().unwrap(), store.state().unwrap()));
    assert!(std::ptr::eq(
        v.transactions_with_receipts().next().unwrap().0,
        store.transactions_with_receipts().next().unwrap().0
    ));
    let mut tickets = (0..3)
        .map(|_| reader.capture().unwrap())
        .collect::<Vec<_>>();
    assert!(matches!(reader.clone().capture(), Err(Failure::Busy)));
    drop(tickets.pop());
    let t = reader.capture().unwrap();
    drop((v, tickets, t));
    release(&reader, &witness).unwrap();
    let old = reader.verify(reader.capture().unwrap(), &witness).unwrap();
    store
        .commit(support::transaction(store.head(), 21, vec![], vec![]))
        .unwrap();
    let new = reader.verify(reader.capture().unwrap(), &witness).unwrap();
    assert!(std::ptr::eq(
        old.transactions_with_receipts().next().unwrap().0,
        new.transactions_with_receipts().next().unwrap().0
    ));
    assert!(!std::ptr::eq(old.state().unwrap(), new.state().unwrap()));
    drop((old, new));
    // Retain four DISTINCT old State/index-source generations while the writer
    // keeps accepting. The cap is on tickets, not just same-generation clones.
    let mut generations = Vec::new();
    for n in 22..26 {
        generations.push(reader.verify(reader.capture().unwrap(), &witness).unwrap());
        store
            .commit(support::transaction(store.head(), n, vec![], vec![]))
            .unwrap();
    }
    assert!(matches!(reader.capture(), Err(Failure::Busy)));
    for pair in generations.windows(2) {
        assert!(!std::ptr::eq(
            pair[0].state().unwrap(),
            pair[1].state().unwrap()
        ));
        assert!(std::ptr::eq(
            pair[0].transactions_with_receipts().next().unwrap().0,
            pair[1].transactions_with_receipts().next().unwrap().0
        ));
    }
    for old in generations {
        assert_eq!(
            old.release(Instant::now() + Duration::from_secs(1)),
            Err(Failure::Raced)
        );
    }
    release(&reader, &witness).unwrap();
}
struct Other(Anchor);
impl Witness for Other {
    fn read(&self, _: Stream) -> Result<Anchor, Error> {
        Ok(self.0)
    }
}
#[test]
fn rollback_unknown_heads_epochs_and_missing_registers_are_fatal_not_local_races() {
    for case in 0..4 {
        let (_temp, mut store, reader, witness) = fixture();
        let old = store.head();
        store
            .commit(support::transaction(old, 20, vec![], vec![]))
            .unwrap();
        let current = store.head();
        let anchor = match case {
            0 => Anchor {
                epoch: 1,
                head: Some(old),
            },
            1 => Anchor {
                epoch: 1,
                head: Some(Head {
                    hash: [1; 32],
                    ..current
                }),
            },
            2 => Anchor {
                epoch: 2,
                head: Some(current),
            },
            _ => Anchor {
                epoch: 1,
                head: None,
            },
        };
        assert!(matches!(
            reader.verify(reader.capture().unwrap(), &Other(anchor)),
            Err(Failure::Fenced)
        ));
        assert!(reader.fenced());
        assert!(store.state().is_err());
        assert!(release(&reader, &witness).is_err());
    }
}
#[test]
fn deadline_or_generation_change_refuses_release_and_journal_drop_closes_read_boot() {
    let (_temp, mut store, reader, witness) = fixture();
    let v = reader.verify(reader.capture().unwrap(), &witness).unwrap();
    assert_eq!(v.release(Instant::now()), Err(Failure::Raced));
    let v = reader.verify(reader.capture().unwrap(), &witness).unwrap();
    store
        .commit(support::transaction(store.head(), 20, vec![], vec![]))
        .unwrap();
    assert_eq!(
        v.release(Instant::now() + Duration::from_secs(1)),
        Err(Failure::Raced)
    );
    let v = reader.verify(reader.capture().unwrap(), &witness).unwrap();
    assert_eq!(
        v.release_if_running(
            Instant::now() + Duration::from_secs(1),
            &std::sync::atomic::AtomicBool::new(true)
        ),
        Err(Failure::Fenced)
    );
    let v = reader.verify(reader.capture().unwrap(), &witness).unwrap();
    drop(store);
    assert_eq!(
        v.release(Instant::now() + Duration::from_secs(1)),
        Err(Failure::Fenced)
    );
}

#[test]
fn witness_panic_fences_other_verified_tickets() {
    struct Panicking;
    impl Witness for Panicking {
        fn read(&self, _: Stream) -> Result<Anchor, Error> {
            panic!("synthetic witness panic")
        }
    }
    let (_temp, _store, reader, witness) = fixture();
    let old = reader.verify(reader.capture().unwrap(), &witness).unwrap();
    let ticket = reader.capture().unwrap();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || reader.verify(ticket, &Panicking)
        ))
        .is_err()
    );
    assert!(reader.fenced());
    assert_eq!(
        old.release(Instant::now() + Duration::from_secs(1)),
        Err(Failure::Fenced)
    );
}
