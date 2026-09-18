use anchor_lang::prelude::Pubkey;
use cinder_common as cc;
use cinder_ledger::{Book, OpenOid, Position, Residual, UserLedger};
use cinder_operator::{build_reserve_snapshot, RuntimeError};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

fn book() -> Book {
    Book {
        schema_version: cc::ACCOUNT_SCHEMA_VERSION,
        residual_len: 0,
        residuals: [Residual::default(); cc::MAX_BOOK_MARKETS],
        phoenix_collateral: 0,
        last_ack_slot_er: 0,
        invariant_ok: 1,
        halt: 0,
        funding_epoch: 7,
        last_scan_ms: 1000,
        bump: 0,
    }
}
fn user(id: u8, free: u64, reserved: u64, debt: u64) -> ([u8; 32], UserLedger) {
    let user = Pubkey::new_from_array([id; 32]);
    let (pda, bump) =
        Pubkey::find_program_address(&[cc::SEED_USER, user.as_ref()], &cinder_ledger::ID);
    (
        pda.to_bytes(),
        UserLedger {
            schema_version: cc::ACCOUNT_SCHEMA_VERSION,
            user,
            free,
            reserved,
            withdrawable: 0,
            bad_debt_usdc: debt,
            pending_oid_count: 0,
            nonce: 0,
            last_funding_epoch: 7,
            positions_len: 0,
            positions: [Position::default(); cc::MAX_USER_POSITIONS],
            open_oids: [OpenOid::default(); cc::MAX_OPEN_OIDS_PER_USER],
            bump,
        },
    )
}
fn registry(users: &[([u8; 32], UserLedger)]) -> BTreeMap<[u8; 32], [u8; 32]> {
    users
        .iter()
        .map(|(pda, ledger)| (*pda, ledger.user.to_bytes()))
        .collect()
}

#[test]
fn canonical_bytes_follow_frozen_claim_layout_and_include_bad_debt() {
    let users = [user(2, 30, 40, 5), user(1, 10, 20, 3)];
    let s = build_reserve_snapshot(&book(), &users, &registry(&users)).unwrap();
    let mut bytes = b"cinder:cash-claims:v1".to_vec();
    bytes.push(1);
    bytes.extend(2u32.to_le_bytes());
    for (id, free, reserved, debt) in [(1u8, 10u64, 20u64, 3u64), (2, 30, 40, 5)] {
        bytes.extend([id; 32]);
        for value in [free, reserved, debt] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.push(0); // n_pos
    }
    assert_eq!(s.root, <[u8; 32]>::from(Sha256::digest(bytes)));
    assert_eq!(
        (
            s.user_count,
            s.total_free,
            s.total_reserved,
            s.total_bad_debt
        ),
        (2, 40, 60, 8)
    );
    assert_eq!(s.signed_cash_total(), 92);
    let reversed = [user(1, 10, 20, 3), user(2, 30, 40, 5)];
    assert_eq!(
        s,
        build_reserve_snapshot(&book(), &reversed, &registry(&reversed)).unwrap()
    );
}

#[test]
fn position_rows_sort_by_asset_and_root_is_not_an_equity_or_pnl_claim() {
    let make = |reverse: bool| {
        let (address, mut ledger) = user(1, 10, 20, 3);
        ledger.positions_len = 2;
        let positions = [
            Position {
                asset_id: 1,
                lots: 10,
                entry_quote_lots: 100,
                ..Default::default()
            },
            Position {
                asset_id: 2,
                lots: -5,
                entry_quote_lots: -50,
                ..Default::default()
            },
        ];
        for (i, p) in positions.into_iter().enumerate() {
            ledger.positions[if reverse { 1 - i } else { i }] = p;
        }
        (address, ledger)
    };
    let mut b = book();
    b.residual_len = 2;
    b.residuals[0] = Residual {
        asset_id: 1,
        lots: 10,
    };
    b.residuals[1] = Residual {
        asset_id: 2,
        lots: -5,
    };
    let users = [make(false)];
    let s = build_reserve_snapshot(&b, &users, &registry(&users)).unwrap();
    let reversed = [make(true)];
    assert_eq!(
        s,
        build_reserve_snapshot(&b, &reversed, &registry(&reversed)).unwrap()
    );
    let mut changed = [make(false)];
    changed[0].1.positions[0].entry_quote_lots += 1;
    changed[0].1.positions[0].unsettled_funding = 7;
    assert_eq!(
        s.root,
        build_reserve_snapshot(&b, &changed, &registry(&changed))
            .unwrap()
            .root
    );
    changed[0].1.bad_debt_usdc += 1;
    assert_ne!(
        s.root,
        build_reserve_snapshot(&b, &changed, &registry(&changed))
            .unwrap()
            .root
    );
    b.phoenix_collateral += 1;
    assert_ne!(
        s.book_hash,
        build_reserve_snapshot(&b, &users, &registry(&users))
            .unwrap()
            .book_hash
    );
}

#[test]
fn complete_registry_identity_and_uniqueness_are_required() {
    let users = [user(1, 10, 0, 0), user(2, 20, 0, 0)];
    let all = registry(&users);
    assert_eq!(
        build_reserve_snapshot(&book(), &users[..1], &all),
        Err(RuntimeError::Incomplete)
    );
    assert!(build_reserve_snapshot(&book(), &users, &BTreeMap::new()).is_err());
    let mut wrong = registry(&users);
    wrong.insert(users[0].0, [9; 32]);
    assert!(build_reserve_snapshot(&book(), &users, &wrong).is_err());
    let duplicate = [user(1, 10, 0, 0), user(1, 20, 0, 0)];
    assert!(build_reserve_snapshot(&book(), &duplicate, &registry(&duplicate)).is_err());
    let mut wrong_pda = [user(1, 10, 0, 0)];
    wrong_pda[0].0 = [9; 32];
    assert!(build_reserve_snapshot(&book(), &wrong_pda, &registry(&wrong_pda)).is_err());
}

#[test]
fn unfinished_epochs_pending_orders_withdrawals_and_broken_i1_block_snapshot() {
    let mut users = [user(1, 10, 0, 0)];
    let all = registry(&users);
    users[0].1.last_funding_epoch = 6;
    assert_eq!(
        build_reserve_snapshot(&book(), &users, &all),
        Err(RuntimeError::Incomplete)
    );
    users[0].1.last_funding_epoch = 7;
    users[0].1.withdrawable = 1;
    assert_eq!(
        build_reserve_snapshot(&book(), &users, &all),
        Err(RuntimeError::Unsupported)
    );
    users[0].1.withdrawable = 0;
    users[0].1.pending_oid_count = 1;
    users[0].1.open_oids[0] = OpenOid {
        state: cc::OID_PENDING,
        lots_delta: 1,
        ..Default::default()
    };
    assert_eq!(
        build_reserve_snapshot(&book(), &users, &all),
        Err(RuntimeError::Incomplete)
    );
    users[0].1.pending_oid_count = 0;
    users[0].1.open_oids[0] = OpenOid::default();
    let mut b = book();
    b.residual_len = 1;
    b.residuals[0] = Residual {
        asset_id: 1,
        lots: 1,
    };
    assert_eq!(
        build_reserve_snapshot(&b, &users, &all),
        Err(RuntimeError::Identity)
    );
    b = book();
    b.invariant_ok = 0;
    assert!(build_reserve_snapshot(&b, &users, &all).is_err());
}

#[test]
fn totals_overflow_fails_instead_of_wrapping_and_debt_may_exceed_cash() {
    let users = [user(1, u64::MAX, 0, 0), user(2, 1, 0, 0)];
    assert_eq!(
        build_reserve_snapshot(&book(), &users, &registry(&users)),
        Err(RuntimeError::Decode)
    );
    let users = [user(1, 0, 0, 10)];
    let s = build_reserve_snapshot(&book(), &users, &registry(&users)).unwrap();
    assert_eq!(s.signed_cash_total(), -10);
    // Recognition is not authorization to publish a negative aggregate or
    // bypass the runtime's signed I2/release checks.
}

#[test]
fn empty_registry_has_a_reproducible_nonzero_commitment() {
    let s = build_reserve_snapshot(&book(), &[], &BTreeMap::new()).unwrap();
    assert_eq!(s.user_count, 0);
    assert_eq!(s.signed_cash_total(), 0);
    assert_ne!(s.root, [0; 32]);
}
