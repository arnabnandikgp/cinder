#![no_std]
//! Exact financial primitives and deterministic transition boundary.
//!
//! Inputs must contain all required observations. Kernel code must not access
//! clocks, stores, signers or networks. Canonical codecs use bounded heap buffers
//! through `alloc`, without third-party dependencies. No durable store or dispatch exists.

extern crate alloc;

pub mod amounts;
pub mod codec;
pub mod identity;
pub mod ledger;
pub mod math;
pub mod position;
pub mod quota;

/// Explicit primitive rejection; none of these errors mutates a ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The exact result is outside the destination integer's range.
    Overflow,
    /// Division by zero is not a rounding choice.
    DivisionByZero,
    /// A signed/zero value was used where a positive value was required.
    InvalidSign,
    /// The decimal text is not in the accepted canonical grammar.
    InvalidDecimal,
    /// More fractional digits than the explicitly configured scale.
    Overprecision,
    /// An exact conversion would discard a nonzero remainder.
    Inexact,
    /// Asset, market or precision revision does not match.
    UnitMismatch,
    /// A zero/oversized/empty identifier is not valid.
    InvalidId,
    /// Schema is unsupported or configured policy/precision revision differs.
    UnknownVersion,
    /// Framing, length, kind or trailing bytes are invalid.
    InvalidEncoding,
    /// Deployment, account or semantic source scope does not match.
    ScopeMismatch,
    /// An allocation fraction is outside zero through one.
    InvalidRatio,
}

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
