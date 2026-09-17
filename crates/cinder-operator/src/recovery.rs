use std::collections::BTreeMap;

use cinder_common as cc;

use crate::{
    BoundedIntent, ErrorCode, FillFact, Journal, JournalError, Operation, OperationId,
    OrderIdentity, OrderState, VenueObservation,
};

/// Production implementations must use history/transaction evidence, not an
/// absent lookup result or a local timeout, for DefinitelyNeverSubmitted.
pub trait VenueRecoveryPort {
    /// Discard cached evidence before each pass so newly finalized outcomes
    /// and previously failed RPC requests are observed again.
    fn begin_recovery(&mut self) {}
    /// Verify the exact global venue ID, pooled trader, asset, and direction
    /// before returning facts. Per-user client IDs are never a venue lookup key.
    fn observe(&mut self, operation: &Operation) -> Result<VenueObservation, ErrorCode>;
    /// Called only after authoritative never-submitted evidence. An expired
    /// unsent IOC can be fail-acked; unknown signed attempts cannot.
    fn unsent_expired(&mut self, _operation: &Operation) -> Result<bool, ErrorCode> {
        Ok(false)
    }
    fn observe_funding(
        &mut self,
        _record: &crate::FundingRecord,
    ) -> Result<FundingObservation, ErrorCode> {
        Ok(FundingObservation::Unknown)
    }
}

pub enum FundingObservation {
    Unknown,
    /// Authenticated, canonical, finalized vault receipt.
    Funded(cinder_vault::PhoenixFundingReceipt),
    /// Authenticated finalized failure of the exact signed funding transaction.
    Rejected {
        slot: u64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BookSyncObservation {
    Unknown,
    Applied { slot: u64 },
    Rejected { slot: u64 },
}

pub enum VenueSubmitResult {
    Accepted([u8; 64]),
    /// Authenticated RPC preflight failure before this sole broadcast attempt.
    PreflightRejected,
    Unknown,
}

pub trait VenueSubmissionPort {
    /// Recovery-only ports cannot open new venue risk. Disabling submission
    /// must not create a spurious ambiguous send.
    fn submission_enabled(&self) -> bool {
        true
    }
    /// Validate fresh risk and prepare the exact bounded IOC without sending.
    /// Production execution ports return its signed identity for write-ahead
    /// persistence. The default supports abstract, non-network test ports.
    fn prepare_submission(
        &mut self,
        _operation: &Operation,
    ) -> Result<Option<crate::PreparedVenue>, ErrorCode> {
        Ok(None)
    }
    /// Submit exactly the persisted venue ID, direction, integer bound and
    /// L1 deadline. Revalidate fresh risk and expiry before sending.
    fn submit(&mut self, operation: &Operation) -> Result<VenueSubmitResult, ErrorCode>;
    fn prepare_funding(
        &mut self,
        _operation: &Operation,
        _intent: &crate::FundingIntent,
    ) -> Result<crate::PreparedVenue, ErrorCode> {
        Err(ErrorCode::VenueUnavailable)
    }
    /// Only the prepared, journaled identity may be broadcast, once.
    fn submit_funding(
        &mut self,
        _record: &crate::FundingRecord,
    ) -> Result<VenueSubmitResult, ErrorCode> {
        Err(ErrorCode::VenueUnavailable)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErAckCommand {
    Fill {
        intent: BoundedIntent,
        fills: Vec<FillFact>,
    },
    Fail {
        intent: BoundedIntent,
    },
}

pub enum ErAckSubmitResult {
    Accepted([u8; 64]),
    Unknown,
}

pub trait ErAckSubmissionPort {
    /// Prepare an acknowledgement without broadcasting it. Production ports
    /// must return its identity so it can be persisted before submission.
    fn prepare_ack(
        &mut self,
        _command: &ErAckCommand,
    ) -> Result<Option<crate::PreparedAck>, ErrorCode> {
        Ok(None)
    }
    /// Calculate fresh post-transition margin through the shared risk boundary.
    /// Aggregate only the persisted facts and acknowledge this full identity.
    /// Retries must not mutate a later order that reused the client OID.
    fn submit_ack(&mut self, command: &ErAckCommand) -> Result<ErAckSubmitResult, ErrorCode>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AckFinality {
    Unknown,
    Pending,
    /// Proof that no acknowledgement transaction can still land. A pending
    /// OpenOid by itself is insufficient: an earlier transaction may be live.
    DefinitelyNotApplied,
    Filled {
        lots: i64,
        quote_lots: i64,
        fee_usdc: u64,
    },
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AckObservation {
    pub identity: OrderIdentity,
    pub operation_id: OperationId,
    pub finality: AckFinality,
}

pub trait LedgerRecoveryPort {
    /// Attest the full operation with authoritative receipts/history. Current
    /// OpenOid alone cannot prove a nonce, and old ACKED rows are not proof.
    fn observe_ack(&mut self, operation: &Operation) -> Result<AckObservation, ErrorCode>;
    fn observe_ack_attempts(
        &mut self,
        operation: &Operation,
        _attempts: &[crate::PreparedAck],
    ) -> Result<AckObservation, ErrorCode> {
        self.observe_ack(operation)
    }
    fn reconciliation(&mut self) -> Result<ReconciliationSnapshot, ErrorCode>;
    /// Fresh post-intent user, pool and stressed backing checks, including
    /// confirmed collateral and partial-fill paths. Recovery success is not
    /// admission proof. Ports without this implementation cannot dispatch.
    fn check_admission(&mut self, _operation: &Operation) -> Result<(), ErrorCode> {
        Err(ErrorCode::ReconciliationUnavailable)
    }
    fn plan_execution_budget(
        &mut self,
        _operation: &Operation,
    ) -> Result<Option<crate::ExecutionBudget>, ErrorCode> {
        Ok(None)
    }
    fn funding_required(
        &mut self,
        _operation: &Operation,
    ) -> Result<Option<crate::FundingIntent>, ErrorCode> {
        Ok(None)
    }
    fn prepare_funding_book_sync(
        &mut self,
        _record: &crate::FundingRecord,
    ) -> Result<crate::PreparedBookSync, ErrorCode> {
        Err(ErrorCode::LedgerUnavailable)
    }
    fn submit_funding_book_sync(
        &mut self,
        _prepared: &crate::PreparedBookSync,
    ) -> Result<ErAckSubmitResult, ErrorCode> {
        Err(ErrorCode::LedgerUnavailable)
    }
    fn observe_funding_book_sync(
        &mut self,
        _prepared: &crate::PreparedBookSync,
    ) -> Result<BookSyncObservation, ErrorCode> {
        Ok(BookSyncObservation::Unknown)
    }
    /// Refresh native cash and Book. False means fresh mismatch (a terminal
    /// assignment may be superseded); errors/uncertainty never authorize that.
    fn validate_funding_book_sync(
        &mut self,
        _prepared: &crate::PreparedBookSync,
    ) -> Result<bool, ErrorCode> {
        Err(ErrorCode::LedgerUnavailable)
    }
    /// Change ONLY OPERATOR_DOWN in the existing halt masks. Preserve every
    /// other flag. Failure to write/confirm the gate prevents all submissions.
    fn set_operator_down(&mut self, down: bool) -> Result<(), ErrorCode>;
}

/// A complete authoritative view of all markets and private claims. Production
/// ports must prove registry completeness and coherent observation freshness.
#[derive(Clone, Debug)]
pub struct ReconciliationSnapshot {
    pub complete: bool,
    pub ledger_observed_at_ms: u64,
    pub trader_observed_at_ms: u64,
    pub mark_observed_at_ms: u64,
    pub book_lots: BTreeMap<u16, i64>,
    pub phoenix_lots: BTreeMap<u16, i64>,
    /// Signed total cost basis of confirmed private positions, grouped by
    /// asset. Opposing users may have zero net lots and a nonzero total basis.
    pub user_entry_quote_lots: BTreeMap<u16, i128>,
    /// Signed venue cost basis from the authoritative hot/cold position view,
    /// not a locally estimated average entry or a risk-discounted PnL.
    pub phoenix_entry_quote_lots: BTreeMap<u16, i128>,
    pub user_cash_usdc: i128,
    pub user_unsettled_funding_usdc: i128,
    pub vault_usdc: u64,
    pub phoenix_collateral_usdc: u64,
    pub pool_unsettled_funding_usdc: i128,
    pub cash_in_flight_usdc: i128,
    pub halt_flags: u8,
    pub pool_safe: bool,
    /// Accounting equality cannot substitute for whole-book health. Missing
    /// backing/individual-user evidence leaves the ownership gate closed.
    pub solvency: Option<crate::SolvencyReport>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HaltReason {
    Recovering,
    UnresolvedOperations,
    StaleOrIncomplete,
    InvariantMismatch,
    ExternalHalt,
    PortUnavailable,
    SolvencyUnsafe,
    AdmissionUnsafe,
    ExecutionBudgetExceeded,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ReconciliationReport {
    pub entries_enabled: bool,
    pub reason: Option<HaltReason>,
    pub unresolved_operations: usize,
}

/// Serialized coordinator: &mut self covers every venue and private write.
/// No independent worker may submit around this boundary. Autonomous service
/// scheduling is separate from these explicitly authorized one-pass ports.
pub struct RecoveryCoordinator<V, L> {
    journal: Journal,
    venue: V,
    ledger: L,
    entries_enabled: bool,
}

impl<V, L> RecoveryCoordinator<V, L>
where
    V: VenueRecoveryPort + VenueSubmissionPort,
    L: LedgerRecoveryPort + ErAckSubmissionPort,
{
    pub fn new(journal: Journal, venue: V, ledger: L) -> Self {
        Self {
            journal,
            venue,
            ledger,
            entries_enabled: false,
        }
    }

    pub fn journal(&self) -> &Journal {
        &self.journal
    }
    pub fn entries_enabled(&self) -> bool {
        self.entries_enabled
    }

    /// Persist an already-authorized private pending intent. Discovery and
    /// placement receipt validation belong to the production ledger port.
    pub fn prepare(&mut self, intent: BoundedIntent) -> Result<Operation, JournalError> {
        self.entries_enabled = false;
        self.journal.prepare_intent(intent)
    }

    pub fn record_execution_budget(
        &mut self,
        operation_id: &OperationId,
        budget: crate::ExecutionBudget,
    ) -> Result<Operation, JournalError> {
        self.entries_enabled = false;
        self.journal.record_execution_budget(operation_id, budget)
    }

    pub fn prepare_funding(
        &mut self,
        intent: crate::FundingIntent,
    ) -> Result<crate::FundingRecord, JournalError> {
        self.entries_enabled = false;
        self.journal.prepare_funding(intent)
    }

    /// Starts gated on every pass, not just process startup. If a send returns
    /// an ambiguous error, its pre-send journal state survives for recovery.
    /// This fixed observation clock is useful for deterministic fixtures.
    /// I/O implementations must use `recover_with_clock` so freshness is
    /// checked after reads, not against the instant before a network request.
    pub fn recover(&mut self, now_ms: u64) -> Result<ReconciliationReport, JournalError> {
        self.recover_with_clock(|| now_ms)
    }

    /// Refresh the Unix-millisecond clock after reconciliation I/O and before
    /// dispatch or gate release. A backwards clock leaves entries halted.
    pub fn recover_with_clock(
        &mut self,
        mut clock: impl FnMut() -> u64,
    ) -> Result<ReconciliationReport, JournalError> {
        self.entries_enabled = false;
        self.venue.begin_recovery();
        let started_at_ms = clock();
        let mut now_ms = started_at_ms;
        if self.ledger.set_operator_down(true).is_err() {
            return self.report(Some(HaltReason::PortUnavailable));
        }
        // Discover all outcomes before writing acks, so durable fills win over
        // rejects even when the journal originally held only submission intent.
        let mut ready_to_submit = Vec::new();
        for operation in self.journal.nonterminal_operations()? {
            if matches!(
                operation.state,
                OrderState::VenueFilled
                    | OrderState::VenueRejected
                    | OrderState::AckSubmissionIntent
                    | OrderState::AckSubmitted
                    | OrderState::FailSubmissionIntent
                    | OrderState::FailSubmitted
            ) {
                continue;
            }
            match self.venue.observe(&operation) {
                Ok(VenueObservation::DefinitelyNeverSubmitted) if operation.filled_lots == 0 => {
                    self.journal
                        .record_definitely_never_submitted(&operation.operation_id, now_ms)?;
                    match self.venue.unsent_expired(&operation) {
                        Ok(true) if operation.venue_signature.is_none() => {
                            self.journal
                                .record_venue_rejected(&operation.operation_id, now_ms)?;
                        }
                        Ok(false) => ready_to_submit.push(operation.operation_id),
                        _ => {}
                    }
                }
                Ok(VenueObservation::Open { fills }) => {
                    self.journal
                        .record_fill_facts(&operation.operation_id, &fills, now_ms)?;
                    self.journal
                        .record_venue_open(&operation.operation_id, now_ms)?;
                }
                Ok(VenueObservation::Filled { fills }) => {
                    self.journal
                        .record_fill_facts(&operation.operation_id, &fills, now_ms)?;
                    self.journal
                        .record_venue_filled(&operation.operation_id, now_ms)?;
                }
                Ok(VenueObservation::Rejected { fills }) => {
                    let stored =
                        self.journal
                            .record_fill_facts(&operation.operation_id, &fills, now_ms)?;
                    if stored.filled_lots == 0 {
                        self.journal
                            .record_venue_rejected(&operation.operation_id, now_ms)?;
                    } else {
                        // A terminal rejection with fills closes the IOC
                        // remainder; only those fills are acknowledged.
                        self.journal
                            .record_venue_filled(&operation.operation_id, now_ms)?;
                    }
                }
                Err(error)
                    if operation.state == OrderState::Prepared
                        && operation.venue_signature.is_none()
                        && operation.filled_lots == 0 =>
                {
                    // A read failure before any submission intent does not
                    // erase the durable proof that the WAL is still unsent.
                    self.journal
                        .mark_error(&operation.operation_id, error, now_ms)?;
                }
                Ok(_) | Err(_) => {
                    self.journal
                        .record_unknown_venue_outcome(&operation.operation_id, now_ms)?;
                }
            }
        }

        let mut operations = self.journal.nonterminal_operations()?;
        operations.sort_by_key(|op| if op.filled_lots != 0 { 0 } else { 1 });
        let mut confirmed = Vec::new();
        for operation in operations {
            let is_fill = matches!(
                operation.state,
                OrderState::VenueFilled
                    | OrderState::AckSubmissionIntent
                    | OrderState::AckSubmitted
            );
            let is_fail = matches!(
                operation.state,
                OrderState::VenueRejected
                    | OrderState::FailSubmissionIntent
                    | OrderState::FailSubmitted
            );
            if (!is_fill && !is_fail)
                || (is_fill && !self.journal.fill_is_acknowledgeable(&operation))
            {
                continue;
            }
            let observation = match self.ledger.observe_ack_attempts(
                &operation,
                &self.journal.ack_attempts(&operation.operation_id)?,
            ) {
                Ok(value) => value,
                Err(_) => {
                    self.journal.mark_error(
                        &operation.operation_id,
                        ErrorCode::LedgerUnavailable,
                        now_ms,
                    )?;
                    continue;
                }
            };
            if observation.identity != operation.intent.identity
                || observation.operation_id != operation.operation_id
            {
                self.journal.mark_error(
                    &operation.operation_id,
                    ErrorCode::CorrelationMismatch,
                    now_ms,
                )?;
                continue;
            }
            match observation.finality {
                AckFinality::Filled {
                    lots,
                    quote_lots,
                    fee_usdc,
                } if is_fill
                    && lots == operation.filled_lots
                    && quote_lots == operation.fill_vwap_quote_lots
                    && fee_usdc == operation.fee_usdc =>
                {
                    confirmed.push((operation.operation_id, true));
                }
                AckFinality::Failed if is_fail => confirmed.push((operation.operation_id, false)),
                AckFinality::DefinitelyNotApplied => {
                    self.submit_ack(&operation, is_fill, now_ms)?
                }
                _ => {}
            }
        }

        // Existing financial outcomes are acknowledged before custody recovery.
        // A timeout never re-signs a deposit or an absolute Book assignment.
        self.observe_funding(now_ms)?;
        ready_to_submit.retain(|id| {
            self.journal
                .operation(id)
                .is_ok_and(|op| op.state == OrderState::Prepared)
        });
        let snapshot = match self.ledger.reconciliation() {
            Ok(value) => value,
            Err(_) => return self.report(Some(HaltReason::PortUnavailable)),
        };
        now_ms = clock();
        if now_ms < started_at_ms || !snapshot.is_fresh_complete(now_ms) {
            return self.report(Some(HaltReason::StaleOrIncomplete));
        }
        if !snapshot.invariants_hold() {
            return self.report(Some(HaltReason::InvariantMismatch));
        }
        for (id, fill) in confirmed {
            self.journal.finalize_ack(&id, fill, now_ms)?;
        }
        if self.journal.has_execution_budget_breach()? {
            return self.report(Some(HaltReason::ExecutionBudgetExceeded));
        }
        if !snapshot.pool_safe || cc::entries_blocked(snapshot.halt_flags & !cc::OPERATOR_DOWN) {
            return self.report(Some(HaltReason::ExternalHalt));
        }
        if snapshot
            .solvency
            .as_ref()
            .is_none_or(|s| !s.recovery_safe())
        {
            return self.report(Some(HaltReason::SolvencyUnsafe));
        }
        let remaining = self.journal.nonterminal_operations()?;
        if self.journal.has_unresolved_funding()? {
            let funding_at_ms = clock();
            if funding_at_ms < now_ms || !snapshot.is_fresh_complete(funding_at_ms) {
                return self.report(Some(HaltReason::StaleOrIncomplete));
            }
            // Never reuse this snapshot for an IOC after a funding/Book write.
            self.advance_funding(&snapshot, funding_at_ms)?;
            return self.report(Some(HaltReason::UnresolvedOperations));
        }
        if remaining
            .iter()
            .any(|op| !ready_to_submit.contains(&op.operation_id))
        {
            return self.report(Some(HaltReason::UnresolvedOperations));
        }
        let gate_at_ms = clock();
        if gate_at_ms < now_ms || !snapshot.is_fresh_complete(gate_at_ms) {
            return self.report(Some(HaltReason::StaleOrIncomplete));
        }
        now_ms = gate_at_ms;
        // Only one submission against a reconciled snapshot per pass. Other
        // prepared operations wait for this outcome and a new risk observation.
        if let Some(id) = ready_to_submit.first() {
            if self.venue.submission_enabled() {
                if snapshot
                    .solvency
                    .as_ref()
                    .is_none_or(|s| !s.configured_checks_pass())
                {
                    return self.report(Some(HaltReason::AdmissionUnsafe));
                }
                let mut operation = self.journal.operation(id)?;
                if operation.execution_budget.is_none() {
                    match self.ledger.plan_execution_budget(&operation) {
                        Ok(Some(budget)) => {
                            operation = self.journal.record_execution_budget(id, budget)?
                        }
                        Ok(None) => {}
                        Err(error) => {
                            self.journal.mark_error(id, error, now_ms)?;
                            return self.report(Some(HaltReason::AdmissionUnsafe));
                        }
                    }
                }
                match self.ledger.funding_required(&operation) {
                    Ok(Some(intent)) => {
                        if self
                            .journal
                            .funding_records()?
                            .iter()
                            .any(|r| r.intent.operation_id == *id)
                        {
                            return self.report(Some(HaltReason::AdmissionUnsafe));
                        }
                        self.journal.prepare_funding(intent)?;
                        return self.report(Some(HaltReason::UnresolvedOperations));
                    }
                    Ok(None) => {}
                    Err(error) => {
                        self.journal.mark_error(id, error, now_ms)?;
                        return self.report(Some(HaltReason::AdmissionUnsafe));
                    }
                }
                if let Err(error) = self.ledger.check_admission(&operation) {
                    self.journal.mark_error(id, error, now_ms)?;
                    return self.report(Some(HaltReason::AdmissionUnsafe));
                }
                let admission_at_ms = clock();
                if admission_at_ms < now_ms || !snapshot.is_fresh_complete(admission_at_ms) {
                    return self.report(Some(HaltReason::StaleOrIncomplete));
                }
                now_ms = admission_at_ms;
                self.submit_venue(id, now_ms)?;
            }
            return self.report(Some(HaltReason::UnresolvedOperations));
        }
        if self.ledger.set_operator_down(false).is_err() {
            return self.report(Some(HaltReason::PortUnavailable));
        }
        self.entries_enabled = true;
        self.report(None)
    }

    fn observe_funding(&mut self, now_ms: u64) -> Result<(), JournalError> {
        for record in self.journal.funding_records()? {
            let id = record.intent.operation_id;
            if record.failed_at_slot.is_some()
                || record.book_synced_at_slot.is_some()
                || record.cancelled_at_ms.is_some()
            {
                continue;
            }
            if record.attempt.is_none() {
                let operation = self.journal.operation(&id)?;
                if matches!(
                    operation.state,
                    OrderState::VenueRejected
                        | OrderState::FailSubmissionIntent
                        | OrderState::FailSubmitted
                        | OrderState::Failed
                ) && operation.venue_signature.is_none()
                    && operation.filled_lots == 0
                {
                    self.journal.cancel_unsent_funding(&id, now_ms)?;
                    continue;
                }
            }
            if record.confirmed_at_slot.is_none() && record.attempt.is_some() {
                match self.venue.observe_funding(&record) {
                    Ok(FundingObservation::Funded(receipt)) => {
                        self.journal.confirm_funding(&id, &receipt)?
                    }
                    Ok(FundingObservation::Rejected { slot }) => {
                        self.journal.reject_funding(&id, slot, now_ms)?
                    }
                    _ => {}
                }
            }
            if let Some(attempt) = self.journal.book_sync_attempts(&id)?.last() {
                if attempt.applied_at_slot.is_none() && attempt.failed_at_slot.is_none() {
                    match self.ledger.observe_funding_book_sync(&attempt.prepared) {
                        Ok(BookSyncObservation::Applied { slot }) => {
                            self.journal.finish_book_sync_attempt(
                                &id,
                                &attempt.prepared.transaction.signature,
                                slot,
                                true,
                            )?
                        }
                        Ok(BookSyncObservation::Rejected { slot }) => {
                            self.journal.finish_book_sync_attempt(
                                &id,
                                &attempt.prepared.transaction.signature,
                                slot,
                                false,
                            )?
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn advance_funding(
        &mut self,
        snapshot: &ReconciliationSnapshot,
        now_ms: u64,
    ) -> Result<(), JournalError> {
        let record = self.journal.funding_records()?.into_iter().find(|r| {
            r.failed_at_slot.is_none()
                && r.book_synced_at_slot.is_none()
                && r.cancelled_at_ms.is_none()
        });
        let Some(record) = record else {
            return Ok(());
        };
        let id = record.intent.operation_id;
        if record.confirmed_at_slot.is_some() {
            if let Some(attempt) = self.journal.book_sync_attempts(&id)?.last() {
                if attempt.applied_at_slot.is_some() {
                    match self.ledger.validate_funding_book_sync(&attempt.prepared) {
                        Ok(true) => {
                            self.journal.complete_funding_book_sync(
                                &id,
                                attempt.prepared.collateral_usdc,
                            )?;
                            return Ok(());
                        }
                        Ok(false) => {}
                        Err(_) => return Ok(()),
                    }
                }
                if attempt.failed_at_slot.is_none() && attempt.applied_at_slot.is_none() {
                    return Ok(());
                }
            }
            if let Ok(prepared) = self.ledger.prepare_funding_book_sync(&record) {
                self.journal.record_prepared_book_sync(&id, &prepared)?;
                // Any reply is observational only; completion requires the
                // exact receipt and a fresh native/Book comparison next pass.
                let _ = self.ledger.submit_funding_book_sync(&prepared);
            }
        } else if record.attempt.is_none()
            && self.journal.operation(&id)?.state == OrderState::Prepared
            && snapshot
                .solvency
                .as_ref()
                .is_some_and(|s| s.configured_checks_pass())
        {
            match self
                .venue
                .prepare_funding(&self.journal.operation(&id)?, &record.intent)
            {
                Ok(prepared) => {
                    self.journal.record_prepared_funding(&id, &prepared)?;
                    let persisted = self
                        .journal
                        .funding(&id)?
                        .ok_or(JournalError::CorruptState("missing funding attempt"))?;
                    let _ = self.venue.submit_funding(&persisted);
                }
                Err(error) => self.journal.mark_error(&id, error, now_ms)?,
            }
        }
        Ok(())
    }

    fn submit_venue(&mut self, id: &OperationId, now_ms: u64) -> Result<(), JournalError> {
        let operation = self.journal.operation(id)?;
        let prepared = match self.venue.prepare_submission(&operation) {
            Ok(value) => value,
            Err(error) => {
                // Nothing was broadcast: retain Prepared, not an ambiguous
                // SubmissionIntent, so a later fresh preflight may succeed.
                self.journal.mark_error(id, error, now_ms)?;
                return Ok(());
            }
        };
        let operation = if let Some(attempt) = &prepared {
            self.journal.record_prepared_venue(id, attempt, now_ms)?
        } else {
            self.journal.begin_venue_submission(id, now_ms)?
        };
        match self.venue.submit(&operation) {
            Ok(VenueSubmitResult::Accepted(signature)) => {
                if prepared
                    .as_ref()
                    .is_some_and(|attempt| attempt.signature != signature)
                {
                    self.journal.record_unknown_venue_outcome(id, now_ms)?;
                    self.journal
                        .mark_error(id, ErrorCode::CorrelationMismatch, now_ms)?;
                    return Ok(());
                }
                self.journal
                    .record_venue_submission(id, signature, now_ms)?;
            }
            Ok(VenueSubmitResult::Unknown) | Err(_) => {
                self.journal.record_unknown_venue_outcome(id, now_ms)?;
            }
            Ok(VenueSubmitResult::PreflightRejected) => {
                self.journal.record_venue_rejected(id, now_ms)?;
            }
        }
        Ok(())
    }

    fn submit_ack(
        &mut self,
        operation: &Operation,
        fill: bool,
        now_ms: u64,
    ) -> Result<(), JournalError> {
        // An expired/rejected earlier ack may be retried only after the port
        // proves it cannot land. Keep the write-ahead state for every attempt.
        self.journal
            .retry_ack_intent(&operation.operation_id, fill, now_ms)?;
        let command = if fill {
            ErAckCommand::Fill {
                intent: operation.intent.clone(),
                fills: self.journal.fill_facts(&operation.operation_id)?,
            }
        } else {
            ErAckCommand::Fail {
                intent: operation.intent.clone(),
            }
        };
        let prepared = match self.ledger.prepare_ack(&command) {
            Ok(value) => value,
            Err(_) => {
                self.journal.mark_error(
                    &operation.operation_id,
                    ErrorCode::LedgerUnavailable,
                    now_ms,
                )?;
                return Ok(());
            }
        };
        if let Some(ack) = &prepared {
            self.journal
                .record_prepared_ack(&operation.operation_id, ack)?;
        }
        match self.ledger.submit_ack(&command) {
            Ok(ErAckSubmitResult::Accepted(signature)) => {
                if prepared
                    .as_ref()
                    .is_some_and(|ack| ack.signature != signature)
                {
                    self.journal.mark_error(
                        &operation.operation_id,
                        ErrorCode::CorrelationMismatch,
                        now_ms,
                    )?;
                    return Ok(());
                }
                if fill {
                    self.journal.record_fill_ack_submission(
                        &operation.operation_id,
                        signature,
                        now_ms,
                    )?;
                } else {
                    self.journal.record_fail_ack_submission(
                        &operation.operation_id,
                        signature,
                        now_ms,
                    )?;
                }
            }
            Ok(ErAckSubmitResult::Unknown) | Err(_) => {
                self.journal.mark_error(
                    &operation.operation_id,
                    ErrorCode::LedgerUnavailable,
                    now_ms,
                )?;
            }
        }
        Ok(())
    }

    fn report(&self, reason: Option<HaltReason>) -> Result<ReconciliationReport, JournalError> {
        Ok(ReconciliationReport {
            entries_enabled: self.entries_enabled,
            reason,
            unresolved_operations: self.journal.nonterminal_operations()?.len(),
        })
    }
}

impl ReconciliationSnapshot {
    fn is_fresh_complete(&self, now_ms: u64) -> bool {
        let fresh =
            |observed: u64, ttl: u64| now_ms.checked_sub(observed).is_some_and(|age| age <= ttl);
        self.complete
            && fresh(self.ledger_observed_at_ms, cc::TRADER_STATE_STALE_MS)
            && fresh(self.trader_observed_at_ms, cc::TRADER_STATE_STALE_MS)
            && fresh(self.mark_observed_at_ms, cc::MARK_STALE_MS)
    }

    fn invariants_hold(&self) -> bool {
        let positions_match =
            self.book_lots
                .iter()
                .chain(self.phoenix_lots.iter())
                .all(|(asset, _)| {
                    self.book_lots.get(asset).copied().unwrap_or(0)
                        == self.phoenix_lots.get(asset).copied().unwrap_or(0)
                });
        let basis_complete = self
            .book_lots
            .iter()
            .all(|(asset, lots)| *lots == 0 || self.user_entry_quote_lots.contains_key(asset))
            && self.phoenix_lots.iter().all(|(asset, lots)| {
                *lots == 0 || self.phoenix_entry_quote_lots.contains_key(asset)
            })
            && self.user_entry_quote_lots.iter().all(|(asset, basis)| {
                *basis == 0 || self.phoenix_entry_quote_lots.contains_key(asset)
            });
        // At equal net lots the common marked position value cancels. The
        // remaining raw-equity difference is private basis minus venue basis.
        // Keep this separate from funding, cash in flight and risk haircuts.
        let basis_delta = self
            .user_entry_quote_lots
            .values()
            .try_fold(0i128, |sum, basis| sum.checked_add(*basis))
            .and_then(|sum| {
                self.phoenix_entry_quote_lots
                    .values()
                    .try_fold(sum, |sum, basis| sum.checked_sub(*basis))
            });
        let lhs = self
            .user_cash_usdc
            .checked_add(self.user_unsettled_funding_usdc);
        let rhs = i128::from(self.vault_usdc)
            .checked_add(i128::from(self.phoenix_collateral_usdc))
            .and_then(|value| value.checked_add(self.pool_unsettled_funding_usdc))
            .and_then(|value| value.checked_add(basis_delta?))
            .and_then(|value| value.checked_add(self.cash_in_flight_usdc));
        positions_match && basis_complete && lhs.is_some_and(|cash| cash >= 0) && lhs == rhs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native_markets(
        mark: u64,
        tick_size: u64,
    ) -> std::collections::HashMap<String, phoenix_rise_math::PerpAssetMetadata> {
        use phoenix_rise_math::*;
        let tiers = LeverageTiers::new(std::array::from_fn(|i| LeverageTier {
            upper_bound_size: BaseLots::new((i as u64 + 1) * 1_000_000),
            max_leverage: Constant::new(10),
            limit_order_risk_factor: BasisPoints::new(10_000),
        }))
        .unwrap();
        std::collections::HashMap::from([(
            "SOL".to_owned(),
            PerpAssetMetadata::new(
                "SOL".to_owned(),
                1,
                0,
                Ticks::new(mark),
                QuoteLotsPerBaseLotPerTick::new(tick_size),
                tiers,
                [5_000, 2_000, 1_000],
                7_500,
                10_000,
                10_000,
            ),
        )])
    }

    fn native_portfolio(lots: i64, entry: i64) -> phoenix_rise_math::TraderPortfolio {
        use phoenix_rise_math::*;
        let builder =
            TraderPortfolio::builder().quote_lot_collateral(SignedQuoteLots::new(2_000_000_000));
        if lots == 0 {
            builder.build()
        } else {
            let mut position = TraderPosition::new();
            position.base_lot_position = SignedBaseLots::new(lots);
            position.virtual_quote_lot_position = SignedQuoteLots::new(-entry);
            builder.position("SOL", position).build()
        }
    }

    fn native_fill(
        pool: &phoenix_rise_math::TraderPortfolio,
        lots: i64,
        ticks: u64,
        tick_size: u64,
    ) -> phoenix_rise_math::SimulatedPositionFill {
        use phoenix_rise_math::*;
        pool.simulate_position_fill(
            SimulatePositionFillParams {
                symbol: "SOL",
                side: if lots > 0 { Side::Bid } else { Side::Ask },
                base_lots: BaseLots::new(lots.unsigned_abs()),
                price_ticks: Ticks::new(ticks),
                fee_quote_lots: None,
                margin_mode: MarginSimulationMode::Cross,
                isolated_collateral: None,
            },
            &native_markets(ticks, tick_size),
        )
        .unwrap()
    }

    #[test]
    fn uniform_fill_accounting_matches_pinned_native_sdk() {
        for before in [0i64, 3, -3, 6, -6] {
            let entry = before.signum() * 10;
            for lots in [1i64, -1, 2, -2, 3, -3, 5, -5, 6, -6, 8, -8] {
                let private = cc::realize_on_fill(before, lots, entry, lots * 4).unwrap();
                let native = native_fill(&native_portfolio(before, entry), lots, 4, 1);
                assert_eq!(private.realized_usdc, native.realized_pnl.as_inner());
                assert_eq!(
                    private.new_entry_quote,
                    native
                        .projected_position
                        .map(|p| -p.virtual_quote_lot_position.as_inner())
                        .unwrap_or(0)
                );
            }
        }
    }

    #[test]
    fn collapsing_sdk_executions_can_change_cash_and_basis_without_changing_equity() {
        // Diagnostic, not a claim about on-chain match granularity. Dispatch
        // stays disabled until an actual native receipt settles that question.
        for (before, entry, ticks, tick_size, expected_aggregate, expected_sequential) in [
            (
                1,
                100_000_000,
                [110, 120],
                1_000_000,
                (15_000_000, -115_000_000),
                (10_000_000, -120_000_000),
            ),
            (6, 10, [3, 3], 1, (3, 7), (4, 8)),
        ] {
            let mut native_pool = native_portfolio(before, entry);
            let (mut lots, mut basis, mut cash) = (before, entry, 0i64);
            let mut native_cash = 0;
            for price in ticks {
                let private =
                    cc::realize_on_fill(lots, -1, basis, -((price * tick_size) as i64)).unwrap();
                let native = native_fill(&native_pool, -1, price, tick_size);
                assert_eq!(private.realized_usdc, native.realized_pnl.as_inner());
                assert_eq!(
                    private.new_entry_quote,
                    native
                        .projected_position
                        .map(|p| -p.virtual_quote_lot_position.as_inner())
                        .unwrap_or(0)
                );
                lots -= 1;
                basis = private.new_entry_quote;
                cash += private.realized_usdc;
                native_cash += native.realized_pnl.as_inner();
                native_pool = native.projected_portfolio;
            }
            let aggregate_quote = -(((ticks[0] + ticks[1]) * tick_size) as i64);
            let aggregate = cc::realize_on_fill(before, -2, entry, aggregate_quote).unwrap();
            assert_eq!(
                (aggregate.realized_usdc, aggregate.new_entry_quote),
                expected_aggregate
            );
            assert_eq!((cash, basis), expected_sequential);
            assert_eq!(cash, native_cash);
            assert_ne!(aggregate.realized_usdc, native_cash);
            assert_eq!(
                aggregate.realized_usdc - aggregate.new_entry_quote,
                cash - basis,
                "I2 consistency does not establish identical realized PnL allocation"
            );
        }
    }

    fn snapshot() -> ReconciliationSnapshot {
        ReconciliationSnapshot {
            complete: true,
            ledger_observed_at_ms: 1000,
            trader_observed_at_ms: 1000,
            mark_observed_at_ms: 1000,
            book_lots: BTreeMap::new(),
            phoenix_lots: BTreeMap::new(),
            user_entry_quote_lots: BTreeMap::new(),
            phoenix_entry_quote_lots: BTreeMap::new(),
            user_cash_usdc: 10,
            user_unsettled_funding_usdc: 0,
            vault_usdc: 10,
            phoenix_collateral_usdc: 0,
            pool_unsettled_funding_usdc: 0,
            cash_in_flight_usdc: 0,
            halt_flags: 0,
            pool_safe: true,
            solvency: Some(crate::SolvencyReport {
                user_margin_safe: true,
                positive_claims_usdc: 10,
                backing_equity_usdc: 10,
                backing_surplus_usdc: 0,
                bad_debt_usdc: 0,
                gross_notional_usdc: 0,
                gross_limits_safe: None,
                worst_stress_surplus_usdc: None,
            }),
        }
    }

    #[test]
    fn future_observations_are_not_fresh() {
        let mut value = snapshot();
        assert!(value.is_fresh_complete(1000));
        value.mark_observed_at_ms = 1001;
        assert!(!value.is_fresh_complete(1000));
    }

    #[test]
    fn signed_reconciliation_rejects_overflow_and_negative_aggregate() {
        let mut value = snapshot();
        assert!(value.invariants_hold());
        value.user_cash_usdc = i128::MAX;
        value.user_unsettled_funding_usdc = 1;
        assert!(!value.invariants_hold());
        value.user_unsettled_funding_usdc = 0;
        value.user_cash_usdc = -10;
        value.vault_usdc = 0;
        value.cash_in_flight_usdc = -10;
        assert!(!value.invariants_hold());
        value.user_cash_usdc = 10;
        value.cash_in_flight_usdc = i128::MAX;
        value.pool_unsettled_funding_usdc = 1;
        assert!(!value.invariants_hold());
    }

    #[test]
    fn basis_cannot_mask_missing_inventory_unmatched_lots_or_arithmetic_overflow() {
        let mut value = snapshot();
        value.book_lots.insert(1, 2);
        value.phoenix_lots.insert(1, 2);
        assert!(!value.invariants_hold(), "missing basis is not zero basis");
        value.user_entry_quote_lots.insert(1, 200);
        assert!(!value.invariants_hold());
        value.phoenix_entry_quote_lots.insert(1, 200);
        assert!(value.invariants_hold());
        value.phoenix_lots.insert(1, 1);
        assert!(
            !value.invariants_hold(),
            "basis cannot excuse an I1 mismatch"
        );
        value.phoenix_lots.insert(1, 2);
        value.user_entry_quote_lots.insert(2, i128::MAX);
        value.phoenix_entry_quote_lots.insert(2, i128::MAX);
        assert!(
            !value.invariants_hold(),
            "overflow must not silently cancel"
        );
    }

    #[test]
    fn pooled_realization_with_open_opposing_users_reconciles_only_with_exact_basis() {
        use phoenix_rise_math::{
            BaseLots, MarginSimulationMode, Side, SignedBaseLots, SignedQuoteLots,
            SimulatePositionFillParams, Ticks, TraderPortfolio, TraderPosition,
        };

        const USDC: i64 = 1_000_000;
        for closing_price in [90i64, 110] {
            let provider = native_markets(closing_price as u64, USDC as u64);
            // User A opened +10 at $100; Phoenix has the same net position.
            let mut pool_position = TraderPosition::new();
            pool_position.base_lot_position = SignedBaseLots::new(10);
            pool_position.virtual_quote_lot_position = SignedQuoteLots::new(-1000 * USDC);
            let portfolio = TraderPortfolio::builder()
                .quote_lot_collateral(SignedQuoteLots::new(2000 * USDC))
                .position("SOL", pool_position)
                .build();
            // User B now opens -10. This closes Phoenix's net long, not A's
            // private long. The official venue math realizes pool PnL.
            let native = portfolio
                .simulate_position_fill(
                    SimulatePositionFillParams {
                        symbol: "SOL",
                        side: Side::Ask,
                        base_lots: BaseLots::new(10),
                        price_ticks: Ticks::new(closing_price as u64),
                        fee_quote_lots: None,
                        margin_mode: MarginSimulationMode::Cross,
                        isolated_collateral: None,
                    },
                    &provider,
                )
                .unwrap();
            let user_b = cc::realize_on_fill(0, -10, 0, -10 * closing_price * USDC).unwrap();
            assert_eq!(user_b.realized_usdc, 0);
            assert!(native.projected_position.is_none());
            let venue_cash = native.margin.quote_lot_collateral.as_inner();
            assert_eq!(venue_cash, (2000 + 10 * (closing_price - 100)) * USDC);
            let mut value = snapshot();
            value.vault_usdc = 0;
            value.user_cash_usdc = i128::from(2000 * USDC);
            value.phoenix_collateral_usdc = u64::try_from(venue_cash).unwrap();
            assert!(
                !value.invariants_hold(),
                "missing basis must not silently accept different realization bases"
            );
            let users_upnl = cc::upnl_usdc(10, closing_price * USDC, 1000 * USDC)
                + cc::upnl_usdc(-10, closing_price * USDC, user_b.new_entry_quote);
            assert_eq!(
                value.user_cash_usdc + users_upnl,
                i128::from(venue_cash),
                "raw equity balances despite different realized cash"
            );
            value.user_entry_quote_lots.insert(
                1,
                i128::from(1000 * USDC) + i128::from(user_b.new_entry_quote),
            );
            value.phoenix_entry_quote_lots.insert(1, 0);
            assert!(value.invariants_hold());
            value.phoenix_collateral_usdc += 1;
            assert!(
                !value.invariants_hold(),
                "no unexplained cash or rounding tolerance"
            );
        }
    }

    #[test]
    fn averaged_user_entries_and_net_pool_reconcile_through_partial_closes_and_flips() {
        use phoenix_rise_math::{
            BaseLots, MarginSimulationMode, QuoteLots, Side, SignedQuoteLots,
            SimulatePositionFillParams, Ticks, TraderPortfolio,
        };
        // Micro quote lots deliberately exercise indivisible per-lot basis.
        let mut pool = TraderPortfolio::builder()
            .quote_lot_collateral(SignedQuoteLots::new(2000))
            .build();
        let mut users = [(0i64, 0i64, 1000i128); 2];
        let fills = [
            (0, 3i64, 100i64),
            (0, 2, 120),
            (1, -4, 110),
            (0, -1, 115),
            (1, 1, 90),
            (0, -6, 130),
            (1, 5, 125),
            (0, 2, 120),
            (1, 1, 140),
            (1, -3, 135),
        ];
        for (user, lots, price) in fills {
            let (before, entry, cash) = users[user];
            let fill = cc::realize_on_fill(before, lots, entry, lots * price).unwrap();
            users[user] = (
                before + lots,
                fill.new_entry_quote,
                cash + i128::from(fill.realized_usdc) - 2,
            );
            let native = pool
                .simulate_position_fill(
                    SimulatePositionFillParams {
                        symbol: "SOL",
                        side: if lots > 0 { Side::Bid } else { Side::Ask },
                        base_lots: BaseLots::new(lots.unsigned_abs()),
                        price_ticks: Ticks::new(price as u64),
                        fee_quote_lots: Some(QuoteLots::new(2)),
                        margin_mode: MarginSimulationMode::Cross,
                        isolated_collateral: None,
                    },
                    &native_markets(price as u64, 1),
                )
                .unwrap();
            let mut value = snapshot();
            let net = users.iter().map(|u| u.0).sum();
            value.book_lots.insert(1, net);
            value.phoenix_lots.insert(
                1,
                native
                    .projected_position
                    .map(|p| p.base_lot_position.as_inner())
                    .unwrap_or(0),
            );
            value
                .user_entry_quote_lots
                .insert(1, users.iter().map(|u| i128::from(u.1)).sum());
            value.phoenix_entry_quote_lots.insert(
                1,
                native
                    .projected_position
                    .map(|p| -i128::from(p.virtual_quote_lot_position.as_inner()))
                    .unwrap_or(0),
            );
            value.user_cash_usdc = users.iter().map(|u| u.2).sum();
            value.vault_usdc = 0;
            value.phoenix_collateral_usdc =
                u64::try_from(native.margin.quote_lot_collateral.as_inner()).unwrap();
            assert!(
                value.invariants_hold(),
                "user {user}, fill {lots} at {price}"
            );
            value.user_unsettled_funding_usdc = 3;
            value.pool_unsettled_funding_usdc = 3;
            assert!(
                value.invariants_hold(),
                "matched unsettled funding stays separate"
            );
            value.pool_unsettled_funding_usdc = 2;
            assert!(
                !value.invariants_hold(),
                "basis cannot cover unexplained funding"
            );
            pool = native.projected_portfolio;
        }
        assert!(users.iter().all(|u| u.0 == 0 && u.1 == 0));
        assert!(pool.positions.is_empty());
        assert_eq!(
            users.iter().map(|u| u.2).sum::<i128>(),
            i128::from(pool.quote_lot_collateral.as_inner())
        );
    }
}
