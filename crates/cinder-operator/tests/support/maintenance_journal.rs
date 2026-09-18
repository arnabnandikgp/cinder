use super::*;
use crate::funding_maintenance::tests::{observation, Fixture};
use crate::{BoundedIntent, OrderIdentity, OrderKind};
use std::path::Path;

fn tempdir() -> tempfile::TempDir {
    let mut builder = tempfile::Builder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    builder.tempdir().unwrap()
}

fn journal(path: &Path, f: &Fixture) -> Journal {
    let mut j = Journal::open(path).unwrap();
    j.bind_runtime([9; 32]).unwrap();
    j.remember_users(
        &f.users
            .iter()
            .map(|(key, l)| (*key, l.user.to_bytes()))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    j
}
fn prepared(path: &Path, lots: [i64; 2]) -> (Journal, Fixture, FundingEpochPlan) {
    let flat = Fixture::new([0, 0], 0, 1);
    let mut j = journal(path, &flat);
    j.initialize_funding_checkpoint(&flat.checkpoint()).unwrap();
    let mut f = Fixture::new(lots, 0, 1);
    let old = f.checkpoint();
    j.rebase_funding_inventory(&old).unwrap();
    f.advance(100, 2);
    let plan = f.plan(&old);
    j.prepare_funding_epoch(&plan).unwrap();
    (j, f, plan)
}
fn apply(j: &mut Journal, plan: &FundingEpochPlan, scope: [u8; 32], signature: [u8; 64]) {
    j.record_funding_epoch_attempt(plan.epoch(), scope, signature, 999)
        .unwrap();
    let o = observation(plan, scope, signature, true);
    j.observe_funding_epoch_step(&o).unwrap();
}
fn aligned(f: &mut Fixture, plan: &FundingEpochPlan) -> FundingCheckpoint {
    f.book.funding_epoch = plan.epoch();
    for (_, l) in &mut f.users {
        l.last_funding_epoch = plan.epoch();
    }
    f.checkpoint()
}

#[test]
fn bootstrap_is_bound_flat_only_idempotent_and_migrations_do_not_guess() {
    let dir = tempdir();
    let path = dir.path().join("journal.sqlite");
    let flat = Fixture::new([0, 0], 0, 1);
    let cp = flat.checkpoint();
    let mut unbound = Journal::open(&path).unwrap();
    assert!(unbound.initialize_funding_checkpoint(&cp).is_err());
    drop(unbound);
    let mut j = journal(&path, &flat);
    assert!(j.funding_checkpoint().unwrap().is_none());
    assert!(j
        .initialize_funding_checkpoint(&Fixture::new([10, -10], 0, 1).checkpoint())
        .is_err());
    j.initialize_funding_checkpoint(&cp).unwrap();
    j.initialize_funding_checkpoint(&cp).unwrap();
    drop(j);
    let j = Journal::open(&path).unwrap();
    assert_eq!(j.funding_checkpoint().unwrap(), Some(cp));
}

#[test]
fn rate_or_generation_changes_cannot_be_erased_by_inventory_rebase() {
    let dir = tempdir();
    let flat = Fixture::new([0, 0], 0, 1);
    let mut j = journal(&dir.path().join("journal.sqlite"), &flat);
    j.initialize_funding_checkpoint(&flat.checkpoint()).unwrap();
    let mut f = Fixture::new([10, -4], 100, 2);
    assert!(j.rebase_funding_inventory(&f.checkpoint()).is_err());
    f.advance(0, 2); // accumulator canceled, generation still crossed
    assert!(j.rebase_funding_inventory(&f.checkpoint()).is_err());
    f.advance(0, 1);
    j.rebase_funding_inventory(&f.checkpoint()).unwrap();
}

#[test]
fn immutable_epoch_and_unknown_signed_attempt_survive_restart() {
    let dir = tempdir();
    let path = dir.path().join("journal.sqlite");
    let (mut j, f, plan) = prepared(&path, [10, -4]);
    let saved = j.prepare_funding_epoch(&plan).unwrap();
    assert_eq!(saved.steps.len(), 3);
    assert!(j
        .record_funding_epoch_attempt(1, f.users[0].0, [8; 64], 1)
        .is_err());
    j.record_funding_epoch_attempt(1, book_key(), [8; 64], 1)
        .unwrap();
    drop(j);
    let mut j = Journal::open(&path).unwrap();
    assert!(j.has_unresolved_maintenance().unwrap());
    assert_eq!(j.funding_checkpoint().unwrap(), Some(plan.previous.clone()));
    // Even expiry=1 does not turn absence/timeout into authoritative rejection.
    assert!(j
        .record_funding_epoch_attempt(1, book_key(), [9; 64], u64::MAX)
        .is_err());
    assert!(j
        .record_funding_epoch_attempt(1, f.users[0].0, [9; 64], 999)
        .is_err());
    j.record_funding_epoch_attempt(1, book_key(), [8; 64], 1)
        .unwrap();
    let resumed =
        FundingEpochPlan::resume(&saved.previous, &saved.target, &f.book, &f.users).unwrap();
    assert_eq!(resumed.steps, plan.steps);
    let o = observation(&plan, book_key(), [8; 64], true);
    j.observe_funding_epoch_step(&o).unwrap();
    j.observe_funding_epoch_step(&o).unwrap();
    assert!(j
        .record_funding_epoch_attempt(1, book_key(), [9; 64], 999)
        .is_err());
}

#[test]
fn exact_failed_receipt_alone_allows_a_replacement_attempt() {
    let dir = tempdir();
    let (mut j, _, plan) = prepared(&dir.path().join("journal.sqlite"), [10, -4]);
    j.record_funding_epoch_attempt(1, book_key(), [8; 64], 999)
        .unwrap();
    assert!(j
        .observe_funding_epoch_step(&observation(&plan, book_key(), [9; 64], false))
        .is_err());
    let rejected = observation(&plan, book_key(), [8; 64], false);
    j.observe_funding_epoch_step(&rejected).unwrap();
    j.observe_funding_epoch_step(&rejected).unwrap();
    j.record_funding_epoch_attempt(1, book_key(), [9; 64], 1000)
        .unwrap();
    assert!(j
        .record_funding_epoch_attempt(1, book_key(), [10; 64], 1001)
        .is_err());
    assert!(j
        .observe_funding_epoch_step(&observation(&plan, book_key(), [8; 64], true))
        .is_err());
    j.observe_funding_epoch_step(&observation(&plan, book_key(), [9; 64], true))
        .unwrap();
    let r = j.funding_epoch(1).unwrap().unwrap();
    assert_eq!(
        r.steps
            .iter()
            .find(|s| s.scope == book_key())
            .unwrap()
            .attempts
            .len(),
        2
    );
}

#[test]
fn checkpoint_advances_only_after_every_user_receipt_and_aligned_inventory() {
    let dir = tempdir();
    let path = dir.path().join("journal.sqlite");
    let (mut j, mut f, plan) = prepared(&path, [10, 0]);
    apply(&mut j, &plan, book_key(), [8; 64]);
    apply(&mut j, &plan, f.users[0].0, [9; 64]);
    assert!(j.complete_funding_epoch(&aligned(&mut f, &plan)).is_err());
    drop(j);
    let mut j = Journal::open(&path).unwrap();
    apply(&mut j, &plan, f.users[1].0, [10; 64]); // flat user must still advance
    f.users[0].1.nonce += 1;
    assert!(j.complete_funding_epoch(&f.checkpoint()).is_err());
    f.users[0].1.nonce -= 1;
    let mut invalid = f.checkpoint();
    invalid.rates.get_mut(&1).unwrap().last_update_seconds = 1;
    assert!(j.complete_funding_epoch(&invalid).is_err());
    j.complete_funding_epoch(&f.checkpoint()).unwrap();
    j.complete_funding_epoch(&f.checkpoint()).unwrap();
    assert!(!j.has_unresolved_maintenance().unwrap());
    assert_eq!(j.funding_checkpoint().unwrap(), Some(plan.target.clone()));
    drop(j);
    assert!(
        Journal::open(&path)
            .unwrap()
            .funding_epoch(1)
            .unwrap()
            .unwrap()
            .completed
    );
}

#[test]
fn later_native_generation_does_not_skip_the_completed_target() {
    let dir = tempdir();
    let path = dir.path().join("journal.sqlite");
    let (mut j, mut f, plan) = prepared(&path, [10, -4]);
    for (i, scope) in std::iter::once(book_key())
        .chain(f.users.iter().map(|(a, _)| *a))
        .enumerate()
    {
        apply(&mut j, &plan, scope, [i as u8 + 8; 64]);
    }
    aligned(&mut f, &plan);
    f.advance(300, 3);
    j.complete_funding_epoch(&f.checkpoint()).unwrap();
    assert_eq!(j.funding_checkpoint().unwrap(), Some(plan.target.clone()));
    let next = f.plan(&plan.target);
    j.prepare_funding_epoch(&next).unwrap();
    drop(j);
    assert!(Journal::open(&path)
        .unwrap()
        .has_unresolved_maintenance()
        .unwrap());
}

#[test]
fn active_epoch_cannot_adopt_new_registry_or_new_orders() {
    let dir = tempdir();
    let (mut j, f, plan) = prepared(&dir.path().join("journal.sqlite"), [10, -4]);
    assert!(j.remember_users(&[([4; 32], [5; 32])]).is_err());
    assert!(j
        .prepare_intent(BoundedIntent {
            identity: OrderIdentity {
                user_ledger: f.users[0].0,
                user_pubkey: f.users[0].1.user.to_bytes(),
                user_nonce: 2,
                client_oid: [3; 16],
                kind: OrderKind::User
            },
            asset_id: 1,
            requested_lots: 1,
            limit_price_ticks: 100,
            last_valid_slot: 100,
            post_fail_position_im_usdc: 0,
            created_at_ms: 2000,
        })
        .is_err());
    assert!(j.initialize_funding_checkpoint(&plan.previous).is_err());
    assert!(j.rebase_funding_inventory(&plan.previous).is_err());
    assert_eq!(j.known_users().unwrap().len(), 2);
}

#[test]
fn corrupt_checkpoint_plan_hash_or_completed_history_fail_closed() {
    for mode in 0..3 {
        let dir = tempdir();
        let path = dir.path().join("journal.sqlite");
        let (mut j, mut f, plan) = prepared(&path, [10, -4]);
        if mode == 2 {
            for (i, scope) in std::iter::once(book_key())
                .chain(f.users.iter().map(|(a, _)| *a))
                .enumerate()
            {
                apply(&mut j, &plan, scope, [i as u8 + 8; 64]);
            }
            j.complete_funding_epoch(&aligned(&mut f, &plan)).unwrap();
        }
        match mode {
            0 => {
                j.connection
                    .execute(
                        "UPDATE funding_checkpoint SET payload=?",
                        params![b"{}".as_slice()],
                    )
                    .unwrap();
            }
            1 => {
                j.connection
                    .execute(
                        "UPDATE funding_epoch_steps SET body_hash=?",
                        params![[0u8; 32].as_slice()],
                    )
                    .unwrap();
            }
            2 => {
                j.connection
                    .execute("DELETE FROM funding_checkpoint", [])
                    .unwrap();
            }
            _ => unreachable!(),
        }
        drop(j);
        assert!(matches!(
            Journal::open(&path),
            Err(JournalError::CorruptState(_))
        ));
    }
}

#[test]
fn maintenance_epoch_keeps_recovery_gated_without_false_i2_repair() {
    use crate::{
        AckObservation, ErAckCommand, ErAckSubmissionPort, ErAckSubmitResult, ErrorCode,
        HaltReason, LedgerRecoveryPort, Operation, ReconciliationSnapshot, RecoveryCoordinator,
        VenueObservation, VenueRecoveryPort, VenueSubmissionPort, VenueSubmitResult,
    };
    struct Port;
    impl VenueRecoveryPort for Port {
        fn observe(&mut self, _: &Operation) -> Result<VenueObservation, ErrorCode> {
            panic!("no order may overlap funding maintenance")
        }
    }
    impl VenueSubmissionPort for Port {
        fn submit(&mut self, _: &Operation) -> Result<VenueSubmitResult, ErrorCode> {
            panic!("maintenance cannot submit a native order")
        }
    }
    impl LedgerRecoveryPort for Port {
        fn observe_ack(&mut self, _: &Operation) -> Result<AckObservation, ErrorCode> {
            panic!("no order may overlap funding maintenance")
        }
        fn reconciliation(&mut self) -> Result<ReconciliationSnapshot, ErrorCode> {
            panic!("a partially allocated epoch must not be treated as an I2 incident")
        }
        fn set_operator_down(&mut self, down: bool) -> Result<(), ErrorCode> {
            assert!(down, "maintenance must not reopen entries");
            Ok(())
        }
    }
    impl ErAckSubmissionPort for Port {
        fn submit_ack(&mut self, _: &ErAckCommand) -> Result<ErAckSubmitResult, ErrorCode> {
            panic!("no order may overlap funding maintenance")
        }
    }
    let dir = tempdir();
    let (j, _, _) = prepared(&dir.path().join("journal.sqlite"), [10, -4]);
    let mut coordinator = RecoveryCoordinator::new(j, Port, Port);
    let report = coordinator.recover(2000).unwrap();
    assert_eq!(report.reason, Some(HaltReason::UnresolvedMaintenance));
    assert!(!report.entries_enabled);
    assert!(!coordinator.entries_enabled());
}

#[test]
fn allocation_bodies_are_not_persisted_in_the_maintenance_journal() {
    let dir = tempdir();
    let (j, _, plan) = prepared(&dir.path().join("journal.sqlite"), [10, -4]);
    let mut statement = j
        .connection
        .prepare("SELECT body_hash FROM funding_epoch_steps")
        .unwrap();
    let hashes = statement
        .query_map([], |r| r.get::<_, Vec<u8>>(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();
    assert!(hashes.iter().all(|hash| hash.len() == 32));
    for body in plan.steps.values() {
        assert!(hashes.contains(&Sha256::digest(body).to_vec()));
        assert!(!hashes.contains(body));
    }
    for table in [
        "funding_checkpoint",
        "funding_epochs",
        "funding_epoch_steps",
        "funding_epoch_attempts",
    ] {
        let mut schema = j
            .connection
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap();
        let columns = schema
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect::<Vec<_>>();
        assert!(columns.iter().all(
            |c| !["instruction", "transaction", "account_bytes", "token"].contains(&c.as_str())
        ));
    }
}
