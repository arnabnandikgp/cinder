//! Synthetic normalization contract, not a Pacifica/BULK adapter or wire fixture.
use cinder_kernel::{
    Error,
    amounts::{AssetUnit, QuoteAtoms},
    identity::{AssetId, EconomicEventId, PrecisionVersion},
    ledger::economics::NativePnl,
    math::DecimalScale,
};
use cinder_ports::NormalizeObservation;

struct WireObservation {
    execution: &'static str,
    fee: &'static str,
    pnl: Option<(&'static str, bool)>, // Qualified fixture convention: true is fee-inclusive.
    revision: u32,
    arrival: u64, // Not a semantic economic identity.
}
#[derive(Debug, PartialEq, Eq)]
struct Normalized {
    id: EconomicEventId,
    fee: QuoteAtoms,
    gross_pnl: Option<QuoteAtoms>,
}
struct FixtureNormalizer;
impl NormalizeObservation for FixtureNormalizer {
    type Raw = WireObservation;
    type Normalized = Normalized;
    type Error = Error;
    fn normalize(&self, raw: &Self::Raw) -> Result<Self::Normalized, Self::Error> {
        if raw.revision != 1 {
            return Err(Error::UnknownVersion);
        }
        let unit = AssetUnit {
            asset: AssetId::new([1; 32])?,
            precision: PrecisionVersion::new(1)?,
        };
        let parse = |text| QuoteAtoms::from_decimal(unit, text, DecimalScale::new(2)?);
        let fee = parse(raw.fee)?;
        let gross_pnl = raw
            .pnl
            .map(|(text, inclusive)| {
                let value = parse(text)?;
                (if inclusive {
                    NativePnl::NetOfFee(value)
                } else {
                    NativePnl::Gross(value)
                })
                .gross(fee)
            })
            .transpose()?;
        Ok(Normalized {
            id: EconomicEventId::new(raw.execution.as_bytes())?,
            fee,
            gross_pnl,
        })
    }
}

#[test]
fn rest_and_ws_conventions_normalize_to_equal_economic_output() {
    let rest = WireObservation {
        execution: "fill-1",
        fee: "0.25",
        pnl: Some(("2.00", false)),
        revision: 1,
        arrival: 10,
    };
    let ws = WireObservation {
        pnl: Some(("1.75", true)),
        arrival: 20,
        ..rest
    };
    assert_ne!(rest.arrival, ws.arrival);
    let a = FixtureNormalizer.normalize(&rest).unwrap();
    let b = FixtureNormalizer.normalize(&ws).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.gross_pnl.unwrap().atoms(), 200);
    assert_eq!(a.fee.atoms(), 25);
}

#[test]
fn unknown_revision_overprecision_and_missing_pnl_never_become_zero() {
    let unknown = WireObservation {
        execution: "fill-1",
        fee: "0.25",
        pnl: None,
        revision: 1,
        arrival: 10,
    };
    assert_eq!(
        FixtureNormalizer.normalize(&unknown).unwrap().gross_pnl,
        None
    );
    let unsupported = WireObservation {
        revision: 2,
        ..unknown
    };
    assert_eq!(
        FixtureNormalizer.normalize(&unsupported),
        Err(Error::UnknownVersion)
    );
    let overprecise = WireObservation {
        fee: "0.001",
        ..unknown
    };
    assert_eq!(
        FixtureNormalizer.normalize(&overprecise),
        Err(Error::Overprecision)
    );
    let corrupt = WireObservation {
        fee: "NaN",
        ..unknown
    };
    assert_eq!(
        FixtureNormalizer.normalize(&corrupt),
        Err(Error::InvalidDecimal)
    );
}
