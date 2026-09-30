//! Linear-perp position accounting. Native normalization/finality is a caller obligation.

use crate::{
    Error,
    amounts::*,
    math::{Rounding, mul_div},
};

/// Qualified conversion: quote atoms per (lot × tick), as an exact rational.
/// This is configuration, not a venue default. Inexact notionals are rejected;
/// an adapter must qualify a suitable precision, not silently round actual fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Market {
    unit: MarketUnit,
    numerator: u64,
    denominator: u64,
}

impl Market {
    /// Bind a positive conversion to an explicit market/precision revision.
    pub fn new(unit: MarketUnit, numerator: u64, denominator: u64) -> Result<Self, Error> {
        if numerator == 0 || denominator == 0 {
            return Err(Error::InvalidRatio);
        }
        Ok(Self {
            unit,
            numerator,
            denominator,
        })
    }
    /// Configured unit identity.
    pub fn unit(self) -> MarketUnit {
        self.unit
    }
    /// Exact signed value, not an actual collateral debit for opening a perp.
    pub fn notional(self, quantity: QuantityLots, price: PriceTicks) -> Result<QuoteAtoms, Error> {
        quantity.unit().require(self.unit)?;
        self.value(i128::from(quantity.lots()), price)
    }
    fn value(self, lots: i128, price: PriceTicks) -> Result<QuoteAtoms, Error> {
        price.unit().require(self.unit)?;
        let product = lots
            .checked_mul(i128::from(price.ticks()))
            .ok_or(Error::Overflow)?;
        Ok(QuoteAtoms::new(
            self.unit.quote,
            mul_div(product, self.numerator, self.denominator, Rounding::Exact)?,
        ))
    }
}

/// Signed remaining basis, not a rounded average entry price.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    quantity: QuantityLots,
    basis: BasisAtoms,
}

/// One fee-exclusive economic fill result. Funding/fees have separate transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FillResult {
    /// Remaining position.
    pub position: Position,
    /// Realized PnL credited to signed cash, not notional paid for an opening.
    pub realized: QuoteAtoms,
}

impl Position {
    /// Validate a checkpoint position; this does not insert it into a ledger.
    pub fn new(quantity: QuantityLots, basis: BasisAtoms) -> Result<Self, Error> {
        quantity.unit().require(basis.unit())?;
        if (quantity.lots() == 0 && basis.atoms() != 0)
            || (basis.atoms() != 0
                && i128::from(quantity.lots()).signum() != basis.atoms().signum())
        {
            return Err(Error::InvalidSign);
        }
        Ok(Self { quantity, basis })
    }
    /// Empty position in a configured market.
    pub fn flat(unit: MarketUnit) -> Self {
        Self {
            quantity: QuantityLots::new(unit, 0),
            basis: BasisAtoms::new(unit, 0),
        }
    }
    /// Signed exposure.
    pub fn quantity(self) -> QuantityLots {
        self.quantity
    }
    /// Exact unconsumed signed entry basis.
    pub fn basis(self) -> BasisAtoms {
        self.basis
    }
    /// Marked PnL. Price freshness/authority must be established outside this primitive.
    pub fn unrealized(self, market: Market, mark: PriceTicks) -> Result<QuoteAtoms, Error> {
        market
            .notional(self.quantity, mark)?
            .checked_sub(QuoteAtoms::new(market.unit.quote, self.basis.atoms()))
    }
    /// Apply an actual fill, preserving division residue and consuming full-close basis.
    /// Uses unsigned magnitudes so i64::MIN does not require negating i64.
    pub fn fill(
        self,
        market: Market,
        delta: QuantityLots,
        price: PriceTicks,
    ) -> Result<FillResult, Error> {
        self.quantity.unit().require(market.unit)?;
        delta.unit().require(market.unit)?;
        price.unit().require(market.unit)?;
        if delta.lots() == 0 {
            return Err(Error::InvalidSign);
        }
        let quantity = self.quantity.checked_add(delta)?;
        let reducing =
            self.quantity.lots() != 0 && self.quantity.lots().signum() != delta.lots().signum();
        let (basis, realized) = if reducing {
            let closed = self.quantity.magnitude().min(delta.magnitude());
            let (allocated, remaining) = self.basis.split(closed, self.quantity.magnitude())?;
            // Only the whole actual fill must be exact. Split its signed value
            // toward zero; retain any conversion residue in the opening basis.
            let total = market.notional(delta, price)?;
            let (closing, opening) =
                BasisAtoms::new(market.unit, total.atoms()).split(closed, delta.magnitude())?;
            let realized = QuoteAtoms::new(market.unit.quote, closing.atoms())
                .checked_neg()?
                .checked_sub(QuoteAtoms::new(market.unit.quote, allocated.atoms()))?;
            (remaining.checked_add(opening)?, realized)
        } else {
            (
                self.basis.checked_add(BasisAtoms::new(
                    market.unit,
                    market.notional(delta, price)?.atoms(),
                ))?,
                QuoteAtoms::new(market.unit.quote, 0),
            )
        };
        Ok(FillResult {
            position: Self::new(quantity, basis)?,
            realized,
        })
    }
}
