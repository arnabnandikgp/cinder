//! Phoenix funding crank (`docs/08-funding-allocation.md` §7).
//! Accrue unsettled (health) each interval; fold cash when venue collateral moves.

use cinder_common as cc;

use crate::ledger::FundingPort;
use crate::{Adapter, AdapterError, PubkeyBytes};
use crate::phoenix::PhoenixVenue;

/// One Phoenix funding interval, already converted to native USDC where noted.
#[derive(Clone, Debug)]
pub struct FundingInterval {
    pub asset_id: u16,
    /// Phoenix Δacc in native USDC per base lot (`lots * usdc_per_lot`).
    /// Longs pay when this is negative.
    pub usdc_per_lot: i64,
    /// Rise/Hawkeye `unsettledFundingOwed` on the pool, native USDC.
    pub pool_unsettled_usdc: i64,
    pub phoenix_collateral: u64,
}

/// Convert Phoenix quote-lot funding to native USDC (6 decimals), toward zero.
pub fn quote_lots_to_usdc(quote_lots: i64, quote_decimals: u32) -> i64 {
    if quote_decimals == cc::USDC_DECIMALS {
        return quote_lots;
    }
    if quote_decimals < cc::USDC_DECIMALS {
        let p = 10i64.saturating_pow(cc::USDC_DECIMALS - quote_decimals);
        quote_lots.saturating_mul(p)
    } else {
        let p = 10i64.saturating_pow(quote_decimals - cc::USDC_DECIMALS);
        if p == 0 {
            0
        } else {
            quote_lots / p
        }
    }
}

pub fn delta_usdc_for_lots(lots: i64, usdc_per_lot: i64) -> Result<i64, AdapterError> {
    lots.checked_mul(usdc_per_lot)
        .ok_or_else(|| AdapterError::Ledger("funding overflow".into()))
}

#[derive(Clone, Debug, Default)]
pub struct FundingCrankReport {
    pub epoch: u64,
    pub folded: bool,
    pub skipped_invariant: bool,
    pub dust_abs: u64,
    pub dust_over_cap: bool,
    pub liquidated: Vec<(PubkeyBytes, u16)>,
}

impl<P: PhoenixVenue, L: crate::ledger::LedgerPort + FundingPort> Adapter<P, L> {
    /// Accrue every user; fold if Phoenix collateral moved vs last crank.
    pub fn crank_funding(
        &mut self,
        interval: &FundingInterval,
    ) -> Result<FundingCrankReport, AdapterError> {
        let mut report = FundingCrankReport::default();
        if !self.ledger.invariant_ok() {
            report.skipped_invariant = true;
            return Ok(report);
        }

        let epoch = self.ledger.book_funding_epoch().saturating_add(1);
        self.ledger.bump_funding_epoch(epoch)?;
        report.epoch = epoch;

        let users = self.ledger.user_ids();
        let mut sum_delta: i128 = 0;
        for user in &users {
            let lots = self.ledger.lots_of(user, interval.asset_id);
            let delta = if lots == 0 {
                0
            } else {
                delta_usdc_for_lots(lots, interval.usdc_per_lot)?
            };
            sum_delta += delta as i128;
            let owned = if delta == 0 {
                Vec::new()
            } else {
                vec![(interval.asset_id, delta)]
            };
            self.ledger
                .allocate_funding(user, epoch, false, &owned)?;

            let mm = cc::stub_cinder_im(lots.unsigned_abs()).unwrap_or(0) as i128;
            if lots != 0 && self.ledger.user_equity(user) < mm {
                self.ledger.liquidate_user(user, interval.asset_id)?;
                report.liquidated.push((*user, interval.asset_id));
            }
        }

        let pool_delta = delta_usdc_for_lots(
            self.ledger.book_lots(interval.asset_id),
            interval.usdc_per_lot,
        )? as i128;
        let dust = (sum_delta - pool_delta).unsigned_abs() as u64;
        report.dust_abs = dust;
        report.dust_over_cap = dust > cc::FUNDING_DUST_CAP;

        let prev = self.last_phoenix_collateral;
        let moved = prev.is_some() && prev != Some(interval.phoenix_collateral);
        if moved {
            for user in &users {
                self.ledger
                    .allocate_funding(user, epoch, true, &[])?;
            }
            self.ledger
                .set_phoenix_collateral(interval.phoenix_collateral);
            report.folded = true;
        }
        self.last_phoenix_collateral = Some(interval.phoenix_collateral);
        let _ = interval.pool_unsettled_usdc;
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Adapter;
    use crate::ledger::{LedgerPort, MemoryLedger};
    use crate::operator::{MockTeeAuth, OperatorAuth};
    use crate::phoenix::MockPhoenix;
    use crate::residual::i2_holds_unsettled;

    fn user(n: u8) -> PubkeyBytes {
        [n; 32]
    }

    fn adapter() -> Adapter<MockPhoenix, MemoryLedger> {
        let mut operator = OperatorAuth::new("http://127.0.0.1:6699");
        operator
            .authenticate(&MockTeeAuth { token: "op".into() }, &user(0), &|_| [0u8; 64])
            .unwrap();
        Adapter::new(MockPhoenix::new(), MemoryLedger::new(), operator)
    }

    #[test]
    fn quote_lots_identity_at_six_decimals() {
        assert_eq!(quote_lots_to_usdc(-1_000, 6), -1_000);
        assert_eq!(quote_lots_to_usdc(5, 4), 500);
        assert_eq!(quote_lots_to_usdc(-5_000_000, 8), -50_000);
    }

    #[test]
    fn offsetting_users_accrue_unsettled_free_unchanged() {
        let mut ad = adapter();
        let a = user(1);
        let b = user(2);
        ad.ledger.ensure_user(a, 2_000_000);
        ad.ledger.ensure_user(b, 2_000_000);
        ad.ledger.user_lots.insert(a, [(1, 10)].into_iter().collect());
        ad.ledger.user_lots.insert(b, [(1, -10)].into_iter().collect());
        ad.ledger.vault_ata = 4_000_000;
        ad.ledger.phoenix_collateral = 0;

        let r = ad
            .crank_funding(&FundingInterval {
                asset_id: 1,
                usdc_per_lot: -1_000,
                pool_unsettled_usdc: 0,
                phoenix_collateral: 0,
            })
            .unwrap();
        assert_eq!(r.epoch, 1);
        assert!(!r.folded);
        assert!(!r.dust_over_cap);
        assert_eq!(ad.ledger.user_free[&a], 2_000_000);
        assert_eq!(ad.ledger.user_free[&b], 2_000_000);
        assert_eq!(ad.ledger.user_unsettled[&a][&1], -10_000);
        assert_eq!(ad.ledger.user_unsettled[&b][&1], 10_000);
        assert!(ad.ledger.i2_ok_unsettled(0, 0));
        assert!(i2_holds_unsettled(4_000_000, 0, 4_000_000, 0, 0, 0));
    }

    #[test]
    fn partial_net_i2_uses_pool_unsettled_until_fold() {
        let mut ad = adapter();
        let a = user(1);
        let b = user(2);
        ad.ledger.ensure_user(a, 2_000_000);
        ad.ledger.ensure_user(b, 2_000_000);
        ad.ledger.user_lots.insert(a, [(1, 10)].into_iter().collect());
        ad.ledger.user_lots.insert(b, [(1, -4)].into_iter().collect());
        ad.ledger.book.insert(1, 6);
        ad.ledger.vault_ata = 3_500_000;
        ad.ledger.phoenix_collateral = 500_000;

        let r = ad
            .crank_funding(&FundingInterval {
                asset_id: 1,
                usdc_per_lot: -1_000,
                pool_unsettled_usdc: -6_000,
                phoenix_collateral: 500_000,
            })
            .unwrap();
        assert!(!r.folded);
        assert_eq!(ad.ledger.sum_unsettled(), -6_000);
        assert!(ad.ledger.i2_ok_unsettled(-6_000, 0));

        let r2 = ad
            .crank_funding(&FundingInterval {
                asset_id: 1,
                usdc_per_lot: 0,
                pool_unsettled_usdc: 0,
                phoenix_collateral: 494_000,
            })
            .unwrap();
        assert!(r2.folded);
        assert_eq!(ad.ledger.user_free[&a], 1_990_000);
        assert_eq!(ad.ledger.user_free[&b], 2_004_000);
        assert_eq!(ad.ledger.sum_unsettled(), 0);
        assert_eq!(ad.ledger.phoenix_collateral, 494_000);
        assert!(ad.ledger.i2_ok(0));
    }

    #[test]
    fn dust_under_cap_does_not_halt() {
        let mut ad = adapter();
        let a = user(1);
        ad.ledger.ensure_user(a, 50_000);
        ad.ledger.user_lots.insert(a, [(1, 1)].into_iter().collect());
        ad.ledger.book.insert(1, 1);
        let r = ad
            .crank_funding(&FundingInterval {
                asset_id: 1,
                usdc_per_lot: 1,
                pool_unsettled_usdc: 0,
                phoenix_collateral: 0,
            })
            .unwrap();
        assert!(!r.dust_over_cap);
        assert!(ad.ledger.invariant_ok());
    }

    #[test]
    fn invariant_broken_skips_crank() {
        let mut ad = adapter();
        ad.ledger.write_halt(cc::INVARIANT_BROKEN).unwrap();
        let r = ad
            .crank_funding(&FundingInterval {
                asset_id: 1,
                usdc_per_lot: 1_000,
                pool_unsettled_usdc: 0,
                phoenix_collateral: 0,
            })
            .unwrap();
        assert!(r.skipped_invariant);
        assert_eq!(r.epoch, 0);
    }

    #[test]
    fn flat_user_bump_only() {
        let mut ad = adapter();
        let a = user(1);
        ad.ledger.ensure_user(a, 10_000);
        let r = ad
            .crank_funding(&FundingInterval {
                asset_id: 1,
                usdc_per_lot: 1_000,
                pool_unsettled_usdc: 0,
                phoenix_collateral: 0,
            })
            .unwrap();
        assert_eq!(r.epoch, 1);
        assert_eq!(ad.ledger.user_epoch[&a], 1);
        assert!(ad.ledger.user_unsettled[&a].is_empty());
    }

    #[test]
    fn under_mm_after_accrue_liquidates() {
        let mut ad = adapter();
        let a = user(1);
        ad.ledger.ensure_user(a, 100);
        ad.ledger.user_lots.insert(a, [(1, 10)].into_iter().collect());
        let r = ad
            .crank_funding(&FundingInterval {
                asset_id: 1,
                usdc_per_lot: 1_000,
                pool_unsettled_usdc: -10_000,
                phoenix_collateral: 0,
            })
            .unwrap();
        assert_eq!(r.liquidated, vec![(a, 1)]);
    }
}
