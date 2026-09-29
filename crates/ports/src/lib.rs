#![no_std]
//! Minimal synchronous test seams, not frozen runtime APIs.
//!
//! These ports deliberately select no async runtime, wire format, database,
//! signing library or venue. Owning phases refine them with actual admission,
//! receipt identity, atomic commits and authority epochs before dispatch exists.

/// An injected local scheduling clock, never native execution/finality evidence.
pub trait Clock {
    /// Caller-defined origin and units; not interchangeable with venue time.
    type Instant: Copy;
    /// Observe the injected time. Trusted freshness requires a separate policy.
    fn now(&self) -> Self::Instant;
}

/// Append seam for persistence failure tests, not the P05 production transaction API.
///
/// An error may mean an acknowledgement was lost after commit. Callers must not
/// assume every error means no state change. P05 adds typed commit outcomes,
/// expected versions, atomic postings/holds/consumption and reconciliation.
pub trait Journal {
    /// Opaque record; canonical, domain-bound schema belongs to P02/P05.
    type Record;
    /// Adapter-specific error, including ambiguous outcomes where applicable.
    type Error;
    /// Attempt to append one record.
    fn append(&mut self, record: Self::Record) -> Result<(), Self::Error>;
}

/// Explicit venue-effect seam; no implementation here can access a real venue.
///
/// A reply is not necessarily a fill. An error is not necessarily rejection.
/// Production signing/admission/reconciliation is intentionally absent.
pub trait Venue {
    /// Admitted request format, refined in the owning execution phase.
    type Request;
    /// Transport response, not an accounting event by default.
    type Response;
    /// Native/transport error with its qualified outcome semantics.
    type Error;
    /// Attempt a request; never call from the pure accounting kernel.
    fn submit(&mut self, request: Self::Request) -> Result<Self::Response, Self::Error>;
}
