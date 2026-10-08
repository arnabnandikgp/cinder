//! Offline joined funds tests; public synthetic keys and qualified fake evidence.
#[path = "../../journal/tests/support/mod.rs"]
mod support;
use cinder_journal::{collateral, funds as lifecycle, model::*, orders, sqlite::*, *};
use cinder_kernel::{
    amounts::*,
    codec::Canonical,
    identity::*,
    ledger::{evidence::*, funds::*, *},
};
use cinder_pacifica::{
    Error as AdapterError,
    execution::{Dispatch, Gateway, Origin, Outbound, Policy, Reply, Transport},
    funding::recovery::{BindingError, CustodyState, CustomerState},
    funding::*,
    profile::*,
};

fn recovery_cut(j: &mut Store) -> (Head, EventKey) {
    let freeze = tx(
        j,
        210,
        vec![],
        vec![Control::Funds(lifecycle::Action::Freeze)],
    );
    assert_eq!(j.commit(freeze).unwrap().receipt.controls, None);
    let l = j.state().unwrap().ledger();
    let check = NativeCheck {
        expected_version: l.version(),
        cash: Some(l.venue().cash()),
        funding: Some(l.venue().funding()),
        positions: Some(l.venue().positions().to_vec()),
        complete: true,
        resolves: vec![],
    };
    let check = tx(
        j,
        211,
        vec![(Location::Venue, Change::Reconcile(check))],
        vec![],
    );
    assert_eq!(
        j.commit(check).unwrap().receipt.inputs,
        [InputResult::Normalized(Disposition::Applied)]
    );
    (j.head(), key(211, Location::Venue))
}
fn begin_recovery(j: &mut Store) {
    let control = Control::Recovery(cinder_journal::recovery::Action::Begin {
        expected_version: j.state().unwrap().ledger().version(),
        authority_epoch: 7,
        valid_until: 20_000,
        fence: [99; 32], // Explicit synthetic qualified cutover port, not native proof.
    });
    let t = tx(j, 208, vec![], vec![control]);
    assert_eq!(j.commit(t).unwrap().receipt.controls, None);
}
fn recovery_intent(n: u8, source: Location, destination: Destination) -> lifecycle::Intent {
    lifecycle::Intent {
        request: a(n).request,
        source,
        destination,
        net: atoms(20),
        maximum_fee: atoms(0),
        fee_payer: Owner::House,
        allow_partial: false,
        policy: config().policy,
        authority_epoch: 7,
        expires_at: 10_000,
    }
}
fn recovery_accept(i: lifecycle::Intent) -> Control {
    let approval = orders::Approval {
        account: user(1),
        intent_hash: i.digest().unwrap(),
        authority_epoch: 7,
    };
    Control::Funds(lifecycle::Action::RecoveryAccept {
        intent: Box::new(i),
        approval,
    })
}
#[test]
fn frozen_recovery_return_uses_the_incremented_epoch_and_never_reopens_normal_paths() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    prepare(&mut j, 10, Rail::Release, 20, 0);
    let plan = expose(&mut j, &c, 10, Rail::Release, 60);
    c.observe_chain(&mut j, id(60), 100, chain(&plan, 60))
        .unwrap();
    begin_recovery(&mut j);
    let i = recovery_intent(11, Location::Broker, Destination::Location(Location::Vault));
    let approval = orders::Approval {
        account: user(1),
        intent_hash: i.digest().unwrap(),
        authority_epoch: 7,
    };
    let ordinary = tx(
        &j,
        9,
        vec![],
        vec![Control::Funds(lifecycle::Action::Accept {
            intent: Box::new(i.clone()),
            approval,
        })],
    );
    assert!(j.commit(ordinary).unwrap().receipt.controls.is_some());
    let prepare = tx(
        &j,
        11,
        vec![],
        vec![
            recovery_accept(i),
            Control::Funds(lifecycle::Action::Prepare {
                attempt: a(11),
                net: atoms(20),
            }),
        ],
    );
    assert_eq!(j.commit(prepare).unwrap().receipt.controls, None);
    let plan = expose(&mut j, &c, 11, Rail::Return, 61);
    assert_eq!(plan.epoch(), 2);
    let seal = tx(
        &j,
        12,
        vec![],
        vec![Control::Recovery(cinder_journal::recovery::Action::Seal {
            expected_version: j.state().unwrap().ledger().version(),
        })],
    );
    assert!(j.commit(seal).unwrap().receipt.controls.is_some()); // Unknown return cannot seal.
    c.observe_chain(&mut j, id(61), 100, chain(&plan, 61))
        .unwrap();
    let (head, check) = recovery_cut(&mut j);
    assert_eq!(
        c.recovery_backing(&mut j, head, check.clone(), &custody(0, 0, 1, 120))
            .unwrap()
            .total(),
        20
    );
    let seal = tx(
        &j,
        13,
        vec![],
        vec![Control::Recovery(cinder_journal::recovery::Action::Seal {
            expected_version: j.state().unwrap().ledger().version(),
        })],
    );
    assert_eq!(j.commit(seal).unwrap().receipt.controls, None);
    assert!(j.state().unwrap().frozen());
    assert!(j.state().unwrap().recovery_sealed());
    let sealed = j.head();
    assert!(
        c.recovery_backing(&mut j, head, check.clone(), &custody(0, 0, 1, 120))
            .is_err()
    );
    assert_eq!(
        c.recovery_backing(&mut j, sealed, check, &custody(0, 0, 1, 120))
            .unwrap()
            .total(),
        20
    );
    let expected = j.state().unwrap().clone();
    drop(j);
    assert_eq!(open(&t).state().unwrap(), &expected);
}
#[test]
fn recovery_permissions_cannot_authorize_release_payout_or_an_old_prepared_return() {
    let t = Temp::new();
    let (mut j, _, _) = setup(&t, true);
    begin_recovery(&mut j);
    for (n, source, destination) in [
        (10, Location::Vault, Destination::Recipient([11; 32])),
        (11, Location::Vault, Destination::Location(Location::Broker)),
        (12, Location::Broker, Destination::Location(Location::Venue)),
    ] {
        let t = tx(
            &j,
            n,
            vec![],
            vec![recovery_accept(recovery_intent(n, source, destination))],
        );
        assert!(j.commit(t).unwrap().receipt.controls.is_some());
    }
    assert!(j.state().unwrap().funds().is_empty());
    let t = Temp::new();
    let (mut j, _, _) = setup(&t, true);
    prepare(&mut j, 10, Rail::Withdraw, 20, 0);
    begin_recovery(&mut j);
    let t = tx(&j, 11, vec![], vec![Control::Expose(a(10))]);
    assert!(j.commit(t).unwrap().receipt.controls.is_some());
    assert!(!j.state().unwrap().attempts()[0].possibly_exposed);
}
#[test]
fn recovery_permission_is_one_way_and_expiry_cannot_authorize_a_return() {
    let t = Temp::new();
    let (mut j, _, _) = setup(&t, true);
    begin_recovery(&mut j);
    let begin = Control::Recovery(cinder_journal::recovery::Action::Begin {
        expected_version: j.state().unwrap().ledger().version(),
        authority_epoch: 8,
        valid_until: 30_000,
        fence: [98; 32],
    });
    let t = tx(&j, 10, vec![], vec![begin]);
    assert!(j.commit(t).unwrap().receipt.controls.is_some());
    let mut t = tx(
        &j,
        11,
        vec![],
        vec![recovery_accept(recovery_intent(
            11,
            Location::Venue,
            Destination::Location(Location::Broker),
        ))],
    );
    t.at = 20_000;
    assert!(j.commit(t).unwrap().receipt.controls.is_some());
    assert_eq!(j.state().unwrap().recovery_epoch(), None);
    assert!(j.state().unwrap().frozen());
}
fn custody(paid: u64, sequence: u64, funding_sequence: u64, vault_amount: u64) -> CustodyState {
    let r = route();
    CustodyState {
        network: config().domain.network.bytes(),
        program: r.program,
        config: r.config,
        vault: r.vault,
        mint: r.mint,
        domain: r.domain,
        pool: r.pool,
        broker: r.broker,
        broker_tokens: r.broker_tokens,
        decimals: r.decimals,
        epoch: r.epoch + 1,
        mode: 1,
        normal_paid: paid,
        funding_sequence,
        vault_amount,
        finalized_slot: 200,
        customers: vec![
            CustomerState {
                wallet: r.beneficiaries[0].wallet,
                tokens: r.beneficiaries[0].tokens,
                paid,
                payout_sequence: sequence,
            },
            CustomerState {
                wallet: r.beneficiaries[1].wallet,
                tokens: r.beneficiaries[1].tokens,
                paid: 0,
                payout_sequence: 0,
            },
        ],
    }
}

#[test]
fn recovery_backing_binds_real_p16_payout_history_without_a_second_deduction() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    prepare(&mut j, 10, Rail::Payout, 5, 0);
    let plan = expose(&mut j, &c, 10, Rail::Payout, 60);
    c.observe_chain(&mut j, id(60), 100, chain(&plan, 60))
        .unwrap();
    // A repeated authenticated receipt is evidence, not a second paid event.
    c.observe_chain(&mut j, id(61), 100, chain(&plan, 60))
        .unwrap();
    let (head, check) = recovery_cut(&mut j);
    let observation = custody(5, 1, 0, 115);
    let matched = c
        .recovery_backing(&mut j, head, check.clone(), &observation)
        .unwrap();
    assert_eq!(matched.total(), 15);
    assert_eq!(matched.normal_paid(), 5);
    assert_eq!(matched.epoch(), 2);
    assert_eq!(matched.funding_sequence(), 0);
    assert_eq!(matched.finalized_slot(), 200);
    assert_eq!(matched.cut().accounts().len(), 2);
    assert_eq!(matched.claims().len(), 1);
    let claim = &matched.claims()[0];
    assert_eq!(claim.account(), user(1));
    assert_eq!(claim.wallet(), [11; 32]);
    assert_eq!(claim.tokens(), [34; 32]);
    assert_eq!(claim.amount(), 15);
    assert_eq!(claim.paid_base(), 5);
    assert_eq!(claim.payout_sequence_base(), 1);
    assert_eq!(format!("{matched:?}"), "RecoveryBacking([PRIVATE])");
    assert_eq!(format!("{claim:?}"), "ClaimBasis([PRIVATE])");
    assert_eq!(format!("{observation:?}"), "CustodyState([PRIVATE])");
    drop(j);
    let mut reopened = open(&t);
    assert_eq!(
        c.recovery_backing(&mut reopened, head, check, &observation)
            .unwrap(),
        matched
    );
}

#[test]
fn recovery_custody_context_inventory_counters_and_coherent_slot_are_exact() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    let (head, check) = recovery_cut(&mut j);
    let original = custody(0, 0, 0, 120);
    assert!(
        c.recovery_backing(&mut j, head, check.clone(), &original)
            .is_ok()
    );
    for variant in 0..22 {
        let mut changed = original.clone();
        match variant {
            0 => changed.network = [99; 32],
            1 => changed.program = [99; 32],
            2 => changed.config = [99; 32],
            3 => changed.vault = [99; 32],
            4 => changed.mint = [99; 32],
            5 => changed.domain = [99; 32],
            6 => changed.pool = [99; 32],
            7 => changed.broker = [99; 32],
            8 => changed.broker_tokens = [99; 32],
            9 => changed.decimals = 6,
            10 => changed.epoch = 1,
            11 => changed.mode = 0,
            12 => changed.mode = 3,
            13 => changed.normal_paid = 1,
            14 => changed.funding_sequence = 1,
            15 => changed.vault_amount = 119,
            16 => changed.vault_amount = 121,
            17 => changed.finalized_slot = 99,
            18 => {
                changed.customers.pop();
            }
            19 => changed.customers[1] = changed.customers[0].clone(),
            20 => changed.customers[0].tokens = [99; 32],
            _ => changed.customers[0].payout_sequence = 1,
        }
        assert_eq!(
            c.recovery_backing(&mut j, head, check.clone(), &changed),
            Err(BindingError::Mismatch),
            "variant {variant}"
        );
    }
    let mut reordered = original;
    reordered.customers.reverse();
    assert!(
        c.recovery_backing(&mut j, head, check.clone(), &reordered)
            .is_ok()
    );
    let extra = tx(&j, 212, vec![], vec![]);
    j.commit(extra).unwrap();
    assert_eq!(
        c.recovery_backing(&mut j, head, check, &reordered),
        Err(BindingError::Cut(cinder_journal::recovery::CutError::Stale))
    );
}

#[test]
fn returned_working_capital_keeps_its_funding_sequence_in_recovery() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    prepare(&mut j, 10, Rail::Release, 20, 0);
    let plan = expose(&mut j, &c, 10, Rail::Release, 60);
    c.observe_chain(&mut j, id(60), 100, chain(&plan, 60))
        .unwrap();
    prepare(&mut j, 11, Rail::Return, 20, 0);
    let plan = expose(&mut j, &c, 11, Rail::Return, 61);
    c.observe_chain(&mut j, id(61), 100, chain(&plan, 61))
        .unwrap();
    let (head, check) = recovery_cut(&mut j);
    let state = custody(0, 0, 1, 120);
    assert_eq!(
        c.recovery_backing(&mut j, head, check.clone(), &state)
            .unwrap()
            .funding_sequence(),
        1
    );
    let mut changed = state.clone();
    changed.funding_sequence = 0;
    assert_eq!(
        c.recovery_backing(&mut j, head, check.clone(), &changed),
        Err(BindingError::Mismatch)
    );
    changed = state;
    changed.finalized_slot = 199;
    assert_eq!(
        c.recovery_backing(&mut j, head, check, &changed),
        Err(BindingError::Mismatch)
    );
}

#[test]
fn recovery_rejects_a_program_counter_jump_not_explained_by_accepted_history() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    prepare(&mut j, 10, Rail::Payout, 5, 0);
    let action = c
        .expose_chain(
            &mut j,
            Dispatch {
                attempt: a(10),
                commit: id(40),
                at: 100,
            },
            Rail::Payout,
            counters(Rail::Payout, 0, 5),
        )
        .unwrap();
    let plan = signed_fixture(&c, &mut j, action, 60);
    c.observe_chain(&mut j, id(60), 100, chain(&plan, 60))
        .unwrap();
    let (head, check) = recovery_cut(&mut j);
    assert_eq!(
        c.recovery_backing(&mut j, head, check, &custody(5, 6, 0, 115)),
        Err(BindingError::Mismatch)
    );
}

#[test]
fn recovery_cannot_truncate_a_journal_amount_to_program_mint_atoms() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    let extra = tx(
        &j,
        209,
        vec![(
            Location::Vault,
            Change::Receipt {
                owner: Owner::House,
                location: Location::Vault,
                amount: atoms(i128::from(u64::MAX)),
            },
        )],
        vec![],
    );
    assert_eq!(
        j.commit(extra).unwrap().receipt.inputs,
        [InputResult::Normalized(Disposition::Applied)]
    );
    let (head, check) = recovery_cut(&mut j);
    assert_eq!(
        c.recovery_backing(&mut j, head, check, &custody(0, 0, 0, u64::MAX)),
        Err(BindingError::Precision)
    );
}

#[test]
fn recovery_uses_the_qualified_raw_resolution_cut_not_an_absent_original_cut() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    let mut unknown = tx(&j, 207, vec![], vec![]);
    unknown.inputs.push(Input {
        source: source(Location::Vault),
        source_cut: None,
        authority_epoch: 1,
        observed_at: 100,
        raw: raw(b"synthetic initially unknown chain input"),
        event: None,
    });
    assert_eq!(
        j.commit(unknown).unwrap().receipt.inputs,
        [InputResult::Unnormalized]
    );
    let entry = &j.state().unwrap().raw_inputs()[0];
    let proof = raw(b"synthetic authenticated no-effect chain history through slot 250");
    let resolution = cinder_journal::raw::Resolution {
        key: entry.key,
        fingerprint: entry.fingerprint,
        source: entry.source,
        authority_epoch: 1,
        through: 250,
        observed_at: 100,
        evidence: Sha256::digest(proof.as_bytes()).into(),
        no_effect: true,
        effects: vec![],
    };
    let mut resolved = tx(&j, 208, vec![], vec![Control::ResolveRaw(resolution)]);
    resolved.evidence.push(proof);
    assert_eq!(j.commit(resolved).unwrap().receipt.controls, None);
    let (head, check) = recovery_cut(&mut j);
    let mut observation = custody(0, 0, 0, 120);
    assert_eq!(
        c.recovery_backing(&mut j, head, check.clone(), &observation),
        Err(BindingError::Mismatch)
    );
    observation.finalized_slot = 250;
    assert!(
        c.recovery_backing(&mut j, head, check, &observation)
            .is_ok()
    );
}
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use support::{FixtureProtection, Store, Temp, raw, user};
use zeroize::Zeroizing;

#[test]
fn marginal_forecast_counts_prepared_and_partial_ingress_once_without_trading_credit() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    assert_eq!(c.funding_need(&mut j, 100, 20).unwrap().additional, 20);
    prepare(&mut j, 10, Rail::Release, 20, 0);
    assert_eq!(c.funding_need(&mut j, 100, 20).unwrap().pending_ingress, 20);
    let p = expose(&mut j, &c, 10, Rail::Release, 60);
    c.observe_chain(&mut j, id(60), 100, chain(&p, 60)).unwrap();
    let need = c.funding_need(&mut j, 100, 20).unwrap();
    assert_eq!(
        (need.broker_usable, need.pending_ingress, need.additional),
        (20, 0, 0)
    );
    prepare(&mut j, 11, Rail::Deposit, 20, 0);
    let need = c.funding_need(&mut j, 100, 20).unwrap();
    assert_eq!(
        (need.broker_usable, need.pending_ingress, need.additional),
        (0, 20, 0)
    );
    let p = expose(&mut j, &c, 11, Rail::Deposit, 61);
    c.observe_chain(&mut j, id(61), 100, chain(&p, 61)).unwrap();
    credit(&c, &mut j, 62, 8, false);
    let need = c.funding_need(&mut j, 100, 20).unwrap();
    assert_eq!(
        (need.venue_usable, need.pending_ingress, need.additional),
        (8, 12, 0)
    );
    let intent = orders::Intent {
        request: a(20).request,
        quantity: support::q(1),
        minimum: support::p(90),
        maximum: support::p(110),
        maximum_fee_per_lot: atoms(0),
        reduce_only: false,
        time_in_force: orders::TimeInForce::GoodTilCancelled,
        policy: config().policy,
        authority_epoch: 1,
        expires_at: 1000,
    };
    let approval = orders::Approval {
        account: user(1),
        intent_hash: intent.digest().unwrap(),
        authority_epoch: 1,
    };
    let t = tx(
        &j,
        70,
        vec![],
        vec![Control::Order(orders::Action::Accept {
            intent: Box::new(intent),
            approval,
            reservations: vec![],
        })],
    );
    assert_eq!(
        j.commit(t).unwrap().receipt.controls,
        Some(ControlError::Unqualified)
    );
    credit(&c, &mut j, 63, 12, true);
    let need = c.funding_need(&mut j, 100, 20).unwrap();
    assert_eq!(
        (need.venue_usable, need.pending_ingress, need.additional),
        (20, 0, 0)
    );
    assert!(j.state().unwrap().native_funding_ready());
    bridge(&j);
}

#[test]
fn exact_wire_must_be_durable_and_cannot_be_replaced_after_restart() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    prepare(&mut j, 10, Rail::Release, 20, 0);
    let action = c
        .expose_chain(
            &mut j,
            Dispatch {
                attempt: a(10),
                commit: id(40),
                at: 100,
            },
            Rail::Release,
            counters(Rail::Release, 0, 0),
        )
        .unwrap();
    let p = action.plan().clone();
    assert!(c.observe_chain(&mut j, id(60), 100, chain(&p, 60)).is_err());
    let binding = Sha256::digest(
        c.original_chain_contract(&mut j, a(10), 100)
            .unwrap()
            .as_bytes(),
    )
    .into();
    let good = VerifiedWire {
        attempt: a(10),
        binding,
        signature: [60; 64],
        wire: raw(b"verified synthetic original wire"),
    };
    c.persist_wire(&mut j, id(80), 100, action, good).unwrap();
    drop(j);
    let mut j = open(&t);
    assert!(
        c.expose_chain(
            &mut j,
            Dispatch {
                attempt: a(10),
                commit: id(81),
                at: 100
            },
            Rail::Release,
            counters(Rail::Release, 0, 0)
        )
        .is_err()
    );
    assert!(c.observe_chain(&mut j, id(61), 100, chain(&p, 61)).is_err());
    c.observe_chain(&mut j, id(60), 100, chain(&p, 60)).unwrap();
    bridge(&j);
}

struct KillTransport {
    marker: std::path::PathBuf,
}
#[test]
fn original_chain_capability_rechecks_grant_freeze_setup_and_wire_binding() {
    for case in 0..4 {
        let t = Temp::new();
        let (mut j, c, _) = setup(&t, false);
        prepare(&mut j, 10, Rail::Release, 20, 0);
        let action = c
            .expose_chain(
                &mut j,
                Dispatch {
                    attempt: a(10),
                    commit: id(40),
                    at: 100,
                },
                Rail::Release,
                counters(Rail::Release, 0, 0),
            )
            .unwrap();
        let plan = action.plan().clone();
        let binding = Sha256::digest(action.encode().unwrap().as_bytes()).into();
        let mut wire = VerifiedWire {
            attempt: a(10),
            binding,
            signature: [60; 64],
            wire: raw(b"synthetic original signed wire"),
        };
        match case {
            0 => {
                let tx = tx(
                    &j,
                    50,
                    vec![],
                    vec![Control::Order(orders::Action::AdvanceAuthority {
                        account: user(1),
                        epoch: 2,
                    })],
                );
                assert_eq!(j.commit(tx).unwrap().receipt.controls, None);
            }
            1 => {
                let tx = tx(
                    &j,
                    50,
                    vec![],
                    vec![Control::Funds(lifecycle::Action::Freeze)],
                );
                assert_eq!(j.commit(tx).unwrap().receipt.controls, None);
            }
            2 => {
                let mut s = good_setup();
                s.lending_disabled = false;
                assert!(!c.observe_setup(&mut j, id(50), 100, s).unwrap());
            }
            _ => wire.binding = [1; 32],
        }
        assert!(c.persist_wire(&mut j, id(80), 100, action, wire).is_err());
        assert!(j.transaction(id(80)).is_none());
        assert!(
            c.observe_chain(&mut j, id(60), 100, chain(&plan, 60))
                .is_err()
        );
        assert_eq!(j.state().unwrap().ledger().vault(), atoms(120));
        bridge(&j);
    }
}
impl Transport for KillTransport {
    fn post(&mut self, request: Outbound) -> Reply {
        // Validate the signature just like normal tests, then stop at the exact
        // external-effect boundary before any reply can be journaled.
        Fake::default().post(request);
        std::fs::write(&self.marker, b"one POST exposed").unwrap();
        loop {
            std::thread::park();
        }
    }
}
#[test]
#[ignore = "invoked only by bounded killed-child test"]
fn funding_post_child() {
    let db = std::env::var_os("CINDER_P16_TEST_DB").expect("parent-owned disposable path");
    let marker = std::env::var_os("CINDER_P16_TEST_MARKER").expect("parent-owned marker");
    let mut j = Journal::open(
        SqliteBackend::open(std::path::Path::new(&db), Migration::None).unwrap(),
        FixtureProtection,
        config(),
    )
    .unwrap();
    controller()
        .withdraw(
            &mut j,
            &gateway(),
            Dispatch {
                attempt: a(10),
                commit: id(40),
                at: 100,
            },
            &mut KillTransport {
                marker: marker.into(),
            },
        )
        .unwrap();
}
#[test]
fn kill_after_withdrawal_post_before_reply_reconciles_original_without_second_send() {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let t = Temp::new();
    let (mut j, c, g) = setup(&t, true);
    prepare(&mut j, 10, Rail::Withdraw, 19, 1);
    drop(j);
    let marker = t.root.join("post-exposed");
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut child = Child(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "funding_post_child", "--ignored", "--nocapture"])
            .env("CINDER_P16_TEST_DB", &t.db)
            .env("CINDER_P16_TEST_MARKER", &marker)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    let started = Instant::now();
    while !marker.exists() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "child POST boundary timeout"
        );
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "child exited before POST exposure"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    drop(child);
    let mut j = open(&t);
    assert!(
        j.state()
            .unwrap()
            .attempts()
            .iter()
            .any(|entry| entry.key == a(10) && entry.possibly_exposed)
    );
    assert!(
        j.state()
            .unwrap()
            .holds()
            .iter()
            .any(|h| h.request == a(10).request && h.active)
    );
    let mut f = Fake::default();
    assert!(
        c.withdraw(
            &mut j,
            &g,
            Dispatch {
                attempt: a(10),
                commit: id(41),
                at: 101
            },
            &mut f
        )
        .is_err()
    );
    assert_eq!(f.calls, 0);
    c.observe_withdrawal(&mut j, id(60), 101, withdrawal(10, 20, 19))
        .unwrap();
    assert_eq!(j.state().unwrap().ledger().broker(), atoms(19));
    c.observe_withdrawal(&mut j, id(61), 101, withdrawal(10, 20, 19))
        .unwrap();
    assert_eq!(j.state().unwrap().ledger().broker(), atoms(19));
    bridge(&j);
}
#[test]
fn broker_risk_requires_explicit_third_location_and_separate_accessibility() {
    use cinder_journal::risk;
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    prepare(&mut j, 10, Rail::Release, 20, 0);
    let p = expose(&mut j, &c, 10, Rail::Release, 60);
    c.observe_chain(&mut j, id(60), 100, chain(&p, 60)).unwrap();
    let locations = |broker: bool| {
        let mut locations = vec![Location::Vault, Location::Venue];
        if broker {
            locations.push(Location::Broker)
        };
        locations
            .into_iter()
            .map(|location| risk::Liquidity {
                location,
                accessible_bps: 10000,
                due: atoms(0),
            })
            .collect()
    };
    let mut policy = risk::Policy {
        revision: config().policy,
        markets: vec![risk::MarketRule {
            market: config().markets[0].unit(),
            maximum_leverage: 100000,
            maintenance_bps: 500,
            native_maintenance_bps: 500,
            gross_limit: atoms(1000),
            net_limit: atoms(1000),
        }],
        buffer: atoms(0),
        horizon_ms: 100,
        valid_until: 1000,
        paths: vec![risk::Path {
            id: [1; 32],
            steps: vec![risk::Step {
                after_ms: 1,
                marks: vec![support::p(100)],
                events: vec![],
                liquidity: locations(false),
            }],
        }],
    };
    let install = |j: &Store, n, p: risk::Policy| {
        tx(
            j,
            n,
            vec![],
            vec![Control::Risk(risk::Action::Install {
                expected_version: j.state().unwrap().ledger().version(),
                policy: Box::new(p),
            })],
        )
    };
    let tx = install(&j, 70, policy.clone());
    assert_eq!(
        j.commit(tx).unwrap().receipt.controls,
        Some(ControlError::Invalid)
    );
    policy.paths[0].steps[0].liquidity = locations(true);
    let tx = install(&j, 71, policy.clone());
    assert_eq!(j.commit(tx).unwrap().receipt.controls, None);
    let report = j.state().unwrap().risk_report().unwrap();
    assert_eq!(report.current.broker_free, atoms(20));
    assert_eq!(report.current.venue_free, atoms(0));
    let broker = policy.paths[0].steps[0]
        .liquidity
        .iter_mut()
        .find(|x| x.location == Location::Broker)
        .unwrap();
    broker.accessible_bps = 0;
    broker.due = atoms(1);
    policy.revision = PolicyVersion::new(2).unwrap();
    let tx = install(&j, 72, policy);
    assert_eq!(j.commit(tx).unwrap().receipt.controls, None);
    let report = j.state().unwrap().risk_report().unwrap();
    assert!(!report.admissible);
    assert!(report.paths[0].prefixes[1].flags.illiquid);
    assert_eq!(report.paths[0].prefixes[1].broker_free, atoms(-1));
    bridge(&j);
}

fn config() -> Config {
    let mut c = support::config();
    for (tag, location) in [(8, Location::Vault), (9, Location::Broker)] {
        c.sources.push(Source {
            scope: EventScope {
                namespace: NamespaceId::new([tag; 32]).unwrap(),
                ..c.sources[0].scope
            },
            location,
        });
    }
    c
}
fn account() -> [u8; 32] {
    SigningKey::from_bytes(&[9; 32]).verifying_key().to_bytes()
}
fn profile() -> Profile {
    Profile {
        config: config(),
        source: config().sources[0].scope,
        account: bs58::encode(account()).into_string(),
        environment: Origin::Testnet.url().into(),
        revision: 1,
        evidence: "offline funds fixture".into(),
        precision: Level::Qualified,
        fills: Level::Qualified,
        markets: vec![Mapping {
            symbol: "BTC".into(),
            market: 0,
            size: Grid { places: 0, step: 1 },
            price: Grid { places: 0, step: 1 },
        }],
        quote_places: 0,
        perp_tag: 0,
    }
}
fn route() -> Route {
    Route {
        domain: config().domain.deployment.bytes(),
        pool: [18; 32],
        funds: [19; 32],
        beneficiaries: vec![
            Beneficiary {
                account: user(1).bytes(),
                wallet: [11; 32],
                tokens: [34; 32],
            },
            Beneficiary {
                account: user(2).bytes(),
                wallet: [12; 32],
                tokens: [35; 32],
            },
        ],
        program: [20; 32],
        config: [21; 32],
        vault: [22; 32],
        mint: [23; 32],
        broker: account(),
        broker_tokens: [24; 32],
        venue_program: [25; 32],
        venue_vault: [26; 32],
        epoch: 1,
        decimals: 0,
        withdrawal: Level::Qualified,
        chain: Level::Qualified,
        settings: Level::Qualified,
        withdrawal_cost: 10,
        maximum_movement: 1000,
        maximum_fee: 5,
        setup_max_age: 10_000,
    }
}
fn controller() -> Controller {
    Controller::new(profile(), route(), Zeroizing::new([9; 32])).unwrap()
}

#[test]
fn demo_ingress_keeps_withdrawal_unknown_and_cannot_obtain_strong_readiness() {
    let mut p = profile();
    p.fills = Level::Unknown;
    let mut r = route();
    r.withdrawal = Level::Unknown;
    r.settings = Level::Observed;
    assert!(Controller::new(p.clone(), r.clone(), Zeroizing::new([9; 32])).is_err());
    let c = Controller::new_demo_ingress(p.clone(), r.clone(), Zeroizing::new([9; 32])).unwrap();
    let t = Temp::new();
    let mut j = Journal::create(
        SqliteBackend::create(&t.db).unwrap(),
        FixtureProtection,
        config(),
    )
    .unwrap();
    c.bind(&mut j, id(80), 100).unwrap();
    assert!(
        !c.observe_setup(
            &mut j,
            id(81),
            101,
            Setup {
                account: r.broker,
                observed_at: 101,
                lending_disabled: true,
                borrowed: "0".into(),
                interest: "0".into(),
                complete: true,
            }
        )
        .unwrap()
    );
    assert!(!j.state().unwrap().native_funding_ready());
    for rail in [Rail::Release, Rail::Deposit, Rail::Return, Rail::Payout] {
        let head = j.head();
        assert!(
            c.expose_chain(
                &mut j,
                Dispatch {
                    attempt: a(10),
                    commit: id(82),
                    at: 102
                },
                rail,
                Counters {
                    sequence: 0,
                    paid: 0,
                    recipient_tokens: [0; 32],
                    expires_at_slot: 1000
                }
            )
            .is_err()
        );
        assert_eq!(j.head(), head);
    }
    for field in 0..4 {
        let mut p = p.clone();
        let mut r = r.clone();
        match field {
            0 => r.withdrawal = Level::Qualified,
            1 => r.settings = Level::Qualified,
            2 => r.chain = Level::Unknown,
            _ => p.environment = Origin::Mainnet.url().into(),
        }
        assert!(Controller::new_demo_ingress(p, r, Zeroizing::new([9; 32])).is_err());
    }
}
fn gateway() -> Gateway {
    Gateway::new(
        profile(),
        Policy {
            revision: 1,
            evidence: "offline shared credits".into(),
            execution: Level::Qualified,
            origin: Origin::Testnet,
            expiry_ms: 30_000,
            credits: 100,
            cleanup_reserve: 20,
            read_cost: 10,
        },
        Zeroizing::new([7; 32]),
        1,
    )
    .unwrap()
}
fn id(n: u8) -> CommitId {
    CommitId::new([n; 32]).unwrap()
}
fn a(n: u8) -> AttemptKey {
    support::attempt(n)
}
fn atoms(n: i128) -> QuoteAtoms {
    QuoteAtoms::new(config().quote, n)
}
fn source(location: Location) -> EventScope {
    config()
        .sources
        .into_iter()
        .find(|s| s.location == location)
        .unwrap()
        .scope
}
fn key(n: u8, location: Location) -> EventKey {
    EventKey {
        scope: source(location),
        event: EconomicEventId::new(&[n]).unwrap(),
        leg: 0,
    }
}
fn tx(j: &Store, n: u8, events: Vec<(Location, Change)>, controls: Vec<Control>) -> Transaction {
    Transaction {
        id: id(n),
        expected: j.head(),
        at: 100,
        evidence: vec![],
        inputs: events
            .into_iter()
            .enumerate()
            .map(|(i, (l, c))| {
                let k = EventKey {
                    leg: u32::try_from(i).unwrap(),
                    ..key(n, l)
                };
                Input {
                    source: k.scope,
                    source_cut: Some(100),
                    authority_epoch: 1,
                    observed_at: 100,
                    raw: raw(b"synthetic trusted observation"),
                    event: Some(Event {
                        key: RecordKey::Economic(k),
                        policy: config().policy,
                        change: c,
                    }),
                }
            })
            .collect(),
        order_observations: vec![],
        funds_observations: vec![],
        controls,
    }
}
fn open(t: &Temp) -> Store {
    Journal::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        FixtureProtection,
        config(),
    )
    .unwrap()
}
fn setup(t: &Temp, native: bool) -> (Store, Controller, Gateway) {
    setup_with_gateway(t, native, gateway())
}
fn setup_with_gateway(t: &Temp, native: bool, g: Gateway) -> (Store, Controller, Gateway) {
    let mut j = Journal::create(
        SqliteBackend::create(&t.db).unwrap(),
        FixtureProtection,
        config(),
    )
    .unwrap();
    let c = controller();
    let customer_location = if native {
        Location::Venue
    } else {
        Location::Vault
    };
    let t = tx(
        &j,
        1,
        vec![
            (
                customer_location,
                Change::Receipt {
                    owner: Owner::Customer(user(1)),
                    location: customer_location,
                    amount: atoms(20),
                },
            ),
            (
                Location::Vault,
                Change::Receipt {
                    owner: Owner::House,
                    location: Location::Vault,
                    amount: atoms(100),
                },
            ),
        ],
        vec![Control::Order(orders::Action::AdvanceAuthority {
            account: user(1),
            epoch: 1,
        })],
    );
    assert_eq!(j.commit(t).unwrap().receipt.controls, None);
    let l = j.state().unwrap().ledger();
    let version = l.version();
    let check = NativeCheck {
        expected_version: version,
        cash: Some(l.venue().cash()),
        funding: Some(l.venue().funding()),
        positions: Some(l.venue().positions().to_vec()),
        complete: true,
        resolves: vec![],
    };
    let cut = collateral::Cut {
        expected_version: version + 1,
        policy: collateral::Policy {
            revision: config().policy,
            markets: vec![collateral::MarginRule {
                market: config().markets[0].unit(),
                private_bps: 1000,
                native_bps: 1000,
            }],
            evidence: EvidencePolicy {
                max_issue_age: 10_000,
                max_mark_age: 10_000,
                max_check_age: 10_000,
            },
        },
        marks: vec![MarkObservation {
            price: support::p(100),
            evidence: key(2, Location::Venue),
            observed_at: 100,
            valid_until: 10_100,
            qualified: true,
        }],
    };
    let t = tx(
        &j,
        2,
        vec![(Location::Venue, Change::Reconcile(check))],
        vec![Control::Collateral(cut)],
    );
    assert_eq!(j.commit(t).unwrap().receipt.controls, None);
    c.bind(&mut j, id(3), 100).unwrap();
    g.activate(&mut j, id(4), 100).unwrap();
    assert!(c.observe_setup(&mut j, id(5), 100, good_setup()).unwrap());
    (j, c, g)
}
fn good_setup() -> Setup {
    Setup {
        account: account(),
        observed_at: 100,
        lending_disabled: true,
        borrowed: "0".into(),
        interest: "0".into(),
        complete: true,
    }
}
fn prepare(j: &mut Store, n: u8, rail: Rail, net: i128, fee: i128) {
    let (source, destination) = match rail {
        Rail::Release => (Location::Vault, Destination::Location(Location::Broker)),
        Rail::Deposit => (Location::Broker, Destination::Location(Location::Venue)),
        Rail::Withdraw => (Location::Venue, Destination::Location(Location::Broker)),
        Rail::Return => (Location::Broker, Destination::Location(Location::Vault)),
        Rail::Payout => (Location::Vault, Destination::Recipient([34; 32])),
    };
    let i = lifecycle::Intent {
        request: a(n).request,
        source,
        destination,
        net: atoms(net),
        maximum_fee: atoms(fee),
        fee_payer: Owner::House,
        allow_partial: false,
        policy: config().policy,
        authority_epoch: 1,
        expires_at: 10_000,
    };
    let approval = orders::Approval {
        account: user(1),
        intent_hash: i.digest().unwrap(),
        authority_epoch: 1,
    };
    let t = tx(
        j,
        n,
        vec![],
        vec![
            Control::Funds(lifecycle::Action::Accept {
                intent: Box::new(i),
                approval,
            }),
            Control::Funds(lifecycle::Action::Prepare {
                attempt: a(n),
                net: atoms(net),
            }),
        ],
    );
    assert_eq!(j.commit(t).unwrap().receipt.controls, None);
}
fn counters(rail: Rail, paid: u64, sequence: u64) -> Counters {
    Counters {
        paid,
        sequence,
        recipient_tokens: if rail == Rail::Payout {
            [34; 32]
        } else {
            [0; 32]
        },
        expires_at_slot: 500,
    }
}
fn expose(j: &mut Store, c: &Controller, n: u8, rail: Rail, signature: u8) -> Plan {
    let action = c
        .expose_chain(
            j,
            Dispatch {
                attempt: a(n),
                commit: id(n + 30),
                at: 100,
            },
            rail,
            counters(rail, 0, 0),
        )
        .unwrap();
    signed_fixture(c, j, action, signature)
}
fn chain(p: &Plan, n: u8) -> ChainReceipt {
    let r = route();
    let (program, from, to) = match p.rail() {
        Rail::Release => (r.program, r.vault, r.broker_tokens),
        Rail::Deposit => (r.venue_program, r.broker_tokens, r.venue_vault),
        Rail::Return => (r.program, r.broker_tokens, r.vault),
        Rail::Payout => (r.program, r.vault, p.counters().recipient_tokens),
        Rail::Withdraw => (r.venue_program, r.venue_vault, r.broker_tokens),
    };
    ChainReceipt {
        network: config().domain.network.bytes(),
        attempt: p.attempt().unwrap(),
        mint: r.mint,
        program,
        source: from,
        destination: to,
        amount: p.amount(),
        succeeded: true,
        signature: [n; 64],
        slot: 200,
        operation: if p.rail() == Rail::Deposit {
            None
        } else {
            Some(p.operation())
        },
        epoch: Some(p.epoch()),
        config: Some(r.config),
        paid: if p.rail() == Rail::Payout {
            Some(p.counters().paid + p.amount())
        } else {
            None
        },
        sequence: Some(p.counters().sequence + 1),
        raw: raw(b"finalized synthetic token/program receipt"),
    }
}
// Only a synthetic trusted codec port here. Real signed Solana wires are checked
// independently by the TypeScript client tests; this fixture is NOT crypto proof.
fn signed_fixture(c: &Controller, j: &mut Store, action: ChainAction, signature: u8) -> Plan {
    let plan = action.plan().clone();
    let contract = action.encode().unwrap();
    let wire = VerifiedWire {
        attempt: plan.attempt().unwrap(),
        binding: Sha256::digest(contract.as_bytes()).into(),
        signature: [signature; 64],
        wire: raw(b"synthetic codec-verified wire"),
    };
    let mut h = Sha256::new();
    h.update(b"synthetic wire commit");
    h.update(plan.attempt().unwrap().encode());
    let wire_id = CommitId::new(h.finalize().into()).unwrap();
    c.persist_wire(j, wire_id, 100, action, wire).unwrap();
    plan
}
fn bridge(j: &Store) {
    let l = j.state().unwrap().ledger();
    l.check_bridge().unwrap();
    let d = l.diagnostics(&[support::p(100)]).unwrap();
    assert_eq!(
        d.net_assets,
        d.customer_claims.checked_add(d.house_equity).unwrap()
    );
}
fn credit(c: &Controller, j: &mut Store, n: u8, amount: u64, final_credit: bool) {
    c.observe_credit(
        j,
        id(n),
        100,
        Credit {
            attempt: a(11),
            account: account(),
            deposit_signature: [61; 64],
            event: EconomicEventId::new(&[n]).unwrap(),
            cut: 300,
            amount,
            fee: 0,
            final_credit,
            raw: raw(b"operation-linked native deposit credit"),
        },
    )
    .unwrap();
}
#[derive(Default)]
struct Fake {
    calls: usize,
    status: Option<u16>,
    body: Option<Vec<u8>>,
    received_at: Option<u64>,
}
impl Transport for Fake {
    fn post(&mut self, r: Outbound) -> Reply {
        self.calls += 1;
        assert_eq!(r.origin(), Origin::Testnet);
        assert_eq!(r.path(), "/api/v1/account/withdraw");
        let mut b: Value = serde_json::from_slice(r.body()).unwrap();
        let signature: [u8; 64] = bs58::decode(b["signature"].as_str().unwrap())
            .into_vec()
            .unwrap()
            .try_into()
            .unwrap();
        let preimage = json!({"data":{"amount":b["amount"].take(),"idempotency_key":b["idempotency_key"].take()},"expiry_window":b["expiry_window"],"timestamp":b["timestamp"],"type":"withdraw"});
        VerifyingKey::from_bytes(&account())
            .unwrap()
            .verify_strict(
                serde_json::to_string(&preimage).unwrap().as_bytes(),
                &Signature::from_bytes(&signature),
            )
            .unwrap();
        assert_eq!(b["account"], bs58::encode(account()).into_string());
        assert!(b.get("agent_wallet").is_none());
        match self.status {
            Some(status) => Reply::Response {
                status,
                body: raw(self.body.as_deref().unwrap_or(
                    br#"{"success":true,"data":{"batch_nonce":42,"requested_amount":"20","fee_amount":"1"}}"#,
                )),
                received_at: self.received_at.unwrap_or(100),
                retry_after_ms: None,
            },
            None => Reply::Unknown,
        }
    }
}
fn withdraw(c: &Controller, j: &mut Store, g: &Gateway, n: u8, fake: &mut Fake) {
    c.withdraw(
        j,
        g,
        Dispatch {
            attempt: a(n),
            commit: id(n + 30),
            at: 100,
        },
        fake,
    )
    .unwrap();
}
fn payment(n: u8, net: u64) -> ChainReceipt {
    ChainReceipt {
        network: config().domain.network.bytes(),
        attempt: a(n),
        mint: route().mint,
        program: route().venue_program,
        source: route().venue_vault,
        destination: route().broker_tokens,
        amount: net,
        succeeded: true,
        signature: [77; 64],
        slot: 400,
        operation: None,
        epoch: None,
        config: None,
        paid: None,
        sequence: None,
        raw: raw(b"finalized owner-wallet transfer"),
    }
}
fn withdrawal(n: u8, gross: u64, net: u64) -> Withdrawal {
    Withdrawal {
        attempt: a(n),
        idempotency_key: withdrawal_id(a(n)),
        event: EconomicEventId::new(&[88]).unwrap(),
        complete: true,
        cut: 350,
        gross,
        payment: payment(n, net),
        raw: raw(b"qualified complete original-intent withdrawal history"),
    }
}
fn transfer_body(batch: u64, requested: &str, amount: &str, fee: &str) -> Value {
    json!({"channel":"account_transfers","data":{
        "u":profile().account,"e":"withdrawal_confirmed","a":"USDP",
        "am":amount,"ra":requested,"f":fee,"bn":batch,"t":100,
        "tx":bs58::encode([77;64]).into_string()
    }})
}
fn linked_transfer(c: &Controller, j: &mut Store, n: u8, v: &Value) -> bool {
    c.linked_withdrawal_transfer(
        j,
        a(n),
        cinder_pacifica::funding::evidence::TransferMessage {
            quote_symbol: "USDP",
            bytes: &serde_json::to_vec(v).unwrap(),
            received_at: 101,
            maximum_age_ms: 1000,
        },
    )
    .is_ok()
}
#[test]
fn retained_withdrawal_ack_rebuilds_original_batch_after_restart_without_cash_or_resend() {
    let t = Temp::new();
    let (mut j, c, g) = setup(&t, true);
    prepare(&mut j, 10, Rail::Withdraw, 19, 1);
    let mut f = Fake {
        status: Some(200),
        ..Fake::default()
    };
    withdraw(&c, &mut j, &g, 10, &mut f);
    let head = j.head();
    drop(j);
    let mut j = open(&t);
    let ack = c
        .withdrawal_acknowledgment(&mut j, a(10), 101)
        .unwrap()
        .unwrap();
    assert_eq!((ack.batch, ack.requested, ack.fee), (42, 20, 1));
    assert!(linked_transfer(
        &c,
        &mut j,
        10,
        &transfer_body(42, "20", "19", "1")
    ));
    // Fee changes/overruns are still real evidence, not erased by the estimate.
    assert!(linked_transfer(
        &c,
        &mut j,
        10,
        &transfer_body(42, "20", "18", "2")
    ));
    let mut pending = transfer_body(42, "20", "19", "1");
    pending["data"]["e"] = json!("withdrawal_pending");
    pending["data"].as_object_mut().unwrap().remove("tx");
    assert!(linked_transfer(&c, &mut j, 10, &pending));
    assert_eq!(j.head(), head);
    assert_eq!(j.state().unwrap().ledger().venue().cash(), atoms(20));
    assert_eq!(j.state().unwrap().ledger().broker(), atoms(0));
    assert_eq!(
        j.state()
            .unwrap()
            .reserved(Resource::Location(Location::Venue))
            .unwrap(),
        atoms(20)
    );
    assert_eq!(c.residuals(&mut j, 101).unwrap().unresolved, [a(10)]);
    assert!(
        c.withdraw(
            &mut j,
            &g,
            Dispatch {
                attempt: a(10),
                commit: id(91),
                at: 101
            },
            &mut f
        )
        .is_err()
    );
    assert_eq!(f.calls, 1);
    for (field, value) in [
        ("bn", json!(43)),
        ("u", json!(bs58::encode([41; 32]).into_string())),
        ("a", json!("USDC")),
        ("t", json!(99)),
        ("ra", json!("19")),
        ("e", json!("deposit")),
    ] {
        let mut v = transfer_body(42, "20", "19", "1");
        v["data"][field] = value;
        assert!(!linked_transfer(&c, &mut j, 10, &v), "accepted {field}");
    }
    assert!(c.withdrawal_acknowledgment(&mut j, a(11), 101).is_err());
}
#[test]
fn split_withdrawal_retains_original_ack_after_later_writer_and_key_revocation() {
    let t = Temp::new();
    let (mut j, c, g) = setup(&t, true);
    prepare(&mut j, 10, Rail::Withdraw, 19, 1);
    let dispatch = Dispatch {
        attempt: a(10),
        commit: id(40),
        at: 100,
    };
    let (request, completion) = c.prepare_withdrawal(&mut j, &g, dispatch).unwrap();
    assert_eq!(request.expires_at(), 10_000);
    let before = j.state().unwrap().ledger().clone();
    g.deactivate(&mut j, id(41), 150).unwrap();
    let mut update = tx(&j, 42, vec![], vec![]);
    update.at = 160;
    j.commit(update).unwrap();
    let mut f = Fake {
        status: Some(200),
        received_at: Some(120),
        ..Fake::default()
    };
    let reply = f.post(request);
    c.complete_withdrawal(&mut j, &g, completion, reply, 170)
        .unwrap();
    assert_eq!(f.calls, 1);
    assert_eq!(j.transactions().last().unwrap().at, 170);
    assert_eq!(j.state().unwrap().ledger(), &before);
    assert_eq!(
        c.withdrawal_acknowledgment(&mut j, a(10), 170)
            .unwrap()
            .unwrap()
            .batch,
        42
    );
    assert!(c.prepare_withdrawal(&mut j, &g, dispatch).is_err());
    let expected = j.state().unwrap().clone();
    drop(j);
    let mut j = open(&t);
    assert_eq!(j.state().unwrap(), &expected);
    assert_eq!(
        c.withdrawal_acknowledgment(&mut j, a(10), 170)
            .unwrap()
            .unwrap()
            .requested,
        20
    );
}
#[test]
fn split_withdrawal_rejects_future_reply_and_different_journal_before_retention() {
    let t = Temp::new();
    let (mut j, c, g) = setup(&t, true);
    prepare(&mut j, 10, Rail::Withdraw, 19, 1);
    let (request, completion) = c
        .prepare_withdrawal(
            &mut j,
            &g,
            Dispatch {
                attempt: a(10),
                commit: id(40),
                at: 100,
            },
        )
        .unwrap();
    let reply = Fake {
        status: Some(200),
        received_at: Some(200),
        ..Fake::default()
    }
    .post(request);
    c.complete_withdrawal(&mut j, &g, completion, reply, 150)
        .unwrap();
    assert!(
        c.withdrawal_acknowledgment(&mut j, a(10), 150)
            .unwrap()
            .is_none()
    );
    let other = Temp::new();
    let (mut wrong, _, _) = setup(&other, true);
    prepare(&mut wrong, 11, Rail::Withdraw, 19, 1);
    let (_, completion) = c
        .prepare_withdrawal(
            &mut wrong,
            &g,
            Dispatch {
                attempt: a(11),
                commit: id(41),
                at: 100,
            },
        )
        .unwrap();
    let head = j.head();
    assert!(
        c.complete_withdrawal(&mut j, &g, completion, Reply::Unknown, 150)
            .is_err()
    );
    assert_eq!(j.head(), head);
}
#[test]
fn deposit_link_requires_original_finalized_chain_signature_and_never_mints_credit() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    prepare(&mut j, 10, Rail::Release, 20, 0);
    let p = expose(&mut j, &c, 10, Rail::Release, 60);
    c.observe_chain(&mut j, id(60), 100, chain(&p, 60)).unwrap();
    prepare(&mut j, 11, Rail::Deposit, 20, 0);
    let p = expose(&mut j, &c, 11, Rail::Deposit, 61);
    let mut body = json!({"channel":"account_transfers","data":{
        "u":profile().account,"e":"deposit","a":"USDP","am":"20","t":100,
        "tx":bs58::encode([61;64]).into_string()
    }});
    let linked = |j: &mut Store, v: &Value| {
        c.linked_deposit_transfer(
            j,
            a(11),
            cinder_pacifica::funding::evidence::TransferMessage {
                quote_symbol: "USDP",
                bytes: &serde_json::to_vec(v).unwrap(),
                received_at: 101,
                maximum_age_ms: 1000,
            },
        )
    };
    assert!(linked(&mut j, &body).is_err());
    c.observe_chain(&mut j, id(61), 100, chain(&p, 61)).unwrap();
    drop(j);
    let mut j = open(&t);
    let head = j.head();
    for amount in ["8", "20"] {
        body["data"]["am"] = json!(amount);
        assert!(linked(&mut j, &body).is_ok());
    }
    for (field, value) in [
        ("tx", json!(bs58::encode([62; 64]).into_string())),
        ("u", json!(bs58::encode([41; 32]).into_string())),
        ("a", json!("USDC")),
        ("am", json!("21")),
        ("t", json!(99)),
    ] {
        let mut changed = body.clone();
        changed["data"][field] = value;
        assert!(linked(&mut j, &changed).is_err(), "accepted {field}");
    }
    assert_eq!(j.head(), head);
    assert_eq!(j.state().unwrap().ledger().venue().cash(), atoms(0));
    assert_eq!(j.state().unwrap().ledger().in_transit().unwrap(), atoms(20));
    assert!(!j.state().unwrap().native_funding_ready());
}
#[test]
fn lost_failed_malformed_or_backdated_native_ack_cannot_be_inferred_from_transfer() {
    for (status, body, received_at) in [
        (None, None, 100),
        (Some(409), None, 100),
        (Some(429), None, 100),
        (Some(500), None, 100),
        (
            Some(200),
            Some(br#"{"success":true,"data":{"batch_nonce":42}}"#.to_vec()),
            100,
        ),
        (Some(200), None, 99),
    ] {
        let t = Temp::new();
        let (mut j, c, g) = setup(&t, true);
        prepare(&mut j, 10, Rail::Withdraw, 19, 1);
        let mut f = Fake {
            status,
            body,
            received_at: Some(received_at),
            ..Fake::default()
        };
        withdraw(&c, &mut j, &g, 10, &mut f);
        drop(j);
        let mut j = open(&t);
        assert!(
            c.withdrawal_acknowledgment(&mut j, a(10), 101)
                .unwrap()
                .is_none()
        );
        assert!(!linked_transfer(
            &c,
            &mut j,
            10,
            &transfer_body(42, "20", "19", "1")
        ));
        assert_eq!(c.residuals(&mut j, 101).unwrap().unresolved, [a(10)]);
        assert_eq!(f.calls, 1);
    }
}
#[test]
fn withdrawal_bindings_refuse_shared_batches_and_changed_gross_but_retain_fee_overrun() {
    let t = Temp::new();
    let (mut j, c, g) = setup(&t, true);
    prepare(&mut j, 10, Rail::Withdraw, 9, 1);
    let response = |requested: &str, fee: &str| {
        serde_json::to_vec(&json!({
            "success":true,"data":{"batch_nonce":42,"requested_amount":requested,"fee_amount":fee}
        }))
        .unwrap()
    };
    withdraw(
        &c,
        &mut j,
        &g,
        10,
        &mut Fake {
            status: Some(200),
            body: Some(response("10", "2")),
            ..Fake::default()
        },
    );
    let ack = c
        .withdrawal_acknowledgment(&mut j, a(10), 100)
        .unwrap()
        .unwrap();
    assert_eq!(ack.fee, 2);
    // The controller serializes exposed fund operations. Complete the first
    // with a separately qualified fixture before testing reuse of its batch.
    c.observe_withdrawal(&mut j, id(60), 100, withdrawal(10, 10, 9))
        .unwrap();
    prepare(&mut j, 11, Rail::Withdraw, 9, 1);
    withdraw(
        &c,
        &mut j,
        &g,
        11,
        &mut Fake {
            status: Some(200),
            body: Some(response("10", "1")),
            ..Fake::default()
        },
    );
    assert!(c.withdrawal_acknowledgment(&mut j, a(10), 100).is_err());
    assert!(c.withdrawal_acknowledgment(&mut j, a(11), 100).is_err());
    assert_eq!(j.state().unwrap().ledger().venue().cash(), atoms(10));
    let t = Temp::new();
    let (mut j, c, g) = setup(&t, true);
    prepare(&mut j, 10, Rail::Withdraw, 19, 1);
    withdraw(
        &c,
        &mut j,
        &g,
        10,
        &mut Fake {
            status: Some(200),
            body: Some(response("19", "1")),
            ..Fake::default()
        },
    );
    assert!(c.withdrawal_acknowledgment(&mut j, a(10), 100).is_err());
}
#[test]
fn opaque_native_reply_cannot_inject_funding_or_gateway_control_records() {
    for header in [3, 4] {
        let t = Temp::new();
        let (mut j, c, g) = setup(&t, true);
        let injected = j.transaction(id(header)).unwrap().evidence[0]
            .as_bytes()
            .to_vec();
        prepare(&mut j, 10, Rail::Withdraw, 19, 1);
        withdraw(
            &c,
            &mut j,
            &g,
            10,
            &mut Fake {
                status: Some(200),
                body: Some(injected),
                ..Fake::default()
            },
        );
        drop(j);
        let mut j = open(&t);
        assert!(c.bound(&mut j, 100).unwrap());
        g.reserve_read(&mut j, id(90), 100, true).unwrap();
        assert!(
            c.withdrawal_acknowledgment(&mut j, a(10), 100)
                .unwrap()
                .is_none()
        );
        assert_eq!(j.state().unwrap().ledger().venue().cash(), atoms(20));
    }
}

#[test]
fn complete_round_trip_uses_one_ledger_and_actual_fees_with_no_second_credit() {
    let t = Temp::new();
    let (mut j, c, g) = setup(&t, false);
    prepare(&mut j, 10, Rail::Release, 20, 0);
    let release = expose(&mut j, &c, 10, Rail::Release, 60);
    c.observe_chain(&mut j, id(60), 100, chain(&release, 60))
        .unwrap();
    bridge(&j);
    assert_eq!(j.state().unwrap().ledger().broker(), atoms(20));
    prepare(&mut j, 11, Rail::Deposit, 20, 0);
    let deposit = expose(&mut j, &c, 11, Rail::Deposit, 61);
    assert!(!j.state().unwrap().native_funding_ready());
    c.observe_chain(&mut j, id(61), 100, chain(&deposit, 61))
        .unwrap();
    bridge(&j);
    assert_eq!(j.state().unwrap().ledger().venue().cash(), atoms(0));
    assert_eq!(j.state().unwrap().ledger().in_transit().unwrap(), atoms(20));
    credit(&c, &mut j, 62, 8, false);
    assert!(!j.state().unwrap().native_funding_ready());
    bridge(&j);
    credit(&c, &mut j, 63, 12, true);
    assert!(j.state().unwrap().native_funding_ready());
    bridge(&j);
    assert_eq!(
        j.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(user(1)))
            .unwrap()
            .cash(),
        atoms(20)
    );
    prepare(&mut j, 12, Rail::Withdraw, 19, 1);
    let mut fake = Fake {
        status: Some(200),
        ..Fake::default()
    };
    withdraw(&c, &mut j, &g, 12, &mut fake);
    assert_eq!(j.state().unwrap().ledger().venue().cash(), atoms(20));
    assert!(!j.state().unwrap().funds().last().unwrap().terminal);
    drop(j);
    let mut j = open(&t);
    assert!(
        c.withdraw(
            &mut j,
            &g,
            Dispatch {
                attempt: a(12),
                commit: id(90),
                at: 100
            },
            &mut fake
        )
        .is_err()
    );
    assert_eq!(fake.calls, 1);
    c.observe_withdrawal(&mut j, id(64), 100, withdrawal(12, 20, 19))
        .unwrap();
    bridge(&j);
    assert_eq!(
        j.state()
            .unwrap()
            .ledger()
            .book(Owner::House)
            .unwrap()
            .cash(),
        atoms(99)
    );
    assert_eq!(
        j.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(user(1)))
            .unwrap()
            .cash(),
        atoms(20)
    );
    prepare(&mut j, 13, Rail::Return, 19, 0);
    let returned = expose(&mut j, &c, 13, Rail::Return, 65);
    c.observe_chain(&mut j, id(65), 100, chain(&returned, 65))
        .unwrap();
    bridge(&j);
    prepare(&mut j, 14, Rail::Payout, 19, 0);
    let payout = expose(&mut j, &c, 14, Rail::Payout, 66);
    assert_eq!(payout.customer(), [11; 32]);
    c.observe_chain(&mut j, id(66), 100, chain(&payout, 66))
        .unwrap();
    bridge(&j);
    assert_eq!(
        j.state().unwrap().ledger().paid(user(1)).unwrap(),
        atoms(19)
    );
    assert_eq!(
        j.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(user(1)))
            .unwrap()
            .cash(),
        atoms(1)
    );
    let residual = c.residuals(&mut j, 100).unwrap();
    assert_eq!(residual.vault, atoms(100));
    assert_eq!(residual.broker, atoms(0));
    assert_eq!(residual.native, atoms(0));
    assert_eq!(residual.transit, atoms(0));
    assert!(residual.unresolved.is_empty());
    c.observe_chain(&mut j, id(67), 100, chain(&payout, 66))
        .unwrap();
    assert_eq!(
        j.state().unwrap().ledger().paid(user(1)).unwrap(),
        atoms(19)
    );
    bridge(&j);
}
#[test]
fn immutable_routes_private_identity_mapping_and_qualified_caps_have_no_fallback() {
    for f in [0, 1, 2, 3, 4, 5, 6, 7] {
        let mut r = route();
        match f {
            0 => r.chain = Level::Observed,
            1 => r.withdrawal = Level::Documented,
            2 => r.settings = Level::Unknown,
            3 => r.broker = [1; 32],
            4 => r.broker_tokens = r.vault,
            5 => r.decimals = 1,
            6 => r.beneficiaries[1].wallet = r.beneficiaries[0].wallet,
            _ => r.maximum_fee = r.maximum_movement,
        };
        assert!(Controller::new(profile(), r, Zeroizing::new([9; 32])).is_err());
    }
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    assert!(c.bind(&mut j, id(50), 100).is_err());
    let mut r = route();
    r.epoch = 2;
    let changed = Controller::new(profile(), r, Zeroizing::new([9; 32])).unwrap();
    assert!(changed.residuals(&mut j, 100).is_err());
}
#[test]
fn missing_lending_disable_debt_interest_and_stale_setup_block_funding_and_native_orders() {
    for case in 0..5 {
        let t = Temp::new();
        let (mut j, c, _) = setup(&t, false);
        let mut s = good_setup();
        match case {
            0 => s.lending_disabled = false,
            1 => s.borrowed = "1".into(),
            2 => s.interest = "1".into(),
            3 => s.complete = false,
            _ => s.borrowed = "not-a-number".into(),
        };
        assert!(!c.observe_setup(&mut j, id(6), 100, s).unwrap());
        assert!(!j.state().unwrap().native_funding_ready());
        prepare(&mut j, 10, Rail::Release, 20, 0);
        assert!(
            c.expose_chain(
                &mut j,
                Dispatch {
                    attempt: a(10),
                    commit: id(40),
                    at: 100
                },
                Rail::Release,
                counters(Rail::Release, 0, 0)
            )
            .is_err()
        );
    }
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    prepare(&mut j, 10, Rail::Release, 20, 0);
    assert!(
        c.expose_chain(
            &mut j,
            Dispatch {
                attempt: a(10),
                commit: id(40),
                at: 10_101
            },
            Rail::Release,
            counters(Rail::Release, 0, 0)
        )
        .is_err()
    );
}
#[test]
fn wrong_network_program_mint_route_receipt_epoch_counters_never_post_a_credit() {
    for case in 0..9 {
        let t = Temp::new();
        let (mut j, c, _) = setup(&t, false);
        prepare(&mut j, 10, Rail::Release, 20, 0);
        let p = expose(&mut j, &c, 10, Rail::Release, 60);
        let mut r = chain(&p, 60);
        match case {
            0 => r.network = [99; 32],
            1 => r.program = [99; 32],
            2 => r.mint = [99; 32],
            3 => r.source = [99; 32],
            4 => r.destination = [99; 32],
            5 => r.operation = Some([99; 32]),
            6 => r.epoch = Some(2),
            7 => r.sequence = Some(2),
            _ => r.config = Some([99; 32]),
        };
        let before = j.state().unwrap().clone();
        assert!(c.observe_chain(&mut j, id(60), 100, r).is_err());
        assert_eq!(j.state().unwrap(), &before);
        assert_eq!(j.state().unwrap().ledger().broker(), atoms(0));
    }
}
#[test]
fn finalized_failed_cpi_closes_zero_effect_without_counter_or_claim_consumption() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    prepare(&mut j, 10, Rail::Payout, 19, 0);
    let p = expose(&mut j, &c, 10, Rail::Payout, 60);
    let mut r = chain(&p, 60);
    r.succeeded = false;
    r.amount = 0;
    r.operation = None;
    r.epoch = None;
    r.config = None;
    r.paid = None;
    r.sequence = None;
    c.observe_chain(&mut j, id(60), 100, r).unwrap();
    assert_eq!(j.state().unwrap().ledger().paid(user(1)).unwrap(), atoms(0));
    assert_eq!(j.state().unwrap().ledger().vault(), atoms(120));
    assert!(j.state().unwrap().funds()[0].terminal);
    bridge(&j);
}
#[test]
fn payout_uses_persistent_paid_cas_and_public_wallet_not_private_account_id() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    prepare(&mut j, 10, Rail::Payout, 5, 0);
    let p = expose(&mut j, &c, 10, Rail::Payout, 60);
    c.observe_chain(&mut j, id(60), 100, chain(&p, 60)).unwrap();
    prepare(&mut j, 11, Rail::Payout, 5, 0);
    assert!(
        c.expose_chain(
            &mut j,
            Dispatch {
                attempt: a(11),
                commit: id(41),
                at: 100
            },
            Rail::Payout,
            counters(Rail::Payout, 0, 0)
        )
        .is_err()
    );
    let action = c
        .expose_chain(
            &mut j,
            Dispatch {
                attempt: a(11),
                commit: id(41),
                at: 100,
            },
            Rail::Payout,
            counters(Rail::Payout, 5, 1),
        )
        .unwrap();
    let p = signed_fixture(&c, &mut j, action, 61);
    let mut r = chain(&p, 61);
    r.paid = Some(5);
    assert!(c.observe_chain(&mut j, id(61), 100, r).is_err());
    c.observe_chain(&mut j, id(61), 100, chain(&p, 61)).unwrap();
    assert_eq!(
        j.state().unwrap().ledger().paid(user(1)).unwrap(),
        atoms(10)
    );
}
#[test]
fn withdrawal_ack_timeout_error_and_empty_observations_never_release_or_resend() {
    for status in [None, Some(200), Some(409), Some(429), Some(500)] {
        let t = Temp::new();
        let (mut j, c, g) = setup(&t, true);
        prepare(&mut j, 10, Rail::Withdraw, 19, 1);
        let mut f = Fake {
            status,
            ..Fake::default()
        };
        withdraw(&c, &mut j, &g, 10, &mut f);
        drop(j);
        let mut j = open(&t);
        assert_eq!(j.state().unwrap().ledger().venue().cash(), atoms(20));
        assert_eq!(
            j.state()
                .unwrap()
                .reserved(Resource::Location(Location::Venue))
                .unwrap(),
            atoms(20)
        );
        assert!(
            c.withdraw(
                &mut j,
                &g,
                Dispatch {
                    attempt: a(10),
                    commit: id(91),
                    at: 100
                },
                &mut f
            )
            .is_err()
        );
        assert_eq!(f.calls, 1);
        let mut w = withdrawal(10, 20, 19);
        w.complete = false;
        assert!(c.observe_withdrawal(&mut j, id(60), 100, w).is_err());
    }
}
#[test]
fn withdrawal_matches_original_uuid_and_owner_tokens_and_accounts_actual_overrun() {
    let t = Temp::new();
    let (mut j, c, g) = setup(&t, true);
    prepare(&mut j, 10, Rail::Withdraw, 19, 1);
    withdraw(&c, &mut j, &g, 10, &mut Fake::default());
    let mut w = withdrawal(10, 20, 19);
    w.idempotency_key = withdrawal_id(a(11));
    assert!(c.observe_withdrawal(&mut j, id(60), 100, w).is_err());
    let mut w = withdrawal(10, 20, 19);
    w.payment.destination = [99; 32];
    assert!(c.observe_withdrawal(&mut j, id(60), 100, w).is_err());
    // A real extra fee belongs to house and contains the broken bound, not erased data.
    assert!(
        c.observe_withdrawal(&mut j, id(60), 100, withdrawal(10, 20, 18))
            .is_err()
    );
    bridge(&j);
    assert_eq!(j.state().unwrap().ledger().broker(), atoms(18));
    assert_eq!(
        j.state()
            .unwrap()
            .ledger()
            .book(Owner::House)
            .unwrap()
            .cash(),
        atoms(98)
    );
    assert!(j.state().unwrap().funds()[0].faulted);
}
#[test]
fn withdrawal_uses_shared_read_cleanup_credits_and_rate_limit_backoff() {
    let t = Temp::new();
    let (mut j, c, g) = setup(&t, true);
    for n in 100..110 {
        g.reserve_read(&mut j, id(n), 100, true).unwrap();
    }
    prepare(&mut j, 10, Rail::Withdraw, 19, 1);
    let mut f = Fake::default();
    assert_eq!(
        c.withdraw(
            &mut j,
            &g,
            Dispatch {
                attempt: a(10),
                commit: id(40),
                at: 100
            },
            &mut f
        ),
        Err(AdapterError::Limit)
    );
    assert_eq!(f.calls, 0);
    let t = Temp::new();
    let (mut j, c, g) = setup(&t, true);
    prepare(&mut j, 10, Rail::Withdraw, 19, 1);
    withdraw(
        &c,
        &mut j,
        &g,
        10,
        &mut Fake {
            status: Some(429),
            ..Fake::default()
        },
    );
    assert!(g.reserve_read(&mut j, id(100), 100, true).is_err());
}
#[test]
fn persisted_native_key_fence_prevents_funding_signature() {
    let t = Temp::new();
    let (mut j, c, g) = setup(&t, true);
    prepare(&mut j, 10, Rail::Withdraw, 19, 1);
    g.deactivate(&mut j, id(90), 100).unwrap();
    let mut f = Fake::default();
    assert!(
        c.withdraw(
            &mut j,
            &g,
            Dispatch {
                attempt: a(10),
                commit: id(40),
                at: 100
            },
            &mut f
        )
        .is_err()
    );
    assert_eq!(f.calls, 0);
}
#[test]
fn partial_native_credit_preserves_transit_and_requires_original_chain_correlation() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    prepare(&mut j, 10, Rail::Release, 20, 0);
    let p = expose(&mut j, &c, 10, Rail::Release, 60);
    c.observe_chain(&mut j, id(60), 100, chain(&p, 60)).unwrap();
    prepare(&mut j, 11, Rail::Deposit, 20, 0);
    let p = expose(&mut j, &c, 11, Rail::Deposit, 61);
    c.observe_chain(&mut j, id(61), 100, chain(&p, 61)).unwrap();
    let wrong = Credit {
        attempt: a(11),
        account: account(),
        deposit_signature: [99; 64],
        event: EconomicEventId::new(&[62]).unwrap(),
        cut: 300,
        amount: 20,
        fee: 0,
        final_credit: true,
        raw: raw(b"wrong original deposit"),
    };
    assert!(c.observe_credit(&mut j, id(62), 100, wrong).is_err());
    credit(&c, &mut j, 62, 8, false);
    let r = c.residuals(&mut j, 100).unwrap();
    assert_eq!(r.native, atoms(8));
    assert_eq!(r.transit, atoms(12));
    assert_eq!(r.unresolved, [a(11)]);
    assert!(!j.state().unwrap().native_funding_ready());
    bridge(&j);
}
#[test]
fn broker_source_is_nonnegative_and_not_venue_or_vault_payout_capacity() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    prepare(&mut j, 10, Rail::Release, 20, 0);
    let p = expose(&mut j, &c, 10, Rail::Release, 60);
    c.observe_chain(&mut j, id(60), 100, chain(&p, 60)).unwrap();
    assert_eq!(j.state().unwrap().ledger().vault(), atoms(100));
    assert_eq!(j.state().unwrap().ledger().broker(), atoms(20));
    assert_eq!(j.state().unwrap().ledger().venue().cash(), atoms(0));
    let i = lifecycle::Intent {
        request: a(11).request,
        source: Location::Broker,
        destination: Destination::Location(Location::Venue),
        net: atoms(21),
        maximum_fee: atoms(0),
        fee_payer: Owner::House,
        allow_partial: false,
        policy: config().policy,
        authority_epoch: 1,
        expires_at: 1000,
    };
    let approval = orders::Approval {
        account: user(1),
        intent_hash: i.digest().unwrap(),
        authority_epoch: 1,
    };
    let t = tx(
        &j,
        11,
        vec![],
        vec![Control::Funds(lifecycle::Action::Accept {
            intent: Box::new(i),
            approval,
        })],
    );
    assert_eq!(
        j.commit(t).unwrap().receipt.controls,
        Some(ControlError::Capacity)
    );
    bridge(&j);
}

mod demo_tests {
    use super::*;
    use cinder_pacifica::{funding::demo, reads};

    pub(super) fn gateway_for(origin: Origin) -> Gateway {
        let mut profile = profile();
        profile.environment = origin.url().into();
        Gateway::new(
            profile,
            Policy {
                revision: 1,
                evidence: "offline demo GET fixture".into(),
                execution: Level::Qualified,
                origin,
                expiry_ms: 30_000,
                credits: 12_000,
                cleanup_reserve: 120,
                read_cost: reads::MIN_READ_COST,
            },
            Zeroizing::new([7; 32]),
            1,
        )
        .unwrap()
    }
    pub(super) fn policy() -> demo::Policy {
        demo::Policy {
            revision: 1,
            maximum_reads: 8,
            maximum_pages: 4,
            interval_ms: 1_000,
            maximum_backoff_ms: 4_000,
            lifetime_ms: 9_000,
            initial_setup: None,
        }
    }
    fn deposited(t: &Temp) -> (Store, Controller, Gateway) {
        let (mut j, c, g) = setup_with_gateway(t, false, gateway_for(Origin::Testnet));
        prepare(&mut j, 10, Rail::Release, 20, 0);
        let p = expose(&mut j, &c, 10, Rail::Release, 60);
        c.observe_chain(&mut j, id(60), 100, chain(&p, 60)).unwrap();
        prepare(&mut j, 11, Rail::Deposit, 20, 0);
        let p = expose(&mut j, &c, 11, Rail::Deposit, 61);
        c.observe_chain(&mut j, id(61), 100, chain(&p, 61)).unwrap();
        (j, c, g)
    }
    fn poll(n: u8, at: u64) -> demo::Poll {
        demo::Poll {
            attempt: a(11),
            reservation: id(n),
            evidence: id(n + 50),
            at,
        }
    }
    fn deposit(amount: &str, signature: u8) -> Value {
        json!({"amount":amount,"transaction_id":bs58::encode([signature;64]).into_string(),"created_at":100})
    }
    fn page(rows: Value) -> Value {
        json!({"success":true,"data":rows,"has_more":false})
    }
    fn balance(pending: &str) -> Value {
        page(
            json!([{"amount":"20","balance":"20","pending_balance":pending,"event_type":"deposit_release","created_at":100}]),
        )
    }
    fn deliver(
        j: &mut Store,
        c: &Controller,
        g: &Gateway,
        p: &demo::Policy,
        n: u8,
        at: u64,
        body: Value,
    ) -> String {
        let demo::Step::Request(request, completion) =
            c.prepare_demo_deposit_poll(j, g, p, poll(n, at)).unwrap()
        else {
            panic!("expected one GET");
        };
        let query = request.consume(at).unwrap();
        assert_eq!(query.origin(), Origin::Testnet);
        let target = query.target().to_owned();
        let outcome = reads::complete(
            j,
            g,
            *completion,
            Reply::Response {
                status: 200,
                body: raw(body.to_string().as_bytes()),
                received_at: at,
                retry_after_ms: None,
            },
        )
        .unwrap();
        assert!(matches!(outcome, reads::Outcome::Ingested(_)));
        target
    }
    fn observed(j: &mut Store, c: &Controller, g: &Gateway, p: &demo::Policy) {
        assert!(
            deliver(j, c, g, p, 70, 100, page(json!([deposit("20", 61)])))
                .starts_with("/api/v1/account/deposit/history?")
        );
        assert!(deliver(j, c, g, p, 71, 1100, balance("0")).contains("include_trades=true"));
    }
    fn pending(j: &Store) {
        let s = j.state().unwrap();
        assert_eq!(s.ledger().venue().cash(), atoms(0));
        assert_eq!(s.ledger().in_transit().unwrap(), atoms(20));
        assert!(
            !s.funds()
                .iter()
                .find(|o| o.attempt == Some(a(11)))
                .unwrap()
                .terminal
        );
        assert!(!s.native_funding_ready());
        bridge(j);
    }

    #[test]
    fn demo_observed_testnet_history_shapes_do_not_recredit_a_withdrawn_deposit() {
        let fixture: Value =
            serde_json::from_str(include_str!("fixtures/demo-native-shapes.json")).unwrap();
        let t = Temp::new();
        let (mut j, c, g) = deposited(&t);
        let p = policy();
        let mut history = fixture["deposit"].clone();
        history["data"][0]["transaction_id"] = json!(bs58::encode([61; 64]).into_string());
        deliver(&mut j, &c, &g, &p, 70, 100, history);
        deliver(
            &mut j,
            &c,
            &g,
            &p,
            71,
            1100,
            fixture["balance_after_withdrawal"].clone(),
        );
        assert!(
            !c.confirm_demo_deposit(&mut j, &g, &p, a(11), id(80), 1100)
                .unwrap()
        );
        pending(&j);
    }

    #[test]
    fn demo_synthetic_first_credit_projection_accepts_observed_omitted_cursor_shape() {
        let fixture: Value =
            serde_json::from_str(include_str!("fixtures/demo-native-shapes.json")).unwrap();
        let t = Temp::new();
        let (mut j, c, g) = deposited(&t);
        let p = policy();
        let mut history = fixture["deposit"].clone();
        history["data"][0]["transaction_id"] = json!(bs58::encode([61; 64]).into_string());
        deliver(&mut j, &c, &g, &p, 70, 100, history);
        // Removing the later withdrawal is an explicit synthetic prefix, not a
        // new actual deposit or a complete history certificate.
        let mut balance = fixture["balance_after_withdrawal"].clone();
        balance["data"].as_array_mut().unwrap().remove(0);
        deliver(&mut j, &c, &g, &p, 71, 1100, balance);
        assert!(
            c.confirm_demo_deposit(&mut j, &g, &p, a(11), id(80), 1100)
                .unwrap()
        );
        assert_eq!(j.state().unwrap().ledger().venue().cash(), atoms(20));
        assert!(!j.state().unwrap().native_funding_ready());
    }

    #[test]
    fn demo_original_credit_posts_once_and_replays_without_strong_cut_or_new_risk_grant() {
        let t = Temp::new();
        let (mut j, c, g) = deposited(&t);
        let p = policy();
        assert_eq!(
            c.demo_deposit_candidate(&mut j, &g, &p, 100).unwrap(),
            Some(a(11))
        );
        observed(&mut j, &c, &g, &p);
        assert!(
            c.confirm_demo_deposit(&mut j, &g, &p, a(11), id(80), 1100)
                .unwrap()
        );
        let s = j.state().unwrap();
        let o = s.funds().iter().find(|o| o.attempt == Some(a(11))).unwrap();
        assert!(o.terminal && !o.faulted && o.demo.is_some() && o.proof.is_none());
        assert_eq!(s.ledger().venue().cash(), atoms(20));
        assert_eq!(s.ledger().in_transit().unwrap(), atoms(0));
        assert_eq!(
            s.ledger().book(Owner::Customer(user(1))).unwrap().cash(),
            atoms(20)
        );
        assert!(!s.native_funding_ready());
        assert_eq!(s.unresolved_raw(), 0); // GET provenance is not fake unresolved economics.
        assert_eq!(j.transaction(id(80)).unwrap().inputs[0].source_cut, None);
        assert!(j.transaction(id(80)).unwrap().funds_observations.is_empty());
        bridge(&j);
        let head = j.head();
        let expected = j.state().unwrap().clone();
        drop(j);
        let mut j = open(&t);
        assert_eq!(j.state().unwrap(), &expected);
        assert_eq!(
            c.demo_deposit_candidate(&mut j, &g, &p, 2100).unwrap(),
            None
        );
        assert!(
            c.confirm_demo_deposit(&mut j, &g, &p, a(11), id(81), 2100)
                .unwrap()
        );
        assert!(matches!(
            c.prepare_demo_deposit_poll(&mut j, &g, &p, poll(72, 2100))
                .unwrap(),
            demo::Step::Confirmed
        ));
        assert_eq!(j.head(), head);
        let frozen = Transaction {
            at: 2100,
            ..tx(
                &j,
                82,
                vec![],
                vec![Control::Funds(lifecycle::Action::Freeze)],
            )
        };
        assert_eq!(j.commit(frozen).unwrap().receipt.controls, None);
        assert!(matches!(
            j.recovery_cut(j.head(), key(211, Location::Venue)),
            Err(cinder_journal::recovery::CutError::OpenCommitments)
        ));
    }
    #[test]
    fn demo_balance_only_wrong_signature_partial_or_conflicting_credit_never_settles() {
        for rows in [
            json!([]),
            json!([deposit("20", 99)]),
            json!([deposit("8", 61)]),
            json!([deposit("20", 61), deposit("21", 61)]),
        ] {
            let t = Temp::new();
            let (mut j, c, g) = deposited(&t);
            let p = policy();
            deliver(&mut j, &c, &g, &p, 70, 100, page(rows));
            assert!(!matches!(
                c.confirm_demo_deposit(&mut j, &g, &p, a(11), id(80), 100),
                Ok(true)
            ));
            pending(&j);
        }
        let t = Temp::new();
        let (mut j, c, g) = deposited(&t);
        deliver(&mut j, &c, &g, &policy(), 70, 100, balance("0")); // Not a deposit-history record.
        assert!(
            !c.confirm_demo_deposit(&mut j, &g, &policy(), a(11), id(80), 100)
                .unwrap()
        );
        pending(&j);
    }
    #[test]
    fn demo_duplicate_deposit_rows_are_idempotent_but_pending_or_ambiguous_balances_wait() {
        for pending_amount in ["1", "0"] {
            let t = Temp::new();
            let (mut j, c, g) = deposited(&t);
            let p = policy();
            deliver(
                &mut j,
                &c,
                &g,
                &p,
                70,
                100,
                page(json!([deposit("20", 61), deposit("20", 61)])),
            );
            let mut b = balance(pending_amount);
            if pending_amount == "0" {
                let duplicate = b["data"][0].clone();
                b["data"].as_array_mut().unwrap().push(duplicate);
            }
            deliver(&mut j, &c, &g, &p, 71, 1100, b);
            assert!(
                !c.confirm_demo_deposit(&mut j, &g, &p, a(11), id(80), 1100)
                    .unwrap()
            );
            pending(&j);
        }
        let t = Temp::new();
        let (mut j, c, g) = deposited(&t);
        let p = policy();
        deliver(
            &mut j,
            &c,
            &g,
            &p,
            70,
            100,
            page(json!([deposit("20", 61), deposit("20", 61)])),
        );
        deliver(&mut j, &c, &g, &p, 71, 1100, balance("0"));
        assert!(
            c.confirm_demo_deposit(&mut j, &g, &p, a(11), id(80), 1100)
                .unwrap()
        );
        assert_eq!(j.state().unwrap().ledger().venue().cash(), atoms(20));
    }
    #[test]
    fn demo_missing_response_budget_and_backoff_survive_restart_without_resend() {
        let t = Temp::new();
        let (mut j, c, g) = deposited(&t);
        let p = demo::Policy {
            maximum_reads: 2,
            ..policy()
        };
        let demo::Step::Request(request, _) = c
            .prepare_demo_deposit_poll(&mut j, &g, &p, poll(70, 100))
            .unwrap()
        else {
            panic!("GET");
        };
        assert!(
            request
                .consume(100)
                .unwrap()
                .target()
                .starts_with("/api/v1/account/deposit/history?")
        );
        let head = j.head();
        drop(j);
        let mut j = open(&t);
        assert!(matches!(
            c.prepare_demo_deposit_poll(&mut j, &g, &p, poll(71, 1100))
                .unwrap(),
            demo::Step::Waiting
        ));
        assert_eq!(j.head(), head);
        assert!(matches!(
            c.prepare_demo_deposit_poll(&mut j, &g, &p, poll(71, 2100))
                .unwrap(),
            demo::Step::Request(_, _)
        ));
        assert!(matches!(
            c.prepare_demo_deposit_poll(&mut j, &g, &p, poll(72, 6100))
                .unwrap(),
            demo::Step::Exhausted
        ));
        pending(&j);
    }
    #[test]
    fn demo_pages_use_retained_escaped_cursor_and_refuse_loops_or_truncation() {
        for limit in [1, 4] {
            let t = Temp::new();
            let (mut j, c, g) = deposited(&t);
            let p = demo::Policy {
                maximum_pages: limit,
                ..policy()
            };
            deliver(
                &mut j,
                &c,
                &g,
                &p,
                70,
                100,
                json!({"success":true,"data":[],"has_more":true,"next_cursor":"a/b?c"}),
            );
            if limit == 1 {
                assert!(matches!(
                    c.prepare_demo_deposit_poll(&mut j, &g, &p, poll(71, 1100))
                        .unwrap(),
                    demo::Step::Exhausted
                ));
            } else {
                let target = deliver(
                    &mut j,
                    &c,
                    &g,
                    &p,
                    71,
                    1100,
                    json!({"success":true,"data":[],"has_more":true,"next_cursor":"a/b?c"}),
                );
                assert!(target.contains("cursor=a%2Fb%3Fc"));
                assert!(
                    c.prepare_demo_deposit_poll(&mut j, &g, &p, poll(72, 2100))
                        .is_err()
                );
            }
            pending(&j);
        }
    }
    #[test]
    fn demo_policy_mainnet_rebinding_and_missing_original_refuse_before_get_spend() {
        let t = Temp::new();
        let (mut j, c, g) = deposited(&t);
        let p = policy();
        let head = j.head();
        assert!(
            c.prepare_demo_deposit_poll(&mut j, &gateway_for(Origin::Mainnet), &p, poll(70, 100))
                .is_err()
        );
        assert_eq!(j.head(), head);
        deliver(&mut j, &c, &g, &p, 70, 100, page(json!([])));
        let head = j.head();
        assert!(
            c.prepare_demo_deposit_poll(
                &mut j,
                &g,
                &demo::Policy { revision: 2, ..p },
                poll(71, 1100)
            )
            .is_err()
        );
        assert_eq!(j.head(), head);
        let t = Temp::new();
        let (mut j, c, g) = setup_with_gateway(&t, false, gateway_for(Origin::Testnet));
        let head = j.head();
        assert!(
            c.prepare_demo_deposit_poll(&mut j, &g, &policy(), poll(70, 100))
                .is_err()
        );
        assert_eq!(j.head(), head);
    }
    #[test]
    fn demo_expiry_stale_future_or_unrelated_balance_events_preserve_original_hold() {
        for timestamp in [99, 101] {
            let t = Temp::new();
            let (mut j, c, g) = deposited(&t);
            let mut row = deposit("20", 61);
            row["created_at"] = json!(timestamp);
            deliver(&mut j, &c, &g, &policy(), 70, 100, page(json!([row])));
            assert!(
                c.confirm_demo_deposit(&mut j, &g, &policy(), a(11), id(80), 100)
                    .is_err()
            );
            pending(&j);
        }
        for timestamp in [99, 1101] {
            let t = Temp::new();
            let (mut j, c, g) = deposited(&t);
            deliver(
                &mut j,
                &c,
                &g,
                &policy(),
                70,
                100,
                page(json!([deposit("20", 61)])),
            );
            let mut body = balance("0");
            body["data"][0]["created_at"] = json!(timestamp);
            deliver(&mut j, &c, &g, &policy(), 71, 1100, body);
            assert!(
                !c.confirm_demo_deposit(&mut j, &g, &policy(), a(11), id(80), 1100)
                    .unwrap()
            );
            pending(&j);
        }
        for kind in ["withdraw", "funding", "trade", "adl_liquidation"] {
            let t = Temp::new();
            let (mut j, c, g) = deposited(&t);
            let p = policy();
            deliver(
                &mut j,
                &c,
                &g,
                &p,
                70,
                100,
                page(json!([deposit("20", 61)])),
            );
            let mut b = balance("0");
            b["data"][0]["event_type"] = json!(kind);
            deliver(&mut j, &c, &g, &p, 71, 1100, b);
            assert!(
                !c.confirm_demo_deposit(&mut j, &g, &p, a(11), id(80), 1100)
                    .unwrap()
            );
            pending(&j);
        }
        let t = Temp::new();
        let (mut j, c, g) = deposited(&t);
        observed(&mut j, &c, &g, &policy());
        assert!(
            !c.confirm_demo_deposit(&mut j, &g, &policy(), a(11), id(80), 9100)
                .unwrap()
        );
        assert!(matches!(
            c.prepare_demo_deposit_poll(&mut j, &g, &policy(), poll(72, 9100))
                .unwrap(),
            demo::Step::Exhausted
        ));
        pending(&j);
    }
    #[test]
    fn demo_malformed_or_empty_success_replies_back_off_without_cash_or_refund() {
        for body in [
            page(json!([])),
            json!({"success":true,"data":[{"amount":"20","created_at":100}],"has_more":false}),
            json!({"success":true,"data":[],"has_more":false,"unexpected":1}),
        ] {
            let t = Temp::new();
            let (mut j, c, g) = deposited(&t);
            deliver(&mut j, &c, &g, &policy(), 70, 100, body);
            assert!(
                !c.confirm_demo_deposit(&mut j, &g, &policy(), a(11), id(80), 100)
                    .unwrap()
            );
            let head = j.head();
            assert!(matches!(
                c.prepare_demo_deposit_poll(&mut j, &g, &policy(), poll(71, 1100))
                    .unwrap(),
                demo::Step::Waiting
            ));
            assert_eq!(j.head(), head);
            drop(j);
            let mut j = open(&t);
            assert!(matches!(
                c.prepare_demo_deposit_poll(&mut j, &g, &policy(), poll(71, 2100))
                    .unwrap(),
                demo::Step::Request(..)
            ));
            pending(&j);
        }
    }
    #[test]
    fn demo_rate_limit_survives_restart_and_uses_shared_gateway_cooldown() {
        let t = Temp::new();
        let (mut j, c, g) = deposited(&t);
        let demo::Step::Request(request, completion) = c
            .prepare_demo_deposit_poll(&mut j, &g, &policy(), poll(70, 100))
            .unwrap()
        else {
            panic!("expected GET");
        };
        request.consume(100).unwrap();
        assert!(matches!(
            reads::complete(
                &mut j,
                &g,
                *completion,
                Reply::Response {
                    status: 429,
                    body: raw(b"rate limited"),
                    received_at: 100,
                    retry_after_ms: Some(5000)
                }
            )
            .unwrap(),
            reads::Outcome::Limited
        ));
        let head = j.head();
        drop(j);
        let mut j = open(&t);
        assert!(matches!(
            c.prepare_demo_deposit_poll(&mut j, &g, &policy(), poll(71, 1100))
                .unwrap(),
            demo::Step::Waiting
        ));
        assert!(
            c.prepare_demo_deposit_poll(&mut j, &g, &policy(), poll(71, 2100))
                .is_err()
        );
        assert_eq!(j.head(), head);
        pending(&j);
    }
    #[test]
    fn demo_late_response_is_retained_but_cannot_confirm_after_policy_deadline() {
        let t = Temp::new();
        let (mut j, c, g) = deposited(&t);
        deliver(
            &mut j,
            &c,
            &g,
            &policy(),
            70,
            100,
            page(json!([deposit("20", 61)])),
        );
        let demo::Step::Request(request, completion) = c
            .prepare_demo_deposit_poll(&mut j, &g, &policy(), poll(71, 1100))
            .unwrap()
        else {
            panic!("expected GET");
        };
        request.consume(1100).unwrap();
        reads::complete(
            &mut j,
            &g,
            *completion,
            Reply::Response {
                status: 200,
                body: raw(balance("0").to_string().as_bytes()),
                received_at: 9100,
                retry_after_ms: None,
            },
        )
        .unwrap();
        assert!(j.transaction(id(121)).is_some());
        assert!(
            !c.confirm_demo_deposit(&mut j, &g, &policy(), a(11), id(80), 9100)
                .unwrap()
        );
        pending(&j);
    }
    #[test]
    fn demo_marker_cannot_be_upgraded_to_a_strong_completion_observation() {
        let t = Temp::new();
        let (mut j, c, g) = deposited(&t);
        observed(&mut j, &c, &g, &policy());
        c.confirm_demo_deposit(&mut j, &g, &policy(), a(11), id(80), 1100)
            .unwrap();
        let demo = j
            .state()
            .unwrap()
            .funds()
            .iter()
            .find(|o| o.attempt == Some(a(11)))
            .unwrap()
            .demo
            .as_ref()
            .unwrap()
            .clone();
        let head = j.head();
        assert!(
            c.observe_credit(
                &mut j,
                id(81),
                2100,
                Credit {
                    attempt: a(11),
                    account: account(),
                    deposit_signature: [61; 64],
                    event: demo.credit.event.clone(),
                    cut: 999,
                    amount: 20,
                    fee: 0,
                    final_credit: true,
                    raw: raw(b"attempted demo-to-strong upgrade")
                }
            )
            .is_err()
        );
        assert_eq!(j.head(), head);
        assert_eq!(j.state().unwrap().ledger().venue().cash(), atoms(20));
        assert!(!j.state().unwrap().native_funding_ready());
        let mut transaction = tx(&j, 81, vec![], vec![]);
        transaction.at = 2100;
        transaction.funds_observations.push(lifecycle::Observation {
            key: key(212, Location::Venue),
            authority_epoch: 1,
            observed_at: 2100,
            raw: raw(b"invented strong completion"),
            terminal: lifecycle::Terminal {
                attempt: a(11),
                debit: atoms(20),
                settled: atoms(20),
                receipts: vec![demo.debit, demo.credit],
                coverage: config()
                    .sources
                    .iter()
                    .map(|s| lifecycle::Coverage {
                        source: s.scope,
                        through: 999,
                    })
                    .collect(),
                no_later_execution: [1; 32],
            },
        });
        j.commit(transaction).unwrap();
        let o = j
            .state()
            .unwrap()
            .funds()
            .iter()
            .find(|o| o.attempt == Some(a(11)))
            .unwrap();
        assert!(o.demo.is_some() && o.proof.is_none() && o.faulted);
        assert!(!j.state().unwrap().native_funding_ready());
    }
    #[test]
    fn demo_late_economic_conflict_faults_instead_of_remaining_terminally_healthy() {
        let t = Temp::new();
        let (mut j, c, g) = deposited(&t);
        observed(&mut j, &c, &g, &policy());
        assert!(
            c.confirm_demo_deposit(&mut j, &g, &policy(), a(11), id(80), 1100)
                .unwrap()
        );
        let late = Transaction {
            at: 2100,
            ..tx(
                &j,
                81,
                vec![(
                    Location::Venue,
                    Change::Funds(FundsChange::Observe {
                        attempt: a(11),
                        leg: Leg::Arrive(Destination::Location(Location::Venue)),
                        amount: atoms(1),
                        fee: atoms(0),
                    }),
                )],
                vec![],
            )
        };
        j.commit(late).unwrap();
        assert!(
            j.state()
                .unwrap()
                .funds()
                .iter()
                .find(|o| o.attempt == Some(a(11)))
                .unwrap()
                .faulted
        );
        assert!(
            c.confirm_demo_deposit(&mut j, &g, &policy(), a(11), id(82), 2100)
                .is_err()
        );
    }
}

mod setup_read_tests {
    use super::*;
    use cinder_pacifica::{funding::setup, reads};
    use serde_json::{Value, json};

    fn unqualified(t: &Temp) -> (Store, Controller, Gateway) {
        let (mut j, c, g) = setup_with_gateway(t, false, demo_tests::gateway_for(Origin::Testnet));
        let mut s = good_setup();
        s.complete = false;
        s.lending_disabled = false;
        assert!(!c.observe_setup(&mut j, id(6), 100, s).unwrap());
        (j, c, g)
    }
    fn bodies() -> [Value; 3] {
        [
            json!({"success":true,"data":{"auto_lend_disabled":true,"margin_settings":[],"spot_settings":[]}}),
            json!({"success":true,"data":{"borrowed":"0","pending_interest":"0","spot_balances":[],"updated_at":100}}),
            json!({"success":true,"data":{
                "balance":"0","account_equity":"0","available_to_spend":"0","available_to_withdraw":"0",
                "pending_balance":"0","pending_interest":"0","total_margin_used":"0","cross_mmr":"0",
                "spot_collateral":"0","spot_market_value":"0","cross_account_equity":null,
                "positions_count":0,"orders_count":0,"stop_orders_count":0,"spot_balances":[],"updated_at":100
            }}),
        ]
    }
    fn poll(n: u8, at: u64) -> setup::Poll {
        setup::Poll {
            reservation: id(n),
            evidence: id(n + 1),
            at,
        }
    }
    fn read(j: &mut Store, c: &Controller, g: &Gateway, n: u8, at: u64, status: u16, body: Value) {
        read_policy(j, c, g, &demo_tests::policy(), (n, at), (status, body));
    }
    fn read_policy(
        j: &mut Store,
        c: &Controller,
        g: &Gateway,
        policy: &demo::Policy,
        (n, at): (u8, u64),
        (status, body): (u16, Value),
    ) {
        let setup::Step::Request(request, completion) =
            c.prepare_setup_poll(j, g, policy, poll(n, at)).unwrap()
        else {
            panic!("expected one setup GET")
        };
        let query = request.consume(at).unwrap();
        assert_eq!(query.origin(), Origin::Testnet);
        let expected = ["settings", "loan", ""][(usize::from(n) - 30) / 2 % 3];
        let path = if expected.is_empty() {
            "/api/v1/account".to_owned()
        } else {
            format!("/api/v1/account/{expected}")
        };
        assert_eq!(
            query.target(),
            format!("{path}?account={}", profile().account)
        );
        reads::complete(
            j,
            g,
            *completion,
            Reply::Response {
                status,
                body: raw(&serde_json::to_vec(&body).unwrap()),
                received_at: at,
                retry_after_ms: None,
            },
        )
        .unwrap();
    }
    fn initial_policy() -> demo::Policy {
        demo::Policy {
            initial_setup: Some(demo::InitialSetup {
                revision: 1,
                exclusive_control: true,
            }),
            ..demo_tests::policy()
        }
    }
    fn observed_round(
        j: &mut Store,
        c: &Controller,
        g: &Gateway,
        p: &demo::Policy,
        values: [Value; 3],
    ) {
        for (i, body) in values.into_iter().enumerate() {
            read_policy(
                j,
                c,
                g,
                p,
                (30 + 2 * i as u8, 100 + 1000 * i as u64),
                (200, body),
            );
        }
        c.observe_setup_reads(j, g, p, id(41), 2100).unwrap();
    }
    fn accept_initial(j: &mut Store, n: u8, rail: Rail) {
        let (source, destination) = match rail {
            Rail::Release => (Location::Vault, Destination::Location(Location::Broker)),
            Rail::Deposit => (Location::Broker, Destination::Location(Location::Venue)),
            _ => panic!("initial ingress only"),
        };
        let intent = lifecycle::Intent {
            request: a(n).request,
            source,
            destination,
            net: atoms(20),
            maximum_fee: atoms(0),
            fee_payer: Owner::House,
            allow_partial: false,
            policy: config().policy,
            authority_epoch: 1,
            expires_at: 10000,
        };
        let approval = orders::Approval {
            account: user(1),
            authority_epoch: 1,
            intent_hash: intent.digest().unwrap(),
        };
        let mut transaction = tx(
            j,
            n,
            vec![],
            vec![Control::Funds(lifecycle::Action::Accept {
                intent: Box::new(intent),
                approval,
            })],
        );
        transaction.at = 2100;
        assert_eq!(j.commit(transaction).unwrap().receipt.controls, None);
    }
    fn original_initial(
        j: &mut Store,
        c: &Controller,
        g: &Gateway,
        p: &demo::Policy,
        n: u8,
        base: u8,
    ) -> Plan {
        c.prepare_demo_ingress(
            j,
            g,
            p,
            Dispatch {
                attempt: a(n),
                commit: id(base),
                at: 2100,
            },
        )
        .unwrap();
        let action = c
            .expose_demo_ingress(
                j,
                g,
                p,
                Dispatch {
                    attempt: a(n),
                    commit: id(base + 1),
                    at: 2100,
                },
                counters(
                    if n == 10 {
                        Rail::Release
                    } else {
                        Rail::Deposit
                    },
                    0,
                    0,
                ),
            )
            .unwrap();
        let plan = action.plan().clone();
        let wire = VerifiedWire {
            attempt: a(n),
            binding: Sha256::digest(action.encode().unwrap().as_bytes()).into(),
            signature: [base + 2; 64],
            wire: raw(b"synthetic codec verified wire, not a live signature"),
        };
        c.persist_wire(j, id(base + 2), 2100, action, wire).unwrap();
        c.observe_chain(j, id(base + 3), 2100, chain(&plan, base + 2))
            .unwrap();
        plan
    }
    #[test]
    fn approved_initial_setup_is_separate_durable_and_never_strong_readiness() {
        let t = Temp::new();
        let (mut j, c, g) = unqualified(&t);
        let p = initial_policy();
        observed_round(&mut j, &c, &g, &p, bodies());
        assert!(matches!(
            c.prepare_setup_poll(&mut j, &g, &p, poll(42, 2100))
                .unwrap(),
            setup::Step::DemoQualified
        ));
        assert!(!j.state().unwrap().native_funding_ready());
        assert_eq!(j.state().unwrap().ledger().venue().cash(), atoms(0));
        let head = j.head();
        drop(j);
        let mut j = open(&t);
        assert!(!c.observe_setup_reads(&mut j, &g, &p, id(42), 2100).unwrap());
        assert_eq!(j.head(), head);
        accept_initial(&mut j, 10, Rail::Release);
        let dispatch = Dispatch {
            attempt: a(10),
            commit: id(70),
            at: 2100,
        };
        assert!(c.prepare_ingress(&mut j, dispatch).is_err());
        assert!(
            c.prepare_demo_ingress(&mut j, &g, &demo_tests::policy(), dispatch)
                .is_err()
        );
        assert!(
            c.prepare_demo_ingress(
                &mut j,
                &demo_tests::gateway_for(Origin::Mainnet),
                &p,
                dispatch
            )
            .is_err()
        );
        c.prepare_demo_ingress(&mut j, &g, &p, dispatch).unwrap();
        assert!(
            j.state()
                .unwrap()
                .attempts()
                .iter()
                .all(|a| !a.possibly_exposed)
        );
        assert!(
            c.expose_chain(
                &mut j,
                Dispatch {
                    commit: id(71),
                    ..dispatch
                },
                Rail::Release,
                counters(Rail::Release, 0, 0)
            )
            .is_err()
        );
        assert!(!j.state().unwrap().native_funding_ready());
    }
    #[test]
    fn initial_setup_policy_is_explicit_and_legacy_bytes_are_unchanged() {
        let p = demo_tests::policy();
        let value = serde_json::to_value(&p).unwrap();
        assert!(value.get("initial_setup").is_none());
        assert_eq!(serde_json::from_value::<demo::Policy>(value).unwrap(), p);
        let c = controller();
        let g = demo_tests::gateway_for(Origin::Testnet);
        assert_ne!(
            p.release_commitment(&c, &g).unwrap(),
            initial_policy().release_commitment(&c, &g).unwrap()
        );
        for case in 0..3 {
            let mut p = initial_policy();
            match case {
                0 => p.initial_setup.as_mut().unwrap().revision = 0,
                1 => p.initial_setup.as_mut().unwrap().exclusive_control = false,
                _ => p.maximum_reads = 2,
            }
            assert!(p.validate().is_err());
        }
    }
    #[test]
    fn demo_setup_expiry_and_new_authority_block_original_wire_without_refunding_holds() {
        for case in 0..4 {
            let t = Temp::new();
            let (mut j, c, g) = unqualified(&t);
            let p = initial_policy();
            observed_round(&mut j, &c, &g, &p, bodies());
            accept_initial(&mut j, 10, Rail::Release);
            c.prepare_demo_ingress(
                &mut j,
                &g,
                &p,
                Dispatch {
                    attempt: a(10),
                    commit: id(70),
                    at: 2100,
                },
            )
            .unwrap();
            let action = c
                .expose_demo_ingress(
                    &mut j,
                    &g,
                    &p,
                    Dispatch {
                        attempt: a(10),
                        commit: id(71),
                        at: 2100,
                    },
                    counters(Rail::Release, 0, 0),
                )
                .unwrap();
            let wire = VerifiedWire {
                attempt: a(10),
                binding: Sha256::digest(action.encode().unwrap().as_bytes()).into(),
                signature: [72; 64],
                wire: raw(b"synthetic not live"),
            };
            let mut at = 2100;
            match case {
                0 => at = 9100, // Original setup lifetime, not refreshed on restart.
                1 => {
                    let mut t = tx(
                        &j,
                        60,
                        vec![],
                        vec![Control::Order(orders::Action::AdvanceAuthority {
                            account: user(1),
                            epoch: 2,
                        })],
                    );
                    t.at = at;
                    assert!(j.commit(t).unwrap().receipt.controls.is_none());
                }
                2 => {
                    let mut t = tx(
                        &j,
                        60,
                        vec![],
                        vec![Control::Funds(lifecycle::Action::Freeze)],
                    );
                    t.at = at;
                    assert!(j.commit(t).unwrap().receipt.controls.is_none());
                }
                _ => {
                    let mut setup = good_setup();
                    setup.observed_at = at;
                    setup.complete = false;
                    setup.borrowed = "1".into();
                    assert!(!c.observe_setup(&mut j, id(60), at, setup).unwrap());
                }
            }
            drop(j);
            let mut j = open(&t);
            let head = j.head();
            assert!(
                c.persist_wire(&mut j, id(72), at, action, wire).is_err(),
                "case {case}"
            );
            assert_eq!(j.head(), head);
            assert!(
                j.state()
                    .unwrap()
                    .holds()
                    .iter()
                    .any(|h| h.request == a(10).request && h.active)
            );
            assert!(
                j.state()
                    .unwrap()
                    .attempts()
                    .iter()
                    .any(|a| a.key == super::a(10) && a.possibly_exposed)
            );
            assert_eq!(j.state().unwrap().ledger().vault(), atoms(120));
        }
    }
    #[test]
    fn an_original_demo_deposit_can_reconcile_after_preflight_age_without_reauthorizing_ingress() {
        let t = Temp::new();
        let (mut j, c, g) = unqualified(&t);
        let p = demo::Policy {
            lifetime_ms: 60000,
            ..initial_policy()
        };
        observed_round(&mut j, &c, &g, &p, bodies());
        accept_initial(&mut j, 10, Rail::Release);
        original_initial(&mut j, &c, &g, &p, 10, 70);
        accept_initial(&mut j, 11, Rail::Deposit);
        original_initial(&mut j, &c, &g, &p, 11, 74);
        drop(j);
        let mut j = open(&t);
        assert!(matches!(
            c.prepare_setup_poll(&mut j, &g, &p, poll(78, 12100))
                .unwrap(),
            setup::Step::DemoQualified
        ));
        assert_eq!(
            c.demo_deposit_candidate(&mut j, &g, &p, 12100).unwrap(),
            Some(a(11))
        );
        let demo::Step::Request(request, completion) = c
            .prepare_demo_deposit_poll(
                &mut j,
                &g,
                &p,
                demo::Poll {
                    attempt: a(11),
                    reservation: id(80),
                    evidence: id(81),
                    at: 12100,
                },
            )
            .unwrap()
        else {
            panic!("reconcile original only");
        };
        request.consume(12100).unwrap();
        reads::complete(&mut j, &g, *completion, Reply::Unknown).unwrap();
        assert!(
            j.state()
                .unwrap()
                .holds()
                .iter()
                .any(|h| h.request == a(11).request && h.active)
        );
        assert!(
            c.expose_demo_ingress(
                &mut j,
                &g,
                &p,
                Dispatch {
                    attempt: a(11),
                    commit: id(82),
                    at: 12100
                },
                counters(Rail::Deposit, 0, 0)
            )
            .is_err()
        );
        assert_eq!(j.state().unwrap().ledger().in_transit().unwrap(), atoms(20));
        assert!(!j.state().unwrap().native_funding_ready());
    }
    #[test]
    fn adverse_or_stale_initial_reads_do_not_qualify_a_demo() {
        for case in 0..18 {
            let t = Temp::new();
            let (mut j, c, g) = unqualified(&t);
            let p = initial_policy();
            let mut replies = bodies();
            match case {
                0 => replies[0]["data"]["auto_lend_disabled"] = json!(false),
                1 => {
                    replies[0]["data"]["margin_settings"] =
                        json!([{"symbol":"BTC","isolated":true,"leverage":20}])
                }
                2 => replies[1]["data"]["borrowed"] = json!("1"),
                3 => replies[1]["data"]["pending_interest"] = json!("1"),
                4 => replies[2]["data"]["orders_count"] = json!(1),
                5 => replies[2]["data"]["positions_count"] = json!(1),
                6 => replies[2]["data"]["stop_orders_count"] = json!(1),
                7 => replies[2]["data"]["pending_balance"] = json!("1"),
                8 => replies[2]["data"]["spot_balances"] = json!([{"balance":"1"}]),
                9 => replies[1] = json!({"success":false,"error":"Account loan cache not found"}),
                10 => {
                    replies[2]["data"]
                        .as_object_mut()
                        .unwrap()
                        .remove("account_equity");
                }
                11 => replies[1]["data"]["updated_at"] = json!(1200),
                12 => replies[1]["data"]["spot_balances"] = json!([{"borrowed":"1"}]),
                13 => {
                    replies[0]["data"]["spot_settings"] =
                        json!([{"symbol":"SOL","auto_borrow":true}])
                }
                14 => replies[1]["data"]["error"] = json!("partial loan view"),
                15 => replies[2]["data"]["code"] = json!(400),
                16 => {
                    replies[1]["data"]
                        .as_object_mut()
                        .unwrap()
                        .remove("spot_balances");
                }
                _ => {
                    replies[0]["data"]
                        .as_object_mut()
                        .unwrap()
                        .remove("spot_settings");
                }
            }
            observed_round(&mut j, &c, &g, &p, replies);
            assert!(
                !matches!(
                    c.prepare_setup_poll(&mut j, &g, &p, poll(42, 2100))
                        .unwrap(),
                    setup::Step::DemoQualified
                ),
                "case {case}"
            );
            assert!(!j.state().unwrap().native_funding_ready());
        }
        let t = Temp::new();
        let (mut j, c, g) = unqualified(&t);
        let p = initial_policy();
        observed_round(&mut j, &c, &g, &p, bodies());
        assert!(matches!(
            c.prepare_setup_poll(&mut j, &g, &p, poll(42, 12100))
                .unwrap(),
            setup::Step::Exhausted
        ));
        assert!(
            !c.observe_setup_reads(&mut j, &g, &p, id(43), 12100)
                .unwrap()
        );
    }
    #[test]
    fn demo_preflight_carries_only_one_original_deposit_and_cannot_upgrade_it() {
        let t = Temp::new();
        let (mut j, c, g) = unqualified(&t);
        let p = initial_policy();
        observed_round(&mut j, &c, &g, &p, bodies());
        accept_initial(&mut j, 10, Rail::Release);
        original_initial(&mut j, &c, &g, &p, 10, 70);
        drop(j);
        let mut j = open(&t);
        accept_initial(&mut j, 11, Rail::Deposit);
        original_initial(&mut j, &c, &g, &p, 11, 74);
        // Even before credit, no second release can consume this initial grant.
        accept_initial(&mut j, 12, Rail::Release);
        assert!(
            c.prepare_demo_ingress(
                &mut j,
                &g,
                &p,
                Dispatch {
                    attempt: a(12),
                    commit: id(69),
                    at: 2100
                }
            )
            .is_err()
        );
        let mut cancelled = tx(
            &j,
            68,
            vec![],
            vec![Control::Funds(lifecycle::Action::CancelUnexposed(
                a(12).request,
            ))],
        );
        cancelled.at = 2100;
        assert!(j.commit(cancelled).unwrap().receipt.controls.is_none());
        assert_eq!(
            c.demo_deposit_candidate(&mut j, &g, &p, 2100).unwrap(),
            Some(a(11))
        );
        let before = j.head();
        assert!(
            c.expose_demo_ingress(
                &mut j,
                &g,
                &p,
                Dispatch {
                    attempt: a(11),
                    commit: id(78),
                    at: 2100
                },
                counters(Rail::Deposit, 0, 0)
            )
            .is_err()
        );
        assert!(
            c.observe_credit(
                &mut j,
                id(79),
                2100,
                Credit {
                    attempt: a(11),
                    account: account(),
                    deposit_signature: [76; 64],
                    event: EconomicEventId::new(b"not a strong proof").unwrap(),
                    cut: 999,
                    amount: 20,
                    fee: 0,
                    final_credit: true,
                    raw: raw(b"synthetic attempted upgrade"),
                }
            )
            .is_err()
        );
        assert_eq!(j.head(), before);
        for (n, at, body) in [
            (
                80,
                2100,
                json!({"success":true,"data":[{"amount":"20","transaction_id":bs58::encode([76;64]).into_string(),"created_at":2100}],"has_more":false}),
            ),
            (
                82,
                3100,
                json!({"success":true,"data":[{"amount":"20","balance":"20","pending_balance":"0","event_type":"deposit_release","created_at":2100}],"has_more":false}),
            ),
        ] {
            let demo::Step::Request(request, completion) = c
                .prepare_demo_deposit_poll(
                    &mut j,
                    &g,
                    &p,
                    demo::Poll {
                        attempt: a(11),
                        reservation: id(n),
                        evidence: id(n + 1),
                        at,
                    },
                )
                .unwrap()
            else {
                panic!("one original GET");
            };
            request.consume(at).unwrap();
            reads::complete(
                &mut j,
                &g,
                *completion,
                Reply::Response {
                    status: 200,
                    body: raw(&serde_json::to_vec(&body).unwrap()),
                    received_at: at,
                    retry_after_ms: None,
                },
            )
            .unwrap();
        }
        assert!(
            c.confirm_demo_deposit(&mut j, &g, &p, a(11), id(84), 3100)
                .unwrap()
        );
        assert_eq!(j.state().unwrap().ledger().venue().cash(), atoms(20));
        assert_eq!(
            j.state()
                .unwrap()
                .ledger()
                .book(Owner::Customer(user(1)))
                .unwrap()
                .cash(),
            atoms(20)
        );
        assert!(!j.state().unwrap().native_funding_ready());
        assert!(
            j.state()
                .unwrap()
                .funds()
                .iter()
                .any(|o| o.attempt == Some(a(11)) && o.demo.is_some() && o.proof.is_none())
        );
        assert!(
            c.expose_demo_ingress(
                &mut j,
                &g,
                &p,
                Dispatch {
                    attempt: a(12),
                    commit: id(85),
                    at: 3100
                },
                counters(Rail::Deposit, 0, 0)
            )
            .is_err()
        );
        bridge(&j);
    }
    #[test]
    fn authenticated_setup_round_replays_without_credit_readiness_or_repeat_record() {
        let t = Temp::new();
        let (mut j, c, g) = unqualified(&t);
        for (i, body) in bodies().into_iter().enumerate() {
            read(
                &mut j,
                &c,
                &g,
                30 + 2 * i as u8,
                100 + 1000 * i as u64,
                200,
                body,
            );
        }
        let setup::Step::Observed(observed) = c
            .prepare_setup_poll(&mut j, &g, &demo_tests::policy(), poll(40, 2100))
            .unwrap()
        else {
            panic!("expected retained observations")
        };
        assert!(observed.idle && observed.compatible_margin && observed.lending_disabled);
        assert_eq!((observed.borrowed, observed.interest), (0, 0));
        assert_eq!(format!("{observed:?}"), "SetupObservation([PRIVATE])");
        assert!(
            c.observe_setup_reads(&mut j, &g, &demo_tests::policy(), id(41), 2100)
                .unwrap()
        );
        let head = j.head();
        assert!(!j.state().unwrap().native_funding_ready());
        assert_eq!(j.state().unwrap().unresolved_raw(), 0);
        assert_eq!(j.state().unwrap().ledger().venue().cash(), atoms(0));
        assert_eq!(
            j.state()
                .unwrap()
                .ledger()
                .book(Owner::Customer(user(1)))
                .unwrap()
                .cash(),
            atoms(20)
        );
        drop(j);
        let mut j = open(&t);
        assert!(
            !c.observe_setup_reads(&mut j, &g, &demo_tests::policy(), id(42), 2100)
                .unwrap()
        );
        assert_eq!(j.head(), head);
        let mut changed = demo_tests::policy();
        changed.revision += 1;
        assert!(
            c.prepare_setup_poll(&mut j, &g, &changed, poll(43, 2100))
                .is_err()
        );
        let changed = Controller::new(
            profile(),
            Route {
                epoch: 2,
                ..route()
            },
            Zeroizing::new([9; 32]),
        )
        .unwrap();
        assert!(
            changed
                .prepare_setup_poll(&mut j, &g, &demo_tests::policy(), poll(44, 2100))
                .is_err()
        );
    }
    #[test]
    fn missing_cache_malformed_and_future_sources_never_supply_setup() {
        for case in 0..5 {
            let t = Temp::new();
            let (mut j, c, g) = unqualified(&t);
            let mut bodies = bodies();
            match case {
                0 => bodies[1] = json!({"success":false,"error":"Account loan cache not found"}),
                1 => {
                    bodies[2]["data"]
                        .as_object_mut()
                        .unwrap()
                        .remove("account_equity");
                }
                2 => bodies[1]["data"]["updated_at"] = json!(1200),
                3 => bodies[2]["data"]["balance"] = json!("1e0"),
                _ => bodies[0]["data"]["error"] = json!("conflicting response"),
            }
            for (i, body) in bodies.into_iter().enumerate() {
                read(
                    &mut j,
                    &c,
                    &g,
                    30 + 2 * i as u8,
                    100 + 1000 * i as u64,
                    if case == 0 && i == 1 { 404 } else { 200 },
                    body,
                );
            }
            let head = j.head();
            assert!(
                !c.observe_setup_reads(&mut j, &g, &demo_tests::policy(), id(41), 2100)
                    .unwrap(),
                "case {case}"
            );
            assert_eq!(j.head(), head);
            assert!(!j.state().unwrap().native_funding_ready());
        }
    }
    #[test]
    fn lost_setup_requests_keep_original_budget_cadence_and_no_signed_action() {
        let t = Temp::new();
        let (mut j, c, g) = unqualified(&t);
        for i in 0..8 {
            let at = 100 + 1000 * i;
            let setup::Step::Request(request, completion) = c
                .prepare_setup_poll(
                    &mut j,
                    &g,
                    &demo_tests::policy(),
                    poll(30 + 2 * i as u8, at),
                )
                .unwrap()
            else {
                panic!("expected bounded GET")
            };
            request.consume(at).unwrap();
            reads::complete(&mut j, &g, *completion, Reply::Unknown).unwrap();
            assert!(matches!(
                c.prepare_setup_poll(
                    &mut j,
                    &g,
                    &demo_tests::policy(),
                    poll(60 + 2 * i as u8, at)
                )
                .unwrap(),
                setup::Step::Waiting | setup::Step::Exhausted
            ));
            drop(j);
            j = open(&t);
        }
        assert!(matches!(
            c.prepare_setup_poll(&mut j, &g, &demo_tests::policy(), poll(90, 8100))
                .unwrap(),
            setup::Step::Exhausted
        ));
        assert!(j.state().unwrap().attempts().is_empty());
        assert_eq!(j.state().unwrap().unresolved_raw(), 0);
        assert!(!j.state().unwrap().native_funding_ready());
    }
    #[test]
    fn adverse_setup_remains_visible_and_deadline_cannot_refresh_from_retained_replies() {
        let t = Temp::new();
        let (mut j, c, g) = unqualified(&t);
        let mut replies = bodies();
        replies[0]["data"]["margin_settings"] =
            json!([{"symbol":"BTC","isolated":true,"leverage":20}]);
        replies[1]["data"]["borrowed"] = json!("2");
        replies[1]["data"]["pending_interest"] = json!("1");
        replies[2]["data"]["stop_orders_count"] = json!(1);
        for (i, body) in replies.into_iter().enumerate() {
            read(
                &mut j,
                &c,
                &g,
                30 + 2 * i as u8,
                100 + 1000 * i as u64,
                200,
                body,
            );
        }
        let setup::Step::Observed(observed) = c
            .prepare_setup_poll(&mut j, &g, &demo_tests::policy(), poll(40, 2100))
            .unwrap()
        else {
            panic!("adverse data must remain observable")
        };
        assert!(!observed.idle && !observed.compatible_margin);
        assert_eq!((observed.borrowed, observed.interest), (2, 1));
        assert!(
            c.observe_setup_reads(&mut j, &g, &demo_tests::policy(), id(41), 2100)
                .unwrap()
        );
        assert!(!j.state().unwrap().native_funding_ready());
        let head = j.head();
        assert!(
            !c.observe_setup_reads(&mut j, &g, &demo_tests::policy(), id(42), 9100)
                .unwrap()
        );
        assert!(matches!(
            c.prepare_setup_poll(&mut j, &g, &demo_tests::policy(), poll(43, 9100))
                .unwrap(),
            setup::Step::Exhausted
        ));
        assert_eq!(j.head(), head);
    }
    #[test]
    fn setup_rate_limit_and_oversized_reply_do_not_refund_original_read() {
        for oversized in [false, true] {
            let t = Temp::new();
            let (mut j, c, g) = unqualified(&t);
            let setup::Step::Request(request, completion) = c
                .prepare_setup_poll(&mut j, &g, &demo_tests::policy(), poll(30, 100))
                .unwrap()
            else {
                panic!("expected finite GET")
            };
            request.consume(100).unwrap();
            let head = j.head();
            let result = reads::complete(
                &mut j,
                &g,
                *completion,
                Reply::Response {
                    status: if oversized { 200 } else { 429 },
                    received_at: 100,
                    retry_after_ms: Some(3000),
                    body: raw(&if oversized {
                        vec![b' '; demo::MAX_BODY + 1]
                    } else {
                        b"limited".to_vec()
                    }),
                },
            );
            if oversized {
                assert!(result.is_err());
                assert_eq!(j.head(), head);
            } else {
                assert!(matches!(result.unwrap(), reads::Outcome::Limited));
            }
            assert!(
                !c.observe_setup_reads(&mut j, &g, &demo_tests::policy(), id(40), 1100)
                    .unwrap()
            );
            if !oversized {
                drop(j);
                j = open(&t);
                assert!(matches!(
                    c.prepare_setup_poll(&mut j, &g, &demo_tests::policy(), poll(41, 1100))
                        .unwrap(),
                    setup::Step::Waiting
                ));
            }
            assert!(!j.state().unwrap().native_funding_ready());
            assert!(j.state().unwrap().attempts().is_empty());
        }
    }
}

#[test]
fn original_ingress_preparation_uses_existing_authorization_and_never_exposes_or_retries() {
    let t = Temp::new();
    let (mut j, c, _) = setup(&t, false);
    let intent = lifecycle::Intent {
        request: a(10).request,
        source: Location::Vault,
        destination: Destination::Location(Location::Broker),
        net: atoms(20),
        maximum_fee: atoms(0),
        fee_payer: Owner::House,
        allow_partial: false,
        policy: config().policy,
        authority_epoch: 1,
        expires_at: 10_000,
    };
    let approval = orders::Approval {
        account: user(1),
        intent_hash: intent.digest().unwrap(),
        authority_epoch: 1,
    };
    let accepted = tx(
        &j,
        10,
        vec![],
        vec![Control::Funds(lifecycle::Action::Accept {
            intent: Box::new(intent),
            approval,
        })],
    );
    assert_eq!(j.commit(accepted).unwrap().receipt.controls, None);
    let mut incomplete = good_setup();
    incomplete.complete = false;
    c.observe_setup(&mut j, id(11), 100, incomplete).unwrap();
    let before = j.head();
    assert!(
        c.prepare_ingress(
            &mut j,
            Dispatch {
                attempt: a(10),
                commit: id(12),
                at: 100
            }
        )
        .is_err()
    );
    assert_eq!(j.head(), before);
    c.observe_setup(&mut j, id(13), 100, good_setup()).unwrap();
    assert_eq!(
        c.prepare_ingress(
            &mut j,
            Dispatch {
                attempt: a(10),
                commit: id(14),
                at: 100
            }
        )
        .unwrap(),
        Rail::Release
    );
    let original = j.head();
    assert!(
        !j.state()
            .unwrap()
            .attempts()
            .iter()
            .find(|record| record.key == a(10))
            .unwrap()
            .possibly_exposed
    );
    assert_eq!(j.state().unwrap().ledger().vault(), atoms(120));
    assert_eq!(j.state().unwrap().ledger().broker(), atoms(0));
    drop(j);
    let mut j = open(&t);
    assert!(
        c.prepare_ingress(
            &mut j,
            Dispatch {
                attempt: a(10),
                commit: id(15),
                at: 100
            }
        )
        .is_err()
    );
    assert!(
        c.prepare_ingress(
            &mut j,
            Dispatch {
                attempt: a(16),
                commit: id(16),
                at: 100
            }
        )
        .is_err()
    );
    assert_eq!(j.head(), original);
    let action = c
        .expose_chain(
            &mut j,
            Dispatch {
                attempt: a(10),
                commit: id(17),
                at: 100,
            },
            Rail::Release,
            counters(Rail::Release, 0, 1),
        )
        .unwrap();
    assert_eq!(action.plan().amount(), 20);
}
