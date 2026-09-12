//! Mark-driven Cinder MM scanner (`docs/09-liquidation-liveness.md`).
//! Independent of `hedge_pending` and `crank_funding`. I1-safe: Book moves on ack.

use cinder_common as cc;

use crate::engine::{Adapter, HedgeOutcome, LiqQueueItem, PendingOid};
use crate::inflight::InFlight;
use crate::ledger::{FundingPort, LedgerPort};
use crate::phoenix::{MarketOrder, PhoenixVenue, PlaceResult};
use crate::residual::i1_holds;
use crate::{AdapterError, PubkeyBytes};

#[derive(Clone, Debug, Default)]
pub struct ScanReport {
    pub classified: usize,
    pub liquidated: Vec<(PubkeyBytes, u16)>,
    pub im_pass: Vec<(PubkeyBytes, u16)>,
    pub heartbeat: bool,
    pub halt: u8,
    pub hedges: Vec<HedgeOutcome>,
}

impl<P: PhoenixVenue, L: LedgerPort + FundingPort> Adapter<P, L> {
    /// One scan tick. `now_ms` is wall clock, not ER slots.
    pub fn scan_liquidations(
        &mut self,
        now_ms: u64,
        asset_id: u16,
    ) -> Result<ScanReport, AdapterError> {
        let mut report = ScanReport::default();
        let mark_age = self
            .mark_observed_at_ms
            .map(|t| now_ms.saturating_sub(t))
            .unwrap_or(u64::MAX);

        if let Some(prev) = self.last_in_process_scan_ms {
            if now_ms.saturating_sub(prev) > cc::SCAN_DEAD_MS {
                let flags = self.ledger.config_halt() | cc::OPERATOR_DOWN | cc::HALT_ENTRIES;
                self.ledger.write_halt(flags)?;
            }
        }
        self.last_in_process_scan_ms = Some(now_ms);

        if mark_age > cc::MARK_STALE_MS {
            let flags = self.ledger.config_halt() | cc::HALT_ENTRIES;
            self.ledger.write_halt(flags)?;
        }
        if mark_age > cc::MARK_DEAD_MS {
            let flags = self.ledger.config_halt() | cc::OPERATOR_DOWN | cc::HALT_ENTRIES;
            self.ledger.write_halt(flags)?;
        }

        if !self.ledger.invariant_ok() {
            report.halt = self.ledger.config_halt();
            self.maybe_heartbeat(now_ms, &mut report)?;
            return Ok(report);
        }

        let fresh = mark_age <= cc::MARK_STALE_MS;
        if fresh {
            self.queued = self.classify(asset_id);
            report.classified = self.queued.len();
        }
        // Soft/hard stale: drain the queue from the last fresh print. Do not reclassify.

        let mm_queue: Vec<LiqQueueItem> = self
            .queued
            .iter()
            .filter(|q| q.equity < q.mm && q.lots != 0)
            .cloned()
            .collect();
        self.queued.retain(|q| !(q.equity < q.mm && q.lots != 0));

        for item in mm_queue {
            if let Some(out) = self.flatten_and_hedge(now_ms, &item)? {
                report.liquidated.push((item.user, item.asset_id));
                report.hedges.push(out);
            }
        }

        let unsafe_pool = self.phoenix.pool_health().is_unsafe()
            || self.pool_health.is_unsafe();
        if unsafe_pool && self.phoenix.pool_health().is_unsafe() {
            let mut im_items: Vec<LiqQueueItem> = self
                .queued
                .iter()
                .filter(|q| q.equity < q.im && q.lots != 0)
                .cloned()
                .collect();
            im_items.sort_by_key(|q| q.equity - q.im);
            for item in im_items {
                if !self.phoenix.pool_health().is_unsafe() {
                    break;
                }
                if let Some(out) = self.flatten_and_hedge(now_ms, &item)? {
                    report.im_pass.push((item.user, item.asset_id));
                    report.hedges.push(out);
                }
                self.queued
                    .retain(|q| !(q.user == item.user && q.asset_id == item.asset_id));
            }
        }

        if unsafe_pool {
            let flags = self.ledger.config_halt() | cc::UNSAFE_POOL;
            self.ledger.write_halt(flags)?;
        }

        report.halt = self.ledger.config_halt();
        self.maybe_heartbeat(now_ms, &mut report)?;
        Ok(report)
    }

    fn classify(&self, asset_id: u16) -> Vec<LiqQueueItem> {
        let mut items = Vec::new();
        for user in self.ledger.user_ids() {
            let lots = self.ledger.lots_of(&user, asset_id);
            if lots == 0 {
                continue;
            }
            let equity = self.ledger.user_equity(&user);
            let mm = cc::stub_cinder_mm(lots.unsigned_abs()).unwrap_or(0) as i128;
            let im = cc::stub_cinder_im(lots.unsigned_abs()).unwrap_or(0) as i128;
            items.push(LiqQueueItem {
                user,
                asset_id,
                equity,
                mm,
                im,
                lots,
            });
        }
        items.sort_by_key(|q| q.equity - q.mm);
        items
    }

    fn flatten_and_hedge(
        &mut self,
        now_ms: u64,
        item: &LiqQueueItem,
    ) -> Result<Option<HedgeOutcome>, AdapterError> {
        let oid = self.next_liq_oid();
        self.ledger
            .liquidate_user(&item.user, item.asset_id, oid)?;
        let pending = PendingOid {
            user: item.user,
            client_oid: oid,
            asset_id: item.asset_id,
            lots_delta: item
                .lots
                .checked_neg()
                .ok_or_else(|| AdapterError::Ledger("liq delta overflow".into()))?,
            created_at_ms: now_ms,
        };
        let out = self.hedge_liq(now_ms, &pending)?;
        Ok(Some(out))
    }

    /// Reducing hedge: allowed under HALT_ENTRIES and unsafe pool. Does not
    /// set INVARIANT_BROKEN merely because a liq is in flight.
    pub fn hedge_liq(
        &mut self,
        now_ms: u64,
        oid: &PendingOid,
    ) -> Result<HedgeOutcome, AdapterError> {
        if !self.ledger.invariant_ok() {
            return Ok(HedgeOutcome::InvariantBroken {
                asset_id: oid.asset_id,
            });
        }
        self.inflight.insert(InFlight {
            user: oid.user,
            client_oid: oid.client_oid,
            asset_id: oid.asset_id,
            lots_delta: oid.lots_delta,
            inserted_at_ms: now_ms,
            venue_filled: false,
        });
        let order = MarketOrder {
            asset_id: oid.asset_id,
            lots: oid.lots_delta,
            client_oid: oid.client_oid,
        };
        let placed = self.phoenix.place_market(&order)?;
        match placed {
            PlaceResult::Reject { reason } => {
                self.ledger.ack_fail(&oid.user, &oid.client_oid)?;
                self.inflight.remove(&oid.client_oid);
                Ok(HedgeOutcome::Failed {
                    oid: oid.client_oid,
                    reason,
                })
            }
            PlaceResult::Fill(fill) => {
                if fill.filled_lots == 0 {
                    self.ledger.ack_fail(&oid.user, &oid.client_oid)?;
                    self.inflight.remove(&oid.client_oid);
                    return Ok(HedgeOutcome::Failed {
                        oid: oid.client_oid,
                        reason: "0-fill".into(),
                    });
                }
                self.inflight.mark_venue_filled(&oid.client_oid);
                self.ledger.ack_fill(&oid.user, &fill)?;
                self.inflight.remove(&oid.client_oid);
                if !i1_holds(&self.phoenix, &self.ledger, fill.asset_id) {
                    let flags = self.ledger.config_halt() | cc::INVARIANT_BROKEN;
                    self.ledger.write_halt(flags)?;
                    return Ok(HedgeOutcome::InvariantBroken {
                        asset_id: fill.asset_id,
                    });
                }
                self.note_fill_for_root(now_ms);
                let _ = self.crank_reserve_root(now_ms);
                Ok(HedgeOutcome::Filled(fill))
            }
        }
    }

    fn maybe_heartbeat(
        &mut self,
        now_ms: u64,
        report: &mut ScanReport,
    ) -> Result<(), AdapterError> {
        if now_ms.saturating_sub(self.last_heartbeat_ms) >= cc::HEARTBEAT_MS
            || self.last_heartbeat_ms == 0
        {
            self.ledger.write_last_scan_ms(now_ms);
            self.last_heartbeat_ms = now_ms;
            report.heartbeat = true;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Adapter;
    use crate::ledger::MemoryLedger;
    use crate::operator::{MockTeeAuth, OperatorAuth};
    use crate::phoenix::{MockPhoenix, PoolHealth};
    use crate::residual::i1_live;

    fn user(n: u8) -> PubkeyBytes {
        [n; 32]
    }

    fn adapter() -> Adapter<MockPhoenix, MemoryLedger> {
        let mut operator = OperatorAuth::new("http://127.0.0.1:6699");
        operator
            .authenticate(&MockTeeAuth { token: "op".into() }, &user(0), &|_| [0u8; 64])
            .unwrap();
        let mut phoenix = MockPhoenix::new();
        phoenix.auto_fill = true;
        Adapter::new(phoenix, MemoryLedger::new(), operator)
    }

    fn seed_long(ad: &mut Adapter<MockPhoenix, MemoryLedger>, u: PubkeyBytes, lots: i64, free: u64) {
        ad.ledger.ensure_user(u, free);
        ad.ledger.user_lots.insert(u, [(1, lots)].into_iter().collect());
        ad.ledger.book.insert(1, ad.ledger.book.get(&1).copied().unwrap_or(0) + lots);
        ad.phoenix.set_lots(1, ad.phoenix.base_lots(1) + lots);
        ad.mark_observed_at_ms = Some(1_000);
    }

    #[test]
    fn quiet_book_mark_loss_liquidates_and_i1() {
        let mut ad = adapter();
        let a = user(1);
        // 10 lots, stub MM = 625_000. Cash 100k is under MM even with uPnL=0.
        seed_long(&mut ad, a, 10, 100_000);
        let r = ad.scan_liquidations(1_000, 1).unwrap();
        assert_eq!(r.liquidated, vec![(a, 1)]);
        assert_eq!(ad.ledger.lots_of(&a, 1), 0);
        assert_eq!(ad.ledger.book_lots(1), ad.phoenix.base_lots(1));
        assert!(i1_holds(&ad.phoenix, &ad.ledger, 1));
    }

    #[test]
    fn upnl_loss_trips_mm_profit_does_not() {
        let mut ad = adapter();
        let loser = user(1);
        let winner = user(2);
        seed_long(&mut ad, loser, 10, 700_000);
        seed_long(&mut ad, winner, 10, 700_000);
        // MM = 625_000. Loser: entry high so mark < entry → negative uPnL.
        ad.ledger
            .user_entry
            .insert(loser, [(1, 10 * 1_000_000 + 200_000)].into_iter().collect());
        ad.ledger.mark_usdc_per_lot = 1_000_000;
        // Winner: entry below mark → positive uPnL, haircut 50%, still above MM.
        ad.ledger
            .user_entry
            .insert(winner, [(1, 10 * 1_000_000 - 400_000)].into_iter().collect());
        let r = ad.scan_liquidations(1_000, 1).unwrap();
        assert!(r.liquidated.contains(&(loser, 1)));
        assert!(!r.liquidated.contains(&(winner, 1)));
    }

    #[test]
    fn netted_only_underwater_user_flattened() {
        let mut ad = adapter();
        let a = user(1);
        let b = user(2);
        seed_long(&mut ad, a, 10, 100_000);
        seed_long(&mut ad, b, -10, 2_000_000);
        let r = ad.scan_liquidations(1_000, 1).unwrap();
        assert_eq!(r.liquidated, vec![(a, 1)]);
        assert_eq!(ad.ledger.lots_of(&b, 1), -10);
        assert_eq!(ad.ledger.book_lots(1), ad.phoenix.base_lots(1));
    }

    #[test]
    fn i1_in_flight_pending_liq_then_ack() {
        let mut ad = adapter();
        let a = user(1);
        seed_long(&mut ad, a, 10, 100_000);
        let oid = ad.next_liq_oid();
        ad.ledger.liquidate_user(&a, 1, oid).unwrap();
        assert_eq!(ad.ledger.lots_of(&a, 1), 0);
        assert_eq!(ad.ledger.book_lots(1), 10);
        assert_eq!(ad.phoenix.base_lots(1), 10);
        assert_eq!(ad.ledger.pending_liq_delta(1), -10);
        // Venue has not moved yet: live I1 is after the IOC.
        let pending = PendingOid {
            user: a,
            client_oid: oid,
            asset_id: 1,
            lots_delta: -10,
            created_at_ms: 1_000,
        };
        let out = ad.hedge_liq(1_000, &pending).unwrap();
        assert!(matches!(out, HedgeOutcome::Filled(_)));
        assert!(i1_holds(&ad.phoenix, &ad.ledger, 1));
        assert_eq!(ad.ledger.book_lots(1), 0);
    }

    #[test]
    fn liq_fail_ack_restores_user() {
        let mut ad = adapter();
        ad.phoenix.auto_fill = false;
        ad.phoenix.reject_next("no liq");
        let a = user(1);
        seed_long(&mut ad, a, 10, 100_000);
        let r = ad.scan_liquidations(1_000, 1).unwrap();
        assert!(matches!(r.hedges[0], HedgeOutcome::Failed { .. }));
        assert_eq!(ad.ledger.lots_of(&a, 1), 10);
        assert_eq!(ad.ledger.book_lots(1), 10);
    }

    #[test]
    fn stale_mark_drains_queue_does_not_reclassify() {
        let mut ad = adapter();
        let a = user(1);
        seed_long(&mut ad, a, 10, 100_000);
        let r0 = ad.scan_liquidations(1_000, 1).unwrap();
        assert_eq!(r0.liquidated.len(), 1);

        let b = user(2);
        seed_long(&mut ad, b, 10, 100_000);
        // Soft stale: do not newly classify B.
        ad.mark_observed_at_ms = Some(1_000);
        let r1 = ad.scan_liquidations(1_000 + cc::MARK_STALE_MS + 1, 1).unwrap();
        assert_eq!(r1.classified, 0);
        assert!(r1.liquidated.is_empty());
        assert_eq!(ad.ledger.lots_of(&b, 1), 10);
        assert_eq!(ad.ledger.config_halt() & cc::HALT_ENTRIES, cc::HALT_ENTRIES);
    }

    #[test]
    fn dead_scan_sets_operator_down() {
        let mut ad = adapter();
        ad.mark_observed_at_ms = Some(1_000);
        ad.scan_liquidations(1_000, 1).unwrap();
        let r = ad
            .scan_liquidations(1_000 + cc::SCAN_DEAD_MS + 1, 1)
            .unwrap();
        assert_eq!(r.halt & cc::OPERATOR_DOWN, cc::OPERATOR_DOWN);
    }

    #[test]
    fn unsafe_pool_mm_first_then_im_until_safe() {
        let mut ad = adapter();
        ad.phoenix.health = PoolHealth::Cancellable;
        ad.phoenix.flips_safe_after_places = Some(1);
        ad.pool_health = PoolHealth::Cancellable;
        let underwater = user(1);
        let tight = user(2);
        // underwater: equity 100k < MM 625k
        seed_long(&mut ad, underwater, 10, 100_000);
        // tight: 800k is ≥ MM 625k and < IM 1.25m
        seed_long(&mut ad, tight, 10, 800_000);
        let r = ad.scan_liquidations(1_000, 1).unwrap();
        assert!(r.liquidated.contains(&(underwater, 1)));
        // First hedge flipped pool to Safe, so IM pass should not run (or stop).
        assert!(r.im_pass.is_empty() || !r.im_pass.contains(&(tight, 1)));
        assert_eq!(ad.ledger.lots_of(&tight, 1), 10);
    }

    #[test]
    fn heartbeat_not_every_scan() {
        let mut ad = adapter();
        ad.mark_observed_at_ms = Some(1_000);
        let r0 = ad.scan_liquidations(1_000, 1).unwrap();
        assert!(r0.heartbeat);
        let r1 = ad.scan_liquidations(1_050, 1).unwrap();
        assert!(!r1.heartbeat);
        let r2 = ad.scan_liquidations(1_000 + cc::HEARTBEAT_MS, 1).unwrap();
        assert!(r2.heartbeat);
    }

    #[test]
    fn live_i1_helper() {
        assert!(i1_live(10, -10, 0));
        assert!(!i1_live(10, -10, 10));
    }
}
