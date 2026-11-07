//! Synthetic RPC/code/state qualification, actual journal, crypto and funding
//! ports. The separate joined SBF test executes the owner deposit and sweep.
use super::*;
use cinder_journal::escrow::{Batch, Status};
use cinder_pacifica::funding::escrow::Routing;

fn create(temp: &support::Temp) -> (support::Store, Controller) {
    seed(
        Journal::create(
            SqliteBackend::create(&temp.db).unwrap(),
            support::FixtureProtection,
            config(),
        )
        .unwrap(),
    )
}

#[test]
fn post_genesis_owner_routes_and_deposit_hints_use_the_accepted_journal_without_rebinding_the_release()
 {
    let t = support::Temp::new();
    let (mut j, c) = create(&t);
    let release = c.release_commitment(j.configuration()).unwrap();
    let bind = |wallet, tokens| {
        let account = cinder_journal::onboarding::account(config().domain, wallet).unwrap();
        cinder_journal::onboarding::Binding {
            account,
            wallet,
            tokens,
        }
    };
    let public = |n: u8| -> [u8; 32] {
        openssl::pkey::PKey::private_key_from_raw_bytes(&[n; 32], openssl::pkey::Id::ED25519)
            .unwrap()
            .raw_public_key()
            .unwrap()
            .try_into()
            .unwrap()
    };
    let b = bind(public(77), [78; 32]);
    let commit = |j: &mut support::Store, n, b: cinder_journal::onboarding::Binding| {
        let mut tx = super::tx(j, n, vec![], vec![]);
        tx.controls = vec![
            Control::Onboarding(cinder_journal::onboarding::Action::Register(b.clone())),
            Control::Order(orders::Action::AdvanceAuthority {
                account: b.account,
                epoch: 1,
            }),
        ];
        assert_eq!(j.commit(tx).unwrap().receipt.controls, None);
    };
    commit(&mut j, 80, b.clone());
    let snapshot = c.route_for(j.state().unwrap(), b.account).unwrap();
    assert_eq!(snapshot.beneficiaries.len(), 1);
    assert_eq!(snapshot.beneficiaries[0].wallet, b.wallet);
    let c2 = bind(public(79), [80; 32]);
    commit(&mut j, 81, c2.clone());
    assert_eq!(
        snapshot,
        c.route_for(j.state().unwrap(), b.account).unwrap()
    );
    assert_eq!(c.release_commitment(j.configuration()).unwrap(), release);
    let s = chain::Escrow::derive(c.route(), b.wallet, [81; 32], 400).unwrap();
    let batch = Batch {
        request: RequestKey {
            domain: config().domain,
            account: b.account,
            request: RequestId::new([81; 32]).unwrap(),
        },
        route: s.route,
        batch: s.batch,
        operation: s.operation,
        owner: b.wallet,
        amount: atoms(20),
        epoch: 1,
        expires_at_slot: 400,
        signature: [82; 64],
    };
    c.observe_escrow_deposit(
        &mut j,
        id(82),
        100,
        batch.clone(),
        PrivateBytes::new(b"explicit synthetic qualified new-owner inventory".to_vec()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        j.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(b.account))
            .unwrap()
            .cash(),
        atoms(0)
    );
    let mut wrong = batch.clone();
    wrong.request.account = c2.account;
    assert!(
        c.observe_escrow_deposit(
            &mut j,
            id(83),
            100,
            wrong,
            PrivateBytes::new(b"wrong owner".to_vec()).unwrap()
        )
        .is_err()
    );
    let mut hint = super::tx(&j, 84, vec![], vec![]);
    hint.controls = vec![Control::Onboarding(
        cinder_journal::onboarding::Action::Notice(cinder_journal::onboarding::Notice {
            account: b.account,
            operation: batch.operation,
            signature: batch.signature,
        }),
    )];
    assert_eq!(j.commit(hint).unwrap().receipt.controls, None);
    let fake = fake(10);
    let port = port(&c, &fake);
    let locators = port.locators(j.state().unwrap());
    assert_eq!(locators.len(), 1);
    assert_eq!(locators[0].account, b.account.bytes());
    assert_eq!(locators[0].signature, batch.signature);
}
pub(crate) fn seed<B: Backend, P: Protection>(
    journal: Journal<B, P>,
) -> (Journal<B, P>, Controller) {
    let r = route();
    let c = Controller::new_escrow(
        profile(),
        r.clone(),
        Routing {
            route: chain::pda(r.program, &[b"venue_route", &r.config]).unwrap(),
            venue: config().venue.bytes(),
        },
        Zeroizing::new([9; 32]),
    )
    .unwrap();
    seed_with_controller(journal, Location::Vault, c)
}
pub(crate) fn start<B: Backend, P: Protection>(
    j: &mut Journal<B, P>,
    c: &Controller,
) -> chain::Escrow {
    start_with_expiry(j, c, 10_000)
}
pub(crate) fn start_with_expiry<B: Backend, P: Protection>(
    j: &mut Journal<B, P>,
    c: &Controller,
    expires_at: u64,
) -> chain::Escrow {
    let r = c.route();
    let s = chain::Escrow::derive(r, r.beneficiaries[0].wallet, [10; 32], 400).unwrap();
    c.observe_escrow_deposit(
        j,
        id(60),
        100,
        Batch {
            request: support::request(10),
            route: s.route,
            batch: s.batch,
            operation: s.operation,
            owner: r.beneficiaries[0].wallet,
            amount: atoms(20_000_000),
            epoch: 1,
            expires_at_slot: 400,
            signature: [60; 64],
        },
        PrivateBytes::new(b"synthetic qualified original owner deposit".to_vec()).unwrap(),
    )
    .unwrap();
    c.prepare_escrow_sweep(j, id(61), 100, support::attempt(10), expires_at)
        .unwrap();
    s
}
pub(crate) fn pending(f: &Fake, s: &chain::Escrow, c: &Controller) {
    let r = c.route();
    let mut state = f.0.lock().unwrap();
    let put = |b: &mut [u8], at: usize, x: &[u8]| b[at..at + x.len()].copy_from_slice(x);
    let mut values = vec![];
    let mut b = vec![0; 106];
    put(&mut b, 0, &sha256(b"account:VenueRoute")[..8]);
    b[8] = 1;
    put(&mut b, 10, &r.config);
    put(&mut b, 42, &config().venue.bytes());
    put(&mut b, 74, &r.broker);
    values.push((s.route, r.program, b));
    let mut b = vec![0; 83];
    put(&mut b, 0, &sha256(b"account:VenueEscrow")[..8]);
    b[8] = 1;
    put(&mut b, 11, &s.route);
    put(&mut b, 43, &r.beneficiaries[0].wallet);
    put(&mut b, 75, &20_000_000u64.to_le_bytes());
    values.push((s.escrow, r.program, b));
    let mut b = vec![0; 179];
    put(&mut b, 0, &sha256(b"account:EscrowBatch")[..8]);
    b[8] = 1;
    put(&mut b, 11, &s.escrow);
    put(&mut b, 43, &s.operation);
    put(&mut b, 75, &20_000_000u64.to_le_bytes());
    put(&mut b, 83, &1u64.to_le_bytes());
    put(&mut b, 91, &400u64.to_le_bytes());
    values.push((s.batch, r.program, b));
    let mut b = vec![0; 165];
    put(&mut b, 0, &r.mint);
    put(&mut b, 32, &s.escrow);
    put(&mut b, 64, &20_000_000u64.to_le_bytes());
    b[108] = 1;
    values.push((
        s.tokens,
        chain::parse_address("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap(),
        b,
    ));
    for (key, owner, data) in values {
        state.accounts.insert(
            key,
            account_json(&Account {
                key,
                owner,
                executable: false,
                lamports: 1_000_000,
                data: PrivateBytes::new(data).unwrap(),
            }),
        );
    }
}
#[test]
fn sweep_port_persists_one_original_wire_and_reconciles_lost_ack_without_second_credit_or_send() {
    let temp = support::Temp::new();
    let (mut j, c) = create(&temp);
    let s = start(&mut j, &c);
    let f = fake(10);
    pending(&f, &s, &c);
    f.0.lock().unwrap().lose_ack = true;
    let mut p = port(&c, &f);
    assert!(p.issue(&mut j, &c, support::attempt(10)).unwrap() == Outcome::Unknown);
    assert_eq!(j.state().unwrap().ledger().broker(), atoms(0));
    assert_eq!(j.state().unwrap().escrows()[0].status, Status::Exposed);
    retain_at(&f, &mut j, &c, support::attempt(10), 100);
    assert!(p.reconcile(&mut j, &c, support::attempt(10)).unwrap() == Outcome::Settled);
    assert_eq!(j.state().unwrap().ledger().broker(), atoms(20_000_000));
    assert_eq!(
        j.state()
            .unwrap()
            .reserved(Resource::Customer(support::user(1)))
            .unwrap(),
        atoms(20_000_000)
    );
    let before = j.state().unwrap().clone();
    assert!(p.reconcile(&mut j, &c, support::attempt(10)).unwrap() == Outcome::Settled);
    assert_eq!(j.state().unwrap(), &before);
    assert!(p.issue(&mut j, &c, support::attempt(10)).is_err());
    assert_eq!(f.calls("sendTransaction"), 1);
    let current = c
        .retained_wire(&mut j, support::attempt(10), 100)
        .unwrap()
        .unwrap();
    let contract = c
        .original_chain_contract(&mut j, support::attempt(10), 100)
        .unwrap();
    for offset in [8, 10, 11, 43, 75, 83, 91, 99, 131, 163, 171] {
        let mut case = chain_receipt::tests::fixture_from(
            contract.as_bytes().to_vec(),
            support::attempt(10),
            current.wire.as_bytes().to_vec(),
        );
        let a = case
            .accounts
            .values
            .iter_mut()
            .flatten()
            .find(|a| a.key == s.batch)
            .unwrap();
        let mut b = a.data.as_bytes().to_vec();
        b[offset] ^= 1;
        a.data = PrivateBytes::new(b).unwrap();
        assert!(
            chain_receipt::recognize(
                &case.contract,
                case.attempt,
                &case.wire,
                case.tx,
                case.accounts,
                &case.deployment,
                6000
            )
            .is_err(),
            "batch field {offset}"
        );
    }
}
#[test]
fn refund_missing_batch_expiry_and_underbacking_refuse_before_exposure_or_signature() {
    for kind in 0..4 {
        let temp = support::Temp::new();
        let (mut j, c) = create(&temp);
        let s = start(&mut j, &c);
        let f = fake(10);
        pending(&f, &s, &c);
        {
            let mut rpc = f.0.lock().unwrap();
            match kind {
                0 => {
                    let a = rpc.accounts.get_mut(&s.batch).unwrap();
                    let mut b = STANDARD.decode(a["data"][0].as_str().unwrap()).unwrap();
                    b[10] = 2;
                    a["data"][0] = json!(STANDARD.encode(b));
                }
                1 => {
                    rpc.accounts.remove(&s.batch);
                }
                2 => {
                    let a = rpc.accounts.get_mut(&s.batch).unwrap();
                    let mut b = STANDARD.decode(a["data"][0].as_str().unwrap()).unwrap();
                    b[91..99].copy_from_slice(&100u64.to_le_bytes());
                    a["data"][0] = json!(STANDARD.encode(b));
                }
                _ => {
                    let a = rpc.accounts.get_mut(&s.tokens).unwrap();
                    let mut b = STANDARD.decode(a["data"][0].as_str().unwrap()).unwrap();
                    b[64..72].fill(0);
                    a["data"][0] = json!(STANDARD.encode(b));
                }
            }
        }
        assert!(
            port(&c, &f)
                .issue(&mut j, &c, support::attempt(10))
                .is_err()
        );
        assert_eq!(j.state().unwrap().escrows()[0].status, Status::Prepared);
        assert_eq!(f.calls("sendTransaction"), 0);
    }
}

#[test]
fn expired_sweep_closes_only_without_retained_wire_never_from_timeout_alone() {
    for retained in [false, true] {
        let temp = support::Temp::new();
        let (mut j, c) = create(&temp);
        let s = start(&mut j, &c);
        let f = fake(10);
        pending(&f, &s, &c);
        f.0.lock().unwrap().fail_sim = !retained;
        let mut p = port(&c, &f);
        let issued = p.issue(&mut j, &c, support::attempt(10));
        assert_eq!(issued.is_ok(), retained);
        assert_eq!(
            c.prepare_next_escrow(&mut j, 10_000).unwrap(),
            Some(support::attempt(10))
        );
        assert_eq!(j.state().unwrap().escrows()[0].status, Status::Exposed);
        assert!(
            !c.close_unsent_chain(&mut j, id(70), 100, support::attempt(10))
                .unwrap()
        );
        assert_eq!(
            c.close_unsent_chain(&mut j, id(71), 10_000, support::attempt(10))
                .unwrap(),
            !retained
        );
        assert_eq!(
            j.state().unwrap().escrows()[0].status,
            if retained {
                Status::Exposed
            } else {
                Status::Failed
            }
        );
        assert_eq!(j.state().unwrap().ledger().broker(), atoms(0));
        assert!(p.issue(&mut j, &c, support::attempt(10)).is_err());
        assert_eq!(f.calls("sendTransaction"), usize::from(retained));
    }
}

#[test]
fn invalid_owner_locator_cannot_poison_the_rightful_deposit_signature() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../pacifica/tests/fixtures/escrow-deposit-public.json"
    ))
    .unwrap();
    let wire: Vec<u8> = serde_json::from_value(fixture["wire"].clone()).unwrap();
    let temp = support::Temp::new();
    let (mut j, c) = create(&temp);
    let before = j.state().unwrap().ledger().clone();
    let f = fake(41);
    f.0.lock().unwrap().wire = Some(wire.clone());
    let valid = crate::customer_deposit::Locator {
        account: [1; 32],
        operation: [41; 32],
        signature: wire[1..65].to_vec(),
    };
    let mut wrong_owner = valid.clone();
    wrong_owner.account = [2; 32];
    let mut wrong_operation = valid.clone();
    wrong_operation.operation = [42; 32];
    let mut p = loaded(&c, vec![])
        .with_transport(f.clone(), Arc::new(Fixed))
        .unwrap();
    for invalid in [wrong_owner, wrong_operation] {
        assert!(
            p.collect_locator(c.route(), true, invalid)
                .unwrap()
                .complete(&mut j, &c)
                .unwrap()
                == Outcome::Rejected
        );
    }
    assert!(
        p.collect_locator(c.route(), true, valid)
            .unwrap()
            .complete(&mut j, &c)
            .unwrap()
            == Outcome::Settled
    );
    assert_eq!(j.state().unwrap().escrows()[0].status, Status::Pending);
    assert_eq!(j.state().unwrap().ledger(), &before);
    assert_eq!(f.calls("sendTransaction"), 0);
}

#[test]
fn original_owner_escrow_wire_is_pending_not_cash_and_refund_observation_is_atomic() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../pacifica/tests/fixtures/escrow-deposit-public.json"
    ))
    .unwrap();
    let wire: Vec<u8> = serde_json::from_value(fixture["wire"].clone()).unwrap();
    for refunded in [false, true] {
        let temp = support::Temp::new();
        let (mut j, c) = create(&temp);
        let before = j.state().unwrap().ledger().clone();
        let f = fake(41);
        {
            let mut state = f.0.lock().unwrap();
            state.wire = Some(wire.clone());
            state.escrow_refunded = refunded;
        }
        let locator = crate::customer_deposit::Locator {
            account: [1; 32],
            operation: [41; 32],
            signature: wire[1..65].to_vec(),
        };
        let mut p = loaded(&c, vec![locator])
            .with_transport(f.clone(), Arc::new(Fixed))
            .unwrap();
        assert!(p.observe_deposit(&mut j, &c, 0).unwrap() == Outcome::Settled);
        assert_eq!(j.state().unwrap().ledger(), &before);
        assert_eq!(
            j.state().unwrap().escrows()[0].status,
            if refunded {
                Status::Refunded
            } else {
                Status::Pending
            }
        );
        assert_eq!(
            j.transactions().last().unwrap().controls.len(),
            if refunded { 2 } else { 1 }
        );
        let head = j.head();
        assert!(p.observe_deposit(&mut j, &c, 0).unwrap() == Outcome::Settled);
        assert_eq!(j.head(), head);
        assert_eq!(f.calls("sendTransaction"), 0);
    }
}

#[test]
fn escrow_controller_cannot_reinterpret_a_legacy_funding_history_or_reuse_its_contract() {
    let temp = support::Temp::new();
    let (mut journal, legacy) = setup(&temp);
    let r = route();
    let escrow = Controller::new_escrow(
        profile(),
        r.clone(),
        Routing {
            route: chain::pda(r.program, &[b"venue_route", &r.config]).unwrap(),
            venue: config().venue.bytes(),
        },
        Zeroizing::new([9; 32]),
    )
    .unwrap();
    assert_ne!(
        legacy.release_commitment(&config()).unwrap(),
        escrow.release_commitment(&config()).unwrap()
    );
    let head = journal.head();
    assert!(escrow.prepare_next_escrow(&mut journal, 100).is_err());
    assert_eq!(journal.head(), head);
    assert!(journal.state().unwrap().escrows().is_empty());
}

#[test]
fn expired_unexposed_sweep_cancels_once_after_restart_and_unblocks_later_owner_inventory() {
    let temp = support::Temp::new();
    let (mut j, c) = create(&temp);
    start_with_expiry(&mut j, &c, 101);
    let mut b = j.state().unwrap().escrows()[0].batch.clone();
    let s = chain::Escrow::derive(c.route(), b.owner, [20; 32], 400).unwrap();
    b.request = support::request(20);
    b.operation = s.operation;
    b.batch = s.batch;
    b.signature = [20; 64];
    c.observe_escrow_deposit(
        &mut j,
        id(70),
        100,
        b,
        support::raw(b"qualified later owner deposit"),
    )
    .unwrap();
    let ledger = j.state().unwrap().ledger().clone();
    let reopen = || {
        Journal::open(
            SqliteBackend::open(&temp.db, Migration::None).unwrap(),
            support::FixtureProtection,
            config(),
        )
        .unwrap()
    };
    drop(j);
    let mut j = reopen();
    assert_eq!(c.prepare_next_escrow(&mut j, 101).unwrap(), None);
    assert_eq!(j.state().unwrap().escrows()[0].status, Status::Failed);
    assert_eq!(j.state().unwrap().ledger(), &ledger);
    assert!(
        !j.state()
            .unwrap()
            .attempts()
            .iter()
            .find(|a| a.key == support::attempt(10))
            .unwrap()
            .possibly_exposed
    );
    assert!(
        c.retained_wire(&mut j, support::attempt(10), 101)
            .unwrap()
            .is_none()
    );
    let first = j.head();
    drop(j);
    let mut j = reopen();
    assert_eq!(j.head(), first);
    let next = c.prepare_next_escrow(&mut j, 102).unwrap().unwrap();
    assert_eq!(next.request, support::request(20));
    assert_eq!(j.state().unwrap().escrows()[0].status, Status::Failed);
    c.observe_escrow_refund(
        &mut j,
        id(71),
        103,
        support::request(10),
        support::raw(b"qualified original owner refund"),
    )
    .unwrap();
    assert_eq!(j.state().unwrap().escrows()[0].status, Status::Refunded);
    assert_eq!(j.state().unwrap().ledger(), &ledger);
    assert!(
        c.prepare_escrow_sweep(&mut j, id(72), 104, support::attempt(10), 1000)
            .is_err()
    );
}

#[test]
fn expired_unexposed_sweep_cleanup_does_not_require_fresh_setup_or_normal_admission() {
    for recovery in [false, true] {
        let temp = support::Temp::new();
        let (mut j, c) = create(&temp);
        start_with_expiry(&mut j, &c, 101);
        let control = if recovery {
            Control::Recovery(cinder_journal::recovery::Action::Begin {
                expected_version: j.state().unwrap().ledger().version(),
                authority_epoch: 7,
                valid_until: 30_000,
                fence: [99; 32],
            })
        } else {
            Control::Funds(lifecycle::Action::Freeze)
        };
        let t = tx(&j, 70, vec![], vec![control]);
        assert_eq!(j.commit(t).unwrap().receipt.controls, None);
        assert_eq!(c.prepare_next_escrow(&mut j, 20_000).unwrap(), None);
        assert_eq!(j.state().unwrap().escrows()[0].status, Status::Failed);
        let head = j.head();
        assert_eq!(c.prepare_next_escrow(&mut j, 20_000).unwrap(), None);
        assert_eq!(j.head(), head);
        assert!(
            j.state()
                .unwrap()
                .holds()
                .iter()
                .filter(|h| h.request == support::request(10))
                .all(|h| !h.active)
        );
    }
}

#[test]
fn expired_sweep_cleanup_requires_the_verified_current_journal_head() {
    let temp = support::Temp::new();
    let fault = Arc::new(StorageFault::default());
    let (mut j, c) = seed(
        Journal::create(
            Guarded {
                inner: SqliteBackend::create(&temp.db).unwrap(),
                fault: fault.clone(),
            },
            support::FixtureProtection,
            config(),
        )
        .unwrap(),
    );
    start_with_expiry(&mut j, &c, 101);
    let head = j.head();
    fault.fenced.store(true, Ordering::SeqCst);
    assert!(
        c.cancel_expired_escrow_sweep(&mut j, 101, support::attempt(10))
            .is_err()
    );
    assert_eq!(j.head(), head);
    assert!(j.state().is_err());
    drop(j);
    let j = Journal::open(
        SqliteBackend::open(&temp.db, Migration::None).unwrap(),
        support::FixtureProtection,
        config(),
    )
    .unwrap();
    assert_eq!(j.state().unwrap().escrows()[0].status, Status::Prepared);
}

#[test]
fn frozen_reconciliation_prioritizes_original_exposed_sweep_over_earlier_unexposed_work() {
    let temp = support::Temp::new();
    let (mut j, c) = create(&temp);
    start(&mut j, &c);
    let mut b = j.state().unwrap().escrows()[0].batch.clone();
    let s = chain::Escrow::derive(c.route(), b.owner, [20; 32], 400).unwrap();
    b.request = support::request(20);
    b.operation = s.operation;
    b.batch = s.batch;
    b.signature = [20; 64];
    c.observe_escrow_deposit(
        &mut j,
        id(70),
        100,
        b,
        support::raw(b"qualified second original"),
    )
    .unwrap();
    c.prepare_escrow_sweep(&mut j, id(71), 100, support::attempt(20), 10_000)
        .unwrap();
    let f = fake(20);
    pending(&f, &s, &c);
    port(&c, &f)
        .issue(&mut j, &c, support::attempt(20))
        .unwrap();
    let t = tx(
        &j,
        72,
        vec![],
        vec![Control::Funds(lifecycle::Action::Freeze)],
    );
    j.commit(t).unwrap();
    assert_eq!(
        c.prepare_next_escrow(&mut j, 100).unwrap(),
        Some(support::attempt(20))
    );
    assert_eq!(j.state().unwrap().escrows()[0].status, Status::Prepared);
}

#[test]
fn full_batch_forecast_survives_handoff_acceptance_and_preparation_without_double_allocation() {
    let temp = support::Temp::new();
    let (mut j, c) = create(&temp);
    let s = start(&mut j, &c);
    let f = fake(10);
    pending(&f, &s, &c);
    let mut p = port(&c, &f);
    p.issue(&mut j, &c, support::attempt(10)).unwrap();
    retain_at(&f, &mut j, &c, support::attempt(10), 100);
    p.reconcile(&mut j, &c, support::attempt(10)).unwrap();
    for prepared in 0..3 {
        let n = c.funding_need(&mut j, 100, 20_000_000).unwrap();
        assert_eq!(
            (n.broker_usable, n.pending_ingress, n.additional),
            (0, 20_000_000, 0)
        );
        if prepared == 0 {
            assert_eq!(c.prepare_next_escrow(&mut j, 100).unwrap(), None);
        } else if prepared == 1 {
            let request = j.state().unwrap().escrows()[0].funding.unwrap();
            let t = tx(
                &j,
                73,
                vec![],
                vec![Control::Funds(lifecycle::Action::Prepare {
                    attempt: AttemptKey {
                        request,
                        attempt: support::attempt(80).attempt,
                    },
                    net: atoms(20_000_000),
                })],
            );
            assert_eq!(j.commit(t).unwrap().receipt.controls, None);
        }
    }
}

#[test]
fn funded_sweep_can_be_recalled_through_original_return_port_and_shared_recovery_counters() {
    use cinder_pacifica::funding::recovery::{CustodyState, CustomerState};
    let temp = support::Temp::new();
    let (mut j, c) = create(&temp);
    let original = start(&mut j, &c);
    let f = fake(10);
    pending(&f, &original, &c);
    let mut p = port(&c, &f);
    p.issue(&mut j, &c, support::attempt(10)).unwrap();
    retain_at(&f, &mut j, &c, support::attempt(10), 100);
    p.reconcile(&mut j, &c, support::attempt(10)).unwrap();
    let version = j.state().unwrap().ledger().version();
    let t = tx(
        &j,
        70,
        vec![],
        vec![Control::Recovery(cinder_journal::recovery::Action::Begin {
            expected_version: version,
            authority_epoch: 7,
            valid_until: 10_000,
            fence: [99; 32],
        })],
    );
    assert_eq!(j.commit(t).unwrap().receipt.controls, None);
    c.prepare_escrow_recall(
        &mut j,
        id(71),
        100,
        support::request(10),
        support::attempt(60),
        10_000,
    )
    .unwrap();
    assert_eq!(
        c.funding_need(&mut j, 100, 20_000_000)
            .unwrap()
            .pending_ingress,
        0
    );
    f.next(60);
    {
        let mut state = f.0.lock().unwrap();
        state.frozen = true;
        let a = state.accounts.get_mut(&route().config).unwrap();
        let mut b = STANDARD.decode(a["data"][0].as_str().unwrap()).unwrap();
        b[268..276].copy_from_slice(&2u64.to_le_bytes());
        b[276] = 1;
        a["data"][0] = json!(STANDARD.encode(b));
    }
    p.issue(&mut j, &c, support::attempt(60)).unwrap();
    retain_at(&f, &mut j, &c, support::attempt(60), 100);
    p.reconcile(&mut j, &c, support::attempt(60)).unwrap();
    assert_eq!(j.state().unwrap().escrows()[0].status, Status::Returned);
    assert_eq!(j.state().unwrap().ledger().broker(), atoms(0));
    assert_eq!(
        j.state().unwrap().ledger().paid(support::user(1)).unwrap(),
        atoms(0)
    );
    let l = j.state().unwrap().ledger();
    let check = NativeCheck {
        expected_version: l.version(),
        cash: Some(l.venue().cash()),
        funding: Some(l.venue().funding()),
        positions: Some(l.venue().positions().to_vec()),
        complete: true,
        resolves: vec![],
    };
    let t = tx(
        &j,
        72,
        vec![(Location::Venue, Change::Reconcile(check))],
        vec![],
    );
    j.commit(t).unwrap();
    let r = route();
    let observed = CustodyState {
        network: [1; 32],
        program: r.program,
        config: r.config,
        vault: r.vault,
        mint: r.mint,
        domain: r.domain,
        pool: r.pool,
        broker: r.broker,
        broker_tokens: r.broker_tokens,
        decimals: r.decimals,
        epoch: 2,
        mode: 1,
        normal_paid: 0,
        funding_sequence: 1,
        vault_amount: 140_000_000,
        finalized_slot: 201,
        customers: r
            .beneficiaries
            .iter()
            .map(|b| CustomerState {
                wallet: b.wallet,
                tokens: b.tokens,
                paid: 0,
                payout_sequence: 0,
            })
            .collect(),
    };
    let check = EventKey {
        scope: config().sources[0].scope,
        event: EconomicEventId::new(&[72]).unwrap(),
        leg: 0,
    };
    let head = j.head();
    assert_eq!(
        c.recovery_backing(&mut j, head, check.clone(), &observed)
            .unwrap()
            .total(),
        40_000_000
    );
    let mut wrong = observed.clone();
    wrong.funding_sequence = 0;
    assert!(
        c.recovery_backing(&mut j, head, check.clone(), &wrong)
            .is_err()
    );
    wrong = observed;
    wrong.finalized_slot = 199;
    assert!(c.recovery_backing(&mut j, head, check, &wrong).is_err());
}
