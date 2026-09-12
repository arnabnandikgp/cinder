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
    pub book: std::collections::BTreeMap<u16, i64>,
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
    pub book_funding_epoch: u64,
    pub user_epoch: std::collections::BTreeMap<PubkeyBytes, u64>,
    pub user_free: std::collections::BTreeMap<PubkeyBytes, u64>,
    pub user_reserved: std::collections::BTreeMap<PubkeyBytes, u64>,
    pub user_unsettled: std::collections::BTreeMap<PubkeyBytes, std::collections::BTreeMap<u16, i64>>,
    pub user_entry: std::collections::BTreeMap<PubkeyBytes, std::collections::BTreeMap<u16, i64>>,
    pub pending_liq: std::collections::BTreeMap<ClientOid, PendingLiq>,
    pub liquidations: Vec<(PubkeyBytes, u16)>,
    pub last_scan_ms: u64,
    pub mark_usdc_per_lot: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingLiq {
    pub user: PubkeyBytes,
    pub client_oid: ClientOid,
    pub asset_id: u16,
    pub lots_delta: i64,
}

impl MemoryLedger {
    pub fn new() -> Self {
        Self {
            invariant_ok: 1,
            mark_usdc_per_lot: cc::STUB_NOTIONAL_PER_LOT as i64,
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

    pub fn sum_unsettled(&self) -> i64 {
        self.user_unsettled
            .values()
            .flat_map(|m| m.values())
            .copied()
            .sum()
    }

    pub fn i2_ok_unsettled(&self, pool_unsettled: i64, in_flight_usdc: i64) -> bool {
        crate::residual::i2_holds_unsettled(
            self.sum_user_cash(),
            self.sum_unsettled(),
            self.vault_ata,
            self.phoenix_collateral,
            pool_unsettled,
            in_flight_usdc,
        )
    }

    pub fn ensure_user(&mut self, user: PubkeyBytes, free: u64) {
        self.user_free.entry(user).or_insert(free);
        self.user_reserved.entry(user).or_insert(0);
        self.user_epoch.entry(user).or_insert(self.book_funding_epoch);
        self.user_unsettled.entry(user).or_default();
        self.user_lots.entry(user).or_default();
        self.user_cash.insert(
            user,
            self.user_free.get(&user).copied().unwrap_or(0)
                + self.user_reserved.get(&user).copied().unwrap_or(0),
        );
    }
}

/// Ledger writes the funding crank needs. [`MemoryLedger`] implements this.
pub trait FundingPort {
    fn book_funding_epoch(&self) -> u64;
    fn bump_funding_epoch(&mut self, epoch: u64) -> Result<(), AdapterError>;
    fn user_ids(&self) -> Vec<PubkeyBytes>;
    fn lots_of(&self, user: &PubkeyBytes, asset_id: u16) -> i64;
    fn last_funding_epoch(&self, user: &PubkeyBytes) -> u64;
    fn allocate_funding(
        &mut self,
        user: &PubkeyBytes,
        epoch: u64,
        fold: bool,
        entries: &[(u16, i64)],
    ) -> Result<(), AdapterError>;
    fn user_equity(&self, user: &PubkeyBytes) -> i128;
    fn liquidate_user(
        &mut self,
        user: &PubkeyBytes,
        asset_id: u16,
        client_oid: ClientOid,
    ) -> Result<(), AdapterError>;
    fn last_scan_ms(&self) -> u64;
    fn write_last_scan_ms(&mut self, ms: u64);
    fn pending_liq_delta(&self, asset_id: u16) -> i64;
    fn pending_liq_lots(&self, oid: &ClientOid) -> Option<i64>;
    fn phoenix_collateral(&self) -> u64;
    fn set_phoenix_collateral(&mut self, v: u64);
    fn i2_ok_unsettled(&self, pool_unsettled: i64, in_flight_usdc: i64) -> bool;
}

impl LedgerPort for MemoryLedger {
    fn ack_fill(&mut self, user: &PubkeyBytes, fill: &Fill) -> Result<(), AdapterError> {
        let e = self.book.entry(fill.asset_id).or_insert(0);
        *e += fill.filled_lots;
        if *e == 0 {
            self.book.remove(&fill.asset_id);
        }
        self.fills.push((*user, fill.clone()));
        if let Some(liq) = self.pending_liq.remove(&fill.client_oid) {
            let cur = self.lots_of(user, fill.asset_id);
            let unfilled = liq
                .lots_delta
                .checked_sub(fill.filled_lots)
                .ok_or_else(|| AdapterError::Ledger("liq unfilled overflow".into()))?;
            let final_lots = cur
                .checked_sub(unfilled)
                .ok_or_else(|| AdapterError::Ledger("liq restore overflow".into()))?;
            if final_lots == 0 {
                if let Some(m) = self.user_lots.get_mut(user) {
                    m.remove(&fill.asset_id);
                }
                if let Some(m) = self.user_entry.get_mut(user) {
                    m.remove(&fill.asset_id);
                }
            } else {
                self.user_lots
                    .entry(*user)
                    .or_default()
                    .insert(fill.asset_id, final_lots);
                if let Some(orig) = liq.lots_delta.checked_neg() {
                    if orig != 0 {
                        if let Some(e) = self
                            .user_entry
                            .get_mut(user)
                            .and_then(|m| m.get_mut(&fill.asset_id))
                        {
                            *e = ((*e as i128) * (final_lots as i128) / (orig as i128)) as i64;
                        }
                    }
                }
            }
        } else {
            *self
                .user_lots
                .entry(*user)
                .or_default()
                .entry(fill.asset_id)
                .or_insert(0) += fill.filled_lots;
        }
        let lots = self.book_lots(fill.asset_id);
        let im = cc::stub_cinder_im(lots.unsigned_abs())
            .ok_or_else(|| AdapterError::Ledger("im overflow".into()))?;
        self.reserved.insert(fill.asset_id, im);
        Ok(())
    }

    fn ack_fail(&mut self, user: &PubkeyBytes, oid: &ClientOid) -> Result<(), AdapterError> {
        if let Some(liq) = self.pending_liq.remove(oid) {
            let restored = liq
                .lots_delta
                .checked_neg()
                .ok_or_else(|| AdapterError::Ledger("liq fail overflow".into()))?;
            if restored == 0 {
                if let Some(m) = self.user_lots.get_mut(user) {
                    m.remove(&liq.asset_id);
                }
            } else {
                self.user_lots
                    .entry(*user)
                    .or_default()
                    .insert(liq.asset_id, restored);
            }
        }
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

impl FundingPort for MemoryLedger {
    fn book_funding_epoch(&self) -> u64 {
        self.book_funding_epoch
    }

    fn bump_funding_epoch(&mut self, epoch: u64) -> Result<(), AdapterError> {
        if self.book_funding_epoch == u64::MAX {
            return Err(AdapterError::Ledger("funding epoch overflow".into()));
        }
        if epoch != self.book_funding_epoch + 1 {
            return Err(AdapterError::Ledger("bad funding epoch".into()));
        }
        self.book_funding_epoch = epoch;
        Ok(())
    }

    fn user_ids(&self) -> Vec<PubkeyBytes> {
        let mut ids: Vec<_> = self.user_lots.keys().copied().collect();
        for k in self.user_free.keys() {
            if !ids.contains(k) {
                ids.push(*k);
            }
        }
        ids.sort();
        ids
    }

    fn lots_of(&self, user: &PubkeyBytes, asset_id: u16) -> i64 {
        MemoryLedger::lots_of(self, user, asset_id)
    }

    fn last_funding_epoch(&self, user: &PubkeyBytes) -> u64 {
        self.user_epoch.get(user).copied().unwrap_or(0)
    }

    fn allocate_funding(
        &mut self,
        user: &PubkeyBytes,
        epoch: u64,
        fold: bool,
        entries: &[(u16, i64)],
    ) -> Result<(), AdapterError> {
        let last = self.last_funding_epoch(user);
        if !fold {
            if epoch != self.book_funding_epoch || epoch != last.saturating_add(1) {
                return Err(AdapterError::Ledger("bad funding epoch".into()));
            }
            for (asset, delta) in entries {
                *self
                    .user_unsettled
                    .entry(*user)
                    .or_default()
                    .entry(*asset)
                    .or_insert(0) += *delta;
            }
            self.user_epoch.insert(*user, epoch);
            return Ok(());
        }
        let catch_up = epoch == self.book_funding_epoch && epoch == last.saturating_add(1);
        let replay = epoch == self.book_funding_epoch && epoch == last;
        if !catch_up && !replay {
            return Err(AdapterError::Ledger("bad funding epoch".into()));
        }
        if catch_up {
            for (asset, delta) in entries {
                *self
                    .user_unsettled
                    .entry(*user)
                    .or_default()
                    .entry(*asset)
                    .or_insert(0) += *delta;
            }
            self.user_epoch.insert(*user, epoch);
        } else if !entries.is_empty() {
            return Err(AdapterError::Ledger("fold replay rejects entries".into()));
        }
        let mut leftover_map = std::collections::BTreeMap::new();
        if let Some(m) = self.user_unsettled.remove(user) {
            let mut credits = Vec::new();
            let mut debits = Vec::new();
            for (asset, delta) in m {
                if delta >= 0 {
                    credits.push((asset, delta));
                } else {
                    debits.push((asset, delta));
                }
            }
            for (asset, delta) in credits.into_iter().chain(debits) {
                let rest = apply_signed_cash(self, user, delta)?;
                leftover_map.insert(asset, rest);
            }
        }
        leftover_map.retain(|_, v| *v != 0);
        self.user_unsettled.insert(*user, leftover_map);
        let cash = self.user_free.get(user).copied().unwrap_or(0)
            + self.user_reserved.get(user).copied().unwrap_or(0);
        self.user_cash.insert(*user, cash);
        Ok(())
    }

    fn user_equity(&self, user: &PubkeyBytes) -> i128 {
        let free = self.user_free.get(user).copied().unwrap_or(0);
        let reserved = self.user_reserved.get(user).copied().unwrap_or(0);
        let unsettled: i64 = self
            .user_unsettled
            .get(user)
            .map(|m| m.values().copied().sum::<i64>())
            .unwrap_or(0);
        let mut upnl = 0i128;
        if let Some(lots_m) = self.user_lots.get(user) {
            for (asset, lots) in lots_m {
                let entry = self
                    .user_entry
                    .get(user)
                    .and_then(|m| m.get(asset))
                    .copied()
                    .unwrap_or(0);
                upnl += cc::upnl_usdc(*lots, self.mark_usdc_per_lot, entry);
            }
        }
        cc::cinder_equity(free, reserved, unsettled, upnl)
    }

    fn liquidate_user(
        &mut self,
        user: &PubkeyBytes,
        asset_id: u16,
        client_oid: ClientOid,
    ) -> Result<(), AdapterError> {
        let delta = self
            .user_unsettled
            .get(user)
            .and_then(|m| m.get(&asset_id).copied());
        if let Some(delta) = delta {
            let rest = apply_signed_cash(self, user, delta)?;
            if let Some(m) = self.user_unsettled.get_mut(user) {
                if rest == 0 {
                    m.remove(&asset_id);
                } else {
                    m.insert(asset_id, rest);
                }
            }
        }
        let lots = self.lots_of(user, asset_id);
        if lots != 0 {
            let lots_delta = lots
                .checked_neg()
                .ok_or_else(|| AdapterError::Ledger("liq oid overflow".into()))?;
            self.pending_liq.insert(
                client_oid,
                PendingLiq {
                    user: *user,
                    client_oid,
                    asset_id,
                    lots_delta,
                },
            );
            if let Some(m) = self.user_lots.get_mut(user) {
                m.remove(&asset_id);
            }
        }
        self.liquidations.push((*user, asset_id));
        let cash = self.user_free.get(user).copied().unwrap_or(0)
            + self.user_reserved.get(user).copied().unwrap_or(0);
        self.user_cash.insert(*user, cash);
        Ok(())
    }

    fn last_scan_ms(&self) -> u64 {
        self.last_scan_ms
    }

    fn write_last_scan_ms(&mut self, ms: u64) {
        self.last_scan_ms = ms;
    }

    fn pending_liq_delta(&self, asset_id: u16) -> i64 {
        self.pending_liq
            .values()
            .filter(|p| p.asset_id == asset_id)
            .map(|p| p.lots_delta)
            .sum()
    }

    fn pending_liq_lots(&self, oid: &ClientOid) -> Option<i64> {
        self.pending_liq.get(oid).map(|p| p.lots_delta)
    }

    fn phoenix_collateral(&self) -> u64 {
        self.phoenix_collateral
    }

    fn set_phoenix_collateral(&mut self, v: u64) {
        self.phoenix_collateral = v;
    }

    fn i2_ok_unsettled(&self, pool_unsettled: i64, in_flight_usdc: i64) -> bool {
        MemoryLedger::i2_ok_unsettled(self, pool_unsettled, in_flight_usdc)
    }
}

fn apply_signed_cash(
    ledger: &mut MemoryLedger,
    user: &PubkeyBytes,
    delta: i64,
) -> Result<i64, AdapterError> {
    let free = ledger.user_free.entry(*user).or_insert(0);
    let reserved = ledger.user_reserved.entry(*user).or_insert(0);
    if delta > 0 {
        *free = free
            .checked_add(delta as u64)
            .ok_or_else(|| AdapterError::Ledger("overflow".into()))?;
        return Ok(0);
    }
    if delta == 0 {
        return Ok(0);
    }
    if delta == i64::MIN {
        return Err(AdapterError::Ledger("funding delta overflow".into()));
    }
    let mut owe = delta.unsigned_abs();
    let take_free = owe.min(*free);
    *free -= take_free;
    owe -= take_free;
    if owe > 0 {
        let take_res = owe.min(*reserved);
        *reserved -= take_res;
        owe -= take_res;
    }
    Ok(if owe == 0 { 0 } else { -(owe as i64) })
}
