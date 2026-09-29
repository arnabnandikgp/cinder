//! Distinct financial dimensions with runtime asset/market/revision checks.
//! No numeric operator silently combines different units or saturates a result.
//!
//! Basis is not cash, even when both use signed quote atoms:
//! ```compile_fail
//! use cinder_kernel::amounts::{QuoteAtoms, BasisAtoms};
//! fn mix(cash: QuoteAtoms, basis: BasisAtoms) { let _ = cash.checked_add(basis); }
//! ```

use crate::{
    Error,
    identity::{AssetId, MarketId, PrecisionVersion},
    math::{DecimalScale, Rounding, mul_div},
};

/// Identity of a quote/token accounting atom, not its external display scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetUnit {
    /// Exact asset identity.
    pub asset: AssetId,
    /// Version of the trusted atom definition.
    pub precision: PrecisionVersion,
}

impl AssetUnit {
    /// Validate against trusted metadata, never a precision revision from the wire.
    pub fn require(self, expected: Self) -> Result<(), Error> {
        if self.asset != expected.asset {
            return Err(Error::UnitMismatch);
        }
        self.precision.require(expected.precision)
    }
}

/// Identity of a lot/tick and its quote-basis atom definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketUnit {
    /// Exact instrument identity.
    pub market: MarketId,
    /// Revision binding lot size, tick size and price-to-basis conversion.
    pub precision: PrecisionVersion,
    /// Basis denomination; not automatically physical collateral token units.
    pub quote: AssetUnit,
}

impl MarketUnit {
    /// Validate the complete unit definition against trusted metadata.
    pub fn require(self, expected: Self) -> Result<(), Error> {
        if self.market != expected.market {
            return Err(Error::UnitMismatch);
        }
        self.precision.require(expected.precision)?;
        self.quote.require(expected.quote)
    }
}

/// Declared external decimal scale and positive atomic tick/lot step.
/// The adapter must bind this to a qualified MarketUnit; no venue default exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecimalGrid {
    scale: DecimalScale,
    step: u64,
}

impl DecimalGrid {
    /// Step is expressed in atoms of this explicit decimal scale.
    pub fn new(scale: DecimalScale, step: u64) -> Result<Self, Error> {
        if step == 0 {
            return Err(Error::InvalidSign);
        }
        Ok(Self { scale, step })
    }
    /// Exact number of signed steps. Off-grid values reject rather than round.
    pub fn parse(self, input: &str) -> Result<i128, Error> {
        let atoms = self.scale.parse(input)?;
        let step = i128::from(self.step);
        if atoms % step != 0 {
            return Err(Error::Inexact);
        }
        Ok(atoms / step)
    }
}

macro_rules! signed_atoms {
    ($name:ident, $unit:ty, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name {
            unit: $unit,
            atoms: i128,
        }
        impl $name {
            /// Exact normalized atoms; unit metadata must be qualified separately.
            pub fn new(unit: $unit, atoms: i128) -> Self {
                Self { unit, atoms }
            }
            /// Decode with a caller-supplied, qualified scale; no float path exists.
            pub fn from_decimal(
                unit: $unit,
                input: &str,
                scale: DecimalScale,
            ) -> Result<Self, Error> {
                Ok(Self::new(unit, scale.parse(input)?))
            }
            /// Denomination and revision.
            pub fn unit(self) -> $unit {
                self.unit
            }
            /// Signed exact value, including legitimate debts/short basis.
            pub fn atoms(self) -> i128 {
                self.atoms
            }
            /// Same-unit checked addition. Inputs are not mutated on failure.
            pub fn checked_add(self, rhs: Self) -> Result<Self, Error> {
                self.unit.require(rhs.unit)?;
                Ok(Self::new(
                    self.unit,
                    self.atoms.checked_add(rhs.atoms).ok_or(Error::Overflow)?,
                ))
            }
            /// Same-unit checked subtraction.
            pub fn checked_sub(self, rhs: Self) -> Result<Self, Error> {
                self.unit.require(rhs.unit)?;
                Ok(Self::new(
                    self.unit,
                    self.atoms.checked_sub(rhs.atoms).ok_or(Error::Overflow)?,
                ))
            }
            /// Checked sign reversal; i128::MIN cannot be positively represented.
            pub fn checked_neg(self) -> Result<Self, Error> {
                Ok(Self::new(
                    self.unit,
                    self.atoms.checked_neg().ok_or(Error::Overflow)?,
                ))
            }
        }
    };
}

signed_atoms!(
    QuoteAtoms,
    AssetUnit,
    "Signed quote cash/accrual amount; not a physical-token balance by itself."
);
signed_atoms!(
    BasisAtoms,
    MarketUnit,
    "Signed remaining entry basis; never a rounded average entry price."
);

impl BasisAtoms {
    /// Allocate `part/total` toward zero and retain all residue in remaining basis.
    /// Full allocation consumes every atom; both signs and i128::MIN are supported.
    pub fn split(self, part: u64, total: u64) -> Result<(Self, Self), Error> {
        if total == 0 {
            return Err(Error::DivisionByZero);
        }
        if part > total {
            return Err(Error::InvalidRatio);
        }
        let allocated = Self::new(
            self.unit,
            mul_div(self.atoms, part, total, Rounding::TowardZero)?,
        );
        Ok((allocated, self.checked_sub(allocated)?))
    }
}

/// Signed integer lot count. Negative means short/sell, zero may mean flat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuantityLots {
    unit: MarketUnit,
    lots: i64,
}

impl QuantityLots {
    /// Construct a normalized position/delta; order-specific nonzero checks are later.
    pub fn new(unit: MarketUnit, lots: i64) -> Self {
        Self { unit, lots }
    }
    /// Decode an exact number of configured lots; no fractional lot truncation.
    pub fn from_decimal(unit: MarketUnit, input: &str, grid: DecimalGrid) -> Result<Self, Error> {
        Ok(Self::new(
            unit,
            i64::try_from(grid.parse(input)?).map_err(|_| Error::Overflow)?,
        ))
    }
    /// Market and precision definition.
    pub fn unit(self) -> MarketUnit {
        self.unit
    }
    /// Signed lots.
    pub fn lots(self) -> i64 {
        self.lots
    }
    /// Absolute lots, including the full magnitude of i64::MIN.
    pub fn magnitude(self) -> u64 {
        self.lots.unsigned_abs()
    }
    /// Same-market checked addition, not a financial fill transition.
    pub fn checked_add(self, rhs: Self) -> Result<Self, Error> {
        self.unit.require(rhs.unit)?;
        Ok(Self::new(
            self.unit,
            self.lots.checked_add(rhs.lots).ok_or(Error::Overflow)?,
        ))
    }
    /// Same-market checked subtraction.
    pub fn checked_sub(self, rhs: Self) -> Result<Self, Error> {
        self.unit.require(rhs.unit)?;
        Ok(Self::new(
            self.unit,
            self.lots.checked_sub(rhs.lots).ok_or(Error::Overflow)?,
        ))
    }
    /// Checked sign reversal; minimum signed quantity cannot become positive.
    pub fn checked_neg(self) -> Result<Self, Error> {
        Ok(Self::new(
            self.unit,
            self.lots.checked_neg().ok_or(Error::Overflow)?,
        ))
    }
}

/// Strictly positive integer price ticks; not quote atoms per lot automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriceTicks {
    unit: MarketUnit,
    ticks: u64,
}

impl PriceTicks {
    /// Reject zero. A negative signed input cannot be passed without conversion.
    pub fn new(unit: MarketUnit, ticks: u64) -> Result<Self, Error> {
        if ticks == 0 {
            return Err(Error::InvalidSign);
        }
        Ok(Self { unit, ticks })
    }
    /// Decode an exact strictly positive number of ticks.
    pub fn from_decimal(unit: MarketUnit, input: &str, grid: DecimalGrid) -> Result<Self, Error> {
        let ticks = grid.parse(input)?;
        if ticks <= 0 {
            return Err(Error::InvalidSign);
        }
        Self::new(unit, u64::try_from(ticks).map_err(|_| Error::Overflow)?)
    }
    /// Market and precision definition.
    pub fn unit(self) -> MarketUnit {
        self.unit
    }
    /// Positive ticks. Conversion into basis requires qualified market metadata.
    pub fn ticks(self) -> u64 {
        self.ticks
    }
}
