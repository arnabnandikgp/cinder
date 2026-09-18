//! Single-mutation production maintenance passes. The outbox always drains
//! before preparing another private or public write.
use crate::journal::maintenance_writes::{
    MaintenanceParameters as P, MaintenanceWrite, WriteState,
};
use crate::rpc::{unix_ms, Result, RuntimeError};
use crate::runtime::{
    book_key, config_key, decode, ledger_id, pda, text, text_signature, vault_id,
};
use crate::transaction::{anchor_ix, send, sign};
use crate::{LedgerRecoveryPort, OperatorRuntime};
use anchor_lang::InstructionData;
use cinder_common as cc;
use cinder_ledger::{Book, FundingEntry, UserLedger};
use sha2::{Digest, Sha256};
use solana_signer::Signer;

fn private_hash(book: &Book, users: &[([u8; 32], UserLedger)]) -> Result<[u8; 32]> {
    let mut h = Sha256::new();
    h.update(b"cinder:maintenance-private:v1");
    h.update(crate::ledger::book_fingerprint(book)?);
    let mut rows = users.iter().collect::<Vec<_>>();
    rows.sort_by_key(|(a, _)| *a);
    for (a, l) in rows {
        h.update(a);
        h.update(crate::ledger::private_fingerprint(l)?);
    }
    Ok(h.finalize().into())
}
pub(crate) fn collateral_shortfall(view: &crate::rise::RiseView) -> Result<u64> {
    use phoenix_rise_math::{self as math, LimitOrderMarginState, SignedBaseLots, TraderPosition};
    let mut im = 0u64;
    for (asset, m) in &view.markets {
        let mut p = TraderPosition::new();
        p.base_lot_position = SignedBaseLots::new(*view.positions.get(asset).unwrap_or(&0));
        im = im
            .checked_add(
                math::initial_margin_for_asset(
                    m,
                    &p,
                    &LimitOrderMarginState::empty(),
                    math::risk::RiskAction::View,
                )
                .map_err(|_| RuntimeError::Decode)?
                .as_inner(),
            )
            .ok_or(RuntimeError::Decode)?;
    }
    Ok(crate::CollateralRequirement::for_hedge(im, 0)?.shortfall_usdc(view.collateral))
}
impl OperatorRuntime {
    /// Observe the sole persisted transaction before any later mutation. An
    /// unsigned abandoned plan can be cancelled; signed uncertainty cannot.
    pub(crate) fn drain_maintenance_write(&mut self) -> Result<bool> {
        let Some(w) = self
            .journal()
            .active_maintenance_write()
            .map_err(|_| RuntimeError::Journal)?
        else {
            return Ok(false);
        };
        if w.state == WriteState::Prepared {
            // A crash before signing made no chain side effect. Replanning
            // refreshes marks/margins instead of inventing old private bodies.
            self.coordinator
                .maintenance_journal()
                .finish_maintenance_write(w.id, WriteState::Cancelled)
                .map_err(|_| RuntimeError::Journal)?;
            return Ok(true);
        }
        let finality = {
            let mut c = self.context.borrow_mut();
            let c = &mut *c;
            let sig = w.signature.ok_or(RuntimeError::Journal)?;
            let public = matches!(w.parameters, P::Root { .. } | P::PostCollateral { .. });
            if let P::PostCollateral {
                nonce,
                amount,
                trader,
                program,
                ..
            } = w.parameters
            {
                let value=c.l1.call("getAccountInfo",serde_json::json!([text(&w.scope),{"encoding":"base64","commitment":"finalized"}]),&c.signer)?;
                if !value["value"].is_null() {
                    let row = &value["value"];
                    let receipt = crate::decode_phoenix_funding_receipt(
                        w.scope,
                        crate::transaction::bytes(crate::rpc::string(&row["owner"])?)?,
                        &crate::rpc::data(row, &text(&vault_id()))?,
                        &crate::FundingIntent::new(nonce, amount, trader, program),
                    )?;
                    if crate::rpc::number(&value["context"]["slot"])? < receipt.funded_at_slot {
                        return Err(RuntimeError::Stale);
                    }
                    WriteState::Applied(receipt.funded_at_slot)
                } else {
                    let receipt = c.l1.transaction(&text_signature(&sig), &c.signer, true)?;
                    if receipt.is_null() {
                        return Ok(true);
                    }
                    w.receipt(&receipt, c.signer.pubkey().to_bytes())?
                }
            } else {
                let rpc = if public { &mut c.l1 } else { &mut c.qfs };
                let receipt = rpc.transaction(&text_signature(&sig), &c.signer, public)?;
                if receipt.is_null() {
                    return Ok(true);
                }
                w.receipt(&receipt, c.signer.pubkey().to_bytes())?
            }
        };
        self.coordinator
            .maintenance_journal()
            .finish_maintenance_write(w.id, finality)
            .map_err(|_| RuntimeError::Journal)?;
        Ok(true)
    }
    pub(crate) fn maintenance_send(
        &mut self,
        scope: [u8; 32],
        body: Vec<u8>,
        parameters: P,
        book: &Book,
        users: &[([u8; 32], UserLedger)],
        evidence: [u8; 32],
    ) -> Result<()> {
        let before = private_hash(book, users)?;
        let w = MaintenanceWrite::new(scope, &body, before, evidence, parameters);
        // Snapshot coherence is checked after all valuation work and before WAL.
        let (b, ls, v) = self.maintenance_snapshot()?;
        if private_hash(&b, &ls)? != before
            || b.halt & cc::OPERATOR_DOWN == 0
            || v.halt & cc::OPERATOR_DOWN == 0
            || unix_ms()
                .checked_sub(v.mark_ms)
                .is_none_or(|age| age > cc::MARK_STALE_MS)
            || v.financial_fingerprint() != evidence
        {
            return Err(RuntimeError::Stale);
        }
        self.coordinator
            .maintenance_journal()
            .prepare_maintenance_write(&w)
            .map_err(|_| RuntimeError::Journal)?;
        let tx = {
            let mut c = self.context.borrow_mut();
            let c = &mut *c;
            let public = matches!(w.parameters, P::Root { .. } | P::PostCollateral { .. });
            if let P::PostCollateral {
                nonce,
                amount,
                trader,
                program,
                gti,
            } = w.parameters
            {
                let cfg = c.vault_config()?;
                let global = c.config.phoenix_global_config.clone();
                let (_, rows) = c.l1.accounts(std::slice::from_ref(&global), &c.signer)?;
                let accounts = crate::PhoenixFundingAccounts::from_global(
                    &c.config,
                    &cfg,
                    crate::transaction::bytes(&global)?,
                    crate::transaction::bytes(&c.config.phoenix_program)?,
                    &crate::rpc::data(&rows[0], &c.config.phoenix_program)?,
                )?;
                if usize::from(gti) != c.config.global_trader_index.len() {
                    return Err(RuntimeError::Identity);
                }
                let ix = crate::build_phoenix_funding(
                    &crate::FundingIntent::new(nonce, amount, trader, program),
                    c.signer.pubkey().to_bytes(),
                    &accounts,
                )?;
                if ix.data != body {
                    return Err(RuntimeError::Identity);
                }
                crate::transaction::sign_native(&mut c.l1, &c.signer, ix)?
            } else {
                let mut accounts = vec![
                    (c.signer.pubkey().to_bytes(), true, false),
                    (config_key(), false, false),
                    (scope, false, true),
                ];
                if matches!(w.parameters, P::Fold { .. } | P::Liquidate { .. }) {
                    accounts.push((book_key(), false, true));
                }
                let ix = anchor_ix(
                    if public { vault_id() } else { ledger_id() },
                    accounts,
                    body,
                );
                sign(if public { &mut c.l1 } else { &mut c.qfs }, &c.signer, ix)?
            }
        };
        self.coordinator
            .maintenance_journal()
            .sign_maintenance_write(w.id, tx.signature, tx.expiry)
            .map_err(|_| RuntimeError::Journal)?;
        let mut c = self.context.borrow_mut();
        let c = &mut *c;
        send(
            if matches!(w.parameters, P::Root { .. } | P::PostCollateral { .. }) {
                &mut c.l1
            } else {
                &mut c.qfs
            },
            &c.signer,
            &tx,
        )?;
        Ok(())
    }
    /// Authoritative observed native settlement, not collateral-change guessing.
    /// Per-asset funding must be zero and the saved accumulator generations must
    /// still match. A netted book's zero native liability is a valid no-op witness.
    pub fn fold_funding(&mut self) -> Result<bool> {
        crate::ledger::set_down(&mut self.context.borrow_mut(), true)?;
        if self.drain_maintenance_write()? {
            return Ok(true);
        }
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
        let cp = self
            .journal()
            .funding_checkpoint()
            .map_err(|_| RuntimeError::Journal)?
            .ok_or(RuntimeError::FundingAllocationRequired)?;
        let (book, users, view) = self.maintenance_snapshot()?;
        let observed = crate::FundingCheckpoint::capture(&view, &book, &users, unix_ms())?;
        if cp.epoch() != observed.epoch()
            || cp.rates() != observed.rates()
            || cp.inventory_hash != observed.inventory_hash
            || cp.registry_hash != observed.registry_hash
            || view.funding != 0
            || view.asset_funding.values().any(|n| *n != 0)
        {
            return Ok(false);
        }
        // Settlement preserves total raw backing. Prove the complete extended
        // I2 BEFORE folding any private user; a healthy pool alone is insufficient.
        let snapshot = crate::ledger::QfsLedger::new(self.context.clone())
            .reconciliation()
            .map_err(|_| RuntimeError::Incomplete)?;
        if !snapshot.accounting_holds() {
            return Err(RuntimeError::Identity);
        }
        let Some((scope, l)) = users.iter().find(|(_, l)| {
            l.positions[..l.positions_len as usize]
                .iter()
                .any(|p| p.unsettled_funding != 0)
        }) else {
            return Ok(false);
        };
        let value = crate::solvency::value_user(&view, l, unix_ms())?;
        let notional = l.positions[..l.positions_len as usize]
            .iter()
            .try_fold(0u64, |n, p| {
                let m = view
                    .markets
                    .get(&p.asset_id)
                    .ok_or(RuntimeError::Incomplete)?;
                n.checked_add(
                    p.lots
                        .unsigned_abs()
                        .checked_mul(m.mark_price.as_inner())
                        .and_then(|n| n.checked_mul(m.tick_size.as_inner()))
                        .ok_or(RuntimeError::Decode)?,
                )
                .ok_or(RuntimeError::Decode)
            })?;
        let mut entries = Vec::new();
        for p in &l.positions[..l.positions_len as usize] {
            let risk = crate::rise::position_risk(
                &view,
                p.asset_id,
                p.lots,
                p.entry_quote_lots,
                notional,
                0,
                unix_ms(),
            )?;
            entries.push(FundingEntry {
                asset_id: p.asset_id,
                delta_usdc: 0,
                post_position_im_usdc: risk.post_position_im_usdc,
            });
        }
        if entries
            .iter()
            .try_fold(0u64, |n, e| n.checked_add(e.post_position_im_usdc))
            .ok_or(RuntimeError::Decode)?
            != value.initial_margin
        {
            return Err(RuntimeError::Identity);
        }
        let body = cinder_ledger::instruction::AllocateFunding {
            epoch: book.funding_epoch,
            fold: true,
            entries,
        }
        .data();
        self.maintenance_send(
            *scope,
            body,
            P::Fold {
                epoch: book.funding_epoch,
            },
            &book,
            &users,
            view.financial_fingerprint(),
        )?;
        Ok(true)
    }
    /// Confirm a full fresh scan before journaling its Unix-ms heartbeat.
    pub fn heartbeat(&mut self) -> Result<bool> {
        crate::ledger::set_down(&mut self.context.borrow_mut(), true)?;
        if self.drain_maintenance_write()? {
            return Ok(true);
        }
        if self
            .journal()
            .has_unresolved_maintenance()
            .map_err(|_| RuntimeError::Journal)?
            || !self
                .journal()
                .nonterminal_operations()
                .map_err(|_| RuntimeError::Journal)?
                .is_empty()
        {
            return Ok(false);
        }
        let (book, users, view) = self.maintenance_snapshot()?;
        self.heartbeat_from(&book, &users, &view)
    }
    pub(crate) fn heartbeat_from(
        &mut self,
        book: &Book,
        users: &[([u8; 32], UserLedger)],
        view: &crate::rise::RiseView,
    ) -> Result<bool> {
        crate::maintenance::scan_user_health(view, users, unix_ms())?;
        let now = unix_ms();
        if now < book.last_scan_ms {
            return Err(RuntimeError::Stale);
        }
        if now
            .checked_sub(book.last_scan_ms)
            .is_none_or(|age| age < cc::HEARTBEAT_MS)
        {
            return Ok(false);
        }
        self.maintenance_send(
            book_key(),
            cinder_ledger::instruction::HeartbeatScan { now_ms: now }.data(),
            P::Heartbeat { now_ms: now },
            book,
            users,
            view.financial_fingerprint(),
        )?;
        Ok(true)
    }
    /// Publish aggregate cash claims only after complete fresh I1/I2 and no
    /// unresolved financial outboxes. Debt does not get hidden by root totals.
    pub fn publish_reserve(&mut self) -> Result<bool> {
        crate::ledger::set_down(&mut self.context.borrow_mut(), true)?;
        if self.drain_maintenance_write()? {
            return Ok(true);
        }
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
        let (book, users, view) = self.maintenance_snapshot()?;
        let snapshot = crate::ledger::QfsLedger::new(self.context.clone())
            .reconciliation()
            .map_err(|_| RuntimeError::Incomplete)?;
        if !snapshot.accounting_holds() {
            return Err(RuntimeError::Identity);
        }
        let registry = self
            .journal()
            .known_users()
            .map_err(|_| RuntimeError::Journal)?
            .into_iter()
            .collect();
        let claims = crate::build_reserve_snapshot(&book, &users, &registry)?;
        let scope = pda(vault_id(), &[cc::SEED_RESERVE]);
        let root = {
            let mut c = self.context.borrow_mut();
            let c = &mut *c;
            let (_, rows) = c.l1.accounts(&[text(&scope)], &c.signer)?;
            decode::<cinder_vault::ReserveRoot>(&rows[0], &text(&vault_id()))?
        };
        if root.schema_version != cc::ACCOUNT_SCHEMA_VERSION {
            return Err(RuntimeError::Identity);
        }
        if root.root == claims.root
            && root.book_hash == claims.book_hash
            && root.total_bad_debt == claims.total_bad_debt
            && root.user_count == claims.user_count
            && root.total_free == claims.total_free
            && root.total_reserved == claims.total_reserved
        {
            return Ok(false);
        }
        let params = P::Root {
            epoch: root.epoch,
            root: claims.root,
            users: claims.user_count,
            free: claims.total_free,
            reserved: claims.total_reserved,
            debt: claims.total_bad_debt,
            book_hash: claims.book_hash,
            ack_count: self
                .journal()
                .acknowledged_fills()
                .map_err(|_| RuntimeError::Journal)?,
            observed_ms: unix_ms(),
        };
        let body = cinder_vault::instruction::WriteReserveRootGuarded {
            expected_epoch: root.epoch,
            root: claims.root,
            user_count: claims.user_count,
            total_free: claims.total_free,
            total_reserved: claims.total_reserved,
            total_bad_debt: claims.total_bad_debt,
            book_hash: claims.book_hash,
        }
        .data();
        self.maintenance_send(
            scope,
            body,
            params,
            &book,
            &users,
            view.financial_fingerprint(),
        )?;
        Ok(true)
    }
    pub fn sync_collateral(&mut self) -> Result<bool> {
        crate::ledger::set_down(&mut self.context.borrow_mut(), true)?;
        if self.drain_maintenance_write()? {
            return Ok(true);
        }
        if self
            .journal()
            .has_unresolved_maintenance()
            .map_err(|_| RuntimeError::Journal)?
            || !self
                .journal()
                .nonterminal_operations()
                .map_err(|_| RuntimeError::Journal)?
                .is_empty()
        {
            return Ok(false);
        }
        let (book, users, view) = self.maintenance_snapshot()?;
        if book.phoenix_collateral == view.collateral {
            return Ok(false);
        }
        self.maintenance_send(
            book_key(),
            cinder_ledger::instruction::UpdateBookCollateral {
                phoenix_collateral: view.collateral,
            }
            .data(),
            P::Collateral {
                usdc: view.collateral,
            },
            &book,
            &users,
            view.financial_fingerprint(),
        )?;
        Ok(true)
    }

    /// Periodic repair uses the same PDA custody conversion as pre-trade
    /// funding, but its own single-writer identity, not an invented user order.
    /// Moving cash never creates backing. Uncertain deposits drain before Book
    /// synchronization, another funding generation, or any venue dispatch.
    pub(crate) fn repair_collateral(
        &mut self,
        book: &Book,
        users: &[([u8; 32], UserLedger)],
        view: &crate::rise::RiseView,
    ) -> Result<bool> {
        if !view.safe || (book.halt | view.halt) & (cc::INVARIANT_BROKEN | cc::VENUE_BREACH) != 0 {
            return Ok(false);
        }
        let amount = collateral_shortfall(view)?;
        if amount == 0 {
            return Ok(false);
        }
        if amount > view.vault_balance {
            return Err(RuntimeError::Incomplete);
        }
        let snapshot = crate::ledger::QfsLedger::new(self.context.clone())
            .reconciliation()
            .map_err(|_| RuntimeError::Incomplete)?;
        if !snapshot.accounting_holds() {
            return Err(RuntimeError::Identity);
        }
        let (nonce, trader, program, gti) = {
            let c = self.context.borrow();
            (
                solana_keypair::Keypair::new().pubkey().to_bytes(),
                crate::transaction::bytes(&c.config.phoenix_trader)?,
                crate::transaction::bytes(&c.config.phoenix_program)?,
                u8::try_from(c.config.global_trader_index.len())
                    .map_err(|_| RuntimeError::Configuration)?,
            )
        };
        let intent = crate::FundingIntent::new(nonce, amount, trader, program);
        let scope = pda(
            vault_id(),
            &[cinder_vault::SEED_PHOENIX_FUNDING, &intent.funding_id],
        );
        let body = cinder_vault::instruction::FundPhoenix {
            funding_id: intent.funding_id,
            amount,
            global_trader_index_count: gti,
        }
        .data();
        self.maintenance_send(
            scope,
            body,
            P::PostCollateral {
                nonce,
                amount,
                trader,
                program,
                gti,
            },
            book,
            users,
            view.financial_fingerprint(),
        )?;
        Ok(true)
    }
}
