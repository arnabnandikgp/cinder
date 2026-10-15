//! Exact-batch inventory in the authoritative journal, not another balance book.
//! Observation actions are trusted INTERNAL qualification results. The owning
//! chain/controller port must verify original wire, finality, code and route.
use crate::{
    Error, funds,
    model::*,
    orders::Approval,
    wire::{MAX_ITEMS, Reader, Writer},
};
use cinder_kernel::{
    amounts::QuoteAtoms,
    identity::*,
    ledger::{evidence::Disposition, funds::Destination, *},
};
use sha2::{Digest, Sha256};

/// Immutable original owner deposit; public wallets are not private account IDs.
#[derive(Clone, PartialEq, Eq)]
pub struct Batch {
    /// Private owner and permanent journal operation.
    pub request: RequestKey,
    /// Immutable custody route PDA.
    pub route: [u8; 32],
    /// Permanent on-chain consumption identity.
    pub batch: [u8; 32],
    /// Owner's deposit operation, distinct from the operator's sweep operation.
    pub operation: [u8; 32],
    /// Registered public customer wallet.
    pub owner: [u8; 32],
    /// Full original collateral, never the changing account balance.
    pub amount: QuoteAtoms,
    /// Original on-chain funds-authority epoch.
    pub epoch: u64,
    /// Original sweep consent deadline in Solana slots.
    pub expires_at_slot: u64,
    /// Original finalized owner transaction signature.
    pub signature: [u8; 64],
}
impl std::fmt::Debug for Batch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EscrowBatch([PRIVATE])")
    }
}
impl Batch {
    /// Stable economic identity independent of lookup, commit or attempt IDs.
    pub fn handoff(&self, source: EventScope) -> Result<EventKey, Error> {
        let mut hash = Sha256::new();
        hash.update(b"CINDER-ESCROW-HANDOFF-1\0");
        hash.update(self.route);
        hash.update(self.batch);
        let digest: [u8; 32] = hash.finalize().into();
        Ok(EventKey {
            scope: source,
            event: EconomicEventId::new(&digest).map_err(|_| Error::Invalid)?,
            leg: 0,
        })
    }
}
/// Lifecycle, not additive financial balances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Owner-refundable; no financial posting.
    Pending,
    /// Persisted sweep intent, not exposed.
    Prepared,
    /// Possibly signed/sent; timeout cannot return it to Pending.
    Exposed,
    /// Finalized failed or cancelled unexposed sweep; owner can still refund.
    Failed,
    /// Finalized owner refund; cannot later sweep or credit.
    Refunded,
    /// Recognized at broker handoff, with funding restrictions.
    Funding,
    /// Strongly qualified full native credit; restrictions released.
    Available,
    /// Full fenced return of unallocated collateral to the payout vault.
    Returned,
}
/// Replay-derived inventory referring to the original batch and existing ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Original immutable consent.
    pub batch: Batch,
    /// Current phase.
    pub status: Status,
    /// At most one capability; restart cannot grant another.
    pub attempt: Option<AttemptKey>,
    /// Original economic handoff, retained after full funding.
    pub receipt: Option<EventKey>,
    /// Actual finalized Solana handoff slot, not a native venue causal cut.
    pub handoff_slot: Option<u64>,
    /// Sole linked ordinary Broker -> Venue operation.
    pub funding: Option<RequestKey>,
    /// Recovery-only return; never another customer deposit.
    pub returning: Option<RequestKey>,
}
/// Internal qualification results, not customer-supplied proof claims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Original finalized deposit as pending owner inventory only.
    Pending(Box<Batch>),
    /// Persist exact sweep action before exposure/signature.
    Prepare {
        /// Sole original sweep attempt.
        attempt: AttemptKey,
        /// Exact qualified unsigned action.
        message: PrivateBytes,
        /// Original funds authority, not an authentication proof.
        epoch: u64,
        /// Injected millisecond dispatch deadline, not a Solana slot.
        expires_at: u64,
    },
    /// Cancel only before exposure; permanent identity remains consumed.
    Cancel(RequestKey),
    /// Finalized owner refund, including one that won a race with sweep.
    Refund(RequestKey),
    /// Finalized original sweep failure, never a timeout.
    Failed(AttemptKey),
    /// Trusted controller proved the expired original plan has no retained
    /// signed-wire capability at the independently verified current head. Expiry
    /// alone is insufficient; this is not chain execution/failure evidence.
    Unsent(AttemptKey),
    /// Finalized full sweep: atomically recognize and restrict new credit.
    Handoff {
        /// Original exposed attempt.
        attempt: AttemptKey,
        /// Stable batch-bound broker economic identity.
        receipt: EventKey,
        /// Qualified finalized original sweep slot, separate from milliseconds.
        slot: u64,
        /// Trusted original receive time.
        observed_at: u64,
        /// Exact sanitized qualification evidence, not credentials.
        raw: PrivateBytes,
    },
    /// Move this broker restriction into the existing funds lifecycle. Approval
    /// is a qualified transfer instruction, never a fabricated wallet signature.
    Fund {
        /// Original escrow request.
        request: RequestKey,
        /// Full ordinary Broker -> Venue instruction.
        intent: Box<funds::Intent>,
        /// Authenticated intent digest from the trusted caller.
        approval: Approval,
    },
    /// Release only after full, non-demo native arrival and strong completion.
    Finish(RequestKey),
    /// Fenced operator recalls only unsent/strongly failed native allocation.
    Recall {
        /// Original recognized batch.
        request: RequestKey,
        /// Exact full Broker -> Vault recovery instruction.
        intent: Box<funds::Intent>,
        /// Authenticated recovery-operator digest, not customer consent.
        approval: Approval,
    },
    /// Release only after a full qualified return into the payout vault.
    FinishReturn(RequestKey),
}
impl State {
    /// Private inventory/status from this journal, not a second financial store.
    pub fn escrows(&self) -> &[Entry] {
        &self.escrows
    }
    /// Recheck before the original sweep wire is delivered. No resend authority.
    pub fn qualified_escrow_delivery(&self, key: AttemptKey) -> Result<(), ControlError> {
        let a = self
            .attempts
            .iter()
            .find(|a| a.key == key)
            .ok_or(ControlError::Invalid)?;
        if a.kind != AttemptKind::Escrow || !a.possibly_exposed || a.expires_at <= self.now {
            return Err(ControlError::Unqualified);
        }
        self.escrow_exposure(a)?;
        self.exposure_qualified(a)
    }
    pub(crate) fn escrow_exposure(&self, a: &Attempt) -> Result<(), ControlError> {
        if a.kind != AttemptKind::Escrow {
            return Ok(());
        }
        let e = self
            .escrows
            .iter()
            .find(|e| e.attempt == Some(a.key))
            .ok_or(ControlError::Invalid)?;
        if !matches!(e.status, Status::Prepared | Status::Exposed)
            || a.authority_epoch != e.batch.epoch
            || !self
                .holds
                .iter()
                .any(|h| h.request == a.key.request && h.active)
        {
            return Err(ControlError::Unqualified);
        }
        Ok(())
    }
    pub(crate) fn escrow_action(&mut self, action: &Action) -> Result<(), ControlError> {
        if let Action::Pending(batch) = action {
            self.request(batch.request)?;
            if let Some(old) = self
                .escrows
                .iter()
                .find(|e| e.batch.request == batch.request || e.batch.batch == batch.batch)
            {
                return if old.batch == **batch {
                    Ok(())
                } else {
                    Err(ControlError::Invalid)
                };
            }
            if self.escrows.len() >= MAX_ITEMS
                || self.holds.len() >= MAX_ITEMS
                || self.holds.iter().any(|h| h.request == batch.request)
                || self.selections.iter().any(|s| s.request == batch.request)
                || [batch.route, batch.batch, batch.operation, batch.owner].contains(&[0; 32])
                || batch.signature == [0; 64]
                || batch.epoch == 0
                || batch.expires_at_slot == 0
                || batch.amount.unit() != self.config.quote
                || batch.amount.atoms() <= 0
            {
                return Err(ControlError::Invalid);
            }
            // Consume identity without an active financial hold/recovery claim.
            self.holds.push(Hold {
                request: batch.request,
                reservations: vec![],
                active: false,
            });
            self.escrows.push(Entry {
                batch: (**batch).clone(),
                status: Status::Pending,
                attempt: None,
                receipt: None,
                handoff_slot: None,
                funding: None,
                returning: None,
            });
            return Ok(());
        }
        let request = match action {
            Action::Pending(_) => return Err(ControlError::Invalid),
            Action::Prepare { attempt, .. }
            | Action::Failed(attempt)
            | Action::Unsent(attempt)
            | Action::Handoff { attempt, .. } => attempt.request,
            Action::Cancel(r)
            | Action::Refund(r)
            | Action::Finish(r)
            | Action::FinishReturn(r)
            | Action::Recall { request: r, .. }
            | Action::Fund { request: r, .. } => *r,
        };
        let index = self
            .escrows
            .iter()
            .position(|e| e.batch.request == request)
            .ok_or(ControlError::Invalid)?;
        let entry = self.escrows[index].clone();
        match action {
            Action::Pending(_) => return Err(ControlError::Invalid),
            Action::Prepare {
                attempt,
                message,
                epoch,
                expires_at,
            } => {
                if entry.status != Status::Pending
                    || entry.attempt.is_some()
                    || *epoch != entry.batch.epoch
                    || self.frozen
                    || self.recovery.is_some()
                {
                    return Err(ControlError::Unqualified);
                }
                self.hold_escrow(request)?.active = true;
                self.prepare_attempt(
                    *attempt,
                    message.clone(),
                    *epoch,
                    *expires_at,
                    AttemptKind::Escrow,
                )?;
                self.escrows[index].attempt = Some(*attempt);
                self.escrows[index].status = Status::Prepared;
                self.all_capacity()?;
            }
            Action::Cancel(_) => {
                if entry.status != Status::Prepared
                    || self
                        .attempts
                        .iter()
                        .any(|a| Some(a.key) == entry.attempt && a.possibly_exposed)
                {
                    return Err(ControlError::Exposed);
                }
                self.escrows[index].status = Status::Failed;
                self.hold_escrow(request)?.active = false;
            }
            Action::Refund(_) => {
                if matches!(
                    entry.status,
                    Status::Funding | Status::Available | Status::Returned
                ) {
                    return Err(ControlError::Unqualified);
                }
                self.escrows[index].status = Status::Refunded;
                self.hold_escrow(request)?.active = false;
            }
            Action::Failed(attempt) | Action::Unsent(attempt) => {
                if entry.attempt != Some(*attempt)
                    || !matches!(entry.status, Status::Exposed | Status::Failed)
                    || !self
                        .attempts
                        .iter()
                        .any(|a| a.key == *attempt && a.possibly_exposed)
                {
                    return Err(ControlError::Unqualified);
                }
                if matches!(action, Action::Unsent(_))
                    && !self
                        .attempts
                        .iter()
                        .any(|a| a.key == *attempt && a.expires_at <= self.now)
                {
                    return Err(ControlError::Unqualified);
                }
                self.escrows[index].status = Status::Failed;
                self.hold_escrow(request)?.active = false;
            }
            Action::Handoff {
                attempt,
                receipt,
                slot,
                observed_at,
                raw,
            } => {
                if *slot == 0
                    || entry.attempt != Some(*attempt)
                    || *observed_at > self.now
                    || raw.as_bytes().is_empty()
                    || !self
                        .config
                        .sources
                        .iter()
                        .any(|s| s.location == Location::Broker && s.scope == receipt.scope)
                    || entry
                        .batch
                        .handoff(receipt.scope)
                        .map_err(|_| ControlError::Invalid)?
                        != *receipt
                {
                    return Err(ControlError::Invalid);
                }
                if entry.receipt.as_ref() == Some(receipt) {
                    return if entry.handoff_slot == Some(*slot) {
                        Ok(())
                    } else {
                        Err(ControlError::Invalid)
                    };
                }
                if entry.status != Status::Exposed
                    || !self
                        .attempts
                        .iter()
                        .any(|a| a.key == *attempt && a.possibly_exposed)
                {
                    return Err(ControlError::Unqualified);
                }
                let customer = Owner::Customer(request.account);
                let before = self
                    .ledger
                    .book(customer)
                    .map_err(|_| ControlError::Invalid)?
                    .cash()
                    .atoms();
                let event = Event {
                    key: RecordKey::Economic(receipt.clone()),
                    policy: self.config.policy,
                    change: Change::Receipt {
                        owner: customer,
                        location: Location::Broker,
                        amount: entry.batch.amount,
                    },
                };
                let next = self
                    .ledger
                    .ingest(&event, self.now)
                    .map_err(|_| ControlError::Invalid)?;
                if next.disposition != Disposition::Applied {
                    return Err(ControlError::Unqualified);
                }
                self.ledger = next.state;
                let after = self
                    .ledger
                    .book(customer)
                    .map_err(|_| ControlError::Invalid)?
                    .cash()
                    .atoms();
                let reservations =
                    receipt_restrictions(request.account, entry.batch.amount, before, after)?;
                let h = self.hold_escrow(request)?;
                h.reservations = reservations;
                h.active = true;
                self.escrows[index].status = Status::Funding;
                self.escrows[index].receipt = Some(receipt.clone());
                self.escrows[index].handoff_slot = Some(*slot);
                self.native_funding_ready = false;
                // Actual facts survive a freeze/shortfall; no admission check.
                // Posting and holds are one atomic control candidate.
            }
            Action::Fund {
                intent, approval, ..
            } => {
                if entry.status != Status::Funding
                    || entry.funding.is_some()
                    || entry.returning.is_some()
                    || intent.request == request
                    || intent.request.account != request.account
                    || intent.source != Location::Broker
                    || intent.destination != Destination::Location(Location::Venue)
                    || intent.net != entry.batch.amount
                    || intent.allow_partial
                    || intent.maximum_fee != QuoteAtoms::new(self.config.quote, 0)
                    || intent.fee_payer != Owner::House
                {
                    return Err(ControlError::Unqualified);
                }
                self.hold_escrow(request)?
                    .reservations
                    .retain(|r| r.resource != Resource::Location(Location::Broker));
                self.funds_action(&funds::Action::Accept {
                    intent: intent.clone(),
                    approval: approval.clone(),
                })?;
                self.escrows[index].funding = Some(intent.request);
            }
            Action::Finish(_) => {
                if entry.status == Status::Available {
                    return Ok(());
                }
                let i = self
                    .funds
                    .iter()
                    .position(|o| Some(o.intent.request) == entry.funding)
                    .ok_or(ControlError::Unqualified)?;
                let o = &self.funds[i];
                let m = self
                    .ledger
                    .movements()
                    .iter()
                    .find(|m| Some(m.mandate.attempt) == o.attempt)
                    .ok_or(ControlError::Unqualified)?;
                if entry.status != Status::Funding
                    || o.recovery
                    || entry.returning.is_some()
                    || o.demo.is_some()
                    || !o.terminal
                    || !self.funds_complete(i)
                    || m.debit != entry.batch.amount
                    || m.arrived != entry.batch.amount
                    || m.fees.atoms() != 0
                    || m.returned.atoms() != 0
                    || m.impaired.atoms() != 0
                {
                    return Err(ControlError::Unqualified);
                }
                self.hold_escrow(request)?.active = false;
                self.escrows[index].status = Status::Available;
            }
            Action::Recall {
                intent, approval, ..
            } => {
                if entry.status != Status::Funding
                    || entry.returning.is_some()
                    || self.recovery_epoch().is_none()
                    || intent.request == request
                    || intent.request.account != request.account
                    || intent.source != Location::Broker
                    || intent.destination != Destination::Location(Location::Vault)
                    || intent.net != entry.batch.amount
                    || intent.maximum_fee != QuoteAtoms::new(self.config.quote, 0)
                    || intent.allow_partial
                    || intent.fee_payer != Owner::House
                {
                    return Err(ControlError::Unqualified);
                }
                if let Some(original) = entry.funding {
                    let i = self
                        .funds
                        .iter()
                        .position(|o| o.intent.request == original)
                        .ok_or(ControlError::Unqualified)?;
                    let o = &self.funds[i];
                    if !o.terminal || o.faulted || o.demo.is_some() {
                        return Err(ControlError::Unqualified);
                    }
                    if let Some(attempt) = o.attempt {
                        let a = self
                            .attempts
                            .iter()
                            .find(|a| a.key == attempt)
                            .ok_or(ControlError::Unqualified)?;
                        let m = self
                            .ledger
                            .movements()
                            .iter()
                            .find(|m| m.mandate.attempt == attempt)
                            .ok_or(ControlError::Unqualified)?;
                        if m.faulted
                            || m.debit.atoms() != 0
                            || m.settled.atoms() != 0
                            || !m.receipts.is_empty()
                            || a.possibly_exposed && !self.funds_complete(i)
                        {
                            return Err(ControlError::Unqualified);
                        }
                    }
                }
                self.hold_escrow(request)?
                    .reservations
                    .retain(|r| r.resource != Resource::Location(Location::Broker));
                self.funds_action(&funds::Action::RecoveryAccept {
                    intent: intent.clone(),
                    approval: approval.clone(),
                })?;
                self.escrows[index].returning = Some(intent.request);
            }
            Action::FinishReturn(_) => {
                if entry.status == Status::Returned {
                    return Ok(());
                }
                let i = self
                    .funds
                    .iter()
                    .position(|o| Some(o.intent.request) == entry.returning)
                    .ok_or(ControlError::Unqualified)?;
                let o = &self.funds[i];
                let m = self
                    .ledger
                    .movements()
                    .iter()
                    .find(|m| Some(m.mandate.attempt) == o.attempt)
                    .ok_or(ControlError::Unqualified)?;
                if entry.status != Status::Funding
                    || !o.recovery
                    || !o.terminal
                    || o.demo.is_some()
                    || !self.funds_complete(i)
                    || m.debit != entry.batch.amount
                    || m.arrived != entry.batch.amount
                    || m.fees.atoms() != 0
                    || m.impaired.atoms() != 0
                    || m.returned.atoms() != 0
                {
                    return Err(ControlError::Unqualified);
                }
                self.hold_escrow(request)?.active = false;
                self.escrows[index].status = Status::Returned;
            }
        }
        Ok(())
    }
    fn hold_escrow(&mut self, request: RequestKey) -> Result<&mut Hold, ControlError> {
        self.holds
            .iter_mut()
            .find(|h| h.request == request)
            .ok_or(ControlError::Invalid)
    }
    pub(crate) fn escrow_exposed(&mut self, key: AttemptKey) {
        if let Some(e) = self.escrows.iter_mut().find(|e| e.attempt == Some(key)) {
            e.status = Status::Exposed;
        }
    }
}
// Both repayment of unabsorbed debt and replenishment after prior absorption
// improve conservative house backing. Restrict the residual gross receipt,
// not merely the change in the house's nominal cash book. No asset is posted
// here: the kernel's single Receipt already applied the existing debt rules.
fn receipt_restrictions(
    account: AccountId,
    amount: QuoteAtoms,
    before: i128,
    after: i128,
) -> Result<Vec<Reservation>, ControlError> {
    let customer = after
        .max(0)
        .checked_sub(before.max(0))
        .ok_or(ControlError::Capacity)?;
    let house = amount
        .atoms()
        .checked_sub(customer)
        .ok_or(ControlError::Capacity)?;
    if customer < 0 || house < 0 {
        return Err(ControlError::Invalid);
    }
    let mut result = vec![Reservation {
        resource: Resource::Location(Location::Broker),
        amount,
    }];
    for (resource, atoms) in [
        (Resource::Customer(account), customer),
        (Resource::House, house),
    ] {
        if atoms > 0 {
            result.push(Reservation {
                resource,
                amount: QuoteAtoms::new(amount.unit(), atoms),
            });
        }
    }
    Ok(result)
}
fn encode_batch(w: &mut Writer, b: &Batch) {
    w.item(&b.request);
    w.raw(&b.route);
    w.raw(&b.batch);
    w.raw(&b.operation);
    w.raw(&b.owner);
    w.item(&b.amount);
    w.u64(b.epoch);
    w.u64(b.expires_at_slot);
    w.raw(&b.signature);
}
fn decode_batch(r: &mut Reader<'_>) -> Result<Batch, Error> {
    Ok(Batch {
        request: r.item()?,
        route: r.array()?,
        batch: r.array()?,
        operation: r.array()?,
        owner: r.array()?,
        amount: r.item()?,
        epoch: r.u64()?,
        expires_at_slot: r.u64()?,
        signature: r.array()?,
    })
}
pub(crate) fn encode_action(w: &mut Writer, a: &Action) {
    match a {
        Action::Pending(b) => {
            w.byte(0);
            encode_batch(w, b);
        }
        Action::Prepare {
            attempt,
            message,
            epoch,
            expires_at,
        } => {
            w.byte(1);
            w.item(attempt);
            w.blob(message.as_bytes());
            w.u64(*epoch);
            w.u64(*expires_at);
        }
        Action::Cancel(k) => {
            w.byte(2);
            w.item(k);
        }
        Action::Refund(k) => {
            w.byte(3);
            w.item(k);
        }
        Action::Failed(k) => {
            w.byte(4);
            w.item(k);
        }
        Action::Handoff {
            attempt,
            receipt,
            slot,
            observed_at,
            raw,
        } => {
            w.byte(5);
            w.item(attempt);
            w.item(receipt);
            w.u64(*slot);
            w.u64(*observed_at);
            w.blob(raw.as_bytes());
        }
        Action::Fund {
            request,
            intent,
            approval,
        } => {
            w.byte(6);
            w.item(request);
            funds::encode_intent(w, intent);
            w.raw(&approval.account.bytes());
            w.raw(&approval.intent_hash);
            w.u64(approval.authority_epoch);
        }
        Action::Finish(k) => {
            w.byte(7);
            w.item(k);
        }
        Action::Unsent(k) => {
            w.byte(8);
            w.item(k);
        }
        Action::Recall {
            request,
            intent,
            approval,
        } => {
            w.byte(9);
            w.item(request);
            funds::encode_intent(w, intent);
            w.raw(&approval.account.bytes());
            w.raw(&approval.intent_hash);
            w.u64(approval.authority_epoch);
        }
        Action::FinishReturn(k) => {
            w.byte(10);
            w.item(k);
        }
    }
}
pub(crate) fn decode_action(r: &mut Reader<'_>) -> Result<Action, Error> {
    Ok(match r.byte()? {
        0 => Action::Pending(Box::new(decode_batch(r)?)),
        1 => Action::Prepare {
            attempt: r.item()?,
            message: PrivateBytes::new(r.blob()?.to_vec())?,
            epoch: r.u64()?,
            expires_at: r.u64()?,
        },
        2 => Action::Cancel(r.item()?),
        3 => Action::Refund(r.item()?),
        4 => Action::Failed(r.item()?),
        5 => Action::Handoff {
            attempt: r.item()?,
            receipt: r.item()?,
            slot: r.u64()?,
            observed_at: r.u64()?,
            raw: PrivateBytes::new(r.blob()?.to_vec())?,
        },
        6 => Action::Fund {
            request: r.item()?,
            intent: Box::new(funds::decode_intent(r)?),
            approval: Approval {
                account: AccountId::new(r.array()?).map_err(|_| Error::Codec)?,
                intent_hash: r.array()?,
                authority_epoch: r.u64()?,
            },
        },
        7 => Action::Finish(r.item()?),
        8 => Action::Unsent(r.item()?),
        9 => Action::Recall {
            request: r.item()?,
            intent: Box::new(funds::decode_intent(r)?),
            approval: Approval {
                account: AccountId::new(r.array()?).map_err(|_| Error::Codec)?,
                intent_hash: r.array()?,
                authority_epoch: r.u64()?,
            },
        },
        10 => Action::FinishReturn(r.item()?),
        _ => return Err(Error::Codec),
    })
}
pub(crate) fn encode_state(w: &mut Writer, state: &State) {
    w.count(state.escrows.len());
    for e in &state.escrows {
        encode_batch(w, &e.batch);
        w.byte(match e.status {
            Status::Pending => 0,
            Status::Prepared => 1,
            Status::Exposed => 2,
            Status::Failed => 3,
            Status::Refunded => 4,
            Status::Funding => 5,
            Status::Available => 6,
            Status::Returned => 7,
        });
        w.option(&e.attempt, |w, k| w.item(k));
        w.option(&e.receipt, |w, k| w.item(k));
        w.option(&e.handoff_slot, |w, s| w.u64(*s));
        w.option(&e.funding, |w, k| w.item(k));
        w.option(&e.returning, |w, k| w.item(k));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gross_restrictions_include_unabsorbed_debt_and_absorbed_replenishment_without_double_counting()
     {
        let account = AccountId::new([1; 32]).unwrap();
        let unit = cinder_kernel::amounts::AssetUnit {
            asset: AssetId::new([3; 32]).unwrap(),
            precision: PrecisionVersion::new(1).unwrap(),
        };
        for (before, after, customer, house) in [
            (100, 200, 100, 0),
            (-20, 80, 80, 20),
            (0, 80, 80, 20),
            (-200, -100, 0, 100),
            (-40, 0, 0, 100),
        ] {
            let rs =
                receipt_restrictions(account, QuoteAtoms::new(unit, 100), before, after).unwrap();
            let held = |resource| {
                rs.iter()
                    .filter(|r| r.resource == resource)
                    .map(|r| r.amount.atoms())
                    .sum::<i128>()
            };
            assert_eq!(held(Resource::Location(Location::Broker)), 100);
            assert_eq!(held(Resource::Customer(account)), customer);
            assert_eq!(held(Resource::House), house);
            assert_eq!(customer + house, 100);
        }
        assert!(receipt_restrictions(account, QuoteAtoms::new(unit, 100), 0, 101).is_err());
    }
}
