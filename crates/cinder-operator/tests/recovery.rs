use std::{
    cell::RefCell,
    collections::BTreeMap,
    path::{Path, PathBuf},
    rc::Rc,
};

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
    venue_reads: usize,
    ack_reads: usize,
    reconciliation_reads: usize,
    recovery_passes: usize,
    native_attempt: Option<PreparedVenue>,
    preflight_unavailable: bool,
    preflight_rejected: bool,
    wrong_native_response: bool,
    journal_path: Option<PathBuf>,
    solvency_unsafe: bool,
    solvency_missing: bool,
    admission_unsafe: bool,
    stress_policy_missing: bool,
    unsent_expired: bool,
    planned_budget: Option<ExecutionBudget>,
    funding_shortfall: u64,
    funding_enabled: bool,
    funding_receipt: bool,
    funding_failure: bool,
    funding_sends: usize,
    funding_observation_unknown: bool,
    book_sync_sends: usize,
    book_sync_observation_unknown: bool,
    book_sync_failure: bool,
    book_sync_mismatch: bool,
    book_cash: u64,
    vault_cash: u64,
    sync_receipts: BTreeMap<[u8; 64], BookSyncObservation>,
}

type Shared = Rc<RefCell<World>>;

#[derive(Clone)]
struct Venue(Shared);
#[derive(Clone)]
struct Ledger(Shared);

impl VenueRecoveryPort for Venue {
    fn unsent_expired(&mut self, _: &Operation) -> Result<bool, ErrorCode> {
        Ok(self.0.borrow().unsent_expired)
    }
    fn observe_funding(&mut self, record: &FundingRecord) -> Result<FundingObservation, ErrorCode> {
        let w = self.0.borrow();
        if w.funding_observation_unknown {
            return Ok(FundingObservation::Unknown);
        }
        if w.funding_receipt {
            return Ok(FundingObservation::Funded(
                cinder_vault::PhoenixFundingReceipt {
                    schema_version: cc::ACCOUNT_SCHEMA_VERSION,
                    funding_id: record.intent.funding_id,
                    amount: record.intent.amount,
                    phoenix_trader: anchor_lang::prelude::Pubkey::new_from_array(
                        record.intent.phoenix_trader,
                    ),
                    phoenix_program: anchor_lang::prelude::Pubkey::new_from_array(
                        record.intent.phoenix_program,
                    ),
                    funded_at_slot: 100,
                    bump: 1,
                },
            ));
        }
        Ok(if w.funding_failure {
            FundingObservation::Rejected { slot: 100 }
        } else {
            FundingObservation::Unknown
        })
    }
    fn begin_recovery(&mut self) {
        self.0.borrow_mut().recovery_passes += 1;
    }
    fn observe(&mut self, operation: &Operation) -> Result<VenueObservation, ErrorCode> {
        let mut world = self.0.borrow_mut();
        world.venue_reads += 1;
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
    fn prepare_funding(
        &mut self,
        _: &Operation,
        _: &FundingIntent,
    ) -> Result<PreparedVenue, ErrorCode> {
        if !self.0.borrow().funding_enabled {
            return Err(ErrorCode::VenueUnavailable);
        }
        Ok(PreparedVenue {
            signature: [21; 64],
            last_valid_block_height: 1,
        })
    }
    fn submit_funding(&mut self, record: &FundingRecord) -> Result<VenueSubmitResult, ErrorCode> {
        let mut w = self.0.borrow_mut();
        let reader = rusqlite::Connection::open_with_flags(
            w.journal_path.as_ref().unwrap(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let signature: Vec<u8> = reader
            .query_row(
                "SELECT signature FROM funding_outbox WHERE operation_id=?",
                [record.intent.operation_id.as_slice()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(signature, record.attempt.as_ref().unwrap().signature);
        w.funding_sends += 1;
        if w.funding_failure {
            return Ok(VenueSubmitResult::Unknown);
        }
        w.vault_cash -= record.intent.amount;
        w.collateral += record.intent.amount;
        w.funding_receipt = true;
        // Deliberately lose the response after the economic side effect.
        Ok(VenueSubmitResult::Unknown)
    }
    fn prepare_submission(&mut self, _: &Operation) -> Result<Option<PreparedVenue>, ErrorCode> {
        let world = self.0.borrow();
        if world.preflight_unavailable {
            Err(ErrorCode::VenueUnavailable)
        } else {
            Ok(world.native_attempt.clone())
        }
    }
    fn submit(&mut self, operation: &Operation) -> Result<VenueSubmitResult, ErrorCode> {
        assert_eq!(operation.state, OrderState::SubmissionIntent);
        let mut world = self.0.borrow_mut();
        if world.preflight_rejected {
            return Ok(VenueSubmitResult::PreflightRejected);
        }
        if let Some(attempt) = &world.native_attempt {
            assert_eq!(operation.venue_signature, Some(attempt.signature));
            // An independent SQLite reader sees the committed identity before
            // this fixture performs the broadcast/economic side effect.
            let reader = rusqlite::Connection::open_with_flags(
                world.journal_path.as_ref().unwrap(),
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap();
            let (state, signature): (i64, Vec<u8>) = reader
                .query_row(
                    "SELECT o.state,a.signature FROM operations o JOIN venue_attempts a
                 ON a.operation_id=o.operation_id WHERE o.operation_id=?",
                    [operation.operation_id.as_slice()],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap();
            assert_eq!(state, OrderState::SubmissionIntent as i64);
            assert_eq!(signature, attempt.signature);
        }
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
        } else if world.wrong_native_response {
            Ok(VenueSubmitResult::Accepted([99; 64]))
        } else {
            Ok(VenueSubmitResult::Accepted([1; 64]))
        }
    }
}

impl LedgerRecoveryPort for Ledger {
    fn plan_execution_budget(
        &mut self,
        _: &Operation,
    ) -> Result<Option<ExecutionBudget>, ErrorCode> {
        Ok(self.0.borrow().planned_budget)
    }
    fn funding_required(
        &mut self,
        operation: &Operation,
    ) -> Result<Option<FundingIntent>, ErrorCode> {
        let amount = self.0.borrow().funding_shortfall;
        Ok((amount > 0)
            .then(|| FundingIntent::new(operation.operation_id, amount, [7; 32], [8; 32])))
    }
    fn prepare_funding_book_sync(
        &mut self,
        _: &FundingRecord,
    ) -> Result<PreparedBookSync, ErrorCode> {
        let w = self.0.borrow();
        Ok(PreparedBookSync {
            transaction: PreparedVenue {
                signature: [30 + w.book_sync_sends as u8; 64],
                last_valid_block_height: 1,
            },
            collateral_usdc: w.collateral,
            native_observed_slot: 100,
        })
    }
    fn submit_funding_book_sync(
        &mut self,
        prepared: &PreparedBookSync,
    ) -> Result<ErAckSubmitResult, ErrorCode> {
        let mut w = self.0.borrow_mut();
        let reader = rusqlite::Connection::open_with_flags(
            w.journal_path.as_ref().unwrap(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let count: i64 = reader
            .query_row(
                "SELECT COUNT(*) FROM funding_book_sync WHERE signature=?",
                [prepared.transaction.signature.as_slice()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        w.book_sync_sends += 1;
        if w.book_sync_failure {
            w.sync_receipts.insert(
                prepared.transaction.signature,
                BookSyncObservation::Rejected { slot: 200 },
            );
        } else {
            w.book_cash = prepared.collateral_usdc;
            w.sync_receipts.insert(
                prepared.transaction.signature,
                BookSyncObservation::Applied { slot: 200 },
            );
        }
        Ok(ErAckSubmitResult::Unknown)
    }
    fn observe_funding_book_sync(
        &mut self,
        prepared: &PreparedBookSync,
    ) -> Result<BookSyncObservation, ErrorCode> {
        let w = self.0.borrow();
        if w.book_sync_observation_unknown {
            return Ok(BookSyncObservation::Unknown);
        }
        Ok(w.sync_receipts
            .get(&prepared.transaction.signature)
            .copied()
            .unwrap_or(BookSyncObservation::Unknown))
    }
    fn validate_funding_book_sync(
        &mut self,
        prepared: &PreparedBookSync,
    ) -> Result<bool, ErrorCode> {
        let w = self.0.borrow();
        if w.book_sync_mismatch {
            return Err(ErrorCode::ReconciliationUnavailable);
        }
        Ok(w.book_cash == w.collateral && w.book_cash == prepared.collateral_usdc)
    }
    fn check_admission(&mut self, _: &Operation) -> Result<(), ErrorCode> {
        if self.0.borrow().admission_unsafe {
            Err(ErrorCode::ReconciliationUnavailable)
        } else {
            Ok(()) // Explicit abstract fixture, never a production fallback.
        }
    }
    fn observe_ack(&mut self, operation: &Operation) -> Result<AckObservation, ErrorCode> {
        let mut world = self.0.borrow_mut();
        world.ack_reads += 1;
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
        let mut world = self.0.borrow_mut();
        world.reconciliation_reads += 1;
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
            user_entry_quote_lots: world
                .book_lots
                .iter()
                .map(|(asset, lots)| (*asset, i128::from(*lots) * 100))
                .collect(),
            phoenix_entry_quote_lots: world
                .venue_lots
                .iter()
                .map(|(asset, lots)| (*asset, i128::from(*lots) * 100))
                .collect(),
            user_cash_usdc: world.user_cash,
            user_unsettled_funding_usdc: 0,
            vault_usdc: world.vault_cash,
            phoenix_collateral_usdc: world.collateral,
            pool_unsettled_funding_usdc: 0,
            cash_in_flight_usdc: 0,
            halt_flags: world.halt,
            pool_safe: true,
            solvency: (!world.solvency_missing).then_some(SolvencyReport {
                user_margin_safe: !world.solvency_unsafe,
                positive_claims_usdc: world.user_cash,
                backing_equity_usdc: i128::from(world.collateral) + i128::from(world.vault_cash),
                backing_surplus_usdc: i128::from(world.collateral) + i128::from(world.vault_cash)
                    - world.user_cash,
                bad_debt_usdc: 0,
                gross_notional_usdc: 0,
                gross_limits_safe: (!world.stress_policy_missing).then_some(true),
                worst_stress_surplus_usdc: (!world.stress_policy_missing).then_some(0),
            }),
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
    shared.borrow_mut().journal_path = Some(path.to_owned());
    RecoveryCoordinator::new(
        Journal::open(path).unwrap(),
        Venue(shared.clone()),
        Ledger(shared.clone()),
    )
}

#[test]
fn expired_never_submitted_order_is_fail_acked_without_funding_or_ioc() {
    let dir = tempdir();
    let shared = world();
    shared.borrow_mut().unsent_expired = true;
    let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
    let op = runtime.prepare(intent(3, 0)).unwrap();
    runtime.recover(1000).unwrap();
    assert_eq!(shared.borrow().venue_sends, 0);
    assert_eq!(shared.borrow().funding_sends, 0);
    assert_eq!(
        shared.borrow().acknowledgements.get(&op.operation_id),
        Some(&AckFinality::Failed)
    );
    assert!(runtime.recover(1000).unwrap().entries_enabled);
}

#[test]
fn expiry_does_not_fail_ack_an_unknown_attempt() {
    let dir = tempdir();
    let shared = world();
    shared.borrow_mut().unsent_expired = true;
    let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
    let op = runtime.prepare(intent(3, 0)).unwrap();
    shared
        .borrow_mut()
        .venue
        .insert(op.operation_id, VenueObservation::Unknown);
    assert!(!runtime.recover(1000).unwrap().entries_enabled);
    assert_eq!(shared.borrow().ack_sends, 0);
    assert_eq!(shared.borrow().venue_sends, 0);
}

#[test]
fn expiry_cancels_only_never_signed_funding_and_preserves_audit() {
    let dir = tempdir();
    let shared = world();
    let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
    let op = runtime.prepare(intent(3, 0)).unwrap();
    runtime
        .prepare_funding(FundingIntent::new(op.operation_id, 100, [7; 32], [8; 32]))
        .unwrap();
    shared.borrow_mut().unsent_expired = true;
    runtime.recover(1000).unwrap();
    let funding = runtime
        .journal()
        .funding(&op.operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(funding.cancelled_at_ms, Some(1000));
    assert_eq!(funding.failed_at_slot, None);
    assert_eq!(funding.attempt, None);
    assert!(!runtime.journal().has_unresolved_funding().unwrap());
    assert_eq!(shared.borrow().funding_sends, 0);
    assert_eq!(shared.borrow().venue_sends, 0);
    assert!(runtime.recover(1000).unwrap().entries_enabled);
}

#[test]
fn automatic_topup_freezes_budget_before_funding_outbox_and_never_dispatches_on_that_view() {
    let dir = tempdir();
    let shared = world();
    let budget = ExecutionBudget {
        max_quote_lots: 1200,
        max_fee_usdc: 5,
    };
    {
        let mut w = shared.borrow_mut();
        w.planned_budget = Some(budget);
        w.funding_shortfall = 100;
    }
    let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
    let op = runtime.prepare(intent(3, 0)).unwrap();
    assert!(!runtime.recover(1000).unwrap().entries_enabled);
    assert_eq!(
        runtime
            .journal()
            .operation(&op.operation_id)
            .unwrap()
            .execution_budget,
        Some(budget)
    );
    assert_eq!(
        runtime.journal().funding_records().unwrap()[0]
            .intent
            .amount,
        100
    );
    assert_eq!(shared.borrow().venue_sends, 0);
    assert_eq!(shared.borrow().funding_sends, 0);
}

#[test]
fn funding_and_book_write_lost_responses_recover_without_duplicate_debits_or_assignments() {
    let dir = tempdir();
    let path = dir.path().join("journal.sqlite");
    let shared = world();
    {
        let mut w = shared.borrow_mut();
        w.funding_enabled = true;
        w.vault_cash = 100;
        w.user_cash += 100;
    }
    let mut runtime = coordinator(&path, &shared);
    let op = runtime.prepare(intent(3, 0)).unwrap();
    runtime
        .prepare_funding(FundingIntent::new(op.operation_id, 100, [7; 32], [8; 32]))
        .unwrap();
    runtime.recover(1000).unwrap();
    assert_eq!(shared.borrow().funding_sends, 1);
    assert_eq!(shared.borrow().venue_sends, 0);
    drop(runtime);
    // An expired deposit signature with unknown evidence must not be replaced.
    shared.borrow_mut().funding_observation_unknown = true;
    let mut runtime = coordinator(&path, &shared);
    for _ in 0..3 {
        runtime.recover(1000).unwrap();
    }
    assert_eq!(shared.borrow().funding_sends, 1);
    assert_eq!(shared.borrow().book_sync_sends, 0);
    shared.borrow_mut().funding_observation_unknown = false;
    runtime.recover(1000).unwrap();
    assert_eq!(shared.borrow().book_sync_sends, 1);
    assert!(runtime.journal().has_unresolved_funding().unwrap());
    drop(runtime);
    shared.borrow_mut().book_sync_observation_unknown = true;
    let mut runtime = coordinator(&path, &shared);
    for _ in 0..3 {
        runtime.recover(1000).unwrap();
    }
    assert_eq!(shared.borrow().book_sync_sends, 1);
    assert_eq!(shared.borrow().venue_sends, 0);
    shared.borrow_mut().book_sync_observation_unknown = false;
    shared.borrow_mut().book_sync_mismatch = true;
    runtime.recover(1000).unwrap();
    assert!(runtime.journal().has_unresolved_funding().unwrap());
    assert_eq!(shared.borrow().book_sync_sends, 1);
    shared.borrow_mut().book_sync_mismatch = false;
    runtime.recover(1000).unwrap();
    assert!(!runtime.journal().has_unresolved_funding().unwrap());
    assert_eq!(
        shared.borrow().venue_sends,
        0,
        "discard pre-funding snapshot even at completion"
    );
    runtime.recover(1000).unwrap();
    assert_eq!(shared.borrow().venue_sends, 1);
    assert_eq!(shared.borrow().funding_sends, 1);
    assert_eq!(shared.borrow().book_sync_sends, 1);
}

#[test]
fn proved_failed_deposit_rejects_unsent_hedge_and_fail_acks_without_trading() {
    let dir = tempdir();
    let path = dir.path().join("journal.sqlite");
    let shared = world();
    {
        let mut w = shared.borrow_mut();
        w.funding_enabled = true;
        w.funding_failure = true;
    }
    let mut runtime = coordinator(&path, &shared);
    let op = runtime.prepare(intent(3, 0)).unwrap();
    runtime
        .prepare_funding(FundingIntent::new(op.operation_id, 100, [7; 32], [8; 32]))
        .unwrap();
    runtime.recover(1000).unwrap();
    drop(runtime);
    let mut runtime = coordinator(&path, &shared);
    runtime.recover(1000).unwrap();
    assert_eq!(
        runtime.journal().operation(&op.operation_id).unwrap().state,
        OrderState::VenueRejected
    );
    assert_eq!(
        runtime
            .journal()
            .funding(&op.operation_id)
            .unwrap()
            .unwrap()
            .failed_at_slot,
        Some(100)
    );
    runtime.recover(1000).unwrap();
    runtime.recover(1000).unwrap();
    assert_eq!(
        runtime.journal().operation(&op.operation_id).unwrap().state,
        OrderState::Failed
    );
    assert_eq!(shared.borrow().venue_sends, 0);
    assert_eq!(shared.borrow().funding_sends, 1);
}

#[test]
fn proved_failed_book_write_allows_a_fresh_replacement() {
    let dir = tempdir();
    let path = dir.path().join("journal.sqlite");
    let shared = world();
    {
        let mut w = shared.borrow_mut();
        w.funding_enabled = true;
        w.vault_cash = 100;
        w.user_cash += 100;
        w.book_sync_failure = true;
    }
    let mut runtime = coordinator(&path, &shared);
    let op = runtime.prepare(intent(3, 0)).unwrap();
    runtime
        .prepare_funding(FundingIntent::new(op.operation_id, 100, [7; 32], [8; 32]))
        .unwrap();
    runtime.recover(1000).unwrap();
    runtime.recover(1000).unwrap();
    shared.borrow_mut().book_sync_failure = false;
    runtime.recover(1000).unwrap();
    assert_eq!(shared.borrow().book_sync_sends, 2);
    let attempts = runtime
        .journal()
        .book_sync_attempts(&op.operation_id)
        .unwrap();
    assert_eq!(attempts[0].failed_at_slot, Some(200));
    assert_ne!(
        attempts[0].prepared.transaction.signature,
        attempts[1].prepared.transaction.signature
    );
    runtime.recover(1000).unwrap();
    assert!(!runtime.journal().has_unresolved_funding().unwrap());
}

#[test]
fn stale_or_backwards_post_io_clock_prevents_unsigned_funding_broadcast() {
    for final_clock in [999, 5000] {
        let dir = tempdir();
        let path = dir.path().join("journal.sqlite");
        let shared = world();
        {
            let mut w = shared.borrow_mut();
            w.funding_enabled = true;
            w.vault_cash = 100;
            w.user_cash += 100;
        }
        let mut runtime = coordinator(&path, &shared);
        let op = runtime.prepare(intent(3, 0)).unwrap();
        runtime
            .prepare_funding(FundingIntent::new(op.operation_id, 100, [7; 32], [8; 32]))
            .unwrap();
        let mut calls = 0;
        let report = runtime
            .recover_with_clock(|| {
                calls += 1;
                if calls < 3 {
                    1000
                } else {
                    final_clock
                }
            })
            .unwrap();
        assert_eq!(report.reason, Some(HaltReason::StaleOrIncomplete));
        assert_eq!(shared.borrow().funding_sends, 0);
        assert!(runtime
            .journal()
            .funding(&op.operation_id)
            .unwrap()
            .unwrap()
            .attempt
            .is_none());
    }
}

#[test]
fn terminal_book_write_can_be_superseded_after_a_fresh_native_cash_change() {
    let dir = tempdir();
    let path = dir.path().join("journal.sqlite");
    let shared = world();
    {
        let mut w = shared.borrow_mut();
        w.funding_enabled = true;
        w.vault_cash = 200;
        w.user_cash += 200;
    }
    let mut runtime = coordinator(&path, &shared);
    let op = runtime.prepare(intent(3, 0)).unwrap();
    runtime
        .prepare_funding(FundingIntent::new(op.operation_id, 100, [7; 32], [8; 32]))
        .unwrap();
    runtime.recover(1000).unwrap();
    runtime.recover(1000).unwrap();
    {
        let mut w = shared.borrow_mut();
        w.collateral += 10;
        w.vault_cash -= 10;
    }
    runtime.recover(1000).unwrap();
    let attempts = runtime
        .journal()
        .book_sync_attempts(&op.operation_id)
        .unwrap();
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0].applied_at_slot, Some(200));
    assert_eq!(attempts[1].prepared.collateral_usdc, 1110);
    assert!(runtime.journal().has_unresolved_funding().unwrap());
    runtime.recover(1000).unwrap();
    assert!(!runtime.journal().has_unresolved_funding().unwrap());
    assert_eq!(shared.borrow().funding_sends, 1);
    assert_eq!(shared.borrow().venue_sends, 0);
}

#[test]
fn unresolved_funding_keeps_operator_down_and_blocks_venue_sends_after_restart() {
    let dir = tempdir();
    let path = dir.path().join("journal.sqlite");
    let shared = world();
    let mut journal = Journal::open(&path).unwrap();
    let op = journal.prepare_intent(intent(3, 0)).unwrap();
    journal
        .prepare_funding(FundingIntent::new(
            op.operation_id,
            50_000_000,
            [7; 32],
            [8; 32],
        ))
        .unwrap();
    journal
        .record_prepared_funding(
            &op.operation_id,
            &PreparedVenue {
                signature: [9; 64],
                last_valid_block_height: 1,
            },
        )
        .unwrap();
    drop(journal);
    let mut runtime = coordinator(&path, &shared);
    let report = runtime.recover(1000).unwrap();
    assert_eq!(report.reason, Some(HaltReason::UnresolvedOperations));
    assert!(!report.entries_enabled);
    assert_eq!(shared.borrow().venue_sends, 0);
    assert!(cc::entries_blocked(shared.borrow().halt));
    assert!(cc::withdraw_blocked(shared.borrow().halt));
}

#[test]
fn signed_native_attempt_is_committed_before_send_and_recovers_lost_response() {
    for wrong_response in [false, true] {
        let dir = tempdir();
        let path = dir.path().join("journal.sqlite");
        let shared = world();
        shared.borrow_mut().native_attempt = Some(PreparedVenue {
            signature: [1; 64],
            last_valid_block_height: u64::MAX,
        });
        shared.borrow_mut().lost_venue_response = !wrong_response;
        shared.borrow_mut().wrong_native_response = wrong_response;
        let mut runtime = coordinator(&path, &shared);
        let operation = runtime.prepare(intent(3, 0)).unwrap();
        runtime.recover(1000).unwrap();
        let persisted = runtime
            .journal()
            .operation(&operation.operation_id)
            .unwrap();
        assert_eq!(persisted.state, OrderState::Reconciling);
        assert_eq!(persisted.venue_signature, Some([1; 64]));
        if wrong_response {
            assert_eq!(
                persisted.last_error_code,
                Some(ErrorCode::CorrelationMismatch)
            );
        }
        drop(runtime);
        let mut runtime = coordinator(&path, &shared);
        runtime.recover(1000).unwrap();
        assert!(runtime.recover(1000).unwrap().entries_enabled);
        assert_eq!(shared.borrow().venue_sends, 1);
        assert_eq!(shared.borrow().ack_sends, 1);
    }
}

#[test]
fn preflight_failure_does_not_create_an_ambiguous_native_send() {
    let dir = tempdir();
    let shared = world();
    shared.borrow_mut().preflight_unavailable = true;
    let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
    let operation = runtime.prepare(intent(3, 0)).unwrap();
    runtime.recover(1000).unwrap();
    let persisted = runtime
        .journal()
        .operation(&operation.operation_id)
        .unwrap();
    assert_eq!(persisted.state, OrderState::Prepared);
    assert_eq!(persisted.venue_signature, None);
    assert_eq!(
        runtime
            .journal()
            .venue_attempt(&operation.operation_id)
            .unwrap(),
        None
    );
    assert_eq!(shared.borrow().venue_sends, 0);
    assert!(!runtime.entries_enabled());
}

#[test]
fn explicit_broadcast_preflight_rejection_restores_once_without_a_trade() {
    let dir = tempdir();
    let shared = world();
    let path = dir.path().join("journal.sqlite");
    let signature = [8; 64];
    {
        let mut w = shared.borrow_mut();
        w.preflight_rejected = true;
        w.native_attempt = Some(PreparedVenue {
            signature,
            last_valid_block_height: 1,
        });
        w.journal_path = Some(path.clone());
    }
    let mut runtime = coordinator(&path, &shared);
    let op = runtime.prepare(intent(3, 0)).unwrap();
    runtime.recover(1000).unwrap();
    let rejected = runtime.journal().operation(&op.operation_id).unwrap();
    assert_eq!(rejected.state, OrderState::VenueRejected);
    assert_eq!(rejected.venue_signature, Some(signature));
    assert_eq!(rejected.filled_lots, 0);
    assert_eq!(rejected.fee_usdc, 0);
    drop(runtime);
    let mut runtime = coordinator(&path, &shared);
    for _ in 0..3 {
        runtime.recover(1000).unwrap();
    }
    assert_eq!(
        runtime.journal().operation(&op.operation_id).unwrap().state,
        OrderState::Failed
    );
    assert_eq!(shared.borrow().venue_sends, 0);
    assert_eq!(shared.borrow().ack_sends, 1);
}

#[test]
fn venue_read_failure_preserves_pristine_unsent_wal_but_never_dispatches() {
    let dir = tempdir();
    let shared = world();
    shared.borrow_mut().unavailable = true;
    let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
    let op = runtime.prepare(intent(3, 0)).unwrap();
    runtime.recover(1000).unwrap();
    assert_eq!(
        runtime.journal().operation(&op.operation_id).unwrap().state,
        OrderState::Prepared
    );
    assert_eq!(shared.borrow().venue_sends, 0);
    assert!(!runtime.entries_enabled());
}

#[test]
fn reconciled_but_unsafe_or_missing_solvency_never_dispatches_or_releases_gate() {
    for missing in [false, true] {
        let dir = tempdir();
        let shared = world();
        shared.borrow_mut().solvency_missing = missing;
        shared.borrow_mut().solvency_unsafe = !missing;
        let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
        let op = runtime.prepare(intent(3, 0)).unwrap();
        let report = runtime.recover(1000).unwrap();
        assert_eq!(report.reason, Some(HaltReason::SolvencyUnsafe));
        assert!(!report.entries_enabled);
        assert_eq!(shared.borrow().venue_sends, 0);
        assert!(cc::withdraw_blocked(shared.borrow().halt));
        assert_eq!(
            runtime.journal().operation(&op.operation_id).unwrap().state,
            OrderState::Prepared
        );
    }
}

#[test]
fn post_intent_admission_failure_preserves_unsent_identity() {
    let dir = tempdir();
    let shared = world();
    shared.borrow_mut().admission_unsafe = true;
    let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
    let op = runtime.prepare(intent(3, 0)).unwrap();
    assert_eq!(
        runtime.recover(1000).unwrap().reason,
        Some(HaltReason::AdmissionUnsafe)
    );
    assert_eq!(shared.borrow().venue_sends, 0);
    assert_eq!(
        runtime.journal().operation(&op.operation_id).unwrap().state,
        OrderState::Prepared
    );
    assert!(runtime
        .journal()
        .venue_attempt(&op.operation_id)
        .unwrap()
        .is_none());
    shared.borrow_mut().admission_unsafe = false;
    runtime.recover(1000).unwrap();
    assert_eq!(shared.borrow().venue_sends, 1);
}

#[test]
fn current_solvency_without_stress_policy_cannot_authorize_dispatch() {
    let dir = tempdir();
    let shared = world();
    shared.borrow_mut().stress_policy_missing = true;
    let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
    let op = runtime.prepare(intent(3, 0)).unwrap();
    let report = runtime.recover(1000).unwrap();
    assert_eq!(report.reason, Some(HaltReason::AdmissionUnsafe));
    assert_eq!(shared.borrow().venue_sends, 0);
    assert_eq!(
        runtime.journal().operation(&op.operation_id).unwrap().state,
        OrderState::Prepared
    );
}

#[test]
fn a_recovery_port_without_explicit_admission_implementation_defaults_to_denial() {
    struct RecoveryOnly(Ledger);
    impl LedgerRecoveryPort for RecoveryOnly {
        fn observe_ack(&mut self, operation: &Operation) -> Result<AckObservation, ErrorCode> {
            self.0.observe_ack(operation)
        }
        fn reconciliation(&mut self) -> Result<ReconciliationSnapshot, ErrorCode> {
            self.0.reconciliation()
        }
        fn set_operator_down(&mut self, down: bool) -> Result<(), ErrorCode> {
            self.0.set_operator_down(down)
        }
    }
    let dir = tempdir();
    let shared = world();
    let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
    let operation = runtime.prepare(intent(3, 0)).unwrap();
    let mut port = RecoveryOnly(Ledger(shared));
    assert_eq!(
        port.check_admission(&operation),
        Err(ErrorCode::ReconciliationUnavailable)
    );
}

#[test]
fn confirmed_fill_is_acknowledged_even_when_whole_pool_health_is_unsafe() {
    let dir = tempdir();
    let shared = world();
    let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
    let op = runtime.prepare(intent(3, 0)).unwrap();
    runtime.recover(1000).unwrap();
    shared.borrow_mut().solvency_unsafe = true;
    runtime.recover(1000).unwrap();
    let report = runtime.recover(1000).unwrap();
    assert_eq!(report.reason, Some(HaltReason::SolvencyUnsafe));
    assert_eq!(
        runtime.journal().operation(&op.operation_id).unwrap().state,
        OrderState::Acked
    );
    assert_eq!(shared.borrow().ack_sends, 1);
    assert_eq!(shared.borrow().venue_sends, 1);
    assert!(cc::withdraw_blocked(shared.borrow().halt));
}

#[test]
fn crash_after_native_preparation_never_blindly_resubmits_an_ioc() {
    let dir = tempdir();
    let path = dir.path().join("journal.sqlite");
    let shared = world();
    let mut journal = Journal::open(&path).unwrap();
    let operation = journal.prepare_intent(intent(3, 0)).unwrap();
    journal
        .record_prepared_venue(
            &operation.operation_id,
            &PreparedVenue {
                signature: [1; 64],
                last_valid_block_height: 1,
            },
            1000,
        )
        .unwrap();
    drop(journal);
    shared
        .borrow_mut()
        .venue
        .insert(operation.operation_id, VenueObservation::Unknown);
    let mut runtime = coordinator(&path, &shared);
    for _ in 0..3 {
        assert!(!runtime.recover(1000).unwrap().entries_enabled);
    }
    assert_eq!(shared.borrow().venue_sends, 0);
    assert_eq!(shared.borrow().ack_sends, 0);
    assert_eq!(
        runtime
            .journal()
            .operation(&operation.operation_id)
            .unwrap()
            .venue_signature,
        Some([1; 64])
    );
}

#[test]
fn recovery_checks_freshness_against_the_clock_after_io() {
    let dir = tempdir();
    let shared = world();
    shared.borrow_mut().observed_at = 1500;
    let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
    let mut times = [1000, 1500, 1500].into_iter();
    let report = runtime
        .recover_with_clock(|| times.next().unwrap())
        .unwrap();
    assert!(report.entries_enabled);
    assert_eq!(shared.borrow().halt & cc::OPERATOR_DOWN, 0);
}

#[test]
fn stale_or_backwards_clock_cannot_dispatch_or_release_the_gate() {
    let stale_at = 1000 + cc::MARK_STALE_MS.max(cc::TRADER_STATE_STALE_MS) + 1;
    for times in [
        vec![1000, 999],
        vec![1000, 1000, 999],
        vec![1000, 1000, stale_at],
    ] {
        for prepared in [false, true] {
            let dir = tempdir();
            let shared = world();
            let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
            if prepared {
                runtime.prepare(intent(3, 0)).unwrap();
            }
            let mut clock = times.clone().into_iter();
            let report = runtime
                .recover_with_clock(|| clock.next().unwrap())
                .unwrap();
            assert_eq!(report.reason, Some(HaltReason::StaleOrIncomplete));
            assert!(!report.entries_enabled);
            assert_eq!(shared.borrow().venue_sends, 0);
            assert_ne!(shared.borrow().halt & cc::OPERATOR_DOWN, 0);
        }
    }
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
fn unavailable_venue_ack_and_reconciliation_stay_gated_without_sends() {
    for terminal in [false, true] {
        let dir = tempdir();
        let path = dir.path().join("journal.sqlite");
        let shared = world();
        let mut journal = Journal::open(&path).unwrap();
        let op = journal.prepare_intent(intent(3, 0)).unwrap();
        if terminal {
            journal
                .record_fill_facts(&op.operation_id, &[fact(10, 1)], 1000)
                .unwrap();
            journal.record_venue_filled(&op.operation_id, 1000).unwrap();
        }
        drop(journal);
        shared.borrow_mut().unavailable = true;
        let mut runtime = coordinator(&path, &shared);
        for _ in 0..2 {
            let report = runtime.recover(1000).unwrap();
            assert!(!report.entries_enabled);
            assert_eq!(report.reason, Some(HaltReason::PortUnavailable));
        }
        let world = shared.borrow();
        assert_eq!(world.recovery_passes, 2);
        assert_eq!(world.venue_reads, if terminal { 0 } else { 2 });
        assert_eq!(world.ack_reads, if terminal { 2 } else { 0 });
        assert_eq!(world.reconciliation_reads, 2);
        assert_eq!(world.venue_sends, 0);
        assert_eq!(world.ack_sends, 0);
        assert_ne!(world.halt & cc::OPERATOR_DOWN, 0);
        assert_eq!(
            runtime
                .journal()
                .operation(&op.operation_id)
                .unwrap()
                .last_error_code,
            Some(if terminal {
                ErrorCode::LedgerUnavailable
            } else {
                ErrorCode::VenueUnavailable
            })
        );
    }
}

#[test]
fn partial_fills_are_deduplicated_and_never_fail_acked() {
    for lots in [4, -4] {
        for rejected in [false, true] {
            terminal_partial_fill_recovers(lots, rejected);
        }
    }
}

#[test]
fn proved_quote_or_fee_breaches_are_acknowledged_but_never_reopen_trading() {
    for budget in [
        ExecutionBudget {
            max_quote_lots: 999,
            max_fee_usdc: 2,
        },
        ExecutionBudget {
            max_quote_lots: 1_000,
            max_fee_usdc: 1,
        },
    ] {
        let dir = tempdir();
        let path = dir.path().join("journal.sqlite");
        let shared = world();
        let mut runtime = coordinator(&path, &shared);
        let op = runtime.prepare(intent(3, 0)).unwrap();
        runtime
            .record_execution_budget(&op.operation_id, budget)
            .unwrap();
        // Adversarial port fixture reports a real fill outside its budget.
        // The economic fact must not disappear or turn into a fail ACK.
        runtime.recover(1000).unwrap();
        assert_eq!(
            runtime.recover(1000).unwrap().reason,
            Some(HaltReason::ExecutionBudgetExceeded)
        );
        assert_eq!(shared.borrow().ack_sends, 1);
        drop(runtime);
        let mut runtime = coordinator(&path, &shared);
        assert_eq!(
            runtime.recover(1000).unwrap().reason,
            Some(HaltReason::ExecutionBudgetExceeded)
        );
        let recovered = runtime.journal().operation(&op.operation_id).unwrap();
        assert_eq!(recovered.state, OrderState::Acked);
        assert_eq!(recovered.filled_lots, 10);
        assert_eq!(recovered.fee_usdc, 2);
        assert!(runtime.journal().has_execution_budget_breach().unwrap());
        for _ in 0..2 {
            assert_eq!(
                runtime.recover(1000).unwrap().reason,
                Some(HaltReason::ExecutionBudgetExceeded)
            );
        }
        let world = shared.borrow();
        assert_eq!(world.ack_sends, 1);
        assert_eq!(world.venue_sends, 1);
        assert_eq!(world.order, vec![true]);
        assert_ne!(world.halt & cc::OPERATOR_DOWN, 0);
    }
}

fn terminal_partial_fill_recovers(lots: i64, rejected: bool) {
    let dir = tempdir();
    let shared = world();
    let path = dir.path().join("journal.sqlite");
    let mut runtime = coordinator(&path, &shared);
    let mut bounded = intent(3, 0);
    bounded.requested_lots = 10 * lots.signum();
    let operation = runtime.prepare(bounded).unwrap();
    runtime
        .record_execution_budget(
            &operation.operation_id,
            ExecutionBudget {
                max_quote_lots: lots.unsigned_abs() * 100,
                max_fee_usdc: 2,
            },
        )
        .unwrap();
    shared.borrow_mut().venue_lots.insert(1, lots);
    shared.borrow_mut().collateral = 998;
    shared.borrow_mut().venue.insert(
        operation.operation_id,
        VenueObservation::Open {
            fills: vec![fact(lots, 1)],
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
        lots
    );
    assert_eq!(shared.borrow().ack_sends, 0, "open fills cannot be acked");
    drop(runtime);
    let mut runtime = coordinator(&path, &shared);
    let fills = vec![fact(lots, 1)];
    shared.borrow_mut().venue.insert(
        operation.operation_id,
        if rejected {
            VenueObservation::Rejected { fills }
        } else {
            VenueObservation::Filled { fills }
        },
    );
    shared.borrow_mut().lost_ack_response = true;
    runtime.recover(1000).unwrap();
    assert!(!runtime.entries_enabled());
    assert_eq!(shared.borrow().ack_sends, 1);
    drop(runtime);
    let mut runtime = coordinator(&path, &shared);
    assert!(runtime.recover(1000).unwrap().entries_enabled);
    let recovered = runtime
        .journal()
        .operation(&operation.operation_id)
        .unwrap();
    assert_eq!(recovered.state, OrderState::Acked);
    assert_eq!(recovered.filled_lots, lots);
    assert_eq!(recovered.fee_usdc, 2);
    let world = shared.borrow();
    assert_eq!(world.ack_sends, 1, "restart must not duplicate the ack");
    assert_eq!(
        world.venue_sends, 0,
        "IOC remainder must not be resubmitted"
    );
    assert_eq!(world.order, vec![true], "never fail-ack a partial fill");
    assert_eq!(world.book_lots.get(&1), Some(&lots));
    assert_eq!(world.user_cash, 998);
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
fn even_full_open_fills_require_a_terminal_venue_outcome() {
    let dir = tempdir();
    let shared = world();
    let mut runtime = coordinator(&dir.path().join("journal.sqlite"), &shared);
    let operation = runtime.prepare(intent(3, 0)).unwrap();
    shared.borrow_mut().venue.insert(
        operation.operation_id,
        VenueObservation::Open {
            fills: vec![fact(10, 1)],
        },
    );
    runtime.recover(1000).unwrap();
    assert_eq!(shared.borrow().ack_sends, 0);
    assert!(!runtime.entries_enabled());
    let operation = runtime
        .journal()
        .operation(&operation.operation_id)
        .unwrap();
    assert!(!runtime.journal().fill_is_acknowledgeable(&operation));
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
