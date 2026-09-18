//! Pure maintenance decisions. No timer may bypass the mutation coordinator.
//!
//! Planning is not completion: timestamps advance only after the corresponding
//! work succeeds. These decisions do not grant execution or entry permission.
use crate::rise::RiseView;
use crate::rpc::{Result, RuntimeError};
use crate::runtime::validate_ledger;
use cinder_common as cc;
use cinder_ledger::UserLedger;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaintenanceTask {
    RecoverOrders,
    RefreshFeeds,
    AccrueFunding,
    FoldFunding,
    ScanLiquidations,
    ReconcileExpiry,
    CheckCollateral,
    MirrorHalts,
    Heartbeat,
    PublishReserve,
}

/// Successful-work timestamps are Unix milliseconds, NEVER L1/ER slots.
/// Funding flags are set by authoritative venue observations, not elapsed time.
#[derive(Clone, Debug, Default)]
pub struct MaintenanceProgress {
    pub last_poll_ms: Option<u64>,
    pub feeds_refreshed_ms: Option<u64>,
    pub mark_observed_ms: Option<u64>,
    pub trader_observed_ms: Option<u64>,
    pub scan_completed_ms: Option<u64>,
    pub heartbeat_written_ms: Option<u64>,
    pub expiry_reconciled_ms: Option<u64>,
    pub collateral_checked_ms: Option<u64>,
    pub root_written_ms: Option<u64>,
    pub first_unpublished_ack_ms: Option<u64>,
    pub unpublished_fills: u64,
    pub incident_requires_root: bool,
    pub funding_interval_pending: bool,
    pub venue_settlement_observed: bool,
    pub halt_disagreement: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaintenancePlan {
    /// Bits to add atomically. Planning never clears administrative/debt halts.
    pub halt_to_add: u8,
    /// Coalesced, bounded intents in coordinator priority order. Recovery wins
    /// over every maintenance mutation, including publication and heartbeats.
    /// Execute at most one mutating intent, then reobserve and replan; this is
    /// not permission to run later intents against a pre-mutation snapshot.
    pub tasks: Vec<MaintenanceTask>,
}

/// Aggregate health only; user identities and rankings stay private.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaintenanceHealth {
    pub users_scanned: usize,
    pub confirmed_positions_scanned: usize,
    pub users_below_mm: usize,
    pub users_below_im: usize,
}

impl MaintenanceProgress {
    pub fn plan(&self, now_ms: u64) -> Result<MaintenancePlan> {
        let timestamps = [
            self.last_poll_ms,
            self.feeds_refreshed_ms,
            self.mark_observed_ms,
            self.trader_observed_ms,
            self.scan_completed_ms,
            self.heartbeat_written_ms,
            self.expiry_reconciled_ms,
            self.collateral_checked_ms,
            self.root_written_ms,
            self.first_unpublished_ack_ms,
        ];
        if now_ms == 0
            || timestamps
                .into_iter()
                .flatten()
                .any(|t| t == 0 || t > now_ms)
        {
            return Err(RuntimeError::Stale);
        }
        let age = |time: Option<u64>| time.map(|t| now_ms - t).unwrap_or(u64::MAX);
        let mut halt = 0;
        if age(self.mark_observed_ms) > cc::MARK_STALE_MS
            || age(self.trader_observed_ms) > cc::TRADER_STATE_STALE_MS
        {
            halt |= cc::HALT_ENTRIES;
        }
        if age(self.mark_observed_ms) > cc::MARK_DEAD_MS
            || age(self.scan_completed_ms) > cc::SCAN_DEAD_MS
        {
            halt |= cc::OPERATOR_DOWN | cc::HALT_ENTRIES;
        }
        let mut tasks = vec![MaintenanceTask::RecoverOrders];
        if age(self.feeds_refreshed_ms) >= cc::SCAN_INTERVAL_MS {
            tasks.push(MaintenanceTask::RefreshFeeds);
        }
        if halt != 0 || self.halt_disagreement {
            tasks.push(MaintenanceTask::MirrorHalts);
        }
        if self.funding_interval_pending {
            tasks.push(MaintenanceTask::AccrueFunding);
        }
        if self.venue_settlement_observed {
            tasks.push(MaintenanceTask::FoldFunding);
        }
        // Stale marks still wake the scanner so it can drain the last fresh
        // queue. The scanner must not reclassify books against a stale print.
        if age(self.scan_completed_ms) >= cc::SCAN_INTERVAL_MS {
            tasks.push(MaintenanceTask::ScanLiquidations);
        }
        if age(self.expiry_reconciled_ms) >= cc::HEARTBEAT_MS {
            tasks.push(MaintenanceTask::ReconcileExpiry);
        }
        if age(self.collateral_checked_ms) >= cc::HEARTBEAT_MS {
            tasks.push(MaintenanceTask::CheckCollateral);
        }
        // Never mint a healthy heartbeat just because a timer fired. Only a
        // successfully completed, still-live scan may advance Book.last_scan_ms.
        if age(self.scan_completed_ms) <= cc::SCAN_DEAD_MS
            && age(self.heartbeat_written_ms) >= cc::HEARTBEAT_MS
        {
            tasks.push(MaintenanceTask::Heartbeat);
        }
        let unpublished = self.first_unpublished_ack_ms;
        if self.incident_requires_root
            || (self.unpublished_fills != 0
                && (self.unpublished_fills >= cc::COMMIT_EVERY_FILLS
                    || age(self.root_written_ms.or(unpublished)) >= cc::COMMIT_EVERY_MS))
        {
            tasks.push(MaintenanceTask::PublishReserve);
        }
        Ok(MaintenancePlan {
            halt_to_add: halt,
            tasks,
        })
    }
}

/// Private decision data. Never serialize this into public logs or a journal.
pub(crate) struct UserHealth {
    pub ledger: [u8; 32],
    pub effective_equity_usdc: i128,
    pub initial_margin_usdc: u64,
    pub maintenance_margin_usdc: u64,
    pub confirmed_positions: Vec<(u16, i64)>,
}
impl UserHealth {
    pub fn below_mm(&self) -> bool {
        self.effective_equity_usdc < i128::from(self.maintenance_margin_usdc)
    }
}

/// Rank whole USERS worst-MM-deficit first. Counting a user's cash once per
/// asset grants fictitious collateral to multi-market books. Tentative lots do
/// not enter health; gains use the shared SDK risk factor and losses stay full.
pub(crate) fn scan_user_health(
    view: &RiseView,
    ledgers: &[([u8; 32], UserLedger)],
    now_ms: u64,
) -> Result<Vec<UserHealth>> {
    if [view.mark_ms, view.observed_ms].into_iter().any(|t| {
        t == 0
            || now_ms
                .checked_sub(t)
                .is_none_or(|age| age > cc::MARK_STALE_MS)
    }) {
        return Err(RuntimeError::Stale);
    }
    let mut users = BTreeSet::new();
    let mut ranked = Vec::with_capacity(ledgers.len());
    for (address, ledger) in ledgers {
        validate_ledger(ledger)?;
        if !users.insert(*address) {
            return Err(RuntimeError::Identity);
        }
        let value = crate::solvency::value_user(view, ledger, now_ms)?;
        let health = UserHealth {
            ledger: *address,
            effective_equity_usdc: value.effective_equity,
            initial_margin_usdc: value.initial_margin,
            maintenance_margin_usdc: value.maintenance_margin,
            confirmed_positions: value.confirmed_positions,
        };
        ranked.push(health);
    }
    // Sort comparisons without subtracting two potentially extreme i128s.
    let mut keyed = ranked
        .into_iter()
        .map(|h| {
            h.effective_equity_usdc
                .checked_sub(i128::from(h.maintenance_margin_usdc))
                .map(|deficit| (deficit, h))
                .ok_or(RuntimeError::Decode)
        })
        .collect::<Result<Vec<_>>>()?;
    keyed.sort_by_key(|(deficit, h)| (*deficit, h.ledger));
    Ok(keyed.into_iter().map(|(_, h)| h).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvency::tests::{user, view};
    const USDC: u64 = 1_000_000;

    #[test]
    fn netted_pool_safe_does_not_hide_a_quiet_book_user_mm_breach() {
        let venue = view(130, 400);
        let ranked = scan_user_health(
            &venue,
            &[([1; 32], user(10, 200)), ([2; 32], user(-10, 200))],
            1000,
        )
        .unwrap();
        assert!(venue.safe && venue.positions.is_empty());
        assert_eq!(ranked[0].ledger, [2; 32]);
        assert!(ranked[0].below_mm());
        assert_eq!(ranked[0].effective_equity_usdc, -100 * i128::from(USDC));
        assert!(!ranked[1].below_mm());
        assert_eq!(ranked[1].effective_equity_usdc, 350 * i128::from(USDC));
    }

    #[test]
    fn multi_market_cash_counts_once_and_health_sums_all_margin_and_funding() {
        let mut venue = view(100, 200);
        venue.markets.insert(2, venue.markets[&1].clone());
        let mut ledger = user(10, 200);
        ledger.positions_len = 2;
        ledger.positions[1] = cinder_ledger::Position {
            asset_id: 2,
            lots: 10,
            entry_quote_lots: 1000 * USDC as i64,
            ..Default::default()
        };
        ledger.positions[0].unsettled_funding = -10 * USDC as i64;
        ledger.positions[1].unsettled_funding = 5 * USDC as i64;
        let h = scan_user_health(&venue, &[([1; 32], ledger)], 1000)
            .unwrap()
            .remove(0);
        assert_eq!(h.effective_equity_usdc, 195 * i128::from(USDC));
        assert_eq!(h.initial_margin_usdc, 250 * USDC);
        assert_eq!(h.confirmed_positions, vec![(1, 10), (2, 10)]);
        assert!(h.maintenance_margin_usdc > 0);
    }

    #[test]
    fn tentative_lots_never_pay_funding_or_change_confirmed_health() {
        let venue = view(100, 200);
        let mut ledger = user(10, 200);
        ledger.positions[0].lots = 20;
        ledger.pending_oid_count = 1;
        ledger.open_oids[0] = cinder_ledger::OpenOid {
            asset_id: 1,
            lots_delta: 10,
            state: cc::OID_PENDING,
            client_oid: [9; 16],
            ..Default::default()
        };
        let h = scan_user_health(&venue, &[([1; 32], ledger)], 1000)
            .unwrap()
            .remove(0);
        assert_eq!(h.confirmed_positions, vec![(1, 10)]);
        assert_eq!(h.effective_equity_usdc, 200 * i128::from(USDC));
        assert_eq!(h.initial_margin_usdc, 125 * USDC);
    }

    #[test]
    fn debt_and_unsettled_funding_enter_mm_health() {
        let venue = view(100, 200);
        let mut ledger = user(10, 200);
        ledger.free = 20 * USDC;
        ledger.reserved = 10 * USDC;
        ledger.bad_debt_usdc = 40 * USDC;
        ledger.positions[0].unsettled_funding = -5 * USDC as i64;
        let h = scan_user_health(&venue, &[([1; 32], ledger)], 1000)
            .unwrap()
            .remove(0);
        assert_eq!(h.effective_equity_usdc, -15 * i128::from(USDC));
        assert!(h.below_mm());
    }

    #[test]
    fn mm_boundary_is_strict_and_order_is_stable_for_equal_deficits() {
        let venue = view(100, 0);
        let im = scan_user_health(&venue, &[([1; 32], user(10, 0))], 1000)
            .unwrap()
            .remove(0)
            .maintenance_margin_usdc;
        let make = || {
            let mut ledger = user(10, 0);
            ledger.free = im;
            ledger
        };
        let ranked =
            scan_user_health(&venue, &[([2; 32], make()), ([1; 32], make())], 1000).unwrap();
        assert_eq!(ranked[0].ledger, [1; 32]);
        assert!(!ranked[0].below_mm());
    }

    #[test]
    fn stale_future_duplicate_or_incomplete_views_fail_closed() {
        let mut venue = view(100, 200);
        for now in [999, 3001] {
            assert!(scan_user_health(&venue, &[([1; 32], user(10, 200))], now).is_err());
        }
        assert!(scan_user_health(
            &venue,
            &[([1; 32], user(10, 200)), ([1; 32], user(10, 200))],
            1000
        )
        .is_err());
        venue.markets.clear();
        assert!(scan_user_health(&venue, &[([1; 32], user(10, 200))], 1000).is_err());
    }
}
