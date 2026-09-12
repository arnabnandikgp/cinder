//! Adapter skeleton (S4). Phoenix is a trait; tests use a mock.
//! Holds **one** operator QFS token. Never a per-user token table.

mod engine;
mod funding;
mod inflight;
mod ledger;
mod liquidation;
mod operator;
mod phoenix;
mod residual;

pub use engine::{Adapter, HedgeOutcome, LiqQueueItem, PendingOid};
pub use funding::{quote_lots_to_usdc, FundingCrankReport, FundingInterval};
pub use liquidation::ScanReport;
pub use inflight::{InFlight, InFlightTable};
pub use ledger::{FundingPort, LedgerPort, MemoryLedger};
pub use operator::{MockTeeAuth, OperatorAuth, TeeAuth};
pub use phoenix::{
    Fill, MarketOrder, MockPhoenix, PhoenixVenue, PlaceResult, PoolHealth, ASSET_SOL,
};
pub use residual::{i1_holds, i2_holds, i2_holds_unsettled, intended_residual};

pub type PubkeyBytes = [u8; 32];
pub type ClientOid = [u8; 16];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    Unauthorized,
    Phoenix(String),
    Ledger(String),
    Auth(String),
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthorized => write!(f, "unauthorized"),
            Self::Phoenix(s) | Self::Ledger(s) | Self::Auth(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for AdapterError {}
