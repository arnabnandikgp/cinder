//! Durable, single-writer order recovery for Cinder.
//!
//! Journals bounded intent before side effects and authoritative venue facts
//! before private acknowledgement; timeouts never prove rejection. Includes
//! restricted key loading, operator QFS authentication, placement/ack receipt
//! joins, official Rise recovery views, and fresh startup reconciliation.
//! New venue dispatch, cancellation, and autonomous scheduling are disabled.
//! The recovery-only command leaves both operator-down gates closed on exit.
//!
//! A production [`LedgerRecoveryPort`] must attest the full immutable
//! [`OrderIdentity`] when it reports an acknowledgement.  The current
//! on-chain `OpenOid` alone cannot do that: it has no per-order nonce, so a
//! reused client OID is not sufficient restart evidence.

mod journal;
mod ledger;
mod recovery;
mod rise;
mod rpc;
mod runtime;
mod transaction;
mod types;
mod venue;
#[cfg(test)]
#[path = "../tests/support/venue_fixture.rs"]
mod venue_fixture;
pub use rpc::{load_signer, unix_ms, RuntimeError};
pub use runtime::{MarketMapping, OperatorRuntime, RuntimeConfig};

pub use cinder_adapter::{ClientOid, PubkeyBytes};
pub use journal::{Journal, JournalError, JournalStatus, SCHEMA_VERSION};
pub use recovery::{
    AckFinality, AckObservation, ErAckCommand, ErAckSubmissionPort, ErAckSubmitResult, HaltReason,
    LedgerRecoveryPort, ReconciliationReport, ReconciliationSnapshot, RecoveryCoordinator,
    VenueRecoveryPort, VenueSubmissionPort, VenueSubmitResult,
};
pub use types::{
    derive_venue_identity, BoundedIntent, ErrorCode, FillFact, Operation, OperationId,
    OrderIdentity, OrderKind, OrderState, PreparedAck, VenueIdentity, VenueObservation, VenueOid,
};
