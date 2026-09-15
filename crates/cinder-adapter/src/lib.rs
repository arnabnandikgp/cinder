//! Operator logic. Phoenix is abstracted behind a trait for testability.
//! Holds **one** operator QFS token. Never a per-user token table.

mod engine;
mod funding;
mod inflight;
mod ledger;
mod liquidation;
mod operator;
mod phoenix;
mod residual;
mod risk;

pub use engine::{Adapter, HedgeOutcome, LiqQueueItem, PendingOid, PreflightOrder};
pub use funding::{quote_lots_to_usdc, FundingCrankReport, FundingInterval};
pub use inflight::{InFlight, InFlightTable};
pub use ledger::{FundingPort, LedgerPort, MemoryLedger};
pub use liquidation::ScanReport;
pub use operator::{MockTeeAuth, OperatorAuth, TeeAuth};
pub use phoenix::{
    Fill, MarketOrder, MockPhoenix, PhoenixVenue, PlaceResult, PoolHealth, ASSET_SOL,
};
pub use residual::{i1_holds, i2_holds, i2_holds_unsettled, intended_residual};
#[cfg(any(test, feature = "test-utils"))]
pub use risk::StubRiskEngine;
pub use risk::{
    MarketStatus, PoolRiskQuote, RiseRiskEngine, RiskEngine, RiskSnapshot, UnitStatus,
    UserRiskQuote,
};

pub type PubkeyBytes = [u8; 32];
pub type ClientOid = [u8; 16];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    Unauthorized,
    Phoenix(String),
    Ledger(String),
    Auth(String),
    Risk(String),
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthorized => write!(f, "unauthorized"),
            Self::Phoenix(s) | Self::Ledger(s) | Self::Auth(s) | Self::Risk(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for AdapterError {}
