use super::*;
use crate::solvency::tests::{user, view};
use crate::transaction::{anchor_ix, decode_ix, test_receipt};
use anchor_lang::prelude::Pubkey;
use cinder_ledger::Residual;
use phoenix_rise_math::{SignedBaseLots, SignedQuoteLotsPerBaseLot};

pub(crate) struct Fixture {
    pub book: Book,
    pub users: Vec<([u8; 32], UserLedger)>,
    pub native: RiseView,
}
impl Fixture {
    pub fn new(lots: [i64; 2], rate: i64, generation: u64) -> Self {
        let users = lots
            .into_iter()
            .enumerate()
            .map(|(i, lots)| {
                let mut l = user(lots, 200);
                l.user = Pubkey::new_from_array([i as u8 + 1; 32]);
                l.nonce = u64::from(lots != 0);
                let (address, bump) = Pubkey::find_program_address(
                    &[cc::SEED_USER, l.user.as_ref()],
                    &cinder_ledger::ID,
                );
                l.bump = bump;
                (address.to_bytes(), l)
            })
            .collect();
        let net = lots.into_iter().sum::<i64>();
        let mut book = Book {
            schema_version: cc::ACCOUNT_SCHEMA_VERSION,
            residual_len: u8::from(net != 0),
            residuals: [Residual::default(); cc::MAX_BOOK_MARKETS],
            phoenix_collateral: 0,
            last_ack_slot_er: u64::from(lots != [0, 0]),
            invariant_ok: 1,
            halt: cc::OPERATOR_DOWN,
            funding_epoch: 0,
            last_scan_ms: 0,
            bump: 0,
        };
        if net != 0 {
            book.residuals[0] = Residual {
                asset_id: 1,
                lots: net,
            };
        }
        let mut native = view(100, 400);
        native.positions = BTreeMap::from([(1, net)]);
        native.entry_quote_lots = BTreeMap::from([(1, i128::from(net) * 100_000_000)]);
        native.halt = cc::OPERATOR_DOWN;
        native.slot = generation;
        native.observed_ms = generation * 1000;
        native.mark_ms = native.observed_ms;
        native.funding_updates_seconds = BTreeMap::from([(1, generation)]);
        native.markets.get_mut(&1).unwrap().cumulative_funding_rate =
            SignedQuoteLotsPerBaseLot::new(rate);
        Self {
            book,
            users,
            native,
        }
    }
    pub fn checkpoint(&self) -> FundingCheckpoint {
        FundingCheckpoint::capture(
            &self.native,
            &self.book,
            &self.users,
            self.native.observed_ms,
        )
        .unwrap()
    }
    pub fn advance(&mut self, rate: i64, generation: u64) {
        self.native.slot = generation;
        self.native.observed_ms = generation * 1000;
        self.native.mark_ms = self.native.observed_ms;
        self.native.funding_updates_seconds.insert(1, generation);
        self.native
            .markets
            .get_mut(&1)
            .unwrap()
            .cumulative_funding_rate = SignedQuoteLotsPerBaseLot::new(rate);
    }
    pub fn plan(&self, old: &FundingCheckpoint) -> FundingEpochPlan {
        FundingEpochPlan::build(
            old,
            &self.checkpoint(),
            &self.book,
            &self.users,
            self.native.observed_ms,
        )
        .unwrap()
    }
}
pub(crate) fn observation(
    plan: &FundingEpochPlan,
    scope: [u8; 32],
    signature: [u8; 64],
    succeeded: bool,
) -> FundingStepObservation {
    let body = plan.instruction(&scope).unwrap().to_vec();
    let mut accounts = vec![([7; 32], true, false), (config_key(), false, false)];
    accounts.push((scope, false, true));
    if scope != book_key() {
        accounts.push((book_key(), false, false));
    }
    let ix = anchor_ix(ledger_id(), accounts, body.clone());
    FundingStepObservation::decode(
        &test_receipt(&ix, &signature, succeeded),
        &signature,
        &[7; 32],
        plan.epoch(),
        scope,
        Sha256::digest(body).into(),
    )
    .unwrap()
}
fn entries(plan: &FundingEpochPlan, scope: [u8; 32]) -> Vec<FundingEntry> {
    let p =
        decode_ix::<cinder_ledger::instruction::AllocateFunding>(plan.instruction(&scope).unwrap())
            .unwrap()
            .unwrap();
    assert!(!p.fold);
    p.entries
}

#[test]
fn native_sign_units_and_gross_to_net_conservation() {
    for rate in [-1000, -1, 0, 1, 1000] {
        for lots in [[10, -4], [10, -10], [-10, 4], [0, 0]] {
            let mut f = Fixture::new(lots, -23, 1);
            let old = f.checkpoint();
            f.advance(rate, 2);
            let plan = f.plan(&old);
            let mut total = 0;
            for ((address, _), lots) in f.users.iter().zip(lots) {
                let es = entries(&plan, *address);
                let delta = es.first().map_or(0, |e| e.delta_usdc);
                let sdk = -(SignedQuoteLotsPerBaseLot::new(rate - (-23))
                    * SignedBaseLots::new(lots))
                .as_inner();
                assert_eq!(delta, sdk);
                assert_eq!(es.len(), usize::from(lots != 0));
                total += delta;
            }
            assert_eq!(total, -(rate + 23) * lots.into_iter().sum::<i64>());
            assert_eq!(plan.steps.len(), 3); // flat users advance too
        }
    }
}

#[test]
fn partially_applied_epoch_reconstructs_identical_bodies() {
    let mut f = Fixture::new([10, -4], 0, 1);
    let old = f.checkpoint();
    f.advance(100, 2);
    let plan = f.plan(&old);
    f.book.funding_epoch = 1;
    f.users[0].1.last_funding_epoch = 1;
    f.users[0].1.positions[0].unsettled_funding = -1000;
    f.users[0].1.free -= 10;
    let resumed = FundingEpochPlan::resume(&old, &plan.target, &f.book, &f.users).unwrap();
    assert_eq!(plan.steps, resumed.steps);
    f.book.residuals[0].lots += 1;
    assert!(FundingEpochPlan::resume(&old, &plan.target, &f.book, &f.users).is_err());
    f.book.residuals[0].lots -= 1;
    f.book.halt = 0;
    assert!(FundingEpochPlan::resume(&old, &plan.target, &f.book, &f.users).is_err());
    f.book.halt = cc::OPERATOR_DOWN;
    f.users[0].1.nonce += 1; // even a round-trip with the same endpoint lots
    assert!(FundingEpochPlan::resume(&old, &plan.target, &f.book, &f.users).is_err());
}

#[test]
fn existing_user_funding_cannot_overflow_during_accrual() {
    let mut f = Fixture::new([10, -4], 0, 1);
    let old = f.checkpoint();
    f.advance(100, 2);
    f.users[0].1.positions[0].unsettled_funding = i64::MIN + 500;
    assert!(FundingEpochPlan::build(&old, &f.checkpoint(), &f.book, &f.users, 2000).is_err());
}

#[test]
fn inventory_changes_never_charge_current_size_across_missed_updates() {
    let mut f = Fixture::new([10, -4], 0, 1);
    let old = f.checkpoint();
    f.advance(100, 2);
    f.users[0].1.positions[0].lots = 11;
    f.users[0].1.positions[0].entry_quote_lots = 1_100_000_000;
    f.book.residuals[0].lots = 7;
    f.native.positions.insert(1, 7);
    assert!(FundingEpochPlan::build(&old, &f.checkpoint(), &f.book, &f.users, 2000).is_err());
}

#[test]
fn real_generation_required_even_when_accumulator_is_equal() {
    let mut f = Fixture::new([10, -4], 0, 1);
    let old = f.checkpoint();
    assert!(FundingEpochPlan::build(&old, &old, &f.book, &f.users, 1000).is_err());
    f.advance(100, 1); // contradictory rate, same update generation
    assert!(FundingEpochPlan::build(&old, &f.checkpoint(), &f.book, &f.users, 1000).is_err());
    f.advance(0, 2);
    let plan = f.plan(&old);
    assert_eq!(entries(&plan, f.users[0].0)[0].delta_usdc, 0);
    f.advance(0, 3); // quiet catch-up across multiple updates is exact
    assert!(FundingEpochPlan::build(&old, &f.checkpoint(), &f.book, &f.users, 3000).is_ok());
}

#[test]
fn stale_open_pending_future_or_overflow_snapshots_fail_closed() {
    let mut f = Fixture::new([10, -4], i64::MIN, 1);
    let old = f.checkpoint();
    f.advance(i64::MAX, 2);
    assert!(FundingEpochPlan::build(&old, &f.checkpoint(), &f.book, &f.users, 2000).is_err());
    assert!(FundingCheckpoint::capture(&f.native, &f.book, &f.users, 5001).is_err());
    assert!(FundingCheckpoint::capture(&f.native, &f.book, &f.users, 1999).is_err());
    f.users[0].1.pending_oid_count = 1;
    assert!(FundingCheckpoint::capture(&f.native, &f.book, &f.users, 2000).is_err());
    f.users[0].1.pending_oid_count = 0;
    f.native.halt = 0;
    assert!(FundingEpochPlan::build(&old, &f.checkpoint(), &f.book, &f.users, 2000).is_err());
    f.native.halt = cc::OPERATOR_DOWN;
    f.native.funding_updates_seconds.insert(1, 3);
    assert!(FundingCheckpoint::capture(&f.native, &f.book, &f.users, 2000).is_err());
}

#[test]
fn bootstrap_requires_never_traded_flat_zero_funding_backing() {
    let mut f = Fixture::new([0, 0], 99, 1);
    assert!(f.checkpoint().bootstrap_safe);
    f.users[0].1.nonce = 1;
    assert!(!f.checkpoint().bootstrap_safe);
    f.users[0].1.nonce = 0;
    f.native.funding = 1;
    assert!(!f.checkpoint().bootstrap_safe);
    f.native.funding = 0;
    f.native.vault_balance -= 1;
    assert!(!f.checkpoint().bootstrap_safe);
    assert!(!Fixture::new([10, -10], 0, 1).checkpoint().bootstrap_safe);
}

#[test]
fn receipts_join_signature_program_epoch_scope_and_immutable_body() {
    let mut f = Fixture::new([10, -4], 0, 1);
    let old = f.checkpoint();
    f.advance(100, 2);
    let plan = f.plan(&old);
    for scope in [book_key(), f.users[0].0] {
        let o = observation(&plan, scope, [8; 64], true);
        assert!(o.succeeded);
        assert!(!observation(&plan, scope, [8; 64], false).succeeded);
        let mut accounts = vec![
            ([7; 32], true, false),
            (config_key(), false, false),
            (scope, false, true),
        ];
        if scope != book_key() {
            accounts.push((book_key(), false, false));
        }
        let body = plan.instruction(&scope).unwrap().to_vec();
        let ix = anchor_ix(ledger_id(), accounts.clone(), body.clone());
        let receipt = test_receipt(&ix, &[8; 64], true);
        for (signature, adapter, epoch, address, hash) in [
            ([9; 64], [7; 32], 1, scope, o.body_hash),
            ([8; 64], [6; 32], 1, scope, o.body_hash),
            ([8; 64], [7; 32], 2, scope, o.body_hash),
            ([8; 64], [7; 32], 1, [6; 32], o.body_hash),
            ([8; 64], [7; 32], 1, scope, [0; 32]),
        ] {
            assert!(FundingStepObservation::decode(
                &receipt, &signature, &adapter, epoch, address, hash
            )
            .is_err());
        }
        let wrong_program = anchor_ix([6; 32], accounts.clone(), body.clone());
        assert!(FundingStepObservation::decode(
            &test_receipt(&wrong_program, &[8; 64], true),
            &[8; 64],
            &[7; 32],
            1,
            scope,
            o.body_hash
        )
        .is_err());
        accounts[1].0 = [6; 32];
        let wrong_config = anchor_ix(ledger_id(), accounts, body);
        assert!(FundingStepObservation::decode(
            &test_receipt(&wrong_config, &[8; 64], true),
            &[8; 64],
            &[7; 32],
            1,
            scope,
            o.body_hash
        )
        .is_err());
        let mut zero_slot = receipt.clone();
        zero_slot["slot"] = 0.into();
        assert!(FundingStepObservation::decode(
            &zero_slot,
            &[8; 64],
            &[7; 32],
            1,
            scope,
            o.body_hash
        )
        .is_err());
        let mut extra = receipt;
        let ix = extra["transaction"]["message"]["instructions"][0].clone();
        extra["transaction"]["message"]["instructions"]
            .as_array_mut()
            .unwrap()
            .push(ix);
        assert!(
            FundingStepObservation::decode(&extra, &[8; 64], &[7; 32], 1, scope, o.body_hash)
                .is_err()
        );
    }
}
