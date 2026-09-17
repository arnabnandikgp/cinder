//! Durable, single-writer order recovery for Cinder.
//!
//! This crate is deliberately a bounded R4a foundation.  It journals a
//! prepared bounded intent before a venue side effect, persists authoritative
//! venue fill facts before an ER acknowledgement, and refuses to infer a
//! failure from a timeout.  It does **not** contain a Phoenix/Rise client,
//! QFS authentication, key loading, scheduling, cancellation, or a live
//! operator loop; those require the R4b/R5 evidence and integration work.
//!
//! A production [`LedgerRecoveryPort`] must attest the full immutable
//! [`OrderIdentity`] when it reports an acknowledgement.  The current
//! on-chain `OpenOid` alone cannot do that: it has no per-order nonce, so a
//! reused client OID is not sufficient restart evidence.

mod journal;
mod recovery;
mod types;

pub use cinder_adapter::{ClientOid, PubkeyBytes};
pub use journal::{Journal, JournalError, JournalStatus, SCHEMA_VERSION};
pub use recovery::{
    AckFinality, AckObservation, ErAckCommand, ErAckSubmissionPort, ErAckSubmitResult, HaltReason,
    LedgerRecoveryPort, ReconciliationReport, ReconciliationSnapshot, RecoveryCoordinator,
    VenueRecoveryPort, VenueSubmissionPort, VenueSubmitResult,
};
pub use types::{
    derive_venue_identity, BoundedIntent, ErrorCode, FillFact, Operation, OperationId,
    OrderIdentity, OrderKind, OrderState, VenueIdentity, VenueObservation, VenueOid,
};
