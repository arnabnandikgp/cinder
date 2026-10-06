//! Independent bounded integer oracle. No production position/math calls in the
//! reference; exact normalized prices, not qualified Pacifica wire precision.
use cinder_kernel::{
    amounts::*,
    identity::*,
    ledger::{economics::*, evidence::Disposition, *},
    math::Rounding,
    position::*,
};
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct RefBook {
    cash: i128,
    quantity: i64,
    basis: i128,
    funding: i128,
}
impl RefBook {
    fn equity(self, mark: i128) -> i128 {
        self.cash + self.funding + i128::from(self.quantity) * mark - self.basis
    }
    fn fill(&mut self, delta: i64, price: i128, fee: i128) {
        // Partition inventory by direction and calculate released cost using an
        // independent widened integer quotient. Residue remains in inventory.
        let opposing = self.quantity.signum() * delta.signum() == -1;
        let closed = if opposing {
            self.quantity.unsigned_abs().min(delta.unsigned_abs())
        } else {
            0
        };
        let released = if closed == 0 {
            0
        } else {
            self.basis * i128::from(closed) / i128::from(self.quantity.unsigned_abs())
        };
        let close_value = i128::from(self.quantity.signum()) * i128::from(closed) * price;
        self.cash += close_value - released - fee;
        self.basis = self.basis - released + (i128::from(delta) * price + close_value);
        self.quantity += delta;
    }
}
fn config() -> Config {
    let domain = Domain {
        network: NetworkId::new([1; 32]).unwrap(),
        deployment: DeploymentId::new([2; 32]).unwrap(),
    };
    let quote = AssetUnit {
        asset: AssetId::new([3; 32]).unwrap(),
        precision: PrecisionVersion::new(1).unwrap(),
    };
    let venue = VenueId::new([4; 32]).unwrap();
    let venue_account = VenueAccountId::new([5; 32]).unwrap();
    let scope = EventScope {
        domain,
        venue,
        account: venue_account,
        namespace: NamespaceId::new([6; 32]).unwrap(),
    };
    let market = MarketUnit {
        market: MarketId::new([7; 32]).unwrap(),
        precision: quote.precision,
        quote,
    };
    Config {
        domain,
        quote,
        venue,
        venue_account,
        policy: PolicyVersion::new(1).unwrap(),
        sources: vec![Source {
            scope,
            location: Location::Venue,
        }],
        markets: vec![Market::new(market, 1, 1).unwrap()],
        customers: vec![user(1), user(2)],
    }
}
fn user(n: u8) -> AccountId {
    AccountId::new([n; 32]).unwrap()
}
fn cash(n: i128) -> QuoteAtoms {
    QuoteAtoms::new(config().quote, n)
}
fn q(n: i64) -> QuantityLots {
    QuantityLots::new(config().markets[0].unit(), n)
}
fn p(n: u64) -> PriceTicks {
    PriceTicks::new(q(0).unit(), n).unwrap()
}
fn key(n: u64) -> EventKey {
    EventKey {
        scope: config().sources[0].scope,
        event: EconomicEventId::new(&n.to_be_bytes()).unwrap(),
        leg: 0,
    }
}
fn event(n: u64, change: Change) -> Event {
    Event {
        key: RecordKey::Economic(key(n)),
        policy: config().policy,
        change,
    }
}
fn route(u: u8, buy: bool) -> AttemptKey {
    AttemptKey {
        request: RequestKey {
            domain: config().domain,
            account: user(u),
            request: RequestId::new([u * 2 + u8::from(buy); 32]).unwrap(),
        },
        attempt: AttemptId::new([u * 2 + u8::from(buy); 32]).unwrap(),
    }
}
fn setup() -> Ledger {
    let mut l = Ledger::new(config()).unwrap();
    for u in [1, 2] {
        l = l
            .apply(&event(
                u.into(),
                Change::Receipt {
                    owner: Owner::Customer(user(u)),
                    location: Location::Venue,
                    amount: cash(100),
                },
            ))
            .unwrap();
        for buy in [false, true] {
            l = l
                .apply(&Event {
                    key: RecordKey::Attempt(route(u, buy)),
                    policy: config().policy,
                    change: Change::BindExecution {
                        market: q(0).unit(),
                        side: if buy { Side::Buy } else { Side::Sell },
                    },
                })
                .unwrap();
        }
    }
    l.apply(&event(
        3,
        Change::Receipt {
            owner: Owner::House,
            location: Location::Venue,
            amount: cash(30),
        },
    ))
    .unwrap()
}
fn execution(n: u64, u: u8, delta: i64, price: u64, fee: i128) -> Event {
    event(
        n,
        Change::Economics(EconomicChange::Execution {
            target: FillTarget::Customer(route(u, delta > 0)),
            quantity: q(delta),
            price: p(price),
            fee: cash(fee),
            pnl: None,
        }),
    )
}
fn check(l: &Ledger, users: [RefBook; 2], native: RefBook, mark: u64) {
    let check_book = |actual: &Book, expected: RefBook| {
        assert_eq!(actual.cash().atoms(), expected.cash);
        assert_eq!(actual.funding().atoms(), expected.funding);
        assert_eq!(actual.positions()[0].quantity().lots(), expected.quantity);
        assert_eq!(actual.positions()[0].basis().atoms(), expected.basis);
    };
    for (i, b) in users.iter().enumerate() {
        check_book(l.book(Owner::Customer(user(i as u8 + 1))).unwrap(), *b);
    }
    check_book(
        l.book(Owner::House).unwrap(),
        RefBook {
            cash: 30,
            ..RefBook::default()
        },
    );
    check_book(l.venue(), native);
    l.check_bridge().unwrap();
    let values = users.map(|b| b.equity(mark.into()));
    let claims: i128 = values.iter().map(|v| (*v).max(0)).sum();
    let deficits: i128 = values.iter().map(|v| (-*v).max(0)).sum();
    let assets = native.equity(mark.into());
    let d = l.diagnostics(&[p(mark)]).unwrap();
    assert_eq!(d.net_assets.atoms(), assets);
    assert_eq!(d.customer_claims.atoms(), claims);
    assert_eq!(d.customer_deficits.atoms(), deficits);
    assert_eq!(d.shortfall.atoms(), (claims - assets).max(0));
    assert_eq!(assets, values.iter().sum::<i128>() + 30); // Negative debt is not positive backing.
}
fn step(l: &mut Ledger, e: &Event) {
    *l = l.apply(e).unwrap();
    assert_eq!(l.apply(e).unwrap(), *l, "duplicate economic input");
}
fn random(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *seed ^ (*seed >> 32)
}

#[test]
fn property_exhaustive_position_reference_and_marked_equity() {
    // 4,632 valid q/b/delta/price states; three independent valuation cuts each.
    let mut cases = 0;
    for lots in -3_i64..=3 {
        for magnitude in 0_i128..=31 {
            if lots == 0 && magnitude != 0 {
                continue;
            }
            for delta in -4_i64..=4 {
                if delta == 0 {
                    continue;
                }
                for price in [1_u64, 7, 11] {
                    let before = RefBook {
                        cash: 17,
                        quantity: lots,
                        basis: magnitude * i128::from(lots.signum()),
                        funding: 0,
                    };
                    let mut expected = before;
                    expected.fill(delta, price.into(), 0);
                    let actual = Position::new(q(lots), BasisAtoms::new(q(0).unit(), before.basis))
                        .unwrap()
                        .fill(config().markets[0], q(delta), p(price))
                        .unwrap();
                    assert_eq!(actual.position.quantity().lots(), expected.quantity);
                    assert_eq!(actual.position.basis().atoms(), expected.basis);
                    assert_eq!(actual.realized.atoms(), expected.cash - before.cash);
                    for mark in [1_i128, 50, 250] {
                        assert_eq!(
                            expected.equity(mark) - before.equity(mark),
                            i128::from(delta) * (mark - i128::from(price))
                        );
                    }
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 4632);
}

#[test]
fn property_seeded_joined_ledger_matches_independent_reference_at_every_cut() {
    let started = Instant::now();
    let mut boundaries = 0;
    let mut shorts = 0;
    for seed in include_str!("fixtures/acceptance-seeds-v1.txt").lines() {
        let mut rng = u64::from_str_radix(seed, 16).unwrap();
        let mut l = setup();
        let mut n = 10;
        let mut users = [RefBook {
            cash: 100,
            ..RefBook::default()
        }; 2];
        let mut native = RefBook {
            cash: 230,
            ..RefBook::default()
        };
        for turn in 0..64 {
            let u = (random(&mut rng) % 2) as usize;
            let delta = (random(&mut rng) % 7) as i64 - 3;
            let delta = if delta == 0 { 1 } else { delta };
            let price = 20 + random(&mut rng) % 231;
            let fee = (random(&mut rng) % 5) as i128 - 1;
            let e = execution(n, u as u8 + 1, delta, price, fee);
            step(&mut l, &e);
            n += 1;
            users[u].fill(delta, price.into(), fee);
            native.fill(delta, price.into(), fee);
            // Same identity with a changed economic payload is contained, not applied.
            let mut conflicting = e.clone();
            if let Change::Economics(EconomicChange::Execution { fee, .. }) =
                &mut conflicting.change
            {
                *fee = fee.checked_add(cash(1)).unwrap();
            }
            let contained = l.ingest(&conflicting, n).unwrap();
            assert_eq!(
                contained.disposition,
                Disposition::Rejected(LedgerError::Conflict)
            );
            assert_eq!(contained.state.venue(), l.venue());
            assert_eq!(
                contained
                    .state
                    .book(Owner::Customer(user(u as u8 + 1)))
                    .unwrap(),
                l.book(Owner::Customer(user(u as u8 + 1))).unwrap()
            );
            l = contained.state; // Retain conflicts; later actual losses still post.
            if turn % 8 == 0 {
                let cut = n;
                let rate = (random(&mut rng) % 5) as i128 - 2;
                let version = l.version();
                step(
                    &mut l,
                    &event(
                        n,
                        Change::Economics(EconomicChange::FundingBoundary {
                            market: q(0).unit(),
                            expected_version: version,
                        }),
                    ),
                );
                n += 1;
                let payments = users.map(|b| i128::from(b.quantity) * rate);
                let native_payment = i128::from(native.quantity) * rate;
                step(
                    &mut l,
                    &event(
                        n,
                        Change::Economics(EconomicChange::FundingInputs {
                            boundary: key(cut),
                            rate: Some(FundingRate {
                                numerator: cash(rate),
                                denominator: 1,
                                rounding: Rounding::TowardZero,
                            }),
                            native: Some(cash(native_payment)),
                        }),
                    ),
                );
                n += 1;
                for i in 0..2 {
                    users[i].funding += payments[i];
                }
                native.funding += native_payment;
                check(&l, users, native, price);
                boundaries += 1;
                step(
                    &mut l,
                    &event(
                        n,
                        Change::Economics(EconomicChange::FundingSettlement {
                            boundary: key(cut),
                            native: cash(native_payment),
                        }),
                    ),
                );
                n += 1;
                for b in &mut users {
                    b.cash += b.funding;
                    b.funding = 0;
                }
                native.cash += native.funding;
                native.funding = 0;
            }
            check(&l, users, native, price);
            boundaries += 1;
            shorts += usize::from(l.diagnostics(&[p(price)]).unwrap().shortfall.atoms() > 0);
        }
    }
    assert_eq!(boundaries, 576);
    assert!(
        shorts > 0,
        "reverse-stress paths must reach shortfalls, not cap losses at capital"
    );
    eprintln!(
        "P22 differential: {boundaries} cuts, {shorts} shortfall cuts, {} ms",
        started.elapsed().as_millis()
    );
}

#[test]
fn reverse_stress_finds_first_net_flat_positive_claim_shortfall_without_erasing_debt() {
    for lots in 1_i64..=4 {
        let l = setup()
            .apply(&execution(10, 1, lots, 100, 0))
            .unwrap()
            .apply(&execution(11, 2, -lots, 100, 0))
            .unwrap();
        let first = (100_u64..=400)
            .find(|mark| l.diagnostics(&[p(*mark)]).unwrap().shortfall.atoms() > 0)
            .unwrap();
        assert_eq!(first, 101 + 130 / lots as u64);
        assert_eq!(l.venue().positions()[0].quantity().lots(), 0);
        let d = l.diagnostics(&[p(first)]).unwrap();
        assert!(d.customer_deficits.atoms() > 30);
        assert_eq!(d.net_assets.atoms(), 230);
        assert_eq!(
            l.book(Owner::Customer(user(2))).unwrap().positions()[0]
                .quantity()
                .lots(),
            -lots
        );
        eprintln!(
            "P22 counterexample: {lots} opposing lots; first shortfall at mark {first}; deficit {}",
            d.customer_deficits.atoms()
        );
    }
}
