use std::collections::BTreeMap;

use cinder_common as cc;

use crate::{
    BoundedIntent, ErrorCode, FillFact, Journal, JournalError, Operation, OperationId,
    OrderIdentity, OrderState, VenueObservation,
};

/// Production implementations must use history/transaction evidence, not an
/// absent lookup result or a local timeout, for DefinitelyNeverSubmitted.
pub trait VenueRecoveryPort {
    /// Verify the exact global venue ID, pooled trader, asset, and direction
    /// before returning facts. Per-user client IDs are never a venue lookup key.
    fn observe(&mut self, operation: &Operation) -> Result<VenueObservation, ErrorCode>;
}

pub enum VenueSubmitResult {
    Accepted([u8; 64]),
    Unknown,
}

pub trait VenueSubmissionPort {
    /// Submit exactly the persisted venue ID, direction, integer bound and
    /// L1 deadline. Revalidate fresh R3 risk and expiry before sending.
    fn submit(&mut self, operation: &Operation) -> Result<VenueSubmitResult, ErrorCode>;
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
    /// Calculate fresh post-transition margin through the R3 risk boundary.
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
    fn reconciliation(&mut self) -> Result<ReconciliationSnapshot, ErrorCode>;
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
    pub user_cash_usdc: i128,
    pub user_unsettled_funding_usdc: i128,
    pub vault_usdc: u64,
    pub phoenix_collateral_usdc: u64,
    pub pool_unsettled_funding_usdc: i128,
    pub cash_in_flight_usdc: i128,
    pub halt_flags: u8,
    pub pool_safe: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HaltReason {
    Recovering,
    UnresolvedOperations,
    StaleOrIncomplete,
    InvariantMismatch,
    ExternalHalt,
    PortUnavailable,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ReconciliationReport {
    pub entries_enabled: bool,
    pub reason: Option<HaltReason>,
    pub unresolved_operations: usize,
}

/// Serialized coordinator: &mut self covers every venue and private write.
/// No independent worker may submit around this boundary. It is not a live
/// service until authoritative production ports have been implemented.
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

    /// Starts gated on every pass, not just process startup. If a send returns
    /// an ambiguous error, its pre-send journal state survives for recovery.
    pub fn recover(&mut self, now_ms: u64) -> Result<ReconciliationReport, JournalError> {
        self.entries_enabled = false;
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
                    ready_to_submit.push(operation.operation_id);
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
                        // Terminal partial IOC facts remain blocked in this
                        // foundation; R5 provides partial-fill ack semantics.
                        self.journal
                            .record_venue_filled(&operation.operation_id, now_ms)?;
                    }
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
            if (!is_fill && !is_fail) || (is_fill && !self.journal.fill_is_complete(&operation)) {
                continue;
            }
            let observation = match self.ledger.observe_ack(&operation) {
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

        let snapshot = match self.ledger.reconciliation() {
            Ok(value) => value,
            Err(_) => return self.report(Some(HaltReason::PortUnavailable)),
        };
        if !snapshot.is_fresh_complete(now_ms) {
            return self.report(Some(HaltReason::StaleOrIncomplete));
        }
        if !snapshot.invariants_hold() {
            return self.report(Some(HaltReason::InvariantMismatch));
        }
        for (id, fill) in confirmed {
            self.journal.finalize_ack(&id, fill, now_ms)?;
        }
        if !snapshot.pool_safe || cc::entries_blocked(snapshot.halt_flags & !cc::OPERATOR_DOWN) {
            return self.report(Some(HaltReason::ExternalHalt));
        }
        let remaining = self.journal.nonterminal_operations()?;
        if remaining
            .iter()
            .any(|op| !ready_to_submit.contains(&op.operation_id))
        {
            return self.report(Some(HaltReason::UnresolvedOperations));
        }
        // Only one submission against a reconciled snapshot per pass. Other
        // prepared operations wait for this outcome and a new risk observation.
        if let Some(id) = ready_to_submit.first() {
            self.submit_venue(id, now_ms)?;
            return self.report(Some(HaltReason::UnresolvedOperations));
        }
        if self.ledger.set_operator_down(false).is_err() {
            return self.report(Some(HaltReason::PortUnavailable));
        }
        self.entries_enabled = true;
        self.report(None)
    }

    fn submit_venue(&mut self, id: &OperationId, now_ms: u64) -> Result<(), JournalError> {
        let operation = self.journal.begin_venue_submission(id, now_ms)?;
        match self.venue.submit(&operation) {
            Ok(VenueSubmitResult::Accepted(signature)) => {
                self.journal
                    .record_venue_submission(id, signature, now_ms)?;
            }
            Ok(VenueSubmitResult::Unknown) | Err(_) => {
                self.journal.record_unknown_venue_outcome(id, now_ms)?;
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
        match self.ledger.submit_ack(&command) {
            Ok(ErAckSubmitResult::Accepted(signature)) => {
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
        let lhs = self
            .user_cash_usdc
            .checked_add(self.user_unsettled_funding_usdc);
        let rhs = i128::from(self.vault_usdc)
            .checked_add(i128::from(self.phoenix_collateral_usdc))
            .and_then(|value| value.checked_add(self.pool_unsettled_funding_usdc))
            .and_then(|value| value.checked_add(self.cash_in_flight_usdc));
        positions_match && lhs.is_some_and(|cash| cash >= 0) && lhs == rhs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> ReconciliationSnapshot {
        ReconciliationSnapshot {
            complete: true,
            ledger_observed_at_ms: 1000,
            trader_observed_at_ms: 1000,
            mark_observed_at_ms: 1000,
            book_lots: BTreeMap::new(),
            phoenix_lots: BTreeMap::new(),
            user_cash_usdc: 10,
            user_unsettled_funding_usdc: 0,
            vault_usdc: 10,
            phoenix_collateral_usdc: 0,
            pool_unsettled_funding_usdc: 0,
            cash_in_flight_usdc: 0,
            halt_flags: 0,
            pool_safe: true,
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
}
