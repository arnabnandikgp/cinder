use cinder_common as cc;

use crate::{ClientOid, PubkeyBytes};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InFlight {
    pub user: PubkeyBytes,
    pub client_oid: ClientOid,
    pub asset_id: u16,
    pub lots_delta: i64,
    pub inserted_at_ms: u64,
}

#[derive(Clone, Debug, Default)]
pub struct InFlightTable {
    entries: std::collections::BTreeMap<ClientOid, InFlight>,
}

impl InFlightTable {
    pub fn insert(&mut self, row: InFlight) {
        self.entries.insert(row.client_oid, row);
    }

    pub fn remove(&mut self, oid: &ClientOid) -> Option<InFlight> {
        self.entries.remove(oid)
    }

    pub fn get(&self, oid: &ClientOid) -> Option<&InFlight> {
        self.entries.get(oid)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Hedge rows older than [`cc::IN_FLIGHT_TTL_MS`].
    pub fn expired(&self, now_ms: u64) -> Vec<InFlight> {
        self.entries
            .values()
            .filter(|r| now_ms.saturating_sub(r.inserted_at_ms) > cc::IN_FLIGHT_TTL_MS)
            .cloned()
            .collect()
    }
}
