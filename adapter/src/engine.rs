use cinder_common as cc;

use crate::inflight::{InFlight, InFlightTable};
use crate::ledger::LedgerPort;
use crate::operator::OperatorAuth;
use crate::phoenix::{Fill, MarketOrder, PhoenixVenue, PlaceResult};
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
}

impl<P: PhoenixVenue, L: LedgerPort> Adapter<P, L> {
    pub fn new(phoenix: P, ledger: L, operator: OperatorAuth) -> Self {
        Self {
            phoenix,
            ledger,
            inflight: InFlightTable::default(),
            operator,
        }
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

        self.inflight.insert(InFlight {
            user: oid.user,
            client_oid: oid.client_oid,
            asset_id: oid.asset_id,
            lots_delta: oid.lots_delta,
            inserted_at_ms: now_ms,
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
                self.ledger.ack_fill(&oid.user, &fill)?;
                self.inflight.remove(&oid.client_oid);
                if !i1_holds(&self.phoenix, &self.ledger, fill.asset_id) {
                    let flags = self.ledger.config_halt() | cc::INVARIANT_BROKEN;
                    self.ledger.write_halt(flags)?;
                    return Ok(HedgeOutcome::InvariantBroken {
                        asset_id: fill.asset_id,
                    });
                }
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
    use crate::phoenix::MockPhoenix;

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
        Adapter::new(phoenix, MemoryLedger::new(), operator)
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
}
