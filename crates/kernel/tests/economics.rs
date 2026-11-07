//! P04 synthetic joined-ledger tests. No native API, external clock or private fixtures.
use cinder_kernel::{
    Error,
    amounts::*,
    identity::*,
    ledger::{economics::*, evidence::*, *},
    math::Rounding,
    position::*,
};

fn user(n: u8) -> AccountId {
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
        sources: vec![Source {
            scope,
            location: Location::Venue,
        }],
        markets: vec![Market::new(unit, 1, 1).unwrap()],
        customers: vec![user(1), user(2)],
    }
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
fn econ(n: u64, change: EconomicChange) -> Event {
    event(n, Change::Economics(change))
}
fn put(s: &Ledger, e: Event) -> Ledger {
    let result = s.ingest(&e, s.version() + 1).unwrap();
    assert_eq!(result.disposition, Disposition::Applied, "{e:?}");
    result.state
}
fn route(u: u8, buy: bool) -> AttemptKey {
    let n = u * 2 + u8::from(buy);
    AttemptKey {
        request: RequestKey {
            domain: config().domain,
            account: user(u),
            request: RequestId::new([n; 32]).unwrap(),
        },
        attempt: AttemptId::new([n; 32]).unwrap(),
    }
}
fn execution(n: u64, u: u8, lots: i64, price: u64, fee: i128, pnl: Option<NativePnl>) -> Event {
    econ(
        n,
        EconomicChange::Execution {
            target: FillTarget::Customer(route(u, lots > 0)),
            quantity: q(lots),
            price: p(price),
            fee: cash(fee),
            pnl,
        },
    )
}
fn setup(a: i64, b: i64) -> Ledger {
    let mut s = Ledger::new(config()).unwrap();
    for u in [1, 2] {
        s = put(
            &s,
            event(
                u as u64,
                Change::Receipt {
                    owner: Owner::Customer(user(u)),
                    location: Location::Venue,
                    amount: cash(100),
                },
            ),
        );
        for buy in [false, true] {
            s = put(
                &s,
                Event {
                    key: RecordKey::Attempt(route(u, buy)),
                    policy: config().policy,
                    change: Change::BindExecution {
                        market: q(0).unit(),
                        side: if buy { Side::Buy } else { Side::Sell },
                    },
                },
            );
        }
    }
    if a != 0 {
        s = put(&s, execution(3, 1, a, 100, 0, None));
    }
    if b != 0 {
        s = put(&s, execution(4, 2, b, 100, 0, None));
    }
    s
}
fn boundary(s: &Ledger, n: u64) -> Ledger {
    put(
        s,
        econ(
            n,
            EconomicChange::FundingBoundary {
                market: q(0).unit(),
                expected_version: s.version(),
            },
        ),
    )
}
fn rate(n: i128, d: u64, rounding: Rounding) -> FundingRate {
    FundingRate {
        numerator: cash(n),
        denominator: d,
        rounding,
    }
}
fn inputs(s: &Ledger, n: u64, b: u64, r: Option<FundingRate>, native: Option<i128>) -> Ledger {
    put(
        s,
        econ(
            n,
            EconomicChange::FundingInputs {
                boundary: key(b),
                rate: r,
                native: native.map(cash),
            },
        ),
    )
}
fn settle(s: &Ledger, n: u64, b: u64, value: i128) -> Ledger {
    put(
        s,
        econ(
            n,
            EconomicChange::FundingSettlement {
                boundary: key(b),
                native: cash(value),
            },
        ),
    )
}
fn policy() -> EvidencePolicy {
    EvidencePolicy {
        max_mark_age: 10,
        max_issue_age: 20,
        max_check_age: 50,
    }
}
fn check(s: &Ledger) -> NativeCheck {
    NativeCheck {
        expected_version: s.version(),
        cash: Some(s.venue().cash()),
        funding: Some(s.venue().funding()),
        positions: Some(s.venue().positions().to_vec()),
        complete: true,
        resolves: vec![],
    }
}
fn marks(now: u64) -> Vec<MarkObservation> {
    vec![MarkObservation {
        price: p(100),
        evidence: key(999),
        observed_at: now,
        valid_until: now + 10,
        qualified: true,
    }]
}
fn issue(s: &Ledger, subject: u64, kind: IssueKind) -> &Issue {
    s.issues()
        .iter()
        .find(|i| i.subject == RecordKey::Economic(key(subject)) && i.kind == kind)
        .unwrap()
}

#[test]
fn v04_gross_funding_on_net_flat_pool_and_settlement_once() {
    let s = boundary(&setup(10, -10), 10);
    let s = inputs(&s, 11, 10, Some(rate(-1, 1, Rounding::Exact)), Some(0));
    assert_eq!(
        s.book(Owner::Customer(user(1))).unwrap().funding(),
        cash(-10)
    );
    assert_eq!(
        s.book(Owner::Customer(user(2))).unwrap().funding(),
        cash(10)
    );
    assert_eq!(s.venue().funding(), cash(0));
    let before = s.diagnostics(&[p(100)]).unwrap();
    let s = settle(&s, 12, 10, 0);
    assert_eq!(s.book(Owner::Customer(user(1))).unwrap().cash(), cash(90));
    assert_eq!(s.book(Owner::Customer(user(2))).unwrap().cash(), cash(110));
    assert_eq!(s.book(Owner::Customer(user(2))).unwrap().funding(), cash(0));
    assert_eq!(s.diagnostics(&[p(100)]).unwrap(), before);
    let duplicate = s
        .ingest(
            &econ(
                12,
                EconomicChange::FundingSettlement {
                    boundary: key(10),
                    native: cash(0),
                },
            ),
            20,
        )
        .unwrap();
    assert_eq!(duplicate.disposition, Disposition::Duplicate);
    assert_eq!(duplicate.state.venue(), s.venue());
    let second = s
        .ingest(
            &econ(
                13,
                EconomicChange::FundingSettlement {
                    boundary: key(10),
                    native: cash(0),
                },
            ),
            20,
        )
        .unwrap();
    assert_eq!(
        second.disposition,
        Disposition::Rejected(LedgerError::FundingState)
    );
    assert_eq!(
        second.state.book(Owner::Customer(user(1))).unwrap().cash(),
        cash(90)
    );
}

#[test]
fn delayed_rate_uses_boundary_inventory_across_partial_close_entry_and_reversal() {
    let s = boundary(&setup(3, 0), 10);
    let s = put(&s, execution(11, 1, -1, 110, 0, None));
    let s = put(&s, execution(12, 2, 2, 110, 0, None));
    let s = put(&s, execution(13, 1, -4, 120, 0, None)); // Alice now short 2.
    let s = inputs(&s, 14, 10, Some(rate(-2, 1, Rounding::Exact)), Some(-6));
    assert_eq!(
        s.book(Owner::Customer(user(1))).unwrap().funding(),
        cash(-6)
    );
    assert_eq!(s.book(Owner::Customer(user(2))).unwrap().funding(), cash(0));
    let s = boundary(&s, 15);
    let s = inputs(&s, 16, 15, Some(rate(-2, 1, Rounding::Exact)), Some(0));
    assert_eq!(
        s.book(Owner::Customer(user(1))).unwrap().funding(),
        cash(-2)
    );
    assert_eq!(
        s.book(Owner::Customer(user(2))).unwrap().funding(),
        cash(-4)
    );
    let s = settle(&s, 17, 10, -6); // Does not sweep the second boundary.
    assert_eq!(s.book(Owner::Customer(user(1))).unwrap().funding(), cash(4));
    assert_eq!(
        s.book(Owner::Customer(user(2))).unwrap().funding(),
        cash(-4)
    );
    assert_eq!(s.venue().funding(), cash(0));
    s.check_bridge().unwrap();
}

#[test]
fn unknown_rate_known_native_payment_is_suspense_until_private_evidence_arrives() {
    let s = boundary(&setup(3, 0), 10);
    let s = inputs(&s, 11, 10, None, Some(-6));
    assert_eq!(s.venue().funding(), cash(-6));
    assert_eq!(s.book(Owner::Suspense).unwrap().funding(), cash(-6));
    assert_eq!(s.book(Owner::Customer(user(1))).unwrap().funding(), cash(0));
    assert!(issue(&s, 10, IssueKind::FundingInputs).open);
    let s = settle(&s, 12, 10, -6);
    assert_eq!(s.book(Owner::Suspense).unwrap().cash(), cash(-6));
    let s = inputs(&s, 13, 10, Some(rate(-2, 1, Rounding::Exact)), None);
    assert_eq!(s.book(Owner::Customer(user(1))).unwrap().cash(), cash(94));
    assert_eq!(s.book(Owner::Suspense).unwrap().cash(), cash(0));
    assert_eq!(s.venue().cash(), cash(194));
    assert!(!issue(&s, 10, IssueKind::FundingInputs).open);
    s.check_bridge().unwrap();
}

#[test]
fn settlement_can_arrive_before_rate_and_recognition_without_double_funding() {
    let s = boundary(&setup(2, -1), 10);
    let s = settle(&s, 11, 10, -3);
    assert_eq!(s.book(Owner::Suspense).unwrap().cash(), cash(-3));
    let s = inputs(&s, 12, 10, Some(rate(-3, 1, Rounding::Exact)), Some(-3));
    assert_eq!(s.book(Owner::Customer(user(1))).unwrap().cash(), cash(94));
    assert_eq!(s.book(Owner::Customer(user(2))).unwrap().cash(), cash(103));
    assert_eq!(s.venue().cash(), cash(197));
    assert_eq!(s.venue().funding(), cash(0));
}

#[test]
fn unknown_native_amount_is_not_zero_even_if_net_position_is_zero() {
    let s = boundary(&setup(1, -1), 10);
    let s = inputs(&s, 11, 10, Some(rate(-1, 1, Rounding::Exact)), None);
    assert!(issue(&s, 10, IssueKind::FundingInputs).open);
    assert_eq!(
        s.funding_rounding_residual(&key(10)),
        Err(LedgerError::Evidence)
    );
    let s = put(&s, event(12, Change::Reconcile(check(&s))));
    assert_eq!(
        s.qualified_diagnostics(&marks(s.version()), s.version(), policy()),
        Err(LedgerError::Restricted)
    );
    let s = inputs(&s, 13, 10, None, Some(0));
    assert_eq!(
        s.book(Owner::Customer(user(1))).unwrap().funding(),
        cash(-1)
    );
    assert_eq!(s.book(Owner::Customer(user(2))).unwrap().funding(), cash(1));
}

#[test]
fn fee_inclusive_pnl_and_rebates_are_not_charged_twice() {
    let s = put(
        &setup(0, 0),
        execution(10, 1, 2, 100, 3, Some(NativePnl::NetOfFee(cash(-3)))),
    );
    assert_eq!(s.book(Owner::Customer(user(1))).unwrap().cash(), cash(97));
    let s = put(
        &s,
        execution(11, 1, -1, 120, 2, Some(NativePnl::Gross(cash(20)))),
    );
    assert_eq!(s.book(Owner::Customer(user(1))).unwrap().cash(), cash(115));
    let s = put(
        &s,
        execution(12, 1, -1, 120, -1, Some(NativePnl::NetOfFee(cash(21)))),
    );
    assert_eq!(s.book(Owner::Customer(user(1))).unwrap().cash(), cash(136));
    assert_eq!(s.venue().cash(), cash(236));
    assert!(s.issues().iter().all(|i| !i.open));
    let request = RequestKey {
        domain: config().domain,
        account: user(1),
        request: RequestId::new([99; 32]).unwrap(),
    };
    let s = put(
        &s,
        Event {
            key: RecordKey::Request(request),
            policy: config().policy,
            change: Change::Economics(EconomicChange::BrokerFee {
                customer: user(1),
                amount: cash(4),
            }),
        },
    );
    assert_eq!(s.book(Owner::House).unwrap().cash(), cash(4));
    assert_eq!(s.book(Owner::Customer(user(1))).unwrap().cash(), cash(132));
    assert_eq!(s.venue().cash(), cash(236));
}

#[test]
fn native_pnl_is_compared_with_native_not_private_basis() {
    let s = put(
        &setup(1, 0),
        execution(10, 2, -1, 120, 1, Some(NativePnl::NetOfFee(cash(19)))),
    );
    assert_eq!(s.venue().cash(), cash(219));
    assert_eq!(s.book(Owner::Customer(user(2))).unwrap().cash(), cash(99));
    assert_eq!(s.book(Owner::House).unwrap().cash(), cash(0));
    assert!(s.issues().is_empty());
    s.check_bridge().unwrap();
}

#[test]
fn unexpected_pnl_stays_named_and_aged_until_source_correction() {
    let s = put(
        &setup(1, 0),
        execution(10, 1, -1, 120, 2, Some(NativePnl::NetOfFee(cash(17)))),
    );
    assert_eq!(s.book(Owner::Customer(user(1))).unwrap().cash(), cash(118));
    assert_eq!(s.venue().cash(), cash(217));
    assert_eq!(s.book(Owner::Suspense).unwrap().cash(), cash(-1));
    assert_eq!(
        issue(&s, 10, IssueKind::NativePnl).difference,
        Some(cash(-1))
    );
    let first = issue(&s, 10, IssueKind::NativePnl).first_seen;
    let s = s
        .ingest(
            &execution(10, 1, -1, 120, 2, Some(NativePnl::NetOfFee(cash(17)))),
            first + 10,
        )
        .unwrap()
        .state;
    assert_eq!(issue(&s, 10, IssueKind::NativePnl).first_seen, first);
    assert_eq!(
        s.evidence_mode(first + 20, policy()).unwrap(),
        EvidenceMode::Frozen
    );
    let s = s
        .ingest(
            &econ(
                11,
                EconomicChange::CorrectExecutionPnl {
                    execution: key(10),
                    pnl: NativePnl::NetOfFee(cash(18)),
                },
            ),
            first + 21,
        )
        .unwrap()
        .state;
    assert_eq!(s.venue().cash(), cash(218));
    assert_eq!(s.book(Owner::Suspense).unwrap().cash(), cash(0));
    assert_eq!(s.book(Owner::Customer(user(1))).unwrap().cash(), cash(118));
    assert!(!issue(&s, 10, IssueKind::NativePnl).open);
    assert_eq!(s.observations().len(), s.version() as usize);
}

#[test]
fn derived_rounding_is_distinct_from_unexplained_native_funding() {
    let s = boundary(&setup(1, 1), 10);
    let s = inputs(
        &s,
        11,
        10,
        Some(rate(-1, 2, Rounding::TowardZero)),
        Some(-3),
    );
    assert_eq!(s.funding_rounding_residual(&key(10)).unwrap(), cash(-1));
    assert_eq!(s.book(Owner::House).unwrap().funding(), cash(-1));
    assert_eq!(s.book(Owner::Suspense).unwrap().funding(), cash(-2));
    assert_eq!(
        issue(&s, 10, IssueKind::FundingDifference).difference,
        Some(cash(-2))
    );
    let s = settle(&s, 12, 10, -3);
    let s = put(
        &s,
        econ(
            13,
            EconomicChange::CorrectFundingNative {
                boundary: key(10),
                native: cash(-1),
            },
        ),
    );
    assert_eq!(s.venue().cash(), cash(199));
    assert_eq!(s.book(Owner::House).unwrap().cash(), cash(-1));
    assert_eq!(s.book(Owner::Suspense).unwrap().cash(), cash(0));
    assert!(!issue(&s, 10, IssueKind::FundingDifference).open);
}

#[test]
fn normalized_rejections_conflicts_and_rest_ws_overlap_are_retained_without_posting_twice() {
    let s = setup(0, 0);
    let e = execution(10, 1, 1, 100, 1, Some(NativePnl::NetOfFee(cash(-1))));
    let s = put(&s, e.clone());
    let duplicate = s.ingest(&e, s.version() + 1).unwrap(); // Different transport arrival time.
    assert_eq!(duplicate.disposition, Disposition::Duplicate);
    let bad = execution(10, 2, 1, 100, 1, Some(NativePnl::NetOfFee(cash(-1))));
    let contained = duplicate
        .state
        .ingest(&bad, duplicate.state.version() + 1)
        .unwrap();
    assert_eq!(
        contained.disposition,
        Disposition::Rejected(LedgerError::Conflict)
    );
    assert_eq!(contained.state.venue(), s.venue());
    assert_eq!(
        contained
            .state
            .book(Owner::Customer(user(2)))
            .unwrap()
            .cash(),
        cash(100)
    );
    assert_eq!(
        contained
            .state
            .evidence_mode(contained.state.version(), policy())
            .unwrap(),
        EvidenceMode::Frozen
    );
    let again = contained
        .state
        .ingest(&e, contained.state.version() + 1)
        .unwrap();
    assert_eq!(again.disposition, Disposition::Duplicate);
    assert!(issue(&again.state, 10, IssueKind::ReplayConflict).open);
    // Frozen evidence never suppresses an actual losing close.
    let s = put(
        &again.state,
        execution(11, 1, -1, 1, 0, Some(NativePnl::Gross(cash(-99)))),
    );
    assert_eq!(s.book(Owner::Customer(user(1))).unwrap().cash(), cash(0));
    let rebuilt = s
        .observations()
        .iter()
        .fold(Ledger::new(config()).unwrap(), |state, o| {
            let result = state.ingest(&o.event, o.observed_at).unwrap();
            assert_eq!(result.disposition, o.disposition);
            result.state
        });
    assert_eq!(rebuilt, s);
}

#[test]
fn rejected_unready_exact_input_can_retry_but_changed_payload_cannot_replace_it() {
    let s = setup(1, 0);
    let pending = econ(
        11,
        EconomicChange::FundingInputs {
            boundary: key(10),
            rate: Some(rate(-1, 1, Rounding::Exact)),
            native: Some(cash(-1)),
        },
    );
    let rejected = s.ingest(&pending, s.version() + 1).unwrap();
    assert_eq!(
        rejected.disposition,
        Disposition::Rejected(LedgerError::UnknownIdentity)
    );
    let s = boundary(&rejected.state, 10);
    let s = put(&s, pending.clone());
    assert_eq!(
        s.book(Owner::Customer(user(1))).unwrap().funding(),
        cash(-1)
    );
    assert!(!issue(&s, 11, IssueKind::Rejected(LedgerError::UnknownIdentity)).open);
    let wrong = econ(
        11,
        EconomicChange::FundingInputs {
            boundary: key(10),
            rate: Some(rate(-2, 1, Rounding::Exact)),
            native: Some(cash(-2)),
        },
    );
    assert_eq!(
        s.ingest(&wrong, s.version() + 1).unwrap().disposition,
        Disposition::Rejected(LedgerError::Conflict)
    );
}

#[test]
fn missing_source_fields_do_not_become_zero_and_reconciliation_does_not_write_cash() {
    let s = setup(1, 0);
    let mut incomplete = check(&s);
    incomplete.cash = None;
    incomplete.complete = false;
    let s = put(&s, event(10, Change::Reconcile(incomplete)));
    assert_eq!(issue(&s, 10, IssueKind::NativeSnapshot).difference, None);
    assert_eq!(
        s.evidence_mode(s.version(), policy()).unwrap(),
        EvidenceMode::Restricted
    );
    let mut complete = check(&s);
    complete.resolves.push(CheckResolution {
        check: key(10),
        applied_effects: vec![],
    });
    let s = put(&s, event(11, Change::Reconcile(complete)));
    assert_eq!(
        s.evidence_mode(s.version(), policy()).unwrap(),
        EvidenceMode::Reconciled
    );
    let mut mismatch = check(&s);
    mismatch.cash = Some(cash(199));
    mismatch.positions = Some(vec![Position::flat(q(0).unit())]);
    let s = put(&s, event(12, Change::Reconcile(mismatch)));
    assert_eq!(s.venue().cash(), cash(200));
    let mut fake_reset = check(&s);
    fake_reset.resolves.push(CheckResolution {
        check: key(12),
        applied_effects: vec![],
    });
    assert_eq!(
        s.apply(&event(13, Change::Reconcile(fake_reset))),
        Err(LedgerError::Evidence)
    );
    let s = put(
        &s,
        execution(14, 1, -1, 100, 1, Some(NativePnl::NetOfFee(cash(-1)))),
    );
    let mut resolved = check(&s);
    resolved.resolves.push(CheckResolution {
        check: key(12),
        applied_effects: vec![key(14)],
    });
    let s = put(&s, event(15, Change::Reconcile(resolved)));
    assert!(!issue(&s, 12, IssueKind::NativeSnapshot).open);
    assert_eq!(s.venue().cash(), cash(199));
}

#[test]
fn stale_unknown_and_corrupt_marks_block_dependent_views_not_incoming_losses() {
    let s = setup(1, 0);
    let s = put(&s, event(10, Change::Reconcile(check(&s))));
    let now = s.version();
    assert!(s.qualified_diagnostics(&marks(now), now, policy()).is_ok());
    assert!(s.qualified_diagnostics(&[], now, policy()).is_err());
    for variant in 0..5 {
        let mut bad = marks(now);
        match variant {
            0 => bad[0].qualified = false,
            1 => bad[0].observed_at = now + 1,
            2 => bad[0].valid_until = now - 1,
            3 => bad[0].evidence.scope.account = VenueAccountId::new([99; 32]).unwrap(),
            _ => {
                bad[0].price = PriceTicks::new(
                    MarketUnit {
                        precision: PrecisionVersion::new(2).unwrap(),
                        ..q(0).unit()
                    },
                    100,
                )
                .unwrap()
            }
        }
        assert!(s.qualified_diagnostics(&bad, now, policy()).is_err());
    }
    assert!(
        s.qualified_diagnostics(&marks(now), now + 11, policy())
            .is_err()
    );
    assert_eq!(
        s.evidence_mode(now + 51, policy()).unwrap(),
        EvidenceMode::Restricted
    );
    let loss = s
        .ingest(&execution(11, 1, -1, 1, 0, None), now + 51)
        .unwrap();
    assert_eq!(loss.disposition, Disposition::Applied);
    assert_eq!(
        loss.state.book(Owner::Customer(user(1))).unwrap().cash(),
        cash(1)
    );
}

#[test]
fn unrelated_or_repeated_effects_cannot_clear_prior_known_snapshot_gaps() {
    for variant in 0..5 {
        let s = setup(1, 0);
        let mut original = check(&s);
        original.cash = Some(cash(199));
        original.positions = Some(vec![Position::flat(q(0).unit())]);
        match variant {
            0 => original.cash = Some(cash(198)),
            1 => original.funding = Some(cash(1)),
            2 => original.positions = Some(s.venue().positions().to_vec()),
            3 => {
                original.complete = false;
                original.cash = Some(cash(198));
            }
            _ => {} // Duplicate evidence cannot be counted twice.
        }
        let s = put(&s, event(10, Change::Reconcile(original)));
        let s = put(&s, execution(11, 1, -1, 100, 1, None));
        let mut later = check(&s);
        later.resolves.push(CheckResolution {
            check: key(10),
            applied_effects: if variant == 4 {
                vec![key(11), key(11)]
            } else {
                vec![key(11)]
            },
        });
        assert_eq!(
            s.apply(&event(12, Change::Reconcile(later))),
            Err(LedgerError::Evidence)
        );
        assert!(issue(&s, 10, IssueKind::NativeSnapshot).open);
        assert_eq!(s.venue().cash(), cash(199));
    }
}

#[test]
fn settlement_difference_is_suspense_not_changed_customer_funding() {
    let s = boundary(&setup(1, 0), 10);
    let s = inputs(&s, 11, 10, Some(rate(-2, 1, Rounding::Exact)), Some(-2));
    let s = settle(&s, 12, 10, -3);
    assert_eq!(s.book(Owner::Customer(user(1))).unwrap().cash(), cash(98));
    assert_eq!(s.book(Owner::Customer(user(1))).unwrap().funding(), cash(0));
    assert_eq!(s.book(Owner::Suspense).unwrap().cash(), cash(-1));
    assert_eq!(s.book(Owner::House).unwrap().cash(), cash(0));
    assert_eq!(
        issue(&s, 10, IssueKind::FundingDifference).difference,
        Some(cash(-1))
    );
    let double_settlement = econ(
        13,
        EconomicChange::FundingSettlement {
            boundary: key(10),
            native: cash(-3),
        },
    );
    assert_eq!(s.apply(&double_settlement), Err(LedgerError::FundingState));
    // Known rate/amount cannot silently change through another input message.
    let overwrite = econ(
        14,
        EconomicChange::FundingInputs {
            boundary: key(10),
            rate: Some(rate(-3, 1, Rounding::Exact)),
            native: Some(cash(-3)),
        },
    );
    assert_eq!(s.apply(&overwrite), Err(LedgerError::FundingState));
    let s = put(
        &s,
        econ(
            15,
            EconomicChange::CorrectFundingNative {
                boundary: key(10),
                native: cash(-2),
            },
        ),
    );
    assert_eq!(s.venue().cash(), cash(198));
    assert_eq!(s.book(Owner::Suspense).unwrap().cash(), cash(0));
    assert!(!issue(&s, 10, IssueKind::FundingDifference).open);
}

#[test]
fn unrepresentable_fee_is_retained_without_a_partial_fill() {
    let s = setup(0, 0);
    let result = s
        .ingest(&execution(10, 1, 1, 100, i128::MIN, None), s.version() + 1)
        .unwrap();
    assert_eq!(
        result.disposition,
        Disposition::Rejected(LedgerError::Primitive(Error::Overflow))
    );
    assert_eq!(result.state.venue(), s.venue());
    assert_eq!(
        result.state.book(Owner::Customer(user(1))).unwrap(),
        s.book(Owner::Customer(user(1))).unwrap()
    );
    assert_eq!(result.state.events(), s.events());
    assert_eq!(
        result.state.observations().len(),
        s.observations().len() + 1
    );
    assert!(
        issue(
            &result.state,
            10,
            IssueKind::Rejected(LedgerError::Primitive(Error::Overflow))
        )
        .open
    );
}

#[test]
fn stale_inventory_cut_and_invalid_units_are_retained_without_partial_posting() {
    let s = setup(1, 0);
    let stale = econ(
        10,
        EconomicChange::FundingBoundary {
            market: q(0).unit(),
            expected_version: s.version() - 1,
        },
    );
    let contained = s.ingest(&stale, s.version() + 1).unwrap();
    assert_eq!(
        contained.disposition,
        Disposition::Rejected(LedgerError::StaleCut)
    );
    assert_eq!(contained.state.venue(), s.venue());
    let wrong = QuoteAtoms::new(
        AssetUnit {
            asset: AssetId::new([99; 32]).unwrap(),
            ..config().quote
        },
        1,
    );
    let bad = econ(
        11,
        EconomicChange::Execution {
            target: FillTarget::Customer(route(1, false)),
            quantity: q(-1),
            price: p(110),
            fee: cash(1),
            pnl: Some(NativePnl::Gross(wrong)),
        },
    );
    let rejected = contained
        .state
        .ingest(&bad, contained.state.version() + 1)
        .unwrap();
    assert_eq!(
        rejected.disposition,
        Disposition::Rejected(LedgerError::Primitive(Error::UnitMismatch))
    );
    assert_eq!(rejected.state.venue(), s.venue());
    assert_eq!(
        rejected.state.book(Owner::Customer(user(1))).unwrap(),
        s.book(Owner::Customer(user(1))).unwrap()
    );
}

#[test]
fn funding_minimum_signed_value_can_move_from_accrual_to_cash() {
    let s = boundary(&Ledger::new(config()).unwrap(), 10);
    let s = inputs(&s, 11, 10, None, Some(i128::MIN));
    let s = settle(&s, 12, 10, i128::MIN);
    assert_eq!(s.venue().cash(), cash(i128::MIN));
    assert_eq!(s.venue().funding(), cash(0));
    assert_eq!(s.book(Owner::Suspense).unwrap().cash(), cash(i128::MIN));
    s.check_bridge().unwrap();
}

#[test]
fn property_funding_signed_rounding_and_bridge_match_small_independent_oracle() {
    fn div(n: i128, d: i128, mode: Rounding) -> Option<i128> {
        let q = n / d;
        let r = n % d;
        match mode {
            Rounding::Exact if r != 0 => None,
            Rounding::Floor if r < 0 => Some(q - 1),
            Rounding::Ceil if r > 0 => Some(q + 1),
            _ => Some(q),
        }
    }
    for a in -2..=2 {
        for b in -2..=2 {
            for n in -2..=2 {
                for mode in [
                    Rounding::Exact,
                    Rounding::TowardZero,
                    Rounding::Floor,
                    Rounding::Ceil,
                ] {
                    let s = boundary(&setup(a, b), 10);
                    let values = [
                        div(i128::from(a * n), 3, mode),
                        div(i128::from(b * n), 3, mode),
                        div(i128::from((a + b) * n), 3, mode),
                    ];
                    let e = econ(
                        11,
                        EconomicChange::FundingInputs {
                            boundary: key(10),
                            rate: Some(rate(i128::from(n), 3, mode)),
                            native: Some(cash(values[2].unwrap_or(0))),
                        },
                    );
                    let result = s.ingest(&e, s.version() + 1).unwrap();
                    if let [Some(fa), Some(fb), Some(fv)] = values {
                        assert_eq!(result.disposition, Disposition::Applied);
                        assert_eq!(
                            result
                                .state
                                .book(Owner::Customer(user(1)))
                                .unwrap()
                                .funding(),
                            cash(fa)
                        );
                        assert_eq!(
                            result
                                .state
                                .book(Owner::Customer(user(2)))
                                .unwrap()
                                .funding(),
                            cash(fb)
                        );
                        assert_eq!(
                            result.state.book(Owner::House).unwrap().funding(),
                            cash(fv - fa - fb)
                        );
                        assert_eq!(
                            result.state.book(Owner::Suspense).unwrap().funding(),
                            cash(0)
                        );
                        assert_eq!(
                            result.state.funding_rounding_residual(&key(10)).unwrap(),
                            cash(fv - fa - fb)
                        );
                        let before = result.state.diagnostics(&[p(100)]).unwrap();
                        let settled = settle(&result.state, 12, 10, fv);
                        assert_eq!(settled.diagnostics(&[p(100)]).unwrap(), before);
                    } else {
                        assert_eq!(
                            result.disposition,
                            Disposition::Rejected(LedgerError::Primitive(Error::Inexact))
                        );
                        assert_eq!(result.state.venue(), s.venue());
                    }
                }
            }
        }
    }
}
