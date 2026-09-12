use std::collections::BTreeMap;

use crate::phoenix::PhoenixVenue;
use crate::ledger::LedgerPort;

/// `intended[asset] = Book.residuals[asset] + sum(pending.lots_delta for asset)`.
pub fn intended_residual(
    book: &[(u16, i64)],
    pending: &[(u16, i64)],
) -> BTreeMap<u16, i64> {
    let mut m = BTreeMap::new();
    for (asset, lots) in book {
        *m.entry(*asset).or_insert(0) += *lots;
    }
    for (asset, lots) in pending {
        *m.entry(*asset).or_insert(0) += *lots;
    }
    m.retain(|_, lots| *lots != 0);
    m
}

/// I1 after ack: Book residual == Phoenix base lots.
pub fn i1_holds<P: PhoenixVenue, L: LedgerPort>(
    phoenix: &P,
    ledger: &L,
    asset_id: u16,
) -> bool {
    ledger.book_lots(asset_id) == phoenix.base_lots(asset_id)
}

/// I2 cash form (after funding fold).
pub fn i2_holds(
    user_cash_sum: u64,
    vault_ata: u64,
    phoenix_collateral: u64,
    in_flight_usdc: i64,
) -> bool {
    i2_holds_unsettled(user_cash_sum, 0, vault_ata, phoenix_collateral, 0, in_flight_usdc)
}

/// I2 between funding settles: include unsettled on both sides.
pub fn i2_holds_unsettled(
    user_cash_sum: u64,
    user_unsettled: i64,
    vault_ata: u64,
    phoenix_collateral: u64,
    pool_unsettled: i64,
    in_flight_usdc: i64,
) -> bool {
    let lhs = user_cash_sum as i128 + user_unsettled as i128;
    let rhs = vault_ata as i128
        + phoenix_collateral as i128
        + pool_unsettled as i128
        + in_flight_usdc as i128;
    lhs == rhs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intended_sums_book_and_pending() {
        let got = intended_residual(&[(1, 10), (2, -3)], &[(1, 5), (2, 3)]);
        assert_eq!(got.get(&1).copied(), Some(15));
        assert!(!got.contains_key(&2));
    }

    #[test]
    fn i2_allows_in_flight_delta() {
        assert!(i2_holds(100, 40, 50, 10));
        assert!(!i2_holds(100, 40, 50, 0));
    }

    #[test]
    fn i2_unsettled_balances_until_fold() {
        assert!(i2_holds_unsettled(200, -6, 150, 50, -6, 0));
        assert!(!i2_holds(194, 150, 50, 0));
    }
}
