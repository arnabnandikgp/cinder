//! Trusted enclave custody workers, not public API methods. Existing funding
//! qualification and finality gates remain authoritative; no BULK rail is added.
use super::*;
use cinder_journal::{controller::Controller, model::CommitId, units};
use cinder_kernel::identity::{AttemptKey, EventKey, UnitId};
use cinder_pacifica::{
    execution::{Dispatch, Gateway, Outbound, Reply},
    funding as funds,
};

/// Original qualified chain inventory, with no caller-supplied effective time.
pub struct Inventory {
    /// Durable observation identity.
    pub commit: CommitId,
    /// Exact original owner batch.
    pub batch: cinder_journal::escrow::Batch,
    /// Qualified finalized refund, not a timeout.
    pub refunded: bool,
    /// Authenticated port's retained original evidence; never a host assertion.
    pub raw: PrivateBytes,
}
impl<B: Backend, P: Protection, A: cinder_api::Admission> Runtime<B, P, A> {
    /// Use the existing qualified chain port's non-clone phases. Account/code
    /// reads, simulation and submission never retain the shared journal lock.
    pub fn issue_funds<T: crate::chain_rpc::Transport>(
        &self,
        unit: UnitId,
        controller: &funds::Controller,
        port: &mut crate::chain_funding::Port<T>,
        attempt: AttemptKey,
    ) -> Result<crate::chain_funding::Outcome, Error> {
        let request = {
            let mut w = self.native_writer()?;
            let at = self.now()?;
            port.prepare_issue(
                &mut w.select(unit).map_err(|_| Error)?,
                controller,
                attempt,
                None,
                at,
            )?
        };
        let read = port.collect_issue(request)?;
        let work = {
            let mut w = self.native_writer()?;
            let at = self.now()?;
            read.expose(
                &mut w.select(unit).map_err(|_| Error)?,
                controller,
                None,
                at,
            )?
        };
        let slot = work.slot();
        let simulation = port.simulate_issue(&work)?;
        let delivery = {
            let mut w = self.native_writer()?;
            let at = self.now()?;
            port.persist_issue(
                &mut w.select(unit).map_err(|_| Error)?,
                controller,
                work,
                simulation,
                at,
            )?
        };
        port.submit_issue(delivery, slot)
    }
    /// Observe the original retained signed wire, never generate a replacement.
    pub fn reconcile_funds<T: crate::chain_rpc::Transport>(
        &self,
        unit: UnitId,
        controller: &funds::Controller,
        port: &mut crate::chain_funding::Port<T>,
        attempt: AttemptKey,
    ) -> Result<crate::chain_funding::Outcome, Error> {
        let request = {
            let mut w = self.native_writer()?;
            let at = self.now()?;
            let mut selected = w.select(unit).map_err(|_| Error)?;
            let Some(request) = port.prepare_reconcile(&mut selected, controller, attempt, at)?
            else {
                return Ok(
                    if cinder_journal::read::Source::state(&selected)
                        .map_err(|_| Error)?
                        .funds()
                        .iter()
                        .any(|o| o.attempt == Some(attempt) && o.terminal)
                    {
                        crate::chain_funding::Outcome::Settled
                    } else {
                        crate::chain_funding::Outcome::Pending
                    },
                );
            };
            request
        };
        let read = port.collect_reconcile(request)?;
        let mut w = self.native_writer()?;
        let at = self.now()?;
        read.complete(&mut w.select(unit).map_err(|_| Error)?, controller, at)
    }
    /// Observe only a configured or owner-signed original locator in this unit.
    /// Native/chain reads run outside the writer; a notice is not credit.
    pub fn observe_deposit<T: crate::chain_rpc::Transport>(
        &self,
        unit: UnitId,
        controller: &funds::Controller,
        port: &mut crate::chain_funding::Port<T>,
        index: usize,
    ) -> Result<crate::chain_funding::Outcome, Error> {
        let (route, locator) = {
            let mut w = self.native_writer()?;
            let selected = w.select(unit).map_err(|_| Error)?;
            controller
                .unit_binding(selected.descriptor().ok_or(Error)?)
                .map_err(|_| Error)?;
            let state = cinder_journal::read::Source::state(&selected).map_err(|_| Error)?;
            let locator = port.locators(state).into_iter().nth(index).ok_or(Error)?;
            if crate::escrow_deposit::recorded(&selected, &locator)? {
                return Ok(crate::chain_funding::Outcome::Settled);
            }
            (
                controller.effective_route(state).map_err(|_| Error)?,
                locator,
            )
        };
        let read = port.collect_locator(&route, true, locator)?;
        let mut w = self.native_writer()?;
        self.now()?;
        read.complete(&mut w.select(unit).map_err(|_| Error)?, controller)
    }
    /// Install only the selected physical custody/funding route.
    pub fn bind_funding(
        &self,
        unit: UnitId,
        controller: &funds::Controller,
        commit: CommitId,
    ) -> Result<(), Error> {
        let mut w = self.native_writer()?;
        let at = self.now()?;
        controller
            .bind(&mut w.select(unit).map_err(|_| Error)?, commit, at)
            .map_err(|_| Error)
    }
    /// Retain exact-batch inventory without creating trading credit.
    pub fn observe_inventory(
        &self,
        unit: UnitId,
        controller: &funds::Controller,
        input: Inventory,
    ) -> Result<(), Error> {
        let mut w = self.native_writer()?;
        let at = self.now()?;
        controller
            .observe_escrow_inventory(
                &mut w.select(unit).map_err(|_| Error)?,
                input.commit,
                at,
                input.batch,
                input.refunded,
                input.raw,
            )
            .map_err(|_| Error)
    }
    /// Continue only the original full-batch consent. An already exposed attempt
    /// is reconciliation-only; this method returns no native/chain delivery.
    pub fn prepare_escrow(
        &self,
        unit: UnitId,
        controller: &funds::Controller,
    ) -> Result<Option<AttemptKey>, Error> {
        let mut w = self.native_writer()?;
        let at = self.now()?;
        controller
            .prepare_next_escrow(&mut w.select(unit).map_err(|_| Error)?, at)
            .map_err(|_| Error)
    }
    /// Persist one physical sweep plan before any chain signing outside the writer.
    pub fn expose_sweep(
        &self,
        unit: UnitId,
        controller: &funds::Controller,
        mut dispatch: Dispatch,
        counters: funds::Counters,
    ) -> Result<funds::ChainAction, Error> {
        let mut w = self.native_writer()?;
        dispatch.at = self.now()?;
        controller
            .expose_escrow_sweep(&mut w.select(unit).map_err(|_| Error)?, dispatch, counters)
            .map_err(|_| Error)
    }
    /// Same original physical rail capability for deposit, return or beneficiary
    /// payout. Existing route/counter/hold checks still precede all signing.
    pub fn expose_funds(
        &self,
        unit: UnitId,
        controller: &funds::Controller,
        mut dispatch: Dispatch,
        rail: funds::Rail,
        counters: funds::Counters,
    ) -> Result<funds::ChainAction, Error> {
        let mut w = self.native_writer()?;
        dispatch.at = self.now()?;
        controller
            .expose_chain(
                &mut w.select(unit).map_err(|_| Error)?,
                dispatch,
                rail,
                counters,
            )
            .map_err(|_| Error)
    }
    /// Retain the exact verified signed wire before one-shot submission. Never
    /// construct a replacement blockhash/signature after an uncertain commit.
    pub fn persist_funds_wire(
        &self,
        unit: UnitId,
        controller: &funds::Controller,
        commit: CommitId,
        action: funds::ChainAction,
        wire: funds::VerifiedWire,
    ) -> Result<funds::ChainDelivery, Error> {
        let mut w = self.native_writer()?;
        let at = self.now()?;
        controller
            .persist_wire(
                &mut w.select(unit).map_err(|_| Error)?,
                commit,
                at,
                action,
                wire,
            )
            .map_err(|_| Error)
    }
    /// Qualified original chain facts, not ACKs or a balance delta alone.
    pub fn observe_funds_chain(
        &self,
        unit: UnitId,
        controller: &funds::Controller,
        commit: CommitId,
        receipt: funds::ChainReceipt,
    ) -> Result<(), Error> {
        let mut w = self.native_writer()?;
        let at = self.now()?;
        controller
            .observe_chain(&mut w.select(unit).map_err(|_| Error)?, commit, at, receipt)
            .map_err(|_| Error)
    }
    /// Native credit still requires the existing operation-linked evidence.
    /// Missing/uncertain credit leaves original transit/holds in place.
    pub fn observe_native_credit(
        &self,
        unit: UnitId,
        controller: &funds::Controller,
        commit: CommitId,
        credit: funds::Credit,
    ) -> Result<(), Error> {
        let mut w = self.native_writer()?;
        let at = self.now()?;
        controller
            .observe_credit(&mut w.select(unit).map_err(|_| Error)?, commit, at, credit)
            .map_err(|_| Error)
    }
    /// Prepare a native owner-directed return request; no network under writer.
    pub fn prepare_funds_withdrawal(
        &self,
        unit: UnitId,
        controller: &funds::Controller,
        gateway: &Gateway,
        mut dispatch: Dispatch,
    ) -> Result<(Outbound, funds::WithdrawalCompletion), Error> {
        let mut w = self.native_writer()?;
        dispatch.at = self.now()?;
        let mut selected = w.select(unit).map_err(|_| Error)?;
        gateway
            .unit_binding(selected.descriptor().ok_or(Error)?)
            .map_err(|_| Error)?;
        controller
            .prepare_withdrawal(&mut selected, gateway, dispatch)
            .map_err(|_| Error)
    }
    /// Actual late return acknowledgment; neither payout nor terminal proof.
    pub fn complete_funds_withdrawal(
        &self,
        unit: UnitId,
        controller: &funds::Controller,
        gateway: &Gateway,
        completion: funds::WithdrawalCompletion,
        reply: Reply,
    ) -> Result<(), Error> {
        let mut w = self.native_writer()?;
        let at = self.now()?;
        controller
            .complete_withdrawal(
                &mut w.select(unit).map_err(|_| Error)?,
                gateway,
                completion,
                reply,
                at,
            )
            .map_err(|_| Error)
    }
    /// Atomic deployment-wide cutover after independently qualified custody,
    /// native-key and writer fencing. This is not an ordinary customer command.
    pub fn begin_recovery(
        &self,
        expected: cinder_journal::Head,
        commit: CommitId,
        epoch: u64,
        valid_until: u64,
        fence: [u8; 32],
    ) -> Result<(), Error> {
        let mut w = self.native_writer()?;
        w.verified_state().map_err(|_| Error)?;
        if w.head() != expected {
            return Err(Error);
        }
        let versions = w
            .configuration()
            .units()
            .iter()
            .map(|u| {
                Ok((
                    u.id,
                    w.state()
                        .map_err(|_| Error)?
                        .unit(u.id)
                        .ok_or(Error)?
                        .ledger()
                        .version(),
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let tx = units::Transaction {
            id: commit,
            expected,
            at: self.now()?,
            change: units::Change::BeginRecovery {
                versions,
                authority_epoch: epoch,
                valid_until,
                fence,
            },
        };
        w.commit(tx).map_err(|_| Error)?;
        Ok(())
    }
    /// Backed settled cut after the global cutover; returns no root activation or
    /// payout permission. Existing lifetime paid counters are checked, not reset.
    pub fn recovery_backing(
        &self,
        unit: UnitId,
        controller: &funds::Controller,
        expected: cinder_journal::Head,
        check: EventKey,
        observed: &funds::recovery::CustodyState,
    ) -> Result<funds::recovery::Backing, Error> {
        let mut w = self.native_writer()?;
        self.now()?;
        controller
            .recovery_backing(
                &mut w.select(unit).map_err(|_| Error)?,
                expected,
                check,
                observed,
            )
            .map_err(|_| Error)
    }
}
