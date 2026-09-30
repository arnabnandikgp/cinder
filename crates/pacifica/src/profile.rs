//! Explicit native qualification and exact decimal grids. No venue defaults.
use crate::Error;
use cinder_kernel::{amounts::*, identity::*, ledger::Config, math::DecimalScale};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Evidence level is not automatically promotion to permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Level {
    /// Not established.
    Unknown,
    /// Public documentation only.
    Documented,
    /// Bounded historical observation, not universal semantics.
    Observed,
    /// Explicit qualification for the bound profile/environment (synthetic in tests).
    Qualified,
}
/// Exact wire decimal grid, distinct from quote accounting scale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grid {
    /// Decimal places.
    pub places: u8,
    /// Atomic units per integer lot/tick.
    pub step: u64,
}
impl Grid {
    /// Checked parsing with no rounding, float or exponent path.
    pub fn parse(&self, value: &str) -> Result<i128, Error> {
        Ok(DecimalGrid::new(DecimalScale::new(self.places)?, self.step)?.parse(value)?)
    }
    /// Exact native decimal text for signing, including declared scale.
    pub fn format(&self, value: u64) -> Result<String, Error> {
        DecimalGrid::new(DecimalScale::new(self.places)?, self.step)?;
        let n = u128::from(value)
            .checked_mul(u128::from(self.step))
            .ok_or(Error::Limit)?;
        let d = 10_u128
            .checked_pow(u32::from(self.places))
            .ok_or(Error::Limit)?;
        if self.places == 0 {
            Ok(n.to_string())
        } else {
            Ok(format!(
                "{}.{:0width$}",
                n / d,
                n % d,
                width = usize::from(self.places)
            ))
        }
    }
}
/// One native symbol bound to a configured kernel market, never a guessed alias.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mapping {
    /// Exact native perp symbol.
    pub symbol: String,
    /// Index in the immutable ledger market configuration.
    pub market: usize,
    /// Native quantity grid.
    pub size: Grid,
    /// Native execution price grid; not rounded entry-price precision.
    pub price: Grid,
}
/// Immutable profile; changing its commitment requires an explicit migration.
/// P13 does not supply a production-qualified profile.
#[derive(Clone)]
pub struct Profile {
    /// Exact authoritative ledger configuration.
    pub config: Config,
    /// Configured economic source shared across REST/WS delivery.
    pub source: EventScope,
    /// Native pool wallet address, not private customer wallet.
    pub account: String,
    /// Bound environment, e.g. a specific testnet or synthetic fixture version.
    pub environment: String,
    /// Nonzero adapter evidence revision.
    pub revision: u64,
    /// Named qualification evidence; fixture evidence is not live evidence.
    pub evidence: String,
    /// Native metadata and exact conversion qualification.
    pub precision: Level,
    /// Execution price/identity/side and net-of-fee PnL qualification.
    pub fills: Level,
    /// Bound quantity and price grids.
    pub markets: Vec<Mapping>,
    /// Exact quote atom decimal scale; not chosen from display formatting.
    pub quote_places: u8,
    /// Explicit qualified WS perp instrument tag, never inferred from numeric zero.
    pub perp_tag: u64,
}
impl Profile {
    /// Validate bounded mapping and bind every field used by normalization.
    pub fn commitment(&self) -> Result<[u8; 32], Error> {
        if self.revision == 0
            || self.markets.is_empty()
            || self.markets.len() > 32
            || self.markets.len() != self.config.markets.len()
            || self.source.domain != self.config.domain
            || self.source.venue != self.config.venue
            || self.source.account != self.config.venue_account
            || !self.config.sources.iter().any(|s| s.scope == self.source)
            || [&self.account, &self.environment, &self.evidence]
                .iter()
                .any(|v| v.is_empty() || v.len() > 256 || !v.is_ascii())
        {
            return Err(Error::Qualification);
        }
        DecimalScale::new(self.quote_places)?;
        for (i, m) in self.markets.iter().enumerate() {
            if m.market >= self.config.markets.len()
                || m.symbol.is_empty()
                || m.symbol.len() > 32
                || !m
                    .symbol
                    .bytes()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'-')
                || self.markets[..i]
                    .iter()
                    .any(|x| x.symbol == m.symbol || x.market == m.market)
            {
                return Err(Error::Qualification);
            }
            m.size.parse("0")?;
            m.price.parse("0")?;
            // Exact dimensional equality, not merely independently plausible grids.
            let unit = self.config.markets[m.market];
            let (n, d) = unit.conversion();
            let left = u128::from(n)
                .checked_mul(
                    10_u128
                        .checked_pow(u32::from(m.size.places) + u32::from(m.price.places))
                        .ok_or(Error::Limit)?,
                )
                .ok_or(Error::Limit)?;
            let right = u128::from(d)
                .checked_mul(u128::from(m.size.step))
                .and_then(|x| x.checked_mul(u128::from(m.price.step)))
                .and_then(|x| x.checked_mul(10_u128.checked_pow(u32::from(self.quote_places))?))
                .ok_or(Error::Limit)?;
            if left != right {
                return Err(Error::Qualification);
            }
        }
        let mut h = Sha256::new();
        h.update(b"CINDER-PACIFICA-PROFILE-1\0");
        h.update(cinder_journal::wire::encode_config(&self.config)?);
        h.update(self.source.namespace.bytes());
        h.update(
            serde_json::to_vec(&(
                &self.account,
                &self.environment,
                self.revision,
                &self.evidence,
                self.precision,
                self.fills,
                &self.markets,
                self.quote_places,
                self.perp_tag,
            ))
            .map_err(|_| Error::Codec)?,
        );
        Ok(h.finalize().into())
    }
    /// Source-specific amount, never an external equity-to-cash conversion.
    pub fn quote(&self, value: &str) -> Result<QuoteAtoms, Error> {
        Ok(QuoteAtoms::from_decimal(
            self.config.quote,
            value,
            DecimalScale::new(self.quote_places)?,
        )?)
    }
    pub(crate) fn mapping(&self, symbol: &str) -> Result<&Mapping, Error> {
        self.markets
            .iter()
            .find(|m| m.symbol == symbol)
            .ok_or(Error::Qualification)
    }
}
