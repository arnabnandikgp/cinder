use cinder_common as cc;

use crate::inflight::{InFlight, InFlightTable};
use crate::ledger::LedgerPort;
use crate::operator::OperatorAuth;
use crate::phoenix::{Fill, MarketOrder, PhoenixVenue, PlaceResult, PoolHealth};
use crate::residual::{i1_holds, i1_live};
use crate::{AdapterError, ClientOid, PubkeyBytes, RiskEngine, RiskSnapshot, UserRiskQuote};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingOid {
    pub user: PubkeyBytes,
    pub client_oid: ClientOid,
    pub asset_id: u16,
    pub lots_delta: i64,
    pub created_at_ms: u64,
    pub limit_price_ticks: u64,
    pub last_valid_slot: u64,
    /// Exact margin for the restored position if this tentative order fails.
    pub post_fail_position_im_usdc: u64,
}

/// The two values that the adapter co-signs into `place_order`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreflightOrder {
    pub post_position_im_usdc: u64,
    pub post_total_notional_usdc: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiqQueueItem {
    pub user: PubkeyBytes,
    pub asset_id: u16,
    pub equity: i128,
    pub mm: i128,
    pub im: i128,
    pub lots: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HedgeOutcome {
    Filled(Fill),
    Failed { oid: ClientOid, reason: String },
    InvariantBroken { asset_id: u16 },
}

pub struct Adapter<P, L, R> {
    pub phoenix: P,
    pub ledger: L,
    pub inflight: InFlightTable,
    pub operator: OperatorAuth,
    /// Last mark observation. None or older than MARK_STALE_MS rejects the hedge.
    pub mark_observed_at_ms: Option<u64>,
    /// Last Phoenix trader-state observation. None or older than
    /// TRADER_STATE_STALE_MS rejects a new hedge.
    pub trader_observed_at_ms: Option<u64>,
    pub pool_health: PoolHealth,
    pub fills_since_root: u64,
    pub last_root_ms: u64,
    pub last_phoenix_collateral: Option<u64>,
    pub liq_oid_seq: u64,
    pub last_in_process_scan_ms: Option<u64>,
    pub last_heartbeat_ms: u64,
    pub queued: Vec<LiqQueueItem>,
    pub risk: R,
    /// Latest validated normalized pooled-trader snapshots, used for venue
    /// expiry and residual checks.
    pub pool_risk_snapshots: std::collections::BTreeMap<u16, RiskSnapshot>,
    /// Latest validated normalized per-user Rise simulations.  A pool snapshot
    /// must never be re-used as a user's health snapshot.
    pub user_risk_snapshots: std::collections::BTreeMap<(PubkeyBytes, u16), RiskSnapshot>,
}

impl<P: PhoenixVenue, L: LedgerPort, R: RiskEngine> Adapter<P, L, R> {
    pub fn new(phoenix: P, ledger: L, operator: OperatorAuth, risk: R) -> Self {
        Self {
            phoenix,
            ledger,
            inflight: InFlightTable::default(),
            operator,
            mark_observed_at_ms: None,
            trader_observed_at_ms: None,
            pool_health: PoolHealth::Safe,
            fills_since_root: 0,
            last_root_ms: 0,
            last_phoenix_collateral: None,
            liq_oid_seq: 0,
            last_in_process_scan_ms: None,
            last_heartbeat_ms: 0,
            queued: Vec::new(),
            risk,
            pool_risk_snapshots: std::collections::BTreeMap::new(),
            user_risk_snapshots: std::collections::BTreeMap::new(),
        }
    }

    pub fn observe_risk_snapshot(&mut self, snapshot: RiskSnapshot) {
        self.pool_risk_snapshots.insert(snapshot.asset_id, snapshot);
    }

    pub fn observe_user_risk_snapshot(&mut self, user: PubkeyBytes, snapshot: RiskSnapshot) {
        self.user_risk_snapshots
            .insert((user, snapshot.asset_id), snapshot);
    }

    /// Produce the exact values co-signed into the tentative ledger order.
    /// The pool snapshot is separate because it describes the aggregate
    /// Phoenix cross trader, while the user snapshot describes the simulated
    /// user transition.
    pub fn preflight_order(
        &self,
        user_snapshot: &RiskSnapshot,
        pool_snapshot: &RiskSnapshot,
        current_lots: i64,
        lots_delta: i64,
        post_intended_residual_lots: i64,
        collateral_usdc: i128,
        now_ms: u64,
    ) -> Result<PreflightOrder, AdapterError> {
        if user_snapshot.asset_id != pool_snapshot.asset_id {
            return Err(AdapterError::Risk("user/pool asset mismatch".into()));
        }
        let post_lots = current_lots
            .checked_add(lots_delta)
            .ok_or_else(|| AdapterError::Risk("position overflow".into()))?;
        let user = self.risk.quote_user_after_order(
            user_snapshot,
            current_lots,
            post_lots,
            collateral_usdc,
            now_ms,
        )?;
        let pool = self.risk.quote_pool_after_residual(
            pool_snapshot,
            post_intended_residual_lots,
            now_ms,
        )?;
        if !pool.health.allows_new_hedge() {
            return Err(AdapterError::Risk(
                "post-residual Phoenix pool is not safe".into(),
            ));
        }
        if user.effective_equity_usdc < user.post_position_im_usdc as i128 {
            return Err(AdapterError::Risk("insufficient initial margin".into()));
        }
        let leverage = user_snapshot
            .first_tier_leverage
            .min(cc::MAX_USER_LEVERAGE as u64);
        let capacity = u128::try_from(user.effective_equity_usdc)
            .ok()
            .and_then(|equity| equity.checked_mul(leverage as u128))
            .ok_or_else(|| AdapterError::Risk("non-positive or overflowing collateral".into()))?;
        if user.total_notional_usdc as u128 > capacity {
            return Err(AdapterError::Risk("user leverage exceeded".into()));
        }
        Ok(PreflightOrder {
            post_position_im_usdc: user.post_position_im_usdc,
            post_total_notional_usdc: user.total_notional_usdc,
        })
    }

    pub(crate) fn quote_user_health(
        &self,
        user: &PubkeyBytes,
        asset_id: u16,
        lots: i64,
        now_ms: u64,
    ) -> Result<UserRiskQuote, AdapterError> {
        self.quote_user_health_with_collateral(
            user,
            asset_id,
            lots,
            self.ledger.user_collateral(user),
            now_ms,
        )
    }

    pub(crate) fn quote_user_health_with_collateral(
        &self,
        user: &PubkeyBytes,
        asset_id: u16,
        lots: i64,
        collateral_usdc: i128,
        now_ms: u64,
    ) -> Result<UserRiskQuote, AdapterError> {
        let snapshot = self.user_risk_snapshots.get(&(*user, asset_id));
        #[cfg(any(test, feature = "test-utils"))]
        let snapshot = snapshot.or_else(|| self.pool_risk_snapshots.get(&asset_id));
        let snapshot = snapshot
            .ok_or_else(|| AdapterError::Risk("missing Rise/Phoenix risk snapshot".into()))?;
        self.risk
            .quote_user_health(snapshot, lots, collateral_usdc, now_ms)
    }

    pub(crate) fn dispatch_rejection_reason(&self, oid: &PendingOid) -> Option<&'static str> {
        if oid.limit_price_ticks == 0 || oid.last_valid_slot == 0 {
            return Some("missing Phoenix bound/deadline");
        }
        let Some(snapshot) = self.pool_risk_snapshots.get(&oid.asset_id) else {
            return Some("missing Phoenix risk snapshot");
        };
        (snapshot.observed_slot > oid.last_valid_slot).then_some("Phoenix deadline expired")
    }

    fn entry_dispatch_rejection_reason(
        &self,
        now_ms: u64,
        oid: &PendingOid,
    ) -> Option<&'static str> {
        if let Some(reason) = self.dispatch_rejection_reason(oid) {
            return Some(reason);
        }
        let snapshot = self.pool_risk_snapshots.get(&oid.asset_id)?;
        let Some(age_ms) = now_ms.checked_sub(snapshot.observed_at_ms) else {
            return Some("future Phoenix risk snapshot");
        };
        (age_ms > cc::MARK_STALE_MS).then_some("stale Phoenix risk snapshot")
    }

    pub(crate) fn fill_breaches_bound(fill: &Fill, oid: &PendingOid) -> bool {
        fill.fill_price_ticks == 0
            || if fill.filled_lots > 0 {
                fill.fill_price_ticks > oid.limit_price_ticks
            } else {
                fill.fill_price_ticks < oid.limit_price_ticks
            }
    }

    pub fn next_liq_oid(&mut self) -> ClientOid {
        self.liq_oid_seq = self.liq_oid_seq.saturating_add(1);
        let mut oid = [0u8; 16];
        oid[0] = b'L';
        oid[8..16].copy_from_slice(&self.liq_oid_seq.to_le_bytes());
        oid
    }

    /// Record a fill toward the reserve-root cadence. Does **not** write or
    /// clear counters; [`Self::crank_reserve_root`] does that after success.
    pub fn note_fill_for_root(&mut self, now_ms: u64) -> bool {
        self.fills_since_root = self.fills_since_root.saturating_add(1);
        self.reserve_root_due(now_ms)
    }

    pub fn reserve_root_due(&self, now_ms: u64) -> bool {
        self.fills_since_root >= cc::COMMIT_EVERY_FILLS
            || now_ms.saturating_sub(self.last_root_ms) >= cc::COMMIT_EVERY_MS
    }

    /// Submit `write_reserve_root` when 20 fills or 30s have elapsed.
    /// Counters reset only after the write succeeds. Safe to call with no fills.
    pub fn crank_reserve_root(&mut self, now_ms: u64) -> Result<bool, AdapterError> {
        if !self.reserve_root_due(now_ms) {
            return Ok(false);
        }
        self.ledger.write_reserve_root(now_ms)?;
        self.fills_since_root = 0;
        self.last_root_ms = now_ms;
        Ok(true)
    }

    /// Window=0 hedge of one pending oid. Book is not moved here; `ack_*` does.
    pub fn hedge_pending(
        &mut self,
        now_ms: u64,
        oid: &PendingOid,
    ) -> Result<HedgeOutcome, AdapterError> {
        if now_ms.saturating_sub(oid.created_at_ms) > cc::OID_TTL_MS {
            self.ledger
                .ack_fail(&oid.user, &oid.client_oid, oid.post_fail_position_im_usdc)?;
            return Ok(HedgeOutcome::Failed {
                oid: oid.client_oid,
                reason: "oid ttl".into(),
            });
        }

        if let Some(reason) = self.entry_dispatch_rejection_reason(now_ms, oid) {
            self.ledger
                .ack_fail(&oid.user, &oid.client_oid, oid.post_fail_position_im_usdc)?;
            return Ok(HedgeOutcome::Failed {
                oid: oid.client_oid,
                reason: reason.into(),
            });
        }

        let mark_age = self
            .mark_observed_at_ms
            .map(|t| now_ms.saturating_sub(t))
            .unwrap_or(u64::MAX);
        if mark_age > cc::MARK_STALE_MS {
            let flags = self.ledger.config_halt() | cc::HALT_ENTRIES;
            self.ledger.write_halt(flags)?;
            return Ok(HedgeOutcome::Failed {
                oid: oid.client_oid,
                reason: "stale mark".into(),
            });
        }

        let trader_age = self
            .trader_observed_at_ms
            .map(|t| now_ms.saturating_sub(t))
            .unwrap_or(u64::MAX);
        if trader_age > cc::TRADER_STATE_STALE_MS {
            let flags = self.ledger.config_halt() | cc::HALT_ENTRIES;
            self.ledger.write_halt(flags)?;
            return Ok(HedgeOutcome::Failed {
                oid: oid.client_oid,
                reason: "stale trader state".into(),
            });
        }

        if !self.pool_health.allows_new_hedge() {
            return Ok(HedgeOutcome::Failed {
                oid: oid.client_oid,
                reason: "pool not safe".into(),
            });
        }

        if self
            .inflight
            .insert(InFlight {
                user: oid.user,
                client_oid: oid.client_oid,
                asset_id: oid.asset_id,
                lots_delta: oid.lots_delta,
                post_fail_position_im_usdc: oid.post_fail_position_im_usdc,
                inserted_at_ms: now_ms,
                venue_filled: false,
            })
            .is_err()
        {
            self.ledger
                .ack_fail(&oid.user, &oid.client_oid, oid.post_fail_position_im_usdc)?;
            return Ok(HedgeOutcome::Failed {
                oid: oid.client_oid,
                reason: "duplicate live client oid".into(),
            });
        }

        let order = MarketOrder {
            asset_id: oid.asset_id,
            lots: oid.lots_delta,
            client_oid: oid.client_oid,
            limit_price_ticks: oid.limit_price_ticks,
            last_valid_slot: oid.last_valid_slot,
        };
        let placed = self.phoenix.place_market(&order)?;
        match placed {
            PlaceResult::Reject { reason } => {
                self.ledger
                    .ack_fail(&oid.user, &oid.client_oid, oid.post_fail_position_im_usdc)?;
                self.inflight.remove(&oid.client_oid);
                Ok(HedgeOutcome::Failed {
                    oid: oid.client_oid,
                    reason,
                })
            }
            PlaceResult::Fill(fill) => {
                if fill.client_oid != oid.client_oid || fill.asset_id != oid.asset_id {
                    let flags = self.ledger.config_halt() | cc::INVARIANT_BROKEN;
                    self.ledger.write_halt(flags)?;
                    return Ok(HedgeOutcome::InvariantBroken {
                        asset_id: oid.asset_id,
                    });
                }
                if fill.filled_lots == 0 {
                    self.ledger.ack_fail(
                        &oid.user,
                        &oid.client_oid,
                        oid.post_fail_position_im_usdc,
                    )?;
                    self.inflight.remove(&oid.client_oid);
                    return Ok(HedgeOutcome::Failed {
                        oid: oid.client_oid,
                        reason: "0-fill".into(),
                    });
                }
                if fill.filled_lots.signum() != oid.lots_delta.signum()
                    || fill.filled_lots.unsigned_abs() > oid.lots_delta.unsigned_abs()
                {
                    let flags = self.ledger.config_halt() | cc::INVARIANT_BROKEN;
                    self.ledger.write_halt(flags)?;
                    return Ok(HedgeOutcome::InvariantBroken {
                        asset_id: oid.asset_id,
                    });
                }
                if !i1_live(
                    self.ledger.book_lots(fill.asset_id),
                    fill.filled_lots,
                    self.phoenix.base_lots(fill.asset_id),
                ) {
                    let flags = self.ledger.config_halt() | cc::INVARIANT_BROKEN;
                    self.ledger.write_halt(flags)?;
                    return Ok(HedgeOutcome::InvariantBroken {
                        asset_id: fill.asset_id,
                    });
                }
                self.inflight.mark_venue_filled(&oid.client_oid);
                self.ledger.ack_fill(&oid.user, &fill)?;
                self.inflight.remove(&oid.client_oid);
                if Self::fill_breaches_bound(&fill, oid) {
                    let flags = self.ledger.config_halt() | cc::VENUE_BREACH | cc::HALT_ENTRIES;
                    self.ledger.write_halt(flags)?;
                }
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

    /// Fail-ack in-flight rows past [`cc::IN_FLIGHT_TTL_MS`].
    pub fn expire_inflight(&mut self, now_ms: u64) -> Result<Vec<ClientOid>, AdapterError> {
        let stale = self.inflight.expired(now_ms);
        let mut oids = Vec::new();
        for row in stale {
            self.ledger
                .ack_fail(&row.user, &row.client_oid, row.post_fail_position_im_usdc)?;
            self.inflight.remove(&row.client_oid);
            oids.push(row.client_oid);
        }
        Ok(oids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::MemoryLedger;
    use crate::operator::MockTeeAuth;
    use crate::phoenix::{MockPhoenix, PoolHealth};
    use crate::{MarketStatus, RiskSnapshot, StubRiskEngine, UnitStatus};

    fn oid(n: u8) -> ClientOid {
        let mut o = [0u8; 16];
        o[0] = n;
        o
    }

    fn user() -> PubkeyBytes {
        [7u8; 32]
    }

    fn pending(n: u8, lots: i64, created_at_ms: u64) -> PendingOid {
        PendingOid {
            user: user(),
            client_oid: oid(n),
            asset_id: 1,
            lots_delta: lots,
            created_at_ms,
            limit_price_ticks: 100,
            last_valid_slot: 100,
            post_fail_position_im_usdc: 0,
        }
    }

    fn rise_snapshot(asset_id: u16, lots: i64, observed_at_ms: u64) -> RiskSnapshot {
        RiskSnapshot {
            asset_id,
            position_lots: lots,
            mark_price_ticks: 1_000_000,
            observed_slot: 10,
            observed_at_ms,
            market_status: MarketStatus::Active,
            units: UnitStatus::Verified,
            phoenix_initial_margin_usdc: lots.unsigned_abs() * 100_000,
            phoenix_maintenance_margin_usdc: lots.unsigned_abs() * 50_000,
            total_notional_usdc: lots.unsigned_abs() * 1_000_000,
            unrealized_pnl_usdc: 0,
            first_tier_leverage: 10,
            upnl_gain_factor_bps: Some(5_000),
            tick_size_in_quote_lots_per_base_lot: 1,
            quote_lot_to_usdc_numerator: 1,
            quote_lot_to_usdc_denominator: 1,
            post_pool_health: Some(PoolHealth::Safe),
        }
    }

    fn adapter_with(phoenix: MockPhoenix) -> Adapter<MockPhoenix, MemoryLedger, StubRiskEngine> {
        let mut operator = OperatorAuth::new("http://127.0.0.1:6699");
        operator
            .authenticate(&MockTeeAuth { token: "op".into() }, &user(), &|_| [0u8; 64])
            .unwrap();
        let mut ad = Adapter::new(phoenix, MemoryLedger::new(), operator, StubRiskEngine);
        ad.mark_observed_at_ms = Some(1_000);
        ad.trader_observed_at_ms = Some(1_000);
        ad.pool_health = PoolHealth::Safe;
        ad.observe_risk_snapshot(RiskSnapshot {
            asset_id: 1,
            position_lots: 0,
            mark_price_ticks: 100,
            observed_slot: 1,
            observed_at_ms: 1_000,
            market_status: MarketStatus::Active,
            units: UnitStatus::Verified,
            phoenix_initial_margin_usdc: 1,
            phoenix_maintenance_margin_usdc: 1,
            total_notional_usdc: 0,
            unrealized_pnl_usdc: 0,
            first_tier_leverage: 10,
            upnl_gain_factor_bps: Some(5_000),
            quote_lot_to_usdc_numerator: 1,
            quote_lot_to_usdc_denominator: 1,
            tick_size_in_quote_lots_per_base_lot: 1,
            post_pool_health: Some(PoolHealth::Safe),
        });
        ad
    }

    #[test]
    fn operator_holds_one_token_not_a_user_table() {
        let auth = MockTeeAuth {
            token: "tee-op".into(),
        };
        let mut op = OperatorAuth::new("http://127.0.0.1:6699");
        op.authenticate(&auth, &[1u8; 32], &|_| [0u8; 64]).unwrap();
        assert_eq!(op.token(), Some("tee-op"));
    }

    #[test]
    fn mock_fill_updates_book() {
        let mut phoenix = MockPhoenix::new();
        phoenix.fill_next(Fill {
            client_oid: oid(1),
            asset_id: 1,
            filled_lots: 10,
            fee_usdc: 0,
            vwap_quote_lots: 0,
            fill_price_ticks: 100,
            post_position_im_usdc: 1_250_000,
        });
        let mut ad = adapter_with(phoenix);
        let out = ad.hedge_pending(1_000, &pending(1, 10, 1_000)).unwrap();
        assert!(matches!(out, HedgeOutcome::Filled(_)));
        assert_eq!(ad.ledger.book_lots(1), 10);
        assert_eq!(ad.phoenix.base_lots(1), 10);
        assert!(ad.ledger.fails.is_empty());
        assert_eq!(ad.inflight.len(), 0);
    }

    #[test]
    fn preflight_requires_coherent_fresh_rise_snapshots() {
        let operator = OperatorAuth::new("http://127.0.0.1:6699");
        let adapter = Adapter::new(
            MockPhoenix::new(),
            MemoryLedger::new(),
            operator,
            crate::RiseRiskEngine,
        );
        let mut user_risk = rise_snapshot(1, 10, 1_000);
        // Includes five million of notional from the user's other positions.
        user_risk.total_notional_usdc = 15_000_000;
        let pool_risk = rise_snapshot(1, 10, 1_000);
        let quote = adapter
            .preflight_order(&user_risk, &pool_risk, 0, 10, 10, 2_000_000, 1_000)
            .unwrap();
        assert_eq!(quote.post_position_im_usdc, 1_250_000);
        assert_eq!(quote.post_total_notional_usdc, 15_000_000);

        let mut wrong_asset = pool_risk.clone();
        wrong_asset.asset_id = 2;
        assert!(adapter
            .preflight_order(&user_risk, &wrong_asset, 0, 10, 10, 2_000_000, 1_000)
            .is_err());
        let mut unsafe_pool = pool_risk.clone();
        unsafe_pool.post_pool_health = Some(PoolHealth::Cancellable);
        assert!(adapter
            .preflight_order(&user_risk, &unsafe_pool, 0, 10, 10, 2_000_000, 1_000)
            .is_err());
        assert!(adapter
            .preflight_order(
                &user_risk,
                &pool_risk,
                0,
                10,
                10,
                2_000_000,
                1_000 + cc::MARK_STALE_MS + 1,
            )
            .is_err());
    }

    #[test]
    fn buy_and_sell_bounds_are_directional_in_ticks() {
        let mut order = pending(40, 10, 1_000);
        order.limit_price_ticks = 100;
        let mut fill = Fill {
            client_oid: order.client_oid,
            asset_id: 1,
            filled_lots: 10,
            fee_usdc: 0,
            vwap_quote_lots: 0,
            fill_price_ticks: 100,
            post_position_im_usdc: 0,
        };
        assert!(
            !Adapter::<MockPhoenix, MemoryLedger, StubRiskEngine>::fill_breaches_bound(
                &fill, &order
            )
        );
        fill.fill_price_ticks = 99;
        assert!(
            !Adapter::<MockPhoenix, MemoryLedger, StubRiskEngine>::fill_breaches_bound(
                &fill, &order
            )
        );
        fill.fill_price_ticks = 101;
        assert!(
            Adapter::<MockPhoenix, MemoryLedger, StubRiskEngine>::fill_breaches_bound(
                &fill, &order
            )
        );

        order.lots_delta = -10;
        fill.filled_lots = -10;
        fill.fill_price_ticks = 100;
        assert!(
            !Adapter::<MockPhoenix, MemoryLedger, StubRiskEngine>::fill_breaches_bound(
                &fill, &order
            )
        );
        fill.fill_price_ticks = 101;
        assert!(
            !Adapter::<MockPhoenix, MemoryLedger, StubRiskEngine>::fill_breaches_bound(
                &fill, &order
            )
        );
        fill.fill_price_ticks = 99;
        assert!(
            Adapter::<MockPhoenix, MemoryLedger, StubRiskEngine>::fill_breaches_bound(
                &fill, &order
            )
        );
    }

    #[test]
    fn violating_confirmed_fill_is_acked_then_halts_entries() {
        let mut phoenix = MockPhoenix::new();
        phoenix.fill_next(Fill {
            client_oid: oid(41),
            asset_id: 1,
            filled_lots: 10,
            fee_usdc: 0,
            vwap_quote_lots: 0,
            fill_price_ticks: 101,
            post_position_im_usdc: 0,
        });
        let mut adapter = adapter_with(phoenix);
        let order = pending(41, 10, 1_000);
        assert!(matches!(
            adapter.hedge_pending(1_000, &order).unwrap(),
            HedgeOutcome::Filled(_)
        ));
        assert_eq!(adapter.ledger.fills.len(), 1);
        assert_eq!(adapter.ledger.book_lots(1), 10);
        assert_eq!(
            adapter.ledger.book_halt() & (cc::VENUE_BREACH | cc::HALT_ENTRIES),
            cc::VENUE_BREACH | cc::HALT_ENTRIES
        );
    }

    #[test]
    fn missing_or_expired_envelope_never_reaches_phoenix() {
        let mut adapter = adapter_with(MockPhoenix::new());
        let mut unbounded = pending(42, 10, 1_000);
        unbounded.limit_price_ticks = 0;
        let outcome = adapter.hedge_pending(1_000, &unbounded).unwrap();
        assert!(
            matches!(outcome, HedgeOutcome::Failed { reason, .. } if reason == "missing Phoenix bound/deadline")
        );

        let mut expired = pending(43, 10, 1_000);
        expired.last_valid_slot = 5;
        adapter
            .pool_risk_snapshots
            .get_mut(&1)
            .unwrap()
            .observed_slot = 6;
        let outcome = adapter.hedge_pending(1_000, &expired).unwrap();
        assert!(
            matches!(outcome, HedgeOutcome::Failed { reason, .. } if reason == "Phoenix deadline expired")
        );
        assert_eq!(adapter.ledger.fails.len(), 2);
        assert!(adapter.inflight.is_empty());
    }

    #[test]
    fn stale_risk_snapshot_never_reaches_phoenix() {
        let mut adapter = adapter_with(MockPhoenix::new());
        let now_ms = 1_000 + cc::MARK_STALE_MS + 1;
        // Keep the generic feed clocks fresh to prove dispatch independently
        // enforces the snapshot used for the Phoenix deadline.
        adapter.mark_observed_at_ms = Some(now_ms);
        adapter.trader_observed_at_ms = Some(now_ms);

        let outcome = adapter
            .hedge_pending(now_ms, &pending(44, 10, 1_000))
            .unwrap();
        assert!(
            matches!(outcome, HedgeOutcome::Failed { reason, .. } if reason == "stale Phoenix risk snapshot")
        );
        assert_eq!(adapter.ledger.fails.len(), 1);
        assert!(adapter.inflight.is_empty());
    }

    #[test]
    fn confirmed_fill_fee_becomes_debt_and_replay_is_a_noop() {
        let fill = Fill {
            client_oid: oid(2),
            asset_id: 1,
            filled_lots: 10,
            fee_usdc: 250,
            vwap_quote_lots: 10_000_000,
            fill_price_ticks: 100,
            post_position_im_usdc: 0,
        };
        let mut phoenix = MockPhoenix::new();
        phoenix.fill_next(fill.clone());
        let mut ad = adapter_with(phoenix);
        ad.ledger.ensure_user(user(), 100);

        let out = ad.hedge_pending(1_000, &pending(2, 10, 1_000)).unwrap();
        assert!(matches!(out, HedgeOutcome::Filled(_)));
        assert_eq!(ad.ledger.user_free[&user()], 0);
        assert_eq!(ad.ledger.user_bad_debt[&user()], 150);
        assert_eq!(ad.ledger.phoenix_fees_paid, 250);
        assert_eq!(
            ad.ledger.book_halt() & (cc::BAD_DEBT | cc::HALT_ENTRIES | cc::HALT_WITHDRAW),
            cc::BAD_DEBT | cc::HALT_ENTRIES | cc::HALT_WITHDRAW
        );
        assert_eq!(ad.ledger.config_halt(), ad.ledger.book_halt());

        ad.ledger.ack_fill(&user(), &fill).unwrap();
        assert_eq!(ad.ledger.fills.len(), 1);
        assert_eq!(ad.ledger.book_lots(1), 10);
        assert_eq!(ad.ledger.user_bad_debt[&user()], 150);
        assert_eq!(ad.ledger.phoenix_fees_paid, 250);
    }

    #[test]
    fn mock_reject_sends_ack_fail() {
        let mut phoenix = MockPhoenix::new();
        phoenix.reject_next("venue reject");
        let mut ad = adapter_with(phoenix);
        let out = ad.hedge_pending(1_000, &pending(2, 10, 1_000)).unwrap();
        assert!(matches!(out, HedgeOutcome::Failed { .. }));
        assert_eq!(ad.ledger.fails.len(), 1);
        assert_eq!(ad.ledger.book_lots(1), 0);
        assert!(ad.inflight.is_empty());
    }

    #[test]
    fn mock_i1_mismatch_sets_invariant_broken_on_book_and_config() {
        let mut phoenix = MockPhoenix::new();
        phoenix.apply_fill_to_position = false;
        phoenix.set_lots(1, 99);
        phoenix.fill_next(Fill {
            client_oid: oid(3),
            asset_id: 1,
            filled_lots: 10,
            fee_usdc: 0,
            vwap_quote_lots: 0,
            fill_price_ticks: 100,
            post_position_im_usdc: 0,
        });
        let mut ad = adapter_with(phoenix);
        let out = ad.hedge_pending(1_000, &pending(3, 10, 1_000)).unwrap();
        assert!(matches!(out, HedgeOutcome::InvariantBroken { asset_id: 1 }));
        assert_eq!(ad.ledger.book_lots(1), 0);
        assert_eq!(ad.phoenix.base_lots(1), 99);
        assert_eq!(
            ad.ledger.config_halt() & cc::INVARIANT_BROKEN,
            cc::INVARIANT_BROKEN
        );
        assert_eq!(
            ad.ledger.book_halt() & cc::INVARIANT_BROKEN,
            cc::INVARIANT_BROKEN
        );
        assert!(!ad.ledger.invariant_ok());
    }

    #[test]
    fn oid_older_than_15s_fail_acks() {
        let mut ad = adapter_with(MockPhoenix::new());
        let created = 0;
        let now = cc::OID_TTL_MS + 1;
        let out = ad.hedge_pending(now, &pending(4, 10, created)).unwrap();
        assert!(matches!(out, HedgeOutcome::Failed { reason, .. } if reason == "oid ttl"));
        assert_eq!(ad.ledger.fails.len(), 1);
        assert_eq!(ad.ledger.book_lots(1), 0);
        assert!(ad.inflight.is_empty());
    }

    #[test]
    fn halt_writer_mirrors_config_and_book() {
        let mut ad = adapter_with(MockPhoenix::new());
        ad.ledger
            .write_halt(cc::HALT_ENTRIES | cc::OPERATOR_DOWN)
            .unwrap();
        assert_eq!(ad.ledger.config_halt(), ad.ledger.book_halt());
        assert_eq!(
            ad.ledger.config_halt(),
            cc::HALT_ENTRIES | cc::OPERATOR_DOWN
        );
    }

    #[test]
    fn mismatched_fill_identity_halts_without_ack() {
        let mut phoenix = MockPhoenix::new();
        phoenix.fill_next(Fill {
            client_oid: oid(99),
            asset_id: 2,
            filled_lots: 10,
            fee_usdc: 0,
            vwap_quote_lots: 0,
            fill_price_ticks: 100,
            post_position_im_usdc: 0,
        });
        let mut ad = adapter_with(phoenix);
        let out = ad.hedge_pending(1_000, &pending(5, 10, 1_000)).unwrap();
        assert!(matches!(out, HedgeOutcome::InvariantBroken { asset_id: 1 }));
        assert!(ad.ledger.fills.is_empty());
        assert_eq!(ad.inflight.len(), 1);
        assert_eq!(
            ad.ledger.config_halt() & cc::INVARIANT_BROKEN,
            cc::INVARIANT_BROKEN
        );
    }

    #[test]
    fn expire_does_not_fail_ack_venue_filled_rows() {
        let mut ad = adapter_with(MockPhoenix::new());
        ad.inflight
            .insert(InFlight {
                user: user(),
                client_oid: oid(8),
                asset_id: 1,
                lots_delta: 10,
                post_fail_position_im_usdc: 0,
                inserted_at_ms: 0,
                venue_filled: true,
            })
            .unwrap();
        let expired = ad.expire_inflight(cc::IN_FLIGHT_TTL_MS + 10).unwrap();
        assert!(expired.is_empty());
        assert!(ad.ledger.fails.is_empty());
        assert_eq!(ad.inflight.len(), 1);
    }

    #[test]
    fn duplicate_live_oid_preserves_original_reconciliation_row() {
        let mut ad = adapter_with(MockPhoenix::new());
        let first = InFlight {
            user: user(),
            client_oid: oid(9),
            asset_id: 1,
            lots_delta: 10,
            post_fail_position_im_usdc: 0,
            inserted_at_ms: 1_000,
            venue_filled: false,
        };
        ad.inflight.insert(first.clone()).unwrap();
        let duplicate = PendingOid {
            user: [8u8; 32],
            client_oid: oid(9),
            asset_id: 1,
            lots_delta: -10,
            created_at_ms: 1_000,
            limit_price_ticks: 100,
            last_valid_slot: 100,
            post_fail_position_im_usdc: 0,
        };

        let outcome = ad.hedge_pending(1_000, &duplicate).unwrap();

        assert!(
            matches!(outcome, HedgeOutcome::Failed { reason, .. } if reason == "duplicate live client oid")
        );
        assert_eq!(ad.inflight.get(&oid(9)), Some(&first));
        assert_eq!(
            ad.ledger.fails,
            vec![(duplicate.user, duplicate.client_oid)]
        );
    }

    #[test]
    fn full_fill_updates_reserved_margin_and_reconciliation() {
        let mut phoenix = MockPhoenix::new();
        phoenix.fill_next(Fill {
            client_oid: oid(10),
            asset_id: 1,
            filled_lots: 10,
            fee_usdc: 0,
            vwap_quote_lots: 0,
            fill_price_ticks: 100,
            post_position_im_usdc: 1_250_000,
        });
        let mut ad = adapter_with(phoenix);
        let out = ad.hedge_pending(1_000, &pending(10, 10, 1_000)).unwrap();
        assert!(matches!(out, HedgeOutcome::Filled(_)));
        assert_eq!(ad.ledger.book_lots(1), ad.phoenix.base_lots(1));
        assert_eq!(ad.ledger.reserved.get(&1).copied(), Some(1_250_000));
    }

    #[test]
    fn zero_fill_restores_user_and_book_state() {
        let mut phoenix = MockPhoenix::new();
        phoenix.fill_next(Fill {
            client_oid: oid(11),
            asset_id: 1,
            filled_lots: 0,
            fee_usdc: 0,
            vwap_quote_lots: 0,
            fill_price_ticks: 100,
            post_position_im_usdc: 0,
        });
        let mut ad = adapter_with(phoenix);
        let out = ad.hedge_pending(1_000, &pending(11, 10, 1_000)).unwrap();
        assert!(matches!(out, HedgeOutcome::Failed { reason, .. } if reason == "0-fill"));
        assert_eq!(ad.ledger.book_lots(1), 0);
        assert_eq!(ad.phoenix.base_lots(1), 0);
        assert_eq!(ad.ledger.fails.len(), 1);
        assert!(ad.ledger.fills.is_empty());
    }

    #[test]
    fn partial_ioc_uses_filled_lots_only() {
        let mut phoenix = MockPhoenix::new();
        phoenix.fill_next(Fill {
            client_oid: oid(12),
            asset_id: 1,
            filled_lots: 4,
            fee_usdc: 0,
            vwap_quote_lots: 0,
            fill_price_ticks: 100,
            post_position_im_usdc: 0,
        });
        let mut ad = adapter_with(phoenix);
        let out = ad.hedge_pending(1_000, &pending(12, 10, 1_000)).unwrap();
        assert!(matches!(out, HedgeOutcome::Filled(ref f) if f.filled_lots == 4));
        assert_eq!(ad.ledger.book_lots(1), 4);
        assert_eq!(ad.phoenix.base_lots(1), 4);
    }

    #[test]
    fn stale_mark_halts_new_entries() {
        let mut ad = adapter_with(MockPhoenix::new());
        ad.mark_observed_at_ms = Some(0);
        let out = ad
            .hedge_pending(
                cc::MARK_STALE_MS + 1,
                &pending(13, 10, cc::MARK_STALE_MS + 1),
            )
            .unwrap();
        assert!(matches!(out, HedgeOutcome::Failed { reason, .. } if reason == "stale mark"));
        assert_eq!(ad.ledger.config_halt() & cc::HALT_ENTRIES, cc::HALT_ENTRIES);
        assert_eq!(ad.ledger.book_halt() & cc::HALT_ENTRIES, cc::HALT_ENTRIES);
        assert_eq!(ad.ledger.book_lots(1), 0);
        assert!(ad.ledger.fails.is_empty());
    }

    #[test]
    fn offsetting_users_match_phoenix_and_collateral_invariant() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        let mut phoenix = MockPhoenix::new();
        phoenix.fill_next(Fill {
            client_oid: oid(20),
            asset_id: 1,
            filled_lots: 10,
            fee_usdc: 0,
            vwap_quote_lots: 0,
            fill_price_ticks: 100,
            post_position_im_usdc: 0,
        });
        let mut ad = adapter_with(phoenix);
        ad.hedge_pending(
            1_000,
            &PendingOid {
                user: a,
                client_oid: oid(20),
                asset_id: 1,
                lots_delta: 10,
                created_at_ms: 1_000,
                limit_price_ticks: 100,
                last_valid_slot: 100,
                post_fail_position_im_usdc: 0,
            },
        )
        .unwrap();
        ad.phoenix.fill_next(Fill {
            client_oid: oid(21),
            asset_id: 1,
            filled_lots: -10,
            fee_usdc: 0,
            vwap_quote_lots: 0,
            fill_price_ticks: 100,
            post_position_im_usdc: 0,
        });
        ad.hedge_pending(
            1_000,
            &PendingOid {
                user: b,
                client_oid: oid(21),
                asset_id: 1,
                lots_delta: -10,
                created_at_ms: 1_000,
                limit_price_ticks: 100,
                last_valid_slot: 100,
                post_fail_position_im_usdc: 0,
            },
        )
        .unwrap();

        assert_eq!(ad.ledger.lots_of(&a, 1), 10);
        assert_eq!(ad.ledger.lots_of(&b, 1), -10);
        assert_eq!(
            ad.phoenix.base_lots(1),
            ad.ledger.lots_of(&a, 1) + ad.ledger.lots_of(&b, 1)
        );
        assert_eq!(ad.phoenix.base_lots(1), ad.ledger.book_lots(1));
        assert_eq!(ad.phoenix.base_lots(1), 0);

        ad.ledger.user_cash.insert(a, 100);
        ad.ledger.user_cash.insert(b, 100);
        ad.ledger.vault_ata = 150;
        ad.ledger.phoenix_collateral = 50;
        assert!(ad.ledger.i2_ok(0));
        ad.ledger.vault_ata = 140;
        assert!(ad.ledger.i2_ok(10));
        assert!(!ad.ledger.i2_ok(0));
    }

    #[test]
    fn reserve_root_crank_runs_by_fill_count_or_time() {
        let mut ad = adapter_with(MockPhoenix::new());
        ad.last_root_ms = 1_000;
        for _ in 0..19 {
            assert!(!ad.note_fill_for_root(1_000));
        }
        assert!(ad.note_fill_for_root(1_000));
        assert_eq!(ad.fills_since_root, 20);
        assert!(ad.ledger.reserve_roots.is_empty());
        assert!(ad.crank_reserve_root(1_000).unwrap());
        assert_eq!(ad.fills_since_root, 0);
        assert_eq!(ad.ledger.reserve_roots, vec![1_000]);
        assert!(!ad.note_fill_for_root(1_000));
        assert!(ad.crank_reserve_root(1_000 + cc::COMMIT_EVERY_MS).unwrap());
        assert_eq!(
            ad.ledger.reserve_roots,
            vec![1_000, 1_000 + cc::COMMIT_EVERY_MS]
        );
    }

    #[test]
    fn reserve_root_time_crank_without_fills() {
        let mut ad = adapter_with(MockPhoenix::new());
        ad.last_root_ms = 0;
        assert!(ad.reserve_root_due(cc::COMMIT_EVERY_MS));
        assert!(ad.crank_reserve_root(cc::COMMIT_EVERY_MS).unwrap());
        assert_eq!(ad.ledger.reserve_roots, vec![cc::COMMIT_EVERY_MS]);
        assert!(!ad.crank_reserve_root(cc::COMMIT_EVERY_MS).unwrap());
    }

    #[test]
    fn reserve_root_stays_due_if_write_fails() {
        let mut ad = adapter_with(MockPhoenix::new());
        ad.last_root_ms = 1_000;
        for _ in 0..20 {
            ad.note_fill_for_root(1_000);
        }
        ad.ledger.fail_next_root = true;
        assert!(ad.crank_reserve_root(1_000).is_err());
        assert_eq!(ad.fills_since_root, 20);
        assert!(ad.ledger.reserve_roots.is_empty());
        assert!(ad.crank_reserve_root(1_000).unwrap());
        assert_eq!(ad.fills_since_root, 0);
        assert_eq!(ad.ledger.reserve_roots.len(), 1);
    }
}
