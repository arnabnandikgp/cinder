//! One bounded native supervisor over the same unit Runtime. No arbitrary
//! command queue, balances, readiness setters, resend or second financial store.
use super::*;
use cinder_bulk::{execution as bulk, reads as br};
use cinder_journal::{controller::Controller, model::*};
use cinder_kernel::identity::{AttemptId, AttemptKey, UnitId};
use cinder_pacifica::{execution as pacifica, reads as pr};
use serde::{Deserialize, Serialize};

/// Explicit measured resource allocation, not a native rate/financial guarantee.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// Minimum interval between observation requests, 1–60 seconds.
    pub poll_ms: u64,
    /// Per-boot Pacifica query ceiling, at most 32.
    pub pacifica_reads: u16,
    /// Per-boot BULK query ceiling, at most 32.
    pub bulk_reads: u16,
    /// History-wide escaped Pacifica order/cancel ceiling, at most 16.
    pub pacifica_dispatches: u16,
    /// History-wide escaped BULK order/cancel ceiling, at most 16.
    pub bulk_dispatches: u16,
}
impl Policy {
    /// Validate finite ceilings, never approve a live run or infer qualification.
    pub fn validate(&self) -> Result<(), Error> {
        if !(1000..=60000).contains(&self.poll_ms)
            || self.pacifica_reads > 32
            || self.bulk_reads > 32
            || self.pacifica_dispatches > 16
            || self.bulk_dispatches > 16
        {
            return Err(Error);
        }
        Ok(())
    }
    /// Whether the measured deployment requests ordinary execution capability.
    pub fn trading(&self) -> bool {
        self.pacifica_dispatches != 0 || self.bulk_dispatches != 0
    }
    /// Whether any native query may be prepared by this deployment.
    pub fn reads(&self) -> bool {
        self.pacifica_reads != 0 || self.bulk_reads != 0
    }
}
/// Coarse trusted worker result; it contains neither native bytes nor balances.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    /// Writer busy, no side effect and no lost completion.
    Busy,
    /// No currently admissible work/budget. Not terminal order confirmation.
    Idle,
    /// One native query completed/retained, including unknown/error outcomes.
    Read,
    /// One signed native request escaped once; ACK is still not an execution.
    Dispatched,
}
/// Actual gateways and private ports. The caller serializes this one supervisor,
/// not the financial writer; no field/command lets a client replace a gateway.
pub struct Worker<P, T> {
    policy: Policy,
    pacifica: pacifica::Gateway,
    bulk: bulk::Gateway,
    pacifica_port: P,
    bulk_port: T,
    units: [UnitId; 2],
    turn: usize,
    read_kind: [usize; 2],
    reads: [u16; 2],
    next_poll: u64,
    prefer_read: bool,
    dispatch_cursor: [usize; 2],
    paper_cleanup: bool,
}
fn id() -> Result<CommitId, Error> {
    let mut bytes = [0; 32];
    openssl::rand::rand_bytes(&mut bytes)?;
    CommitId::new(bytes).map_err(|_| Error)
}
enum Work {
    PacificaRead(pr::Request, pr::Completion),
    BulkRead(br::Request, br::Completion),
    Pacifica(pacifica::Outbound, pacifica::DispatchCompletion),
    Bulk(bulk::Outbound, bulk::Completion),
}
impl<P: pr::Transport + pacifica::Transport, T: br::Transport + bulk::Transport> Worker<P, T> {
    /// Validated loaded controllers, exact unit routes and allocated ceilings.
    /// No I/O, money/readiness initialization or signer registration occurs here.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        policy: Policy,
        pacifica_unit: UnitId,
        bulk_unit: UnitId,
        pacifica: pacifica::Gateway,
        bulk: bulk::Gateway,
        pacifica_port: P,
        bulk_port: T,
    ) -> Result<Self, Error> {
        policy.validate()?;
        if pacifica_unit == bulk_unit {
            return Err(Error);
        }
        Ok(Self {
            policy,
            pacifica,
            bulk,
            pacifica_port,
            bulk_port,
            units: [pacifica_unit, bulk_unit],
            turn: 0,
            read_kind: [0; 2],
            reads: [0; 2],
            next_poll: 0,
            prefer_read: false,
            dispatch_cursor: [0; 2],
            paper_cleanup: false,
        })
    }
    /// Version-9 only: preserve the last native dispatch slot for an attributed
    /// reduce-only IOC, not an opening or a cancellation that refunds nothing.
    pub(super) fn reserve_paper_cleanup(mut self) -> Self {
        self.paper_cleanup = true;
        self
    }
    /// Install only local key/credit epochs, never native registration or funds.
    pub fn initialize<B: Backend, Q: Protection, A: cinder_api::Admission>(
        &self,
        app: &Runtime<B, Q, A>,
    ) -> Result<(), Error> {
        let mut writer = app.completion_writer()?;
        let at = app.now()?;
        {
            let mut selected = writer.select(self.units[0]).map_err(|_| Error)?;
            self.pacifica
                .unit_binding(selected.descriptor().ok_or(Error)?)
                .map_err(|_| Error)?;
            self.pacifica
                .initialize_reads(&mut selected, id()?, at)
                .map_err(|_| Error)?;
        }
        self.bulk
            .initialize(
                &mut writer.select(self.units[1]).map_err(|_| Error)?,
                id()?,
                at,
            )
            .map_err(|_| Error)
    }
    /// At most one escaped request per cut, and none while the writer is busy.
    /// Any integrity/storage/time error fences both units. Native errors/unknown
    /// replies are retained normally; no original request is retried.
    pub fn tick<B: Backend, Q: Protection, A: cinder_api::Admission>(
        &mut self,
        app: &Runtime<B, Q, A>,
    ) -> Result<Step, Error> {
        let result = self.step(app);
        if result.is_err() {
            app.fence();
        }
        result
    }
    fn step<B: Backend, Q: Protection, A: cinder_api::Admission>(
        &mut self,
        app: &Runtime<B, Q, A>,
    ) -> Result<Step, Error> {
        app.now()?;
        // No remaining native capability: do not poll cloud history merely to
        // rediscover idleness. This is not private-read authority; each SDK read
        // still verifies its independent witness, and the boot lease stays live.
        if !self.policy.trading()
            && self.reads[0] >= self.policy.pacifica_reads
            && self.reads[1] >= self.policy.bulk_reads
        {
            return Ok(Step::Idle);
        }
        let work = {
            let mut writer = match app.journal.try_lock() {
                Ok(w) => w,
                Err(std::sync::TryLockError::WouldBlock) => return Ok(Step::Busy),
                Err(_) => return Err(Error),
            };
            let now = app.now()?;
            writer.verified_state().map_err(|_| Error)?;
            let mut work = None;
            for reads_first in [self.prefer_read, !self.prefer_read] {
                for offset in 0..2 {
                    let index = (self.turn + offset) % 2;
                    let mut selected = writer.select(self.units[index]).map_err(|_| Error)?;
                    work = if reads_first {
                        self.read(&mut selected, index, now)?
                    } else {
                        self.dispatch(&mut selected, index, now)?
                    };
                    if work.is_some() {
                        self.turn = (index + 1) % 2;
                        self.prefer_read = !reads_first;
                        break;
                    }
                }
                if work.is_some() {
                    break;
                }
            }
            work
        }; // release financial mutation ownership BEFORE TLS/native port calls.
        let Some(work) = work else {
            return Ok(Step::Idle);
        };
        match work {
            Work::PacificaRead(request, completion) => {
                let reply = self.pacifica_port.get(request);
                let mut writer = app.completion_writer()?;
                app.now()?;
                pr::complete(
                    &mut writer.select(self.units[0]).map_err(|_| Error)?,
                    &self.pacifica,
                    completion,
                    reply,
                )
                .map_err(|_| Error)?;
                Ok(Step::Read)
            }
            Work::BulkRead(request, completion) => {
                let reply = self.bulk_port.query(request);
                let mut writer = app.completion_writer()?;
                let at = app.now()?;
                br::complete(
                    &self.bulk,
                    &mut writer.select(self.units[1]).map_err(|_| Error)?,
                    completion,
                    reply,
                    at,
                )
                .map_err(|_| Error)?;
                Ok(Step::Read)
            }
            Work::Pacifica(request, completion) => {
                let reply = self.pacifica_port.post(request);
                app.complete_pacifica(self.units[0], &self.pacifica, completion, reply)?;
                Ok(Step::Dispatched)
            }
            Work::Bulk(request, completion) => {
                let reply = self.bulk_port.post(request);
                app.complete_bulk(self.units[1], &self.bulk, completion, reply)?;
                Ok(Step::Dispatched)
            }
        }
    }
    fn read(
        &mut self,
        j: &mut impl Controller,
        index: usize,
        now: u64,
    ) -> Result<Option<Work>, Error> {
        let ceiling = [self.policy.pacifica_reads, self.policy.bulk_reads][index];
        if self.reads[index] >= ceiling || now < self.next_poll {
            return Ok(None);
        }
        let next = if index == 0 {
            if !self
                .pacifica
                .read_available(j, now, false)
                .map_err(|_| Error)?
            {
                return Ok(None);
            }
            let kinds = [
                cinder_pacifica::observation::Kind::Account,
                cinder_pacifica::observation::Kind::Positions,
                cinder_pacifica::observation::Kind::Trades,
                cinder_pacifica::observation::Kind::Orders,
                cinder_pacifica::observation::Kind::Funding,
            ];
            let kind = kinds[self.read_kind[index] % kinds.len()];
            // Retained cursor, never caller data. Not a terminal/global frontier.
            let cursor = self.pacifica.read_cursor(j, kind).map_err(|_| Error)?;
            let (r, c) = pr::prepare_poll(
                j,
                &self.pacifica,
                pr::Poll {
                    kind,
                    cursor,
                    reservation: id()?,
                    evidence: id()?,
                    at: now,
                    cleanup: false,
                },
            )
            .map_err(|_| Error)?;
            Work::PacificaRead(r, c)
        } else {
            if !self.bulk.read_available(j, now, false).map_err(|_| Error)? {
                return Ok(None);
            }
            // Bounded first-page refresh only. Do not advertise complete history
            // or infer a causal cut when native pagination/retention is unknown.
            let kinds = [
                cinder_bulk::observation::Kind::Account,
                cinder_bulk::observation::Kind::Trades,
            ];
            let kind = kinds[self.read_kind[index] % kinds.len()];
            let (r, c) = br::prepare(
                &self.bulk,
                j,
                br::Poll {
                    kind,
                    reservation: id()?,
                    evidence: id()?,
                    range: None,
                    continuation: None,
                    limit: 128,
                    at: now,
                    cleanup: false,
                },
            )
            .map_err(|_| Error)?;
            Work::BulkRead(r, c)
        };
        self.reads[index] += 1;
        self.read_kind[index] += 1;
        self.next_poll = now.checked_add(self.policy.poll_ms).ok_or(Error)?;
        Ok(Some(next))
    }
    fn dispatch(
        &mut self,
        j: &mut impl Controller,
        index: usize,
        now: u64,
    ) -> Result<Option<Work>, Error> {
        let ceiling = [self.policy.pacifica_dispatches, self.policy.bulk_dispatches][index];
        let state = j.state().map_err(|_| Error)?;
        let count = state
            .attempts()
            .iter()
            .filter(|a| {
                a.possibly_exposed && matches!(a.kind, AttemptKind::Order | AttemptKind::Cancel)
            })
            .count();
        if count >= usize::from(ceiling) {
            return Ok(None);
        }
        let cleanup_only = self.paper_cleanup && index == 1 && count + 1 == usize::from(ceiling);
        // Stable bounded round-robin over eligible attempts AND unprepared
        // intents. One blocked original cannot starve another accepted order.
        // Escaped, stale and expired originals never enter this queue.
        let mut candidates: Vec<_> = state
            .attempts()
            .iter()
            .filter(|a| {
                !a.possibly_exposed
                    && a.expires_at > now
                    && state.authority_epoch(a.key.request.account) == Some(a.authority_epoch)
                    && matches!(a.kind, AttemptKind::Order | AttemptKind::Cancel)
            })
            .map(|a| {
                (
                    u8::from(a.kind != AttemptKind::Cancel),
                    Some(a.key),
                    a.key.request,
                )
            })
            .collect();
        candidates.extend(
            state
                .orders()
                .iter()
                .filter(|o| {
                    o.attempt.is_none()
                        && !o.abandoned
                        && !o.faulted
                        && o.terminal.is_none()
                        && o.intent.expires_at > now
                        && state.authority_epoch(o.intent.request.account)
                            == Some(o.intent.authority_epoch)
                })
                .map(|o| (2, None, o.intent.request)),
        );
        if cleanup_only {
            candidates.retain(|(_, attempt, request)| {
                attempt.is_none_or(|a| {
                    state
                        .attempts()
                        .iter()
                        .any(|old| old.key == a && old.kind == AttemptKind::Order)
                }) && state.orders().iter().any(|o| {
                    o.intent.request == *request
                        && o.intent.reduce_only
                        && o.intent.time_in_force
                            == cinder_journal::orders::TimeInForce::ImmediateOrCancel
                })
            });
        }
        candidates.sort_by_key(|c| c.0);
        if candidates.is_empty() {
            return Ok(None);
        }
        let cursor = self.dispatch_cursor[index] % candidates.len();
        self.dispatch_cursor[index] = (cursor + 1) % candidates.len();
        let (_, candidate, request) = candidates[cursor];
        let attempt = if let Some(key) = candidate {
            key
        } else {
            let attempt = AttemptKey {
                request,
                attempt: AttemptId::new(id()?.bytes()).map_err(|_| Error)?,
            };
            let tx = Transaction {
                id: id()?,
                expected: j.head(),
                at: now,
                evidence: vec![],
                inputs: vec![],
                order_observations: vec![],
                funds_observations: vec![],
                controls: vec![Control::Order(cinder_journal::orders::Action::Prepare {
                    attempt,
                })],
            };
            if j.preview(&tx).map_err(|_| Error)?.controls.is_some() {
                return Ok(None);
            }
            if j.commit(tx).map_err(|_| Error)?.receipt.controls.is_some() {
                return Err(Error);
            }
            attempt
        };
        if index == 0 {
            let dispatch = pacifica::Dispatch {
                attempt,
                commit: id()?,
                at: now,
            };
            if !self
                .pacifica
                .dispatch_available(j, dispatch)
                .map_err(|_| Error)?
            {
                return Ok(None);
            }
            let (r, c) = self
                .pacifica
                .prepare_dispatch(j, dispatch)
                .map_err(|_| Error)?;
            Ok(Some(Work::Pacifica(r, c)))
        } else {
            // Explicit scheduler nonce from qualified milliseconds, with durable
            // floor across restart/key epoch; raw adapter never invents one.
            let nonce = now.checked_mul(1_000_000).ok_or(Error)?.max(
                self.bulk
                    .last_nonce(j, now)
                    .map_err(|_| Error)?
                    .checked_add(1)
                    .ok_or(Error)?,
            );
            let dispatch = bulk::Dispatch {
                attempt,
                commit: id()?,
                nonce,
                at: now,
            };
            if !self
                .bulk
                .dispatch_available(j, dispatch)
                .map_err(|_| Error)?
            {
                return Ok(None);
            }
            let (r, c) = self.bulk.prepare_dispatch(j, dispatch).map_err(|_| Error)?;
            Ok(Some(Work::Bulk(r, c)))
        }
    }
}
