//! Trusted enclave worker methods, NOT customer API commands or native permission.
//! Preparation/completion hold the one writer; network egress occurs outside it.
use super::*;
use cinder_bulk::{execution as bulk, observation as bulk_reads};
use cinder_journal::{
    controller::Controller,
    model::{CommitId, Control, Transaction},
    orders,
};
use cinder_kernel::identity::{AttemptId, AttemptKey, RequestKey, UnitId};
use cinder_pacifica::{execution as pacifica, observation as pacifica_reads};

impl<B: Backend, P: Protection, A: cinder_api::Admission> Runtime<B, P, A> {
    /// Prepare an unsigned, durable-budgeted account query, outside network I/O.
    pub fn prepare_bulk_read(
        &self,
        unit: UnitId,
        gateway: &bulk::Gateway,
        mut poll: cinder_bulk::reads::Poll,
    ) -> Result<(cinder_bulk::reads::Request, cinder_bulk::reads::Completion), Error> {
        let mut writer = self.native_writer()?;
        poll.at = self.now()?;
        cinder_bulk::reads::prepare(gateway, &mut writer.select(unit).map_err(|_| Error)?, poll)
            .map_err(|_| Error)
    }
    /// Retain original native query/response linkage and actual financial facts.
    pub fn complete_bulk_read(
        &self,
        unit: UnitId,
        gateway: &bulk::Gateway,
        completion: cinder_bulk::reads::Completion,
        reply: bulk::Reply,
    ) -> Result<cinder_journal::Committed, Error> {
        let mut writer = self.completion_writer()?;
        let at = self.now()?;
        cinder_bulk::reads::complete(
            gateway,
            &mut writer.select(unit).map_err(|_| Error)?,
            completion,
            reply,
            at,
        )
        .map_err(|_| Error)
    }
    pub(super) fn native_writer(&self) -> Result<std::sync::MutexGuard<'_, Journal<B, P>>, Error> {
        self.now()?;
        self.journal.try_lock().map_err(|e| {
            if matches!(e, std::sync::TryLockError::Poisoned(_)) {
                self.fence();
            }
            Error
        })
    }
    /// Trusted worker completion already owns an escaped request's only response
    /// correlation. Wait for the mutation owner instead of discarding it on Busy.
    /// No venue I/O occurs here; ordinary admission/preparation stays try_lock.
    /// Recheck boot fencing and the trusted clock after the wait, before use.
    pub(super) fn completion_writer(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Journal<B, P>>, Error> {
        self.now()?;
        let writer = self.journal.lock().map_err(|_| {
            self.fence();
            Error
        })?;
        self.now()?;
        Ok(writer)
    }
    /// Prepare only an existing financially admitted order. Raw customer/native
    /// actions cannot enter this method; epoch, risk and holds are rechecked.
    pub fn prepare_order(
        &self,
        unit: UnitId,
        request: RequestKey,
        attempt: AttemptId,
        commit: CommitId,
    ) -> Result<AttemptKey, Error> {
        let mut writer = self.native_writer()?;
        let now = self.now()?;
        let mut selected = writer.select(unit).map_err(|_| Error)?;
        let key = AttemptKey { request, attempt };
        let tx = Transaction {
            id: commit,
            expected: cinder_journal::read::Source::head(&selected),
            at: now,
            evidence: vec![],
            inputs: vec![],
            order_observations: vec![],
            funds_observations: vec![],
            controls: vec![Control::Order(orders::Action::Prepare { attempt: key })],
        };
        let result = selected.commit(tx).map_err(|_| Error)?;
        if result.receipt.controls.is_some() {
            return Err(Error);
        }
        Ok(key)
    }
    /// Local key/read-budget epoch installation, not upstream agent registration.
    /// Caller must first qualify and include the loaded gateway in its release.
    pub fn initialize_pacifica(
        &self,
        unit: UnitId,
        gateway: &pacifica::Gateway,
        commit: CommitId,
    ) -> Result<(), Error> {
        let mut writer = self.native_writer()?;
        let now = self.now()?;
        let mut selected = writer.select(unit).map_err(|_| Error)?;
        gateway
            .unit_binding(selected.descriptor().ok_or(Error)?)
            .map_err(|_| Error)?;
        gateway
            .initialize_reads(&mut selected, commit, now)
            .map_err(|_| Error)
    }
    /// Return one signed native request AFTER durable exposure; perform private
    /// TLS egress after this method returns, never under the journal writer.
    pub fn prepare_pacifica(
        &self,
        unit: UnitId,
        gateway: &pacifica::Gateway,
        attempt: AttemptKey,
        commit: CommitId,
    ) -> Result<(pacifica::Outbound, pacifica::DispatchCompletion), Error> {
        let mut writer = self.native_writer()?;
        let at = self.now()?;
        let mut selected = writer.select(unit).map_err(|_| Error)?;
        gateway
            .unit_binding(selected.descriptor().ok_or(Error)?)
            .map_err(|_| Error)?;
        gateway
            .prepare_dispatch(
                &mut selected,
                pacifica::Dispatch {
                    attempt,
                    commit,
                    at,
                },
            )
            .map_err(|_| Error)
    }
    /// Retain actual late response at current trusted time; never resend.
    pub fn complete_pacifica(
        &self,
        unit: UnitId,
        gateway: &pacifica::Gateway,
        completion: pacifica::DispatchCompletion,
        reply: pacifica::Reply,
    ) -> Result<pacifica::Outcome, Error> {
        let mut writer = self.completion_writer()?;
        let at = self.now()?;
        let mut selected = writer.select(unit).map_err(|_| Error)?;
        gateway
            .unit_binding(selected.descriptor().ok_or(Error)?)
            .map_err(|_| Error)?;
        gateway
            .complete_dispatch(&mut selected, completion, reply, at)
            .map_err(|_| Error)
    }
    /// Authenticated native observations, including adverse facts under admission
    /// freeze. Account snapshots never become arbitrary cash setters.
    pub fn observe_pacifica(
        &self,
        unit: UnitId,
        profile: &cinder_pacifica::profile::Profile,
        message: pacifica_reads::Message,
        commit: CommitId,
    ) -> Result<cinder_journal::Committed, Error> {
        let mut writer = self.native_writer()?;
        let at = self.now()?;
        let mut selected = writer.select(unit).map_err(|_| Error)?;
        profile
            .unit_binding(selected.descriptor().ok_or(Error)?)
            .map_err(|_| Error)?;
        pacifica_reads::ingest(&mut selected, profile, message, commit, at).map_err(|_| Error)
    }
    /// Local BULK key epoch only; neither agent registration nor a funds action.
    pub fn initialize_bulk(
        &self,
        unit: UnitId,
        gateway: &bulk::Gateway,
        commit: CommitId,
    ) -> Result<(), Error> {
        let mut writer = self.native_writer()?;
        let at = self.now()?;
        gateway
            .initialize(&mut writer.select(unit).map_err(|_| Error)?, commit, at)
            .map_err(|_| Error)
    }
    /// Prepare a bounded native action with explicit nonce and actual runtime
    /// time. No caller timestamp, automatic nonce, master signer or resend.
    pub fn prepare_bulk(
        &self,
        unit: UnitId,
        gateway: &bulk::Gateway,
        attempt: AttemptKey,
        commit: CommitId,
        nonce: u64,
    ) -> Result<(bulk::Outbound, bulk::Completion), Error> {
        let mut writer = self.native_writer()?;
        let at = self.now()?;
        gateway
            .prepare_dispatch(
                &mut writer.select(unit).map_err(|_| Error)?,
                bulk::Dispatch {
                    attempt,
                    commit,
                    nonce,
                    at,
                },
            )
            .map_err(|_| Error)
    }
    /// Native acknowledgement/unknown is retained separately from actual fills.
    pub fn complete_bulk(
        &self,
        unit: UnitId,
        gateway: &bulk::Gateway,
        completion: bulk::Completion,
        reply: bulk::Reply,
    ) -> Result<bulk::Outcome, Error> {
        let mut writer = self.completion_writer()?;
        let at = self.now()?;
        gateway
            .complete_dispatch(
                &mut writer.select(unit).map_err(|_| Error)?,
                completion,
                reply,
                at,
            )
            .map_err(|_| Error)
    }
    /// Ingest actual native facts even after key revocation; unsupported facts
    /// remain explicitly unresolved and cannot manufacture readiness or credit.
    pub fn observe_bulk(
        &self,
        unit: UnitId,
        gateway: &bulk::Gateway,
        message: bulk_reads::Message,
        commit: CommitId,
    ) -> Result<cinder_journal::Committed, Error> {
        let mut writer = self.native_writer()?;
        let at = self.now()?;
        bulk_reads::ingest(
            gateway,
            &mut writer.select(unit).map_err(|_| Error)?,
            message,
            commit,
            at,
        )
        .map_err(|_| Error)
    }
}
