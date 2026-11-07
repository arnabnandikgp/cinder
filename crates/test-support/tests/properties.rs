//! Harness properties over a toy counter, NOT tests of financial correctness.
use cinder_kernel::Transition;
use cinder_ports::Journal;
use cinder_test_support::{CommitFault, MemoryJournal};

struct Counter;

impl Transition for Counter {
    type State = u8;
    type Event = u8;
    type Error = ();

    fn apply(state: &u8, event: &u8) -> Result<u8, ()> {
        state.checked_add(*event).ok_or(())
    }
}

#[test]
fn property_transition_is_repeatable_and_rejection_preserves_input() {
    // Every possible pair: 65,536 states/events, including every overflow pair.
    for state in u8::MIN..=u8::MAX {
        for event in u8::MIN..=u8::MAX {
            let before = state;
            let first = Counter::apply(&state, &event);
            assert_eq!(first, Counter::apply(&state, &event));
            assert_eq!(state, before);
            let widened = u16::from(state) + u16::from(event);
            assert_eq!(first.ok(), u8::try_from(widened).ok());
        }
    }
}

#[test]
fn property_replay_matches_simulated_commits_at_every_fault_cut() {
    // All 3^5 success/precommit/postcommit fault schedules, no randomness/network.
    for schedule in 0_u32..243 {
        let mut remaining = schedule;
        let mut journal = MemoryJournal::default();
        let mut expected = 0_u8;
        for event in 1_u8..=5 {
            let cut = remaining % 3;
            remaining /= 3;
            match cut {
                1 => journal.fail_next(CommitFault::BeforeCommit),
                2 => journal.fail_next(CommitFault::AfterCommit),
                _ => {}
            }
            let result = journal.append(event);
            if cut != 1 {
                expected = expected.checked_add(event).expect("bounded fixture");
            }
            assert_eq!(result.is_ok(), cut == 0);
            let replayed = journal
                .records()
                .iter()
                .try_fold(0, |state, event| Counter::apply(&state, event));
            assert_eq!(replayed, Ok(expected), "schedule {schedule}, event {event}");
        }
    }
}
