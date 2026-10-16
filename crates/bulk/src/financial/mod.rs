//! Exact reported financial observations, not ledger postings or readiness grants.
pub mod account;
pub mod fees;
pub mod history;
pub use crate::raw::RawRow;
use crate::{
    Error, Scope,
    fills::MAX_MARKETS,
    precision::{Market, decimal, scaled},
};
use cinder_kernel::amounts::{AssetUnit, MarketUnit, QuantityLots, QuoteAtoms};
use sha2::{Digest, Sha256};

/// Original reported number with exact numeric equality after normalization.
/// This is not a qualified rate, oracle mark or cost-basis definition.
#[derive(Clone)]
pub struct ReportedDecimal {
    original: String,
    parts: (bool, u128, i32),
}
impl ReportedDecimal {
    /// Validate bounded native decimal/exponent syntax without rounding to atoms.
    pub fn new(text: String) -> Result<Self, Error> {
        let (sign, mut coefficient, mut power) = decimal(&text)?;
        if coefficient == 0 {
            power = 0;
        } else {
            while coefficient.is_multiple_of(10) {
                coefficient /= 10;
                power += 1;
            }
        }
        Ok(Self {
            original: text,
            parts: (sign, coefficient, power),
        })
    }
    /// Original native display; not a value to multiply into exact ledger basis.
    pub fn text(&self) -> &str {
        &self.original
    }
    pub(crate) fn positive(&self) -> bool {
        !self.parts.0 && self.parts.1 > 0
    }
}
impl PartialEq for ReportedDecimal {
    fn eq(&self, other: &Self) -> bool {
        self.parts == other.parts
    }
}
impl Eq for ReportedDecimal {}
impl std::fmt::Debug for ReportedDecimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReportedDecimal([PRIVATE])")
    }
}

/// Exact native market grid and independently configured kernel denomination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mapping {
    pub(crate) wire: Market,
    pub(crate) unit: MarketUnit,
}
impl Mapping {
    /// Binds metadata only; constructor success does not qualify the venue.
    pub fn new(wire: Market, unit: MarketUnit) -> Self {
        Self { wire, unit }
    }
    pub(crate) fn quantity(&self, text: &str) -> Result<QuantityLots, Error> {
        let n = scaled(text, 8)?;
        let step = i128::from(self.wire.size_step);
        if n % step != 0 {
            return Err(Error::Invalid);
        }
        Ok(QuantityLots::new(
            self.unit,
            i64::try_from(n / step).map_err(|_| Error::Limit)?,
        ))
    }
}
/// Expected current-state account kind; children are not pooled implicitly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// Direct master account.
    Master,
    /// Exact independently controlled sub-account with pinned parent.
    SubAccount {
        /// Original parent address, not a private customer.
        parent: [u8; 32],
    },
}
/// Trusted interpretation profile, never inferred from body precision or symbols.
#[derive(Clone)]
pub struct Profile {
    pub(crate) scope: Scope,
    pub(crate) quote: AssetUnit,
    pub(crate) places: u8,
    pub(crate) target: Target,
    pub(crate) mappings: Vec<Mapping>,
    commitment: [u8; 32],
}
impl Profile {
    /// Explicit unit/precision/routing binding with no native defaults or activation.
    pub fn new(
        scope: Scope,
        quote: AssetUnit,
        places: u8,
        target: Target,
        mut mappings: Vec<Mapping>,
    ) -> Result<Self, Error> {
        if places > 38 || mappings.len() > MAX_MARKETS {
            return Err(Error::Limit);
        }
        if let Target::SubAccount { parent } = target
            && (parent == [0; 32] || parent == scope.account())
        {
            return Err(Error::Invalid);
        }
        mappings.sort_by(|a, b| a.wire.symbol.cmp(&b.wire.symbol));
        for (i, m) in mappings.iter().enumerate() {
            if m.unit.quote != quote || m.wire.quote_places != places {
                return Err(Error::Invalid);
            }
            if mappings[..i]
                .iter()
                .any(|old| old.wire.symbol == m.wire.symbol || old.unit.market == m.unit.market)
            {
                return Err(Error::Conflict);
            }
        }
        let mut h = Sha256::new();
        h.update(b"CINDER-BULK-FINANCIAL-PROFILE-1\0");
        h.update(scope.commitment());
        h.update(quote.asset.bytes());
        h.update(quote.precision.get().to_le_bytes());
        h.update([places]);
        match target {
            Target::Master => h.update([0]),
            Target::SubAccount { parent } => {
                h.update([1]);
                h.update(parent);
            }
        }
        h.update((mappings.len() as u64).to_le_bytes());
        for m in &mappings {
            h.update((m.wire.symbol.len() as u64).to_le_bytes());
            h.update(m.wire.symbol.as_bytes());
            h.update(m.wire.size_step.to_le_bytes());
            h.update(m.wire.price_step.to_le_bytes());
            h.update(m.unit.market.bytes());
            h.update(m.unit.precision.get().to_le_bytes());
        }
        Ok(Self {
            scope,
            quote,
            places,
            target,
            mappings,
            commitment: h.finalize().into(),
        })
    }
    /// Interpretation commitment, separate from stable native economic identities.
    pub fn commitment(&self) -> [u8; 32] {
        self.commitment
    }
    pub(crate) fn mapping(&self, symbol: &str) -> Option<&Mapping> {
        self.mappings.iter().find(|m| m.wire.symbol == symbol)
    }
    pub(crate) fn amount(&self, text: &str) -> Result<QuoteAtoms, Error> {
        Ok(QuoteAtoms::new(self.quote, scaled(text, self.places)?))
    }
    pub(crate) fn key(&self, family: u8, id: crate::fills::TradeId) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"CINDER-BULK-FINANCIAL-OBSERVATION-1\0");
        h.update(self.scope.commitment());
        h.update([family]);
        h.update(id.slot.to_le_bytes());
        h.update(id.sequence.to_le_bytes());
        h.finalize().into()
    }
}
impl std::fmt::Debug for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FinancialProfile([PRIVATE])")
    }
}
/// Why original evidence cannot be normalized under this narrow profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeldReason {
    /// No trusted symbol-to-unit mapping.
    UnmappedMarket,
    /// Per-instrument isolated-lane economics are not supported.
    IsolatedLane,
    /// Native risk-vault/unknown event is not a position execution.
    UnsupportedRisk,
}
/// Retained raw evidence, not a missing event, normalized posting or zero balance.
#[derive(Clone, PartialEq, Eq)]
pub struct HeldRow {
    /// Original native symbol.
    pub symbol: String,
    /// Explicit unresolved interpretation.
    pub reason: HeldReason,
    /// Original row, privately retained rather than reserialized.
    pub raw: RawRow,
}
impl std::fmt::Debug for HeldRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HeldRow([PRIVATE])")
    }
}
