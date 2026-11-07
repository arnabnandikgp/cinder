//! Checked integer arithmetic with explicit rounding and exact decimal decoding.

use crate::Error;

/// A caller-selected rounding direction, never an implicit financial policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rounding {
    /// Reject a fractional result.
    Exact,
    /// Discard the fractional magnitude; retain its owner elsewhere when required.
    TowardZero,
    /// Greatest integer no larger than the rational result.
    Floor,
    /// Least integer no smaller than the rational result.
    Ceil,
}

/// Compute `value * multiplier / divisor` without an overflowing signed product.
///
/// Split magnitude as `q*d + r`: `q*m + (r*m)/d`. Because `m,d` are u64,
/// `r*m` fits u128. Checked overflow of `q*m` implies the final magnitude is also
/// unrepresentable. Sign is restored only after rounding, including i128::MIN.
pub fn mul_div(
    value: i128,
    multiplier: u64,
    divisor: u64,
    rounding: Rounding,
) -> Result<i128, Error> {
    if divisor == 0 {
        return Err(Error::DivisionByZero);
    }
    let magnitude = value.unsigned_abs();
    let d = u128::from(divisor);
    let m = u128::from(multiplier);
    let residual_product = (magnitude % d).checked_mul(m).ok_or(Error::Overflow)?;
    let remainder = residual_product % d;
    // Exactness is checked before the quotient range, even if both would fail.
    if rounding == Rounding::Exact && remainder != 0 {
        return Err(Error::Inexact);
    }
    let mut result = (magnitude / d)
        .checked_mul(m)
        .and_then(|base| base.checked_add(residual_product / d))
        .ok_or(Error::Overflow)?;
    if remainder != 0 {
        let increment = match rounding {
            Rounding::Exact | Rounding::TowardZero => false,
            Rounding::Floor => value < 0,
            Rounding::Ceil => value >= 0,
        };
        if increment {
            result = result.checked_add(1).ok_or(Error::Overflow)?;
        }
    }
    signed_magnitude(result, value < 0)
}

fn signed_magnitude(magnitude: u128, negative: bool) -> Result<i128, Error> {
    if negative && magnitude == (1_u128 << 127) {
        return Ok(i128::MIN);
    }
    let value = i128::try_from(magnitude).map_err(|_| Error::Overflow)?;
    if negative {
        value.checked_neg().ok_or(Error::Overflow)
    } else {
        Ok(value)
    }
}

/// Explicit number of decimal places, zero through 38; not a venue default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecimalScale(u8);

impl DecimalScale {
    /// Reject scales whose power of ten cannot fit the signed accounting range.
    pub fn new(places: u8) -> Result<Self, Error> {
        if places > 38 {
            return Err(Error::Overprecision);
        }
        Ok(Self(places))
    }

    /// Configured fractional places.
    pub fn places(self) -> u8 {
        self.0
    }

    /// Decode decimal text exactly, without floating point or implicit rounding.
    ///
    /// Grammar: optional minus, then `0` or a nonzero digit followed by digits,
    /// optionally a decimal point and 1..scale digits. No plus, whitespace,
    /// exponent, leading zeros, negative zero, separators or Unicode digits.
    /// Fractional trailing zeros within the scale normalize to the same atoms.
    /// At most 80 input bytes; valid i128 values at supported scales fit this bound.
    pub fn parse(self, input: &str) -> Result<i128, Error> {
        if input.is_empty() || input.len() > 80 {
            return Err(Error::InvalidDecimal);
        }
        let (negative, digits) = match input.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, input),
        };
        let mut parts = digits.split('.');
        let whole = parts.next().ok_or(Error::InvalidDecimal)?;
        let fractional = parts.next();
        if parts.next().is_some()
            || whole.is_empty()
            || (whole.len() > 1 && whole.starts_with('0'))
            || fractional.is_some_and(str::is_empty)
        {
            return Err(Error::InvalidDecimal);
        }
        let fraction = fractional.unwrap_or("");
        if fraction.len() > usize::from(self.0) {
            return Err(Error::Overprecision);
        }
        let mut magnitude = 0_u128;
        for digit in whole.bytes().chain(fraction.bytes()) {
            if !digit.is_ascii_digit() {
                return Err(Error::InvalidDecimal);
            }
            magnitude = magnitude
                .checked_mul(10)
                .and_then(|n| n.checked_add(u128::from(digit - b'0')))
                .ok_or(Error::Overflow)?;
        }
        for _ in fraction.len()..usize::from(self.0) {
            magnitude = magnitude.checked_mul(10).ok_or(Error::Overflow)?;
        }
        if negative && magnitude == 0 {
            return Err(Error::InvalidDecimal);
        }
        signed_magnitude(magnitude, negative)
    }
}
