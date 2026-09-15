use std::collections::BTreeMap;

use crate::ledger::LedgerPort;
use crate::phoenix::PhoenixVenue;
use crate::AdapterError;

/// `intended[asset] = Book.residuals[asset] + sum(pending.lots_delta for asset)`.
pub fn intended_residual(
    book: &[(u16, i64)],
    pending: &[(u16, i64)],
) -> Result<BTreeMap<u16, i64>, AdapterError> {
    let mut m: BTreeMap<u16, i64> = BTreeMap::new();
    for (asset, lots) in book.iter().chain(pending) {
        let next = m
            .get(asset)
            .copied()
            .unwrap_or_default()
            .checked_add(*lots)
            .ok_or_else(|| AdapterError::Ledger("residual lots overflow".into()))?;
        if next == 0 {
            m.remove(asset);
        } else {
            m.insert(*asset, next);
        }
    }
    Ok(m)
}

/// I1 after ack: Book residual == Phoenix base lots.
pub fn i1_holds<P: PhoenixVenue, L: LedgerPort>(phoenix: &P, ledger: &L, asset_id: u16) -> bool {
    ledger.book_lots(asset_id) == phoenix.base_lots(asset_id)
}

/// I1 live after a venue fill, before ack: Book + filled (not yet acked) == Phoenix.
pub fn i1_live(book: i64, pending: i64, phoenix: i64) -> bool {
    book.checked_add(pending) == Some(phoenix)
}

/// I2 cash form (after funding fold).
pub fn i2_holds(
    user_cash_sum: i128,
    vault_ata: u64,
    phoenix_collateral: u64,
    in_flight_usdc: i64,
) -> bool {
    i2_holds_unsettled(
        user_cash_sum,
        0,
        vault_ata,
        phoenix_collateral,
        0,
        in_flight_usdc,
    )
}

/// I2 between funding settles: include unsettled on both sides.
pub fn i2_holds_unsettled(
    user_cash_sum: i128,
    user_unsettled: i64,
    vault_ata: u64,
    phoenix_collateral: u64,
    pool_unsettled: i64,
    in_flight_usdc: i64,
) -> bool {
    let lhs = user_cash_sum + user_unsettled as i128;
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
        let got = intended_residual(&[(1, 10), (2, -3)], &[(1, 5), (2, 3)]).unwrap();
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

    #[test]
    fn i2_accounts_for_user_bad_debt_as_negative_cash() {
        assert!(i2_holds(-10, 0, 0, -10));
        assert!(!i2_holds(0, 0, 0, -10));
    }

    #[test]
    fn i1_live_adds_pending() {
        assert!(i1_live(10, -10, 0));
        assert!(!i1_live(10, 0, 0));
        assert!(!i1_live(i64::MAX, 1, i64::MAX));
    }

    #[test]
    fn intended_residual_rejects_overflow() {
        let err = intended_residual(&[(1, i64::MAX)], &[(1, 1)]).unwrap_err();
        assert!(
            matches!(err, AdapterError::Ledger(message) if message == "residual lots overflow")
        );
    }
}
