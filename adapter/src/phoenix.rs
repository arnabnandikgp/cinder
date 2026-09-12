use crate::{AdapterError, ClientOid};

/// Phoenix SOL on the Cinder allowlist (S1 tests used 1). Skip isolatedOnly markets.
pub const ASSET_SOL: u16 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarketOrder {
    pub asset_id: u16,
    pub lots: i64,
    pub client_oid: ClientOid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fill {
    pub client_oid: ClientOid,
    pub asset_id: u16,
    pub filled_lots: i64,
    pub fee_usdc: u64,
    pub vwap_quote_lots: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaceResult {
    Fill(Fill),
    Reject { reason: String },
}

/// Venue client. S5+ uses Rise; S4 ships a mock.
pub trait PhoenixVenue {
    fn place_market(&mut self, order: &MarketOrder) -> Result<PlaceResult, AdapterError>;
    fn base_lots(&self, asset_id: u16) -> i64;
}

/// Pre-trade pool health. New hedges only when Safe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoolHealth {
    Safe,
    Cancellable,
    Other,
}

impl PoolHealth {
    pub fn allows_new_hedge(self) -> bool {
        matches!(self, Self::Safe)
    }
}

#[derive(Clone, Debug, Default)]
pub struct MockPhoenix {
    pub next: Option<PlaceResult>,
    /// When true (default), a fill is applied to `lots`. Set false to force an I1 break.
    pub apply_fill_to_position: bool,
    lots: std::collections::BTreeMap<u16, i64>,
}

impl MockPhoenix {
    pub fn new() -> Self {
        Self {
            next: None,
            apply_fill_to_position: true,
            lots: std::collections::BTreeMap::new(),
        }
    }

    pub fn reject_next(&mut self, reason: impl Into<String>) {
        self.next = Some(PlaceResult::Reject {
            reason: reason.into(),
        });
    }

    pub fn fill_next(&mut self, fill: Fill) {
        self.next = Some(PlaceResult::Fill(fill));
    }

    pub fn set_lots(&mut self, asset_id: u16, lots: i64) {
        if lots == 0 {
            self.lots.remove(&asset_id);
        } else {
            self.lots.insert(asset_id, lots);
        }
    }
}

impl PhoenixVenue for MockPhoenix {
    fn place_market(&mut self, order: &MarketOrder) -> Result<PlaceResult, AdapterError> {
        let result = self.next.take().ok_or_else(|| {
            AdapterError::Phoenix("mock has no scripted PlaceResult".into())
        })?;
        if let PlaceResult::Fill(ref fill) = result {
            if self.apply_fill_to_position {
                let e = self.lots.entry(fill.asset_id).or_insert(0);
                *e += fill.filled_lots;
                if *e == 0 {
                    self.lots.remove(&fill.asset_id);
                }
            }
            let _ = order;
        }
        Ok(result)
    }

    fn base_lots(&self, asset_id: u16) -> i64 {
        self.lots.get(&asset_id).copied().unwrap_or(0)
    }
}
