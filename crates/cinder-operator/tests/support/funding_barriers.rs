use super::*;
use crate::funding_maintenance::tests::Fixture;
use crate::{BoundedIntent, FillFact, OrderIdentity, OrderKind};

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
fn order(j: &mut Journal, f: &Fixture) -> OperationId {
    j.prepare_intent(BoundedIntent {
        identity: OrderIdentity {
            user_ledger: f.users[0].0,
            user_pubkey: f.users[0].1.user.to_bytes(),
            user_nonce: 0,
            client_oid: [1; 16],
            kind: OrderKind::User,
        },
        asset_id: 1,
        requested_lots: 10,
        limit_price_ticks: 100,
        last_valid_slot: 10,
        post_fail_position_im_usdc: 0,
        created_at_ms: 1000,
    })
    .unwrap()
    .operation_id
}
fn fill(j: &mut Journal, id: OperationId) {
    j.begin_venue_submission(&id, 1000).unwrap();
    j.record_fill_facts(
        &id,
        &[FillFact {
            event_id: [1; 32],
            filled_lots: 10,
            vwap_quote_lots: 1_000_000_000,
            fee_usdc: 0,
            fill_price_ticks: 100,
            observed_at_ms: 1000,
        }],
        1000,
    )
    .unwrap();
    j.record_venue_filled(&id, 1000).unwrap();
    j.finalize_ack(&id, true, 1000).unwrap();
}
#[test]
fn atomic_generation_barrier_carries_old_baseline_over_a_later_update() {
    let (dir, mut j, f) = setup();
    let id = order(&mut j, &f);
    j.record_order_funding_barrier(&id, f.checkpoint().rates())
        .unwrap();
    fill(&mut j, id);
    let newer = Fixture::new([10, 0], 100, 2);
    let observed = newer.checkpoint();
    let rebased = j.rebase_acknowledged_inventory(&observed).unwrap();
    assert_eq!(rebased.rates(), f.checkpoint().rates());
    assert_eq!(rebased.inventory_hash, observed.inventory_hash);
    drop(j);
    let j = Journal::open(dir.path().join("private/journal.sqlite")).unwrap();
    assert_eq!(j.funding_checkpoint().unwrap(), Some(rebased.clone()));
    let plan = newer.plan(&rebased);
    let ix = crate::transaction::decode_ix::<cinder_ledger::instruction::AllocateFunding>(
        plan.instruction(&newer.users[0].0).unwrap(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(ix.entries[0].delta_usdc, -1000);
}
#[test]
fn missing_barrier_wrong_rates_and_unexplained_inventory_never_get_rebased() {
    let (_dir, mut j, f) = setup();
    let newer = Fixture::new([10, 0], 0, 1);
    assert!(j
        .rebase_acknowledged_inventory(&newer.checkpoint())
        .is_err());
    let id = order(&mut j, &f);
    assert!(j
        .record_order_funding_barrier(&id, Fixture::new([0, 0], 100, 2).checkpoint().rates())
        .is_err());
    fill(&mut j, id);
    assert!(j
        .rebase_acknowledged_inventory(&newer.checkpoint())
        .is_err());
    assert_eq!(j.funding_checkpoint().unwrap(), Some(f.checkpoint()));
}
#[test]
fn unsent_failure_changes_nonce_but_not_funding_ownership() {
    let (_dir, mut j, mut f) = setup();
    let id = order(&mut j, &f);
    j.record_venue_rejected(&id, 1000).unwrap();
    j.finalize_ack(&id, false, 1000).unwrap();
    f.users[0].1.nonce = 1;
    f.advance(100, 2);
    let cp = j.rebase_acknowledged_inventory(&f.checkpoint()).unwrap();
    assert_eq!(cp.rates()[&1].cumulative_quote_lots_per_base_lot, 0);
    assert!(f.plan(&cp).instruction(&f.users[0].0).is_some());
}
#[test]
fn flat_registry_barrier_is_atomic_and_preserves_unprocessed_generation() {
    let (dir, mut j, mut f) = setup();
    let old = f.checkpoint();
    let mut new_user = f.users[0].1.clone();
    new_user.user = anchor_lang::prelude::Pubkey::new_from_array([3; 32]);
    let (address, bump) = anchor_lang::prelude::Pubkey::find_program_address(
        &[cinder_common::SEED_USER, new_user.user.as_ref()],
        &cinder_ledger::ID,
    );
    new_user.bump = bump;
    f.users.push((address.to_bytes(), new_user));
    f.advance(100, 2);
    let mut cp = f.checkpoint();
    cp.rates = old.rates.clone();
    let registry = f
        .users
        .iter()
        .map(|(a, l)| (*a, l.user.to_bytes()))
        .collect();
    j.incorporate_flat_registry(&cp, &registry).unwrap();
    assert_eq!(j.known_users().unwrap().len(), 3);
    assert_eq!(j.funding_checkpoint().unwrap(), Some(cp.clone()));
    assert!(j
        .funding_registry_transition(old.registry_hash, cp.registry_hash, 0)
        .unwrap());
    drop(j);
    let j = Journal::open(dir.path().join("private/journal.sqlite")).unwrap();
    assert_eq!(j.known_users().unwrap().len(), 3);
    assert_eq!(f.plan(&cp).steps.len(), 4);
}
#[test]
fn migrated_checkpoint_requires_exact_coherent_inventory_before_acquiring_anchor() {
    let (dir, j, f) = setup();
    j.connection.execute_batch("DROP TABLE maintenance_writes; DROP TABLE funding_inventory_anchor; DROP TABLE order_funding_barriers; DROP TABLE funding_registry_barriers; PRAGMA user_version=8;").unwrap();
    drop(j);
    let mut j = Journal::open(dir.path().join("private/journal.sqlite")).unwrap();
    assert!(j
        .ensure_inventory_anchor(&Fixture::new([1, 0], 0, 1).checkpoint())
        .is_err());
    j.ensure_inventory_anchor(&f.checkpoint()).unwrap();
    j.validate_funding_barriers().unwrap();
    j.connection
        .execute("UPDATE funding_inventory_anchor SET last_operation=1", [])
        .unwrap();
    drop(j);
    assert!(Journal::open(dir.path().join("private/journal.sqlite")).is_err());
}
#[test]
fn malformed_public_generation_barrier_cannot_hide_on_restart() {
    let (dir, mut j, f) = setup();
    let id = order(&mut j, &f);
    j.record_order_funding_barrier(&id, f.checkpoint().rates())
        .unwrap();
    j.connection
        .execute(
            "UPDATE order_funding_barriers SET rates=?",
            params![b"{}".as_slice()],
        )
        .unwrap();
    drop(j);
    assert!(Journal::open(dir.path().join("private/journal.sqlite")).is_err());
}

#[test]
fn never_updated_native_generation_zero_is_a_valid_fenced_baseline() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private/journal.sqlite");
    let mut j = Journal::open(&path).unwrap();
    let mut f = Fixture::new([0, 0], 0, 1);
    f.native.funding_updates_seconds.insert(1, 0);
    j.bind_runtime([9; 32]).unwrap();
    j.remember_users(
        &f.users
            .iter()
            .map(|(a, l)| (*a, l.user.to_bytes()))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    j.initialize_funding_checkpoint(&f.checkpoint()).unwrap();
    let id = order(&mut j, &f);
    j.record_order_funding_barrier(&id, f.checkpoint().rates())
        .unwrap();
    drop(j);
    assert!(Journal::open(&path).is_ok());
}
