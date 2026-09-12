use cinder_common as cc;

use crate::inflight::{InFlight, InFlightTable};
use crate::ledger::LedgerPort;
use crate::operator::OperatorAuth;
use crate::phoenix::{Fill, MarketOrder, PhoenixVenue, PlaceResult, PoolHealth};
use crate::residual::i1_holds;
use crate::{AdapterError, ClientOid, PubkeyBytes};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingOid {
    pub user: PubkeyBytes,
    pub client_oid: ClientOid,
    pub asset_id: u16,
    pub lots_delta: i64,
    pub created_at_ms: u64,
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

pub struct Adapter<P, L> {
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
}

impl<P: PhoenixVenue, L: LedgerPort> Adapter<P, L> {
    pub fn new(phoenix: P, ledger: L, operator: OperatorAuth) -> Self {
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
            self.ledger.ack_fail(&oid.user, &oid.client_oid)?;
            return Ok(HedgeOutcome::Failed {
                oid: oid.client_oid,
                reason: "oid ttl".into(),
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
                if fill.client_oid != oid.client_oid || fill.asset_id != oid.asset_id {
                    let flags = self.ledger.config_halt() | cc::INVARIANT_BROKEN;
                    self.ledger.write_halt(flags)?;
                    return Ok(HedgeOutcome::InvariantBroken {
                        asset_id: oid.asset_id,
                    });
                }
                if fill.filled_lots == 0 {
                    self.ledger.ack_fail(&oid.user, &oid.client_oid)?;
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

    /// Fail-ack in-flight rows past [`cc::IN_FLIGHT_TTL_MS`].
    pub fn expire_inflight(&mut self, now_ms: u64) -> Result<Vec<ClientOid>, AdapterError> {
        let stale = self.inflight.expired(now_ms);
        let mut oids = Vec::new();
        for row in stale {
            self.ledger.ack_fail(&row.user, &row.client_oid)?;
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
        }
    }

    fn adapter_with(phoenix: MockPhoenix) -> Adapter<MockPhoenix, MemoryLedger> {
        let mut operator = OperatorAuth::new("http://127.0.0.1:6699");
        operator
            .authenticate(&MockTeeAuth { token: "op".into() }, &user(), &|_| [0u8; 64])
            .unwrap();
        let mut ad = Adapter::new(phoenix, MemoryLedger::new(), operator);
        ad.mark_observed_at_ms = Some(1_000);
        ad.trader_observed_at_ms = Some(1_000);
        ad.pool_health = PoolHealth::Safe;
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
        });
        let mut ad = adapter_with(phoenix);
        let out = ad.hedge_pending(1_000, &pending(3, 10, 1_000)).unwrap();
        assert!(matches!(out, HedgeOutcome::InvariantBroken { asset_id: 1 }));
        assert_eq!(ad.ledger.book_lots(1), 10);
        assert_eq!(ad.phoenix.base_lots(1), 99);
        assert_eq!(ad.ledger.config_halt() & cc::INVARIANT_BROKEN, cc::INVARIANT_BROKEN);
        assert_eq!(ad.ledger.book_halt() & cc::INVARIANT_BROKEN, cc::INVARIANT_BROKEN);
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
        ad.ledger.write_halt(cc::HALT_ENTRIES | cc::OPERATOR_DOWN).unwrap();
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
        });
        let mut ad = adapter_with(phoenix);
        let out = ad.hedge_pending(1_000, &pending(5, 10, 1_000)).unwrap();
        assert!(matches!(out, HedgeOutcome::InvariantBroken { asset_id: 1 }));
        assert!(ad.ledger.fills.is_empty());
        assert_eq!(ad.inflight.len(), 1);
        assert_eq!(ad.ledger.config_halt() & cc::INVARIANT_BROKEN, cc::INVARIANT_BROKEN);
    }

    #[test]
    fn expire_does_not_fail_ack_venue_filled_rows() {
        let mut ad = adapter_with(MockPhoenix::new());
        ad.inflight.insert(InFlight {
            user: user(),
            client_oid: oid(8),
            asset_id: 1,
            lots_delta: 10,
            inserted_at_ms: 0,
            venue_filled: true,
        });
        let expired = ad.expire_inflight(cc::IN_FLIGHT_TTL_MS + 10).unwrap();
        assert!(expired.is_empty());
        assert!(ad.ledger.fails.is_empty());
        assert_eq!(ad.inflight.len(), 1);
    }

    #[test]
    fn s6_i1_and_reserved_im_after_full_fill() {
        let mut phoenix = MockPhoenix::new();
        phoenix.fill_next(Fill {
            client_oid: oid(10),
            asset_id: 1,
            filled_lots: 10,
            fee_usdc: 0,
            vwap_quote_lots: 0,
        });
        let mut ad = adapter_with(phoenix);
        let out = ad.hedge_pending(1_000, &pending(10, 10, 1_000)).unwrap();
        assert!(matches!(out, HedgeOutcome::Filled(_)));
        assert_eq!(ad.ledger.book_lots(1), ad.phoenix.base_lots(1));
        let want = cc::stub_cinder_im(10).unwrap();
        assert!(ad.ledger.reserved.get(&1).copied().unwrap() >= want);
    }

    #[test]
    fn s6_zero_fill_restores_user_book_unchanged() {
        let mut phoenix = MockPhoenix::new();
        phoenix.fill_next(Fill {
            client_oid: oid(11),
            asset_id: 1,
            filled_lots: 0,
            fee_usdc: 0,
            vwap_quote_lots: 0,
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
    fn s6_partial_ioc_uses_filled_lots_only() {
        let mut phoenix = MockPhoenix::new();
        phoenix.fill_next(Fill {
            client_oid: oid(12),
            asset_id: 1,
            filled_lots: 4,
            fee_usdc: 0,
            vwap_quote_lots: 0,
        });
        let mut ad = adapter_with(phoenix);
        let out = ad.hedge_pending(1_000, &pending(12, 10, 1_000)).unwrap();
        assert!(matches!(out, HedgeOutcome::Filled(ref f) if f.filled_lots == 4));
        assert_eq!(ad.ledger.book_lots(1), 4);
        assert_eq!(ad.phoenix.base_lots(1), 4);
    }

    #[test]
    fn s6_stale_mark_sets_halt_entries() {
        let mut ad = adapter_with(MockPhoenix::new());
        ad.mark_observed_at_ms = Some(0);
        let out = ad
            .hedge_pending(cc::MARK_STALE_MS + 1, &pending(13, 10, cc::MARK_STALE_MS + 1))
            .unwrap();
        assert!(matches!(out, HedgeOutcome::Failed { reason, .. } if reason == "stale mark"));
        assert_eq!(ad.ledger.config_halt() & cc::HALT_ENTRIES, cc::HALT_ENTRIES);
        assert_eq!(ad.ledger.book_halt() & cc::HALT_ENTRIES, cc::HALT_ENTRIES);
        assert_eq!(ad.ledger.book_lots(1), 0);
        assert!(ad.ledger.fails.is_empty());
    }

    #[test]
    fn s7_offsetting_users_phoenix_equals_sum_and_i2() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        let mut phoenix = MockPhoenix::new();
        phoenix.fill_next(Fill {
            client_oid: oid(20),
            asset_id: 1,
            filled_lots: 10,
            fee_usdc: 0,
            vwap_quote_lots: 0,
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
            },
        )
        .unwrap();
        ad.phoenix.fill_next(Fill {
            client_oid: oid(21),
            asset_id: 1,
            filled_lots: -10,
            fee_usdc: 0,
            vwap_quote_lots: 0,
        });
        ad.hedge_pending(
            1_000,
            &PendingOid {
                user: b,
                client_oid: oid(21),
                asset_id: 1,
                lots_delta: -10,
                created_at_ms: 1_000,
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
    fn s8_root_crank_every_20_fills_or_30s() {
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
        assert_eq!(ad.ledger.reserve_roots, vec![1_000, 1_000 + cc::COMMIT_EVERY_MS]);
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
