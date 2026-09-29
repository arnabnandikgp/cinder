#![no_std]
//! Deterministic transition boundary; no financial implementation yet.
//!
//! Inputs must contain all required observations. Implementations cannot reach
//! a clock, store, signer or network through this dependency-free crate.
//! P02/P03 add concrete domain types and accounting transitions. This trait is
//! neither a serialization format nor a persistent transaction protocol.

/// Computes a proposed next state without mutating the supplied state.
///
/// A successful result is not a durable commit or permission to expose a side
/// effect. The journal/controller must establish those guarantees separately.
pub trait Transition {
    /// Complete state required by this transition.
    type State;
    /// Explicit, already-normalized input; not implicitly trusted venue truth.
    type Event;
    /// Typed rejection, without side effects to the supplied state.
    type Error;

    /// Compute the next state or reject the event.
    fn apply(state: &Self::State, event: &Self::Event) -> Result<Self::State, Self::Error>;
}
