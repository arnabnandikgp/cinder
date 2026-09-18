//! Public funding fences for inventory-changing acknowledgements. The anchor
//! never grants trading permission and does not persist a private ledger.
use super::{Journal, JournalError};
use crate::{FundingCheckpoint, FundingRate, OperationId, OrderState};
use rusqlite::{params, OptionalExtension};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

impl Journal {
    /// Schema-eight checkpoints acquire an anchor only after the caller has
    /// reobserved exactly the committed inventory, with every outbox idle.
    pub(crate) fn ensure_inventory_anchor(
        &mut self,
        cp: &FundingCheckpoint,
    ) -> Result<(), JournalError> {
        if self.funding_checkpoint()?.as_ref() != Some(cp)
            || self.has_unresolved_maintenance()?
            || self.has_unresolved_funding()?
            || !self.nonterminal_operations()?.is_empty()
        {
            return Err(JournalError::IntentConflict);
        }
        let exists: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM funding_inventory_anchor)",
            [],
            |r| r.get(0),
        )?;
        if !exists {
            Self::write_inventory_anchor(&self.connection, cp)?;
        }
        self.validate_funding_barriers()
    }
    pub(super) fn write_inventory_anchor(
        db: &rusqlite::Connection,
        cp: &FundingCheckpoint,
    ) -> Result<(), JournalError> {
        let bytes = serde_json::to_vec(cp).map_err(|_| JournalError::IntentConflict)?;
        db.execute("INSERT INTO funding_inventory_anchor VALUES(1,?,(SELECT coalesce(max(rowid),0) FROM operations)) ON CONFLICT(singleton) DO UPDATE SET checkpoint_hash=excluded.checkpoint_hash,last_operation=excluded.last_operation",params![Sha256::digest(bytes).as_slice()])?;
        Ok(())
    }
    pub(crate) fn record_order_funding_barrier(
        &mut self,
        id: &OperationId,
        rates: &BTreeMap<u16, FundingRate>,
    ) -> Result<(), JournalError> {
        let cp = self
            .funding_checkpoint()?
            .ok_or(JournalError::IntentConflict)?;
        let op = self.operation(id)?;
        if op.state != OrderState::Prepared
            || op.venue_signature.is_some()
            || cp.rates() != rates
            || self.has_unresolved_maintenance()?
        {
            return Err(JournalError::IntentConflict);
        }
        let body = serde_json::to_vec(rates).map_err(|_| JournalError::IntentConflict)?;
        let old: Option<Vec<u8>> = self
            .connection
            .query_row(
                "SELECT rates FROM order_funding_barriers WHERE operation_id=?",
                params![id.as_slice()],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(old) = old {
            return if old == body {
                Ok(())
            } else {
                Err(JournalError::IntentConflict)
            };
        }
        self.connection.execute(
            "INSERT INTO order_funding_barriers VALUES(?,?)",
            params![id.as_slice(), body],
        )?;
        Ok(())
    }
    /// Caller supplies a coherent post-ACK inventory. Carry the OLD accumulator
    /// baseline only when every intervening native fill has the atomic fence.
    /// A newer observed native generation is processed separately after rebase.
    pub(crate) fn rebase_acknowledged_inventory(
        &mut self,
        observed: &FundingCheckpoint,
    ) -> Result<FundingCheckpoint, JournalError> {
        if self.has_unresolved_maintenance()?
            || self.has_unresolved_funding()?
            || !self.nonterminal_operations()?.is_empty()
        {
            return Err(JournalError::IntentConflict);
        }
        let old = self
            .funding_checkpoint()?
            .ok_or(JournalError::IntentConflict)?;
        if old.epoch() != observed.epoch()
            || old.registry_hash != observed.registry_hash
            || old.native_slot > observed.native_slot
            || old.observed_ms > observed.observed_ms
        {
            return Err(JournalError::IntentConflict);
        }
        let (hash, anchor): (Vec<u8>, i64) = self.connection.query_row(
            "SELECT checkpoint_hash,last_operation FROM funding_inventory_anchor WHERE singleton=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let previous_bytes = serde_json::to_vec(&old).map_err(|_| JournalError::IntentConflict)?;
        if hash.as_slice() != Sha256::digest(previous_bytes).as_slice() || anchor < 0 {
            return Err(JournalError::CorruptState(
                "invalid funding inventory anchor",
            ));
        }
        let mut q = self.connection.prepare("SELECT o.operation_id,b.rates FROM operations o LEFT JOIN order_funding_barriers b ON b.operation_id=o.operation_id WHERE o.rowid>? AND o.filled_lots!=0")?;
        let rows = q.query_map(params![anchor], |r| {
            Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Option<Vec<u8>>>(1)?))
        })?;
        let required = serde_json::to_vec(old.rates()).map_err(|_| JournalError::IntentConflict)?;
        for row in rows {
            let (_, barrier) = row?;
            if barrier.as_deref() != Some(required.as_slice()) {
                return Err(JournalError::IntentConflict);
            }
        }
        // Funding folds/heartbeats do not change the inventory commitment.
        // Never bless an unexplained nonzero inventory change without fill facts.
        let changes: i64 = self.connection.query_row(
            "SELECT count(*) FROM operations WHERE rowid>?",
            params![anchor],
            |r| r.get(0),
        )?;
        if changes == 0 && old.inventory_hash != observed.inventory_hash {
            return Err(JournalError::IntentConflict);
        }
        drop(q);
        let mut rebased = observed.clone();
        rebased.rates = old.rates;
        rebased.bootstrap_safe = false;
        self.rebase_funding_inventory(&rebased)?;
        Ok(rebased)
    }

    /// Authenticated caller proves every joining user is flat, funding-free,
    /// nonce-zero and at this epoch, and the old users' inventory is unchanged.
    pub(crate) fn incorporate_flat_registry(
        &mut self,
        cp: &FundingCheckpoint,
        users: &BTreeMap<[u8; 32], [u8; 32]>,
    ) -> Result<(), JournalError> {
        if self.has_unresolved_maintenance()?
            || self.has_unresolved_funding()?
            || !self.nonterminal_operations()?.is_empty()
        {
            return Err(JournalError::IntentConflict);
        }
        let old = self
            .funding_checkpoint()?
            .ok_or(JournalError::IntentConflict)?;
        cp.validate().map_err(|_| JournalError::IntentConflict)?;
        if cp.epoch() != old.epoch()
            || cp.rates() != old.rates()
            || !cp.ownership_down
            || cp.registry_hash != crate::funding_maintenance::registry_hash(users)
            || self
                .known_users()?
                .iter()
                .any(|(l, u)| users.get(l) != Some(u))
        {
            return Err(JournalError::IntentConflict);
        }
        let tx = self.connection.transaction()?;
        for (l, u) in users {
            tx.execute(
                "INSERT OR IGNORE INTO users VALUES(?,?)",
                params![l.as_slice(), u.as_slice()],
            )?;
        }
        {
            let mut q = tx.prepare("SELECT user_ledger,user_pubkey FROM users")?;
            let actual = q
                .query_map([], |r| {
                    Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?))
                })?
                .map(|r| {
                    let (l, u) = r?;
                    Ok((super::fixed::<32>(&l)?, super::fixed::<32>(&u)?))
                })
                .collect::<Result<BTreeMap<_, _>, JournalError>>()?;
            if &actual != users {
                return Err(JournalError::IntentConflict);
            }
        }
        tx.execute(
            "INSERT INTO funding_registry_barriers VALUES(?,?,?)",
            params![
                old.registry_hash.as_slice(),
                cp.registry_hash.as_slice(),
                super::u64_blob(cp.epoch()).as_slice()
            ],
        )?;
        tx.execute(
            "UPDATE funding_checkpoint SET payload=? WHERE singleton=1",
            params![serde_json::to_vec(cp).map_err(|_| JournalError::IntentConflict)?],
        )?;
        Self::write_inventory_anchor(&tx, cp)?;
        tx.commit()?;
        Ok(())
    }

    pub(super) fn funding_registry_transition(
        &self,
        source: [u8; 32],
        target: [u8; 32],
        epoch: u64,
    ) -> Result<bool, JournalError> {
        let mut cursor = source;
        let mut seen = std::collections::BTreeSet::new();
        while cursor != target {
            if !seen.insert(cursor) || seen.len() > 1024 {
                return Ok(false);
            }
            let next: Option<Vec<u8>> = self
                .connection
                .query_row(
                    "SELECT target FROM funding_registry_barriers WHERE source=? AND epoch=?",
                    params![cursor.as_slice(), super::u64_blob(epoch).as_slice()],
                    |r| r.get(0),
                )
                .optional()?;
            let Some(next) = next else {
                return Ok(false);
            };
            cursor = next
                .try_into()
                .map_err(|_| JournalError::CorruptState("invalid funding registry barrier"))?;
        }
        Ok(true)
    }

    pub(super) fn validate_funding_barriers(&self) -> Result<(), JournalError> {
        let cp = self.funding_checkpoint()?;
        let anchor: Option<(Vec<u8>, i64)> = self
            .connection
            .query_row(
                "SELECT checkpoint_hash,last_operation FROM funding_inventory_anchor",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((hash, last)) = anchor {
            let cp = cp.as_ref().ok_or(JournalError::CorruptState(
                "anchor without funding checkpoint",
            ))?;
            let bytes = serde_json::to_vec(cp).map_err(|_| JournalError::IntentConflict)?;
            let max: i64 = self.connection.query_row(
                "SELECT coalesce(max(rowid),0) FROM operations",
                [],
                |r| r.get(0),
            )?;
            if hash.as_slice() != Sha256::digest(bytes).as_slice() || last < 0 || last > max {
                return Err(JournalError::CorruptState(
                    "invalid funding inventory anchor",
                ));
            }
        }
        let mut q = self
            .connection
            .prepare("SELECT rates FROM order_funding_barriers")?;
        for bytes in q.query_map([], |r| r.get::<_, Vec<u8>>(0))? {
            let bytes = bytes?;
            let rates: BTreeMap<u16, FundingRate> = serde_json::from_slice(&bytes)
                .map_err(|_| JournalError::CorruptState("invalid order funding barrier"))?;
            if rates.is_empty()
                || rates.contains_key(&0)
                || rates.len() > cinder_common::MAX_BOOK_MARKETS
                || rates
                    .values()
                    .any(|r| r.last_update_seconds.checked_mul(1000).is_none())
                || serde_json::to_vec(&rates).map_err(|_| JournalError::IntentConflict)? != bytes
            {
                return Err(JournalError::CorruptState("invalid order funding barrier"));
            }
        }
        let mut q = self
            .connection
            .prepare("SELECT source,target,epoch FROM funding_registry_barriers")?;
        for row in q.query_map([], |r| {
            Ok((
                r.get::<_, Vec<u8>>(0)?,
                r.get::<_, Vec<u8>>(1)?,
                r.get::<_, Vec<u8>>(2)?,
            ))
        })? {
            let (source, target, epoch) = row?;
            let source = super::fixed::<32>(&source)?;
            let target = super::fixed::<32>(&target)?;
            let epoch = u64::from_be_bytes(super::fixed::<8>(&epoch)?);
            if source == [0; 32]
                || target == [0; 32]
                || source == target
                || cp.as_ref().is_none_or(|c| epoch > c.epoch())
            {
                return Err(JournalError::CorruptState(
                    "invalid funding registry barrier",
                ));
            }
            let mut cursor = target;
            let mut seen = std::collections::BTreeSet::from([source]);
            loop {
                if cursor == [0; 32] || !seen.insert(cursor) || seen.len() > 1024 {
                    return Err(JournalError::CorruptState(
                        "cyclic funding registry barrier",
                    ));
                }
                let next: Option<Vec<u8>> = self
                    .connection
                    .query_row(
                        "SELECT target FROM funding_registry_barriers WHERE source=? AND epoch=?",
                        params![cursor.as_slice(), super::u64_blob(epoch).as_slice()],
                        |r| r.get(0),
                    )
                    .optional()?;
                let Some(next) = next else {
                    break;
                };
                cursor = super::fixed::<32>(&next)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/support/funding_barriers.rs"]
mod tests;
