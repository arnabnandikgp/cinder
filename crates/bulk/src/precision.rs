//! Exact decimal/exponent normalization at the native boundary, never f64.
use crate::{Error, symbol};

/// Decode a native numeric lexeme, including bounded scientific notation.
/// Strings and JSON numeric tokens are unquoted by the caller. Only exactly
/// representable results are accepted; extra zero digits are harmless.
pub fn scaled(text: &str, places: u8) -> Result<i128, Error> {
    if places > 38 {
        return Err(Error::Limit);
    }
    let (negative, coefficient, exponent) = decimal(text)?;
    if coefficient == 0 {
        return Ok(0);
    }
    let power = i32::from(places) + exponent;
    if power < -38 {
        return Err(Error::Invalid);
    }
    let factor = 10_u128
        .checked_pow(power.unsigned_abs())
        .ok_or(Error::Limit)?;
    let magnitude = if power >= 0 {
        coefficient.checked_mul(factor).ok_or(Error::Limit)?
    } else {
        if !coefficient.is_multiple_of(factor) {
            return Err(Error::Invalid);
        }
        coefficient / factor
    };
    if negative && magnitude == 1_u128 << 127 {
        return Ok(i128::MIN);
    }
    let value = i128::try_from(magnitude).map_err(|_| Error::Limit)?;
    if negative {
        value.checked_neg().ok_or(Error::Limit)
    } else {
        Ok(value)
    }
}

/// Bounded exact lexical parts for reported decimals that are not accounting
/// atoms (displayed averages/rates). No precision is inferred from these parts.
pub(crate) fn decimal(text: &str) -> Result<(bool, u128, i32), Error> {
    if text.is_empty() || text.len() > 80 {
        return Err(Error::Limit);
    }
    let (negative, rest) = text.strip_prefix('-').map_or((false, text), |s| (true, s));
    let mut exponents = rest.split(['e', 'E']);
    let mantissa = exponents.next().ok_or(Error::Invalid)?;
    let exponent = match exponents.next() {
        None => 0_i16,
        Some(value) => {
            let digits = value.strip_prefix(['+', '-']).unwrap_or(value);
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return Err(Error::Invalid);
            }
            value.parse::<i16>().map_err(|_| Error::Limit)?
        }
    };
    if exponents.next().is_some() {
        return Err(Error::Invalid);
    }
    let mut parts = mantissa.split('.');
    let whole = parts.next().ok_or(Error::Invalid)?;
    let fraction = parts.next();
    if whole.is_empty()
        || (whole.len() > 1 && whole.starts_with('0'))
        || fraction.is_some_and(str::is_empty)
        || parts.next().is_some()
    {
        return Err(Error::Invalid);
    }
    let fraction = fraction.unwrap_or("");
    let mut coefficient = 0_u128;
    for digit in whole.bytes().chain(fraction.bytes()) {
        if !digit.is_ascii_digit() {
            return Err(Error::Invalid);
        }
        coefficient = coefficient
            .checked_mul(10)
            .and_then(|n| n.checked_add(u128::from(digit - b'0')))
            .ok_or(Error::Limit)?;
    }
    if negative && coefficient == 0 {
        return Err(Error::Invalid);
    }
    Ok((
        negative,
        coefficient,
        i32::from(exponent) - i32::try_from(fraction.len()).map_err(|_| Error::Limit)?,
    ))
}

/// Explicit selected-market grids; construction does not qualify native math.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Market {
    pub(crate) symbol: String,
    pub(crate) size_step: u64,
    pub(crate) price_step: u64,
    pub(crate) quote_places: u8,
}
impl Market {
    /// Exact native instrument, not an alias inferred across venues.
    pub fn symbol(&self) -> &str {
        &self.symbol
    }
    /// Bind exact native size/price increments at the documented 1e8 wire scale
    /// and an independently chosen quote accounting precision. No defaults.
    pub fn new(
        name: &str,
        size_step: &str,
        price_step: &str,
        quote_places: u8,
    ) -> Result<Self, Error> {
        symbol(name)?;
        let positive = |s| {
            u64::try_from(scaled(s, 8)?)
                .ok()
                .filter(|n| *n > 0)
                .ok_or(Error::Invalid)
        };
        if quote_places > 38 {
            return Err(Error::Invalid);
        }
        Ok(Self {
            symbol: name.into(),
            size_step: positive(size_step)?,
            price_step: positive(price_step)?,
            quote_places,
        })
    }
    pub(crate) fn units(&self, text: &str, step: u64) -> Result<u64, Error> {
        let amount = u64::try_from(scaled(text, 8)?).map_err(|_| Error::Invalid)?;
        if amount == 0 || !amount.is_multiple_of(step) {
            return Err(Error::Invalid);
        }
        Ok(amount / step)
    }
    /// Positive executed quantity in configured exact native lots.
    pub fn quantity(&self, text: &str) -> Result<u64, Error> {
        self.units(text, self.size_step)
    }
    /// Positive execution price in configured exact ticks, not rounded basis.
    pub fn price(&self, text: &str) -> Result<u64, Error> {
        self.units(text, self.price_step)
    }
    /// Signed fee in configured quote atoms; negative denotes rebate.
    pub fn fee(&self, text: &str) -> Result<i128, Error> {
        scaled(text, self.quote_places)
    }
}
