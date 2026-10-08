//! Private, non-clone phases for the runtime's single financial mutation owner.
//! RPC reads, simulation, send and finality I/O never require a journal guard.
use super::*;
use cinder_pacifica::funding::{
    ChainAction, ChainDelivery, ChainReceipt, Route, VerifiedWire, demo,
};

type Demo<'a> = Option<(&'a Gateway, &'a demo::Policy)>;

pub(crate) struct IssueRequest {
    port: [u8; 32],
    funding: [u8; 32],
    route: Route,
    attempt: AttemptKey,
    rail: Rail,
    recovery: bool,
    commit: CommitId,
    at: u64,
}
pub(crate) struct IssueRead {
    request: IssueRequest,
    context: chain::Context,
    counters: cinder_pacifica::funding::Counters,
    limits: Limits,
    code_at: u64,
}
pub(crate) struct IssueWork {
    action: ChainAction,
    context: chain::Context,
    limits: Limits,
    funding: [u8; 32],
    port: [u8; 32],
}
impl IssueWork {
    pub(crate) fn slot(&self) -> u64 {
        self.context.slot
    }
}
pub(crate) struct ReconcileRequest {
    port: [u8; 32],
    funding: [u8; 32],
    contract: cinder_journal::model::PrivateBytes,
    wire: VerifiedWire,
}
pub(crate) struct ReconcileRead {
    request: ReconcileRequest,
    receipt: Option<ChainReceipt>,
}

impl<T: Transport> Port<T> {
    pub(crate) fn prepare_issue<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        funding: &Controller,
        attempt: AttemptKey,
        demo: Demo<'_>,
        at: u64,
    ) -> Result<IssueRequest, Error> {
        let (rail, recovery) = rail(j, attempt)?;
        if j.state()
            .map_err(|_| Error)?
            .attempts()
            .iter()
            .find(|a| a.key == attempt)
            .is_none_or(|a| a.possibly_exposed)
        {
            return Err(Error);
        }
        let commit = id()?;
        if let Some((gateway, policy)) = demo {
            if recovery || !matches!(rail, Rail::Release | Rail::Deposit) {
                return Err(Error);
            }
            funding
                .validate_demo_ingress(
                    j,
                    gateway,
                    policy,
                    Dispatch {
                        attempt,
                        commit,
                        at,
                    },
                )
                .map_err(|_| Error)?;
        }
        Ok(IssueRequest {
            port: self.loaded.commitment(),
            funding: funding
                .release_commitment(j.configuration())
                .map_err(|_| Error)?,
            route: funding.route().clone(),
            attempt,
            rail,
            recovery,
            commit,
            at,
        })
    }
    pub(crate) fn collect_issue(&mut self, request: IssueRequest) -> Result<IssueRead, Error> {
        if request.port != self.loaded.commitment() {
            return Err(Error);
        }
        let c = &self.loaded.configuration;
        let d = if request.rail == Rail::Deposit {
            &c.native
        } else {
            &c.custody
        };
        let (slot, at) = self.client.slot(self.clock.now()?)?;
        let proof =
            chain_code::verify(&mut self.client, d, self.loaded.endpoint.network, slot, at)?;
        let r = &request.route;
        let b = r
            .beneficiaries
            .iter()
            .find(|b| b.account == request.attempt.request.account.bytes())
            .ok_or(Error)?;
        let mut keys = vec![r.config];
        if request.rail == Rail::Payout {
            keys.extend([
                chain::pda(r.program, &[b"customer", &r.config, &b.wallet]).map_err(|_| Error)?,
                b.tokens,
            ]);
        }
        let accounts = self.client.accounts(&keys, slot, proof.at())?;
        let context = self.client.context(self.clock.now()?)?;
        let epoch = if request.recovery && request.rail == Rail::Return {
            r.epoch.checked_add(1).ok_or(Error)?
        } else {
            r.epoch
        };
        let counters = chain_receipt::counters(
            &accounts,
            r,
            request.rail,
            b.wallet,
            b.tokens,
            epoch,
            context.slot.checked_add(c.expiry_slots).ok_or(Error)?,
        )?;
        Ok(IssueRead {
            request,
            context,
            counters,
            limits: c.limits,
            code_at: proof.at(),
        })
    }
    pub(crate) fn simulate_issue(&mut self, work: &IssueWork) -> Result<chain::Simulation, Error> {
        if work.port != self.loaded.commitment() {
            return Err(Error);
        }
        let prepared = chain::prepare(&work.action, work.context, work.limits, self.clock.now()?)
            .map_err(|_| Error)?;
        self.client.simulate(&prepared, self.clock.now()?)
    }
    pub(crate) fn persist_issue<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        funding: &Controller,
        work: IssueWork,
        simulation: chain::Simulation,
        at: u64,
    ) -> Result<ChainDelivery, Error> {
        if work.port != self.loaded.commitment()
            || funding
                .release_commitment(j.configuration())
                .map_err(|_| Error)?
                != work.funding
        {
            return Err(Error);
        }
        // Current authority is checked BEFORE local signature creation as well
        // as at persistence. A concurrent revoke/freeze during simulation refuses.
        funding
            .validate_chain_signing(j, &work.action, at)
            .map_err(|_| Error)?;
        let prepared =
            chain::prepare(&work.action, work.context, work.limits, at).map_err(|_| Error)?;
        let signed = if matches!(work.action.plan().rail(), Rail::Release | Rail::Payout) {
            prepared
                .sign(self.loaded.funds.clone(), simulation, at)
                .map_err(|_| Error)?
        } else {
            funding
                .sign_chain(prepared, simulation, at)
                .map_err(|_| Error)?
        };
        funding
            .persist_wire(j, id()?, at, work.action, signed)
            .map_err(|_| Error)
    }
    pub(crate) fn submit_issue(
        &mut self,
        delivery: ChainDelivery,
        slot: u64,
    ) -> Result<Outcome, Error> {
        Ok(
            match self.client.submit(delivery, slot, self.clock.now()?) {
                Reply::Response { .. } => Outcome::Submitted,
                _ => Outcome::Unknown,
            },
        )
    }
    pub(crate) fn prepare_reconcile<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        funding: &Controller,
        attempt: AttemptKey,
        at: u64,
    ) -> Result<Option<ReconcileRequest>, Error> {
        let Some(wire) = funding.retained_wire(j, attempt, at).map_err(|_| Error)? else {
            funding
                .close_unsent_chain(j, id()?, at, attempt)
                .map_err(|_| Error)?;
            return Ok(None);
        };
        let contract = funding
            .original_chain_contract(j, attempt, at)
            .map_err(|_| Error)?;
        Ok(Some(ReconcileRequest {
            port: self.loaded.commitment(),
            funding: funding
                .release_commitment(j.configuration())
                .map_err(|_| Error)?,
            contract,
            wire,
        }))
    }
    pub(crate) fn collect_reconcile(
        &mut self,
        request: ReconcileRequest,
    ) -> Result<ReconcileRead, Error> {
        if request.port != self.loaded.commitment() {
            return Err(Error);
        }
        let target = chain::inspect(
            request.contract.as_bytes(),
            request.wire.attempt,
            request.wire.wire.as_bytes(),
        )
        .map_err(|_| Error)?;
        let Some(tx) = self
            .client
            .finalized(request.wire.signature, self.clock.now()?)?
        else {
            return Ok(ReconcileRead {
                request,
                receipt: None,
            });
        };
        let c = &self.loaded.configuration;
        let d = if target.rail == Rail::Deposit {
            &c.native
        } else {
            &c.custody
        };
        let proof = chain_code::verify(
            &mut self.client,
            d,
            self.loaded.endpoint.network,
            tx.slot,
            tx.at,
        )?;
        let keys = chain_receipt::effect_keys(&target, d)?;
        let accounts = self.client.accounts(&keys, tx.slot, proof.at())?;
        let receipt = chain_receipt::recognize_verified(
            request.contract.as_bytes(),
            request.wire.attempt,
            request.wire.wire.as_bytes(),
            tx,
            accounts,
            d,
            c.limits.maximum_fee_lamports,
            proof,
        )?;
        Ok(ReconcileRead {
            request,
            receipt: Some(receipt),
        })
    }
}
impl IssueRead {
    pub(crate) fn expose<B: Backend, P: Protection>(
        self,
        j: &mut Journal<B, P>,
        funding: &Controller,
        demo: Demo<'_>,
        at: u64,
    ) -> Result<IssueWork, Error> {
        if funding
            .release_commitment(j.configuration())
            .map_err(|_| Error)?
            != self.request.funding
            || funding.route() != &self.request.route
            || at < self.request.at
            || at < self.context.at
            || at - self.context.at > self.limits.maximum_age_ms
            || at < self.code_at
            || at - self.code_at > self.limits.maximum_age_ms
        {
            return Err(Error);
        }
        let dispatch = Dispatch {
            attempt: self.request.attempt,
            commit: self.request.commit,
            at,
        };
        let action = match demo {
            Some((gateway, policy)) => {
                funding.expose_demo_ingress(j, gateway, policy, dispatch, self.counters)
            }
            None => funding.expose_chain(j, dispatch, self.request.rail, self.counters),
        }
        .map_err(|_| Error)?;
        Ok(IssueWork {
            action,
            context: self.context,
            limits: self.limits,
            funding: self.request.funding,
            port: self.request.port,
        })
    }
}
impl ReconcileRead {
    pub(crate) fn complete<B: Backend, P: Protection>(
        self,
        j: &mut Journal<B, P>,
        funding: &Controller,
        at: u64,
    ) -> Result<Outcome, Error> {
        if funding
            .release_commitment(j.configuration())
            .map_err(|_| Error)?
            != self.request.funding
            || funding
                .original_chain_contract(j, self.request.wire.attempt, at)
                .map_err(|_| Error)?
                != self.request.contract
        {
            return Err(Error);
        }
        j.verified_state().map_err(|_| Error)?;
        if let Some(receipt) = self.receipt {
            funding
                .observe_chain(j, id()?, at, receipt)
                .map_err(|_| Error)?;
        }
        Ok(
            if j.state()
                .map_err(|_| Error)?
                .funds()
                .iter()
                .any(|o| o.attempt == Some(self.request.wire.attempt) && o.terminal)
            {
                Outcome::Settled
            } else {
                Outcome::Pending
            },
        )
    }
}
