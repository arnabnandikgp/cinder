use cinder_common as cc;

use crate::phoenix::Fill;
use crate::{AdapterError, ClientOid, PubkeyBytes};

/// On-chain / ER writes the adapter needs. S4 tests use [`MemoryLedger`].
pub trait LedgerPort {
    fn ack_fill(&mut self, user: &PubkeyBytes, fill: &Fill) -> Result<(), AdapterError>;
    fn ack_fail(&mut self, user: &PubkeyBytes, oid: &ClientOid) -> Result<(), AdapterError>;
    fn write_halt(&mut self, flags: u8) -> Result<(), AdapterError>;
    fn book_lots(&self, asset_id: u16) -> i64;
    fn config_halt(&self) -> u8;
    fn book_halt(&self) -> u8;
    fn invariant_ok(&self) -> bool;
    fn write_reserve_root(&mut self, now_ms: u64) -> Result<(), AdapterError>;
}

#[derive(Clone, Debug, Default)]
pub struct MemoryLedger {
    book: std::collections::BTreeMap<u16, i64>,
    config_halt: u8,
    book_halt: u8,
    invariant_ok: u8,
    pub fills: Vec<(PubkeyBytes, Fill)>,
    pub fails: Vec<(PubkeyBytes, ClientOid)>,
    pub reserved: std::collections::BTreeMap<u16, u64>,
    pub user_lots: std::collections::BTreeMap<PubkeyBytes, std::collections::BTreeMap<u16, i64>>,
    pub user_cash: std::collections::BTreeMap<PubkeyBytes, u64>,
    pub vault_ata: u64,
    pub phoenix_collateral: u64,
    pub reserve_roots: Vec<u64>,
    pub fail_next_root: bool,
}

impl MemoryLedger {
    pub fn new() -> Self {
        Self {
            invariant_ok: 1,
            ..Default::default()
        }
    }

    pub fn lots_of(&self, user: &PubkeyBytes, asset_id: u16) -> i64 {
        self.user_lots
            .get(user)
            .and_then(|m| m.get(&asset_id))
            .copied()
            .unwrap_or(0)
    }

    pub fn sum_user_lots(&self, asset_id: u16) -> i64 {
        self.user_lots
            .values()
            .map(|m| m.get(&asset_id).copied().unwrap_or(0))
            .sum()
    }

    pub fn sum_user_cash(&self) -> u64 {
        self.user_cash.values().copied().sum()
    }

    pub fn i2_ok(&self, in_flight_usdc: i64) -> bool {
        crate::residual::i2_holds(
            self.sum_user_cash(),
            self.vault_ata,
            self.phoenix_collateral,
            in_flight_usdc,
        )
    }
}

impl LedgerPort for MemoryLedger {
    fn ack_fill(&mut self, user: &PubkeyBytes, fill: &Fill) -> Result<(), AdapterError> {
        let e = self.book.entry(fill.asset_id).or_insert(0);
        *e += fill.filled_lots;
        if *e == 0 {
            self.book.remove(&fill.asset_id);
        }
        self.fills.push((*user, fill.clone()));
        *self
            .user_lots
            .entry(*user)
            .or_default()
            .entry(fill.asset_id)
            .or_insert(0) += fill.filled_lots;
        let lots = self.book_lots(fill.asset_id);
        let im = cc::stub_cinder_im(lots.unsigned_abs())
            .ok_or_else(|| AdapterError::Ledger("im overflow".into()))?;
        self.reserved.insert(fill.asset_id, im);
        Ok(())
    }

    fn ack_fail(&mut self, user: &PubkeyBytes, oid: &ClientOid) -> Result<(), AdapterError> {
        self.fails.push((*user, *oid));
        Ok(())
    }

    fn write_halt(&mut self, flags: u8) -> Result<(), AdapterError> {
        self.config_halt = flags;
        self.book_halt = flags;
        if flags & cc::INVARIANT_BROKEN != 0 {
            self.invariant_ok = 0;
        }
        Ok(())
    }

    fn book_lots(&self, asset_id: u16) -> i64 {
        self.book.get(&asset_id).copied().unwrap_or(0)
    }

    fn config_halt(&self) -> u8 {
        self.config_halt
    }

    fn book_halt(&self) -> u8 {
        self.book_halt
    }

    fn invariant_ok(&self) -> bool {
        self.invariant_ok == 1
    }

    fn write_reserve_root(&mut self, now_ms: u64) -> Result<(), AdapterError> {
        if self.fail_next_root {
            self.fail_next_root = false;
            return Err(AdapterError::Ledger("reserve root write failed".into()));
        }
        self.reserve_roots.push(now_ms);
        Ok(())
    }
}
