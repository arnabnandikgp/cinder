//! Unit-scoped application controller. Transport still owns witnessed final
//! release; neither a projection nor a mutation reply grants native authority.
use super::*;
use cinder_journal::read::{Source, Verified};
use std::sync::{Arc, Mutex};

/// One derived cache and the existing financial projections; no second ledger.
pub struct Service<A> {
    auth: Authorizer,
    admission: A,
    cache: Mutex<history::Cache>,
}
impl<A> fmt::Debug for Service<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UnitService([PRIVATE])")
    }
}
impl<A: crate::Admission> Service<A> {
    /// Explicit governed lifetimes and trusted server-side hold computation.
    pub fn new(auth: Authorizer, admission: A) -> Self {
        Self {
            auth,
            admission,
            cache: Mutex::new(history::Cache::default()),
        }
    }
    // Build the existing projector with a valid, actual selected-owner binding.
    // Never manufacture an invalid legacy Service or use it for v2 auth.
    fn projection(
        &self,
        state: &units::State,
        unit: UnitId,
        account: AccountId,
    ) -> Result<crate::Service<ProjectionAdmission<'_, A>>, Error> {
        let owner = state
            .customers()
            .iter()
            .find(|c| c.account == account)
            .ok_or(Error::Unavailable)?;
        let tokens = state
            .unit(unit)
            .ok_or(Error::Unavailable)?
            .registrations()
            .iter()
            .find(|r| r.account == account)
            .ok_or(Error::Unavailable)?
            .tokens;
        crate::Service::new(
            crate::Contract {
                owners: vec![crate::OwnerBinding {
                    account,
                    wallet: owner.wallet,
                    tokens,
                }],
                maximum_auth_lifetime: self.auth.maximum_auth_lifetime,
                maximum_grant_lifetime: self.auth.maximum_grant_lifetime,
            },
            ProjectionAdmission(&self.admission),
        )
    }
    /// Commitment installed in the shared read binding. It includes the entire
    /// immutable registry and authorization limits, not a caller-selected unit.
    pub fn commitment(&self, config: &units::Configuration) -> Result<[u8; 32], Error> {
        history::fingerprint(config, &self.auth)
    }
    /// Trusted startup verification before attaching a publication/listener.
    /// This loads no key, creates no account and grants no native permission.
    pub fn initialize<B: Backend, P: Protection>(
        &self,
        journal: &mut units::Journal<B, P>,
    ) -> Result<(), Error> {
        journal.verified_state().map_err(|_| Error::Unavailable)?;
        self.history(
            journal.state().map_err(|_| Error::Unavailable)?,
            journal.head(),
            journal.transactions().map_err(|_| Error::Unavailable)?,
        )?;
        Ok(())
    }
    fn history<'a>(
        &self,
        state: &units::State,
        head: Head,
        transactions: impl Iterator<Item = (&'a units::Transaction, &'a units::Receipt)>,
    ) -> Result<Arc<history::History>, Error> {
        history::shared(
            &self.cache,
            head,
            self.commitment(state.configuration())?,
            || history::History::interpret(state, head, &self.auth, transactions),
        )
    }
    /// Reject malformed signatures/sessions/read methods before witness I/O.
    /// Current authority and final release are checked separately.
    pub fn authenticate_read(
        &self,
        config: &units::Configuration,
        channel: &impl ConfidentialChannel,
        bytes: &PrivateBytes,
        now: u64,
    ) -> Result<(), Error> {
        let r = self.auth.authenticate(config, channel, bytes, now)?;
        if !crate::publication::is_read(&r.body.command) {
            return Err(Error::Invalid);
        }
        Ok(())
    }
    fn read_context(
        &self,
        view: &Verified,
        channel: &impl ConfidentialChannel,
        bytes: &PrivateBytes,
        now: u64,
    ) -> Result<(Checked, Arc<history::History>), Error> {
        let state = view.unit_state().map_err(|_| Error::Unavailable)?;
        if view.binding().api != self.commitment(state.configuration())? {
            return Err(Error::Unavailable);
        }
        let request = self
            .auth
            .authenticate(state.configuration(), channel, bytes, now)?;
        if !crate::publication::is_read(&request.body.command) {
            return Err(Error::Invalid);
        }
        let history = self.history(
            state,
            view.head(),
            view.unit_transactions().map_err(|_| Error::Unavailable)?,
        )?;
        let checked = self
            .auth
            .check_at(state, view.head(), request, &*history, now, true)?;
        Ok((checked, history))
    }
    /// Pure private projection from this request's independently witnessed cut.
    /// The caller MUST consume Verified's final generation/time release gate.
    pub fn read(
        &self,
        view: &Verified,
        channel: &impl ConfidentialChannel,
        bytes: &PrivateBytes,
        now: u64,
    ) -> Result<Response, Error> {
        let (checked, history) = self.read_context(view, channel, bytes, now)?;
        let req = checked.request();
        let state = view.unit_state().map_err(|_| Error::Unavailable)?;
        let records = history
            .rows
            .iter()
            .filter(|r| r.request.unit == req.unit)
            .map(|r| {
                let decision = state
                    .api_decisions()
                    .iter()
                    .find(|d| d.request == history::key(&r.request));
                crate::Record {
                    request: r.request.body.clone(),
                    at: r.at,
                    unit: Some(req.unit),
                    recorded_only: r.effect == units::api::Effect::Recorded && decision.is_none(),
                    accepted: if r.effect == units::api::Effect::Recorded {
                        decision.is_some_and(|d| d.receipt.controls.is_none())
                    } else {
                        true
                    },
                }
            })
            .collect::<Vec<_>>();
        self.projection(state, req.unit, req.body.account)?
            .project_scoped(
                &view.unit(req.unit).map_err(|_| Error::Unavailable)?,
                &req.body,
                &records,
                checked.principal() == Principal::Owner,
                now,
                Some(req.unit),
            )
    }
    /// Recheck current clock/session/auth before the consuming release gate.
    pub fn revalidate_read(
        &self,
        view: &Verified,
        channel: &impl ConfidentialChannel,
        bytes: &PrivateBytes,
        now: u64,
    ) -> Result<(), Error> {
        self.read_context(view, channel, bytes, now).map(|_| ())
    }
    /// Authenticate, deduplicate and commit one administration/financial decision.
    /// The caller serializes the shared writer; there is no hidden native I/O.
    pub fn mutate<B: Backend, P: Protection>(
        &self,
        journal: &mut units::Journal<B, P>,
        channel: &impl ConfidentialChannel,
        bytes: &PrivateBytes,
        now: u64,
    ) -> Result<Response, Error> {
        let request = self
            .auth
            .authenticate(journal.configuration(), channel, bytes, now)?;
        // Remote writer freshness cannot serialize the independent read cache.
        journal.verified_state().map_err(|_| Error::Unavailable)?;
        let history = self.history(
            journal.state().map_err(|_| Error::Unavailable)?,
            journal.head(),
            journal.transactions().map_err(|_| Error::Unavailable)?,
        )?;
        let command = request.body.command;
        let (original, accepted, at) = if matches!(command, Command::Grant(_) | Command::Revoke) {
            let c = self
                .auth
                .capture_at(journal, channel, bytes, now, &history)?;
            (c.request, true, c.at)
        } else {
            let d = self
                .auth
                .admit_at(journal, channel, bytes, &self.admission, now, &history)?;
            (d.request, d.refusal.is_none(), d.at)
        };
        let shared = journal.state().map_err(|_| Error::Unavailable)?;
        let state = shared.unit(original.unit).ok_or(Error::Unavailable)?;
        self.projection(shared, original.unit, original.body.account)?
            .receipt(
                state,
                &crate::Record {
                    unit: Some(original.unit),
                    request: original.body,
                    accepted,
                    at,
                    recorded_only: false,
                },
            )
            .map(Response::Receipt)
    }
}
struct ProjectionAdmission<'a, A>(&'a A);
impl<A: crate::Admission> crate::Admission for ProjectionAdmission<'_, A> {
    fn order_holds(
        &self,
        state: &cinder_journal::model::State,
        intent: &cinder_journal::orders::Intent,
    ) -> Result<Vec<cinder_journal::model::Reservation>, Error> {
        self.0.order_holds(state, intent)
    }
}
