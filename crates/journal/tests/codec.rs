//! Exhaustive current event variants and bounded canonical decoder regressions.
mod support;
use cinder_journal::{model::*, wire::*, *};
use cinder_kernel::{
    identity::*,
    ledger::{economics::*, evidence::*, *},
    math::Rounding,
    position::*,
};
use support::*;

fn variants() -> Vec<Event> {
    let mut changes = vec![
        Change::BindExecution {
            market: q(0).unit(),
            side: Side::Buy,
        },
        Change::BindExecution {
            market: q(0).unit(),
            side: Side::Sell,
        },
        Change::TransferDebit {
            source: Location::Vault,
            destination: Location::Venue,
            amount: cash(1),
        },
        Change::TransferArrival { debit: key(1) },
        Change::Economics(EconomicChange::BrokerFee {
            customer: user(1),
            amount: cash(1),
        }),
        Change::Economics(EconomicChange::FundingBoundary {
            market: q(0).unit(),
            expected_version: 0,
        }),
        Change::Economics(EconomicChange::FundingSettlement {
            boundary: key(1),
            native: cash(-2),
        }),
        Change::Economics(EconomicChange::CorrectFundingNative {
            boundary: key(1),
            native: cash(0),
        }),
    ];
    for owner in [Owner::House, Owner::Suspense, Owner::Customer(user(1))] {
        for location in [Location::Vault, Location::Venue] {
            changes.push(Change::Receipt {
                owner,
                location,
                amount: cash(i128::MAX),
            });
        }
    }
    for pnl in [
        None,
        Some(NativePnl::Gross(cash(i128::MIN))),
        Some(NativePnl::NetOfFee(cash(0))),
    ] {
        if let Some(pnl) = pnl {
            changes.push(Change::Economics(EconomicChange::CorrectExecutionPnl {
                execution: key(1),
                pnl,
            }));
        }
        for target in [
            FillTarget::House,
            FillTarget::Unattributed,
            FillTarget::Customer(attempt(1)),
        ] {
            changes.push(Change::Fill {
                target,
                quantity: q(i64::MIN),
                price: p(u64::MAX),
            });
            changes.push(Change::Economics(EconomicChange::Execution {
                target,
                quantity: q(-3),
                price: p(6),
                fee: cash(-4),
                pnl,
            }));
        }
    }
    for rounding in [
        Rounding::Exact,
        Rounding::TowardZero,
        Rounding::Floor,
        Rounding::Ceil,
    ] {
        // Invalid economic denominator is intentionally retained losslessly.
        for rate in [
            None,
            Some(FundingRate {
                numerator: cash(-1),
                denominator: 0,
                rounding,
            }),
        ] {
            for native in [None, Some(cash(0))] {
                changes.push(Change::Economics(EconomicChange::FundingInputs {
                    boundary: key(1),
                    rate,
                    native,
                }));
            }
        }
    }
    for complete in [false, true] {
        for known in [false, true] {
            changes.push(Change::Reconcile(NativeCheck {
                expected_version: 3,
                cash: known.then(|| cash(0)),
                funding: known.then(|| cash(-1)),
                positions: known.then(|| vec![Position::flat(config().markets[0].unit())]),
                complete,
                resolves: vec![CheckResolution {
                    check: key(1),
                    applied_effects: vec![key(2), key(3)],
                }],
            }));
        }
    }
    changes
        .into_iter()
        .enumerate()
        .map(|(n, c)| event(n as u64 + 1, c))
        .collect()
}
#[test]
fn every_p03_p04_event_variant_is_losslessly_canonical() {
    for e in variants() {
        let b = encode_event(&e).unwrap();
        assert_eq!(&b[..12], b"CINDER-J\0\0\x02\x02");
        assert_eq!(decode_event(&b).unwrap(), e);
        assert_eq!(encode_event(&decode_event(&b).unwrap()).unwrap(), b);
        for n in 0..b.len() {
            assert!(decode_event(&b[..n]).is_err());
        }
        let mut trailing = b.clone();
        trailing.push(0);
        assert!(decode_event(&trailing).is_err());
        let mut version = b;
        version[10] = 99;
        assert_eq!(decode_event(&version), Err(Error::Version));
    }
}
#[test]
fn config_retains_exact_conversion_and_rejects_bad_boundaries() {
    let mut c = config();
    c.markets[0] = Market::new(c.markets[0].unit(), 7, 13).unwrap();
    let b = encode_config(&c).unwrap();
    assert_eq!(decode_config(&b).unwrap(), c);
    for n in 0..b.len() {
        assert!(decode_config(&b[..n]).is_err());
    }
    let mut b = b;
    b.push(0);
    assert_eq!(decode_config(&b), Err(Error::Codec));
    assert_eq!(decode_config(&vec![0; MAX_RECORD + 1]), Err(Error::Limit));
}
#[test]
fn full_transaction_retains_optional_cuts_raw_bytes_and_all_controls() {
    let mut tx = transaction(
        Head {
            sequence: 4,
            hash: [9; 32],
        },
        1,
        variants(),
        vec![
            reserve(1, 2),
            Control::Reserve {
                request: request(2),
                reservations: vec![
                    Reservation {
                        resource: Resource::House,
                        amount: cash(1),
                    },
                    Reservation {
                        resource: Resource::Location(Location::Vault),
                        amount: cash(1),
                    },
                ],
            },
            prepare(1),
            Control::Expose(attempt(1)),
            Control::Release(request(1)),
        ],
    );
    tx.inputs[0].source_cut = None;
    tx.inputs[1].event = None;
    tx.inputs[2].event.as_mut().unwrap().key = RecordKey::Request(request(1));
    tx.inputs[3].event.as_mut().unwrap().key = RecordKey::Attempt(attempt(1));
    let bytes = encode_transaction(&tx).unwrap();
    assert_eq!(decode_transaction(&bytes).unwrap(), tx);
    for n in [0, 1, 8, 9, 10, 11, 12, bytes.len() - 1] {
        assert!(decode_transaction(&bytes[..n]).is_err());
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(decode_transaction(&trailing).is_err());
    tx.inputs = vec![input(receipt(1, Owner::House, 1)); MAX_ITEMS + 1];
    assert_eq!(encode_transaction(&tx), Err(Error::Limit));
}
#[test]
fn property_decoder_never_panics_on_bounded_mutated_frames() {
    let b = encode_transaction(&transaction(
        Head::default(),
        1,
        variants(),
        vec![reserve(1, 1)],
    ))
    .unwrap();
    // Seeded finite mutation exploration, not a formal parser proof.
    let mut seed = 0x12345678_u64;
    for _ in 0..2048 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut mutated = b.clone();
        let pos = seed as usize % mutated.len();
        mutated[pos] ^= (seed >> 32) as u8 | 1;
        if let Ok(tx) = decode_transaction(&mutated) {
            assert_eq!(encode_transaction(&tx).unwrap(), mutated);
        }
    }
}
