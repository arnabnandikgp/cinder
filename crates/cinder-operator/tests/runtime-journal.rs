use cinder_operator::*;
fn intent() -> BoundedIntent {
    BoundedIntent {
        identity: OrderIdentity {
            user_ledger: [1; 32],
            user_pubkey: [2; 32],
            user_nonce: u64::MAX,
            client_oid: [3; 16],
            kind: OrderKind::User,
        },
        asset_id: 1,
        requested_lots: 10,
        limit_price_ticks: u64::MAX,
        last_valid_slot: u64::MAX,
        post_fail_position_im_usdc: 0,
        created_at_ms: 0,
    }
}

#[test]
fn execution_budget_is_durable_immutable_and_cannot_be_added_after_side_effects() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private/journal.sqlite");
    let mut journal = Journal::open(&path).unwrap();
    let op = journal.prepare_intent(intent()).unwrap();
    let budget = ExecutionBudget {
        max_quote_lots: i64::MAX as u64,
        max_fee_usdc: u64::MAX,
    };
    assert_eq!(
        journal
            .record_execution_budget(&op.operation_id, budget)
            .unwrap()
            .execution_budget,
        Some(budget)
    );
    assert!(journal
        .record_execution_budget(
            &op.operation_id,
            ExecutionBudget {
                max_quote_lots: budget.max_quote_lots - 1,
                ..budget
            }
        )
        .is_err());
    assert!(journal
        .record_execution_budget(
            &op.operation_id,
            ExecutionBudget {
                max_fee_usdc: budget.max_fee_usdc - 1,
                ..budget
            }
        )
        .is_err());
    let attempt = PreparedVenue {
        signature: [8; 64],
        last_valid_block_height: 9,
    };
    journal
        .record_prepared_venue(&op.operation_id, &attempt, 1)
        .unwrap();
    drop(journal);
    let mut journal = Journal::open(&path).unwrap();
    assert_eq!(
        journal
            .operation(&op.operation_id)
            .unwrap()
            .execution_budget,
        Some(budget)
    );
    assert_eq!(
        journal
            .record_execution_budget(&op.operation_id, budget)
            .unwrap()
            .execution_budget,
        Some(budget)
    );
    for funding in [false, true] {
        let mut next = intent();
        next.identity.user_nonce -= if funding { 1 } else { 2 };
        let other = journal.prepare_intent(next).unwrap();
        if funding {
            // Separate journal so an active IOC cannot overlap new funding.
            let fork_dir = tempfile::tempdir().unwrap();
            let mut funded = Journal::open(fork_dir.path().join("private/journal.sqlite")).unwrap();
            let other = funded.prepare_intent(other.intent).unwrap();
            funded
                .prepare_funding(FundingIntent::new(other.operation_id, 1, [7; 32], [8; 32]))
                .unwrap();
            assert!(funded
                .record_execution_budget(&other.operation_id, budget)
                .is_err());
        } else {
            journal
                .begin_venue_submission(&other.operation_id, 1)
                .unwrap();
            assert!(journal
                .record_execution_budget(&other.operation_id, budget)
                .is_err());
        }
    }
}

#[test]
fn version_five_migration_does_not_invent_budgets_for_legacy_native_attempts() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private/journal.sqlite");
    let mut journal = Journal::open(&path).unwrap();
    let op = journal.prepare_intent(intent()).unwrap();
    let attempt = PreparedVenue {
        signature: [8; 64],
        last_valid_block_height: u64::MAX,
    };
    journal
        .record_prepared_venue(&op.operation_id, &attempt, 1)
        .unwrap();
    drop(journal);
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch("ALTER TABLE operations DROP COLUMN max_quote_lots; ALTER TABLE operations DROP COLUMN max_execution_fee; ALTER TABLE funding_outbox DROP COLUMN cancelled_ms; PRAGMA user_version=5;").unwrap();
    drop(connection);
    let journal = Journal::open(&path).unwrap();
    assert_eq!(journal.status().unwrap().schema_version, SCHEMA_VERSION);
    assert_eq!(
        journal
            .operation(&op.operation_id)
            .unwrap()
            .execution_budget,
        None
    );
    assert_eq!(
        journal.venue_attempt(&op.operation_id).unwrap(),
        Some(attempt)
    );
}

#[test]
fn incomplete_or_invalid_execution_budgets_fail_closed_on_restart() {
    for mutation in 0..3 {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("private/journal.sqlite");
        let mut journal = Journal::open(&path).unwrap();
        journal.prepare_intent(intent()).unwrap();
        drop(journal);
        let connection = rusqlite::Connection::open(&path).unwrap();
        let invalid = if mutation == 2 { u64::MAX } else { 0u64 };
        connection
            .execute(
                "UPDATE operations SET max_quote_lots=?, max_execution_fee=?",
                rusqlite::params![
                    invalid.to_be_bytes().as_slice(),
                    if mutation == 0 {
                        None
                    } else {
                        Some(0u64.to_be_bytes().to_vec())
                    }
                ],
            )
            .unwrap();
        drop(connection);
        assert!(Journal::open(&path).is_err());
    }
}
#[test]
fn prepared_ack_attempts_and_registry_survive_restart_with_full_u64s() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private/journal.sqlite");
    let mut journal = Journal::open(&path).unwrap();
    journal.bind_runtime([4; 32]).unwrap();
    journal.remember_users(&[([1; 32], [2; 32])]).unwrap();
    let op = journal.prepare_intent(intent()).unwrap();
    journal.record_venue_rejected(&op.operation_id, 1).unwrap();
    journal.begin_fail_ack(&op.operation_id, 2).unwrap();
    let ack = PreparedAck {
        signature: [8; 64],
        last_valid_block_height: u64::MAX,
        observed_ledger_nonce: u64::MAX,
    };
    journal.record_prepared_ack(&op.operation_id, &ack).unwrap();
    drop(journal);
    let mut journal = Journal::open(&path).unwrap();
    journal.bind_runtime([4; 32]).unwrap();
    assert_eq!(journal.ack_attempts(&op.operation_id).unwrap(), vec![ack]);
    assert_eq!(journal.known_users().unwrap(), vec![([1; 32], [2; 32])]);
    assert!(journal.bind_runtime([5; 32]).is_err());
    assert!(journal.remember_users(&[([1; 32], [7; 32])]).is_err());
}
#[test]
fn unbound_historical_side_effects_cannot_be_silently_adopted() {
    let dir = tempfile::tempdir().unwrap();
    let mut j = Journal::open(dir.path().join("private/journal.sqlite")).unwrap();
    let op = j.prepare_intent(intent()).unwrap();
    j.begin_venue_submission(&op.operation_id, 1).unwrap();
    assert!(j.bind_runtime([8; 32]).is_err());
}

#[test]
fn terminal_partial_ack_state_survives_restart() {
    for lots in [4i64, -4] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("private/journal.sqlite");
        let mut journal = Journal::open(&path).unwrap();
        let mut bounded = intent();
        bounded.requested_lots = 10 * lots.signum();
        let op = journal.prepare_intent(bounded).unwrap();
        journal
            .record_fill_facts(
                &op.operation_id,
                &[FillFact {
                    event_id: [9; 32],
                    filled_lots: lots,
                    vwap_quote_lots: lots * 100,
                    fee_usdc: 2,
                    fill_price_ticks: 100,
                    observed_at_ms: 1,
                }],
                1,
            )
            .unwrap();
        assert!(journal.begin_fill_ack(&op.operation_id, 2).is_err());
        journal.record_venue_filled(&op.operation_id, 2).unwrap();
        assert!(
            journal
                .record_fill_facts(
                    &op.operation_id,
                    &[FillFact {
                        event_id: [10; 32],
                        filled_lots: lots.signum(),
                        vwap_quote_lots: lots.signum() * 100,
                        fee_usdc: 1,
                        fill_price_ticks: 100,
                        observed_at_ms: 2,
                    }],
                    2,
                )
                .is_err(),
            "terminal remainder cannot gain another fill"
        );
        journal.begin_fill_ack(&op.operation_id, 3).unwrap();
        drop(journal);
        let journal = Journal::open(&path).unwrap();
        let reloaded = journal.operation(&op.operation_id).unwrap();
        assert_eq!(reloaded.state, OrderState::AckSubmissionIntent);
        assert_eq!(reloaded.filled_lots, lots);
        assert!(journal.fill_is_acknowledgeable(&reloaded));
    }
}

#[test]
fn prepared_native_attempt_survives_restart_and_cannot_be_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private/journal.sqlite");
    let mut journal = Journal::open(&path).unwrap();
    let op = journal.prepare_intent(intent()).unwrap();
    let attempt = PreparedVenue {
        signature: [8; 64],
        last_valid_block_height: u64::MAX,
    };
    let prepared = journal
        .record_prepared_venue(&op.operation_id, &attempt, u64::MAX)
        .unwrap();
    assert_eq!(prepared.state, OrderState::SubmissionIntent);
    assert_eq!(prepared.venue_signature, Some(attempt.signature));
    assert!(journal
        .record_prepared_venue(&op.operation_id, &attempt, 1)
        .is_err());
    assert!(journal
        .record_venue_submission(&op.operation_id, [9; 64], 1)
        .is_err());
    assert!(journal
        .record_definitely_never_submitted(&op.operation_id, 1)
        .is_err());
    drop(journal);
    let mut journal = Journal::open(&path).unwrap();
    assert_eq!(
        journal.venue_attempt(&op.operation_id).unwrap(),
        Some(attempt.clone())
    );
    let mut second_intent = intent();
    second_intent.identity.user_nonce -= 1;
    let second = journal.prepare_intent(second_intent).unwrap();
    assert!(journal
        .record_prepared_venue(&second.operation_id, &attempt, 1)
        .is_err());
    assert_eq!(
        journal.operation(&second.operation_id).unwrap().state,
        OrderState::Prepared,
        "conflicting signature must roll back the send state"
    );
}

#[test]
fn version_two_journal_migrates_without_losing_recovery_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private/journal.sqlite");
    let mut journal = Journal::open(&path).unwrap();
    journal.bind_runtime([4; 32]).unwrap();
    let op = journal.prepare_intent(intent()).unwrap();
    journal.begin_venue_submission(&op.operation_id, 1).unwrap();
    journal
        .record_venue_submission(&op.operation_id, [8; 64], 2)
        .unwrap();
    drop(journal);
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "DROP TABLE funding_book_sync; DROP TABLE funding_outbox; DROP TABLE venue_attempts; ALTER TABLE operations DROP COLUMN max_quote_lots; ALTER TABLE operations DROP COLUMN max_execution_fee; PRAGMA user_version=2;",
        )
        .unwrap();
    drop(connection);
    let mut journal = Journal::open(&path).unwrap();
    journal.bind_runtime([4; 32]).unwrap();
    assert_eq!(journal.status().unwrap().schema_version, SCHEMA_VERSION);
    assert_eq!(journal.venue_attempt(&op.operation_id).unwrap(), None);
    assert_eq!(
        journal.operation(&op.operation_id).unwrap().state,
        OrderState::Submitted
    );
    assert_eq!(
        journal.operation(&op.operation_id).unwrap().venue_signature,
        Some([8; 64])
    );
}

#[test]
fn funding_intent_and_exact_signature_survive_restart_and_gate_orders() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private/journal.sqlite");
    let mut journal = Journal::open(&path).unwrap();
    let op = journal.prepare_intent(intent()).unwrap();
    let funding = FundingIntent::new(op.operation_id, u64::MAX, [7; 32], [8; 32]);
    assert_eq!(
        journal.prepare_funding(funding.clone()).unwrap().intent,
        funding
    );
    assert_eq!(
        journal.prepare_funding(funding.clone()).unwrap().intent,
        funding
    );
    assert!(journal
        .prepare_funding(FundingIntent::new(op.operation_id, 1, [7; 32], [8; 32]))
        .is_err());
    let attempt = PreparedVenue {
        signature: [9; 64],
        last_valid_block_height: u64::MAX,
    };
    assert!(journal.has_unresolved_funding().unwrap());
    assert!(journal.begin_venue_submission(&op.operation_id, 0).is_err());
    assert!(journal
        .record_prepared_venue(&op.operation_id, &attempt, 0)
        .is_err());
    journal
        .record_prepared_funding(&op.operation_id, &attempt)
        .unwrap();
    assert!(journal
        .record_prepared_funding(&op.operation_id, &attempt)
        .is_err());
    drop(journal);
    let mut journal = Journal::open(&path).unwrap();
    let record = journal.funding(&op.operation_id).unwrap().unwrap();
    assert_eq!(record.intent, funding);
    assert_eq!(record.attempt, Some(attempt));
    assert_eq!(record.confirmed_at_slot, None);
    assert!(journal.has_unresolved_funding().unwrap());
    assert!(journal
        .record_prepared_funding(
            &op.operation_id,
            &PreparedVenue {
                signature: [10; 64],
                last_valid_block_height: 1
            }
        )
        .is_err());
}

#[test]
fn only_matching_receipt_completes_funding_and_completion_is_immutable() {
    use anchor_lang::prelude::Pubkey;
    let dir = tempfile::tempdir().unwrap();
    let mut journal = Journal::open(dir.path().join("private/journal.sqlite")).unwrap();
    let op = journal.prepare_intent(intent()).unwrap();
    let funding = FundingIntent::new(op.operation_id, 50_000_000, [7; 32], [8; 32]);
    journal.prepare_funding(funding.clone()).unwrap();
    let mut receipt = cinder_vault::PhoenixFundingReceipt {
        schema_version: cinder_common::ACCOUNT_SCHEMA_VERSION,
        funding_id: funding.funding_id,
        amount: funding.amount,
        phoenix_trader: Pubkey::new_from_array([7; 32]),
        phoenix_program: Pubkey::new_from_array([8; 32]),
        funded_at_slot: u64::MAX,
        bump: 1,
    };
    assert!(journal.confirm_funding(&op.operation_id, &receipt).is_err());
    journal
        .record_prepared_funding(
            &op.operation_id,
            &PreparedVenue {
                signature: [9; 64],
                last_valid_block_height: 1,
            },
        )
        .unwrap();
    receipt.amount -= 1;
    assert!(journal.confirm_funding(&op.operation_id, &receipt).is_err());
    receipt.amount += 1;
    receipt.phoenix_trader = Pubkey::new_from_array([6; 32]);
    assert!(journal.confirm_funding(&op.operation_id, &receipt).is_err());
    receipt.phoenix_trader = Pubkey::new_from_array([7; 32]);
    journal.confirm_funding(&op.operation_id, &receipt).unwrap();
    journal.confirm_funding(&op.operation_id, &receipt).unwrap();
    receipt.funded_at_slot -= 1;
    assert!(journal.confirm_funding(&op.operation_id, &receipt).is_err());
    assert!(journal.has_unresolved_funding().unwrap());
    let sync = PreparedBookSync {
        transaction: PreparedVenue {
            signature: [11; 64],
            last_valid_block_height: 2,
        },
        collateral_usdc: 50_000_000,
        native_observed_slot: u64::MAX,
    };
    assert!(journal
        .complete_funding_book_sync(&op.operation_id, sync.collateral_usdc)
        .is_err());
    journal
        .record_prepared_book_sync(&op.operation_id, &sync)
        .unwrap();
    assert!(journal
        .record_prepared_book_sync(&op.operation_id, &sync)
        .is_err());
    assert!(journal
        .complete_funding_book_sync(&op.operation_id, sync.collateral_usdc)
        .is_err());
    journal
        .finish_book_sync_attempt(&op.operation_id, &sync.transaction.signature, 123, true)
        .unwrap();
    assert!(journal
        .complete_funding_book_sync(&op.operation_id, 49_000_000)
        .is_err());
    journal
        .complete_funding_book_sync(&op.operation_id, sync.collateral_usdc)
        .unwrap();
    assert!(!journal.has_unresolved_funding().unwrap());
    assert!(journal
        .record_prepared_funding(
            &op.operation_id,
            &PreparedVenue {
                signature: [10; 64],
                last_valid_block_height: 2
            }
        )
        .is_err());
}

#[test]
fn unresolved_pool_funding_cannot_be_bypassed_by_another_order() {
    let dir = tempfile::tempdir().unwrap();
    let mut journal = Journal::open(dir.path().join("private/journal.sqlite")).unwrap();
    let op = journal.prepare_intent(intent()).unwrap();
    journal
        .prepare_funding(FundingIntent::new(op.operation_id, 1, [7; 32], [8; 32]))
        .unwrap();
    let mut second = intent();
    second.identity.user_nonce -= 1;
    let other = journal.prepare_intent(second).unwrap();
    assert!(journal
        .prepare_funding(FundingIntent::new(other.operation_id, 1, [7; 32], [8; 32]))
        .is_err());
    assert!(journal
        .begin_venue_submission(&other.operation_id, 0)
        .is_err());
    assert_eq!(
        journal.operation(&other.operation_id).unwrap().state,
        OrderState::Prepared
    );
}

#[test]
fn version_four_receipt_only_funding_remains_gated_until_book_synchronization() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private/journal.sqlite");
    let mut journal = Journal::open(&path).unwrap();
    let op = journal.prepare_intent(intent()).unwrap();
    let funding = FundingIntent::new(op.operation_id, 100, [7; 32], [8; 32]);
    journal.prepare_funding(funding.clone()).unwrap();
    journal
        .record_prepared_funding(
            &op.operation_id,
            &PreparedVenue {
                signature: [9; 64],
                last_valid_block_height: 1,
            },
        )
        .unwrap();
    journal
        .confirm_funding(
            &op.operation_id,
            &cinder_vault::PhoenixFundingReceipt {
                schema_version: 1,
                funding_id: funding.funding_id,
                amount: 100,
                phoenix_trader: anchor_lang::prelude::Pubkey::new_from_array([7; 32]),
                phoenix_program: anchor_lang::prelude::Pubkey::new_from_array([8; 32]),
                funded_at_slot: 2,
                bump: 1,
            },
        )
        .unwrap();
    drop(journal);
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch("DROP TABLE funding_book_sync; ALTER TABLE funding_outbox DROP COLUMN failed_slot; ALTER TABLE funding_outbox DROP COLUMN book_synced_slot; ALTER TABLE funding_outbox DROP COLUMN cancelled_ms; ALTER TABLE operations DROP COLUMN max_quote_lots; ALTER TABLE operations DROP COLUMN max_execution_fee; PRAGMA user_version=4;").unwrap();
    drop(connection);
    let mut journal = Journal::open(&path).unwrap();
    assert_eq!(journal.status().unwrap().schema_version, SCHEMA_VERSION);
    let record = journal.funding(&op.operation_id).unwrap().unwrap();
    assert_eq!(record.confirmed_at_slot, Some(2));
    assert_eq!(record.book_synced_at_slot, None);
    assert!(journal.has_unresolved_funding().unwrap());
    assert!(journal.begin_venue_submission(&op.operation_id, 0).is_err());
}
