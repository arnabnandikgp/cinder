//! P02 finite-width and canonical-value conformance, not a complete ledger.
use cinder_kernel::{Error, amounts::*, codec::Canonical, identity::*, math::*};

fn asset() -> AssetUnit {
    AssetUnit {
        asset: AssetId::new([1; 32]).unwrap(),
        precision: PrecisionVersion::new(1).unwrap(),
    }
}
fn market() -> MarketUnit {
    MarketUnit {
        market: MarketId::new([2; 32]).unwrap(),
        precision: PrecisionVersion::new(1).unwrap(),
        quote: asset(),
    }
}

#[test]
fn decimal_grammar_precision_and_full_signed_range() {
    let scale = DecimalScale::new(3).unwrap();
    for (text, atoms) in [
        ("0", 0),
        ("0.001", 1),
        ("-0.001", -1),
        ("12.30", 12300),
        ("12.3", 12300),
    ] {
        assert_eq!(scale.parse(text), Ok(atoms), "{text}");
    }
    for text in [
        "", "+1", "01", "1e3", " 1", "1 ", "1.", ".1", "1.2.3", "--1", "-0", "-0.000", "NaN",
        "inf", "１", "1_000", "\0",
    ] {
        assert!(scale.parse(text).is_err(), "{text:?}");
    }
    assert_eq!(scale.parse("1.0000"), Err(Error::Overprecision));
    assert_eq!(DecimalScale::new(39), Err(Error::Overprecision));
    let integer = DecimalScale::new(0).unwrap();
    assert_eq!(integer.parse(&i128::MIN.to_string()), Ok(i128::MIN));
    assert_eq!(integer.parse(&i128::MAX.to_string()), Ok(i128::MAX));
    assert_eq!(
        integer.parse("170141183460469231731687303715884105728"),
        Err(Error::Overflow)
    );
    assert_eq!(
        integer.parse("-170141183460469231731687303715884105729"),
        Err(Error::Overflow)
    );
    assert_eq!(
        DecimalScale::new(38).unwrap().parse("1"),
        Ok(10_i128.pow(38))
    );
    assert_eq!(
        DecimalScale::new(38).unwrap().parse("2"),
        Err(Error::Overflow)
    );
    assert!(integer.parse(&"9".repeat(81)).is_err());
}

#[test]
fn decimal_grid_and_dimension_bounds_are_not_silent_rounding() {
    let grid = DecimalGrid::new(DecimalScale::new(3).unwrap(), 5).unwrap();
    assert_eq!(
        QuantityLots::from_decimal(market(), "-0.015", grid)
            .unwrap()
            .lots(),
        -3
    );
    assert_eq!(
        QuantityLots::from_decimal(market(), "0.016", grid),
        Err(Error::Inexact)
    );
    assert_eq!(
        PriceTicks::from_decimal(market(), "0.010", grid)
            .unwrap()
            .ticks(),
        2
    );
    assert_eq!(
        PriceTicks::from_decimal(market(), "-0.010", grid),
        Err(Error::InvalidSign)
    );
    assert_eq!(PriceTicks::new(market(), 0), Err(Error::InvalidSign));
    assert_eq!(
        DecimalGrid::new(DecimalScale::new(0).unwrap(), 0),
        Err(Error::InvalidSign)
    );
    let integers = DecimalGrid::new(DecimalScale::new(0).unwrap(), 1).unwrap();
    assert_eq!(
        QuantityLots::from_decimal(market(), "9223372036854775808", integers),
        Err(Error::Overflow)
    );
    assert_eq!(
        PriceTicks::from_decimal(market(), "18446744073709551616", integers),
        Err(Error::Overflow)
    );
    assert_eq!(
        PriceTicks::from_decimal(market(), &u64::MAX.to_string(), integers)
            .unwrap()
            .ticks(),
        u64::MAX
    );
}

#[test]
fn checked_operations_reject_mixed_units_versions_and_overflow_without_mutation() {
    let q = QuoteAtoms::new(asset(), i128::MAX);
    assert_eq!(
        q.checked_add(QuoteAtoms::new(asset(), 1)),
        Err(Error::Overflow)
    );
    assert_eq!(q.atoms(), i128::MAX);
    assert_eq!(
        QuoteAtoms::new(asset(), i128::MIN).checked_neg(),
        Err(Error::Overflow)
    );
    let other = AssetUnit {
        asset: AssetId::new([9; 32]).unwrap(),
        ..asset()
    };
    assert_eq!(
        q.checked_add(QuoteAtoms::new(other, 0)),
        Err(Error::UnitMismatch)
    );
    let revision = AssetUnit {
        precision: PrecisionVersion::new(2).unwrap(),
        ..asset()
    };
    assert_eq!(
        q.checked_add(QuoteAtoms::new(revision, 0)),
        Err(Error::UnknownVersion)
    );
    let lots = QuantityLots::new(market(), i64::MIN);
    assert_eq!(lots.magnitude(), 1_u64 << 63);
    assert_eq!(lots.checked_neg(), Err(Error::Overflow));
    assert_eq!(
        lots.checked_sub(QuantityLots::new(market(), 1)),
        Err(Error::Overflow)
    );
    let other_market = MarketUnit {
        market: MarketId::new([9; 32]).unwrap(),
        ..market()
    };
    assert_eq!(
        lots.checked_add(QuantityLots::new(other_market, 0)),
        Err(Error::UnitMismatch)
    );
    let other_quote = MarketUnit {
        quote: other,
        ..market()
    };
    assert_eq!(
        BasisAtoms::new(market(), 7).checked_sub(BasisAtoms::new(other_quote, 2)),
        Err(Error::UnitMismatch)
    );
}

#[test]
fn mul_div_extremes_do_not_need_an_overflowing_intermediate_product() {
    for value in [i128::MIN, i128::MIN + 1, -1, 0, 1, i128::MAX] {
        assert_eq!(
            mul_div(value, u64::MAX, u64::MAX, Rounding::Exact),
            Ok(value)
        );
        assert_eq!(mul_div(value, 0, 1, Rounding::Exact), Ok(0));
        assert_eq!(
            mul_div(value, 0, 0, Rounding::Exact),
            Err(Error::DivisionByZero)
        );
    }
    assert_eq!(
        mul_div(i128::MAX, 2, 1, Rounding::TowardZero),
        Err(Error::Overflow)
    );
    assert_eq!(
        mul_div(i128::MIN, 2, 1, Rounding::TowardZero),
        Err(Error::Overflow)
    );
    assert_eq!(
        mul_div(i128::MIN, u64::MAX, 1, Rounding::Floor),
        Err(Error::Overflow)
    );
    assert_eq!(mul_div(-1, 1, 2, Rounding::TowardZero), Ok(0));
    assert_eq!(mul_div(-1, 1, 2, Rounding::Floor), Ok(-1));
    assert_eq!(mul_div(-1, 1, 2, Rounding::Ceil), Ok(0));
    assert_eq!(mul_div(1, 1, 2, Rounding::Ceil), Ok(1));
    assert_eq!(mul_div(1, 1, 2, Rounding::Exact), Err(Error::Inexact));
}

#[test]
fn property_exhaustive_small_rounding_matches_independent_signed_division_oracle() {
    for value in -128_i128..=127 {
        for multiplier in 0_u64..=16 {
            for divisor in 1_u64..=16 {
                let numerator = value * i128::from(multiplier); // Bounded test oracle.
                let denominator = i128::from(divisor);
                let truncated = numerator / denominator;
                let remainder = numerator % denominator;
                let floor = truncated - i128::from(remainder < 0);
                let ceil = truncated + i128::from(remainder > 0);
                assert_eq!(
                    mul_div(value, multiplier, divisor, Rounding::TowardZero),
                    Ok(truncated)
                );
                assert_eq!(
                    mul_div(value, multiplier, divisor, Rounding::Floor),
                    Ok(floor)
                );
                assert_eq!(
                    mul_div(value, multiplier, divisor, Rounding::Ceil),
                    Ok(ceil)
                );
                assert_eq!(
                    mul_div(value, multiplier, divisor, Rounding::Exact),
                    if remainder == 0 {
                        Ok(truncated)
                    } else {
                        Err(Error::Inexact)
                    }
                );
            }
        }
    }
}

#[test]
fn property_full_width_mul_div_matches_independent_bigint_vectors() {
    let mut count = 0;
    for line in include_str!("fixtures/mul-div-v1.tsv")
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
    {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 7);
        let value: i128 = fields[0].parse().unwrap();
        let multiplier: u64 = fields[1].parse().unwrap();
        let divisor: u64 = fields[2].parse().unwrap();
        for (mode, expected) in [
            Rounding::TowardZero,
            Rounding::Floor,
            Rounding::Ceil,
            Rounding::Exact,
        ]
        .into_iter()
        .zip(&fields[3..])
        {
            let expected = match *expected {
                "Overflow" => Err(Error::Overflow),
                "Inexact" => Err(Error::Inexact),
                integer => Ok(integer.parse().unwrap()),
            };
            assert_eq!(
                mul_div(value, multiplier, divisor, mode),
                expected,
                "{line}, {mode:?}"
            );
        }
        count += 1;
    }
    assert_eq!(count, 64);
}

#[test]
fn property_basis_allocation_preserves_signed_remainders_and_full_close() {
    for value in [i128::MIN, -100, -1, 0, 1, 100, i128::MAX] {
        let basis = BasisAtoms::new(market(), value);
        for total in [1, 3, 19, u64::MAX] {
            for part in [0, 1, total / 2, total] {
                let (allocated, remaining) = basis.split(part, total).unwrap();
                assert_eq!(allocated.checked_add(remaining), Ok(basis));
                if part == total {
                    assert_eq!(remaining.atoms(), 0);
                }
            }
        }
    }
    assert_eq!(
        BasisAtoms::new(market(), -100)
            .split(1, 3)
            .unwrap()
            .0
            .atoms(),
        -33
    );
    assert_eq!(
        BasisAtoms::new(market(), 100)
            .split(1, 3)
            .unwrap()
            .1
            .atoms(),
        67
    );
    assert_eq!(
        BasisAtoms::new(market(), 100).split(4, 3),
        Err(Error::InvalidRatio)
    );
    assert_eq!(
        BasisAtoms::new(market(), 100).split(0, 0),
        Err(Error::DivisionByZero)
    );
}

#[test]
fn value_encodings_have_golden_bytes_not_only_self_roundtrips() {
    let mut expected = b"CINDER\0\0\x01\x01".to_vec();
    expected.extend([1; 32]);
    expected.extend([0, 0, 0, 1]);
    expected.extend([255; 15]);
    expected.push(254); // Two's complement -2, i128 big-endian.
    let amount = QuoteAtoms::new(asset(), -2);
    assert_eq!(amount.encode(), expected);
    assert_eq!(QuoteAtoms::decode_in(&expected, asset()), Ok(amount));
    assert_eq!(
        QuoteAtoms::from_decimal(asset(), "1.0", DecimalScale::new(2).unwrap())
            .unwrap()
            .encode(),
        QuoteAtoms::from_decimal(asset(), "1.00", DecimalScale::new(2).unwrap())
            .unwrap()
            .encode()
    );
    for value in [i128::MIN, -100, 0, 100, i128::MAX] {
        let amount = QuoteAtoms::new(asset(), value);
        let basis = BasisAtoms::new(market(), value);
        assert_eq!(QuoteAtoms::decode_in(&amount.encode(), asset()), Ok(amount));
        assert_eq!(BasisAtoms::decode_in(&basis.encode(), market()), Ok(basis));
        assert!(BasisAtoms::decode(&amount.encode()).is_err());
    }
    for value in [i64::MIN, -1, 0, 1, i64::MAX] {
        let quantity = QuantityLots::new(market(), value);
        assert_eq!(
            QuantityLots::decode_in(&quantity.encode(), market()),
            Ok(quantity)
        );
    }
    for value in [1, u64::MAX] {
        let price = PriceTicks::new(market(), value).unwrap();
        assert_eq!(PriceTicks::decode_in(&price.encode(), market()), Ok(price));
    }
}

#[test]
fn value_decoders_reject_unknown_schema_units_truncation_and_trailing_bytes() {
    let original = QuoteAtoms::new(asset(), 7).encode();
    for end in 0..original.len() {
        assert!(QuoteAtoms::decode(&original[..end]).is_err());
    }
    let mut trailing = original.clone();
    trailing.push(0);
    assert_eq!(QuoteAtoms::decode(&trailing), Err(Error::InvalidEncoding));
    let mut unknown = original.clone();
    unknown[8] = 2;
    assert_eq!(QuoteAtoms::decode(&unknown), Err(Error::UnknownVersion));
    unknown = original.clone();
    unknown[0] = b'X';
    assert_eq!(QuoteAtoms::decode(&unknown), Err(Error::InvalidEncoding));
    let unit = AssetUnit {
        precision: PrecisionVersion::new(2).unwrap(),
        ..asset()
    };
    assert_eq!(
        QuoteAtoms::decode_in(&original, unit),
        Err(Error::UnknownVersion)
    );
    let unit = AssetUnit {
        asset: AssetId::new([8; 32]).unwrap(),
        ..asset()
    };
    assert_eq!(
        QuoteAtoms::decode_in(&original, unit),
        Err(Error::UnitMismatch)
    );
    let mut price = PriceTicks::new(market(), 1).unwrap().encode();
    let len = price.len();
    price[len - 8..].fill(0);
    assert_eq!(PriceTicks::decode(&price), Err(Error::InvalidSign));
}

// This is only a small conformance driver. P03 owns the production fill transition.
fn fixture_fill(q: i64, b: i128, c: i128, x: i64, price: i128) -> (i64, i128, i128) {
    let basis = BasisAtoms::new(market(), b);
    let new_q = QuantityLots::new(market(), q)
        .checked_add(QuantityLots::new(market(), x))
        .unwrap()
        .lots();
    if q == 0 || q.signum() == x.signum() {
        return (
            new_q,
            basis
                .checked_add(BasisAtoms::new(market(), i128::from(x) * price))
                .unwrap()
                .atoms(),
            c,
        );
    }
    let closed = q.unsigned_abs().min(x.unsigned_abs());
    let (allocated, remaining) = basis.split(closed, q.unsigned_abs()).unwrap();
    let signed_closed = i128::from(q.signum()) * i128::from(closed);
    let realized = QuoteAtoms::new(asset(), signed_closed * price)
        .checked_sub(QuoteAtoms::new(asset(), allocated.atoms()))
        .unwrap();
    let open = i128::from(x) + signed_closed;
    (
        new_q,
        remaining
            .checked_add(BasisAtoms::new(market(), open * price))
            .unwrap()
            .atoms(),
        QuoteAtoms::new(asset(), c)
            .checked_add(realized)
            .unwrap()
            .atoms(),
    )
}

#[test]
fn v02_signed_basis_vectors_use_primitives_without_importing_research_runtime() {
    for line in include_str!("fixtures/basis-v1.tsv")
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
    {
        let v: Vec<i128> = line
            .split('\t')
            .map(|field| field.parse().unwrap())
            .collect();
        assert_eq!(v.len(), 8);
        let q = i64::try_from(v[0]).unwrap();
        let x = i64::try_from(v[3]).unwrap();
        let after = fixture_fill(q, v[1], v[2], x, v[4]);
        assert_eq!(after, (i64::try_from(v[5]).unwrap(), v[6], v[7]));
        for mark in [1, 35, 40, 100] {
            let before_equity = v[2] + i128::from(q) * mark - v[1];
            let after_equity = after.2 + i128::from(after.0) * mark - after.1;
            assert_eq!(after_equity - before_equity, i128::from(x) * (mark - v[4]));
        }
    }
}
