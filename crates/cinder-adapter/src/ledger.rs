use cinder_common as cc;

use crate::phoenix::Fill;
use crate::{AdapterError, ClientOid, PubkeyBytes};

/// On-chain / ER writes the adapter needs. Tests use [`MemoryLedger`].
pub trait LedgerPort {
    fn ack_fill(&mut self, user: &PubkeyBytes, fill: &Fill) -> Result<(), AdapterError>;
    fn ack_fail(
        &mut self,
        user: &PubkeyBytes,
        oid: &ClientOid,
        post_position_im_usdc: u64,
    ) -> Result<(), AdapterError>;
    fn write_halt(&mut self, flags: u8) -> Result<(), AdapterError>;
    fn book_lots(&self, asset_id: u16) -> i64;
    fn config_halt(&self) -> u8;
    fn book_halt(&self) -> u8;
    fn invariant_ok(&self) -> bool;
    fn write_reserve_root(&mut self, now_ms: u64) -> Result<(), AdapterError>;
    /// Cash collateral and unsettled funding, excluding uPnL. The RiskEngine
    /// applies the one authoritative Rise uPnL value from its snapshot.
    fn user_collateral(&self, user: &PubkeyBytes) -> i128;
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
    pub user_position_im:
        std::collections::BTreeMap<PubkeyBytes, std::collections::BTreeMap<u16, u64>>,
    pub user_lots: std::collections::BTreeMap<PubkeyBytes, std::collections::BTreeMap<u16, i64>>,
    pub user_cash: std::collections::BTreeMap<PubkeyBytes, i128>,
    pub vault_ata: u64,
    pub phoenix_collateral: u64,
    pub reserve_roots: Vec<u64>,
    pub fail_next_root: bool,
    pub book_funding_epoch: u64,
    pub user_epoch: std::collections::BTreeMap<PubkeyBytes, u64>,
    pub user_free: std::collections::BTreeMap<PubkeyBytes, u64>,
    pub user_reserved: std::collections::BTreeMap<PubkeyBytes, u64>,
    pub user_bad_debt: std::collections::BTreeMap<PubkeyBytes, u64>,
    pub user_unsettled:
        std::collections::BTreeMap<PubkeyBytes, std::collections::BTreeMap<u16, i64>>,
    pub user_entry: std::collections::BTreeMap<PubkeyBytes, std::collections::BTreeMap<u16, i64>>,
    pub pending_liq: std::collections::BTreeMap<ClientOid, PendingLiq>,
    pub liquidations: Vec<(PubkeyBytes, u16)>,
    pub last_scan_ms: u64,
    pub phoenix_fees_paid: u64,
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

    pub fn sum_user_cash(&self) -> i128 {
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
        self.user_bad_debt.entry(user).or_insert(0);
        self.user_epoch
            .entry(user)
            .or_insert(self.book_funding_epoch);
        self.user_unsettled.entry(user).or_default();
        self.user_lots.entry(user).or_default();
        self.sync_user_cash(&user);
    }

    fn sync_user_cash(&mut self, user: &PubkeyBytes) {
        let free = self.user_free.get(user).copied().unwrap_or(0) as i128;
        let reserved = self.user_reserved.get(user).copied().unwrap_or(0) as i128;
        let bad_debt = self.user_bad_debt.get(user).copied().unwrap_or(0) as i128;
        self.user_cash.insert(*user, free + reserved - bad_debt);
    }

    fn aggregate_asset_im_after_fill(
        &self,
        user: &PubkeyBytes,
        asset_id: u16,
        next_user_lots: i64,
    ) -> Result<u64, AdapterError> {
        let mut total = self
            .user_position_im
            .get(user)
            .and_then(|m| m.get(&asset_id))
            .copied()
            .unwrap_or(0);
        if next_user_lots == 0 {
            total = 0;
        }
        for (owner, positions) in &self.user_lots {
            if owner == user {
                continue;
            }
            let _lots = positions.get(&asset_id).copied().unwrap_or(0);
            let im = self
                .user_position_im
                .get(owner)
                .and_then(|m| m.get(&asset_id))
                .copied()
                .unwrap_or(0);
            total = total
                .checked_add(im)
                .ok_or_else(|| AdapterError::Ledger("aggregate im overflow".into()))?;
        }
        Ok(total)
    }

    fn rebalance_user_margin(&mut self, user: &PubkeyBytes) -> Result<bool, AdapterError> {
        let target = self
            .user_position_im
            .get(user)
            .map(|margins| {
                margins.values().try_fold(0u64, |total, margin| {
                    total
                        .checked_add(*margin)
                        .ok_or_else(|| AdapterError::Ledger("user margin target overflow".into()))
                })
            })
            .transpose()?
            .unwrap_or(0);
        let free = self.user_free.get(user).copied().unwrap_or(0);
        let reserved = self.user_reserved.get(user).copied().unwrap_or(0);
        let available = free
            .checked_add(reserved)
            .ok_or_else(|| AdapterError::Ledger("user collateral overflow".into()))?;
        let next_reserved = target.min(available);
        self.user_reserved.insert(*user, next_reserved);
        self.user_free.insert(*user, available - next_reserved);
        if target > available {
            self.book_halt |= cc::HALT_ENTRIES;
            self.config_halt |= cc::HALT_ENTRIES;
        }
        self.sync_user_cash(user);
        Ok(target > available)
    }

    fn set_user_position_margin(
        &mut self,
        user: &PubkeyBytes,
        asset_id: u16,
        margin_usdc: u64,
    ) -> Result<(), AdapterError> {
        if self.lots_of(user, asset_id) == 0 {
            if margin_usdc != 0 {
                return Err(AdapterError::Ledger("flat funding margin".into()));
            }
            if let Some(margins) = self.user_position_im.get_mut(user) {
                margins.remove(&asset_id);
            }
        } else {
            self.user_position_im
                .entry(*user)
                .or_default()
                .insert(asset_id, margin_usdc);
        }
        Ok(())
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
        entries: &[(u16, i64, u64)],
    ) -> Result<(), AdapterError>;
    /// Persist one user's funding accrual and any resulting tentative
    /// liquidation atomically. Production ports must submit both ER
    /// instructions in one transaction when `liquidation` is present.
    fn apply_funding_decision(
        &mut self,
        user: &PubkeyBytes,
        epoch: u64,
        entries: &[(u16, i64, u64)],
        liquidation: Option<(u16, ClientOid)>,
    ) -> Result<(), AdapterError>;
    fn user_collateral(&self, user: &PubkeyBytes) -> i128;
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
        if let Some((prior_user, prior_fill)) = self
            .fills
            .iter()
            .find(|(_, prior)| prior.client_oid == fill.client_oid)
        {
            if prior_user == user && prior_fill == fill {
                return Ok(());
            }
            return Err(AdapterError::Ledger(
                "conflicting duplicate fill oid".into(),
            ));
        }
        let liq = self.pending_liq.get(&fill.client_oid).cloned();
        let lots_before = if let Some(ref pending) = liq {
            pending
                .lots_delta
                .checked_neg()
                .ok_or_else(|| AdapterError::Ledger("liq lots overflow".into()))?
        } else {
            self.lots_of(user, fill.asset_id)
        };
        let next_user_lots = lots_before
            .checked_add(fill.filled_lots)
            .ok_or_else(|| AdapterError::Ledger("lots overflow".into()))?;
        let entry_before = self
            .user_entry
            .get(user)
            .and_then(|m| m.get(&fill.asset_id))
            .copied()
            .unwrap_or(0);
        let realized = cc::realize_on_fill(
            lots_before,
            fill.filled_lots,
            entry_before,
            fill.vwap_quote_lots,
        )
        .ok_or_else(|| AdapterError::Ledger("fill realization overflow".into()))?;
        let next_book = self
            .book_lots(fill.asset_id)
            .checked_add(fill.filled_lots)
            .ok_or_else(|| AdapterError::Ledger("book overflow".into()))?;
        if next_user_lots == 0 && fill.post_position_im_usdc != 0 {
            return Err(AdapterError::Ledger(
                "flat fill supplied nonzero margin".into(),
            ));
        }
        if next_user_lots == 0 {
            self.user_position_im
                .entry(*user)
                .or_default()
                .remove(&fill.asset_id);
        } else {
            self.user_position_im
                .entry(*user)
                .or_default()
                .insert(fill.asset_id, fill.post_position_im_usdc);
        }
        let aggregate_im =
            self.aggregate_asset_im_after_fill(user, fill.asset_id, next_user_lots)?;
        let next_fee_total = self
            .phoenix_fees_paid
            .checked_add(fill.fee_usdc)
            .ok_or_else(|| AdapterError::Ledger("fee accrual overflow".into()))?;
        let mut next_free = self.user_free.get(user).copied().unwrap_or(0);
        let mut next_reserved = self.user_reserved.get(user).copied().unwrap_or(0);
        let mut next_bad_debt = self.user_bad_debt.get(user).copied().unwrap_or(0);
        cc::apply_signed_cash(
            &mut next_free,
            &mut next_reserved,
            &mut next_bad_debt,
            realized.realized_usdc,
        )
        .ok_or_else(|| AdapterError::Ledger("realized pnl overflow".into()))?;
        cc::debit_cash(
            &mut next_free,
            &mut next_reserved,
            &mut next_bad_debt,
            fill.fee_usdc,
        )
        .ok_or_else(|| AdapterError::Ledger("fee debt overflow".into()))?;

        if next_book == 0 {
            self.book.remove(&fill.asset_id);
        } else {
            self.book.insert(fill.asset_id, next_book);
        }
        self.fills.push((*user, fill.clone()));
        self.pending_liq.remove(&fill.client_oid);
        if next_user_lots == 0 {
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
                .insert(fill.asset_id, next_user_lots);
            self.user_entry
                .entry(*user)
                .or_default()
                .insert(fill.asset_id, realized.new_entry_quote);
        }
        self.user_free.insert(*user, next_free);
        self.user_reserved.insert(*user, next_reserved);
        self.user_bad_debt.insert(*user, next_bad_debt);
        self.phoenix_fees_paid = next_fee_total;
        if next_bad_debt > 0 {
            self.book_halt |= cc::BAD_DEBT | cc::HALT_ENTRIES | cc::HALT_WITHDRAW;
            self.config_halt |= cc::BAD_DEBT | cc::HALT_ENTRIES | cc::HALT_WITHDRAW;
        }
        if aggregate_im == 0 {
            self.reserved.remove(&fill.asset_id);
        } else {
            self.reserved.insert(fill.asset_id, aggregate_im);
        }
        self.rebalance_user_margin(user)?;
        Ok(())
    }

    fn ack_fail(
        &mut self,
        user: &PubkeyBytes,
        oid: &ClientOid,
        post_position_im_usdc: u64,
    ) -> Result<(), AdapterError> {
        if let Some(liq) = self.pending_liq.remove(oid) {
            let restored = liq
                .lots_delta
                .checked_neg()
                .ok_or_else(|| AdapterError::Ledger("liq fail overflow".into()))?;
            if restored == 0 {
                if let Some(m) = self.user_lots.get_mut(user) {
                    m.remove(&liq.asset_id);
                }
                if let Some(m) = self.user_position_im.get_mut(user) {
                    m.remove(&liq.asset_id);
                }
            } else {
                self.user_lots
                    .entry(*user)
                    .or_default()
                    .insert(liq.asset_id, restored);
                self.user_position_im
                    .entry(*user)
                    .or_default()
                    .insert(liq.asset_id, post_position_im_usdc);
            }
        }
        self.rebalance_user_margin(user)?;
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

    fn user_collateral(&self, user: &PubkeyBytes) -> i128 {
        <Self as FundingPort>::user_collateral(self, user)
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
        entries: &[(u16, i64, u64)],
    ) -> Result<(), AdapterError> {
        let last = self.last_funding_epoch(user);
        if !fold {
            if epoch != self.book_funding_epoch || epoch != last.saturating_add(1) {
                return Err(AdapterError::Ledger("bad funding epoch".into()));
            }
            for (asset, delta, _) in entries {
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
            for (asset, delta, im) in entries {
                *self
                    .user_unsettled
                    .entry(*user)
                    .or_default()
                    .entry(*asset)
                    .or_insert(0) += *delta;
                self.set_user_position_margin(user, *asset, *im)?;
            }
            self.user_epoch.insert(*user, epoch);
        } else if entries.iter().any(|(_, delta, _)| *delta != 0) {
            return Err(AdapterError::Ledger(
                "fold replay rejects funding delta".into(),
            ));
        } else {
            for (asset, _, im) in entries {
                self.set_user_position_margin(user, *asset, *im)?;
            }
        }
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
            for (_, delta) in credits.into_iter().chain(debits) {
                apply_signed_cash(self, user, delta)?;
            }
        }
        self.user_unsettled.insert(*user, Default::default());
        self.rebalance_user_margin(user)?;
        Ok(())
    }

    fn apply_funding_decision(
        &mut self,
        user: &PubkeyBytes,
        epoch: u64,
        entries: &[(u16, i64, u64)],
        liquidation: Option<(u16, ClientOid)>,
    ) -> Result<(), AdapterError> {
        let mut next = self.clone();
        <Self as FundingPort>::allocate_funding(&mut next, user, epoch, false, entries)?;
        if let Some((asset_id, client_oid)) = liquidation {
            <Self as FundingPort>::liquidate_user(&mut next, user, asset_id, client_oid)?;
        }
        *self = next;
        Ok(())
    }

    fn user_collateral(&self, user: &PubkeyBytes) -> i128 {
        let free = self.user_free.get(user).copied().unwrap_or(0);
        let reserved = self.user_reserved.get(user).copied().unwrap_or(0);
        let bad_debt = self.user_bad_debt.get(user).copied().unwrap_or(0);
        let unsettled: i64 = self
            .user_unsettled
            .get(user)
            .map(|m| m.values().copied().sum::<i64>())
            .unwrap_or(0);
        free as i128 + reserved as i128 + unsettled as i128 - bad_debt as i128
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
            apply_signed_cash(self, user, delta)?;
            if let Some(m) = self.user_unsettled.get_mut(user) {
                m.remove(&asset_id);
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
            if let Some(m) = self.user_position_im.get_mut(user) {
                m.remove(&asset_id);
            }
        }
        self.liquidations.push((*user, asset_id));
        self.rebalance_user_margin(user)?;
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
) -> Result<(), AdapterError> {
    let free = ledger.user_free.entry(*user).or_insert(0);
    let reserved = ledger.user_reserved.entry(*user).or_insert(0);
    let bad_debt = ledger.user_bad_debt.entry(*user).or_insert(0);
    cc::apply_signed_cash(free, reserved, bad_debt, delta)
        .ok_or_else(|| AdapterError::Ledger("cash overflow".into()))?;
    if *bad_debt > 0 {
        ledger.book_halt |= cc::BAD_DEBT | cc::HALT_ENTRIES | cc::HALT_WITHDRAW;
        ledger.config_halt |= cc::BAD_DEBT | cc::HALT_ENTRIES | cc::HALT_WITHDRAW;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(n: u8) -> PubkeyBytes {
        [n; 32]
    }

    fn fill(tag: u8, lots: i64, fee: u64, vwap: i64) -> Fill {
        let mut client_oid = [0u8; 16];
        client_oid[0] = tag;
        Fill {
            client_oid,
            asset_id: 1,
            filled_lots: lots,
            fee_usdc: fee,
            vwap_quote_lots: vwap,
            fill_price_ticks: 1,
            post_position_im_usdc: 0,
        }
    }

    #[test]
    fn fills_persist_entry_and_realize_loss_before_fee() {
        let a = user(1);
        let mut ledger = MemoryLedger::new();
        ledger.ensure_user(a, 3_000_000);

        ledger.ack_fill(&a, &fill(1, 10, 0, 10_000_000)).unwrap();
        assert_eq!(ledger.user_entry[&a][&1], 10_000_000);

        ledger
            .ack_fill(&a, &fill(2, -10, 500_000, -1_000_000))
            .unwrap();
        assert_eq!(ledger.lots_of(&a, 1), 0);
        assert_eq!(ledger.user_free[&a], 0);
        assert_eq!(ledger.user_bad_debt[&a], 6_500_000);
        assert_eq!(ledger.user_cash[&a], -6_500_000);
        assert_eq!(ledger.phoenix_fees_paid, 500_000);
    }

    #[test]
    fn liquidation_fill_realizes_from_pre_liquidation_position() {
        let a = user(1);
        let mut ledger = MemoryLedger::new();
        ledger.ensure_user(a, 0);
        ledger.user_lots.insert(a, [(1, 10)].into_iter().collect());
        ledger
            .user_entry
            .insert(a, [(1, 10_000_000)].into_iter().collect());
        ledger.book.insert(1, 10);

        let liq_oid = fill(3, -4, 0, -4_400_000).client_oid;
        ledger.liquidate_user(&a, 1, liq_oid).unwrap();
        ledger.ack_fill(&a, &fill(3, -4, 0, -4_400_000)).unwrap();

        assert_eq!(ledger.lots_of(&a, 1), 6);
        assert_eq!(ledger.user_entry[&a][&1], 6_000_000);
        assert_eq!(ledger.user_free[&a], 400_000);
        assert_eq!(ledger.user_bad_debt[&a], 0);
    }

    #[test]
    fn reserved_margin_is_aggregated_across_users() {
        let a = user(1);
        let b = user(2);
        let mut ledger = MemoryLedger::new();
        ledger.ensure_user(a, 0);
        ledger.ensure_user(b, 0);

        let mut a_fill = fill(4, 10, 0, 0);
        a_fill.post_position_im_usdc = 1_250_000;
        ledger.ack_fill(&a, &a_fill).unwrap();
        let mut b_fill = fill(5, 10, 0, 0);
        b_fill.post_position_im_usdc = 1_250_000;
        ledger.ack_fill(&b, &b_fill).unwrap();

        assert_eq!(ledger.reserved[&1], 2_500_000);
    }

    #[test]
    fn funding_decision_rolls_back_epoch_when_liquidation_fails() {
        let user = user(3);
        let mut ledger = MemoryLedger::new();
        ledger.ensure_user(user, u64::MAX);
        ledger
            .user_lots
            .insert(user, [(1, 1)].into_iter().collect());
        ledger.bump_funding_epoch(1).unwrap();

        let result = ledger.apply_funding_decision(&user, 1, &[(1, 1, 0)], Some((1, [9u8; 16])));
        assert!(result.is_err());
        assert_eq!(ledger.last_funding_epoch(&user), 0);
        assert!(ledger.user_unsettled[&user].is_empty());
        assert_eq!(ledger.lots_of(&user, 1), 1);
        assert!(ledger.pending_liq.is_empty());
    }
}
