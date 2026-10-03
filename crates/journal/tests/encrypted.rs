//! Real AEAD/files and synthetic independently trusted witness contract.
mod support;
use cinder_journal::{encrypted::RecordCipher, model::*, replicated::*, *};
use cinder_kernel::ledger::*;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use support::*;
use zeroize::Zeroizing;

fn stream() -> Stream {
    Stream {
        domain: config().domain,
        id: [9; 32],
    }
}
fn cipher() -> RecordCipher {
    RecordCipher::new(Zeroizing::new([42; 32]), 1, stream().id).unwrap()
}
#[derive(Clone, Default)]
struct MemoryReplica(Arc<Mutex<ReplicaState>>);
type ReplicaState = (BTreeMap<[u8; 32], Frame>, bool);
impl Replica for MemoryReplica {
    fn identity(&self) -> [u8; 32] {
        let mut bytes = [0; 32];
        bytes[..8].copy_from_slice(&(Arc::as_ptr(&self.0) as usize as u64).to_be_bytes());
        bytes
    }
    fn put(&mut self, frame: &Frame) -> Result<(), Error> {
        let mut s = self.0.lock().unwrap();
        if s.1 {
            return Err(Error::Storage);
        }
        s.0.insert(frame.head.hash, frame.clone());
        Ok(())
    }
    fn get(&mut self, hash: [u8; 32]) -> Result<Frame, Error> {
        let s = self.0.lock().unwrap();
        if s.1 {
            return Err(Error::Storage);
        }
        s.0.get(&hash).cloned().ok_or(Error::Storage)
    }
}
#[derive(Clone)]
struct TestWitness(Arc<Mutex<(Anchor, bool, bool)>>);
impl Default for TestWitness {
    fn default() -> Self {
        Self(Arc::new(Mutex::new((
            Anchor {
                epoch: 1,
                head: None,
            },
            false,
            false,
        ))))
    }
}
impl Witness for TestWitness {
    fn read(&mut self, s: Stream) -> Result<Anchor, Error> {
        if s != stream() {
            return Err(Error::Conflict);
        }
        let a = self.0.lock().unwrap();
        if a.1 {
            return Err(Error::Storage);
        }
        Ok(a.0)
    }
    fn accept(&mut self, s: Stream, expected: Anchor, next: Head) -> Result<(), Error> {
        if s != stream() {
            return Err(Error::Conflict);
        }
        let mut a = self.0.lock().unwrap();
        if a.1 {
            return Err(Error::Storage);
        }
        if a.0 != expected {
            return Err(Error::Stale);
        }
        a.0.head = Some(next);
        if a.2 {
            a.2 = false;
            return Err(Error::Storage);
        }
        Ok(())
    }
}
type MemoryStore = Journal<Replicated<MemoryReplica, MemoryReplica, TestWitness>, RecordCipher>;
fn backend(
    a: &MemoryReplica,
    b: &MemoryReplica,
    w: &TestWitness,
    epoch: u64,
) -> Replicated<MemoryReplica, MemoryReplica, TestWitness> {
    Replicated::new(stream(), epoch, a.clone(), b.clone(), w.clone()).unwrap()
}
fn create(a: &MemoryReplica, b: &MemoryReplica, w: &TestWitness) -> MemoryStore {
    Journal::create(backend(a, b, w, 1), cipher(), config()).unwrap()
}
fn credit(head: Head, n: u8) -> Transaction {
    transaction(
        head,
        n,
        vec![receipt(n as u64, Owner::Customer(user(1)), 100)],
        vec![],
    )
}

#[test]
fn aead_binds_all_context_key_generation_stream_and_every_byte() {
    let p = cipher();
    let context = RecordContext {
        domain: config().domain,
        sequence: 1,
        previous: [7; 32],
    };
    let msg = b"private financial state";
    let sealed = p.seal(context, msg).unwrap();
    assert_eq!(p.open(context, &sealed).unwrap().as_bytes(), msg);
    assert_ne!(p.seal(context, msg).unwrap(), sealed); // nonce not journal-sequence-derived
    assert!(!sealed.as_bytes().windows(msg.len()).any(|w| w == msg));
    for i in 0..sealed.as_bytes().len() {
        let mut bytes = sealed.as_bytes().to_vec();
        bytes[i] ^= 1;
        assert!(p.open(context, &raw(&bytes)).is_err());
    }
    for n in 0..sealed.as_bytes().len() {
        assert!(p.open(context, &raw(&sealed.as_bytes()[..n])).is_err());
    }
    let mut contexts = [context; 4];
    contexts[0].sequence += 1;
    contexts[1].previous[0] ^= 1;
    contexts[2].domain.network = cinder_kernel::identity::NetworkId::new([99; 32]).unwrap();
    contexts[3].domain.deployment = cinder_kernel::identity::DeploymentId::new([99; 32]).unwrap();
    for c in contexts {
        assert!(p.open(c, &sealed).is_err());
    }
    for (key, generation, id) in [
        ([43; 32], 1, stream().id),
        ([42; 32], 2, stream().id),
        ([42; 32], 1, [8; 32]),
    ] {
        assert!(
            RecordCipher::new(Zeroizing::new(key), generation, id)
                .unwrap()
                .open(context, &sealed)
                .is_err()
        );
    }
    assert!(
        p.seal(context, &vec![0; cinder_journal::wire::MAX_RECORD])
            .is_err()
    );
}

#[test]
fn one_replica_recovers_but_both_missing_corrupt_or_stale_cannot_roll_back() {
    let (a, b, w) = (
        MemoryReplica::default(),
        MemoryReplica::default(),
        TestWitness::default(),
    );
    let mut j = create(&a, &b, &w);
    let genesis = a.0.lock().unwrap().0.clone();
    j.commit(credit(j.head(), 1)).unwrap();
    let expected = j.state().unwrap().clone();
    a.0.lock().unwrap().0 = genesis.clone(); // entire first replica stale
    assert_eq!(
        Journal::open(backend(&a, &b, &w, 1), cipher(), config())
            .unwrap()
            .state()
            .unwrap(),
        &expected
    );
    a.0.lock().unwrap().1 = true; // entire directory unavailable
    assert_eq!(
        Journal::open(backend(&a, &b, &w, 1), cipher(), config())
            .unwrap()
            .state()
            .unwrap(),
        &expected
    );
    b.0.lock().unwrap().0 = genesis;
    assert!(Journal::open(backend(&a, &b, &w, 1), cipher(), config()).is_err());
    assert!(j.commit(credit(j.head(), 2)).is_err());
    assert!(j.state().is_err());
}

#[test]
fn two_copies_precede_acceptance_orphans_do_not_strand_same_sequence() {
    let (a, b, w) = (
        MemoryReplica::default(),
        MemoryReplica::default(),
        TestWitness::default(),
    );
    let mut j = create(&a, &b, &w);
    let before = w.0.lock().unwrap().0;
    b.0.lock().unwrap().1 = true;
    assert_eq!(j.commit(credit(j.head(), 1)).unwrap_err(), Error::Storage);
    assert_eq!(w.0.lock().unwrap().0, before);
    assert_eq!(a.0.lock().unwrap().0.len(), 2); // unaccepted proposal
    b.0.lock().unwrap().1 = false;
    let mut recovered = Journal::open(backend(&a, &b, &w, 1), cipher(), config()).unwrap();
    assert!(recovered.state().unwrap().ledger().events().is_empty());
    recovered.commit(credit(recovered.head(), 2)).unwrap();
    assert_eq!(recovered.state().unwrap().ledger().events().len(), 1);
    assert_eq!(a.0.lock().unwrap().0.len(), 3); // orphan is retained, not replayed
}

#[test]
fn lost_witness_reply_replays_once_and_never_redelivers_exposure() {
    let (a, b, w) = (
        MemoryReplica::default(),
        MemoryReplica::default(),
        TestWitness::default(),
    );
    let mut j = create(&a, &b, &w);
    let tx = transaction(
        j.head(),
        1,
        vec![receipt(1, Owner::Customer(user(1)), 100)],
        vec![reserve(2, 80), prepare(2), Control::Expose(attempt(2))],
    );
    w.0.lock().unwrap().2 = true;
    assert_eq!(j.commit(tx.clone()).unwrap_err(), Error::Storage);
    assert!(j.state().is_err());
    let mut recovered = Journal::open(backend(&a, &b, &w, 1), cipher(), config()).unwrap();
    let duplicate = recovered.commit(tx).unwrap();
    assert!(duplicate.duplicate && duplicate.exposures.is_empty());
    assert!(recovered.state().unwrap().attempts()[0].possibly_exposed);
    assert!(recovered.state().unwrap().holds()[0].active);
}

#[test]
fn new_acceptance_repairs_old_history_before_a_second_replica_loss() {
    let (a, b, w) = (
        MemoryReplica::default(),
        MemoryReplica::default(),
        TestWitness::default(),
    );
    let mut j = create(&a, &b, &w);
    j.commit(credit(j.head(), 1)).unwrap();
    a.0.lock().unwrap().0.clear();
    j.commit(credit(j.head(), 2)).unwrap();
    assert_eq!(a.0.lock().unwrap().0.len(), b.0.lock().unwrap().0.len());
    let expected = j.state().unwrap().clone();
    b.0.lock().unwrap().0.clear();
    assert_eq!(
        Journal::open(backend(&a, &b, &w, 1), cipher(), config())
            .unwrap()
            .state()
            .unwrap(),
        &expected
    );
}

#[test]
fn read_can_recover_from_one_copy_but_failed_historical_repair_blocks_acceptance() {
    // Current writes succeed while this adapter refuses to restore old objects.
    struct RefuseOld {
        inner: MemoryReplica,
        before: u64,
    }
    impl Replica for RefuseOld {
        fn identity(&self) -> [u8; 32] {
            self.inner.identity()
        }
        fn get(&mut self, hash: [u8; 32]) -> Result<Frame, Error> {
            self.inner.get(hash)
        }
        fn put(&mut self, f: &Frame) -> Result<(), Error> {
            if f.head.sequence < self.before {
                return Err(Error::Storage);
            }
            self.inner.put(f)
        }
    }
    let (a, b, w) = (
        MemoryReplica::default(),
        MemoryReplica::default(),
        TestWitness::default(),
    );
    let mut j = create(&a, &b, &w);
    j.commit(credit(j.head(), 1)).unwrap();
    drop(j);
    let before = w.0.lock().unwrap().0;
    a.0.lock().unwrap().0.clear();
    let backend = Replicated::new(
        stream(),
        1,
        RefuseOld {
            inner: a,
            before: 2,
        },
        b,
        w.clone(),
    )
    .unwrap();
    let mut recovered = Journal::open(backend, cipher(), config()).unwrap();
    assert_eq!(recovered.state().unwrap().ledger().events().len(), 1);
    assert_eq!(
        recovered.commit(credit(recovered.head(), 2)).unwrap_err(),
        Error::Storage
    );
    assert_eq!(w.0.lock().unwrap().0, before);
}

#[test]
fn ambiguous_witness_busy_is_not_a_known_prewrite_lock_failure() {
    struct BusyReply(TestWitness);
    impl Witness for BusyReply {
        fn read(&mut self, s: Stream) -> Result<Anchor, Error> {
            self.0.read(s)
        }
        fn accept(&mut self, s: Stream, expected: Anchor, next: Head) -> Result<(), Error> {
            self.0.accept(s, expected, next)?;
            Err(Error::Busy)
        }
    }
    let (a, b, w) = (
        MemoryReplica::default(),
        MemoryReplica::default(),
        TestWitness::default(),
    );
    drop(create(&a, &b, &w));
    let wrapped = Replicated::new(stream(), 1, a.clone(), b.clone(), BusyReply(w.clone())).unwrap();
    let mut j = Journal::open(wrapped, cipher(), config()).unwrap();
    let tx = credit(j.head(), 1);
    assert_eq!(j.commit(tx.clone()).unwrap_err(), Error::Storage);
    assert_eq!(j.state().unwrap_err(), Error::Poisoned);
    let mut recovered = Journal::open(backend(&a, &b, &w, 1), cipher(), config()).unwrap();
    assert!(recovered.commit(tx).unwrap().duplicate);
}

#[test]
fn fenced_writer_cannot_ack_duplicate_or_new_action_with_loaded_key() {
    let (a, b, w) = (
        MemoryReplica::default(),
        MemoryReplica::default(),
        TestWitness::default(),
    );
    let mut old = create(&a, &b, &w);
    let tx = credit(old.head(), 1);
    old.commit(tx.clone()).unwrap();
    w.0.lock().unwrap().0.epoch = 2; // explicit operator act in test witness
    assert_eq!(old.commit(tx).unwrap_err(), Error::Stale);
    assert!(old.state().is_err());
    assert!(Journal::open(backend(&a, &b, &w, 1), cipher(), config()).is_err());
    let mut new = Journal::open(backend(&a, &b, &w, 2), cipher(), config()).unwrap();
    new.commit(credit(new.head(), 2)).unwrap();
    w.0.lock().unwrap().1 = true;
    assert_eq!(new.verified_state().unwrap_err(), Error::Storage);
}

#[test]
fn two_current_epoch_writers_cannot_commit_diverging_histories() {
    let (a, b, w) = (
        MemoryReplica::default(),
        MemoryReplica::default(),
        TestWitness::default(),
    );
    let mut j = create(&a, &b, &w);
    let mut other = Journal::open(backend(&a, &b, &w, 1), cipher(), config()).unwrap();
    j.commit(credit(j.head(), 1)).unwrap();
    assert_eq!(
        other.commit(credit(other.head(), 2)).unwrap_err(),
        Error::Stale
    );
    other.reload().unwrap();
    assert_eq!(j.state().unwrap(), other.state().unwrap());
}

#[test]
fn ciphertext_snapshot_rehydrates_but_never_supersedes_witness() {
    let (a, b, w) = (
        MemoryReplica::default(),
        MemoryReplica::default(),
        TestWitness::default(),
    );
    let mut j = create(&a, &b, &w);
    let old = backend(&a, &b, &w, 1).snapshot().unwrap();
    j.commit(credit(j.head(), 1)).unwrap();
    let snapshot = backend(&a, &b, &w, 1).snapshot().unwrap();
    let expected = j.state().unwrap().clone();
    a.0.lock().unwrap().0.clear();
    b.0.lock().unwrap().0.clear();
    assert_eq!(
        backend(&a, &b, &w, 1).restore_snapshot(&old),
        Err(Error::Stale)
    );
    backend(&a, &b, &w, 1).restore_snapshot(&snapshot).unwrap();
    assert_eq!(
        Journal::open(backend(&a, &b, &w, 1), cipher(), config())
            .unwrap()
            .state()
            .unwrap(),
        &expected
    );
    for n in [0, 16, 100, snapshot.len() - 1] {
        assert!(
            backend(&a, &b, &w, 1)
                .restore_snapshot(&snapshot[..n])
                .is_err()
        );
    }
    let mut changed = snapshot.clone();
    changed[20] ^= 1;
    assert!(backend(&a, &b, &w, 1).restore_snapshot(&changed).is_err());
    changed = snapshot.clone();
    changed.push(0);
    assert!(backend(&a, &b, &w, 1).restore_snapshot(&changed).is_err());
}

#[test]
fn current_snapshot_restore_reuses_valid_create_only_objects() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[derive(Clone)]
    struct CreateOnly(MemoryReplica, Arc<AtomicUsize>);
    impl Replica for CreateOnly {
        fn identity(&self) -> [u8; 32] {
            self.0.identity()
        }
        fn get(&mut self, hash: [u8; 32]) -> Result<Frame, Error> {
            self.0.get(hash)
        }
        fn put(&mut self, frame: &Frame) -> Result<(), Error> {
            if self.0.0.lock().unwrap().0.contains_key(&frame.head.hash) {
                self.1.fetch_add(1, Ordering::SeqCst);
                return Err(Error::Storage);
            }
            self.0.put(frame)
        }
    }
    let a = CreateOnly(MemoryReplica::default(), Arc::new(AtomicUsize::new(0)));
    let b = CreateOnly(MemoryReplica::default(), Arc::new(AtomicUsize::new(0)));
    let w = TestWitness::default();
    let make = || Replicated::new(stream(), 1, a.clone(), b.clone(), w.clone()).unwrap();
    let mut journal = Journal::create(make(), cipher(), config()).unwrap();
    let stale = make().snapshot().unwrap();
    journal.commit(credit(journal.head(), 1)).unwrap();
    let current = make().snapshot().unwrap();
    let accepted = w.0.lock().unwrap().0;
    assert_eq!(make().restore_snapshot(&stale), Err(Error::Stale));
    make().restore_snapshot(&current).unwrap();
    // Restore one missing replica without rewriting the other valid copy.
    a.0.0.lock().unwrap().0.clear();
    make().restore_snapshot(&current).unwrap();
    assert_eq!(a.1.load(Ordering::SeqCst), 0);
    assert_eq!(b.1.load(Ordering::SeqCst), 0);
    assert_eq!(w.0.lock().unwrap().0, accepted);
    let restored = Journal::open(make(), cipher(), config()).unwrap();
    assert_eq!(restored.state().unwrap(), journal.state().unwrap());
    assert_eq!(restored.head(), journal.head());
}

#[test]
fn actual_host_files_have_ciphertext_only_and_wrong_key_stops_replay() {
    let temp = Temp::new();
    let a = temp.root.join("a");
    let b = temp.root.join("b");
    let w = TestWitness::default();
    let mut j = Journal::create(
        Replicated::new(
            stream(),
            1,
            FileReplica::create(&a).unwrap(),
            FileReplica::create(&b).unwrap(),
            w.clone(),
        )
        .unwrap(),
        cipher(),
        config(),
    )
    .unwrap();
    j.commit(credit(j.head(), 1)).unwrap();
    for directory in [&a, &b] {
        for path in std::fs::read_dir(directory).unwrap() {
            let bytes = std::fs::read(path.unwrap().path()).unwrap();
            for private in [b"synthetic-native-evidence".as_slice(), b"CINDER-J\0"] {
                assert!(!bytes.windows(private.len()).any(|w| w == private));
            }
        }
    }
    let wrong = RecordCipher::new(Zeroizing::new([0; 32]), 1, stream().id).unwrap();
    assert!(
        Journal::open(
            Replicated::new(
                stream(),
                1,
                FileReplica::open(&a).unwrap(),
                FileReplica::open(&b).unwrap(),
                w
            )
            .unwrap(),
            wrong,
            config()
        )
        .is_err()
    );
}

#[test]
fn aliased_replicas_cannot_satisfy_two_copy_acceptance() {
    let a = MemoryReplica::default();
    assert!(Replicated::new(stream(), 1, a.clone(), a, TestWitness::default()).is_err());
    let temp = Temp::new();
    let path = temp.root.join("copy");
    let first = FileReplica::create(&path).unwrap();
    let second = FileReplica::open(&path.join(".")).unwrap();
    assert!(Replicated::new(stream(), 1, first, second, TestWitness::default()).is_err());
}

#[test]
fn simultaneous_prepared_writers_commit_only_one_witness_successor() {
    #[derive(Clone)]
    struct Racing(TestWitness, Arc<std::sync::Barrier>);
    impl Witness for Racing {
        fn read(&mut self, s: Stream) -> Result<Anchor, Error> {
            self.0.read(s)
        }
        fn accept(&mut self, s: Stream, a: Anchor, h: Head) -> Result<(), Error> {
            self.1.wait();
            self.0.accept(s, a, h)
        }
    }
    let (a, b, w) = (
        MemoryReplica::default(),
        MemoryReplica::default(),
        TestWitness::default(),
    );
    drop(create(&a, &b, &w));
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = (1..=2)
        .map(|n| {
            let (a, b, w, barrier) = (a.clone(), b.clone(), w.clone(), barrier.clone());
            std::thread::spawn(move || {
                let store = Replicated::new(stream(), 1, a, b, Racing(w, barrier)).unwrap();
                let mut j = Journal::open(store, cipher(), config()).unwrap();
                j.commit(credit(j.head(), n))
            })
        })
        .collect();
    let outcomes: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|r| matches!(r, Err(Error::Stale)))
            .count(),
        1
    );
    let restored = Journal::open(backend(&a, &b, &w, 1), cipher(), config()).unwrap();
    assert_eq!(restored.state().unwrap().ledger().events().len(), 1);
}
