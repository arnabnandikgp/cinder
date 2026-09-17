//! Finalized Phoenix transaction evidence, not REST absence or WS delivery.
use crate::rpc::{array, string, Result, RuntimeError};
use crate::runtime::{text_signature, Shared};
use crate::transaction::{bytes, instruction_data, signature, Receipt};
use crate::{
    ErrorCode, FillFact, Operation, VenueObservation, VenueRecoveryPort, VenueSubmissionPort,
    VenueSubmitResult,
};
use phoenix_rise_events::market_events::OrderPacketKind;
use phoenix_rise_events::{parse_with_errors, MarketEvent, PhoenixLogInstruction};
use phoenix_rise_math::Side;
use serde_json::Value;
use sha2::{Digest, Sha256};
use solana_signer::Signer;
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "../tests/support/native_execution_proof.rs"]
mod native_execution_proof;

#[derive(Default)]
struct EvidenceCache {
    history: Option<Vec<[u8; 64]>>,
    receipts: BTreeMap<[u8; 64], Value>,
    bytes: usize,
}
impl EvidenceCache {
    fn reset(&mut self) {
        *self = Self::default();
    }
    fn transaction(
        &mut self,
        sig: [u8; 64],
        fetch: impl FnOnce() -> Result<Value>,
    ) -> Result<&Value> {
        if !self.receipts.contains_key(&sig) {
            let value = fetch()?; // Transport/RPC failures are never cached.
            let size = serde_json::to_vec(&value)
                .map_err(|_| RuntimeError::Decode)?
                .len();
            let bytes = self
                .bytes
                .checked_add(size)
                .ok_or(RuntimeError::Incomplete)?;
            if bytes > 64 * 1024 * 1024 {
                return Err(RuntimeError::Incomplete);
            }
            self.receipts.insert(sig, value);
            self.bytes = bytes;
        }
        self.receipts.get(&sig).ok_or(RuntimeError::Incomplete)
    }
}

pub(crate) struct RiseRecovery {
    context: Shared,
    evidence: EvidenceCache,
    prepared_funding: Option<(crate::FundingIntent, crate::transaction::SignedTransaction)>,
    enabled: bool,
    prepared_execution: Option<([u8; 32], crate::transaction::SignedTransaction)>,
}
impl RiseRecovery {
    pub fn new(context: Shared) -> Self {
        Self {
            context,
            evidence: EvidenceCache::default(),
            prepared_funding: None,
            enabled: false,
            prepared_execution: None,
        }
    }
    pub fn enable_execution(&mut self) {
        self.enabled = true;
    }
}
impl VenueSubmissionPort for RiseRecovery {
    fn submission_enabled(&self) -> bool {
        self.enabled
    }
    fn prepare_submission(
        &mut self,
        op: &Operation,
    ) -> std::result::Result<Option<crate::PreparedVenue>, ErrorCode> {
        if !self.enabled {
            return Err(ErrorCode::VenueUnavailable);
        }
        let result = (|| {
            let mut c = self.context.borrow_mut();
            let c = &mut *c;
            let (id, budget, view) = c
                .execution_admission
                .take()
                .ok_or(RuntimeError::Incomplete)?;
            if id != op.operation_id || Some(budget) != op.execution_budget {
                return Err(RuntimeError::Identity);
            }
            let cfg = c.vault_config()?;
            let map_key = c.config.phoenix_asset_map.clone();
            let (_, rows) = c.l1.accounts(std::slice::from_ref(&map_key), &c.signer)?;
            let market = crate::IocMarketAccounts::from_asset_map(
                &c.config,
                op.intent.asset_id,
                bytes(&map_key)?,
                bytes(string(&rows[0]["owner"])?)?,
                &crate::rpc::data(&rows[0], &c.config.phoenix_program)?,
            )?;
            let ioc = crate::build_bounded_ioc(
                &c.config,
                op,
                c.signer.pubkey().to_bytes(),
                &market,
                view.slot,
            )?;
            let (_, books) =
                c.l1.accounts(&[crate::runtime::text(&market.orderbook())], &c.signer)?;
            let data = crate::rpc::data(&books[0], &c.config.phoenix_program)?;
            let header = phoenix_rise_accounts::orderbook::Orderbook::try_from_account_bytes(&data)
                .map_err(|_| RuntimeError::Decode)?;
            let guard = cinder_vault::PhoenixExecutionGuard {
                snapshot_hash: view.snapshot_hash,
                ioc_data_hash: Sha256::digest(&ioc.data).into(),
                observed_slot: view.slot,
                oldest_observed_ms: view.mark_ms.min(view.observed_ms),
                taker_fee_counter: header.header().total_taker_quote_lot_fees.as_inner(),
                max_fee_usdc: budget.max_fee_usdc,
                gti_count: u8::try_from(c.config.global_trader_index.len())
                    .map_err(|_| RuntimeError::Configuration)?,
            };
            let instructions = crate::execution::guarded_ioc(
                &c.config,
                c.signer.pubkey().to_bytes(),
                &cfg,
                &market,
                ioc,
                guard,
            )?;
            let tx =
                crate::transaction::sign_native_instructions(&mut c.l1, &c.signer, instructions)?;
            crate::transaction::simulate(&mut c.l1, &c.signer, &tx, view.slot)?;
            let prepared = crate::PreparedVenue {
                signature: tx.signature,
                last_valid_block_height: tx.expiry,
            };
            self.prepared_execution = Some((op.operation_id, tx));
            Ok(Some(prepared))
        })();
        result.map_err(|_| ErrorCode::VenueUnavailable)
    }
    fn submit(&mut self, op: &Operation) -> std::result::Result<VenueSubmitResult, ErrorCode> {
        let (id, tx) = self
            .prepared_execution
            .take()
            .ok_or(ErrorCode::VenueUnavailable)?;
        if !self.enabled || id != op.operation_id || op.venue_signature != Some(tx.signature) {
            return Err(ErrorCode::CorrelationMismatch);
        }
        let mut c = self.context.borrow_mut();
        let c = &mut *c;
        crate::transaction::send_ioc(&mut c.l1, &c.signer, &tx)
            .map_err(|_| ErrorCode::VenueUnavailable)
    }
    fn prepare_funding(
        &mut self,
        operation: &Operation,
        intent: &crate::FundingIntent,
    ) -> std::result::Result<crate::PreparedVenue, ErrorCode> {
        if !self.submission_enabled() {
            return Err(ErrorCode::VenueUnavailable);
        }
        let result = (|| {
            let mut c = self.context.borrow_mut();
            let c = &mut *c;
            let cfg = c.vault_config()?;
            let assets = c.config.markets.iter().map(|m| m.cinder_asset_id).collect();
            let view = crate::rise::load(c, &assets)?;
            if operation.operation_id != intent.operation_id
                || view.slot > operation.intent.last_valid_slot
                || !view.safe
                || view.vault_balance < intent.amount
                || cinder_common::entries_blocked(view.halt & !cinder_common::OPERATOR_DOWN)
                || crate::rpc::unix_ms().saturating_sub(view.observed_ms)
                    > cinder_common::TRADER_STATE_STALE_MS
            {
                return Err(RuntimeError::Incomplete);
            }
            let global = c.config.phoenix_global_config.clone();
            let (_, rows) = c.l1.accounts(std::slice::from_ref(&global), &c.signer)?;
            let program = bytes(&c.config.phoenix_program)?;
            let accounts = crate::PhoenixFundingAccounts::from_global(
                &c.config,
                &cfg,
                bytes(&global)?,
                bytes(string(&rows[0]["owner"])?)?,
                &crate::rpc::data(&rows[0], &c.config.phoenix_program)?,
            )?;
            if intent.phoenix_program != program {
                return Err(RuntimeError::Identity);
            }
            let ix = crate::build_phoenix_funding(intent, c.signer.pubkey().to_bytes(), &accounts)?;
            let tx = crate::transaction::sign_native(&mut c.l1, &c.signer, ix)?;
            crate::transaction::simulate(&mut c.l1, &c.signer, &tx, view.slot)?;
            let attempt = crate::PreparedVenue {
                signature: tx.signature,
                last_valid_block_height: tx.expiry,
            };
            self.prepared_funding = Some((intent.clone(), tx));
            Ok(attempt)
        })();
        result.map_err(|_| ErrorCode::VenueUnavailable)
    }
    fn submit_funding(
        &mut self,
        record: &crate::FundingRecord,
    ) -> std::result::Result<VenueSubmitResult, ErrorCode> {
        let (intent, tx) = self
            .prepared_funding
            .take()
            .ok_or(ErrorCode::VenueUnavailable)?;
        if intent != record.intent
            || record.attempt.as_ref().is_none_or(|a| {
                a.signature != tx.signature || a.last_valid_block_height != tx.expiry
            })
        {
            return Err(ErrorCode::CorrelationMismatch);
        }
        let mut c = self.context.borrow_mut();
        let c = &mut *c;
        crate::transaction::send(&mut c.l1, &c.signer, &tx)
            .map(VenueSubmitResult::Accepted)
            .map_err(|_| ErrorCode::VenueUnavailable)
    }
}
impl VenueRecoveryPort for RiseRecovery {
    fn unsent_expired(&mut self, operation: &Operation) -> std::result::Result<bool, ErrorCode> {
        if operation.venue_signature.is_some() {
            return Err(ErrorCode::VenueUnavailable);
        }
        let mut c = self.context.borrow_mut();
        let c = &mut *c;
        c.l1.call(
            "getSlot",
            serde_json::json!([{"commitment":"confirmed"}]),
            &c.signer,
        )
        .and_then(|v| crate::rpc::number(&v))
        .map(|slot| slot > operation.intent.last_valid_slot)
        .map_err(|_| ErrorCode::VenueUnavailable)
    }
    fn begin_recovery(&mut self) {
        self.evidence.reset();
    }
    fn observe_funding(
        &mut self,
        record: &crate::FundingRecord,
    ) -> std::result::Result<crate::FundingObservation, ErrorCode> {
        let result = (|| {
            let Some(attempt) = &record.attempt else {
                return Ok(crate::FundingObservation::Unknown);
            };
            let mut c = self.context.borrow_mut();
            let c = &mut *c;
            let address = solana_pubkey::Pubkey::find_program_address(
                &[
                    cinder_vault::SEED_PHOENIX_FUNDING,
                    &record.intent.funding_id,
                ],
                &solana_pubkey::Pubkey::new_from_array(cinder_vault::ID.to_bytes()),
            )
            .0
            .to_bytes();
            let value=c.l1.call("getAccountInfo",serde_json::json!([crate::runtime::text(&address),{"encoding":"base64","commitment":"finalized"}]),&c.signer)?;
            if !value["value"].is_null() {
                let row = &value["value"];
                let receipt = crate::decode_phoenix_funding_receipt(
                    address,
                    bytes(string(&row["owner"])?)?,
                    &crate::rpc::data(row, &cinder_vault::ID.to_string())?,
                    &record.intent,
                )?;
                if crate::rpc::number(&value["context"]["slot"])? < receipt.funded_at_slot {
                    return Err(RuntimeError::Stale);
                }
                return Ok(crate::FundingObservation::Funded(receipt));
            }
            let value =
                c.l1.transaction(&text_signature(&attempt.signature), &c.signer, true)?;
            if value.is_null() {
                return Ok(crate::FundingObservation::Unknown);
            }
            let cfg = c.vault_config()?;
            let global = c.config.phoenix_global_config.clone();
            let (_, rows) = c.l1.accounts(std::slice::from_ref(&global), &c.signer)?;
            let accounts = crate::PhoenixFundingAccounts::from_global(
                &c.config,
                &cfg,
                bytes(&global)?,
                bytes(string(&rows[0]["owner"])?)?,
                &crate::rpc::data(&rows[0], &c.config.phoenix_program)?,
            )?;
            let expected = crate::build_phoenix_funding(
                &record.intent,
                c.signer.pubkey().to_bytes(),
                &accounts,
            )?;
            decode_funding_failure(&value, &attempt.signature, &expected)
        })();
        result.map_err(|_| ErrorCode::VenueUnavailable)
    }
    fn observe(&mut self, op: &Operation) -> std::result::Result<VenueObservation, ErrorCode> {
        let result = (|| {
            let mut c = self.context.borrow_mut();
            let c = &mut *c;
            let trader = bytes(&c.config.phoenix_trader)?;
            let program = bytes(&c.config.phoenix_program)?;
            let operator = c.signer.pubkey().to_bytes();
            let native_asset = c.config.market(op.intent.asset_id)?.phoenix_asset_id;
            if let Some(sig) = op.venue_signature {
                let value = self.evidence.transaction(sig, || {
                    c.l1.transaction(&text_signature(&sig), &c.signer, true)
                })?;
                if value.is_null() {
                    return Ok(VenueObservation::Unknown);
                }
                if !Receipt::decode(value, &sig)?.succeeded && op.execution_budget.is_some() {
                    if op.filled_lots != 0 {
                        return Err(RuntimeError::Identity);
                    }
                    let map = c.config.phoenix_asset_map.clone();
                    let (_, rows) = c.l1.accounts(std::slice::from_ref(&map), &c.signer)?;
                    let market = crate::IocMarketAccounts::from_asset_map(
                        &c.config,
                        op.intent.asset_id,
                        bytes(&map)?,
                        bytes(string(&rows[0]["owner"])?)?,
                        &crate::rpc::data(&rows[0], &c.config.phoenix_program)?,
                    )?;
                    let mut unsent = op.clone();
                    unsent.state = crate::OrderState::Prepared;
                    unsent.venue_signature = None;
                    unsent.filled_lots = 0;
                    let expected =
                        crate::build_bounded_ioc(&c.config, &unsent, operator, &market, 1)?;
                    return decode_failed_ioc(value, &sig, &expected);
                }
                return decode_outcome(
                    op,
                    &trader,
                    &program,
                    &operator,
                    native_asset,
                    &sig,
                    value,
                )?
                .ok_or(RuntimeError::Identity);
            }
            let mut found = None;
            // No client-ID filter exists in Rise REST order history. Join
            // native packets/events in authoritative pooled-trader receipts.
            if self.evidence.history.is_none() {
                self.evidence.history = Some(
                    c.l1.history(&c.config.phoenix_trader, &c.signer)?
                        .iter()
                        .map(|row| signature(string(&row["signature"])?))
                        .collect::<Result<Vec<_>>>()?,
                );
            }
            for sig in self
                .evidence
                .history
                .clone()
                .ok_or(RuntimeError::Incomplete)?
            {
                let value = self.evidence.transaction(sig, || {
                    c.l1.transaction(&text_signature(&sig), &c.signer, true)
                })?;
                if value.is_null() {
                    continue;
                }
                if let Some(outcome) =
                    decode_outcome(op, &trader, &program, &operator, native_asset, &sig, value)?
                {
                    // Two executions with the same global ID are an incident,
                    // not alternative candidate receipts to choose between.
                    if found.is_some() {
                        return Err(RuntimeError::Identity);
                    }
                    found = Some(outcome);
                }
            }
            // Only a pristine WAL state proves we have not crossed the native
            // write-ahead boundary. This is NOT proof from REST/history absence:
            // receipt discovery above still wins, and old submission intents,
            // signed attempts and ambiguous outcomes can never use this path.
            let unsent = self.enabled && pristine_wal(op);
            Ok(found.unwrap_or(if unsent {
                VenueObservation::DefinitelyNeverSubmitted
            } else {
                VenueObservation::Unknown
            }))
        })();
        result.map_err(|e| {
            if e == RuntimeError::Identity {
                ErrorCode::CorrelationMismatch
            } else {
                ErrorCode::VenueUnavailable
            }
        })
    }
}

fn pristine_wal(op: &Operation) -> bool {
    op.state == crate::OrderState::Prepared && op.venue_signature.is_none() && op.filled_lots == 0
}

/// A failed finalized transaction rolls back even when the before fence failed
/// before Phoenix could emit any packet logs. Authenticate the exact native
/// instruction instead of treating missing logs as absence or as a fill.
fn decode_failed_ioc(
    value: &Value,
    sig: &[u8; 64],
    expected: &solana_instruction::Instruction,
) -> Result<VenueObservation> {
    let receipt = Receipt::decode(value, sig)?;
    if receipt.succeeded || receipt.slot == 0 {
        return Err(RuntimeError::Identity);
    }
    let native: Vec<_> = receipt
        .instructions
        .iter()
        .filter(|i| receipt.program(i).ok() == Some(expected.program_id.to_bytes()))
        .collect();
    if native.len() != 1
        || instruction_data(native[0])? != expected.data
        || array(&native[0]["accounts"])?.len() != expected.accounts.len()
    {
        return Err(RuntimeError::Identity);
    }
    for (index, meta) in expected.accounts.iter().enumerate() {
        if receipt.account(native[0], index)? != meta.pubkey.to_bytes()
            || (meta.is_signer && !receipt.requires_signature(&meta.pubkey.to_bytes()))
        {
            return Err(RuntimeError::Identity);
        }
    }
    Ok(VenueObservation::Rejected { fills: vec![] })
}

fn decode_funding_failure(
    value: &Value,
    sig: &[u8; 64],
    expected: &solana_instruction::Instruction,
) -> Result<crate::FundingObservation> {
    let receipt = Receipt::decode(value, sig)?;
    let mut matching = 0;
    for ix in &receipt.instructions {
        if receipt.program(ix)? != expected.program_id.to_bytes() {
            continue;
        }
        let raw = instruction_data(ix)?;
        if crate::transaction::decode_ix::<cinder_vault::instruction::FundPhoenix>(&raw)?.is_none()
        {
            continue;
        }
        if raw != expected.data || array(&ix["accounts"])?.len() != expected.accounts.len() {
            return Err(RuntimeError::Identity);
        }
        for (index, meta) in expected.accounts.iter().enumerate() {
            if receipt.account(ix, index)? != meta.pubkey.to_bytes()
                || (meta.is_signer && !receipt.requires_signature(&meta.pubkey.to_bytes()))
            {
                return Err(RuntimeError::Identity);
            }
        }
        matching += 1;
    }
    if matching != 1 || receipt.slot == 0 {
        return Err(RuntimeError::Identity);
    }
    // A successful funding transaction without its canonical finalized receipt
    // is incomplete evidence, never a failure or permission to repeat it.
    Ok(if receipt.succeeded {
        crate::FundingObservation::Unknown
    } else {
        crate::FundingObservation::Rejected { slot: receipt.slot }
    })
}

pub(crate) fn decode_outcome(
    op: &Operation,
    trader: &[u8; 32],
    program: &[u8; 32],
    operator: &[u8; 32],
    native_asset: u32,
    sig: &[u8; 64],
    value: &Value,
) -> Result<Option<VenueObservation>> {
    let receipt = Receipt::decode(value, sig)?;
    if value["meta"]["innerInstructions"].is_null() {
        return Ok(None);
    }
    let groups = array(&value["meta"]["innerInstructions"])?;
    let mut found = None;
    for group in groups {
        let index = usize::try_from(crate::rpc::number(&group["index"])?)
            .map_err(|_| RuntimeError::Decode)?;
        let top = receipt
            .instructions
            .get(index)
            .ok_or(RuntimeError::Decode)?;
        let top_program = receipt.program(top)?;
        // Ember's broker instruction invokes Phoenix; all financial facts
        // below still come from Phoenix's own authenticated log instructions.
        if top_program != *program
            && top_program != bytes("EMBERpYNE6ehWmXymZZS2skiFmCa9V5dp14e1iduM5qy")?
        {
            continue;
        }
        let mut raw = Vec::new();
        for ix in array(&group["instructions"])? {
            if receipt.program(ix)? == *program {
                let data = instruction_data(ix)?;
                if PhoenixLogInstruction::from_instruction_data(&data).is_some() {
                    raw.push(data);
                }
            }
        }
        let parsed = parse_with_errors(
            raw.iter()
                .filter_map(|b| PhoenixLogInstruction::from_instruction_data(b)),
        );
        if !parsed.failures.is_empty() || !parsed.skipped_event_bytes.is_empty() {
            return Err(RuntimeError::Decode);
        }
        let packets = parsed
            .events
            .iter()
            .filter_map(|e| {
                if let MarketEvent::OrderPacket(p) = e {
                    Some(p)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let matches = packets.iter().any(|p| match p.order_packet.kind {
            OrderPacketKind::ImmediateOrCancel {
                client_order_id, ..
            }
            | OrderPacketKind::Limit {
                client_order_id, ..
            }
            | OrderPacketKind::PostOnly {
                client_order_id, ..
            } => client_order_id == op.venue_identity.venue_oid,
        });
        if !matches {
            continue;
        }
        if found.is_some() || packets.len() != 1 {
            return Err(RuntimeError::Identity);
        }
        let packet = packets[0];
        let headers = parsed
            .events
            .iter()
            .filter_map(|e| {
                if let MarketEvent::Header(h) = e {
                    Some(h)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if headers.len() != 1 {
            return Err(RuntimeError::Incomplete);
        }
        let header = headers[0];
        if packet.trader != *trader
            || header.trader_account != *trader
            || header.asset_id != native_asset
            || header.quote_lot_decimals != 6
            || header.tick_size == 0
            || header.signer != *operator
            || !receipt.requires_signature(&header.signer)
        {
            return Err(RuntimeError::Identity);
        }
        let side = if op.intent.requested_lots > 0 {
            Side::Bid
        } else {
            Side::Ask
        };
        match packet.order_packet.kind {
            OrderPacketKind::ImmediateOrCancel {
                side: s,
                price_in_ticks,
                num_base_lots,
                num_quote_lots,
                client_order_id,
                last_valid_slot,
                ..
            } => {
                if s != side
                    || price_in_ticks.map(|p| p.as_inner()) != Some(op.intent.limit_price_ticks)
                    || num_base_lots.as_inner() != op.intent.requested_lots.unsigned_abs()
                    || num_quote_lots.map(|q| q.as_inner())
                        != op.execution_budget.map(|budget| budget.max_quote_lots)
                    || client_order_id != op.venue_identity.venue_oid
                    || last_valid_slot != Some(op.intent.last_valid_slot)
                {
                    return Err(RuntimeError::Identity);
                }
            }
            _ => return Err(RuntimeError::Unsupported),
        }
        if !receipt.succeeded {
            found = Some(VenueObservation::Rejected { fills: vec![] });
            continue;
        }
        let summaries = parsed
            .events
            .iter()
            .filter_map(|e| {
                if let MarketEvent::TradeSummary(s) = e {
                    Some(s)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let mut lots = 0u64;
        let mut quote = 0u64;
        let mut worst = if side == Side::Bid { 0 } else { u64::MAX };
        for event in &parsed.events {
            let fill = match event {
                MarketEvent::OrderFilled(f) => Some((
                    f.side,
                    f.price.as_inner(),
                    f.base_lots_filled.as_inner(),
                    f.quote_lots_filled.as_inner(),
                )),
                MarketEvent::SplineFilled(f) => Some((
                    f.side,
                    f.price.as_inner(),
                    f.base_lots_filled.as_inner(),
                    f.quote_lots_filled.as_inner(),
                )),
                _ => None,
            };
            if let Some((maker_side, price, n, q)) = fill {
                if maker_side == side || price == 0 || n == 0 {
                    return Err(RuntimeError::Identity);
                }
                lots = lots.checked_add(n).ok_or(RuntimeError::Decode)?;
                quote = quote.checked_add(q).ok_or(RuntimeError::Decode)?;
                worst = if side == Side::Bid {
                    worst.max(price)
                } else {
                    worst.min(price)
                };
            }
        }
        if summaries.is_empty() && lots == 0 {
            // The exact IOC completed successfully and cannot leave a resting
            // remainder. This is transaction evidence, not lookup absence.
            found = Some(VenueObservation::Rejected { fills: vec![] });
            continue;
        }
        if summaries.len() != 1 {
            return Err(RuntimeError::Incomplete);
        }
        let summary = summaries[0];
        if summary.trader != *trader
            || summary.side != side
            || summary.base_lots_filled.as_inner() != lots
            || summary.quote_lots_filled.as_inner() != quote
            || lots > op.intent.requested_lots.unsigned_abs()
        {
            return Err(RuntimeError::Identity);
        }
        if lots == 0 {
            found = Some(VenueObservation::Rejected { fills: vec![] });
            continue;
        }
        let slots = parsed
            .events
            .iter()
            .filter_map(|e| {
                if let MarketEvent::SlotContext(s) = e {
                    Some(s)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if slots.len() != 1 || slots[0].slot != receipt.slot {
            return Err(RuntimeError::Incomplete);
        }
        let time = slots[0]
            .timestamp
            .checked_mul(1000)
            .ok_or(RuntimeError::Decode)?;
        let direction = op.intent.requested_lots.signum();
        let lots = i64::try_from(lots)
            .map_err(|_| RuntimeError::Decode)?
            .checked_mul(direction)
            .ok_or(RuntimeError::Decode)?;
        let quote = i64::try_from(quote)
            .map_err(|_| RuntimeError::Decode)?
            .checked_mul(direction)
            .ok_or(RuntimeError::Decode)?;
        let mut hash = Sha256::new();
        hash.update(b"cinder:rise:trade-summary:v1");
        hash.update(sig);
        hash.update((index as u64).to_le_bytes());
        hash.update(summary.trade_sequence_number.to_le_bytes());
        let fact = FillFact {
            event_id: hash.finalize().into(),
            filled_lots: lots,
            vwap_quote_lots: quote,
            fee_usdc: summary.fee_in_quote_lots.as_inner(),
            fill_price_ticks: worst,
            observed_at_ms: time,
        };
        found = Some(VenueObservation::Filled { fills: vec![fact] });
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_native_transaction_needs_exact_packet_not_fill_logs() {
        let sig = [8; 64];
        let ix = crate::transaction::anchor_ix(
            [3; 32],
            vec![([5; 32], true, false), ([6; 32], false, true)],
            vec![1, 2, 3],
        );
        let failed = crate::transaction::test_receipt(&ix, &sig, false);
        assert_eq!(
            decode_failed_ioc(&failed, &sig, &ix).unwrap(),
            VenueObservation::Rejected { fills: vec![] }
        );
        assert!(decode_failed_ioc(&failed, &[9; 64], &ix).is_err());
        assert!(decode_failed_ioc(
            &crate::transaction::test_receipt(&ix, &sig, true),
            &sig,
            &ix
        )
        .is_err());
        let mut wrong = ix.clone();
        wrong.data.push(4);
        assert!(decode_failed_ioc(&failed, &sig, &wrong).is_err());
        wrong = ix.clone();
        wrong.accounts[1].pubkey = solana_pubkey::Pubkey::new_from_array([7; 32]);
        assert!(decode_failed_ioc(&failed, &sig, &wrong).is_err());
        let mut duplicate = failed.clone();
        duplicate["transaction"]["message"]["instructions"]
            .as_array_mut()
            .unwrap()
            .push(failed["transaction"]["message"]["instructions"][0].clone());
        assert!(decode_failed_ioc(&duplicate, &sig, &ix).is_err());
    }

    #[test]
    fn pristine_wal_proof_never_applies_to_old_intents_or_signed_unknowns() {
        let (_, mut op, _) = fixture();
        assert!(pristine_wal(&op));
        for state in [
            crate::OrderState::SubmissionIntent,
            crate::OrderState::Submitted,
            crate::OrderState::Reconciling,
            crate::OrderState::VenueOpen,
        ] {
            op.state = state;
            assert!(!pristine_wal(&op));
        }
        op.state = crate::OrderState::Prepared;
        op.venue_signature = Some([8; 64]);
        assert!(!pristine_wal(&op));
        op.venue_signature = None;
        op.filled_lots = 1;
        assert!(!pristine_wal(&op));
    }

    #[test]
    fn finalized_funding_failure_requires_exact_signed_custody_instruction() {
        use anchor_lang::InstructionData;
        let adapter = [7; 32];
        let sig = [8; 64];
        let ix = crate::transaction::anchor_ix(
            cinder_vault::ID.to_bytes(),
            vec![(adapter, true, true), ([9; 32], false, true)],
            cinder_vault::instruction::FundPhoenix {
                funding_id: [10; 32],
                amount: 50_000_000,
                global_trader_index_count: 1,
            }
            .data(),
        );
        let failed = crate::transaction::test_receipt(&ix, &sig, false);
        assert!(matches!(
            decode_funding_failure(&failed, &sig, &ix).unwrap(),
            crate::FundingObservation::Rejected { slot: 100 }
        ));
        let success = crate::transaction::test_receipt(&ix, &sig, true);
        assert!(matches!(
            decode_funding_failure(&success, &sig, &ix).unwrap(),
            crate::FundingObservation::Unknown
        ));
        assert!(decode_funding_failure(&failed, &[9; 64], &ix).is_err());
        let mut wrong = ix.clone();
        wrong.accounts[1].pubkey = solana_pubkey::Pubkey::new_from_array([11; 32]);
        assert!(decode_funding_failure(&failed, &sig, &wrong).is_err());
        let mut wrong = ix.clone();
        wrong.data = cinder_vault::instruction::FundPhoenix {
            funding_id: [10; 32],
            amount: 50_000_001,
            global_trader_index_count: 1,
        }
        .data();
        assert!(decode_funding_failure(&failed, &sig, &wrong).is_err());
        let mut duplicate = failed.clone();
        duplicate["transaction"]["message"]["instructions"]
            .as_array_mut()
            .unwrap()
            .push(failed["transaction"]["message"]["instructions"][0].clone());
        assert!(decode_funding_failure(&duplicate, &sig, &ix).is_err());
    }
    use crate::venue_fixture::{receipt, VenueFixture};
    use crate::{BoundedIntent, Journal, OrderIdentity, OrderKind};

    #[test]
    fn receipt_cache_is_pass_scoped_and_does_not_cache_failed_requests() {
        let mut cache = EvidenceCache::default();
        let mut calls = 0;
        for _ in 0..2 {
            assert_eq!(
                cache
                    .transaction([1; 64], || {
                        calls += 1;
                        Ok(serde_json::json!({"receipt": 1}))
                    })
                    .unwrap()["receipt"],
                1
            );
        }
        assert_eq!(calls, 1);
        assert_eq!(
            cache.transaction([2; 64], || Err(RuntimeError::Rpc)),
            Err(RuntimeError::Rpc)
        );
        assert!(cache
            .transaction([2; 64], || Ok(Value::Null))
            .unwrap()
            .is_null());
        cache.history = Some(vec![[1; 64]]);
        cache.reset();
        assert!(cache.history.is_none());
        assert!(cache.receipts.is_empty());
        assert_eq!(cache.bytes, 0);
        assert_eq!(
            cache
                .transaction([1; 64], || Ok(serde_json::json!({"receipt": 2})))
                .unwrap()["receipt"],
            2
        );
        cache.bytes = 64 * 1024 * 1024;
        assert_eq!(
            cache.transaction([3; 64], || Ok(Value::Null)),
            Err(RuntimeError::Incomplete)
        );
    }

    #[test]
    fn unrecorded_inner_instructions_are_absent_evidence_not_a_decode_failure() {
        let (_, op, f) = fixture();
        let mut raw = receipt(&f);
        raw["meta"]["innerInstructions"] = Value::Null;
        assert_eq!(
            decode_outcome(
                &op,
                &bytes(&f.trader).unwrap(),
                &bytes(&f.program).unwrap(),
                &[5; 32],
                0,
                &signature(&f.signature).unwrap(),
                &raw
            ),
            Ok(None)
        );
        raw["meta"]["innerInstructions"] = serde_json::json!("malformed");
        assert_eq!(
            decode_outcome(
                &op,
                &bytes(&f.trader).unwrap(),
                &bytes(&f.program).unwrap(),
                &[5; 32],
                0,
                &signature(&f.signature).unwrap(),
                &raw
            ),
            Err(RuntimeError::Decode)
        );
    }
    fn fixture() -> (tempfile::TempDir, Operation, VenueFixture) {
        let dir = tempfile::tempdir().unwrap();
        let mut journal = Journal::open(dir.path().join("private/journal.sqlite")).unwrap();
        let op = journal
            .prepare_intent(BoundedIntent {
                identity: OrderIdentity {
                    user_ledger: [1; 32],
                    user_pubkey: [2; 32],
                    user_nonce: 4,
                    client_oid: [3; 16],
                    kind: OrderKind::User,
                },
                asset_id: 1,
                requested_lots: 10,
                limit_price_ticks: 8000,
                last_valid_slot: u64::MAX,
                post_fail_position_im_usdc: 0,
                created_at_ms: 0,
            })
            .unwrap();
        let f = VenueFixture {
            trader: bs58::encode([4; 32]).into_string(),
            operator: bs58::encode([5; 32]).into_string(),
            program: phoenix_rise_ix::constants::PROD_PHOENIX_PROGRAM_ID.to_string(),
            oid: op.venue_identity.venue_oid,
            native_asset_id: 0,
            lots: 10,
            filled_lots: None,
            max_quote_lots: None,
            ticks: 7191,
            bound: 8000,
            deadline: u64::MAX,
            fee: 7,
            time: 1234,
            slot: 88,
            signature: bs58::encode([6; 64]).into_string(),
            rejected: false,
        };
        (dir, op, f)
    }
    fn decode(op: &Operation, f: &VenueFixture) -> Result<Option<VenueObservation>> {
        decode_outcome(
            op,
            &bytes(&f.trader).unwrap(),
            &bytes(&f.program).unwrap(),
            &[5; 32],
            0,
            &signature(&f.signature).unwrap(),
            &receipt(f),
        )
    }
    #[test]
    fn sdk_receipts_prove_native_identity_all_fills_fee_and_stable_replay() {
        let (dir, op, f) = fixture();
        let VenueObservation::Filled { fills } = decode(&op, &f).unwrap().unwrap() else {
            panic!("expected full venue facts")
        };
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].filled_lots, 10);
        assert_eq!(fills[0].vwap_quote_lots, 7191000);
        assert_eq!(fills[0].fee_usdc, 7);
        assert_eq!(fills[0].observed_at_ms, 1234000);
        let mut journal = Journal::open(dir.path().join("private/journal.sqlite")).unwrap();
        journal
            .record_fill_facts(&op.operation_id, &fills, 1000)
            .unwrap();
        journal
            .record_fill_facts(&op.operation_id, &fills, 2000)
            .unwrap();
        assert_eq!(journal.operation(&op.operation_id).unwrap().fee_usdc, 7);
    }
    #[test]
    fn wrong_native_mapping_bounds_side_or_deadline_cannot_be_recovery_evidence() {
        let (_, op, f) = fixture();
        for mutation in 0..4 {
            let mut altered = f.clone();
            match mutation {
                0 => altered.native_asset_id = 1,
                1 => altered.bound = 7999,
                2 => altered.lots = -10,
                _ => altered.deadline = 99,
            };
            assert_eq!(decode(&op, &altered), Err(RuntimeError::Identity));
        }
        let mut other = f.clone();
        other.oid = [9; 16];
        assert_eq!(decode(&op, &other).unwrap(), None);
        let mut foreign = f.clone();
        foreign.operator = bs58::encode([8; 32]).into_string();
        assert_eq!(decode(&op, &foreign), Err(RuntimeError::Identity));
    }
    #[test]
    fn terminal_partial_ioc_receipt_preserves_actual_quantity_quote_and_fee() {
        for lots in [4i64, -4] {
            let (_, mut op, mut f) = fixture();
            op.intent.requested_lots = 10 * lots.signum();
            f.lots = op.intent.requested_lots;
            f.filled_lots = Some(lots);
            let Some(VenueObservation::Filled { fills }) = decode(&op, &f).unwrap() else {
                panic!("expected terminal partial IOC facts")
            };
            assert_eq!(fills.len(), 1);
            assert_eq!(fills[0].filled_lots, lots);
            assert_eq!(fills[0].vwap_quote_lots, lots * 7191 * 100);
            assert_eq!(fills[0].fee_usdc, 7);
            f.filled_lots = Some(11 * lots.signum());
            assert_eq!(decode(&op, &f), Err(RuntimeError::Identity));
        }
    }

    #[test]
    fn native_receipt_requires_the_exact_persisted_quote_budget_for_both_sides() {
        for lots in [10, -10] {
            let (dir, op, mut fixture) = fixture();
            let mut journal = Journal::open(dir.path().join("private/journal.sqlite")).unwrap();
            // Use a distinct immutable sell intent, rather than mutate a
            // journal projection's identity after preparation.
            let mut intent = op.intent.clone();
            intent.identity.user_nonce += 1;
            intent.requested_lots = lots;
            let op = journal.prepare_intent(intent).unwrap();
            let budget = crate::ExecutionBudget {
                max_quote_lots: 4 * 7191 * 100,
                max_fee_usdc: 7,
            };
            let op = journal
                .record_execution_budget(&op.operation_id, budget)
                .unwrap();
            fixture.oid = op.venue_identity.venue_oid;
            fixture.lots = lots;
            fixture.filled_lots = Some(4 * lots.signum());
            fixture.max_quote_lots = Some(budget.max_quote_lots);
            let Some(VenueObservation::Filled { fills }) = decode(&op, &fixture).unwrap() else {
                panic!("capped partial fill must remain a terminal fill")
            };
            assert_eq!(fills[0].filled_lots, 4 * lots.signum());
            assert_eq!(fills[0].fee_usdc, 7);
            journal
                .record_fill_facts(&op.operation_id, &fills, 1)
                .unwrap();
            journal.record_venue_filled(&op.operation_id, 1).unwrap();
            journal.begin_fill_ack(&op.operation_id, 1).unwrap();
            assert!(!journal.has_execution_budget_breach().unwrap());
            for cap in [
                None,
                Some(budget.max_quote_lots - 1),
                Some(budget.max_quote_lots + 1),
            ] {
                fixture.max_quote_lots = cap;
                assert_eq!(decode(&op, &fixture), Err(RuntimeError::Identity));
            }
            fixture.max_quote_lots = Some(budget.max_quote_lots);
            fixture.rejected = true;
            assert_eq!(
                decode(&op, &fixture).unwrap(),
                Some(VenueObservation::Rejected { fills: vec![] })
            );
        }
    }
    #[test]
    fn zero_fill_ioc_has_terminal_receipt_but_bad_or_duplicate_logs_do_not() {
        let (_, op, mut f) = fixture();
        f.rejected = true;
        assert_eq!(
            decode(&op, &f).unwrap(),
            Some(VenueObservation::Rejected { fills: vec![] })
        );
        let mut raw = receipt(&f);
        // Preserve the log tag so this exercises malformed log decoding,
        // rather than filtering an instruction that is not a recognized log.
        let mut malformed = phoenix_rise_events::LOG_INSTRUCTION_TAG
            .to_le_bytes()
            .to_vec();
        malformed.extend([1, 2, 3]);
        raw["meta"]["innerInstructions"][0]["instructions"][1]["data"] =
            serde_json::json!(bs58::encode(malformed).into_string());
        assert_eq!(
            decode_outcome(
                &op,
                &bytes(&f.trader).unwrap(),
                &bytes(&f.program).unwrap(),
                &[5; 32],
                0,
                &signature(&f.signature).unwrap(),
                &raw
            ),
            Err(RuntimeError::Decode)
        );
        // Missing recognizable evidence must not become a rejection proof.
        raw["meta"]["innerInstructions"][0]["instructions"][1]["data"] =
            serde_json::json!(bs58::encode([1, 2, 3]).into_string());
        assert_eq!(
            decode_outcome(
                &op,
                &bytes(&f.trader).unwrap(),
                &bytes(&f.program).unwrap(),
                &[5; 32],
                0,
                &signature(&f.signature).unwrap(),
                &raw
            ),
            Ok(None)
        );
        let mut raw = receipt(&f);
        let g = raw["meta"]["innerInstructions"][0].clone();
        raw["meta"]["innerInstructions"]
            .as_array_mut()
            .unwrap()
            .push(g);
        assert_eq!(
            decode_outcome(
                &op,
                &bytes(&f.trader).unwrap(),
                &bytes(&f.program).unwrap(),
                &[5; 32],
                0,
                &signature(&f.signature).unwrap(),
                &raw
            ),
            Err(RuntimeError::Identity)
        );
    }
    #[test]
    fn confirmed_execution_breach_is_preserved_not_erased() {
        let (_, op, mut f) = fixture();
        f.ticks = 8100;
        let Some(VenueObservation::Filled { fills }) = decode(&op, &f).unwrap() else {
            panic!("confirmed breach must be acknowledged")
        };
        assert_eq!(fills[0].fill_price_ticks, 8100);
    }
}
