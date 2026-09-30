//! Real killed-child SQLite tests, not a model of host/power-loss reliability.
#![cfg(feature = "test-hooks")]
mod support;
use cinder_journal::{model::*, sqlite::*, *};
use cinder_kernel::ledger::*;
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Duration,
};
use support::*;

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn process_kill_preserves_all_or_none_postings_holds_and_consumed_ids() {
    kill_matrix(false);
}
#[test]
fn process_kill_after_exposure_commit_never_reissues_the_action() {
    kill_matrix(true);
}
fn kill_matrix(exposure_only: bool) {
    for point in [
        "BeforeBegin",
        "AfterInsert",
        "AfterHead",
        "BeforeCommit",
        "AfterCommit",
    ] {
        let temp = Temp::new();
        let mut s = temp.create();
        if exposure_only {
            seed(&mut s);
            hold(&mut s);
        }
        let old = s.state().unwrap().clone();
        let old_head = s.head();
        drop(s);
        let child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "crash_child_worker", "--ignored", "--nocapture"])
            .env_clear()
            .env("P05_TEST_DB", &temp.db)
            .env("P05_TEST_POINT", point)
            .env("P05_TEST_EXPOSURE", if exposure_only { "1" } else { "0" })
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
                if line.unwrap().contains("P05_KILL_READY") {
                    let _ = send.send(());
                    return;
                }
            }
        });
        recv.recv_timeout(Duration::from_secs(15))
            .expect("child did not reach the specified commit boundary");
        child.0.kill().unwrap();
        let status = child.0.wait().unwrap();
        assert!(!status.success());
        reader.join().unwrap();
        let mut s = temp.open();
        let committed = point == "AfterCommit";
        if !committed {
            assert_eq!(s.state().unwrap(), &old);
            assert_eq!(s.head(), old_head);
        } else {
            assert_eq!(s.head().sequence, old_head.sequence + 1);
            assert_eq!(s.state().unwrap().holds().len(), 1);
            assert!(s.state().unwrap().attempts()[0].possibly_exposed);
            let tx = child_transaction(old_head, exposure_only);
            let duplicate = s.commit(tx).unwrap();
            assert!(duplicate.duplicate);
            assert!(duplicate.exposures.is_empty());
            if !exposure_only {
                assert_eq!(s.state().unwrap().ledger().events().len(), 1);
                assert_eq!(
                    s.state()
                        .unwrap()
                        .ledger()
                        .book(Owner::Customer(user(1)))
                        .unwrap()
                        .cash(),
                    cash(100)
                );
            }
        }
    }
}
fn child_transaction(head: Head, exposure_only: bool) -> Transaction {
    if exposure_only {
        transaction(head, 3, vec![], vec![Control::Expose(attempt(2))])
    } else {
        transaction(
            head,
            3,
            vec![receipt(1, Owner::Customer(user(1)), 100)],
            vec![reserve(2, 80), prepare(2), Control::Expose(attempt(2))],
        )
    }
}
#[test]
#[ignore = "invoked explicitly by the parent crash matrix; intentionally killed"]
fn crash_child_worker() {
    let path = std::path::PathBuf::from(std::env::var_os("P05_TEST_DB").expect("test-only child"));
    let point = std::env::var("P05_TEST_POINT").unwrap();
    let exposure_only = std::env::var("P05_TEST_EXPOSURE").unwrap() == "1";
    let mut b = SqliteBackend::open(&path, Migration::None).unwrap();
    b.set_test_hook(move |p| {
        if format!("{p:?}") == point {
            println!("P05_KILL_READY");
            std::io::stdout().flush().unwrap();
            let mut line = String::new();
            std::io::stdin().read_line(&mut line).unwrap();
            panic!("parent must kill the process before allowing further work");
        }
        Ok(())
    });
    let mut s = Journal::open(b, FixtureProtection, config()).unwrap();
    s.commit(child_transaction(s.head(), exposure_only))
        .unwrap();
    panic!("test boundary was not exercised");
}
