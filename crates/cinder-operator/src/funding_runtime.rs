//! Serialized authenticated funding accrual I/O. Each call sends at most one
//! new write, and restarts observe persisted identities before signing again.
use crate::funding_maintenance::FundingCheckpoint;
use crate::rpc::{unix_ms, Result, RuntimeError};
use crate::runtime::{book_key, config_key, ledger_id, text_signature};
use crate::transaction::{anchor_ix, send, sign};
use crate::{FundingEpochPlan, FundingStepObservation, MaintenanceOutcome, OperatorRuntime};
use cinder_common as cc;
use sha2::{Digest, Sha256};
use solana_signer::Signer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FundingProgress {
    Current,
    WaitingForOrders,
    EpochPending { epoch: u64 },
    EpochCompleted { epoch: u64 },
}

impl OperatorRuntime {
    pub(crate) fn incorporate_registry(
        &mut self,
        old_users: &std::collections::BTreeMap<[u8; 32], [u8; 32]>,
    ) -> Result<()> {
        let Some(old) = self
            .journal()
            .funding_checkpoint()
            .map_err(|_| RuntimeError::Journal)?
        else {
            let users = self
                .context
                .borrow()
                .users
                .iter()
                .map(|(a, u)| (*a, *u))
                .collect::<Vec<_>>();
            self.coordinator
                .maintenance_journal()
                .remember_users(&users)
                .map_err(|_| RuntimeError::Journal)?;
            return Ok(());
        };
        if crate::funding_maintenance::registry_hash(&self.context.borrow().users)
            == old.registry_hash
        {
            return Ok(());
        }
        if crate::funding_maintenance::registry_hash(old_users) != old.registry_hash {
            return Err(RuntimeError::FundingAllocationRequired);
        }
        crate::ledger::set_down(&mut self.context.borrow_mut(), true)?;
        let (book, users, view) = self.maintenance_snapshot()?;
        let prior = users
            .iter()
            .filter(|(a, _)| old_users.contains_key(a))
            .map(|(a, l)| (*a, l.clone()))
            .collect::<Vec<_>>();
        let prior = FundingCheckpoint::capture(&view, &book, &prior, unix_ms())?;
        if prior.inventory_hash != old.inventory_hash || prior.registry_hash != old.registry_hash {
            return Err(RuntimeError::FundingAllocationRequired);
        }
        for (_, l) in users.iter().filter(|(a, _)| !old_users.contains_key(a)) {
            if l.nonce != 0
                || l.bad_debt_usdc != 0
                || l.pending_oid_count != 0
                || l.withdrawable != 0
                || l.last_funding_epoch != old.epoch()
                || l.positions[..l.positions_len as usize]
                    .iter()
                    .any(|p| p.lots != 0 || p.entry_quote_lots != 0 || p.unsettled_funding != 0)
            {
                return Err(RuntimeError::FundingAllocationRequired);
            }
        }
        let mut cp = FundingCheckpoint::capture(&view, &book, &users, unix_ms())?;
        cp.rates = old.rates.clone();
        cp.bootstrap_safe = false;
        let registry = self.context.borrow().users.clone();
        self.coordinator
            .maintenance_journal()
            .incorporate_flat_registry(&cp, &registry)
            .map_err(|_| RuntimeError::FundingAllocationRequired)?;
        self.context.borrow_mut().funding_checkpoint = Some(cp);
        Ok(())
    }
    pub(crate) fn refresh_funding_inventory(
        &mut self,
    ) -> Result<Option<crate::runtime::MaintenanceSnapshot>> {
        let cp = self
            .journal()
            .funding_checkpoint()
            .map_err(|_| RuntimeError::Journal)?;
        self.context.borrow_mut().funding_checkpoint = cp.clone();
        let Some(cp) = cp else {
            return Ok(None);
        };
        if self
            .journal()
            .has_unresolved_maintenance()
            .map_err(|_| RuntimeError::Journal)?
            || self
                .journal()
                .has_unresolved_funding()
                .map_err(|_| RuntimeError::Journal)?
            || !self
                .journal()
                .nonterminal_operations()
                .map_err(|_| RuntimeError::Journal)?
                .is_empty()
        {
            return Ok(None);
        }
        let (_, users, _) = self.context.borrow_mut().private_ledgers()?;
        if users.iter().any(|(_, l)| l.pending_oid_count != 0) {
            return Ok(None);
        }
        let (book, users, view) = self.maintenance_snapshot()?;
        // Discovery may not have journaled a newly placed private intent yet.
        // Its tentative inventory is not a funding checkpoint; recover the
        // exact placement and outcome before attempting any rebase.
        if users.iter().any(|(_, l)| l.pending_oid_count != 0) {
            return Ok(None);
        }
        let observed = FundingCheckpoint::capture(&view, &book, &users, unix_ms())?;
        if observed.registry_hash != cp.registry_hash {
            return Err(RuntimeError::FundingAllocationRequired);
        }
        if observed.inventory_hash == cp.inventory_hash {
            self.coordinator
                .maintenance_journal()
                .ensure_inventory_anchor(&cp)
                .map_err(|_| RuntimeError::FundingAllocationRequired)?;
        }
        if observed.inventory_hash != cp.inventory_hash {
            let rebased = self
                .coordinator
                .maintenance_journal()
                .rebase_acknowledged_inventory(&observed)
                .map_err(|_| RuntimeError::FundingAllocationRequired)?;
            self.context.borrow_mut().funding_checkpoint = Some(rebased);
        }
        Ok(Some((book, users, view)))
    }
    /// Accrue observed native updates on unchanged confirmed private inventory.
    /// Does not fold unsettled funding, release gates, or authorize venue orders.
    pub fn accrue_funding(&mut self) -> Result<FundingProgress> {
        self.maintenance_snapshot_cache = None;
        crate::ledger::set_down(&mut self.context.borrow_mut(), true)?;
        if self.drain_maintenance_write()? {
            return Ok(FundingProgress::WaitingForOrders);
        }
        let cached = self.refresh_funding_inventory()?;
        if self
            .journal()
            .has_unresolved_funding()
            .map_err(|_| RuntimeError::Journal)?
            || !self
                .journal()
                .nonterminal_operations()
                .map_err(|_| RuntimeError::Journal)?
                .is_empty()
        {
            return Ok(FundingProgress::WaitingForOrders);
        }
        let source = self
            .journal()
            .funding_checkpoint()
            .map_err(|_| RuntimeError::Journal)?;
        let (book, users, view) = match cached {
            Some(s) => s,
            None => self.maintenance_snapshot()?,
        };
        let now = unix_ms();
        if [view.observed_ms, view.mark_ms]
            .into_iter()
            .any(|t| t == 0 || now.checked_sub(t).is_none_or(|age| age > cc::MARK_STALE_MS))
            || book.halt & cc::OPERATOR_DOWN == 0
            || view.halt & cc::OPERATOR_DOWN == 0
        {
            return Err(RuntimeError::Stale);
        }
        let Some(source) = source else {
            let cp = FundingCheckpoint::capture(&view, &book, &users, now)?;
            self.coordinator
                .maintenance_journal()
                .initialize_funding_checkpoint(&cp)
                .map_err(|_| RuntimeError::FundingAllocationRequired)?;
            self.maintenance_snapshot_cache = Some((book, users, view));
            return Ok(FundingProgress::Current);
        };
        let next_epoch = source.epoch().checked_add(1).ok_or(RuntimeError::Decode)?;
        let existing = self
            .journal()
            .funding_epoch(next_epoch)
            .map_err(|_| RuntimeError::Journal)?;
        let plan = if let Some(record) = &existing {
            if record.completed {
                return Err(RuntimeError::Journal);
            }
            // The frozen checkpoint is not replaced by a later native update.
            for (asset, frozen) in record.target.rates() {
                let update = view
                    .funding_updates_seconds
                    .get(asset)
                    .ok_or(RuntimeError::Incomplete)?;
                let rate = view
                    .markets
                    .get(asset)
                    .ok_or(RuntimeError::Incomplete)?
                    .cumulative_funding_rate
                    .as_inner();
                if *update < frozen.last_update_seconds
                    || (*update == frozen.last_update_seconds
                        && rate != frozen.cumulative_quote_lots_per_base_lot)
                {
                    return Err(RuntimeError::Identity);
                }
            }
            FundingEpochPlan::resume(&record.previous, &record.target, &book, &users)?
        } else {
            let observed = FundingCheckpoint::capture(&view, &book, &users, now)?;
            if observed.inventory_hash != source.inventory_hash
                || observed.registry_hash != source.registry_hash
            {
                return Err(RuntimeError::FundingAllocationRequired);
            }
            if observed.rates() == source.rates() {
                self.maintenance_snapshot_cache = Some((book, users, view));
                return Ok(FundingProgress::Current);
            }
            let plan = FundingEpochPlan::build(&source, &observed, &book, &users, now)?;
            self.coordinator
                .maintenance_journal()
                .prepare_funding_epoch(&plan)
                .map_err(|_| RuntimeError::Journal)?;
            plan
        };
        let record = self
            .journal()
            .funding_epoch(plan.epoch())
            .map_err(|_| RuntimeError::Journal)?
            .ok_or(RuntimeError::Journal)?;
        // Book causality takes precedence over lexicographic user ordering.
        let selected = record
            .steps
            .iter()
            .find(|s| s.scope == book_key() && !applied(s))
            .or_else(|| record.steps.iter().find(|s| !applied(s)));
        if let Some(step) = selected {
            if let Some(attempt) = step.attempts.last() {
                if attempt.outcome == MaintenanceOutcome::Unknown {
                    let value = {
                        let mut c = self.context.borrow_mut();
                        let c = &mut *c;
                        c.qfs
                            .transaction(&text_signature(&attempt.signature), &c.signer, false)?
                    };
                    if !value.is_null() {
                        let adapter = self.context.borrow().signer.pubkey().to_bytes();
                        let observation = FundingStepObservation::decode(
                            &value,
                            &attempt.signature,
                            &adapter,
                            plan.epoch(),
                            step.scope,
                            step.instruction_hash,
                        )?;
                        self.coordinator
                            .maintenance_journal()
                            .observe_funding_epoch_step(&observation)
                            .map_err(|_| RuntimeError::Journal)?;
                    }
                    return Ok(FundingProgress::EpochPending {
                        epoch: plan.epoch(),
                    });
                }
            }
            // Recheck both ownership gates and the private inventory immediately
            // before signing. No cached snapshot grants write permission.
            let (before, before_users, native) = self.maintenance_snapshot()?;
            if native.halt & cc::OPERATOR_DOWN == 0
                || unix_ms()
                    .checked_sub(native.mark_ms)
                    .is_none_or(|age| age > cc::MARK_STALE_MS)
            {
                return Err(RuntimeError::Stale);
            }
            let current = native.funding_rates()?;
            for (asset, frozen) in plan.target.rates() {
                let current = current.get(asset).ok_or(RuntimeError::Incomplete)?;
                if current.last_update_seconds < frozen.last_update_seconds
                    || (current.last_update_seconds == frozen.last_update_seconds
                        && current != frozen)
                {
                    return Err(RuntimeError::Identity);
                }
            }
            let fresh_plan =
                FundingEpochPlan::resume(&plan.previous, &plan.target, &before, &before_users)?;
            let body = fresh_plan
                .instruction(&step.scope)
                .ok_or(RuntimeError::Identity)?
                .to_vec();
            verify_funding_body(&body, step.instruction_hash)?;
            let tx = {
                let mut c = self.context.borrow_mut();
                let c = &mut *c;
                sign(
                    &mut c.qfs,
                    &c.signer,
                    funding_instruction(c.signer.pubkey().to_bytes(), step.scope, body),
                )?
            };
            self.coordinator
                .maintenance_journal()
                .record_funding_epoch_attempt(plan.epoch(), step.scope, tx.signature, tx.expiry)
                .map_err(|_| RuntimeError::Journal)?;
            let mut c = self.context.borrow_mut();
            let c = &mut *c;
            // Errors deliberately preserve Unknown in the WAL. No retry send.
            send(&mut c.qfs, &c.signer, &tx)?;
            return Ok(FundingProgress::EpochPending {
                epoch: plan.epoch(),
            });
        }
        let (book, users, view) = self.maintenance_snapshot()?;
        let aligned = FundingCheckpoint::capture(&view, &book, &users, unix_ms())?;
        self.coordinator
            .maintenance_journal()
            .complete_funding_epoch(&aligned)
            .map_err(|_| RuntimeError::Journal)?;
        Ok(FundingProgress::EpochCompleted {
            epoch: plan.epoch(),
        })
    }
}

fn verify_funding_body(body: &[u8], expected: [u8; 32]) -> Result<()> {
    let actual: [u8; 32] = Sha256::digest(body).into();
    if actual != expected {
        return Err(RuntimeError::Identity);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebuilt_funding_body_must_match_persisted_identity() {
        let body = b"funding instruction";
        let expected = Sha256::digest(body).into();
        assert!(verify_funding_body(body, expected).is_ok());
        assert!(matches!(
            verify_funding_body(b"different instruction", expected),
            Err(RuntimeError::Identity)
        ));
    }
}
fn funding_instruction(
    adapter: [u8; 32],
    scope: [u8; 32],
    body: Vec<u8>,
) -> solana_instruction::Instruction {
    let mut accounts = vec![
        (adapter, true, false),
        (config_key(), false, false),
        (scope, false, true),
    ];
    if scope != book_key() {
        accounts.push((book_key(), false, true));
    }
    anchor_ix(ledger_id(), accounts, body)
}

fn applied(step: &crate::FundingEpochStep) -> bool {
    step.attempts
        .last()
        .is_some_and(|a| matches!(a.outcome, MaintenanceOutcome::Applied { .. }))
}

#[cfg(test)]
mod account_tests {
    use super::*;
    #[test]
    fn allocation_and_epoch_bump_match_anchor_writable_book_contract() {
        let bump = funding_instruction([1; 32], book_key(), vec![]);
        assert_eq!(bump.accounts.len(), 3);
        assert!(bump.accounts[2].is_writable);
        let allocation = funding_instruction([1; 32], [2; 32], vec![]);
        assert_eq!(allocation.accounts.len(), 4);
        assert!(allocation.accounts[2].is_writable);
        assert!(allocation.accounts[3].is_writable);
        assert_eq!(allocation.accounts[3].pubkey.to_bytes(), book_key());
    }
}
