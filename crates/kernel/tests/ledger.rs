//! Synthetic production-kernel conformance. No ignored research or native endpoints.
use cinder_kernel::{Error, amounts::*, identity::*, ledger::*, position::*};

fn id(n: u8) -> AccountId {
    AccountId::new([n; 32]).unwrap()
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
    let unit = MarketUnit {
        market: MarketId::new([7; 32]).unwrap(),
        precision: PrecisionVersion::new(1).unwrap(),
        quote,
    };
    Config {
        domain,
        policy: PolicyVersion::new(1).unwrap(),
        quote,
        venue,
        venue_account,
        sources: vec![
            Source {
                scope,
                location: Location::Venue,
            },
            Source {
                scope: EventScope {
                    venue: VenueId::new([8; 32]).unwrap(),
                    account: VenueAccountId::new([9; 32]).unwrap(),
                    namespace: NamespaceId::new([10; 32]).unwrap(),
                    ..scope
                },
                location: Location::Vault,
            },
        ],
        markets: vec![Market::new(unit, 1, 1).unwrap()],
        customers: vec![id(11), id(12)],
    }
}
fn amount(n: i128) -> QuoteAtoms {
    QuoteAtoms::new(config().quote, n)
}
fn q(n: i64) -> QuantityLots {
    QuantityLots::new(config().markets[0].unit(), n)
}
fn p(n: u64) -> PriceTicks {
    PriceTicks::new(config().markets[0].unit(), n).unwrap()
}
fn attempt(customer: AccountId, n: u8) -> AttemptKey {
    AttemptKey {
        request: RequestKey {
            domain: config().domain,
            account: customer,
            request: RequestId::new([n; 32]).unwrap(),
        },
        attempt: AttemptId::new([n; 32]).unwrap(),
    }
}
fn event(n: u64, location: Location, change: Change) -> Event {
    Event {
        key: RecordKey::Economic(EventKey {
            scope: config()
                .sources
                .iter()
                .find(|s| s.location == location)
                .unwrap()
                .scope,
            event: EconomicEventId::new(&n.to_be_bytes()).unwrap(),
            leg: 0,
        }),
        policy: config().policy,
        change,
    }
}
fn bind(state: &Ledger, a: AttemptKey, side: Side) -> Ledger {
    state
        .apply(&Event {
            key: RecordKey::Attempt(a),
            policy: config().policy,
            change: Change::BindExecution {
                market: q(0).unit(),
                side,
            },
        })
        .unwrap()
}
fn receipt(state: &Ledger, n: u64, owner: Owner, location: Location, value: i128) -> Ledger {
    state
        .apply(&event(
            n,
            location,
            Change::Receipt {
                owner,
                location,
                amount: amount(value),
            },
        ))
        .unwrap()
}
fn fill(state: &Ledger, n: u64, target: FillTarget, lots: i64, price: u64) -> Ledger {
    state
        .apply(&event(
            n,
            Location::Venue,
            Change::Fill {
                target,
                quantity: q(lots),
                price: p(price),
            },
        ))
        .unwrap()
}
fn funded() -> Ledger {
    let s = Ledger::new(config()).unwrap();
    let s = receipt(&s, 1, Owner::Customer(id(11)), Location::Venue, 100);
    let s = receipt(&s, 2, Owner::Customer(id(12)), Location::Venue, 100);
    receipt(&s, 3, Owner::House, Location::Venue, 30)
}

#[test]
fn v01_asset_moves_via_transit_without_minting_another_claim() {
    let s = receipt(
        &Ledger::new(config()).unwrap(),
        1,
        Owner::Customer(id(11)),
        Location::Vault,
        100,
    );
    let debit = event(
        2,
        Location::Vault,
        Change::TransferDebit {
            source: Location::Vault,
            destination: Location::Venue,
            amount: amount(60),
        },
    );
    let RecordKey::Economic(key) = debit.key.clone() else {
        unreachable!()
    };
    let moving = s.apply(&debit).unwrap();
    assert_eq!(
        (
            moving.vault().atoms(),
            moving.in_transit().unwrap().atoms(),
            moving.venue().cash().atoms()
        ),
        (40, 60, 0)
    );
    let arrival = event(
        3,
        Location::Venue,
        Change::TransferArrival { debit: key.clone() },
    );
    let arrived = moving.apply(&arrival).unwrap();
    assert_eq!(
        (
            arrived.vault().atoms(),
            arrived.in_transit().unwrap().atoms(),
            arrived.venue().cash().atoms()
        ),
        (40, 0, 60)
    );
    for state in [&s, &moving, &arrived] {
        assert_eq!(
            state.equity(Owner::Customer(id(11)), &[p(100)]).unwrap(),
            amount(100)
        );
        assert_eq!(
            state.diagnostics(&[p(100)]).unwrap().net_assets,
            amount(100)
        );
    }
    assert_eq!(moving.apply(&debit).unwrap(), moving);
    assert_eq!(arrived.apply(&arrival).unwrap(), arrived);
    assert_eq!(
        arrived.apply(&event(
            4,
            Location::Venue,
            Change::TransferArrival { debit: key }
        )),
        Err(LedgerError::Transfer)
    );
    assert_eq!(s.apply(&arrival), Err(LedgerError::Transfer));
    // Return is a location change too, never a new deposit.
    let returning = event(
        5,
        Location::Venue,
        Change::TransferDebit {
            source: Location::Venue,
            destination: Location::Vault,
            amount: amount(60),
        },
    );
    let RecordKey::Economic(key) = returning.key.clone() else {
        unreachable!()
    };
    let returned = arrived
        .apply(&returning)
        .unwrap()
        .apply(&event(
            6,
            Location::Vault,
            Change::TransferArrival { debit: key },
        ))
        .unwrap();
    assert_eq!(returned.vault(), amount(100));
    assert_eq!(returned.venue().cash(), amount(0));
    assert_eq!(
        returned.diagnostics(&[p(100)]).unwrap().customer_claims,
        amount(100)
    );
}

#[test]
fn v03_net_flat_default_is_underbacked_even_with_a_valid_ledger() {
    let a = attempt(id(11), 20);
    let b = attempt(id(12), 21);
    let s = bind(&bind(&funded(), a, Side::Buy), b, Side::Sell);
    let s = fill(
        &fill(&s, 4, FillTarget::Customer(a), 1, 100),
        5,
        FillTarget::Customer(b),
        -1,
        100,
    );
    s.check_bridge().unwrap();
    assert_eq!(s.venue().positions()[0].quantity().lots(), 0);
    assert_eq!(
        s.equity(Owner::Customer(id(11)), &[p(250)]).unwrap(),
        amount(250)
    );
    assert_eq!(
        s.equity(Owner::Customer(id(12)), &[p(250)]).unwrap(),
        amount(-50)
    );
    let d = s.diagnostics(&[p(250)]).unwrap();
    assert_eq!(
        (
            d.net_assets.atoms(),
            d.customer_claims.atoms(),
            d.customer_deficits.atoms(),
            d.house_equity.atoms(),
            d.backing_margin.atoms(),
            d.shortfall.atoms()
        ),
        (230, 250, 50, 30, -20, 20)
    );
    // Actual losing close still records despite underbacking, without erasing debt.
    let close = attempt(id(12), 22);
    let s = fill(
        &bind(&s, close, Side::Buy),
        6,
        FillTarget::Customer(close),
        1,
        250,
    );
    assert_eq!(s.book(Owner::Customer(id(12))).unwrap().cash(), amount(-50));
    assert_eq!(s.diagnostics(&[p(250)]).unwrap().shortfall, amount(20));
}

#[test]
fn individual_winner_pool_loser_and_individual_loser_pool_winner() {
    for (a_qty, b_qty, alice_pnl, bob_pnl, native_pnl) in
        [(1, -3, 20, -60, -40), (-1, 3, -20, 60, 40)]
    {
        let a = attempt(id(11), 20);
        let b = attempt(id(12), 21);
        let s = bind(
            &bind(&funded(), a, if a_qty > 0 { Side::Buy } else { Side::Sell }),
            b,
            if b_qty > 0 { Side::Buy } else { Side::Sell },
        );
        let s = fill(
            &fill(&s, 4, FillTarget::Customer(a), a_qty, 100),
            5,
            FillTarget::Customer(b),
            b_qty,
            100,
        );
        assert_eq!(
            s.equity(Owner::Customer(id(11)), &[p(120)]).unwrap(),
            amount(100 + alice_pnl)
        );
        assert_eq!(
            s.equity(Owner::Customer(id(12)), &[p(120)]).unwrap(),
            amount(100 + bob_pnl)
        );
        assert_eq!(
            s.diagnostics(&[p(120)]).unwrap().net_assets,
            amount(230 + native_pnl)
        );
        let close = attempt(id(11), 22);
        let s = fill(
            &bind(&s, close, if a_qty > 0 { Side::Sell } else { Side::Buy }),
            6,
            FillTarget::Customer(close),
            -a_qty,
            120,
        );
        assert_eq!(
            s.book(Owner::Customer(id(11))).unwrap().cash(),
            amount(100 + alice_pnl)
        );
        assert_eq!(
            s.diagnostics(&[p(120)]).unwrap().net_assets,
            amount(230 + native_pnl)
        );
    }
}

#[test]
fn native_realization_and_private_basis_can_differ_without_a_balancing_plug() {
    let a = attempt(id(11), 20);
    let b = attempt(id(12), 21);
    let s = bind(&bind(&funded(), a, Side::Buy), b, Side::Sell);
    let s = fill(
        &fill(&s, 4, FillTarget::Customer(a), 1, 100),
        5,
        FillTarget::Customer(b),
        -1,
        120,
    );
    assert_eq!(s.venue().cash(), amount(250)); // Native realizes 20 on net close.
    assert_eq!(s.venue().positions()[0], Position::flat(q(0).unit()));
    assert_eq!(s.book(Owner::Customer(id(11))).unwrap().cash(), amount(100));
    assert_eq!(s.book(Owner::Customer(id(12))).unwrap().cash(), amount(100));
    assert_eq!(
        s.equity(Owner::Customer(id(11)), &[p(120)]).unwrap(),
        amount(120)
    );
    assert_eq!(s.diagnostics(&[p(120)]).unwrap().house_equity, amount(30));
    s.check_bridge().unwrap();
}

#[test]
fn wrong_owner_route_is_not_rescued_by_equal_aggregate_economics() {
    let a = attempt(id(11), 20);
    let s = bind(&funded(), a, Side::Buy);
    let forged = AttemptKey {
        request: RequestKey {
            account: id(12),
            ..a.request
        },
        ..a
    };
    let bad = event(
        4,
        Location::Venue,
        Change::Fill {
            target: FillTarget::Customer(forged),
            quantity: q(1),
            price: p(100),
        },
    );
    let unchanged = s.clone();
    assert_eq!(s.apply(&bad), Err(LedgerError::UnknownIdentity));
    assert_eq!(s, unchanged);
    let good = event(
        4,
        Location::Venue,
        Change::Fill {
            target: FillTarget::Customer(a),
            quantity: q(1),
            price: p(100),
        },
    );
    let accepted = s.apply(&good).unwrap();
    assert_eq!(
        accepted.book(Owner::Customer(id(12))).unwrap().positions()[0].quantity(),
        q(0)
    );
    assert_eq!(accepted.apply(&bad), Err(LedgerError::Conflict));
    let wrong_side = event(
        5,
        Location::Venue,
        Change::Fill {
            target: FillTarget::Customer(a),
            quantity: q(-1),
            price: p(100),
        },
    );
    assert_eq!(s.apply(&wrong_side), Err(LedgerError::Attribution));
    // This only proves the stored route, not authenticity of adapter-supplied causality.
}

#[test]
fn suspense_preserves_unattributed_liabilities_and_exposure_without_house_profit() {
    let s = receipt(&funded(), 4, Owner::Suspense, Location::Vault, 50);
    let d = s.diagnostics(&[p(100)]).unwrap();
    assert_eq!(
        (
            d.net_assets.atoms(),
            d.customer_claims.atoms(),
            d.suspense_equity.atoms(),
            d.backing_margin.atoms()
        ),
        (280, 200, 50, 30)
    );
    let s = fill(&s, 5, FillTarget::Unattributed, 1, 100);
    let d = s.diagnostics(&[p(10)]).unwrap();
    assert_eq!(
        (
            d.suspense_equity.atoms(),
            d.house_equity.atoms(),
            d.backing_margin.atoms(),
            d.unresolved_events
        ),
        (-40, 30, -10, 2)
    );
    assert_eq!(d.shortfall, amount(10));
    let s = fill(&s, 6, FillTarget::House, -1, 10);
    s.check_bridge().unwrap();
    assert_eq!(
        s.book(Owner::House).unwrap().positions()[0].quantity(),
        q(-1)
    );
    assert_eq!(s.diagnostics(&[p(100)]).unwrap().unresolved_events, 2);
}

#[test]
fn replay_conflicts_and_errors_never_partially_commit() {
    let s = funded();
    let e = event(
        4,
        Location::Vault,
        Change::Receipt {
            owner: Owner::Customer(id(11)),
            location: Location::Vault,
            amount: amount(10),
        },
    );
    let next = s.apply(&e).unwrap();
    assert_eq!(next.version(), s.version() + 1);
    assert_eq!(next.apply(&e).unwrap(), next);
    let mut changed = e.clone();
    changed.change = Change::Receipt {
        owner: Owner::Customer(id(12)),
        location: Location::Vault,
        amount: amount(10),
    };
    assert_eq!(next.apply(&changed), Err(LedgerError::Conflict));
    changed.policy = PolicyVersion::new(2).unwrap();
    assert_eq!(
        s.apply(&changed),
        Err(LedgerError::Primitive(Error::UnknownVersion))
    );
    let before = next.clone();
    for amount in [0, -1, i128::MAX] {
        let bad = event(
            5,
            Location::Vault,
            Change::Receipt {
                owner: Owner::Customer(id(11)),
                location: Location::Vault,
                amount: QuoteAtoms::new(config().quote, amount),
            },
        );
        assert!(next.apply(&bad).is_err());
        assert_eq!(next, before);
    }
    let bad = event(
        5,
        Location::Vault,
        Change::TransferDebit {
            source: Location::Vault,
            destination: Location::Venue,
            amount: amount(11),
        },
    );
    assert_eq!(next.apply(&bad), Err(LedgerError::PhysicalShortfall));
    assert_eq!(next, before);
    let replayed = next
        .events()
        .iter()
        .try_fold(Ledger::new(config()).unwrap(), |state, event| {
            state.apply(event)
        })
        .unwrap();
    assert_eq!(replayed, next); // In-memory deterministic replay, not crash durability.
}

#[test]
fn source_domain_location_and_unknown_identity_are_rejected() {
    let s = funded();
    let good = event(
        4,
        Location::Vault,
        Change::Receipt {
            owner: Owner::Customer(id(11)),
            location: Location::Vault,
            amount: amount(1),
        },
    );
    for field in 0..5 {
        let mut bad = good.clone();
        let RecordKey::Economic(ref mut key) = bad.key else {
            unreachable!()
        };
        match field {
            0 => key.scope.domain.deployment = DeploymentId::new([99; 32]).unwrap(),
            1 => key.scope.namespace = NamespaceId::new([99; 32]).unwrap(),
            2 => key.scope.account = VenueAccountId::new([99; 32]).unwrap(),
            3 => key.scope.venue = VenueId::new([99; 32]).unwrap(),
            _ => key.scope = config().sources[0].scope,
        }
        assert!(s.apply(&bad).is_err());
    }
    assert_eq!(
        s.apply(&event(
            5,
            Location::Vault,
            Change::Receipt {
                owner: Owner::Customer(id(99)),
                location: Location::Vault,
                amount: amount(1)
            }
        )),
        Err(LedgerError::UnknownIdentity)
    );
    let a = attempt(id(11), 20);
    let s = bind(&s, a, Side::Buy);
    let wrong_market = MarketUnit {
        precision: PrecisionVersion::new(2).unwrap(),
        ..q(0).unit()
    };
    assert!(
        s.apply(&event(
            6,
            Location::Venue,
            Change::Fill {
                target: FillTarget::Customer(a),
                quantity: QuantityLots::new(wrong_market, 1),
                price: p(100)
            }
        ))
        .is_err()
    );
}

#[test]
fn multiple_markets_need_unique_complete_versioned_marks_and_quote_units() {
    let mut c = config();
    let second = MarketUnit {
        market: MarketId::new([44; 32]).unwrap(),
        ..c.markets[0].unit()
    };
    c.markets.push(Market::new(second, 2, 1).unwrap());
    let s = receipt(
        &Ledger::new(c.clone()).unwrap(),
        1,
        Owner::House,
        Location::Venue,
        10,
    );
    let s = s
        .apply(&event(
            2,
            Location::Venue,
            Change::Fill {
                target: FillTarget::House,
                quantity: QuantityLots::new(second, 2),
                price: PriceTicks::new(second, 50).unwrap(),
            },
        ))
        .unwrap();
    assert_eq!(
        s.diagnostics(&[PriceTicks::new(second, 55).unwrap(), p(100)])
            .unwrap()
            .net_assets,
        amount(30)
    );
    assert!(s.diagnostics(&[p(100)]).is_err());
    assert!(s.diagnostics(&[p(100), p(100)]).is_err());
    let original = config();
    let mut bad = original.clone();
    bad.customers.push(id(11));
    assert_eq!(Ledger::new(bad), Err(LedgerError::Configuration));
    let mut bad = original.clone();
    bad.markets.push(original.markets[0]);
    assert_eq!(Ledger::new(bad), Err(LedgerError::Configuration));
    let mut bad = original.clone();
    bad.sources.push(original.sources[0]);
    assert_eq!(Ledger::new(bad), Err(LedgerError::Configuration));
    let mut bad = original;
    bad.quote.asset = AssetId::new([99; 32]).unwrap();
    assert!(Ledger::new(bad).is_err());
}

#[test]
fn price_conversion_is_explicit_exact_and_extreme_values_fail_without_wrapping() {
    let market = Market::new(q(0).unit(), 3, 2).unwrap();
    assert_eq!(market.notional(q(2), p(5)).unwrap(), amount(15));
    assert_eq!(market.notional(q(1), p(5)), Err(Error::Inexact));
    let negative = Position::new(
        q(i64::MIN),
        BasisAtoms::new(q(0).unit(), i128::from(i64::MIN)),
    )
    .unwrap();
    let result = negative
        .fill(config().markets[0], q(i64::MAX), p(1))
        .unwrap();
    assert_eq!(result.position.quantity(), q(-1));
    assert_eq!(result.position.basis().atoms(), -1);
    assert_eq!(result.realized, amount(0));
    assert_eq!(
        negative.fill(config().markets[0], q(-1), p(1)),
        Err(Error::Overflow)
    );
    let extreme = Market::new(q(0).unit(), u64::MAX, 1).unwrap();
    assert_eq!(
        Position::flat(q(0).unit()).fill(extreme, q(i64::MAX), p(u64::MAX)),
        Err(Error::Overflow)
    );
    assert!(Position::new(q(0), BasisAtoms::new(q(0).unit(), 1)).is_err());
    assert!(Position::new(q(-1), BasisAtoms::new(q(0).unit(), 1)).is_err());
    assert_eq!(
        Position::flat(q(0).unit()).fill(market, q(0), p(2)),
        Err(Error::InvalidSign)
    );
}

#[test]
fn failing_second_fill_leg_does_not_commit_the_private_first_leg() {
    let s = fill(
        &Ledger::new(config()).unwrap(),
        1,
        FillTarget::House,
        i64::MAX,
        1,
    );
    let a = attempt(id(11), 20);
    let s = bind(&s, a, Side::Buy);
    let before = s.clone();
    let incoming = event(
        2,
        Location::Venue,
        Change::Fill {
            target: FillTarget::Customer(a),
            quantity: q(1),
            price: p(1),
        },
    );
    assert_eq!(
        s.apply(&incoming),
        Err(LedgerError::Primitive(Error::Overflow))
    );
    assert_eq!(s, before);
    assert_eq!(
        s.book(Owner::Customer(id(11))).unwrap().positions()[0].quantity(),
        q(0)
    );
    assert!(!s.events().iter().any(|e| e.key == incoming.key));
}

#[test]
fn reversal_splits_exact_whole_fill_and_retains_residue_in_new_basis() {
    let market = Market::new(q(0).unit(), 3, 2).unwrap();
    for sign in [-1, 1] {
        let position = Position::flat(q(0).unit())
            .fill(market, q(sign), p(2))
            .unwrap()
            .position;
        let crossed = position.fill(market, q(-2 * sign), p(5)).unwrap();
        assert_eq!(crossed.position.quantity(), q(-sign));
        assert_eq!(crossed.realized, amount(4 * i128::from(sign)));
        assert_eq!(crossed.position.basis().atoms(), -8 * i128::from(sign));
        // The next full close consumes every atom, including the split residue.
        let closed = crossed.position.fill(market, q(sign), p(6)).unwrap();
        assert_eq!(closed.position, Position::flat(q(0).unit()));
        assert_eq!(closed.realized, amount(-i128::from(sign)));
        // A genuinely inexact whole fill is still rejected, never rounded.
        assert_eq!(position.fill(market, q(-sign), p(5)), Err(Error::Inexact));
    }
}

#[test]
fn rational_crossing_fill_preserves_pooled_bridge_and_replay() {
    for sign in [-1, 1] {
        let mut c = config();
        c.markets[0] = Market::new(q(0).unit(), 3, 2).unwrap();
        let s = fill(
            &Ledger::new(c.clone()).unwrap(),
            1,
            FillTarget::House,
            sign,
            2,
        );
        let route = attempt(id(11), 20);
        let s = bind(&s, route, if sign < 0 { Side::Buy } else { Side::Sell });
        let e = event(
            2,
            Location::Venue,
            Change::Fill {
                target: FillTarget::Customer(route),
                quantity: q(-2 * sign),
                price: p(5),
            },
        );
        let next = s.apply(&e).unwrap();
        assert_eq!(next.venue().cash(), amount(4 * i128::from(sign)));
        assert_eq!(
            next.venue().positions()[0].basis().atoms(),
            -8 * i128::from(sign)
        );
        assert_eq!(
            next.book(Owner::Customer(id(11))).unwrap().cash(),
            amount(0)
        );
        assert_eq!(
            next.book(Owner::Customer(id(11))).unwrap().positions()[0]
                .basis()
                .atoms(),
            -15 * i128::from(sign)
        );
        next.check_bridge().unwrap();
        assert_eq!(
            next.diagnostics(&[p(6)]).unwrap().net_assets,
            amount(3 * i128::from(sign))
        );
        assert_eq!(next.apply(&e).unwrap(), next);
        let replay = next
            .events()
            .iter()
            .try_fold(Ledger::new(c).unwrap(), |s, e| s.apply(e))
            .unwrap();
        assert_eq!(replay, next);
    }
}

#[test]
fn property_rational_fills_preserve_whole_value_at_representable_marks() {
    for (numerator, denominator) in [(1_u64, 2_u64), (2, 3), (3, 2), (5, 3)] {
        let market = Market::new(q(0).unit(), numerator, denominator).unwrap();
        for lots in -3_i64..=3 {
            for magnitude in [0, 1, 7, 13] {
                if lots == 0 && magnitude != 0 {
                    continue;
                }
                let basis = magnitude * i128::from(lots.signum());
                let before = Position::new(q(lots), BasisAtoms::new(market.unit(), basis)).unwrap();
                for delta in -6_i64..=6 {
                    if delta == 0 {
                        continue;
                    }
                    for price in 1..=6 {
                        let product = i128::from(delta) * i128::from(price) * i128::from(numerator);
                        let result = before.fill(market, q(delta), p(price));
                        if product % i128::from(denominator) != 0 {
                            assert_eq!(result, Err(Error::Inexact));
                            continue;
                        }
                        let result = result.unwrap();
                        let total = product / i128::from(denominator);
                        assert_eq!(
                            result.realized.atoms() - (result.position.basis().atoms() - basis),
                            -total
                        );
                        assert_eq!(result.position.quantity(), q(lots + delta));
                        if lots + delta == 0 {
                            assert_eq!(result.position.basis().atoms(), 0);
                        }
                        for mark in [denominator, 2 * denominator] {
                            let change = result.realized.atoms()
                                + result.position.unrealized(market, p(mark)).unwrap().atoms()
                                - before.unrealized(market, p(mark)).unwrap().atoms();
                            let expected =
                                i128::from(delta) * i128::from(mark) * i128::from(numerator)
                                    / i128::from(denominator)
                                    - total;
                            assert_eq!(change, expected);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn signed_native_cash_can_record_a_real_loss_without_inventing_negative_vault_tokens() {
    let s = fill(
        &Ledger::new(config()).unwrap(),
        1,
        FillTarget::House,
        1,
        100,
    );
    let s = fill(&s, 2, FillTarget::House, -1, 1);
    assert_eq!(s.venue().cash(), amount(-99));
    assert_eq!(s.vault(), amount(0));
    assert_eq!(s.diagnostics(&[p(1)]).unwrap().shortfall, amount(99));
    s.check_bridge().unwrap();
}

#[test]
fn property_exhaustive_position_equity_change_and_remaining_basis() {
    let market = config().markets[0];
    for lots in -8_i64..=8 {
        for basis_magnitude in 0_i128..=25 {
            if lots == 0 && basis_magnitude != 0 {
                continue;
            }
            let basis = basis_magnitude * i128::from(lots.signum());
            let position = Position::new(q(lots), BasisAtoms::new(market.unit(), basis)).unwrap();
            for delta in -10_i64..=10 {
                if delta == 0 {
                    continue;
                }
                for price in [1, 7, 13] {
                    let result = position.fill(market, q(delta), p(price)).unwrap();
                    for mark in [1, 17] {
                        let before = position.unrealized(market, p(mark)).unwrap().atoms();
                        let after = result.realized.atoms()
                            + result.position.unrealized(market, p(mark)).unwrap().atoms();
                        assert_eq!(
                            after - before,
                            i128::from(delta) * (i128::from(mark) - i128::from(price))
                        );
                    }
                    assert_eq!(result.position.quantity().lots(), lots + delta);
                    if lots + delta == 0 {
                        assert_eq!(result.position.basis().atoms(), 0);
                    }
                }
            }
        }
    }
}

#[test]
fn property_seeded_joined_ledger_matches_independent_cash_flow_oracle_at_every_prefix() {
    // Independent oracle tracks marked equity from initial cash plus each external
    // cash flow -delta*execution and current exposure*mark, not basis transitions.
    for seed in 1_u64..=12 {
        let mut s = funded();
        let routes = [
            attempt(id(11), 20),
            attempt(id(11), 21),
            attempt(id(12), 22),
            attempt(id(12), 23),
        ];
        for (i, a) in routes.iter().enumerate() {
            s = bind(&s, *a, if i % 2 == 0 { Side::Buy } else { Side::Sell });
        }
        let mut random = seed;
        let mut cash_flow = [100_i128; 2];
        let mut exposure = [0_i128; 2];
        for n in 0..128 {
            random = random
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let route = (random % 4) as usize;
            let quantity =
                (1 + ((random >> 8) % 5)) as i64 * if route.is_multiple_of(2) { 1 } else { -1 };
            let price = 1 + ((random >> 16) % 200);
            let user = route / 2;
            exposure[user] += i128::from(quantity);
            cash_flow[user] -= i128::from(quantity) * i128::from(price);
            let e = event(
                n + 10,
                Location::Venue,
                Change::Fill {
                    target: FillTarget::Customer(routes[route]),
                    quantity: q(quantity),
                    price: p(price),
                },
            );
            s = s.apply(&e).unwrap();
            assert_eq!(s.apply(&e).unwrap(), s);
            for mark in [1, 100, 250] {
                let expected = [
                    cash_flow[0] + exposure[0] * i128::from(mark),
                    cash_flow[1] + exposure[1] * i128::from(mark),
                ];
                let d = s.diagnostics(&[p(mark)]).unwrap();
                for (i, value) in expected.iter().enumerate() {
                    assert_eq!(
                        s.equity(Owner::Customer(id(11 + i as u8)), &[p(mark)])
                            .unwrap(),
                        amount(*value)
                    );
                }
                assert_eq!(d.net_assets, amount(expected[0] + expected[1] + 30));
                assert_eq!(
                    d.customer_claims,
                    amount(expected.iter().map(|v| (*v).max(0)).sum())
                );
                assert_eq!(
                    d.customer_deficits,
                    amount(expected.iter().map(|v| (-v).max(0)).sum())
                );
                assert_eq!(d.backing_margin.atoms(), 30 - d.customer_deficits.atoms());
            }
        }
        let replayed = s
            .events()
            .iter()
            .try_fold(Ledger::new(config()).unwrap(), |state, e| state.apply(e))
            .unwrap();
        assert_eq!(replayed, s);
    }
}
