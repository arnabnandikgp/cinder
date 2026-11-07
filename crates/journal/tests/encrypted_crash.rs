//! Actual process kills with a LOCAL trusted SQLite witness test double. This is
//! not witness independence, Nitro entropy/key-release or remote durability proof.
mod support;
use cinder_journal::{encrypted::RecordCipher, model::*, replicated::*, *};
use cinder_kernel::ledger::*;
use rusqlite::{Connection, TransactionBehavior, params};
use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Duration,
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
fn boundary(point: &str) {
    if std::env::var("P06_TEST_POINT").ok().as_deref() == Some(point) {
        println!("P06_KILL_READY");
        std::io::stdout().flush().unwrap();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        panic!("parent must kill child before proceeding");
    }
}
struct DiskWitness(Connection);
impl DiskWitness {
    fn open(root: &Path, create: bool) -> Self {
        let c = Connection::open(root.join("witness.sqlite3")).unwrap();
        c.execute_batch("PRAGMA synchronous=EXTRA; PRAGMA fullfsync=ON;")
            .unwrap();
        if create {
            c.execute_batch("CREATE TABLE anchor(id INTEGER PRIMARY KEY,epoch INTEGER NOT NULL,seq INTEGER,digest BLOB); INSERT INTO anchor VALUES(1,1,NULL,NULL);").unwrap();
        }
        Self(c)
    }
    fn value(c: &Connection) -> Anchor {
        c.query_row(
            "SELECT epoch,seq,digest FROM anchor WHERE id=1",
            [],
            |row| {
                let epoch: i64 = row.get(0)?;
                let seq: Option<i64> = row.get(1)?;
                let digest: Option<Vec<u8>> = row.get(2)?;
                Ok(Anchor {
                    epoch: epoch.try_into().unwrap(),
                    head: seq.map(|sequence| Head {
                        sequence: sequence.try_into().unwrap(),
                        hash: digest.unwrap().try_into().unwrap(),
                    }),
                })
            },
        )
        .unwrap()
    }
    fn fence(&mut self) {
        self.0
            .execute("UPDATE anchor SET epoch=epoch+1 WHERE id=1", [])
            .unwrap();
    }
}
impl Witness for DiskWitness {
    fn read(&mut self, s: Stream) -> Result<Anchor, Error> {
        assert_eq!(s, stream());
        Ok(Self::value(&self.0))
    }
    fn accept(&mut self, s: Stream, expected: Anchor, next: Head) -> Result<(), Error> {
        assert_eq!(s, stream());
        boundary("BeforeWitness");
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        if Self::value(&tx) != expected {
            return Err(Error::Stale);
        }
        tx.execute(
            "UPDATE anchor SET seq=?1,digest=?2 WHERE id=1",
            params![i64::try_from(next.sequence).unwrap(), &next.hash[..]],
        )
        .unwrap();
        tx.commit().unwrap();
        boundary("AfterWitness");
        Ok(())
    }
}
struct HookReplica(FileReplica, &'static str);
impl Replica for HookReplica {
    fn identity(&self) -> [u8; 32] {
        self.0.identity()
    }
    fn put(&mut self, f: &Frame) -> Result<(), Error> {
        self.0.put(f)?;
        boundary(self.1);
        Ok(())
    }
    fn get(&mut self, h: [u8; 32]) -> Result<Frame, Error> {
        self.0.get(h)
    }
}
fn backend(
    root: &Path,
    epoch: u64,
    create: bool,
) -> Replicated<HookReplica, HookReplica, DiskWitness> {
    let replica = |name| {
        let path = root.join(name);
        if create {
            FileReplica::create(&path)
        } else {
            FileReplica::open(&path)
        }
        .unwrap()
    };
    Replicated::new(
        stream(),
        epoch,
        HookReplica(replica("a"), "AfterFirst"),
        HookReplica(replica("b"), "AfterSecond"),
        DiskWitness::open(root, create),
    )
    .unwrap()
}
fn tx(head: Head) -> Transaction {
    transaction(
        head,
        1,
        vec![receipt(1, Owner::Customer(user(1)), 100)],
        vec![reserve(2, 80), prepare(2), Control::Expose(attempt(2))],
    )
}
struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[test]
fn killed_replication_and_witness_boundaries_restore_only_the_accepted_tail() {
    for point in ["AfterFirst", "AfterSecond", "BeforeWitness", "AfterWitness"] {
        let temp = Temp::new();
        let initial = Journal::create(backend(&temp.root, 1, true), cipher(), config()).unwrap();
        let head = initial.head();
        drop(initial);
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "encrypted_crash_worker",
                "--ignored",
                "--nocapture",
            ])
            .env_clear()
            .env("P06_TEST_ROOT", &temp.root)
            .env("P06_TEST_POINT", point)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut child = ChildGuard(child);
        let stdout = child.0.stdout.take().unwrap();
        let (send, recv) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                if line.unwrap().contains("P06_KILL_READY") {
                    let _ = send.send(());
                    return;
                }
            }
        });
        recv.recv_timeout(Duration::from_secs(15))
            .expect("child missed boundary");
        child.0.kill().unwrap();
        assert!(!child.0.wait().unwrap().success());
        reader.join().unwrap();
        DiskWitness::open(&temp.root, false).fence();
        assert!(Journal::open(backend(&temp.root, 1, false), cipher(), config()).is_err());
        let mut restored =
            Journal::open(backend(&temp.root, 2, false), cipher(), config()).unwrap();
        if point == "AfterWitness" {
            assert_eq!(restored.state().unwrap().ledger().events().len(), 1);
            assert!(restored.state().unwrap().attempts()[0].possibly_exposed);
            assert!(restored.commit(tx(head)).unwrap().exposures.is_empty());
        } else {
            assert_eq!(restored.head(), head);
            assert!(restored.state().unwrap().ledger().events().is_empty());
            // Reusing the same sequence after unaccepted orphan objects is safe.
            assert_eq!(restored.commit(tx(head)).unwrap().exposures.len(), 1);
        }
    }
}
#[test]
#[ignore = "explicitly invoked and killed by the parent boundary matrix"]
fn encrypted_crash_worker() {
    let root = PathBuf::from(std::env::var_os("P06_TEST_ROOT").unwrap());
    let mut j = Journal::open(backend(&root, 1, false), cipher(), config()).unwrap();
    j.commit(tx(j.head())).unwrap();
    panic!("boundary was not reached");
}
