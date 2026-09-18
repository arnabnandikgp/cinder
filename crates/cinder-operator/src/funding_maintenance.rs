//! Funding-rate maintenance, distinct from custody top-ups in `funding`.
//! Checkpoints contain public rates and opaque private inventory/registry hashes.
//! Allocation instruction bodies exist only in memory; the WAL stores hashes.
use crate::ledger::confirmed_position;
use crate::rise::RiseView;
use crate::rpc::{Result, RuntimeError};
use crate::runtime::{book_key, config_key, ledger_id, pda, validate_ledger};
use crate::transaction::{instruction_data, Receipt};
use anchor_lang::InstructionData;
use cinder_common as cc;
use cinder_ledger::{Book, FundingEntry, UserLedger};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
type UserRegistry = BTreeMap<[u8; 32], [u8; 32]>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FundingRate {
    pub cumulative_quote_lots_per_base_lot: i64,
    pub last_update_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FundingCheckpoint {
    pub(crate) epoch: u64,
    pub(crate) native_slot: u64,
    pub(crate) observed_ms: u64,
    pub(crate) rates: BTreeMap<u16, FundingRate>,
    pub(crate) inventory_hash: [u8; 32],
    pub(crate) registry_hash: [u8; 32],
    pub(crate) bootstrap_safe: bool,
    pub(crate) ownership_down: bool,
}
impl FundingCheckpoint {
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn rates(&self) -> &BTreeMap<u16, FundingRate> {
        &self.rates
    }
    pub(crate) fn validate(&self) -> Result<()> {
        if self.native_slot == 0
            || self.observed_ms == 0
            || self.rates.is_empty()
            || self.rates.len() > cc::MAX_BOOK_MARKETS
            || self.rates.contains_key(&0)
            || self.inventory_hash == [0; 32]
            || self.registry_hash == [0; 32]
            || self.rates.values().any(|r| {
                r.last_update_seconds
                    .checked_mul(1000)
                    .is_none_or(|ms| ms > self.observed_ms)
            })
        {
            return Err(RuntimeError::Decode);
        }
        Ok(())
    }
    pub(crate) fn capture(
        view: &RiseView,
        book: &Book,
        ledgers: &[([u8; 32], UserLedger)],
        now_ms: u64,
    ) -> Result<Self> {
        if [view.observed_ms, view.mark_ms].into_iter().any(|t| {
            t == 0
                || now_ms
                    .checked_sub(t)
                    .is_none_or(|age| age > cc::MARK_STALE_MS)
        }) {
            return Err(RuntimeError::Stale);
        }
        if book.schema_version != cc::ACCOUNT_SCHEMA_VERSION
            || book.invariant_ok != 1
            || book.halt & cc::INVARIANT_BROKEN != 0
            || book.residual_len as usize > book.residuals.len()
        {
            return Err(RuntimeError::Identity);
        }
        let (inventory_hash, registry) =
            inventory(book, ledgers, book.funding_epoch, book.funding_epoch)?;
        let mut lots = BTreeMap::new();
        let mut cash = 0i128;
        let mut bootstrap_safe = book.last_ack_slot_er == 0 && view.funding == 0;
        for (_, ledger) in ledgers {
            cash = cash
                .checked_add(
                    i128::from(ledger.free) + i128::from(ledger.reserved)
                        - i128::from(ledger.bad_debt_usdc),
                )
                .ok_or(RuntimeError::Decode)?;
            bootstrap_safe &= ledger.nonce == 0 && ledger.bad_debt_usdc == 0;
            for p in &ledger.positions[..ledger.positions_len as usize] {
                if !view.markets.contains_key(&p.asset_id) {
                    return Err(RuntimeError::Incomplete);
                }
                let n = confirmed_position(p, &ledger.open_oids)?;
                let total = lots.entry(p.asset_id).or_insert(0i128);
                *total = total
                    .checked_add(i128::from(n))
                    .ok_or(RuntimeError::Decode)?;
                bootstrap_safe &= n == 0 && p.unsettled_funding == 0;
            }
        }
        let mut residuals = BTreeMap::new();
        for r in &book.residuals[..book.residual_len as usize] {
            if r.asset_id == 0 || residuals.insert(r.asset_id, i128::from(r.lots)).is_some() {
                return Err(RuntimeError::Identity);
            }
        }
        let mut native = view
            .positions
            .iter()
            .map(|(a, n)| (*a, i128::from(*n)))
            .collect::<BTreeMap<_, _>>();
        lots.retain(|_, n| *n != 0);
        residuals.retain(|_, n| *n != 0);
        native.retain(|_, n| *n != 0);
        if lots != residuals || lots != native {
            return Err(RuntimeError::Identity);
        }
        bootstrap_safe &= native.is_empty()
            && cash == i128::from(view.vault_balance) + i128::from(view.collateral);
        let rates = view
            .markets
            .iter()
            .map(|(asset, m)| {
                Ok((
                    *asset,
                    FundingRate {
                        cumulative_quote_lots_per_base_lot: m.cumulative_funding_rate.as_inner(),
                        last_update_seconds: *view
                            .funding_updates_seconds
                            .get(asset)
                            .ok_or(RuntimeError::Incomplete)?,
                    },
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let out = Self {
            epoch: book.funding_epoch,
            native_slot: view.slot,
            observed_ms: view.observed_ms,
            rates,
            inventory_hash,
            registry_hash: registry_hash(&registry),
            bootstrap_safe,
            ownership_down: book.halt & cc::OPERATOR_DOWN != 0
                && view.halt & cc::OPERATOR_DOWN != 0,
        };
        out.validate()?;
        Ok(out)
    }
}

pub(crate) fn registry_hash(users: &BTreeMap<[u8; 32], [u8; 32]>) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"cinder:funding-registry:v1");
    h.update((users.len() as u64).to_le_bytes());
    for (ledger, user) in users {
        h.update(ledger);
        h.update(user);
    }
    h.finalize().into()
}

fn inventory(
    book: &Book,
    ledgers: &[([u8; 32], UserLedger)],
    floor: u64,
    ceiling: u64,
) -> Result<([u8; 32], UserRegistry)> {
    if book.schema_version != cc::ACCOUNT_SCHEMA_VERSION
        || book.invariant_ok != 1
        || book.halt & cc::INVARIANT_BROKEN != 0
        || !(floor..=ceiling).contains(&book.funding_epoch)
    {
        return Err(RuntimeError::Identity);
    }
    let mut hash = Sha256::new();
    hash.update(b"cinder:funding-inventory:v1");
    hash.update(book.last_ack_slot_er.to_le_bytes());
    if book.residual_len as usize > book.residuals.len() {
        return Err(RuntimeError::Identity);
    }
    let mut residuals = BTreeMap::new();
    for r in &book.residuals[..book.residual_len as usize] {
        if r.asset_id == 0 || residuals.insert(r.asset_id, r.lots).is_some() {
            return Err(RuntimeError::Identity);
        }
    }
    hash.update([book.residual_len]);
    for (asset, lots) in residuals {
        hash.update(asset.to_le_bytes());
        hash.update(lots.to_le_bytes());
    }
    let mut users = BTreeMap::new();
    let mut registry = BTreeMap::new();
    for (address, l) in ledgers {
        validate_ledger(l)?;
        if l.pending_oid_count != 0
            || !(floor..=ceiling).contains(&l.last_funding_epoch)
            || l.withdrawable != 0
            || users.insert(l.user.to_bytes(), l).is_some()
            || *address != pda(ledger_id(), &[cc::SEED_USER, l.user.as_ref()])
            || registry.insert(*address, l.user.to_bytes()).is_some()
        {
            return Err(RuntimeError::Identity);
        }
    }
    hash.update(
        u32::try_from(users.len())
            .map_err(|_| RuntimeError::Decode)?
            .to_le_bytes(),
    );
    for (user, l) in users {
        hash.update(user);
        hash.update(l.nonce.to_le_bytes());
        let mut ps = l.positions[..l.positions_len as usize]
            .iter()
            .filter(|p| p.lots != 0)
            .collect::<Vec<_>>();
        ps.sort_by_key(|p| p.asset_id);
        hash.update([ps.len() as u8]);
        // Validate flat rows too: a nonzero retained basis is not flat inventory.
        for p in &l.positions[..l.positions_len as usize] {
            confirmed_position(p, &l.open_oids)?;
        }
        for p in ps {
            hash.update(p.asset_id.to_le_bytes());
            hash.update(p.lots.to_le_bytes());
            hash.update(p.entry_quote_lots.to_le_bytes());
        }
    }
    Ok((hash.finalize().into(), registry))
}

pub struct FundingEpochPlan {
    pub(crate) previous: FundingCheckpoint,
    pub(crate) target: FundingCheckpoint,
    pub(crate) steps: BTreeMap<[u8; 32], Vec<u8>>,
}
impl FundingEpochPlan {
    /// Quiet-inventory path only. Position changes across update generations
    /// need authoritative history/checkpoint barriers, NEVER current-size catchup.
    pub fn build(
        previous: &FundingCheckpoint,
        observed: &FundingCheckpoint,
        book: &Book,
        ledgers: &[([u8; 32], UserLedger)],
        now_ms: u64,
    ) -> Result<Self> {
        previous.validate()?;
        observed.validate()?;
        if !observed.ownership_down
            || previous.epoch != book.funding_epoch
            || observed.epoch != previous.epoch
            || observed.registry_hash != previous.registry_hash
            || observed.inventory_hash != previous.inventory_hash
            || observed.native_slot < previous.native_slot
            || observed.observed_ms < previous.observed_ms
            || now_ms
                .checked_sub(observed.observed_ms)
                .is_none_or(|a| a > cc::TRADER_STATE_STALE_MS)
            || previous.rates.keys().ne(observed.rates.keys())
        {
            return Err(RuntimeError::Incomplete);
        }
        let mut target = observed.clone();
        target.epoch = previous.epoch.checked_add(1).ok_or(RuntimeError::Decode)?;
        Self::resume(previous, &target, book, ledgers)
    }
    /// Reconstruct the SAME immutable bodies after a partially applied epoch.
    /// Mixed previous/target user epochs are intentional, not a skipped gap.
    pub fn resume(
        previous: &FundingCheckpoint,
        target: &FundingCheckpoint,
        book: &Book,
        ledgers: &[([u8; 32], UserLedger)],
    ) -> Result<Self> {
        validate_transition(previous, target)?;
        if book.halt & cc::OPERATOR_DOWN == 0 {
            return Err(RuntimeError::Identity);
        }
        let (hash, registry) = inventory(book, ledgers, previous.epoch, target.epoch)?;
        if hash != target.inventory_hash || registry_hash(&registry) != target.registry_hash {
            return Err(RuntimeError::Identity);
        }
        let mut steps = BTreeMap::new();
        steps.insert(
            book_key(),
            cinder_ledger::instruction::BumpFundingEpoch {
                epoch: target.epoch,
            }
            .data(),
        );
        for (address, l) in ledgers {
            let mut entries = Vec::new();
            for p in &l.positions[..l.positions_len as usize] {
                if p.lots == 0 {
                    continue;
                }
                let old = previous
                    .rates
                    .get(&p.asset_id)
                    .ok_or(RuntimeError::Incomplete)?;
                let new = &target.rates[&p.asset_id];
                let delta = i128::from(new.cumulative_quote_lots_per_base_lot)
                    .checked_sub(i128::from(old.cumulative_quote_lots_per_base_lot))
                    .and_then(|n| n.checked_mul(i128::from(p.lots)))
                    .and_then(|n| n.checked_neg())
                    .ok_or(RuntimeError::Decode)?;
                let delta_usdc = i64::try_from(delta).map_err(|_| RuntimeError::Decode)?;
                if l.last_funding_epoch == previous.epoch {
                    p.unsettled_funding
                        .checked_add(delta_usdc)
                        .ok_or(RuntimeError::Decode)?;
                }
                entries.push(FundingEntry {
                    asset_id: p.asset_id,
                    delta_usdc,
                    post_position_im_usdc: 0,
                });
            }
            entries.sort_by_key(|e| e.asset_id);
            steps.insert(
                *address,
                cinder_ledger::instruction::AllocateFunding {
                    epoch: target.epoch,
                    fold: false,
                    entries,
                }
                .data(),
            );
        }
        Ok(Self {
            previous: previous.clone(),
            target: target.clone(),
            steps,
        })
    }
    pub fn epoch(&self) -> u64 {
        self.target.epoch
    }
    pub fn instruction(&self, scope: &[u8; 32]) -> Option<&[u8]> {
        self.steps.get(scope).map(Vec::as_slice)
    }
}

pub(crate) fn validate_transition(old: &FundingCheckpoint, new: &FundingCheckpoint) -> Result<()> {
    old.validate()?;
    new.validate()?;
    if old.epoch.checked_add(1) != Some(new.epoch)
        || !new.ownership_down
        || old.inventory_hash != new.inventory_hash
        || old.registry_hash != new.registry_hash
        || new.native_slot < old.native_slot
        || new.observed_ms < old.observed_ms
        || old.rates.keys().ne(new.rates.keys())
    {
        return Err(RuntimeError::Identity);
    }
    let mut changed = false;
    for (a, r) in &old.rates {
        let n = &new.rates[a];
        if n.last_update_seconds < r.last_update_seconds
            || (n.last_update_seconds == r.last_update_seconds && n != r)
        {
            return Err(RuntimeError::Identity);
        }
        changed |= n.last_update_seconds > r.last_update_seconds;
    }
    if !changed {
        return Err(RuntimeError::Incomplete);
    }
    Ok(())
}

/// Produced only after a caller obtains the exact receipt from authenticated
/// QFS. Parsing untrusted/offline JSON is not itself chain-finality proof.
pub struct FundingStepObservation {
    pub(crate) epoch: u64,
    pub(crate) scope: [u8; 32],
    pub(crate) body_hash: [u8; 32],
    pub(crate) signature: [u8; 64],
    pub(crate) slot: u64,
    pub(crate) succeeded: bool,
}
impl FundingStepObservation {
    pub fn decode(
        value: &Value,
        signature: &[u8; 64],
        adapter: &[u8; 32],
        epoch: u64,
        scope: [u8; 32],
        body_hash: [u8; 32],
    ) -> Result<Self> {
        let r = Receipt::decode(value, signature)?;
        if r.slot == 0 || r.instructions.len() != 1 || !r.requires_signature(adapter) {
            return Err(RuntimeError::Identity);
        }
        let ix = &r.instructions[0];
        let body = instruction_data(ix)?;
        if r.program(ix)? != ledger_id()
            || r.account(ix, 0)? != *adapter
            || r.account(ix, 1)? != config_key()
            || <[u8; 32]>::from(Sha256::digest(&body)) != body_hash
        {
            return Err(RuntimeError::Identity);
        }
        let count = crate::rpc::array(&ix["accounts"])?.len();
        if scope == book_key() {
            let p = crate::transaction::decode_ix::<cinder_ledger::instruction::BumpFundingEpoch>(
                &body,
            )?
            .ok_or(RuntimeError::Identity)?;
            if count != 3 || r.account(ix, 2)? != book_key() || p.epoch != epoch {
                return Err(RuntimeError::Identity);
            }
        } else {
            let p = crate::transaction::decode_ix::<cinder_ledger::instruction::AllocateFunding>(
                &body,
            )?
            .ok_or(RuntimeError::Identity)?;
            if count != 4
                || r.account(ix, 2)? != scope
                || r.account(ix, 3)? != book_key()
                || p.epoch != epoch
                || p.fold
            {
                return Err(RuntimeError::Identity);
            }
        }
        Ok(Self {
            epoch,
            scope,
            body_hash,
            signature: *signature,
            slot: r.slot,
            succeeded: r.succeeded,
        })
    }
}

#[cfg(test)]
#[path = "../tests/support/funding_maintenance.rs"]
pub(crate) mod tests;
