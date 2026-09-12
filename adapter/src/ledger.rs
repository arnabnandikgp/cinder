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
}

impl MemoryLedger {
    pub fn new() -> Self {
        Self {
            invariant_ok: 1,
            ..Default::default()
        }
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
}
