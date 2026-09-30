//! Offline atomicity, replay, reservation and corruption conformance.
mod support;
use cinder_journal::{model::*, sqlite::*, *};
use cinder_kernel::{
    identity::*,
    ledger::{economics::*, evidence::*, *},
    math::Rounding,
};
use std::sync::{Arc, Barrier};

struct Playback(Vec<Frame>);
impl Backend for Playback {
    fn load(&mut self) -> Result<Vec<Frame>, Error> {
        Ok(self.0.clone())
    }
    fn append(&mut self, _: Option<Head>, _: &Frame) -> Result<(), Error> {
        Err(Error::Storage)
    }
}
fn rehash(frame: &mut Frame) {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"CINDER-OPAQUE-FRAME-1\0");
    h.update(frame.head.sequence.to_be_bytes());
    h.update(frame.previous);
    h.update((frame.opaque.as_bytes().len() as u64).to_be_bytes());
    h.update(frame.opaque.as_bytes());
    frame.head.hash = h.finalize().into();
}

#[test]
fn history_byte_limit_precedes_append_and_exposure_and_survives_reopen() {
    // Deliberately padded fixture protection, not a production cipher. Count
    // opaque bytes (including genesis/overhead), not just transaction payloads.
    struct Padded;
    impl Protection for Padded {
        fn seal(&self, c: RecordContext, p: &[u8]) -> Result<PrivateBytes, Error> {
            let inner = FixtureProtection.seal(c, p)?;
            let mut bytes = (inner.as_bytes().len() as u32).to_be_bytes().to_vec();
            bytes.extend_from_slice(inner.as_bytes());
            bytes.resize(wire::MAX_RECORD, 0);
            PrivateBytes::new(bytes)
        }
        fn open(&self, c: RecordContext, b: &PrivateBytes) -> Result<PrivateBytes, Error> {
            let n = u32::from_be_bytes(b.as_bytes()[..4].try_into().unwrap()) as usize;
            FixtureProtection.open(c, &PrivateBytes::new(b.as_bytes()[4..4 + n].to_vec())?)
        }
    }
    let t = Temp::new();
    let mut s = Journal::create(SqliteBackend::create(&t.db).unwrap(), Padded, config()).unwrap();
    let first = transaction(
        s.head(),
        1,
        vec![receipt(1, Owner::Customer(user(1)), 100)],
        vec![reserve(1, 1), prepare(1)],
    );
    assert_eq!(s.commit(first.clone()).unwrap().receipt.controls, None);
    let frames = MAX_HISTORY_BYTES / wire::MAX_RECORD;
    for n in 2..frames {
        let tx = transaction(s.head(), n.try_into().unwrap(), vec![], vec![]);
        s.commit(tx).unwrap();
    }
    let head = s.head();
    let overflow = transaction(
        head,
        frames.try_into().unwrap(),
        vec![],
        vec![Control::Expose(attempt(1))],
    );
    assert_eq!(s.commit(overflow.clone()).unwrap_err(), Error::Limit);
    assert_eq!(s.head(), head);
    assert!(!s.state().unwrap().attempts()[0].possibly_exposed);
    assert!(s.commit(first).unwrap().duplicate);
    drop(s);
    let mut reopened = Journal::open(
        SqliteBackend::open(&t.db, Migration::None).unwrap(),
        Padded,
        config(),
    )
    .unwrap();
    assert_eq!(reopened.head(), head);
    assert_eq!(reopened.commit(overflow).unwrap_err(), Error::Limit);
    assert_eq!(reopened.head(), head);
    assert!(!reopened.state().unwrap().attempts()[0].possibly_exposed);
}

#[test]
fn replay_rejects_corrupt_bytes_versions_and_recomputed_wrong_projection() {
    let temp = Temp::new();
    let mut s = temp.create();
    seed(&mut s);
    drop(s);
    let frames = SqliteBackend::open(&temp.db, Migration::None)
        .unwrap()
        .load()
        .unwrap();
    let mut corrupt = frames.clone();
    corrupt[1].opaque = raw(b"corrupt");
    assert_eq!(
        Journal::open(Playback(corrupt), FixtureProtection, config()).unwrap_err(),
        Error::Codec
    );
    // Wire revisions and financial-engine revisions are distinct. Both genesis
    // and transactions must reject old engine revisions rather than reinterpret history.
    for (record, index, revision) in [
        (0, 10, 99),
        (0, 13, 99),
        (0, 13, 1),
        (0, 13, 2),
        (0, 13, 3),
        (0, 13, 4),
        (0, 13, 5),
        (0, 13, 6),
        (0, 13, 11),
        (0, 13, 12),
        (0, 13, 7),
        (0, 13, 8),
        (0, 13, 9),
        (0, 13, 13),
        (0, 13, 14),
        (0, 13, 15),
        (0, 13, 16),
        (0, 13, 10),
        (0, 10, 7),
        (0, 10, 6),
        (0, 10, 5),
        (0, 10, 4),
        (0, 10, 3),
        (0, 10, 2),
        (0, 10, 1),
        (1, 10, 99),
        (1, 13, 99),
        (1, 13, 1),
        (1, 13, 2),
        (1, 13, 3),
        (1, 13, 4),
        (1, 13, 5),
        (1, 13, 6),
        (1, 13, 11),
        (1, 13, 12),
        (1, 13, 7),
        (1, 13, 8),
        (1, 13, 9),
        (1, 13, 13),
        (1, 13, 14),
        (1, 13, 15),
        (1, 13, 16),
        (1, 13, 10),
        (1, 10, 7),
        (1, 10, 6),
        (1, 10, 5),
        (1, 10, 4),
        (1, 10, 3),
        (1, 10, 2),
        (1, 10, 1),
    ] {
        let mut modified = frames.clone();
        let f = &mut modified[record];
        let context = RecordContext {
            domain: config().domain,
            sequence: f.head.sequence,
            previous: f.previous,
        };
        let mut bytes = FixtureProtection
            .open(context, &f.opaque)
            .unwrap()
            .as_bytes()
            .to_vec();
        assert_eq!(&bytes[12..14], &17_u16.to_be_bytes());
        bytes[index] = revision;
        f.opaque = FixtureProtection.seal(context, &bytes).unwrap();
        rehash(f);
        assert_eq!(
            Journal::open(Playback(modified), FixtureProtection, config()).unwrap_err(),
            Error::Version
        );
    }
    let mut modified = frames.clone();
    let f = &mut modified[1];
    let context = RecordContext {
        domain: config().domain,
        sequence: f.head.sequence,
        previous: f.previous,
    };
    let mut bytes = FixtureProtection
        .open(context, &f.opaque)
        .unwrap()
        .as_bytes()
        .to_vec();
    *bytes.last_mut().unwrap() ^= 1;
    f.opaque = FixtureProtection.seal(context, &bytes).unwrap();
    rehash(f);
    assert_eq!(
        Journal::open(Playback(modified), FixtureProtection, config()).unwrap_err(),
        Error::Codec
    );
    // Known limitation: a wholly valid older chain passes P05. P06 must add an
    // independent authenticated current head, not advertise this as rollback-safe.
    let old = Journal::open(Playback(frames[..1].to_vec()), FixtureProtection, config()).unwrap();
    assert!(old.state().unwrap().ledger().events().is_empty());
}

#[test]
fn migration_error_does_not_leave_half_a_schema_upgrade() {
    let temp = Temp::new();
    let mut s = temp.create();
    seed(&mut s);
    drop(s);
    let conn = rusqlite::Connection::open(temp.db.join("journal.sqlite3")).unwrap();
    // v1 fixture with a conflicting trigger makes migration fail after head DDL.
    conn.execute_batch("DROP TABLE head; PRAGMA user_version=1;")
        .unwrap();
    assert_eq!(
        SqliteBackend::open(&temp.db, Migration::V1ToV2).unwrap_err(),
        Error::Storage
    );
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, 1);
    let heads: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name='head'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(heads, 0);
    let records: i64 = conn
        .query_row("SELECT count(*) FROM records", [], |r| r.get(0))
        .unwrap();
    assert_eq!(records, 2);
}

#[test]
fn bounded_lock_wait_returns_busy_without_poisoning_or_spending() {
    let temp = Temp::new();
    let mut s = temp.create();
    seed(&mut s);
    let conn = rusqlite::Connection::open(temp.db.join("journal.sqlite3")).unwrap();
    conn.execute_batch("BEGIN IMMEDIATE").unwrap();
    let tx = transaction(s.head(), 3, vec![], vec![reserve(3, 80)]);
    assert_eq!(s.commit(tx.clone()).unwrap_err(), Error::Busy);
    assert!(s.state().unwrap().holds().is_empty());
    conn.execute_batch("ROLLBACK").unwrap();
    assert_eq!(s.commit(tx).unwrap().receipt.controls, None);
}
use support::*;

#[test]
fn replay_retains_financial_facts_dedup_conflicts_and_raw_evidence() {
    let temp = Temp::new();
    let mut s = temp.create();
    seed(&mut s);
    let bind = Event {
        key: RecordKey::Attempt(attempt(3)),
        policy: config().policy,
        change: Change::BindExecution {
            market: q(0).unit(),
            side: Side::Buy,
        },
    };
    let events = vec![
        bind,
        event(
            3,
            Change::Economics(EconomicChange::Execution {
                target: FillTarget::Customer(attempt(3)),
                quantity: q(2),
                price: p(10),
                fee: cash(1),
                pnl: Some(NativePnl::Gross(cash(0))),
            }),
        ),
    ];
    s.commit(transaction(s.head(), 3, events, vec![])).unwrap();
    let version = s.state().unwrap().ledger().version();
    s.commit(transaction(
        s.head(),
        4,
        vec![
            event(
                4,
                Change::Economics(EconomicChange::FundingBoundary {
                    market: q(0).unit(),
                    expected_version: version,
                }),
            ),
            event(
                5,
                Change::Economics(EconomicChange::FundingInputs {
                    boundary: key(4),
                    rate: Some(FundingRate {
                        numerator: cash(-1),
                        denominator: 1,
                        rounding: Rounding::Exact,
                    }),
                    native: Some(cash(-2)),
                }),
            ),
            event(
                6,
                Change::Economics(EconomicChange::FundingSettlement {
                    boundary: key(4),
                    native: cash(-2),
                }),
            ),
        ],
        vec![],
    ))
    .unwrap();
    let tx = transaction(
        s.head(),
        5,
        vec![
            receipt(1, Owner::Customer(user(1)), 100),
            receipt(1, Owner::Customer(user(1)), 101),
            receipt(7, Owner::Customer(user(1)), -1),
        ],
        vec![],
    );
    let out = s.commit(tx.clone()).unwrap();
    assert_eq!(
        out.receipt.inputs[0],
        InputResult::Normalized(Disposition::Duplicate)
    );
    assert!(matches!(
        out.receipt.inputs[1],
        InputResult::Normalized(Disposition::Rejected(LedgerError::Conflict))
    ));
    assert!(matches!(
        out.receipt.inputs[2],
        InputResult::Normalized(Disposition::Rejected(_))
    ));
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(user(1)))
            .unwrap()
            .cash(),
        cash(97)
    );
    let expected = s.state().unwrap().clone();
    let head = s.head();
    drop(s);
    let mut s = temp.open();
    assert_eq!(s.state().unwrap(), &expected);
    assert_eq!(s.head(), head);
    assert_eq!(s.transaction(tx.id).unwrap(), &tx);
    assert!(s.commit(tx.clone()).unwrap().duplicate);
    let mut changed = tx;
    changed.at += 1;
    assert_eq!(s.commit(changed).unwrap_err(), Error::Conflict);
}

#[test]
fn joined_receipt_hold_and_consumption_are_one_commit() {
    let temp = Temp::new();
    let mut s = temp.create();
    let tx = transaction(
        s.head(),
        1,
        vec![receipt(1, Owner::Customer(user(1)), 100)],
        vec![reserve(1, 80), prepare(1)],
    );
    assert_eq!(s.commit(tx.clone()).unwrap().receipt.controls, None);
    assert!(s.commit(tx).unwrap().duplicate);
    let reopened = temp.open();
    assert_eq!(s.state().unwrap(), reopened.state().unwrap());
    assert_eq!(reopened.state().unwrap().ledger().events().len(), 1);
    assert_eq!(
        reopened
            .state()
            .unwrap()
            .reserved(Resource::Customer(user(1)))
            .unwrap(),
        cash(80)
    );
}

#[test]
fn rejected_controls_do_not_erase_actual_adverse_facts() {
    let temp = Temp::new();
    let mut s = temp.create();
    seed(&mut s);
    hold(&mut s);
    let fee = Event {
        key: RecordKey::Request(request(8)),
        policy: config().policy,
        change: Change::Economics(EconomicChange::BrokerFee {
            customer: user(1),
            amount: cash(50),
        }),
    };
    let out = s
        .commit(transaction(
            s.head(),
            8,
            vec![fee],
            vec![Control::Expose(attempt(2))],
        ))
        .unwrap();
    assert_eq!(out.receipt.controls, Some(ControlError::Capacity));
    assert!(out.exposures.is_empty());
    assert_eq!(
        s.state()
            .unwrap()
            .ledger()
            .book(Owner::Customer(user(1)))
            .unwrap()
            .cash(),
        cash(50)
    );
    assert!(!s.state().unwrap().attempts()[0].possibly_exposed);
    assert_eq!(s.state().unwrap(), temp.open().state().unwrap());
}

#[test]
fn concurrent_writers_cannot_spend_same_customer_house_or_liquidity_capacity() {
    for resource in [
        Resource::Customer(user(1)),
        Resource::House,
        Resource::Location(Location::Venue),
    ] {
        let temp = Temp::new();
        let mut s = temp.create();
        seed(&mut s);
        drop(s);
        let barrier = Arc::new(Barrier::new(2));
        let handles: Vec<_> = (3..5)
            .map(|n| {
                let path = temp.db.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let mut s = open(&path);
                    let amount = if resource == Resource::Location(Location::Venue) {
                        150
                    } else {
                        80
                    };
                    let tx = transaction(
                        s.head(),
                        n,
                        vec![],
                        vec![Control::Reserve {
                            request: request(n),
                            reservations: vec![Reservation {
                                resource,
                                amount: cash(amount),
                            }],
                        }],
                    );
                    barrier.wait();
                    s.commit(tx).map(|r| r.receipt.controls)
                })
            })
            .collect();
        let outcomes: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert_eq!(outcomes.iter().filter(|r| **r == Ok(None)).count(), 1);
        assert_eq!(
            outcomes
                .iter()
                .filter(|r| matches!(r, Err(Error::Stale | Error::Busy)))
                .count(),
            1
        );
        let mut s = temp.open();
        let out = s
            .commit(transaction(
                s.head(),
                9,
                vec![],
                vec![Control::Reserve {
                    request: request(9),
                    reservations: vec![Reservation {
                        resource,
                        amount: cash(80),
                    }],
                }],
            ))
            .unwrap();
        assert_eq!(out.receipt.controls, Some(ControlError::Capacity));
        assert_eq!(s.state().unwrap().holds().len(), 1);
    }
}

#[test]
fn exposure_is_one_shot_and_unknown_action_stays_reserved_after_restart() {
    let temp = Temp::new();
    let mut s = temp.create();
    seed(&mut s);
    hold(&mut s);
    let tx = transaction(s.head(), 3, vec![], vec![Control::Expose(attempt(2))]);
    let mut out = s.commit(tx.clone()).unwrap();
    assert_eq!(out.exposures.len(), 1);
    let (key, bytes) = out.exposures.pop().unwrap().into_message();
    assert_eq!(key, attempt(2));
    assert_eq!(bytes, raw(b"exact-unsigned-private-preimage"));
    drop(s);
    let mut s = temp.open();
    assert!(s.commit(tx).unwrap().exposures.is_empty());
    for (n, c) in [
        (4, Control::Expose(attempt(2))),
        (5, Control::Release(request(2))),
    ] {
        assert_eq!(
            s.commit(transaction(s.head(), n, vec![], vec![c]))
                .unwrap()
                .receipt
                .controls,
            Some(ControlError::Exposed)
        );
    }
    assert!(s.state().unwrap().holds()[0].active);
    assert!(s.state().unwrap().attempts()[0].possibly_exposed);
}

#[test]
fn prepared_bytes_are_immutable_and_expiry_is_not_release() {
    let temp = Temp::new();
    let mut s = temp.create();
    seed(&mut s);
    hold(&mut s);
    let mut other = attempt(2);
    other.attempt = AttemptId::new([99; 32]).unwrap();
    let out = s
        .commit(transaction(
            s.head(),
            3,
            vec![],
            vec![Control::Prepare {
                key: other,
                message: raw(b"other"),
                authority_epoch: 1,
                expires_at: 100,
            }],
        ))
        .unwrap();
    assert_eq!(out.receipt.controls, Some(ControlError::Invalid));
    let mut tx = transaction(s.head(), 4, vec![], vec![Control::Expose(attempt(2))]);
    tx.at = 100;
    assert_eq!(
        s.commit(tx).unwrap().receipt.controls,
        Some(ControlError::Expired)
    );
    assert!(s.state().unwrap().holds()[0].active);
    assert!(!s.state().unwrap().attempts()[0].possibly_exposed);
}

#[test]
fn unnormalized_and_bad_envelopes_are_retained_and_block_exposure() {
    let temp = Temp::new();
    let mut s = temp.create();
    seed(&mut s);
    hold(&mut s);
    let mut tx = transaction(s.head(), 3, vec![receipt(3, Owner::House, 1)], vec![]);
    tx.inputs[0].authority_epoch = 0;
    let mut undecoded = tx.inputs[0].clone();
    undecoded.event = None;
    undecoded.authority_epoch = 1;
    tx.inputs.push(undecoded);
    let r = s.commit(tx.clone()).unwrap();
    assert_eq!(
        r.receipt.inputs,
        vec![InputResult::EnvelopeRejected, InputResult::Unnormalized]
    );
    assert_eq!(s.state().unwrap().unresolved_raw(), 2);
    let r = s
        .commit(transaction(
            s.head(),
            4,
            vec![],
            vec![Control::Expose(attempt(2))],
        ))
        .unwrap();
    assert_eq!(r.receipt.controls, Some(ControlError::Unqualified));
    assert_eq!(temp.open().transaction(tx.id).unwrap(), &tx);
}

#[test]
fn duplicate_evidence_cannot_release_a_new_hold_and_controls_rollback_together() {
    let temp = Temp::new();
    let mut s = temp.create();
    seed(&mut s);
    hold(&mut s);
    let out = s
        .commit(transaction(
            s.head(),
            3,
            vec![receipt(1, Owner::Customer(user(1)), 100)],
            vec![Control::Release(request(2))],
        ))
        .unwrap();
    assert_eq!(out.receipt.controls, Some(ControlError::Unqualified));
    let out = s
        .commit(transaction(
            s.head(),
            4,
            vec![],
            vec![Control::Release(request(2)), reserve(4, 101)],
        ))
        .unwrap();
    assert_eq!(out.receipt.controls, Some(ControlError::Capacity));
    assert!(s.state().unwrap().holds()[0].active);
    let out = s
        .commit(transaction(
            s.head(),
            5,
            vec![],
            vec![Control::Release(request(2))],
        ))
        .unwrap();
    assert_eq!(out.receipt.controls, None);
    assert_eq!(
        s.commit(transaction(s.head(), 6, vec![], vec![reserve(2, 1)]))
            .unwrap()
            .receipt
            .controls,
        Some(ControlError::Invalid)
    );
}

#[test]
fn private_debug_and_errors_never_echo_payloads() {
    let temp = Temp::new();
    let mut s = temp.create();
    seed(&mut s);
    hold(&mut s);
    let tx = transaction(s.head(), 3, vec![receipt(3, Owner::House, 1)], vec![]);
    let text = format!(
        "{s:?} {:?} {tx:?} {:?} {:?}",
        s.state().unwrap(),
        tx.inputs[0],
        s.state().unwrap().attempts()[0]
    );
    assert!(!text.contains("synthetic-native"));
    assert!(!text.contains("preimage"));
    let conn = rusqlite::Connection::open(temp.db.join("journal.sqlite3")).unwrap();
    let schema: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name='records'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!schema.contains("customer"));
    assert!(!schema.contains("balance"));
    // Deliberately tests the seam, NOT that the toy transform is cryptography.
    let bytes = std::fs::read(temp.db.join("journal.sqlite3")).unwrap();
    assert!(!bytes.windows(8).any(|b| b == b"preimage"));
}

#[test]
fn migration_preserves_all_frames_and_requires_explicit_authorization() {
    let temp = Temp::new();
    let mut s = temp.create();
    seed(&mut s);
    hold(&mut s);
    let expected = s.state().unwrap().clone();
    drop(s);
    let before = SqliteBackend::open(&temp.db, Migration::None)
        .unwrap()
        .load()
        .unwrap();
    let conn = rusqlite::Connection::open(temp.db.join("journal.sqlite3")).unwrap();
    conn.execute_batch("BEGIN IMMEDIATE; DROP TABLE head; DROP TRIGGER records_no_update; DROP TRIGGER records_no_delete; PRAGMA user_version=1; COMMIT;").unwrap();
    drop(conn);
    assert_eq!(
        SqliteBackend::open(&temp.db, Migration::None).unwrap_err(),
        Error::Version
    );
    let mut b = SqliteBackend::open(&temp.db, Migration::V1ToV2).unwrap();
    assert_eq!(before, b.load().unwrap());
    let s = Journal::open(b, FixtureProtection, config()).unwrap();
    assert_eq!(s.state().unwrap(), &expected);
}

#[test]
fn unsupported_missing_corrupt_and_wrong_domain_stores_do_not_reset() {
    let temp = Temp::new();
    assert!(SqliteBackend::open(&temp.db, Migration::None).is_err());
    let mut s = temp.create();
    seed(&mut s);
    drop(s);
    assert!(SqliteBackend::create(&temp.db).is_err());
    let mut wrong = config();
    wrong.domain.deployment = DeploymentId::new([99; 32]).unwrap();
    assert!(
        Journal::open(
            SqliteBackend::open(&temp.db, Migration::None).unwrap(),
            FixtureProtection,
            wrong
        )
        .is_err()
    );
    let conn = rusqlite::Connection::open(temp.db.join("journal.sqlite3")).unwrap();
    assert!(conn.execute("DELETE FROM records WHERE seq=1", []).is_err());
    conn.execute_batch("PRAGMA user_version=99;").unwrap();
    assert_eq!(
        SqliteBackend::open(&temp.db, Migration::V1ToV2).unwrap_err(),
        Error::Version
    );
    conn.execute_batch("PRAGMA user_version=2; UPDATE head SET digest=zeroblob(32);")
        .unwrap();
    drop(conn);
    assert!(
        Journal::open(
            SqliteBackend::open(&temp.db, Migration::None).unwrap(),
            FixtureProtection,
            config()
        )
        .is_err()
    );
    assert_eq!(
        rusqlite::Connection::open(temp.db.join("journal.sqlite3"))
            .unwrap()
            .query_row("SELECT count(*) FROM records", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
}

#[cfg(feature = "test-hooks")]
#[test]
fn commit_errors_stop_delivery_and_reopen_resolves_lost_reply() {
    for point in [
        CommitPoint::BeforeBegin,
        CommitPoint::AfterInsert,
        CommitPoint::AfterHead,
        CommitPoint::BeforeCommit,
        CommitPoint::AfterCommit,
    ] {
        let temp = Temp::new();
        let mut s = temp.create();
        seed(&mut s);
        hold(&mut s);
        let head = s.head();
        drop(s);
        let mut b = SqliteBackend::open(&temp.db, Migration::None).unwrap();
        b.set_test_hook(move |p| {
            if p == point {
                Err(Error::Storage)
            } else {
                Ok(())
            }
        });
        let mut s = Journal::open(b, FixtureProtection, config()).unwrap();
        let tx = transaction(head, 3, vec![], vec![Control::Expose(attempt(2))]);
        assert_eq!(s.commit(tx.clone()).unwrap_err(), Error::Storage);
        assert_eq!(s.state().unwrap_err(), Error::Poisoned);
        assert_eq!(s.commit(tx.clone()).unwrap_err(), Error::Poisoned);
        drop(s);
        let mut s = temp.open();
        let persisted = point == CommitPoint::AfterCommit;
        assert_eq!(s.state().unwrap().attempts()[0].possibly_exposed, persisted);
        if persisted {
            assert!(s.commit(tx).unwrap().exposures.is_empty());
        }
    }
}

#[cfg(feature = "test-hooks")]
#[test]
fn sqlite_full_rolls_back_without_credit_consumption_or_dispatch() {
    let temp = Temp::new();
    let s = temp.create();
    let head = s.head();
    drop(s);
    let mut b = SqliteBackend::open(&temp.db, Migration::None).unwrap();
    b.cap_test_pages().unwrap();
    let mut s = Journal::open(b, FixtureProtection, config()).unwrap();
    let mut tx = transaction(
        head,
        1,
        vec![receipt(1, Owner::Customer(user(1)), 100)],
        vec![reserve(1, 80), prepare(1), Control::Expose(attempt(1))],
    );
    tx.inputs[0].raw = PrivateBytes::new(vec![b'x'; 100_000]).unwrap();
    assert_eq!(s.commit(tx).unwrap_err(), Error::Storage);
    assert_eq!(s.state().unwrap_err(), Error::Poisoned);
    drop(s);
    let s = temp.open();
    assert_eq!(s.head(), head);
    assert!(s.state().unwrap().holds().is_empty());
    assert!(s.state().unwrap().ledger().events().is_empty());
}
