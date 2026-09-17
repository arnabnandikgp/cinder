//! Durable, single-writer order recovery for Cinder.
//!
//! Journals bounded intent before side effects and authoritative venue facts
//! before private acknowledgement; timeouts never prove rejection. Includes
//! restricted key loading, operator QFS authentication, placement/ack receipt
//! joins, official Rise recovery views, and fresh startup reconciliation.
//! Bounded native IOC dispatch requires explicit execution policy and admission.
//! Autonomous scheduling and resting-order cancellation are not implemented.
//! The recovery-only command leaves both operator-down gates closed on exit.
//!
//! A production [`LedgerRecoveryPort`] must attest the full immutable
//! [`OrderIdentity`] when it reports an acknowledgement.  The current
//! on-chain `OpenOid` alone cannot do that: it has no per-order nonce, so a
//! reused client OID is not sufficient restart evidence.

mod admission;
mod collateral;
mod execution;
mod funding;
mod journal;
mod ledger;
mod recovery;
mod rise;
mod rpc;
mod runtime;
mod solvency;
mod transaction;
mod types;
mod venue;
#[cfg(test)]
#[path = "../tests/support/venue_fixture.rs"]
mod venue_fixture;
pub use rpc::{load_signer, unix_ms, RuntimeError};
pub use runtime::{MarketMapping, OperatorRuntime, RuntimeConfig};
pub use solvency::{SolvencyPolicy, SolvencyReport, StressScenario};

pub use admission::ExecutionPolicy;
pub use cinder_adapter::{ClientOid, PubkeyBytes};
pub use collateral::CollateralRequirement;
pub use execution::{build_bounded_ioc, IocMarketAccounts};
pub use funding::{build_phoenix_funding, decode_phoenix_funding_receipt, PhoenixFundingAccounts};
pub use journal::{Journal, JournalError, JournalStatus, SCHEMA_VERSION};
pub use recovery::{
    AckFinality, AckObservation, BookSyncObservation, ErAckCommand, ErAckSubmissionPort,
    ErAckSubmitResult, FundingObservation, HaltReason, LedgerRecoveryPort, ReconciliationReport,
    ReconciliationSnapshot, RecoveryCoordinator, VenueRecoveryPort, VenueSubmissionPort,
    VenueSubmitResult,
};
pub use types::{
    derive_venue_identity, BookSyncAttempt, BoundedIntent, ErrorCode, ExecutionBudget, FillFact,
    FundingIntent, FundingRecord, Operation, OperationId, OrderIdentity, OrderKind, OrderState,
    PreparedAck, PreparedBookSync, PreparedVenue, VenueIdentity, VenueObservation, VenueOid,
};
