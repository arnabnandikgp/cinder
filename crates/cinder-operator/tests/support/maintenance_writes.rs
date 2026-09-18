use super::*;
use crate::funding_maintenance::tests::Fixture;
use crate::transaction::{anchor_ix, test_receipt};
use anchor_lang::InstructionData;

fn setup() -> (tempfile::TempDir, Journal, Fixture) {
    let dir = tempfile::tempdir().unwrap();
    let mut j = Journal::open(dir.path().join("private/journal.sqlite")).unwrap();
    let f = Fixture::new([0, 0], 0, 1);
    j.bind_runtime([9; 32]).unwrap();
    j.remember_users(
        &f.users
            .iter()
            .map(|(a, l)| (*a, l.user.to_bytes()))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    j.initialize_funding_checkpoint(&f.checkpoint()).unwrap();
    (dir, j, f)
}
fn heartbeat() -> (MaintenanceWrite, Vec<u8>) {
    let body = cinder_ledger::instruction::HeartbeatScan { now_ms: 1000 }.data();
    (
        MaintenanceWrite::new(
            book_key(),
            &body,
            [1; 32],
            [2; 32],
            MaintenanceParameters::Heartbeat { now_ms: 1000 },
        ),
        body,
    )
}
fn observe(w: &MaintenanceWrite, body: Vec<u8>, success: bool) -> Value {
    let mut a = vec![
        ([7; 32], true, false),
        (config_key(), false, false),
        (w.scope, false, true),
    ];
    if matches!(
        w.parameters,
        MaintenanceParameters::Fold { .. } | MaintenanceParameters::Liquidate { .. }
    ) {
        a.push((book_key(), false, true));
    }
    test_receipt(
        &anchor_ix(
            if matches!(w.parameters, MaintenanceParameters::Root { .. }) {
                vault_id()
            } else {
                ledger_id()
            },
            a,
            body,
        ),
        &w.signature.unwrap(),
        success,
    )
}

#[test]
fn unsigned_crash_can_cancel_but_signed_absence_and_expiry_cannot_resign() {
    let (dir, mut j, _) = setup();
    let (w, _) = heartbeat();
    j.prepare_maintenance_write(&w).unwrap();
    assert!(j.has_unresolved_maintenance().unwrap());
    drop(j);
    let mut j = Journal::open(dir.path().join("private/journal.sqlite")).unwrap();
    assert_eq!(j.active_maintenance_write().unwrap(), Some(w.clone()));
    j.finish_maintenance_write(w.id, WriteState::Cancelled)
        .unwrap();
    assert!(!j.has_unresolved_maintenance().unwrap());
    let (next, _) = heartbeat();
    j.prepare_maintenance_write(&next).unwrap();
    j.sign_maintenance_write(next.id, [8; 64], 1).unwrap();
    drop(j);
    let mut j = Journal::open(dir.path().join("private/journal.sqlite")).unwrap();
    let signed = j.active_maintenance_write().unwrap().unwrap();
    assert_eq!(signed.state, WriteState::Unknown);
    assert!(j.sign_maintenance_write(next.id, [9; 64], 999).is_err());
    assert!(j
        .finish_maintenance_write(next.id, WriteState::Cancelled)
        .is_err());
    assert!(signed.receipt(&Value::Null, [7; 32]).is_err());
    assert!(j.prepare_maintenance_write(&heartbeat().0).is_err());
}

#[test]
fn exact_receipt_is_required_and_failed_history_is_retained() {
    let (_dir, mut j, _) = setup();
    let (w, body) = heartbeat();
    j.prepare_maintenance_write(&w).unwrap();
    j.sign_maintenance_write(w.id, [8; 64], 999).unwrap();
    let w = j.active_maintenance_write().unwrap().unwrap();
    let mut bad = observe(&w, body.clone(), true);
    bad["slot"] = 0.into();
    assert!(w.receipt(&bad, [7; 32]).is_err());
    assert!(w
        .receipt(&observe(&w, body.clone(), true), [9; 32])
        .is_err());
    assert!(w
        .receipt(
            &observe(
                &w,
                cinder_ledger::instruction::HeartbeatScan { now_ms: 2000 }.data(),
                true
            ),
            [7; 32]
        )
        .is_err());
    let rejected = w.receipt(&observe(&w, body, false), [7; 32]).unwrap();
    j.finish_maintenance_write(w.id, rejected).unwrap();
    j.finish_maintenance_write(w.id, rejected).unwrap();
    assert!(j
        .finish_maintenance_write(w.id, WriteState::Applied(100))
        .is_err());
    let replacement = heartbeat().0;
    assert_ne!(replacement.id, w.id);
    j.prepare_maintenance_write(&replacement).unwrap();
    assert_eq!(j.maintenance_write(w.id).unwrap().unwrap().state, rejected);
}

#[test]
fn fold_and_bounded_liquidation_receipts_bind_both_private_accounts() {
    let (_, _, f) = setup();
    let cases = [
        (
            MaintenanceParameters::Fold { epoch: 0 },
            cinder_ledger::instruction::AllocateFunding {
                epoch: 0,
                fold: true,
                entries: vec![],
            }
            .data(),
        ),
        (
            MaintenanceParameters::Liquidate {
                asset: 1,
                oid: [1; 16],
                bound: 100,
                deadline: 10,
                close_lots: 2,
                post_im: 0,
            },
            cinder_ledger::instruction::LiquidateUserBounded {
                asset_id: 1,
                client_oid: [1; 16],
                limit_price_ticks: 100,
                last_valid_slot: 10,
                close_lots: 2,
                post_position_im_usdc: 0,
            }
            .data(),
        ),
    ];
    for (p, body) in cases {
        let mut w = MaintenanceWrite::new(f.users[0].0, &body, [1; 32], [2; 32], p);
        w.signature = Some([8; 64]);
        let receipt = observe(&w, body, true);
        assert!(matches!(
            w.receipt(&receipt, [7; 32]).unwrap(),
            WriteState::Applied(_)
        ));
        let mut wrong = receipt;
        wrong["transaction"]["message"]["instructions"][0]["accounts"][3] = 0.into();
        assert!(w.receipt(&wrong, [7; 32]).is_err());
    }
}

#[test]
fn reserve_receipt_binds_expected_epoch_debt_totals_and_pda() {
    let (_, _, _) = setup();
    let scope = crate::runtime::pda(vault_id(), &[cinder_common::SEED_RESERVE]);
    let p = MaintenanceParameters::Root {
        epoch: 1,
        root: [3; 32],
        users: 2,
        free: 400,
        reserved: 1,
        debt: 25,
        book_hash: [4; 32],
        ack_count: 20,
        observed_ms: 1000,
    };
    let body = cinder_vault::instruction::WriteReserveRootGuarded {
        expected_epoch: 1,
        root: [3; 32],
        user_count: 2,
        total_free: 400,
        total_reserved: 1,
        total_bad_debt: 25,
        book_hash: [4; 32],
    }
    .data();
    let mut w = MaintenanceWrite::new(scope, &body, [1; 32], [2; 32], p);
    w.signature = Some([8; 64]);
    assert!(matches!(
        w.receipt(&observe(&w, body.clone(), true), [7; 32])
            .unwrap(),
        WriteState::Applied(_)
    ));
    if let MaintenanceParameters::Root { debt, .. } = &mut w.parameters {
        *debt = 0;
    }
    assert!(w.receipt(&observe(&w, body, true), [7; 32]).is_err());
}

#[test]
fn corrupted_metadata_and_invalid_scope_fail_closed_on_restart() {
    let (dir, mut j, _) = setup();
    let (mut w, _) = heartbeat();
    w.scope = [1; 32];
    w.id = w.identity();
    assert!(j.prepare_maintenance_write(&w).is_err());
    let (w, _) = heartbeat();
    j.prepare_maintenance_write(&w).unwrap();
    j.connection
        .execute(
            "UPDATE maintenance_writes SET evidence_hash=?",
            params![[9u8; 32].as_slice()],
        )
        .unwrap();
    drop(j);
    assert!(Journal::open(dir.path().join("private/journal.sqlite")).is_err());
}

#[test]
fn journal_never_stores_private_instruction_bodies_or_account_snapshots() {
    let (_dir, mut j, f) = setup();
    let body = cinder_ledger::instruction::AllocateFunding {
        epoch: 0,
        fold: true,
        entries: vec![cinder_ledger::FundingEntry {
            asset_id: 1,
            delta_usdc: 0,
            post_position_im_usdc: 123456789,
        }],
    }
    .data();
    let w = MaintenanceWrite::new(
        f.users[0].0,
        &body,
        [1; 32],
        [2; 32],
        MaintenanceParameters::Fold { epoch: 0 },
    );
    j.prepare_maintenance_write(&w).unwrap();
    let p: Vec<u8> = j
        .connection
        .query_row("SELECT parameters FROM maintenance_writes", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(!String::from_utf8(p).unwrap().contains("123456789"));
    let n:i64=j.connection.query_row("SELECT count(*) FROM pragma_table_info('maintenance_writes') WHERE name IN ('body','account_data','ledger','token')",[],|r|r.get(0)).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn reserve_cadence_uses_durable_ack_counts_and_incidents_not_heartbeat_ticks() {
    let (_dir, mut j, _) = setup();
    assert!(!j.reserve_due(1000).unwrap());
    let (w, body) = heartbeat();
    j.prepare_maintenance_write(&w).unwrap();
    j.sign_maintenance_write(w.id, [8; 64], 999).unwrap();
    let w = j.active_maintenance_write().unwrap().unwrap();
    j.finish_maintenance_write(w.id, w.receipt(&observe(&w, body, true), [7; 32]).unwrap())
        .unwrap();
    assert!(!j
        .reserve_due(1000 + cinder_common::COMMIT_EVERY_MS)
        .unwrap());
    // Synthetic public journal fixture: the cadence evaluator does not claim
    // this SQL row is an authenticated financial receipt.
    let l = cinder_ledger::instruction::AllocateFunding {
        epoch: 0,
        fold: true,
        entries: vec![],
    }
    .data();
    let users = j.known_users().unwrap();
    let w = MaintenanceWrite::new(
        users[0].0,
        &l,
        [1; 32],
        [2; 32],
        MaintenanceParameters::Fold { epoch: 0 },
    );
    j.prepare_maintenance_write(&w).unwrap();
    j.sign_maintenance_write(w.id, [9; 64], 999).unwrap();
    j.finish_maintenance_write(w.id, WriteState::Applied(100))
        .unwrap();
    assert!(j.reserve_due(1001).unwrap());
}

#[test]
fn collateral_repair_has_its_own_native_receipt_identity_and_compute_budget() {
    let (_dir, mut j, _) = setup();
    let trader = [6; 32];
    let program =
        crate::transaction::bytes("EtrnLzgbS7nMMy5fbD42kXiUzGg8XQzJ972Xtk1cjWih").unwrap();
    let intent = crate::FundingIntent::new([5; 32], 1_000_000, trader, program);
    let scope = crate::runtime::pda(
        vault_id(),
        &[cinder_vault::SEED_PHOENIX_FUNDING, &intent.funding_id],
    );
    let body = cinder_vault::instruction::FundPhoenix {
        funding_id: intent.funding_id,
        amount: intent.amount,
        global_trader_index_count: 1,
    }
    .data();
    let w = MaintenanceWrite::new(
        scope,
        &body,
        [1; 32],
        [2; 32],
        MaintenanceParameters::PostCollateral {
            nonce: [5; 32],
            amount: intent.amount,
            trader,
            program,
            gti: 1,
        },
    );
    j.prepare_maintenance_write(&w).unwrap();
    j.sign_maintenance_write(w.id, [8; 64], 999).unwrap();
    let w = j.active_maintenance_write().unwrap().unwrap();
    let mut accounts = vec![([9; 32], false, false); 20];
    accounts[0] = ([7; 32], true, true);
    accounts[1] = (config_key(), false, false);
    accounts[2] = (
        crate::runtime::pda(vault_id(), &[cinder_common::SEED_VAULT_AUTHORITY]),
        false,
        false,
    );
    accounts[3] = (scope, false, true);
    accounts[11] = (program, false, false);
    accounts[14] = (trader, false, true);
    let mut receipt = test_receipt(&anchor_ix(vault_id(), accounts, body), &[8; 64], true);
    let keys = receipt["transaction"]["message"]["accountKeys"]
        .as_array_mut()
        .unwrap();
    let index = keys.len();
    keys.push("ComputeBudget111111111111111111111111111111".into());
    let mut budget = vec![2];
    budget.extend_from_slice(&1_400_000u32.to_le_bytes());
    receipt["transaction"]["message"]["instructions"].as_array_mut().unwrap().insert(0,serde_json::json!({"programIdIndex":index,"accounts":[],"data":bs58::encode(budget).into_string()}));
    assert!(matches!(
        w.receipt(&receipt, [7; 32]).unwrap(),
        WriteState::Applied(_)
    ));
    let mut foreign = receipt.clone();
    foreign["transaction"]["message"]["instructions"][1]["accounts"][14] = 0.into();
    assert!(w.receipt(&foreign, [7; 32]).is_err());
    let mut foreign = receipt;
    foreign["transaction"]["message"]["instructions"][0]["data"] =
        bs58::encode([2, 0, 0, 0, 0]).into_string().into();
    assert!(w.receipt(&foreign, [7; 32]).is_err());
}

#[test]
fn filled_ack_timer_and_twentieth_fill_are_durable_publication_triggers() {
    let (_dir, mut j, f) = setup();
    for n in 0u8..20 {
        let op = j
            .prepare_intent(crate::BoundedIntent {
                identity: crate::OrderIdentity {
                    user_ledger: f.users[0].0,
                    user_pubkey: f.users[0].1.user.to_bytes(),
                    user_nonce: u64::from(n),
                    client_oid: [n + 1; 16],
                    kind: crate::OrderKind::User,
                },
                asset_id: 1,
                requested_lots: 1,
                limit_price_ticks: 100,
                last_valid_slot: 100,
                post_fail_position_im_usdc: 0,
                created_at_ms: 1000,
            })
            .unwrap();
        j.begin_venue_submission(&op.operation_id, 1000).unwrap();
        j.record_fill_facts(
            &op.operation_id,
            &[crate::FillFact {
                event_id: [n + 1; 32],
                filled_lots: 1,
                vwap_quote_lots: 100,
                fee_usdc: 0,
                fill_price_ticks: 100,
                observed_at_ms: 1000,
            }],
            1000,
        )
        .unwrap();
        j.record_venue_filled(&op.operation_id, 1000).unwrap();
        j.finalize_ack(&op.operation_id, true, 1000).unwrap();
        assert_eq!(j.reserve_due(1000).unwrap(), n == 19);
        assert!(j
            .reserve_due(1000 + cinder_common::COMMIT_EVERY_MS)
            .unwrap());
    }
    assert!(j.reserve_debt_changed(1).unwrap());
    assert!(!j.reserve_debt_changed(0).unwrap());
}
