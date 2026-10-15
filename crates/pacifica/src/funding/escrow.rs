//! Explicit new-history escrow composition. Legacy and demo constructors cannot
//! gain sweep authority from an untrusted action/route field.
use super::*;
use cinder_journal::escrow::{Action, Batch, Status};

/// Immutable additional loaded binding for exact-batch escrow ingress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Routing {
    /// Canonical venue-route PDA for this custody config.
    pub route: [u8; 32],
    /// Actual venue identity, not a UI label or native account ID.
    pub venue: [u8; 32],
}
impl Controller {
    pub(super) fn close_unsent_escrow<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        id: CommitId,
        at: u64,
        plan: &Plan,
        history: &History,
    ) -> Result<bool, Error> {
        if at < plan.expires_at || history.wires.iter().any(|(a, _, _)| *a == plan.attempt) {
            return Ok(false);
        }
        let attempt = plan.attempt()?;
        let e = j
            .verified_state()?
            .escrows()
            .iter()
            .find(|e| e.attempt == Some(attempt))
            .ok_or(Error::Qualification)?;
        self.validate_batch(&e.batch)?;
        if history.chains.iter().any(|(a, _, _)| *a == plan.attempt) || e.receipt.is_some() {
            return Err(Error::Qualification);
        }
        if e.status == Status::Failed && history.unsent.contains(&plan.attempt) {
            return Ok(true);
        }
        let mut tx = self.tx(
            j,
            id,
            at,
            Record::Unsent {
                attempt: plan.attempt.clone(),
            },
        )?;
        tx.controls.push(Control::Escrow(Action::Unsent(attempt)));
        checked(j.commit(tx)?)?;
        Ok(true)
    }
    /// Separate new-journal composition, never an implicit history migration or
    /// relaxed demo-native qualification. All original strong rail gates remain.
    pub fn new_escrow(
        profile: Profile,
        route: Route,
        routing: Routing,
        seed: Zeroizing<[u8; 32]>,
    ) -> Result<Self, Error> {
        if routing.venue != profile.config.venue.bytes()
            || routing.route != chain::pda(route.program, &[b"venue_route", &route.config])?
        {
            return Err(Error::Qualification);
        }
        let mut c = Self::new(profile, route, seed)?;
        let mut h = Sha256::new();
        h.update(b"CINDER-ESCROW-FUNDING-2\0");
        h.update(c.contract);
        h.update(serde_json::to_vec(&routing).map_err(|_| Error::Codec)?);
        c.contract = h.finalize().into();
        c.escrow = Some(routing);
        Ok(c)
    }
    /// Explicit loaded binding, absent on legacy/demo compositions.
    pub fn escrow_routing(&self) -> Option<&Routing> {
        self.escrow.as_ref()
    }

    fn validate_batch(&self, batch: &Batch) -> Result<chain::Escrow, Error> {
        let routing = self.escrow.as_ref().ok_or(Error::Qualification)?;
        let b = self
            .route
            .beneficiaries
            .iter()
            .find(|b| b.account == batch.request.account.bytes())
            .ok_or(Error::Qualification)?;
        let s = chain::Escrow::derive(
            &self.route,
            b.wallet,
            batch.operation,
            batch.expires_at_slot,
        )?;
        if batch.request.domain != self.profile.config.domain
            || batch.owner != b.wallet
            || batch.route != routing.route
            || batch.batch != s.batch
            || batch.epoch != self.route.epoch
            || batch.signature == [0; 64]
            || batch.amount.unit() != self.profile.config.quote
            || batch.amount.atoms() <= 0
            || batch.amount.atoms() > i128::from(self.route.maximum_movement)
        {
            return Err(Error::Qualification);
        }
        Ok(s)
    }
    /// Retain qualified original deposit evidence and pending owner inventory;
    /// this posts no asset, claim, available collateral or paid counter.
    pub fn observe_escrow_deposit<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        id: CommitId,
        at: u64,
        batch: Batch,
        raw: PrivateBytes,
    ) -> Result<(), Error> {
        self.observe_escrow_inventory(j, id, at, batch, false, raw)
    }
    /// Atomically retain the original inventory and an already-finalized refund.
    #[allow(clippy::too_many_arguments)]
    pub fn observe_escrow_inventory<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        id: CommitId,
        at: u64,
        batch: Batch,
        refunded: bool,
        raw: PrivateBytes,
    ) -> Result<(), Error> {
        if !self.history(j, at)?.bound || raw.as_bytes().is_empty() {
            return Err(Error::Qualification);
        }
        self.validate_batch(&batch)?;
        if let Some(e) = j
            .state()?
            .escrows()
            .iter()
            .find(|e| e.batch.request == batch.request)
        {
            if e.batch != batch {
                return Err(Error::Qualification);
            }
            if !refunded || e.status == Status::Refunded {
                return Ok(());
            }
        }
        let request = batch.request;
        let mut tx = self.tx(
            j,
            id,
            at,
            Record::Ingress {
                raw: raw.as_bytes().to_vec(),
            },
        )?;
        tx.controls
            .push(Control::Escrow(Action::Pending(Box::new(batch))));
        if refunded {
            tx.controls.push(Control::Escrow(Action::Refund(request)));
        }
        checked(j.commit(tx)?)
    }
    /// Retain actual qualified terminal refund, even during freeze/recovery.
    pub fn observe_escrow_refund<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        id: CommitId,
        at: u64,
        request: RequestKey,
        raw: PrivateBytes,
    ) -> Result<(), Error> {
        self.history(j, at)?;
        let e = j
            .state()?
            .escrows()
            .iter()
            .find(|e| e.batch.request == request)
            .ok_or(Error::Qualification)?;
        self.validate_batch(&e.batch)?;
        if raw.as_bytes().is_empty() {
            return Err(Error::Qualification);
        }
        let mut tx = self.tx(
            j,
            id,
            at,
            Record::Ingress {
                raw: raw.as_bytes().to_vec(),
            },
        )?;
        tx.controls.push(Control::Escrow(Action::Refund(request)));
        checked(j.commit(tx)?)
    }
    /// Durable abstract intent first; no signature, counter read or RPC is
    /// performed under this journal mutation. Only the original batch is eligible.
    pub fn prepare_escrow_sweep<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        id: CommitId,
        at: u64,
        attempt: AttemptKey,
        expires_at: u64,
    ) -> Result<(), Error> {
        let history = self.history(j, at)?;
        if !self.ready(&history, at) {
            return Err(Error::Qualification);
        }
        let e = j
            .state()?
            .escrows()
            .iter()
            .find(|e| e.batch.request == attempt.request)
            .ok_or(Error::Qualification)?;
        let s = self.validate_batch(&e.batch)?;
        let message=PrivateBytes::new(serde_json::to_vec(&json!({"schema":"cinder-escrow-intent-v2","contract":self.contract,
            "batch":s,"amount":e.batch.amount.atoms().to_string(),"epoch":e.batch.epoch.to_string(),"expires_at":expires_at.to_string()})).map_err(|_|Error::Codec)?)?;
        let mut tx = self.tx(j, id, at, Record::Ingress { raw: vec![] })?;
        tx.controls.push(Control::Escrow(Action::Prepare {
            attempt,
            message,
            epoch: self.route.epoch,
            expires_at,
        }));
        checked(j.commit(tx)?)
    }
    /// Physical counter/slot-bound plan, durably exposed once before signing.
    pub fn expose_escrow_sweep<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        dispatch: Dispatch,
        counters: Counters,
    ) -> Result<ChainAction, Error> {
        let h = self.history(j, dispatch.at)?;
        if !self.ready(&h, dispatch.at) || j.transaction(dispatch.commit).is_some() {
            return Err(Error::Qualification);
        }
        let state = j.state()?;
        let e = state
            .escrows()
            .iter()
            .find(|e| e.attempt == Some(dispatch.attempt))
            .ok_or(Error::Qualification)?;
        let s = self.validate_batch(&e.batch)?;
        let a = state
            .attempts()
            .iter()
            .find(|a| a.key == dispatch.attempt)
            .ok_or(Error::Qualification)?;
        if e.status != Status::Prepared
            || a.kind != AttemptKind::Escrow
            || a.possibly_exposed
            || a.expires_at <= dispatch.at
            || counters.expires_at_slot == 0
            || counters.expires_at_slot > e.batch.expires_at_slot
            || counters.paid != 0
            || counters.recipient_tokens != [0; 32]
            || h.plans.iter().any(|p| p.attempt == a.key.encode())
        {
            return Err(Error::Qualification);
        }
        let mut hash = Sha256::new();
        hash.update(b"CINDER-CUSTODY-OPERATION-1\0");
        hash.update(self.contract);
        hash.update(a.key.encode());
        let operation = hash.finalize().into();
        if h.plans.iter().any(|p| p.operation == operation) {
            return Err(Error::Qualification);
        }
        let amount = u64::try_from(e.batch.amount.atoms()).map_err(|_| Error::Limit)?;
        let plan = Plan {
            sweep: Some(s),
            rail: Rail::Sweep,
            attempt: a.key.encode(),
            abstract_action: a.message.as_bytes().to_vec(),
            net: amount,
            gross: amount,
            operation,
            epoch: self.route.epoch,
            customer: e.batch.owner,
            counters,
            at: dispatch.at,
            expires_at: a.expires_at,
            demo_setup: None,
        };
        self.expose_chain_plan(j, dispatch, plan)
    }
    pub(super) fn qualified_chain_delivery(
        &self,
        state: &State,
        attempt: AttemptKey,
        rail: Rail,
    ) -> Result<(), Error> {
        (if rail == Rail::Sweep {
            state.qualified_escrow_delivery(attempt)
        } else {
            state.qualified_funds_delivery(attempt)
        })
        .map_err(|_| Error::Qualification)
    }
    /// Fenced recovery operator recalls only original, unallocated collateral.
    /// The internal Approval is a trusted operator digest, not a user signature;
    /// unknown or partially executed native funding cannot use this return.
    pub fn prepare_escrow_recall<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        id: CommitId,
        at: u64,
        original: RequestKey,
        attempt: AttemptKey,
        expires_at: u64,
    ) -> Result<(), Error> {
        self.history(j, at)?;
        let entry = j
            .state()?
            .escrows()
            .iter()
            .find(|e| e.batch.request == original)
            .ok_or(Error::Qualification)?;
        self.validate_batch(&entry.batch)?;
        let epoch = j.state()?.recovery_epoch().ok_or(Error::Qualification)?;
        let intent = funds::Intent {
            request: attempt.request,
            source: Location::Broker,
            destination: Destination::Location(Location::Vault),
            net: entry.batch.amount,
            maximum_fee: QuoteAtoms::new(self.profile.config.quote, 0),
            fee_payer: Owner::House,
            allow_partial: false,
            policy: self.profile.config.policy,
            authority_epoch: epoch,
            expires_at,
        };
        let approval = cinder_journal::orders::Approval {
            account: intent.request.account,
            authority_epoch: epoch,
            intent_hash: intent.digest()?,
        };
        let mut tx = self.tx(
            j,
            id,
            at,
            Record::Ingress {
                raw: b"qualified recovery-only original escrow recall".to_vec(),
            },
        )?;
        tx.controls.push(Control::Escrow(Action::Recall {
            request: original,
            intent: Box::new(intent),
            approval,
        }));
        tx.controls.push(Control::Funds(funds::Action::Prepare {
            attempt,
            net: entry.batch.amount,
        }));
        checked(j.commit(tx)?)
    }
    /// Continue only the original owner's full-batch venue-funding consent.
    /// Internal Approval is a trusted digest, not a wallet signature, payout
    /// authorization or external signing API. Reconcile exposed work first,
    /// including while new admission is frozen or setup freshness has expired.
    pub fn prepare_next_escrow<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        at: u64,
    ) -> Result<Option<AttemptKey>, Error> {
        if self.escrow.is_none() {
            return Ok(None);
        }
        let h = self.history(j, at)?;
        let candidate = j
            .state()?
            .escrows()
            .iter()
            .filter(|e| {
                matches!(
                    e.status,
                    Status::Pending | Status::Prepared | Status::Exposed
                ) || e.status == Status::Funding && e.funding.is_none() && e.returning.is_none()
            })
            .min_by_key(|e| u8::from(e.status != Status::Exposed))
            .cloned();
        let Some(e) = candidate else { return Ok(None) };
        self.validate_batch(&e.batch)?;
        if e.status == Status::Exposed {
            return Ok(e.attempt);
        }
        if !self.ready(&h, at) || j.state()?.frozen() {
            return Ok(None);
        }
        if e.status == Status::Prepared {
            return Ok(e.attempt);
        }
        let identity = |purpose: &[u8]| -> [u8; 32] {
            let mut h = Sha256::new();
            h.update(purpose);
            h.update(self.contract);
            h.update(e.batch.batch);
            h.finalize().into()
        };
        let expiry = at
            .checked_add(self.route.setup_max_age)
            .ok_or(Error::Limit)?;
        if e.status == Status::Pending {
            let attempt = AttemptKey {
                request: e.batch.request,
                attempt: AttemptId::new(identity(b"CINDER-ESCROW-SWEEP-2\0"))?,
            };
            self.prepare_escrow_sweep(
                j,
                CommitId::new(identity(b"CINDER-ESCROW-PREPARE-2\0"))?,
                at,
                attempt,
                expiry,
            )?;
            return Ok(Some(attempt));
        }
        let epoch = j
            .state()?
            .authority_epoch(e.batch.request.account)
            .ok_or(Error::Qualification)?;
        let intent = funds::Intent {
            request: RequestKey {
                domain: e.batch.request.domain,
                account: e.batch.request.account,
                request: RequestId::new(identity(b"CINDER-ESCROW-NATIVE-2\0"))?,
            },
            source: Location::Broker,
            destination: Destination::Location(Location::Venue),
            net: e.batch.amount,
            maximum_fee: QuoteAtoms::new(self.profile.config.quote, 0),
            fee_payer: Owner::House,
            allow_partial: false,
            policy: self.profile.config.policy,
            authority_epoch: epoch,
            expires_at: expiry,
        };
        let approval = cinder_journal::orders::Approval {
            account: intent.request.account,
            authority_epoch: epoch,
            intent_hash: intent.digest()?,
        };
        let mut tx = self.tx(
            j,
            CommitId::new(identity(b"CINDER-ESCROW-FUND-2\0"))?,
            at,
            Record::Ingress { raw: vec![] },
        )?;
        tx.controls.push(Control::Escrow(Action::Fund {
            request: e.batch.request,
            intent: Box::new(intent),
            approval,
        }));
        checked(j.commit(tx)?)?;
        Ok(None)
    }
    pub(super) fn observe_sweep<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        id: CommitId,
        at: u64,
        plan: Plan,
        r: ChainReceipt,
    ) -> Result<(), Error> {
        let h = self.history(j, at)?;
        let e = j
            .state()?
            .escrows()
            .iter()
            .find(|e| e.attempt == Some(r.attempt))
            .ok_or(Error::Qualification)?;
        let s = self.validate_batch(&e.batch)?;
        if plan.sweep.as_ref() != Some(&s)
            || r.network != self.profile.config.domain.network.bytes()
            || r.program != self.route.program
            || r.mint != self.route.mint
            || r.source != s.tokens
            || r.destination != self.route.broker_tokens
            || r.signature == [0; 64]
            || r.slot == 0
            || r.succeeded
                && (r.amount != plan.gross
                    || r.operation != Some(plan.operation)
                    || r.epoch != Some(plan.epoch)
                    || r.config != Some(self.route.config)
                    || r.sequence != plan.counters.sequence.checked_add(1))
            || !r.succeeded && r.amount != 0
            || !h
                .wires
                .iter()
                .any(|(a, s, _)| *a == plan.attempt && *s == r.signature)
            || h.chains
                .iter()
                .any(|(a, s, ok)| *a == plan.attempt && (*s != r.signature || *ok != r.succeeded))
        {
            return Err(Error::Qualification);
        }
        let receipt = e.batch.handoff(self.scope(Location::Broker)?)?;
        let mut tx = self.tx(
            j,
            id,
            at,
            Record::Chain {
                attempt: plan.attempt,
                signature: r.signature.to_vec(),
                succeeded: r.succeeded,
            },
        )?;
        if r.succeeded {
            tx.controls.push(Control::Escrow(Action::Handoff {
                attempt: r.attempt,
                receipt,
                slot: r.slot,
                observed_at: at,
                raw: r.raw,
            }));
        } else {
            tx.evidence.push(r.raw);
            tx.controls.push(Control::Escrow(Action::Failed(r.attempt)));
        }
        checked(j.commit(tx)?)
    }
}
