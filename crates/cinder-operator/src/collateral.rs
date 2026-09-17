//! The confirmed-collateral requirement before a hedge, not a deposit port.
use crate::rpc::{Result, RuntimeError};
use cinder_common as cc;

/// A minimum posted-cash target, including an independently verified upper
/// bound on execution costs. This is neither insurance nor pool health proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollateralRequirement {
    required_posted_usdc: u64,
}

impl CollateralRequirement {
    /// Use a real intended-residual Phoenix IM quote. The cost allowance must
    /// bound the native charge, not a guessed fee rate or a REST UI estimate.
    /// Margin is rounded upward before applying the protocol's 50 USDC floor.
    pub fn for_hedge(phoenix_initial_margin_usdc: u64, execution_cost_usdc: u64) -> Result<Self> {
        let multiplier = u128::from(cc::BPS_DENOM) + u128::from(cc::BUFFER_MIN_BPS);
        let buffered = u128::from(phoenix_initial_margin_usdc)
            .checked_mul(multiplier)
            .and_then(|n| n.checked_add(u128::from(cc::BPS_DENOM - 1)))
            .ok_or(RuntimeError::Decode)?
            / u128::from(cc::BPS_DENOM);
        let required = buffered
            .max(u128::from(cc::BUFFER_FLOOR_USDC))
            .checked_add(u128::from(execution_cost_usdc))
            .ok_or(RuntimeError::Decode)?;
        Ok(Self {
            required_posted_usdc: u64::try_from(required).map_err(|_| RuntimeError::Decode)?,
        })
    }

    pub fn required_posted_usdc(self) -> u64 {
        self.required_posted_usdc
    }

    /// Only a fresh authoritative native cash balance belongs here. Vault,
    /// transit ATA, pending deposit and raw unrealized gains are not posted cash.
    /// A zero shortfall still requires native margin, user and backing checks.
    pub fn shortfall_usdc(self, confirmed_native_cash_usdc: u64) -> u64 {
        self.required_posted_usdc
            .saturating_sub(confirmed_native_cash_usdc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_floor_or_buffer_and_reserves_execution_costs_afterward() {
        for (im, cost, expected) in [
            (0, 0, cc::BUFFER_FLOOR_USDC),
            (1_000_000, 1_234, cc::BUFFER_FLOOR_USDC + 1_234),
            (100_000_000, 3_500, 120_003_500),
            (100_000_001, 0, 120_000_002),
        ] {
            assert_eq!(
                CollateralRequirement::for_hedge(im, cost)
                    .unwrap()
                    .required_posted_usdc(),
                expected
            );
        }
    }

    #[test]
    fn credits_only_confirmed_native_cash_and_handles_exact_target() {
        let requirement = CollateralRequirement::for_hedge(100_000_000, 3_500).unwrap();
        assert_eq!(requirement.shortfall_usdc(0), 120_003_500);
        assert_eq!(requirement.shortfall_usdc(100_000_000), 20_003_500);
        assert_eq!(requirement.shortfall_usdc(120_003_499), 1);
        assert_eq!(requirement.shortfall_usdc(120_003_500), 0);
        assert_eq!(requirement.shortfall_usdc(u64::MAX), 0);
    }

    #[test]
    fn overflow_is_not_a_saturated_permission_to_hedge() {
        assert!(CollateralRequirement::for_hedge(u64::MAX, 0).is_err());
        assert!(CollateralRequirement::for_hedge(0, u64::MAX).is_err());
        assert!(CollateralRequirement::for_hedge(u64::MAX / 2, u64::MAX / 2).is_err());
    }
}
