//! Adapter skeleton (S4). Phoenix is a trait; tests use a mock.
//! Holds **one** operator QFS token. Never a per-user token table.

mod engine;
mod inflight;
mod ledger;
mod operator;
mod phoenix;
mod residual;

pub use engine::{Adapter, HedgeOutcome, PendingOid};
pub use inflight::{InFlight, InFlightTable};
pub use ledger::{LedgerPort, MemoryLedger};
pub use operator::{MockTeeAuth, OperatorAuth, TeeAuth};
pub use phoenix::{Fill, MarketOrder, MockPhoenix, PhoenixVenue, PlaceResult};
pub use residual::{i1_holds, i2_holds, intended_residual};

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
