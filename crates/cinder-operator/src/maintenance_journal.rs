//! Durable funding-rate causality. No allocation bodies or account bytes enter
//! SQLite; frozen rates and opaque instruction/inventory hashes suffice to
//! reconstruct immutable writes while inventory remains unchanged.
use super::{u64_blob, Journal, JournalError};
use crate::funding_maintenance::{registry_hash, validate_transition};
use crate::runtime::book_key;
use crate::{FundingCheckpoint, FundingEpochPlan, FundingStepObservation};
use rusqlite::{params, OptionalExtension};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
type RawEpoch = (Vec<u8>, Vec<u8>, Vec<u8>, bool);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaintenanceOutcome {
    Unknown,
    Applied { slot: u64 },
    Rejected { slot: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaintenanceAttempt {
    pub signature: [u8; 64],
    pub last_valid_block_height: u64,
    pub outcome: MaintenanceOutcome,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FundingEpochStep {
    pub scope: [u8; 32],
    pub instruction_hash: [u8; 32],
    pub attempts: Vec<MaintenanceAttempt>,
}
impl FundingEpochStep {
    fn applied(&self) -> bool {
        self.attempts
            .last()
            .is_some_and(|a| matches!(a.outcome, MaintenanceOutcome::Applied { .. }))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FundingEpochRecord {
    pub previous: FundingCheckpoint,
    pub target: FundingCheckpoint,
    pub steps: Vec<FundingEpochStep>,
    pub completed: bool,
}
fn encode(c: &FundingCheckpoint) -> Result<Vec<u8>, JournalError> {
    c.validate()
        .map_err(|_| JournalError::CorruptState("invalid funding checkpoint"))?;
    serde_json::to_vec(c).map_err(|_| JournalError::CorruptState("invalid funding checkpoint"))
}
fn decode(b: &[u8]) -> Result<FundingCheckpoint, JournalError> {
    if b.len() > 16384 {
        return Err(JournalError::CorruptState("oversized funding checkpoint"));
    }
    let c: FundingCheckpoint = serde_json::from_slice(b)
        .map_err(|_| JournalError::CorruptState("invalid funding checkpoint"))?;
    if encode(&c)? != b {
        return Err(JournalError::CorruptState(
            "noncanonical funding checkpoint",
        ));
    }
    Ok(c)
}
fn fixed<const N: usize>(b: Vec<u8>) -> Result<[u8; N], JournalError> {
    b.try_into()
        .map_err(|_| JournalError::CorruptState("invalid maintenance field"))
}
fn plan_hash(source: &[u8], target: &[u8], steps: &BTreeMap<[u8; 32], [u8; 32]>) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"cinder:funding-epoch:v1");
    h.update((source.len() as u64).to_le_bytes());
    h.update(source);
    h.update((target.len() as u64).to_le_bytes());
    h.update(target);
    for (scope, body) in steps {
        h.update(scope);
        h.update(body);
    }
    h.finalize().into()
}
impl Journal {
    fn maintenance_idle(&self) -> Result<(), JournalError> {
        if self.has_unresolved_funding()?
            || self.has_unresolved_maintenance()?
            || !self.nonterminal_operations()?.is_empty()
        {
            return Err(JournalError::IntentConflict);
        }
        Ok(())
    }
    fn checkpoint_registry(&self, c: &FundingCheckpoint) -> Result<(), JournalError> {
        c.validate().map_err(|_| JournalError::IntentConflict)?;
        let bound: bool =
            self.connection
                .query_row("SELECT EXISTS(SELECT 1 FROM runtime_binding)", [], |r| {
                    r.get(0)
                })?;
        let users = self.known_users()?.into_iter().collect();
        if !bound || !c.ownership_down || registry_hash(&users) != c.registry_hash {
            return Err(JournalError::IntentConflict);
        }
        Ok(())
    }
    pub fn funding_checkpoint(&self) -> Result<Option<FundingCheckpoint>, JournalError> {
        let b: Option<Vec<u8>> = self
            .connection
            .query_row(
                "SELECT payload FROM funding_checkpoint WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .optional()?;
        b.as_deref().map(decode).transpose()
    }
    /// Bootstrap only a verified never-traded flat book with zero funding and
    /// matching cash. Existing exposure/history needs explicit reconstruction.
    pub fn initialize_funding_checkpoint(
        &mut self,
        c: &FundingCheckpoint,
    ) -> Result<(), JournalError> {
        self.maintenance_idle()?;
        self.checkpoint_registry(c)?;
        if !c.bootstrap_safe
            || self.connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM operations WHERE filled_lots!=0)",
                [],
                |r| r.get::<_, bool>(0),
            )?
        {
            return Err(JournalError::IntentConflict);
        }
        if let Some(old) = self.funding_checkpoint()? {
            return if old == *c {
                Ok(())
            } else {
                Err(JournalError::IntentConflict)
            };
        }
        let tx = self.connection.transaction()?;
        tx.execute(
            "INSERT INTO funding_checkpoint VALUES(1,?)",
            params![encode(c)?],
        )?;
        Self::write_inventory_anchor(&tx, c)?;
        tx.commit()?;
        Ok(())
    }
    /// After an authenticated inventory-changing ACK, rebind only if BOTH rate
    /// and update generation are unchanged. Does not forgive a funding gap.
    pub fn rebase_funding_inventory(&mut self, c: &FundingCheckpoint) -> Result<(), JournalError> {
        self.maintenance_idle()?;
        self.checkpoint_registry(c)?;
        let old = self
            .funding_checkpoint()?
            .ok_or(JournalError::IntentConflict)?;
        if old.epoch != c.epoch
            || old.rates != c.rates
            || old.registry_hash != c.registry_hash
            || c.native_slot < old.native_slot
            || c.observed_ms < old.observed_ms
        {
            return Err(JournalError::IntentConflict);
        }
        let tx = self.connection.transaction()?;
        tx.execute(
            "UPDATE funding_checkpoint SET payload=? WHERE singleton=1",
            params![encode(c)?],
        )?;
        Self::write_inventory_anchor(&tx, c)?;
        tx.commit()?;
        Ok(())
    }
    pub fn has_unresolved_maintenance(&self) -> Result<bool, JournalError> {
        Ok(self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM funding_epochs WHERE completed=0) OR EXISTS(SELECT 1 FROM maintenance_writes WHERE state IN (0,1))",
            [],
            |r| r.get(0),
        )?)
    }
    /// Immutable epoch/rate/write hashes commit atomically BEFORE signing.
    /// Runtime callers must freshly confirm both ownership gates first.
    pub fn prepare_funding_epoch(
        &mut self,
        plan: &FundingEpochPlan,
    ) -> Result<FundingEpochRecord, JournalError> {
        validate_transition(&plan.previous, &plan.target)
            .map_err(|_| JournalError::IntentConflict)?;
        self.checkpoint_registry(&plan.target)?;
        let source = encode(&plan.previous)?;
        let target = encode(&plan.target)?;
        let hashes = plan
            .steps
            .iter()
            .map(|(s, b)| (*s, <[u8; 32]>::from(Sha256::digest(b))))
            .collect::<BTreeMap<_, _>>();
        let hash = plan_hash(&source, &target, &hashes);
        if let Some(old) = self.funding_epoch(plan.epoch())? {
            let old_hash = plan_hash(
                &encode(&old.previous)?,
                &encode(&old.target)?,
                &old.steps
                    .iter()
                    .map(|s| (s.scope, s.instruction_hash))
                    .collect(),
            );
            return if old_hash == hash {
                Ok(old)
            } else {
                Err(JournalError::IntentConflict)
            };
        }
        self.maintenance_idle()?;
        if self.funding_checkpoint()?.as_ref() != Some(&plan.previous) {
            return Err(JournalError::IntentConflict);
        }
        let expected = self
            .known_users()?
            .into_iter()
            .map(|(l, _)| l)
            .chain([book_key()])
            .collect::<std::collections::BTreeSet<_>>();
        if hashes
            .keys()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            != expected
        {
            return Err(JournalError::IntentConflict);
        }
        let epoch = u64_blob(plan.epoch());
        let tx = self.connection.transaction()?;
        tx.execute(
            "INSERT INTO funding_epochs(epoch,source,target,plan_hash) VALUES(?,?,?,?)",
            params![epoch.as_slice(), source, target, hash.as_slice()],
        )?;
        for (scope, body) in hashes {
            tx.execute(
                "INSERT INTO funding_epoch_steps VALUES(?,?,?)",
                params![epoch.as_slice(), scope.as_slice(), body.as_slice()],
            )?;
        }
        tx.commit()?;
        self.funding_epoch(plan.epoch())?
            .ok_or(JournalError::CorruptState("missing funding epoch"))
    }
    pub fn funding_epoch(&self, epoch: u64) -> Result<Option<FundingEpochRecord>, JournalError> {
        let raw: Option<RawEpoch> = self
            .connection
            .query_row(
                "SELECT source,target,plan_hash,completed FROM funding_epochs WHERE epoch=?",
                params![u64_blob(epoch).as_slice()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        let Some((source, target, hash, completed)) = raw else {
            return Ok(None);
        };
        let previous = decode(&source)?;
        let next = decode(&target)?;
        validate_transition(&previous, &next)
            .map_err(|_| JournalError::CorruptState("invalid funding epoch transition"))?;
        if next.epoch != epoch {
            return Err(JournalError::CorruptState("wrong funding epoch"));
        }
        let mut stmt = self.connection.prepare(
            "SELECT scope,body_hash FROM funding_epoch_steps WHERE epoch=? ORDER BY scope",
        )?;
        let rows = stmt.query_map(params![u64_blob(epoch).as_slice()], |r| {
            Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?))
        })?;
        let mut steps = Vec::new();
        let mut hashes = BTreeMap::new();
        for row in rows {
            let (scope, body) = row?;
            let scope = fixed::<32>(scope)?;
            let body = fixed::<32>(body)?;
            hashes.insert(scope, body);
            let mut attempts=self.connection.prepare("SELECT signature,expiry,outcome,slot FROM funding_epoch_attempts WHERE epoch=? AND scope=? ORDER BY sequence")?;
            let mut decoded = Vec::<MaintenanceAttempt>::new();
            for a in
                attempts.query_map(params![u64_blob(epoch).as_slice(), scope.as_slice()], |r| {
                    Ok((
                        r.get::<_, Vec<u8>>(0)?,
                        r.get::<_, Vec<u8>>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, Option<Vec<u8>>>(3)?,
                    ))
                })?
            {
                let (sig, expiry, outcome, slot) = a?;
                if decoded
                    .last()
                    .is_some_and(|a| !matches!(a.outcome, MaintenanceOutcome::Rejected { .. }))
                {
                    return Err(JournalError::CorruptState(
                        "maintenance resend without rejection",
                    ));
                }
                let slot = slot
                    .map(|b| fixed::<8>(b).map(u64::from_be_bytes))
                    .transpose()?;
                let outcome = match (outcome, slot) {
                    (0, None) => MaintenanceOutcome::Unknown,
                    (1, Some(s)) if s > 0 => MaintenanceOutcome::Applied { slot: s },
                    (2, Some(s)) if s > 0 => MaintenanceOutcome::Rejected { slot: s },
                    _ => return Err(JournalError::CorruptState("invalid maintenance finality")),
                };
                let a = MaintenanceAttempt {
                    signature: fixed(sig)?,
                    last_valid_block_height: u64::from_be_bytes(fixed(expiry)?),
                    outcome,
                };
                if a.signature == [0; 64] || a.last_valid_block_height == 0 {
                    return Err(JournalError::CorruptState("invalid maintenance attempt"));
                }
                decoded.push(a);
            }
            steps.push(FundingEpochStep {
                scope,
                instruction_hash: body,
                attempts: decoded,
            });
        }
        if steps.is_empty()
            || fixed::<32>(hash)? != plan_hash(&source, &target, &hashes)
            || (completed && steps.iter().any(|s| !s.applied()))
        {
            return Err(JournalError::CorruptState("invalid maintenance plan"));
        }
        Ok(Some(FundingEpochRecord {
            previous,
            target: next,
            steps,
            completed,
        }))
    }
    /// Persist exact ER identity. Unknown/expired sends never authorize signing
    /// another attempt. The Book epoch bump must apply before any user write.
    pub fn record_funding_epoch_attempt(
        &mut self,
        epoch: u64,
        scope: [u8; 32],
        signature: [u8; 64],
        expiry: u64,
    ) -> Result<(), JournalError> {
        let r = self
            .funding_epoch(epoch)?
            .ok_or(JournalError::IntentConflict)?;
        if r.completed || signature == [0; 64] || expiry == 0 {
            return Err(JournalError::IntentConflict);
        }
        let step = r
            .steps
            .iter()
            .find(|s| s.scope == scope)
            .ok_or(JournalError::IntentConflict)?;
        if let Some(a) = step.attempts.last() {
            if a.signature == signature && a.last_valid_block_height == expiry {
                return Ok(());
            }
            if !matches!(a.outcome, MaintenanceOutcome::Rejected { .. }) {
                return Err(JournalError::IntentConflict);
            }
        }
        if scope != book_key() && !r.steps.iter().any(|s| s.scope == book_key() && s.applied()) {
            return Err(JournalError::IntentConflict);
        }
        let unknown: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM funding_epoch_attempts WHERE outcome=0)",
            [],
            |r| r.get(0),
        )?;
        if unknown {
            return Err(JournalError::IntentConflict);
        }
        self.connection.execute(
            "INSERT INTO funding_epoch_attempts(signature,epoch,scope,expiry) VALUES(?,?,?,?)",
            params![
                signature.as_slice(),
                u64_blob(epoch).as_slice(),
                scope.as_slice(),
                u64_blob(expiry).as_slice()
            ],
        )?;
        Ok(())
    }
    pub fn observe_funding_epoch_step(
        &mut self,
        observation: &FundingStepObservation,
    ) -> Result<(), JournalError> {
        let r = self
            .funding_epoch(observation.epoch)?
            .ok_or(JournalError::IntentConflict)?;
        let step = r
            .steps
            .iter()
            .find(|s| s.scope == observation.scope && s.instruction_hash == observation.body_hash)
            .ok_or(JournalError::IntentConflict)?;
        let a = step
            .attempts
            .iter()
            .find(|a| a.signature == observation.signature)
            .ok_or(JournalError::IntentConflict)?;
        let outcome = if observation.succeeded {
            MaintenanceOutcome::Applied {
                slot: observation.slot,
            }
        } else {
            MaintenanceOutcome::Rejected {
                slot: observation.slot,
            }
        };
        if a.outcome == outcome {
            return Ok(());
        }
        if r.completed || a.outcome != MaintenanceOutcome::Unknown || observation.slot == 0 {
            return Err(JournalError::IntentConflict);
        }
        self.connection.execute(
            "UPDATE funding_epoch_attempts SET outcome=?,slot=? WHERE signature=? AND outcome=0",
            params![
                if observation.succeeded { 1 } else { 2 },
                u64_blob(observation.slot).as_slice(),
                observation.signature.as_slice()
            ],
        )?;
        Ok(())
    }
    /// Receipts prove every economic write. A fresh coherent private snapshot
    /// must additionally attest aligned epochs and the unchanged frozen inventory.
    /// Advancing this checkpoint does not reconcile I2 or release entry gates.
    pub fn complete_funding_epoch(
        &mut self,
        observed: &FundingCheckpoint,
    ) -> Result<(), JournalError> {
        self.checkpoint_registry(observed)?;
        let r = self
            .funding_epoch(observed.epoch)?
            .ok_or(JournalError::IntentConflict)?;
        if r.completed {
            return if self.funding_checkpoint()?.as_ref() == Some(&r.target) {
                Ok(())
            } else {
                Err(JournalError::IntentConflict)
            };
        }
        if self.funding_checkpoint()?.as_ref() != Some(&r.previous)
            || r.steps.iter().any(|s| !s.applied())
            || observed.inventory_hash != r.target.inventory_hash
            || observed.registry_hash != r.target.registry_hash
            || observed.native_slot < r.target.native_slot
            || observed.observed_ms < r.target.observed_ms
            || observed.rates.keys().ne(r.target.rates.keys())
            || r.target.rates.iter().any(|(asset, rate)| {
                observed.rates.get(asset).is_none_or(|current| {
                    current.last_update_seconds < rate.last_update_seconds
                        || (current.last_update_seconds == rate.last_update_seconds
                            && current != rate)
                })
            })
        {
            return Err(JournalError::IntentConflict);
        }
        let tx = self.connection.transaction()?;
        tx.execute(
            "UPDATE funding_checkpoint SET payload=? WHERE singleton=1",
            params![encode(&r.target)?],
        )?;
        tx.execute(
            "UPDATE funding_epochs SET completed=1 WHERE epoch=?",
            params![u64_blob(observed.epoch).as_slice()],
        )?;
        Self::write_inventory_anchor(&tx, &r.target)?;
        tx.commit()?;
        Ok(())
    }
    pub(super) fn validate_maintenance_journal(&self) -> Result<(), JournalError> {
        let cp = self.funding_checkpoint()?;
        if let Some(c) = &cp {
            let bound: bool = self.connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM runtime_binding)",
                [],
                |r| r.get(0),
            )?;
            if !bound || !c.ownership_down {
                return Err(JournalError::CorruptState("unbound funding checkpoint"));
            }
        }
        let mut stmt = self
            .connection
            .prepare("SELECT epoch FROM funding_epochs ORDER BY epoch")?;
        let mut active = None;
        let mut last_completed: Option<FundingCheckpoint> = None;
        for b in stmt.query_map([], |r| r.get::<_, Vec<u8>>(0))? {
            let epoch = u64::from_be_bytes(fixed::<8>(b?)?);
            let r = self
                .funding_epoch(epoch)?
                .ok_or(JournalError::CorruptState("missing maintenance epoch"))?;
            if let Some(last) = &last_completed {
                if last.epoch != r.previous.epoch
                    || last.rates != r.previous.rates
                    || !self.funding_registry_transition(
                        last.registry_hash,
                        r.previous.registry_hash,
                        last.epoch,
                    )?
                    || last.native_slot > r.previous.native_slot
                    || last.observed_ms > r.previous.observed_ms
                {
                    return Err(JournalError::CorruptState("broken funding epoch history"));
                }
            }
            if r.completed {
                if active.is_some() {
                    return Err(JournalError::CorruptState(
                        "completion after active funding epoch",
                    ));
                }
                last_completed = Some(r.target.clone());
            }
            if !r.completed {
                if active.is_some() || cp.as_ref() != Some(&r.previous) {
                    return Err(JournalError::CorruptState(
                        "invalid active funding checkpoint",
                    ));
                }
                active = Some(r);
            }
        }
        if let Some(last) = &last_completed {
            let registry_valid = cp
                .as_ref()
                .map(|c| {
                    self.funding_registry_transition(
                        last.registry_hash,
                        c.registry_hash,
                        last.epoch,
                    )
                })
                .transpose()?
                .unwrap_or(false);
            if cp.as_ref().is_none_or(|c| {
                c.epoch != last.epoch
                    || c.rates != last.rates
                    || !registry_valid
                    || c.native_slot < last.native_slot
                    || c.observed_ms < last.observed_ms
            }) {
                return Err(JournalError::CorruptState(
                    "funding checkpoint contradicts history",
                ));
            }
        }
        if let Some(r) = active {
            if self.has_unresolved_funding()? || !self.nonterminal_operations()?.is_empty() {
                return Err(JournalError::CorruptState(
                    "maintenance overlaps unresolved orders or custody",
                ));
            }
            self.checkpoint_registry(&r.target)
                .map_err(|_| JournalError::CorruptState("maintenance registry mismatch"))?;
            let expected = self
                .known_users()?
                .into_iter()
                .map(|(l, _)| l)
                .chain([book_key()])
                .collect::<std::collections::BTreeSet<_>>();
            if r.steps
                .iter()
                .map(|s| s.scope)
                .collect::<std::collections::BTreeSet<_>>()
                != expected
            {
                return Err(JournalError::CorruptState(
                    "incomplete maintenance registry",
                ));
            }
            let bump = r
                .steps
                .iter()
                .find(|s| s.scope == book_key())
                .ok_or(JournalError::CorruptState("missing funding bump"))?;
            if !bump.applied()
                && r.steps
                    .iter()
                    .any(|s| s.scope != book_key() && !s.attempts.is_empty())
            {
                return Err(JournalError::CorruptState(
                    "funding allocation precedes bump",
                ));
            }
        }
        let unknown: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM funding_epoch_attempts WHERE outcome=0",
            [],
            |r| r.get(0),
        )?;
        if unknown > 1 {
            return Err(JournalError::CorruptState(
                "concurrent unknown maintenance writes",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/support/maintenance_journal.rs"]
mod tests;
