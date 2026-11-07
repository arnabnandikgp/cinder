//! Deterministic fault-double behavior, not production persistence qualification.
use cinder_ports::{Clock, Journal, Venue};
use cinder_test_support::{
    CommitFault, JournalError, ManualClock, MemoryJournal, ScriptedVenue, VenueError, VenueStep,
};

#[test]
fn time_is_injected_including_backwards_observations() {
    let mut clock = ManualClock::default();
    assert_eq!(clock.now(), 0);
    clock.set(100);
    assert_eq!(clock.now(), 100);
    clock.set(3);
    assert_eq!(clock.now(), 3);
}

#[test]
fn rejected_append_has_no_record_and_fault_is_one_shot() {
    let mut journal = MemoryJournal::default();
    journal.fail_next(CommitFault::BeforeCommit);
    assert_eq!(journal.append(7), Err(JournalError::Rejected));
    assert!(journal.records().is_empty());
    assert_eq!(journal.append(8), Ok(()));
    assert_eq!(journal.records(), &[8]);
}

#[test]
fn lost_commit_reply_keeps_record_and_must_not_imply_safe_retry() {
    let mut journal = MemoryJournal::default();
    journal.fail_next(CommitFault::AfterCommit);
    assert_eq!(journal.append(7), Err(JournalError::Unknown));
    assert_eq!(journal.records(), &[7]);
    assert_eq!(journal.append(8), Ok(()));
    assert_eq!(journal.records(), &[7, 8]);
}

#[test]
fn venue_rejection_and_lost_reply_have_different_effects() {
    let mut venue = ScriptedVenue::new([
        VenueStep::Reject,
        VenueStep::LoseReply,
        VenueStep::Reply("ack, not fill"),
    ]);
    assert_eq!(venue.submit(10), Err(VenueError::Rejected));
    assert!(venue.accepted().is_empty());
    assert_eq!(venue.submit(20), Err(VenueError::Unknown));
    assert_eq!(venue.accepted(), &[20]);
    assert_eq!(venue.submit(30), Ok("ack, not fill"));
    assert_eq!(venue.attempts(), &[10, 20, 30]);
    assert_eq!(venue.accepted(), &[20, 30]);
}

#[test]
fn exhausted_script_does_not_fabricate_an_attempt_or_success() {
    let mut venue = ScriptedVenue::<u64, ()>::new([]);
    assert_eq!(venue.submit(1), Err(VenueError::ScriptExhausted));
    assert!(venue.attempts().is_empty());
    assert!(venue.accepted().is_empty());
}
