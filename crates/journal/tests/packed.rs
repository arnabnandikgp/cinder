//! Local candidate: real AEAD, ciphertext-only files, synthetic trusted witness.
mod support;
use cinder_journal::{
    encrypted::RecordCipher,
    packed::{FileStore, Packed, Store},
    replicated::*,
    *,
};
use cinder_kernel::ledger::Owner;
use std::{
    fs,
    sync::{Arc, Mutex},
};
use support::*;
use zeroize::Zeroizing;
#[derive(Clone)]
struct Trust(Arc<Mutex<(Anchor, bool)>>);
impl Default for Trust {
    fn default() -> Self {
        Self(Arc::new(Mutex::new((
            Anchor {
                epoch: 1,
                head: None,
            },
            false,
        ))))
    }
}
impl Witness for Trust {
    fn read(&mut self, s: Stream) -> Result<Anchor, Error> {
        if s != stream() {
            return Err(Error::Conflict);
        }
        Ok(self.0.lock().unwrap().0)
    }
    fn accept(&mut self, s: Stream, expected: Anchor, next: Head) -> Result<(), Error> {
        if s != stream() {
            return Err(Error::Conflict);
        }
        let mut a = self.0.lock().unwrap();
        if a.0 != expected {
            return Err(Error::Stale);
        }
        a.0.head = Some(next);
        if a.1 {
            a.1 = false;
            return Err(Error::Storage);
        }
        Ok(())
    }
}
fn stream() -> Stream {
    Stream {
        domain: config().domain,
        id: [9; 32],
    }
}
fn cipher() -> RecordCipher {
    RecordCipher::new(Zeroizing::new([42; 32]), 1, stream().id).unwrap()
}
fn packs(root: &std::path::Path, w: &Trust, epoch: u64) -> Packed<FileStore, FileStore, Trust> {
    Packed::new(
        stream(),
        epoch,
        FileStore::open(&root.join("packs-a")).unwrap(),
        FileStore::open(&root.join("packs-b")).unwrap(),
        w.clone(),
    )
    .unwrap()
}
#[test]
fn legacy_archive_explicitly_imports_with_identical_aead_replay_no_new_head_and_fenced_old_writer()
{
    let temp = Temp::new();
    let w = Trust::default();
    let backend = Replicated::new(
        stream(),
        1,
        FileReplica::create(&temp.root.join("frames-a")).unwrap(),
        FileReplica::create(&temp.root.join("frames-b")).unwrap(),
        w.clone(),
    )
    .unwrap();
    let mut old = Journal::create(backend, cipher(), config()).unwrap();
    for n in 1..=32 {
        old.commit(transaction(
            old.head(),
            n,
            vec![receipt(n as u64, Owner::Customer(user(1)), 1)],
            vec![],
        ))
        .unwrap();
    }
    let head = old.head();
    let state = old.state().unwrap().clone();
    let mut archive_store = Replicated::new(
        stream(),
        1,
        FileReplica::open(&temp.root.join("frames-a")).unwrap(),
        FileReplica::open(&temp.root.join("frames-b")).unwrap(),
        w.clone(),
    )
    .unwrap();
    let archive = archive_store.snapshot().unwrap();
    FileStore::create(&temp.root.join("packs-a")).unwrap();
    FileStore::create(&temp.root.join("packs-b")).unwrap();
    // Explicit trusted-epoch cut; not an automatic on-boot migration.
    w.0.lock().unwrap().0.epoch = 2;
    let mut candidate = packs(&temp.root, &w, 2);
    candidate.restore_snapshot(&archive).unwrap();
    assert_eq!(candidate.snapshot().unwrap(), archive);
    assert_eq!(w.0.lock().unwrap().0.head, Some(head));
    assert!(old.commit(transaction(head, 33, vec![], vec![])).is_err());
    assert!(old.state().is_err());
    let fresh = Journal::open(candidate, cipher(), config()).unwrap();
    assert_eq!(fresh.state().unwrap(), &state);
    assert_eq!(fresh.head(), head);
    assert_eq!(fresh.transactions().count(), 32);
    for n in 1..=32 {
        assert_eq!(
            fresh.transaction_receipt(cinder_journal::model::CommitId::new([n; 32]).unwrap()),
            old.transaction_receipt(cinder_journal::model::CommitId::new([n; 32]).unwrap())
        );
    }
    assert!(
        Journal::open(
            packs(&temp.root, &w, 2),
            RecordCipher::new(Zeroizing::new([43; 32]), 1, stream().id).unwrap(),
            config()
        )
        .is_err()
    );
}
#[test]
fn unknown_acceptance_poisons_then_original_receipt_survives_file_reopen_without_resend() {
    let temp = Temp::new();
    let w = Trust::default();
    FileStore::create(&temp.root.join("packs-a")).unwrap();
    FileStore::create(&temp.root.join("packs-b")).unwrap();
    let mut j = Journal::create(packs(&temp.root, &w, 1), cipher(), config()).unwrap();
    let tx = transaction(
        j.head(),
        1,
        vec![receipt(1, Owner::Customer(user(1)), 100)],
        vec![],
    );
    w.0.lock().unwrap().1 = true;
    assert_eq!(j.commit(tx.clone()).unwrap_err(), Error::Storage);
    assert!(j.state().is_err());
    let reopened = Journal::open(packs(&temp.root, &w, 1), cipher(), config()).unwrap();
    assert_eq!(reopened.head().sequence, 1);
    assert_eq!(reopened.transactions().count(), 1);
    assert_eq!(reopened.transaction(tx.id), Some(&tx));
    assert!(reopened.transaction_receipt(tx.id).is_some());
    assert_eq!(
        reopened
            .state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(user(1)))
            .unwrap()
            .cash()
            .atoms(),
        100
    );
}
#[test]
fn local_object_port_is_bounded_and_refuses_symlink_or_public_directory() {
    use cinder_journal::packed::MAX_PACK_BYTES;
    let temp = Temp::new();
    let dir = temp.root.join("port");
    let mut port = FileStore::create(&dir).unwrap();
    assert!(FileStore::create(&dir).is_err());
    assert!(port.put([1; 32], &[]).is_err());
    assert!(port.put([1; 32], &vec![0; MAX_PACK_BYTES + 1]).is_err());
    let file = dir.join("02".repeat(32));
    fs::File::create(&file)
        .unwrap()
        .set_len(MAX_PACK_BYTES as u64 + 1)
        .unwrap();
    assert_eq!(port.get([2; 32]).unwrap_err(), Error::Limit);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{PermissionsExt, symlink};
        symlink(file, dir.join("03".repeat(32))).unwrap();
        assert!(port.get([3; 32]).is_err());
        let link = temp.root.join("alias");
        symlink(&dir, &link).unwrap();
        assert!(FileStore::open(&link).is_err());
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(FileStore::open(&dir).is_err());
    }
}
