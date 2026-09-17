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
