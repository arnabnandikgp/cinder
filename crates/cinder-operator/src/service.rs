//! Opt-in, single-writer maintenance service. Feed threads only wake this
//! coordinator; they never sign, mutate the journal or write private state.
use crate::rpc::{unix_ms, Result, RuntimeError};
use crate::{FundingProgress, OperatorRuntime};
use cinder_common as cc;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaintenancePolicy {
    pub liquidation_slippage_bps: u16,
    pub liquidation_deadline_slots: u64,
    pub native_ws_env: String,
    pub fee_balance_index: u8,
    pub minimum_fee_balance_lamports: u64,
    pub minimum_magic_vault_lamports: u64,
}
impl MaintenancePolicy {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.liquidation_slippage_bps == 0
            || self.liquidation_slippage_bps >= 10_000
            || self.liquidation_deadline_slots == 0
            || self.liquidation_deadline_slots > 64
            || self.fee_balance_index == 255
            || self.minimum_fee_balance_lamports == 0
            || self.minimum_magic_vault_lamports == 0
            || self.native_ws_env.is_empty()
            || !self
                .native_ws_env
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
        {
            return Err(RuntimeError::Configuration);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaintenancePass {
    Recovering,
    Funding,
    Mutating,
    Healthy,
    Halted,
}

impl OperatorRuntime {
    /// A pass never overlaps a financial outbox. Signed uncertainty is observed
    /// first; every later financial mutation requires a new authoritative view.
    pub fn maintain(&mut self) -> Result<MaintenancePass> {
        let policy = self
            .context
            .borrow()
            .config
            .maintenance_policy
            .clone()
            .ok_or(RuntimeError::Configuration)?;
        policy.validate()?;
        // Keep a reconciled quiet session usable between mutations. Read-only
        // scans must not close/reopen both gates on every 50-ms wakeup.
        if self.coordinator.entries_enabled() && self.healthy_quiet_pass(&policy)? {
            return Ok(MaintenancePass::Healthy);
        }
        crate::ledger::set_down(&mut self.context.borrow_mut(), true)?;
        if self.drain_maintenance_write()? {
            return Ok(MaintenancePass::Mutating);
        }
        self.refresh_registry()?;
        // Read funding generations before enabling dispatch. The execution
        // port's atomic fences also prevent a tick inside a native transaction.
        if !self
            .journal()
            .nonterminal_operations()
            .map_err(|_| RuntimeError::Journal)?
            .is_empty()
            || self
                .journal()
                .has_unresolved_funding()
                .map_err(|_| RuntimeError::Journal)?
        {
            self.recover()?;
            return Ok(MaintenancePass::Recovering);
        }
        // A user can place while the quiet session is open. Discover its exact
        // private receipt before attempting a confirmed-inventory checkpoint;
        // an intent need not have reached the SQLite journal yet.
        if self
            .context
            .borrow_mut()
            .private_ledgers()?
            .1
            .iter()
            .any(|(_, l)| l.pending_oid_count != 0)
        {
            self.recover()?;
            return Ok(MaintenancePass::Recovering);
        }
        match self.accrue_funding()? {
            FundingProgress::Current => {}
            _ => return Ok(MaintenancePass::Funding),
        }
        let (book, users, view) = self
            .maintenance_snapshot_cache
            .take()
            .ok_or(RuntimeError::Incomplete)?;
        if users.iter().any(|(_, l)| {
            l.positions[..l.positions_len as usize]
                .iter()
                .any(|p| p.unsettled_funding != 0)
        }) && self.fold_funding()?
        {
            return Ok(MaintenancePass::Mutating);
        }
        let health = crate::maintenance::scan_user_health(&view, &users, unix_ms())?;
        // Debt/administrative flags are sticky. More restrictive side wins;
        // automatic recovery never clears a flag owned by another actor.
        let flags = book.halt
            | view.halt
            | if view.risk_tier >= 2 {
                cc::UNSAFE_POOL | cc::HALT_ENTRIES
            } else {
                0
            }
            | if users.iter().any(|(_, l)| l.bad_debt_usdc != 0) {
                cc::BAD_DEBT | cc::HALT_ENTRIES
            } else {
                0
            };
        if self.mirror_halts(flags)? {
            return Ok(MaintenancePass::Mutating);
        }
        if self.initiate_liquidation(&policy, &book, &users, &view, &health)? {
            return Ok(MaintenancePass::Mutating);
        }
        if self.repair_collateral(&book, &users, &view)? {
            return Ok(MaintenancePass::Mutating);
        }
        if book.phoenix_collateral != view.collateral && self.sync_collateral()? {
            return Ok(MaintenancePass::Mutating);
        }
        // Publication wins over another healthy heartbeat when already due.
        let debt = users.iter().try_fold(0u64, |n, (_, l)| {
            n.checked_add(l.bad_debt_usdc).ok_or(RuntimeError::Decode)
        })?;
        if (self
            .journal()
            .reserve_due(unix_ms())
            .map_err(|_| RuntimeError::Journal)?
            || self
                .journal()
                .reserve_debt_changed(debt)
                .map_err(|_| RuntimeError::Journal)?)
            && self.publish_reserve()?
        {
            return Ok(MaintenancePass::Mutating);
        }
        self.monitor_per_fees(&policy)?;
        if self.heartbeat_from(&book, &users, &view)? {
            return Ok(MaintenancePass::Mutating);
        }
        // Startup/recovery reconciles the full registry, I1/I2, all halts and
        // configured solvency again. A completed timer never opens entries.
        let report = self.recover()?;
        Ok(if report.reason.is_none() {
            MaintenancePass::Healthy
        } else {
            MaintenancePass::Halted
        })
    }
    fn healthy_quiet_pass(&mut self, policy: &MaintenancePolicy) -> Result<bool> {
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
            return Ok(false);
        }
        self.refresh_registry()?;
        let (book, users, view) = self.maintenance_snapshot()?;
        let now = unix_ms();
        let Some(cp) = self
            .journal()
            .funding_checkpoint()
            .map_err(|_| RuntimeError::Journal)?
        else {
            return Ok(false);
        };
        if !view.safe
            || (book.halt | view.halt) != 0
            || book.phoenix_collateral != view.collateral
            || book.funding_epoch != cp.epoch()
            || cp.rates() != &view.funding_rates()?
            || users
                .iter()
                .any(|(_, l)| l.pending_oid_count != 0 || l.last_funding_epoch != cp.epoch())
            || now
                .checked_sub(book.last_scan_ms)
                .is_none_or(|age| age >= cc::HEARTBEAT_MS)
            || users.iter().any(|(_, l)| {
                l.positions[..l.positions_len as usize]
                    .iter()
                    .any(|p| p.unsettled_funding != 0)
            }) && view.funding == 0
                && view.asset_funding.values().all(|n| *n == 0)
            || crate::maintenance_runtime::collateral_shortfall(&view)? != 0
            || self
                .journal()
                .reserve_due(now)
                .map_err(|_| RuntimeError::Journal)?
        {
            return Ok(false);
        }
        if crate::maintenance::scan_user_health(&view, &users, now)?
            .iter()
            .any(|h| h.below_mm())
        {
            return Ok(false);
        }
        let solvency = crate::solvency::evaluate(
            &view,
            &users,
            self.context.borrow().config.solvency_policy.as_ref(),
            now,
        )?;
        if !solvency.configured_checks_pass() {
            return Ok(false);
        }
        crate::admission::accounting(&view, &book, &users)?;
        self.monitor_per_fees(policy)?;
        // Fee RPC latency cannot extend the financial snapshot's lifetime.
        if [view.observed_ms, view.mark_ms].into_iter().any(|t| {
            unix_ms()
                .checked_sub(t)
                .is_none_or(|age| age > cc::MARK_STALE_MS)
        }) {
            return Err(RuntimeError::Stale);
        }
        Ok(true)
    }
    fn refresh_registry(&mut self) -> Result<()> {
        let expected = self.context.borrow_mut().registry()?;
        let known = self.context.borrow().users.clone();
        if expected == known {
            return Ok(());
        }
        if known.iter().any(|(a, u)| expected.get(a) != Some(u)) {
            return Err(RuntimeError::Incomplete);
        }
        self.context.borrow_mut().users = expected;
        if let Err(e) = self.incorporate_registry(&known) {
            self.context.borrow_mut().users = known;
            return Err(e);
        }
        Ok(())
    }
    pub fn run(&mut self, stop: &std::sync::atomic::AtomicBool) -> Result<()> {
        use std::sync::atomic::Ordering;
        let policy = self
            .context
            .borrow()
            .config
            .maintenance_policy
            .clone()
            .ok_or(RuntimeError::Configuration)?;
        policy.validate()?;
        let keys = {
            let c = self.context.borrow();
            std::iter::once(c.config.phoenix_asset_map.clone())
                .chain(std::iter::once(c.config.phoenix_trader.clone()))
                .chain(c.config.global_trader_index.clone())
                .chain(c.config.active_trader_buffer.clone())
                .collect()
        };
        let feed = crate::feeds::NativeFeed::start(&policy.native_ws_env, keys)?;
        let result = (|| {
            let mut reported = None;
            // Fee readiness is a session prerequisite, not a reason to stop
            // acknowledging already-confirmed financial outcomes.
            self.halt()?;
            while !stop.load(Ordering::Relaxed) {
                let started = std::time::Instant::now();
                if let Err(error) = self.maintain() {
                    self.halt()?;
                    // OPERATOR_DOWN already blocks entry on both programs.
                    // Do not turn transient RPC/freshness failures into an
                    // administrative HALT_ENTRIES bit we cannot safely clear.
                    self.mirror_halts(cc::OPERATOR_DOWN)?;
                    // Redacted diagnostics only; no RPC URLs/account bodies.
                    if reported != Some(error) {
                        eprintln!("Maintenance paused: {error}");
                        reported = Some(error);
                    }
                } else {
                    reported = None;
                }
                // Best-effort 50 ms target, never an RPC latency guarantee.
                // WS notifications coalesce; authoritative polling is replay
                // fallback and also covers missed hot/cold transitions.
                feed.wait(
                    stop,
                    std::time::Duration::from_millis(cc::SCAN_INTERVAL_MS)
                        .saturating_sub(started.elapsed()),
                );
            }
            Ok(())
        })();
        let halted = self.halt();
        drop(feed);
        result.and(halted)
    }
}
