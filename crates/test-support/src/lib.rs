//! Offline deterministic doubles. No disk, network, system clock or credentials.
//!
//! A simulated lost reply does not establish process-crash durability. These
//! doubles expose ambiguity to tests; they do not resolve it for a controller.

use std::collections::VecDeque;

use cinder_ports::{Clock, Journal, Venue};

/// Explicitly controlled logical test time; no implicit wall-clock reads.
#[derive(Debug, Default)]
pub struct ManualClock {
    tick: u64,
}

impl ManualClock {
    /// Set an arbitrary tick, including backwards movement for adversarial tests.
    pub fn set(&mut self, tick: u64) {
        self.tick = tick;
    }
}

impl Clock for ManualClock {
    type Instant = u64;

    fn now(&self) -> u64 {
        self.tick
    }
}

/// One-shot append fault, injected at a precisely named boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitFault {
    /// No record was appended.
    BeforeCommit,
    /// Record was appended, but its acknowledgement was lost.
    AfterCommit,
}

/// Fake journal outcomes; unknown is deliberately not a safe retry signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalError {
    /// The fake rejected before appending.
    Rejected,
    /// Caller cannot know the outcome from the response alone.
    Unknown,
}

/// In-memory record list for replay tests, explicitly not durable storage.
#[derive(Debug)]
pub struct MemoryJournal<R> {
    records: Vec<R>,
    next_fault: Option<CommitFault>,
}

impl<R> Default for MemoryJournal<R> {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            next_fault: None,
        }
    }
}

impl<R> MemoryJournal<R> {
    /// Fail the next append at the selected boundary; consumed by that attempt.
    pub fn fail_next(&mut self, fault: CommitFault) {
        self.next_fault = Some(fault);
    }

    /// Inspect simulated storage from a test, not an API exposed to a controller.
    pub fn records(&self) -> &[R] {
        &self.records
    }
}

impl<R> Journal for MemoryJournal<R> {
    type Record = R;
    type Error = JournalError;

    fn append(&mut self, record: R) -> Result<(), JournalError> {
        let fault = self.next_fault.take();
        if fault == Some(CommitFault::BeforeCommit) {
            return Err(JournalError::Rejected);
        }
        self.records.push(record);
        if fault == Some(CommitFault::AfterCommit) {
            return Err(JournalError::Unknown);
        }
        Ok(())
    }
}

/// Scripted transport outcome. Acceptance here is not financial execution.
#[derive(Debug)]
pub enum VenueStep<R> {
    /// Record acceptance and return this response.
    Reply(R),
    /// Record an attempt but no acceptance.
    Reject,
    /// Record acceptance but return an ambiguous result.
    LoseReply,
}

/// Outcomes distinguish a known rejection, an unknown action and a harness bug.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VenueError {
    /// Scripted rejection before acceptance.
    Rejected,
    /// Accepted by the fake, but not observable from the response.
    Unknown,
    /// No scripted step exists; no attempt or acceptance is fabricated.
    ScriptExhausted,
}

/// Bounded, offline script. There is no live-transport fallback on exhaustion.
#[derive(Debug)]
pub struct ScriptedVenue<Q, R> {
    steps: VecDeque<VenueStep<R>>,
    attempts: Vec<Q>,
    accepted: Vec<Q>,
}

impl<Q, R> ScriptedVenue<Q, R> {
    /// Install explicit outcomes in transport-attempt order.
    pub fn new(steps: impl IntoIterator<Item = VenueStep<R>>) -> Self {
        Self {
            steps: steps.into_iter().collect(),
            attempts: Vec::new(),
            accepted: Vec::new(),
        }
    }

    /// Inspect attempted requests, including explicitly rejected ones.
    pub fn attempts(&self) -> &[Q] {
        &self.attempts
    }

    /// Inspect acceptance from tests; not available to the controller via Venue.
    pub fn accepted(&self) -> &[Q] {
        &self.accepted
    }
}

impl<Q: Clone, R> Venue for ScriptedVenue<Q, R> {
    type Request = Q;
    type Response = R;
    type Error = VenueError;

    fn submit(&mut self, request: Q) -> Result<R, VenueError> {
        let step = self.steps.pop_front().ok_or(VenueError::ScriptExhausted)?;
        self.attempts.push(request.clone());
        match step {
            VenueStep::Reject => Err(VenueError::Rejected),
            VenueStep::Reply(response) => {
                self.accepted.push(request);
                Ok(response)
            }
            VenueStep::LoseReply => {
                self.accepted.push(request);
                Err(VenueError::Unknown)
            }
        }
    }
}
