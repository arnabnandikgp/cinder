//! One operator credential and guarded ER acknowledgements.
use crate::rpc::{number, Result, RuntimeError};
use crate::runtime::{
    book_key, config_key, fees_key, ledger_id, same_intent, text, text_signature, vault_id,
    Context, Shared,
};
use crate::transaction::{
    anchor_ix, bytes, decode_ix, instruction_data, send, sign, Receipt, SignedTransaction,
};
use crate::{
    AckFinality, AckObservation, BoundedIntent, ErAckCommand, ErAckSubmissionPort,
    ErAckSubmitResult, ErrorCode, LedgerRecoveryPort, Operation, PreparedAck,
    ReconciliationSnapshot,
};
use anchor_lang::InstructionData;
use cinder_common as cc;
use cinder_ledger::AckGuard;
use serde_json::json;
use solana_signer::Signer;
use std::collections::{BTreeMap, BTreeSet};

fn error(_: RuntimeError) -> ErrorCode {
    ErrorCode::LedgerUnavailable
}
fn pending(
    context: &mut Context,
    intent: &BoundedIntent,
) -> Result<(cinder_ledger::UserLedger, AckGuard)> {
    let ledger = context.ledger(&intent.identity.user_ledger)?;
    let latest = context.placements(&intent.identity.user_ledger, &ledger)?;
    if !latest
        .get(&intent.identity.client_oid)
        .is_some_and(|p| same_intent(p, intent))
    {
        return Err(RuntimeError::Identity);
    }
    let guard = AckGuard {
        client_oid: intent.identity.client_oid,
        placement_nonce: intent.identity.user_nonce,
        observed_ledger_nonce: ledger.nonce,
        observed_ledger_hash: cinder_ledger::ledger_state_hash(&ledger)
            .map_err(|_| RuntimeError::Decode)?,
        kind: intent.identity.kind as u8,
        asset_id: intent.asset_id,
        requested_lots: intent.requested_lots,
        limit_price_ticks: intent.limit_price_ticks,
        last_valid_slot: intent.last_valid_slot,
    };
    guard
        .validate(&ledger)
        .map_err(|_| RuntimeError::Identity)?;
    Ok((ledger, guard))
}
fn wait(context: &mut Context, qfs: bool, signature: &[u8; 64]) -> Result<()> {
    // Bounded confirmation, never resend a side effect on a transport failure.
    for _ in 0..20 {
        let rpc = if qfs {
            &mut context.qfs
        } else {
            &mut context.l1
        };
        let status = rpc.call(
            "getSignatureStatuses",
            // This is immediate confirmation of a just-sent ownership gate,
            // not historical financial recovery. Avoid remote history lookup
            // while the private transaction is still entering the recent cache.
            json!([[text_signature(signature)],{"searchTransactionHistory":false}]),
            &context.signer,
        )?;
        let row = &status["value"][0];
        if !row.is_null() {
            if !row["err"].is_null() {
                return Err(RuntimeError::Rpc);
            }
            if matches!(
                row["confirmationStatus"].as_str(),
                Some("confirmed" | "finalized")
            ) {
                return Ok(());
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Err(RuntimeError::Incomplete)
}
pub(crate) fn set_down(context: &mut Context, down: bool) -> Result<()> {
    let cfg = context.vault_config()?;
    let adapter = context.signer.pubkey().to_bytes();
    if (cfg.paused & cc::OPERATOR_DOWN != 0) != down {
        let ix = anchor_ix(
            vault_id(),
            vec![(adapter, true, false), (config_key(), false, true)],
            cinder_vault::instruction::SetOperatorDown { down }.data(),
        );
        let tx = sign(&mut context.l1, &context.signer, ix)?;
        send(&mut context.l1, &context.signer, &tx)?;
        wait(context, false, &tx.signature)?;
    }
    let (_, rows) = context
        .qfs
        .accounts(&[text(&book_key())], &context.signer)?;
    let book: cinder_ledger::Book = crate::runtime::decode(&rows[0], &text(&ledger_id()))?;
    if (book.halt & cc::OPERATOR_DOWN != 0) != down {
        let ix = anchor_ix(
            ledger_id(),
            vec![
                (adapter, true, false),
                (config_key(), false, false),
                (book_key(), false, true),
            ],
            cinder_ledger::instruction::SetOperatorDown { down }.data(),
        );
        let tx = sign(&mut context.qfs, &context.signer, ix)?;
        send(&mut context.qfs, &context.signer, &tx)?;
        // This setter has no financial effect. Confirm its absolute state
        // through authenticated account reads, rather than asking QFS to
        // disclose a transaction's signature metadata. Financial ACKs still
        // require their exact authenticated transaction receipt.
        let mut applied = false;
        for _ in 0..20 {
            let (_, rows) = context
                .qfs
                .accounts(&[text(&book_key())], &context.signer)?;
            let book: cinder_ledger::Book = crate::runtime::decode(&rows[0], &text(&ledger_id()))?;
            if (book.halt & cc::OPERATOR_DOWN != 0) == down {
                applied = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        if !applied {
            return Err(RuntimeError::Incomplete);
        }
    }
    let cfg = context.vault_config()?;
    let (_, rows) = context
        .qfs
        .accounts(&[text(&book_key())], &context.signer)?;
    let book: cinder_ledger::Book = crate::runtime::decode(&rows[0], &text(&ledger_id()))?;
    if (cfg.paused & cc::OPERATOR_DOWN != 0) != down || (book.halt & cc::OPERATOR_DOWN != 0) != down
    {
        return Err(RuntimeError::Incomplete);
    }
    // Clearing the ownership gate may race another halt, which the atomic
    // instructions preserve. Never call that race an enabled entry gate.
    if !down && cc::entries_blocked((cfg.paused | book.halt) & !cc::OPERATOR_DOWN) {
        return Err(RuntimeError::Incomplete);
    }
    Ok(())
}

pub(crate) struct QfsLedger {
    context: Shared,
    prepared: Option<(ErAckCommand, SignedTransaction)>,
    prepared_book_sync: Option<(crate::PreparedBookSync, SignedTransaction)>,
}
impl QfsLedger {
    pub fn new(context: Shared) -> Self {
        Self {
            context,
            prepared: None,
            prepared_book_sync: None,
        }
    }
    fn observe(&mut self, op: &Operation, attempts: &[PreparedAck]) -> Result<AckObservation> {
        let mut context = self.context.borrow_mut();
        let adapter = context.signer.pubkey().to_bytes();
        let mut unresolved = false;
        for attempt in attempts {
            let value = {
                let c = &mut *context;
                c.qfs
                    .transaction(&text_signature(&attempt.signature), &c.signer, false)?
            };
            if !value.is_null() {
                let receipt = Receipt::decode(&value, &attempt.signature)?;
                if !receipt.succeeded {
                    continue;
                }
                let mut finality = None;
                for ix in &receipt.instructions {
                    if receipt.program(ix)? != ledger_id() {
                        continue;
                    }
                    let raw = instruction_data(ix)?;
                    let fill =
                        decode_ix::<cinder_ledger::instruction::AckPhoenixFillGuarded>(&raw)?;
                    let fail =
                        decode_ix::<cinder_ledger::instruction::AckPhoenixFailGuarded>(&raw)?;
                    let guard = if let Some(f) = &fill {
                        &f.guard
                    } else if let Some(f) = &fail {
                        &f.guard
                    } else {
                        continue;
                    };
                    let expected = &op.intent;
                    if guard.client_oid != expected.identity.client_oid
                        || guard.placement_nonce != expected.identity.user_nonce
                        || guard.observed_ledger_nonce != attempt.observed_ledger_nonce
                        || guard.kind != expected.identity.kind as u8
                        || guard.asset_id != expected.asset_id
                        || guard.requested_lots != expected.requested_lots
                        || guard.limit_price_ticks != expected.limit_price_ticks
                        || guard.last_valid_slot != expected.last_valid_slot
                        || receipt.account(ix, 0)? != adapter
                        || receipt.account(ix, 1)? != config_key()
                        || !receipt.requires_signature(&adapter)
                    {
                        return Err(RuntimeError::Identity);
                    }
                    if finality.is_some() {
                        return Err(RuntimeError::Identity);
                    }
                    finality = Some(if let Some(f) = fill {
                        if receipt.account(ix, 2)? != expected.identity.user_ledger
                            || receipt.account(ix, 3)? != book_key()
                            || receipt.account(ix, 4)? != fees_key()
                        {
                            return Err(RuntimeError::Identity);
                        }
                        AckFinality::Filled {
                            lots: f.filled_lots,
                            quote_lots: f.vwap_quote_lots,
                            fee_usdc: f.fee_usdc,
                        }
                    } else {
                        if receipt.account(ix, 2)? != book_key()
                            || receipt.account(ix, 3)? != expected.identity.user_ledger
                        {
                            return Err(RuntimeError::Identity);
                        }
                        AckFinality::Failed
                    });
                }
                if let Some(finality) = finality {
                    return Ok(AckObservation {
                        identity: op.intent.identity,
                        operation_id: op.operation_id,
                        finality,
                    });
                }
                return Err(RuntimeError::Identity);
            }
            let c = &mut *context;
            let status = c.qfs.call(
                "getSignatureStatuses",
                json!([[text_signature(&attempt.signature)],{"searchTransactionHistory":true}]),
                &c.signer,
            )?;
            let row = &status["value"][0];
            if !row.is_null() && row["err"].is_null() {
                unresolved = true;
                continue;
            }
            if row.is_null() {
                let height = number(&c.qfs.call(
                    "getBlockHeight",
                    json!([{"commitment":"confirmed"}]),
                    &c.signer,
                )?)?;
                if height <= attempt.last_valid_block_height {
                    unresolved = true;
                }
            }
        }
        let finality = if unresolved {
            AckFinality::Pending
        } else if pending(&mut context, &op.intent).is_ok() {
            // Expiry alone was not proof. Also require the exact original
            // generation to remain pending, and inspect every prior send.
            if op.er_ack_signature.is_some()
                && !attempts
                    .iter()
                    .any(|a| Some(a.signature) == op.er_ack_signature)
            {
                AckFinality::Unknown
            } else {
                AckFinality::DefinitelyNotApplied
            }
        } else {
            AckFinality::Unknown
        };
        Ok(AckObservation {
            identity: op.intent.identity,
            operation_id: op.operation_id,
            finality,
        })
    }
}
impl ErAckSubmissionPort for QfsLedger {
    fn prepare_ack(
        &mut self,
        command: &ErAckCommand,
    ) -> std::result::Result<Option<PreparedAck>, ErrorCode> {
        self.prepared = None;
        let result = (|| {
            let mut context = self.context.borrow_mut();
            let intent = match command {
                ErAckCommand::Fill { intent, .. } | ErAckCommand::Fail { intent } => intent,
            };
            let (ledger, guard) = pending(&mut context, intent)?;
            let assets = ledger.positions[..ledger.positions_len as usize]
                .iter()
                .map(|p| p.asset_id)
                .chain(std::iter::once(intent.asset_id))
                .collect();
            let view = crate::rise::load(&mut context, &assets)?;
            let position = ledger.positions[..ledger.positions_len as usize]
                .iter()
                .find(|p| p.asset_id == intent.asset_id);
            let tentative = position.map(|p| p.lots).unwrap_or(0);
            let entry = position.map(|p| p.entry_quote_lots).unwrap_or(0);
            let adapter = context.signer.pubkey().to_bytes();
            let accounts;
            let data;
            match command {
                ErAckCommand::Fill { fills, .. } => {
                    let mut lots = 0i64;
                    let mut quote = 0i64;
                    let mut fee = 0u64;
                    let mut worst = if intent.requested_lots > 0 {
                        0
                    } else {
                        u64::MAX
                    };
                    for f in fills {
                        lots = lots
                            .checked_add(f.filled_lots)
                            .ok_or(RuntimeError::Decode)?;
                        quote = quote
                            .checked_add(f.vwap_quote_lots)
                            .ok_or(RuntimeError::Decode)?;
                        fee = fee.checked_add(f.fee_usdc).ok_or(RuntimeError::Decode)?;
                        worst = if intent.requested_lots > 0 {
                            worst.max(f.fill_price_ticks)
                        } else {
                            worst.min(f.fill_price_ticks)
                        };
                    }
                    if lots == 0
                        || lots.signum() != intent.requested_lots.signum()
                        || lots.unsigned_abs() > intent.requested_lots.unsigned_abs()
                    {
                        return Err(RuntimeError::Unsupported);
                    }
                    let delta = ledger
                        .open_oids
                        .iter()
                        .filter(|o| {
                            o.asset_id == intent.asset_id
                                && matches!(o.state, cc::OID_PENDING | cc::OID_LIQUIDATING)
                        })
                        .try_fold(0i64, |s, o| s.checked_add(o.lots_delta))
                        .ok_or(RuntimeError::Decode)?;
                    let before = tentative.checked_sub(delta).ok_or(RuntimeError::Decode)?;
                    let realized = cc::realize_on_fill(before, lots, entry, quote)
                        .ok_or(RuntimeError::Decode)?;
                    let unfilled = intent
                        .requested_lots
                        .checked_sub(lots)
                        .ok_or(RuntimeError::Decode)?;
                    let post_lots = tentative
                        .checked_sub(unfilled)
                        .ok_or(RuntimeError::Decode)?;
                    let margin = crate::rise::user_margin(
                        &view,
                        &ledger,
                        intent.asset_id,
                        post_lots,
                        realized.new_entry_quote,
                    )?;
                    data = cinder_ledger::instruction::AckPhoenixFillGuarded {
                        guard: guard.clone(),
                        filled_lots: lots,
                        fee_usdc: fee,
                        vwap_quote_lots: quote,
                        fill_price_ticks: worst,
                        post_position_im_usdc: margin,
                    }
                    .data();
                    accounts = vec![
                        (adapter, true, false),
                        (config_key(), false, false),
                        (intent.identity.user_ledger, false, true),
                        (book_key(), false, true),
                        (fees_key(), false, true),
                    ];
                }
                ErAckCommand::Fail { .. } => {
                    let restored = tentative
                        .checked_sub(intent.requested_lots)
                        .ok_or(RuntimeError::Decode)?;
                    let margin =
                        crate::rise::user_margin(&view, &ledger, intent.asset_id, restored, entry)?;
                    data = cinder_ledger::instruction::AckPhoenixFailGuarded {
                        guard: guard.clone(),
                        post_position_im_usdc: margin,
                    }
                    .data();
                    accounts = vec![
                        (adapter, true, false),
                        (config_key(), false, false),
                        (book_key(), false, true),
                        (intent.identity.user_ledger, false, true),
                    ];
                }
            }
            // Re-check the private generation after the public risk I/O.
            let (_, fresh_guard) = pending(&mut context, intent)?;
            if fresh_guard != guard {
                return Err(RuntimeError::Stale);
            }
            let c = &mut *context;
            let tx = sign(
                &mut c.qfs,
                &c.signer,
                anchor_ix(ledger_id(), accounts, data),
            )?;
            let ack = PreparedAck {
                signature: tx.signature,
                last_valid_block_height: tx.expiry,
                observed_ledger_nonce: guard.observed_ledger_nonce,
            };
            self.prepared = Some((command.clone(), tx));
            Ok(Some(ack))
        })();
        result.map_err(error)
    }
    fn submit_ack(
        &mut self,
        command: &ErAckCommand,
    ) -> std::result::Result<ErAckSubmitResult, ErrorCode> {
        let (expected, tx) = self.prepared.take().ok_or(ErrorCode::LedgerUnavailable)?;
        if &expected != command {
            return Err(ErrorCode::CorrelationMismatch);
        }
        let mut c = self.context.borrow_mut();
        let c = &mut *c;
        send(&mut c.qfs, &c.signer, &tx)
            .map(ErAckSubmitResult::Accepted)
            .map_err(error)
    }
}
impl LedgerRecoveryPort for QfsLedger {
    fn plan_execution_budget(
        &mut self,
        op: &Operation,
    ) -> std::result::Result<Option<crate::ExecutionBudget>, ErrorCode> {
        (|| {
            let mut c = self.context.borrow_mut();
            pending(&mut c, &op.intent)?;
            let assets = c.config.markets.iter().map(|m| m.cinder_asset_id).collect();
            let view = crate::rise::load(&mut c, &assets)?;
            let policy = c
                .config
                .execution_policy
                .as_ref()
                .ok_or(RuntimeError::Configuration)?;
            Ok(Some(policy.budget(&view, op)?))
        })()
        .map_err(error)
    }
    fn funding_required(
        &mut self,
        op: &Operation,
    ) -> std::result::Result<Option<crate::FundingIntent>, ErrorCode> {
        (|| {
            let mut c = self.context.borrow_mut();
            let (view, shortfall) = admission(&mut c, op)?;
            if shortfall == 0 {
                return Ok(None);
            }
            let cfg = c.vault_config()?;
            if view.vault_balance < shortfall {
                return Err(RuntimeError::Incomplete);
            }
            Ok(Some(crate::FundingIntent::new(
                op.operation_id,
                shortfall,
                cfg.phoenix_trader.to_bytes(),
                bytes(&c.config.phoenix_program)?,
            )))
        })()
        .map_err(error)
    }
    fn check_admission(&mut self, op: &Operation) -> std::result::Result<(), ErrorCode> {
        (|| {
            let mut c = self.context.borrow_mut();
            c.execution_admission = None;
            let (view, shortfall) = admission(&mut c, op)?;
            if shortfall != 0 {
                return Err(RuntimeError::Incomplete);
            }
            c.execution_admission = Some((
                op.operation_id,
                op.execution_budget.ok_or(RuntimeError::Incomplete)?,
                view,
            ));
            Ok(())
        })()
        .map_err(error)
    }
    fn prepare_funding_book_sync(
        &mut self,
        record: &crate::FundingRecord,
    ) -> std::result::Result<crate::PreparedBookSync, ErrorCode> {
        let result = (|| {
            let funded_slot = record.confirmed_at_slot.ok_or(RuntimeError::Incomplete)?;
            let mut c = self.context.borrow_mut();
            let c = &mut *c;
            let assets = c.config.markets.iter().map(|m| m.cinder_asset_id).collect();
            let view = crate::rise::load(c, &assets)?;
            if view.slot < funded_slot
                || !view.safe
                || cc::entries_blocked(view.halt & !cc::OPERATOR_DOWN)
            {
                return Err(RuntimeError::Incomplete);
            }
            let adapter = c.signer.pubkey().to_bytes();
            let ix = anchor_ix(
                ledger_id(),
                vec![
                    (adapter, true, false),
                    (config_key(), false, false),
                    (book_key(), false, true),
                ],
                cinder_ledger::instruction::UpdateBookCollateral {
                    phoenix_collateral: view.collateral,
                }
                .data(),
            );
            let tx = sign(&mut c.qfs, &c.signer, ix)?;
            crate::transaction::simulate(&mut c.qfs, &c.signer, &tx, 0)?;
            // Native and ER slots are different clocks. Only the L1 funding
            // slot is compared to this authoritative L1 view, never to ER.
            let prepared = crate::PreparedBookSync {
                transaction: crate::PreparedVenue {
                    signature: tx.signature,
                    last_valid_block_height: tx.expiry,
                },
                collateral_usdc: view.collateral,
                native_observed_slot: view.slot,
            };
            self.prepared_book_sync = Some((prepared.clone(), tx));
            Ok(prepared)
        })();
        result.map_err(error)
    }
    fn submit_funding_book_sync(
        &mut self,
        prepared: &crate::PreparedBookSync,
    ) -> std::result::Result<ErAckSubmitResult, ErrorCode> {
        let (expected, tx) = self
            .prepared_book_sync
            .take()
            .ok_or(ErrorCode::LedgerUnavailable)?;
        if &expected != prepared {
            return Err(ErrorCode::CorrelationMismatch);
        }
        let mut c = self.context.borrow_mut();
        let c = &mut *c;
        send(&mut c.qfs, &c.signer, &tx)
            .map(ErAckSubmitResult::Accepted)
            .map_err(error)
    }
    fn observe_funding_book_sync(
        &mut self,
        prepared: &crate::PreparedBookSync,
    ) -> std::result::Result<crate::BookSyncObservation, ErrorCode> {
        let result = (|| {
            let mut c = self.context.borrow_mut();
            let c = &mut *c;
            let value = c.qfs.transaction(
                &text_signature(&prepared.transaction.signature),
                &c.signer,
                false,
            )?;
            if value.is_null() {
                return Ok(crate::BookSyncObservation::Unknown);
            }
            decode_book_sync(&value, prepared, &c.signer.pubkey().to_bytes())
        })();
        result.map_err(error)
    }
    fn validate_funding_book_sync(
        &mut self,
        prepared: &crate::PreparedBookSync,
    ) -> std::result::Result<bool, ErrorCode> {
        let result = (|| {
            let mut c = self.context.borrow_mut();
            let c = &mut *c;
            let assets = c.config.markets.iter().map(|m| m.cinder_asset_id).collect();
            let view = crate::rise::load(c, &assets)?;
            let (_, rows) = c.qfs.accounts(&[text(&book_key())], &c.signer)?;
            let book: cinder_ledger::Book = crate::runtime::decode(&rows[0], &text(&ledger_id()))?;
            let now = crate::rpc::unix_ms();
            if view.slot < prepared.native_observed_slot
                || now < view.observed_ms
                || now.saturating_sub(view.observed_ms) > cc::TRADER_STATE_STALE_MS
                || book.halt & cc::OPERATOR_DOWN == 0
                || !view.safe
            {
                return Err(RuntimeError::Stale);
            }
            Ok(view.collateral == prepared.collateral_usdc
                && book.phoenix_collateral == view.collateral)
        })();
        result.map_err(error)
    }
    fn observe_ack(&mut self, op: &Operation) -> std::result::Result<AckObservation, ErrorCode> {
        self.observe(op, &[]).map_err(error)
    }
    fn observe_ack_attempts(
        &mut self,
        op: &Operation,
        attempts: &[PreparedAck],
    ) -> std::result::Result<AckObservation, ErrorCode> {
        self.observe(op, attempts).map_err(error)
    }
    fn set_operator_down(&mut self, down: bool) -> std::result::Result<(), ErrorCode> {
        set_down(&mut self.context.borrow_mut(), down).map_err(error)
    }
    fn reconciliation(&mut self) -> std::result::Result<ReconciliationSnapshot, ErrorCode> {
        (|| {
            let mut context = self.context.borrow_mut();
            let (book, ledgers, observed) = context.private_ledgers()?;
            let mut cash = 0i128;
            let mut funding = 0i128;
            let mut confirmed = BTreeMap::new();
            let mut entry_quote_lots = BTreeMap::new();
            let mut assets = BTreeSet::new();
            let mut fingerprints = Vec::new();
            for (address, ledger) in &ledgers {
                // Without a money-movement outbox, deferred withdrawals
                // cannot be reconciled as an invented zero cash-in-flight.
                if ledger.withdrawable != 0 {
                    return Err(RuntimeError::Unsupported);
                }
                cash = cash
                    .checked_add(
                        i128::from(ledger.free) + i128::from(ledger.reserved)
                            - i128::from(ledger.bad_debt_usdc),
                    )
                    .ok_or(RuntimeError::Decode)?;
                for p in &ledger.positions[..ledger.positions_len as usize] {
                    assets.insert(p.asset_id);
                    funding = funding
                        .checked_add(i128::from(p.unsettled_funding))
                        .ok_or(RuntimeError::Decode)?;
                    let confirmed_lots = confirmed_position(p, &ledger.open_oids)?;
                    let basis = entry_quote_lots.entry(p.asset_id).or_insert(0i128);
                    *basis = basis
                        .checked_add(i128::from(p.entry_quote_lots))
                        .ok_or(RuntimeError::Decode)?;
                    add(&mut confirmed, p.asset_id, confirmed_lots)?;
                }
                // Every pending asset needs its retained pre-ack position row.
                for oid in ledger.open_oids.iter().filter(|o| {
                    o.lots_delta != 0 && matches!(o.state, cc::OID_PENDING | cc::OID_LIQUIDATING)
                }) {
                    assets.insert(oid.asset_id);
                    if !ledger.positions[..ledger.positions_len as usize]
                        .iter()
                        .any(|p| p.asset_id == oid.asset_id)
                    {
                        // A reducing tentative row retains its confirmed
                        // entry basis until ack. Its absence cannot prove zero
                        // basis for the still-confirmed position.
                        return Err(RuntimeError::Incomplete);
                    }
                }
                fingerprints.push((*address, private_fingerprint(ledger)?));
            }
            let mut book_lots = BTreeMap::new();
            for r in &book.residuals[..book.residual_len as usize] {
                if r.asset_id == 0 || book_lots.insert(r.asset_id, r.lots).is_some() {
                    return Err(RuntimeError::Identity);
                }
                assets.insert(r.asset_id);
            }
            if !maps_equal(&confirmed, &book_lots) {
                return Err(RuntimeError::Identity);
            }
            let view = crate::rise::load(&mut context, &assets)?;
            let solvency = crate::solvency::evaluate(
                &view,
                &ledgers,
                context.config.solvency_policy.as_ref(),
                crate::unix_ms(),
            )?;
            let (after, after_ledgers, _) = context.private_ledgers()?;
            if book_fingerprint(&book)? != book_fingerprint(&after)?
                || fingerprints
                    != after_ledgers
                        .iter()
                        .map(|(a, l)| Ok((*a, private_fingerprint(l)?)))
                        .collect::<Result<Vec<_>>>()?
            {
                return Err(RuntimeError::Stale);
            }
            Ok(ReconciliationSnapshot {
                complete: true,
                ledger_observed_at_ms: observed,
                trader_observed_at_ms: view.observed_ms,
                mark_observed_at_ms: view.mark_ms,
                book_lots,
                phoenix_lots: view.positions,
                user_entry_quote_lots: entry_quote_lots,
                phoenix_entry_quote_lots: view.entry_quote_lots,
                user_cash_usdc: cash,
                user_unsettled_funding_usdc: funding,
                vault_usdc: view.vault_balance,
                phoenix_collateral_usdc: view.collateral,
                pool_unsettled_funding_usdc: view.funding,
                cash_in_flight_usdc: 0,
                halt_flags: book.halt | view.halt,
                pool_safe: view.safe,
                solvency: Some(solvency),
            })
        })()
        .map_err(error)
    }
}

fn admission(c: &mut Context, op: &Operation) -> Result<(crate::rise::RiseView, u64)> {
    pending(c, &op.intent)?;
    let (book, ledgers, observed) = c.private_ledgers()?;
    let assets = c.config.markets.iter().map(|m| m.cinder_asset_id).collect();
    let view = crate::rise::load(c, &assets)?;
    let (after_book, after, _) = c.private_ledgers()?;
    if cc::entries_blocked((book.halt | view.halt) & !cc::OPERATOR_DOWN)
        || book.halt & cc::OPERATOR_DOWN == 0
        || view.halt & cc::OPERATOR_DOWN == 0
        || book_fingerprint(&book)? != book_fingerprint(&after_book)?
        || ledgers
            .iter()
            .map(|(a, l)| Ok((*a, private_fingerprint(l)?)))
            .collect::<Result<Vec<_>>>()?
            != after
                .iter()
                .map(|(a, l)| Ok((*a, private_fingerprint(l)?)))
                .collect::<Result<Vec<_>>>()?
        || crate::unix_ms()
            .checked_sub(observed)
            .is_none_or(|age| age > cc::TRADER_STATE_STALE_MS)
    {
        return Err(RuntimeError::Stale);
    }
    crate::admission::accounting(&view, &book, &ledgers)?;
    let execution = c
        .config
        .execution_policy
        .as_ref()
        .ok_or(RuntimeError::Configuration)?;
    let solvency = c
        .config
        .solvency_policy
        .as_ref()
        .ok_or(RuntimeError::Configuration)?;
    let shortfall =
        crate::admission::assess(&view, &ledgers, op, execution, solvency, crate::unix_ms())?;
    Ok((view, shortfall))
}

fn decode_book_sync(
    value: &serde_json::Value,
    prepared: &crate::PreparedBookSync,
    adapter: &[u8; 32],
) -> Result<crate::BookSyncObservation> {
    let receipt = Receipt::decode(value, &prepared.transaction.signature)?;
    let mut matching = 0;
    for ix in &receipt.instructions {
        if receipt.program(ix)? != ledger_id() {
            continue;
        }
        if let Some(update) =
            decode_ix::<cinder_ledger::instruction::UpdateBookCollateral>(&instruction_data(ix)?)?
        {
            if update.phoenix_collateral != prepared.collateral_usdc
                || receipt.account(ix, 0)? != *adapter
                || receipt.account(ix, 1)? != config_key()
                || receipt.account(ix, 2)? != book_key()
                || crate::rpc::array(&ix["accounts"])?.len() != 3
                || !receipt.requires_signature(adapter)
            {
                return Err(RuntimeError::Identity);
            }
            matching += 1;
        }
    }
    if matching != 1 || receipt.slot == 0 {
        return Err(RuntimeError::Identity);
    }
    Ok(if receipt.succeeded {
        crate::BookSyncObservation::Applied { slot: receipt.slot }
    } else {
        crate::BookSyncObservation::Rejected { slot: receipt.slot }
    })
}
pub(crate) fn confirmed_position(
    p: &cinder_ledger::Position,
    oids: &[cinder_ledger::OpenOid],
) -> Result<i64> {
    let pending = oids
        .iter()
        .filter(|o| {
            o.asset_id == p.asset_id && matches!(o.state, cc::OID_PENDING | cc::OID_LIQUIDATING)
        })
        .try_fold(0i64, |sum, oid| sum.checked_add(oid.lots_delta))
        .ok_or(RuntimeError::Decode)?;
    let lots = p.lots.checked_sub(pending).ok_or(RuntimeError::Decode)?;
    let entry = p.entry_quote_lots;
    if (lots == 0 && entry != 0) || (lots != 0 && (entry == 0 || entry.signum() != lots.signum())) {
        return Err(RuntimeError::Identity);
    }
    Ok(lots)
}
fn add(map: &mut BTreeMap<u16, i64>, asset: u16, lots: i64) -> Result<()> {
    let n = map
        .get(&asset)
        .copied()
        .unwrap_or(0)
        .checked_add(lots)
        .ok_or(RuntimeError::Decode)?;
    map.insert(asset, n);
    Ok(())
}
fn maps_equal(a: &BTreeMap<u16, i64>, b: &BTreeMap<u16, i64>) -> bool {
    a.keys()
        .chain(b.keys())
        .all(|k| a.get(k).copied().unwrap_or(0) == b.get(k).copied().unwrap_or(0))
}
fn private_fingerprint(l: &cinder_ledger::UserLedger) -> Result<Vec<u8>> {
    borsh::to_vec(l).map_err(|_| RuntimeError::Decode)
}
fn book_fingerprint(l: &cinder_ledger::Book) -> Result<Vec<u8>> {
    borsh::to_vec(l).map_err(|_| RuntimeError::Decode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn book_sync_receipt_requires_exact_signature_program_operator_accounts_and_amount() {
        let adapter = [7; 32];
        let prepared = crate::PreparedBookSync {
            transaction: crate::PreparedVenue {
                signature: [8; 64],
                last_valid_block_height: 9,
            },
            collateral_usdc: 50_000_000,
            native_observed_slot: 10,
        };
        let ix = anchor_ix(
            ledger_id(),
            vec![
                (adapter, true, false),
                (config_key(), false, false),
                (book_key(), false, true),
            ],
            cinder_ledger::instruction::UpdateBookCollateral {
                phoenix_collateral: prepared.collateral_usdc,
            }
            .data(),
        );
        let value = crate::transaction::test_receipt(&ix, &prepared.transaction.signature, true);
        assert_eq!(
            decode_book_sync(&value, &prepared, &adapter).unwrap(),
            crate::BookSyncObservation::Applied { slot: 100 }
        );
        let failed = crate::transaction::test_receipt(&ix, &prepared.transaction.signature, false);
        assert_eq!(
            decode_book_sync(&failed, &prepared, &adapter).unwrap(),
            crate::BookSyncObservation::Rejected { slot: 100 }
        );
        let mut wrong = prepared.clone();
        wrong.collateral_usdc += 1;
        assert!(decode_book_sync(&value, &wrong, &adapter).is_err());
        wrong = prepared.clone();
        wrong.transaction.signature = [9; 64];
        assert!(decode_book_sync(&value, &wrong, &adapter).is_err());
        assert!(decode_book_sync(&value, &prepared, &[9; 32]).is_err());
        let mut wrong = value.clone();
        wrong["transaction"]["message"]["instructions"][0]["programIdIndex"] = 0.into();
        assert!(decode_book_sync(&wrong, &prepared, &adapter).is_err());
        let mut wrong = value.clone();
        wrong["transaction"]["message"]["instructions"][0]["accounts"][2] = 0.into();
        assert!(decode_book_sync(&wrong, &prepared, &adapter).is_err());
        let mut duplicate = value.clone();
        duplicate["transaction"]["message"]["instructions"]
            .as_array_mut()
            .unwrap()
            .push(value["transaction"]["message"]["instructions"][0].clone());
        assert!(decode_book_sync(&duplicate, &prepared, &adapter).is_err());
    }

    #[test]
    fn reconciliation_uses_confirmed_basis_during_tentative_scale_close_and_reversal() {
        for sign in [1i64, -1] {
            for state in [cc::OID_PENDING, cc::OID_LIQUIDATING] {
                for delta in [2 * sign, -sign, -3 * sign, -5 * sign] {
                    let p = cinder_ledger::Position {
                        asset_id: 1,
                        lots: 3 * sign + delta,
                        entry_quote_lots: 320_000_000 * sign,
                        ..Default::default()
                    };
                    let oid = cinder_ledger::OpenOid {
                        asset_id: 1,
                        lots_delta: delta,
                        state,
                        ..Default::default()
                    };
                    assert_eq!(confirmed_position(&p, &[oid]).unwrap(), 3 * sign);
                    let missing = cinder_ledger::Position {
                        entry_quote_lots: 0,
                        ..p
                    };
                    assert_eq!(
                        confirmed_position(&missing, &[oid]),
                        Err(RuntimeError::Identity)
                    );
                }
            }
        }
        let p = cinder_ledger::Position {
            asset_id: 1,
            lots: 10,
            ..Default::default()
        };
        let oid = cinder_ledger::OpenOid {
            asset_id: 1,
            lots_delta: 10,
            state: cc::OID_PENDING,
            ..Default::default()
        };
        assert_eq!(
            confirmed_position(&p, &[oid]).unwrap(),
            0,
            "a tentative first entry contributes neither confirmed lots nor basis"
        );
    }
}
