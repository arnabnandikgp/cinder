//! Qualified synthetic evidence resolution; not source authentication/finality.
mod support;
use cinder_journal::{model::*, raw::*, wire, *};
use cinder_kernel::{
    identity::*,
    ledger::{evidence::Disposition, *},
};
use sha2::{Digest, Sha256};
use support::*;

fn retain(s: &mut Store, n: u8) {
    let mut tx = transaction(s.head(), n, vec![], vec![]);
    tx.inputs.push(Input {
        source: config().sources[0].scope,
        source_cut: Some(10),
        authority_epoch: 1,
        observed_at: 10,
        raw: raw(b"exact retained source input"),
        event: None,
    });
    assert_eq!(
        s.commit(tx).unwrap().receipt.inputs,
        [InputResult::Unnormalized]
    );
}
fn proof() -> PrivateBytes {
    raw(b"synthetic authenticated complete no-effect qualification")
}
fn resolution(s: &Store, index: usize) -> Resolution {
    let entry = &s.state().unwrap().raw_inputs()[index];
    Resolution {
        key: entry.key,
        fingerprint: entry.fingerprint,
        source: entry.source,
        authority_epoch: 1,
        through: 10,
        observed_at: 10,
        evidence: Sha256::digest(proof().as_bytes()).into(),
        no_effect: true,
        effects: vec![],
    }
}
fn resolve(s: &mut Store, n: u8, r: Resolution, events: Vec<Event>) -> Committed {
    let mut tx = transaction(s.head(), n, events, vec![Control::ResolveRaw(r)]);
    tx.evidence.push(proof());
    assert_eq!(
        wire::decode_transaction(&wire::encode_transaction(&tx).unwrap()).unwrap(),
        tx
    );
    s.commit(tx).unwrap()
}

#[test]
fn each_exact_resolution_is_once_and_unrelated_raw_faults_remain_blocked() {
    let t = Temp::new();
    let mut s = t.create();
    retain(&mut s, 1);
    retain(&mut s, 2);
    let first = resolution(&s, 0);
    assert_eq!(
        resolve(&mut s, 3, first.clone(), vec![]).receipt.controls,
        None
    );
    assert_eq!(s.state().unwrap().unresolved_raw(), 1);
    assert_eq!(
        resolve(&mut s, 4, first.clone(), vec![]).receipt.controls,
        None
    );
    assert_eq!(s.state().unwrap().unresolved_raw(), 1);
    let mut conflict = first;
    conflict.through = 11;
    assert_eq!(
        resolve(&mut s, 5, conflict, vec![]).receipt.controls,
        Some(ControlError::Invalid)
    );
    let state = s.state().unwrap().clone();
    drop(s);
    let mut reopened = t.open();
    assert_eq!(reopened.state().unwrap(), &state);
    let second = resolution(&reopened, 1);
    assert_eq!(
        resolve(&mut reopened, 6, second, vec![]).receipt.controls,
        None
    );
    assert_eq!(reopened.state().unwrap().unresolved_raw(), 0);
    let original = reopened
        .transaction(CommitId::new([1; 32]).unwrap())
        .unwrap();
    assert_eq!(
        original.inputs[0].raw.as_bytes(),
        b"exact retained source input"
    );
    let tx = reopened
        .transaction(CommitId::new([6; 32]).unwrap())
        .unwrap()
        .clone();
    assert!(reopened.commit(tx).unwrap().duplicate);
    assert_eq!(reopened.state().unwrap().unresolved_raw(), 0);
}

#[test]
fn wrong_identity_scope_fingerprint_authority_coverage_or_proof_cannot_clear_raw() {
    for case in 0..13 {
        let t = Temp::new();
        let mut s = t.create();
        retain(&mut s, 1);
        let mut r = resolution(&s, 0);
        match case {
            0 => r.key.ordinal = 1,
            1 => r.key.transaction = CommitId::new([90; 32]).unwrap(),
            2 => r.fingerprint[0] ^= 1,
            3 => r.source.namespace = NamespaceId::new([90; 32]).unwrap(),
            4 => r.authority_epoch = 0,
            5 => r.authority_epoch = 2,
            6 => r.through = 9,
            7 => r.observed_at = 9,
            8 => r.observed_at = 11,
            9 => r.evidence = [0; 32],
            10 => r.evidence = [90; 32],
            11 => r.no_effect = false,
            _ => {
                r.no_effect = false;
                r.effects.push(key(20));
            }
        }
        assert!(resolve(&mut s, 2, r, vec![]).receipt.controls.is_some());
        assert_eq!(s.state().unwrap().unresolved_raw(), 1);
        assert!(s.state().unwrap().raw_inputs()[0].resolved.is_none());
        assert_eq!(s.state().unwrap().ledger().venue().cash(), cash(0));
    }
}

#[test]
fn normalized_effect_and_resolution_share_atomic_journal_and_duplicate_posting_identity() {
    let t = Temp::new();
    let mut s = t.create();
    retain(&mut s, 1);
    let mut r = resolution(&s, 0);
    r.no_effect = false;
    r.effects = vec![key(20)];
    let result = resolve(
        &mut s,
        2,
        r.clone(),
        vec![receipt(20, Owner::Customer(user(1)), 7)],
    );
    assert_eq!(
        result.receipt.inputs,
        [InputResult::Normalized(Disposition::Applied)]
    );
    assert_eq!(result.receipt.controls, None);
    assert_eq!(s.state().unwrap().unresolved_raw(), 0);
    assert_eq!(s.state().unwrap().ledger().venue().cash(), cash(7));
    let state = s.state().unwrap().clone();
    drop(s);
    let mut reopened = t.open();
    assert_eq!(reopened.state().unwrap(), &state);
    assert_eq!(resolve(&mut reopened, 3, r, vec![]).receipt.controls, None);
    assert_eq!(reopened.state().unwrap().ledger().venue().cash(), cash(7));
}

#[test]
fn failed_later_control_keeps_facts_but_cannot_accept_resolution_or_hide_other_faults() {
    let t = Temp::new();
    let mut s = t.create();
    retain(&mut s, 1);
    let mut r = resolution(&s, 0);
    r.no_effect = false;
    r.effects = vec![key(20)];
    let mut tx = transaction(
        s.head(),
        2,
        vec![receipt(20, Owner::Customer(user(1)), 7)],
        vec![Control::ResolveRaw(r), Control::Release(request(80))],
    );
    tx.evidence.push(proof());
    assert!(s.commit(tx).unwrap().receipt.controls.is_some());
    assert_eq!(s.state().unwrap().ledger().venue().cash(), cash(7));
    assert_eq!(s.state().unwrap().unresolved_raw(), 1);
    assert!(s.state().unwrap().raw_inputs()[0].resolved.is_none());
    // A rejected envelope is not an authenticated no-effect finding.
    let mut rejected = transaction(s.head(), 3, vec![], vec![]);
    rejected.inputs.push(Input {
        source: config().sources[0].scope,
        source_cut: Some(10),
        authority_epoch: 0,
        observed_at: 10,
        raw: raw(b"rejected envelope"),
        event: None,
    });
    assert_eq!(
        s.commit(rejected).unwrap().receipt.inputs,
        [InputResult::EnvelopeRejected]
    );
    let mut wrong = resolution(&s, 1);
    wrong.authority_epoch = 0;
    assert!(resolve(&mut s, 4, wrong, vec![]).receipt.controls.is_some());
    assert_eq!(s.state().unwrap().unresolved_raw(), 2);
}

#[test]
fn applied_effect_needs_current_qualified_coverage_and_duplicate_cannot_release_holds() {
    for case in 0..5 {
        let t = Temp::new();
        let mut s = t.create();
        seed(&mut s);
        hold(&mut s);
        retain(&mut s, 3);
        let mut r = resolution(&s, 0);
        r.no_effect = false;
        r.effects = vec![key(20)];
        let fact = receipt(20, Owner::Customer(user(1)), 7);
        s.commit(transaction(s.head(), 4, vec![fact.clone()], vec![]))
            .unwrap();
        let mut tx = transaction(s.head(), 5, vec![fact], vec![Control::ResolveRaw(r)]);
        tx.evidence.push(proof());
        match case {
            0 => {} // An exact duplicate at a qualified cut can resolve, not repost.
            1 => tx.inputs.clear(),
            2 => tx.inputs[0].source_cut = None,
            3 => tx.inputs[0].source_cut = Some(11),
            _ => tx.controls.push(Control::Release(request(2))),
        }
        let result = s.commit(tx).unwrap();
        assert_eq!(result.receipt.controls.is_none(), case == 0);
        assert_eq!(s.state().unwrap().unresolved_raw(), u64::from(case != 0));
        assert_eq!(s.state().unwrap().ledger().venue().cash(), cash(207));
        assert!(s.state().unwrap().holds()[0].active);
        let state = s.state().unwrap().clone();
        drop(s);
        assert_eq!(t.open().state().unwrap(), &state);
    }
}

#[test]
fn exposure_resumes_only_after_all_exact_inputs_resolve() {
    let t = Temp::new();
    let mut s = t.create();
    seed(&mut s);
    hold(&mut s);
    retain(&mut s, 3);
    retain(&mut s, 4);
    let first = resolution(&s, 0);
    resolve(&mut s, 5, first, vec![]);
    let result = s
        .commit(transaction(
            s.head(),
            6,
            vec![],
            vec![Control::Expose(attempt(2))],
        ))
        .unwrap();
    assert_eq!(result.receipt.controls, Some(ControlError::Unqualified));
    assert!(result.exposures.is_empty());
    assert!(!s.state().unwrap().attempts()[0].possibly_exposed);
    let second = resolution(&s, 1);
    resolve(&mut s, 7, second, vec![]);
    let result = s
        .commit(transaction(
            s.head(),
            8,
            vec![],
            vec![Control::Expose(attempt(2))],
        ))
        .unwrap();
    assert_eq!(result.receipt.controls, None);
    assert_eq!(result.exposures.len(), 1);
    let state = s.state().unwrap().clone();
    drop(s);
    assert_eq!(t.open().state().unwrap(), &state);
}

#[test]
fn resolution_work_budget_rejects_before_append_and_missing_proof_never_clears() {
    let t = Temp::new();
    let mut s = t.create();
    retain(&mut s, 1);
    let r = resolution(&s, 0);
    let before = s.head();
    let tx = transaction(s.head(), 2, vec![], vec![Control::ResolveRaw(r.clone())]);
    assert_eq!(
        s.commit(tx).unwrap().receipt.controls,
        Some(ControlError::Unqualified)
    );
    assert_eq!(s.state().unwrap().unresolved_raw(), 1);
    let mut too_large = r;
    too_large.no_effect = false;
    too_large.effects = vec![key(20); wire::MAX_ITEMS];
    let tx = transaction(s.head(), 3, vec![], vec![Control::ResolveRaw(too_large)]);
    let head = s.head();
    assert_eq!(s.commit(tx).unwrap_err(), Error::Limit);
    assert_eq!(s.head(), head);
    assert_ne!(s.head(), before);
    assert_eq!(s.state().unwrap().unresolved_raw(), 1);
}
