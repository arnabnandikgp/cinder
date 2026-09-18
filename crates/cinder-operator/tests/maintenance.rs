use cinder_common as cc;
use cinder_operator::{MaintenanceProgress, MaintenanceTask as Task, RuntimeError};

fn healthy(now: u64) -> MaintenanceProgress {
    MaintenanceProgress {
        last_poll_ms: Some(now),
        feeds_refreshed_ms: Some(now),
        mark_observed_ms: Some(now),
        trader_observed_ms: Some(now),
        scan_completed_ms: Some(now),
        heartbeat_written_ms: Some(now),
        expiry_reconciled_ms: Some(now),
        collateral_checked_ms: Some(now),
        root_written_ms: Some(now),
        ..Default::default()
    }
}

#[test]
fn startup_is_gated_and_never_invents_a_scan_heartbeat_or_funding_interval() {
    let p = MaintenanceProgress::default().plan(1000).unwrap();
    assert_eq!(p.halt_to_add, cc::OPERATOR_DOWN | cc::HALT_ENTRIES);
    assert_eq!(
        p.tasks,
        vec![
            Task::RecoverOrders,
            Task::RefreshFeeds,
            Task::MirrorHalts,
            Task::ScanLiquidations,
            Task::ReconcileExpiry,
            Task::CheckCollateral
        ]
    );
}

#[test]
fn cadence_is_based_on_successful_work_and_repeated_wakeups_are_coalesced() {
    let state = healthy(1000);
    assert_eq!(state.plan(1049).unwrap().tasks, vec![Task::RecoverOrders]);
    let p = state.plan(1050).unwrap();
    assert_eq!(p.halt_to_add, 0);
    assert_eq!(
        p.tasks,
        vec![
            Task::RecoverOrders,
            Task::RefreshFeeds,
            Task::ScanLiquidations
        ]
    );
    assert_eq!(state.plan(1050).unwrap(), p);
    let p = state.plan(2000).unwrap();
    assert!(p.tasks.contains(&Task::Heartbeat));
    assert!(p.tasks.contains(&Task::ReconcileExpiry));
    assert!(p.tasks.contains(&Task::CheckCollateral));
    assert!(!p.tasks.contains(&Task::PublishReserve));
}

#[test]
fn stale_prints_wake_queue_drain_but_do_not_create_healthy_heartbeats() {
    let p = healthy(1000).plan(1000 + cc::MARK_STALE_MS + 1).unwrap();
    assert_eq!(p.halt_to_add, cc::OPERATOR_DOWN | cc::HALT_ENTRIES);
    assert!(p.tasks.contains(&Task::ScanLiquidations));
    assert!(!p.tasks.contains(&Task::Heartbeat));
    let mut s = healthy(1000);
    s.scan_completed_ms = Some(3001);
    let p = s.plan(3001).unwrap();
    assert_eq!(p.halt_to_add, cc::HALT_ENTRIES);
    assert!(p.tasks.contains(&Task::Heartbeat));
    s.scan_completed_ms = Some(11_001);
    assert_eq!(
        s.plan(11_001).unwrap().halt_to_add,
        cc::HALT_ENTRIES | cc::OPERATOR_DOWN
    );
}

#[test]
fn future_or_backwards_clock_is_not_mistaken_for_freshness() {
    for field in 0..10 {
        let mut s = healthy(1000);
        let times = [
            &mut s.last_poll_ms,
            &mut s.feeds_refreshed_ms,
            &mut s.mark_observed_ms,
            &mut s.trader_observed_ms,
            &mut s.scan_completed_ms,
            &mut s.heartbeat_written_ms,
            &mut s.expiry_reconciled_ms,
            &mut s.collateral_checked_ms,
            &mut s.root_written_ms,
            &mut s.first_unpublished_ack_ms,
        ];
        *times.into_iter().nth(field).unwrap() = Some(2000);
        assert_eq!(s.plan(1999), Err(RuntimeError::Stale));
    }
    assert_eq!(healthy(1000).plan(0), Err(RuntimeError::Stale));
}

#[test]
fn funding_and_fold_are_driven_only_by_venue_facts_in_recovery_first_order() {
    let mut s = healthy(1000);
    assert!(!s
        .plan(86_401_000)
        .unwrap()
        .tasks
        .contains(&Task::FoldFunding));
    s.funding_interval_pending = true;
    s.venue_settlement_observed = true;
    s.halt_disagreement = true;
    s.incident_requires_root = true;
    assert_eq!(
        s.plan(1000).unwrap().tasks,
        vec![
            Task::RecoverOrders,
            Task::MirrorHalts,
            Task::AccrueFunding,
            Task::FoldFunding,
            Task::PublishReserve
        ]
    );
}

#[test]
fn roots_require_confirmed_ack_dirty_state_with_fill_time_or_incident_cadence() {
    let mut s = healthy(1000);
    assert!(!s
        .plan(31_000)
        .unwrap()
        .tasks
        .contains(&Task::PublishReserve));
    s.first_unpublished_ack_ms = Some(2000);
    s.unpublished_fills = 1;
    assert!(!s
        .plan(30_999)
        .unwrap()
        .tasks
        .contains(&Task::PublishReserve));
    assert!(s
        .plan(31_000)
        .unwrap()
        .tasks
        .contains(&Task::PublishReserve));
    s.unpublished_fills = 20;
    assert!(s.plan(2000).unwrap().tasks.contains(&Task::PublishReserve));
    s.root_written_ms = None;
    s.unpublished_fills = 1;
    assert!(!s
        .plan(31_999)
        .unwrap()
        .tasks
        .contains(&Task::PublishReserve));
    assert!(s
        .plan(32_000)
        .unwrap()
        .tasks
        .contains(&Task::PublishReserve));
    s.unpublished_fills = 0;
    s.incident_requires_root = true;
    assert!(s.plan(2000).unwrap().tasks.contains(&Task::PublishReserve));
}
