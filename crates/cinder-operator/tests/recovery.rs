use std::{cell::RefCell, collections::BTreeMap, path::Path, rc::Rc};

use cinder_common as cc;
use cinder_operator::*;
use tempfile::TempDir;

fn tempdir() -> TempDir {
    let mut builder = tempfile::Builder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    builder.tempdir().unwrap()
}

fn intent(user: u8, nonce: u64) -> BoundedIntent {
    BoundedIntent {
        identity: OrderIdentity {
            user_ledger: [user.wrapping_add(1); 32],
            user_pubkey: [user; 32],
            user_nonce: nonce,
            client_oid: [7; 16],
            kind: OrderKind::User,
        },
        asset_id: 1,
        requested_lots: 10,
        limit_price_ticks: 100,
        last_valid_slot: u64::MAX,
        post_fail_position_im_usdc: 0,
        created_at_ms: 1000,
    }
}

fn fact(lots: i64, event: u8) -> FillFact {
    FillFact {
        event_id: [event; 32],
        filled_lots: lots,
        vwap_quote_lots: lots * 100,
        fee_usdc: 2,
        fill_price_ticks: 100,
        observed_at_ms: 1000,
    }
}

#[derive(Default)]
struct World {
    venue: BTreeMap<OperationId, VenueObservation>,
    acknowledgements: BTreeMap<OperationId, AckFinality>,
    venue_sends: usize,
    ack_sends: usize,
    order: Vec<bool>,
    venue_lots: BTreeMap<u16, i64>,
    book_lots: BTreeMap<u16, i64>,
    user_cash: i128,
    collateral: u64,
    halt: u8,
    observed_at: u64,
    incomplete: bool,
    lost_venue_response: bool,
    lost_ack_response: bool,
    reject: bool,
    unavailable: bool,
    gate_unavailable: bool,
    foreign_identity: bool,
}

type Shared = Rc<RefCell<World>>;

#[derive(Clone)]
struct Venue(Shared);
#[derive(Clone)]
struct Ledger(Shared);

impl VenueRecoveryPort for Venue {
    fn observe(&mut self, operation: &Operation) -> Result<VenueObservation, ErrorCode> {
        let world = self.0.borrow();
        if world.unavailable {
            return Err(ErrorCode::VenueUnavailable);
        }
        Ok(world
            .venue
            .get(&operation.operation_id)
            .cloned()
            .unwrap_or(VenueObservation::DefinitelyNeverSubmitted))
    }
}

impl VenueSubmissionPort for Venue {
    fn submit(&mut self, operation: &Operation) -> Result<VenueSubmitResult, ErrorCode> {
        assert_eq!(operation.state, OrderState::SubmissionIntent);
        let mut world = self.0.borrow_mut();
        world.venue_sends += 1;
        if world.reject {
            world.venue.insert(
                operation.operation_id,
                VenueObservation::Rejected { fills: vec![] },
            );
        } else {
            *world
                .venue_lots
                .entry(operation.intent.asset_id)
                .or_default() += operation.intent.requested_lots;
            world.collateral -= 2;
            world.venue.insert(
                operation.operation_id,
                VenueObservation::Filled {
                    fills: vec![fact(operation.intent.requested_lots, 1)],
                },
            );
        }
        if world.lost_venue_response {
            Ok(VenueSubmitResult::Unknown)
        } else {
            Ok(VenueSubmitResult::Accepted([1; 64]))
        }
    }
}

impl LedgerRecoveryPort for Ledger {
    fn observe_ack(&mut self, operation: &Operation) -> Result<AckObservation, ErrorCode> {
        let world = self.0.borrow();
        if world.unavailable {
            return Err(ErrorCode::LedgerUnavailable);
        }
        let mut identity = operation.intent.identity;
        if world.foreign_identity {
            identity.user_nonce += 1;
        }
        Ok(AckObservation {
            identity,
            operation_id: operation.operation_id,
            finality: world
                .acknowledgements
                .get(&operation.operation_id)
                .cloned()
                .unwrap_or(AckFinality::DefinitelyNotApplied),
        })
    }

    fn reconciliation(&mut self) -> Result<ReconciliationSnapshot, ErrorCode> {
        let world = self.0.borrow();
        if world.unavailable {
            return Err(ErrorCode::ReconciliationUnavailable);
        }
        Ok(ReconciliationSnapshot {
            complete: !world.incomplete,
            ledger_observed_at_ms: world.observed_at,
            trader_observed_at_ms: world.observed_at,
            mark_observed_at_ms: world.observed_at,
            book_lots: world.book_lots.clone(),
            phoenix_lots: world.venue_lots.clone(),
            user_cash_usdc: world.user_cash,
            user_unsettled_funding_usdc: 0,
            vault_usdc: 0,
            phoenix_collateral_usdc: world.collateral,
            pool_unsettled_funding_usdc: 0,
            cash_in_flight_usdc: 0,
            halt_flags: world.halt,
            pool_safe: true,
        })
    }

    fn set_operator_down(&mut self, down: bool) -> Result<(), ErrorCode> {
        let mut world = self.0.borrow_mut();
        if world.gate_unavailable {
            return Err(ErrorCode::LedgerUnavailable);
        }
        if down {
            world.halt |= cc::OPERATOR_DOWN;
        } else {
            world.halt &= !cc::OPERATOR_DOWN;
        }
        Ok(())
    }
}

impl ErAckSubmissionPort for Ledger {
    fn submit_ack(&mut self, command: &ErAckCommand) -> Result<ErAckSubmitResult, ErrorCode> {
        let mut world = self.0.borrow_mut();
        world.ack_sends += 1;
        let (identity, finality, fill) = match command {
            ErAckCommand::Fill { intent, fills } => {
                let lots = fills.iter().map(|fact| fact.filled_lots).sum();
                let quote_lots = fills.iter().map(|fact| fact.vwap_quote_lots).sum();
                let fee_usdc: u64 = fills.iter().map(|fact| fact.fee_usdc).sum();
                *world.book_lots.entry(intent.asset_id).or_default() += lots;
                world.user_cash -= i128::from(fee_usdc);
                (
                    intent.identity,
                    AckFinality::Filled {
                        lots,
                        quote_lots,
                        fee_usdc,
                    },
                    true,
                )
            }
            ErAckCommand::Fail { intent } => (intent.identity, AckFinality::Failed, false),
        };
        world.order.push(fill);
        world
            .acknowledgements
            .insert(derive_venue_identity(&identity).full_hash, finality);
        if world.lost_ack_response {
            Ok(ErAckSubmitResult::Unknown)
        } else {
            Ok(ErAckSubmitResult::Accepted([2; 64]))
        }
    }
}

fn world() -> Shared {
    Rc::new(RefCell::new(World {
        user_cash: 1000,
        collateral: 1000,
        observed_at: 1000,
        ..World::default()
    }))
}

fn coordinator(path: &Path, shared: &Shared) -> RecoveryCoordinator<Venue, Ledger> {
    RecoveryCoordinator::new(
        Journal::open(path).unwrap(),
        Venue(shared.clone()),
        Ledger(shared.clone()),
    )
}

#[test]
fn crash_matrix_converges_without_duplicate_economic_effects() {
    for boundary in 0..8 {
        let dir = tempdir();
        let path = dir.path().join("journal.sqlite");
        let shared = world();
        let mut journal = Journal::open(&path).unwrap();
        let operation = journal.prepare_intent(intent(3, 0)).unwrap();
        if boundary >= 1 {
            journal
                .begin_venue_submission(&operation.operation_id, 1000)
                .unwrap();
        }
        if boundary >= 2 {
            let operation = journal.operation(&operation.operation_id).unwrap();
            Venue(shared.clone()).submit(&operation).unwrap();
        }
        if boundary >= 3 {
            journal
                .record_venue_submission(&operation.operation_id, [1; 64], 1000)
                .unwrap();
        }
        if boundary >= 4 {
            journal
                .record_fill_facts(&operation.operation_id, &[fact(10, 1)], 1000)
                .unwrap();
            journal
                .record_venue_filled(&operation.operation_id, 1000)
                .unwrap();
        }
        if boundary >= 5 {
            journal
                .begin_fill_ack(&operation.operation_id, 1000)
                .unwrap();
        }
        if boundary >= 6 {
            Ledger(shared.clone())
                .submit_ack(&ErAckCommand::Fill {
                    intent: operation.intent.clone(),
                    fills: vec![fact(10, 1)],
                })
                .unwrap();
        }
        if boundary >= 7 {
            journal
                .record_fill_ack_submission(&operation.operation_id, [2; 64], 1000)
                .unwrap();
        }
        drop(journal);

        let mut restarted = coordinator(&path, &shared);
        assert!(!restarted.entries_enabled());
        for _ in 0..4 {
            restarted.recover(1000).unwrap();
        }
        assert!(restarted.entries_enabled(), "boundary {boundary}");
        assert_eq!(
            restarted
                .journal()
                .operation(&operation.operation_id)
                .unwrap()
                .state,
            OrderState::Acked
        );
        let world = shared.borrow();
        assert_eq!(world.venue_sends, 1, "boundary {boundary}");
        assert_eq!(world.ack_sends, 1, "boundary {boundary}");
        assert_eq!(world.book_lots.get(&1), Some(&10));
        assert_eq!(world.user_cash, 998);
        assert_eq!(world.halt, 0);
    }
}

#[test]
fn lost_send_and_ack_responses_recover_by_observation() {
    let dir = tempdir();
    let path = dir.path().join("journal.sqlite");
    let shared = world();
    shared.borrow_mut().lost_venue_response = true;
    shared.borrow_mut().lost_ack_response = true;
    let mut runtime = coordinator(&path, &shared);
    runtime.prepare(intent(3, 0)).unwrap();
    runtime.recover(1000).unwrap();
    drop(runtime);
    let mut runtime = coordinator(&path, &shared);
    for _ in 0..3 {
        runtime.recover(1000).unwrap();
    }
    assert!(runtime.entries_enabled());
    assert_eq!(shared.borrow().venue_sends, 1);
    assert_eq!(shared.borrow().ack_sends, 1);
}

#[test]
fn unknown_outcome_never_fails_or_resubmits_even_after_deadline() {
    let dir = tempdir();
    let shared = world();
    let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
    let mut request = intent(3, 0);
    request.last_valid_slot = 1;
    let operation = runtime.prepare(request).unwrap();
    shared
        .borrow_mut()
        .venue
        .insert(operation.operation_id, VenueObservation::Unknown);
    for _ in 0..3 {
        assert!(!runtime.recover(1_000_000).unwrap().entries_enabled);
    }
    assert_eq!(shared.borrow().venue_sends, 0);
    assert_eq!(shared.borrow().ack_sends, 0);
    assert_eq!(
        runtime
            .journal()
            .operation(&operation.operation_id)
            .unwrap()
            .state,
        OrderState::Reconciling
    );
}

#[test]
fn reject_and_fail_ack_are_replay_safe_and_preserve_other_halts() {
    let dir = tempdir();
    let path = dir.path().join("journal.sqlite");
    let shared = world();
    shared.borrow_mut().reject = true;
    shared.borrow_mut().lost_ack_response = true;
    let mut runtime = coordinator(&path, &shared);
    let operation = runtime.prepare(intent(3, 0)).unwrap();
    runtime.recover(1000).unwrap();
    runtime.recover(1000).unwrap();
    drop(runtime);
    shared.borrow_mut().halt |= cc::BAD_DEBT | cc::HALT_WITHDRAW;
    let mut runtime = coordinator(&path, &shared);
    assert_eq!(
        runtime.recover(1000).unwrap().reason,
        Some(HaltReason::ExternalHalt)
    );
    assert_eq!(
        runtime
            .journal()
            .operation(&operation.operation_id)
            .unwrap()
            .state,
        OrderState::Failed
    );
    assert_eq!(shared.borrow().ack_sends, 1);
    assert_eq!(
        shared.borrow().halt,
        cc::BAD_DEBT | cc::HALT_WITHDRAW | cc::OPERATOR_DOWN
    );
}

#[test]
fn incomplete_stale_mismatched_or_unwritable_gate_prevents_new_submission() {
    for failure in 0..5 {
        let dir = tempdir();
        let shared = world();
        match failure {
            0 => shared.borrow_mut().incomplete = true,
            1 => shared.borrow_mut().observed_at = 0,
            2 => shared.borrow_mut().user_cash += 1,
            3 => shared.borrow_mut().gate_unavailable = true,
            _ => shared
                .borrow_mut()
                .venue_lots
                .insert(2, 1)
                .map(|_| ())
                .unwrap_or(()),
        }
        let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
        runtime.prepare(intent(3, 0)).unwrap();
        assert!(
            !runtime
                .recover(if failure == 1 { 100_000 } else { 1000 })
                .unwrap()
                .entries_enabled
        );
        assert_eq!(shared.borrow().venue_sends, 0);
    }
}

#[test]
fn partial_fills_are_deduplicated_and_never_fail_acked() {
    let dir = tempdir();
    let shared = world();
    let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
    let operation = runtime.prepare(intent(3, 0)).unwrap();
    shared.borrow_mut().venue.insert(
        operation.operation_id,
        VenueObservation::Open {
            fills: vec![fact(4, 1)],
        },
    );
    for _ in 0..2 {
        runtime.recover(1000).unwrap();
    }
    assert_eq!(
        runtime
            .journal()
            .operation(&operation.operation_id)
            .unwrap()
            .filled_lots,
        4
    );
    shared.borrow_mut().venue.insert(
        operation.operation_id,
        VenueObservation::Rejected {
            fills: vec![fact(4, 1)],
        },
    );
    runtime.recover(1000).unwrap();
    assert!(!runtime.entries_enabled());
    assert_eq!(shared.borrow().ack_sends, 0);
    assert_eq!(
        runtime
            .journal()
            .fill_facts(&operation.operation_id)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn full_identity_and_exact_fill_facts_are_required_for_terminal_confirmation() {
    let dir = tempdir();
    let shared = world();
    let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
    let operation = runtime.prepare(intent(3, 0)).unwrap();
    runtime.recover(1000).unwrap();
    runtime.recover(1000).unwrap();
    shared.borrow_mut().foreign_identity = true;
    assert!(!runtime.recover(1000).unwrap().entries_enabled);
    shared.borrow_mut().foreign_identity = false;
    shared.borrow_mut().acknowledgements.insert(
        operation.operation_id,
        AckFinality::Filled {
            lots: 10,
            quote_lots: 1000,
            fee_usdc: 1,
        },
    );
    assert!(!runtime.recover(1000).unwrap().entries_enabled);
    assert_eq!(shared.borrow().ack_sends, 1);
}

#[test]
fn pending_ack_does_not_trigger_blind_retry() {
    let dir = tempdir();
    let shared = world();
    let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
    let operation = runtime.prepare(intent(3, 0)).unwrap();
    runtime.recover(1000).unwrap();
    runtime.recover(1000).unwrap();
    shared
        .borrow_mut()
        .acknowledgements
        .insert(operation.operation_id, AckFinality::Pending);
    for _ in 0..3 {
        runtime.recover(1000).unwrap();
    }
    assert_eq!(shared.borrow().ack_sends, 1);
    assert!(!runtime.entries_enabled());
}

#[test]
fn uncertain_operation_blocks_other_prepared_submissions() {
    let dir = tempdir();
    let shared = world();
    let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
    let unknown = runtime.prepare(intent(3, 0)).unwrap();
    runtime.prepare(intent(4, 0)).unwrap();
    shared
        .borrow_mut()
        .venue
        .insert(unknown.operation_id, VenueObservation::Unknown);
    assert_eq!(
        runtime.recover(1000).unwrap().reason,
        Some(HaltReason::UnresolvedOperations)
    );
    assert_eq!(shared.borrow().venue_sends, 0);
}

#[test]
fn fill_acknowledgements_take_priority_over_rejections() {
    let dir = tempdir();
    let shared = world();
    let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
    let rejected = runtime.prepare(intent(3, 0)).unwrap();
    let filled = runtime.prepare(intent(4, 0)).unwrap();
    shared.borrow_mut().venue.insert(
        rejected.operation_id,
        VenueObservation::Rejected { fills: vec![] },
    );
    shared.borrow_mut().venue.insert(
        filled.operation_id,
        VenueObservation::Filled {
            fills: vec![fact(10, 1)],
        },
    );
    shared.borrow_mut().venue_lots.insert(1, 10);
    shared.borrow_mut().collateral = 998;
    runtime.recover(1000).unwrap();
    assert_eq!(shared.borrow().order, vec![true, false]);
    assert!(runtime.recover(1000).unwrap().entries_enabled);
    assert_eq!(shared.borrow().venue_sends, 0);
}
