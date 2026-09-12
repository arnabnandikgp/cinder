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
    fn pool_health(&self) -> PoolHealth {
        PoolHealth::Safe
    }
}

/// Pre-trade pool health. New hedges only when Safe.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PoolHealth {
    #[default]
    Safe,
    Cancellable,
    Other,
}

impl PoolHealth {
    pub fn allows_new_hedge(self) -> bool {
        matches!(self, Self::Safe)
    }

    pub fn is_unsafe(self) -> bool {
        !matches!(self, Self::Safe)
    }
}

#[derive(Clone, Debug, Default)]
pub struct MockPhoenix {
    pub next: Option<PlaceResult>,
    /// When true (default), a fill is applied to `lots`. Set false to force an I1 break.
    pub apply_fill_to_position: bool,
    pub health: PoolHealth,
    /// After this many `place_market` calls, `health` becomes Safe.
    pub flips_safe_after_places: Option<u32>,
    /// When true, an unscripted `place_market` fully fills the order.
    pub auto_fill: bool,
    place_count: u32,
    lots: std::collections::BTreeMap<u16, i64>,
}

impl MockPhoenix {
    pub fn new() -> Self {
        Self {
            next: None,
            apply_fill_to_position: true,
            health: PoolHealth::Safe,
            flips_safe_after_places: None,
            auto_fill: false,
            place_count: 0,
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
        self.place_count = self.place_count.saturating_add(1);
        if let Some(n) = self.flips_safe_after_places {
            if self.place_count >= n {
                self.health = PoolHealth::Safe;
            }
        }
        let result = match self.next.take() {
            Some(r) => r,
            None if self.auto_fill => PlaceResult::Fill(Fill {
                client_oid: order.client_oid,
                asset_id: order.asset_id,
                filled_lots: order.lots,
                fee_usdc: 0,
                vwap_quote_lots: 0,
            }),
            None => {
                return Err(AdapterError::Phoenix(
                    "mock has no scripted PlaceResult".into(),
                ))
            }
        };
        if let PlaceResult::Fill(ref fill) = result {
            if self.apply_fill_to_position {
                let e = self.lots.entry(fill.asset_id).or_insert(0);
                *e += fill.filled_lots;
                if *e == 0 {
                    self.lots.remove(&fill.asset_id);
                }
            }
        }
        Ok(result)
    }

    fn base_lots(&self, asset_id: u16) -> i64 {
        self.lots.get(&asset_id).copied().unwrap_or(0)
    }

    fn pool_health(&self) -> PoolHealth {
        self.health
    }
}
