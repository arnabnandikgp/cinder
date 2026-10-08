//! One mutation owner with bounded, one-shot I/O outside its guard. Loaded
//! financial gates and native completion contracts remain independent.
use super::*;
use cinder_pacifica::execution::{Dispatch, Transport as NativeTransport};

impl<B: Backend + Send, P: Protection + Send> Runtime<B, P> {
    pub(super) fn tick_funds<T: crate::chain_rpc::Transport, U: NativeTransport>(
        &self,
        chain: Option<&mut chain_funding::Port<T>>,
        egress: &mut U,
    ) -> Result<(), Error> {
        let next = {
            let mut active = self.active.lock().map_err(|_| Error)?;
            let now = self.active_time()?;
            let Active {
                store,
                funding,
                gateway,
                demo_deposit,
                demo_allocation,
                ..
            } = &mut *active;
            funding
                .release_commitment(store.configuration())
                .map_err(|_| Error)?;
            if store.verified_state().map_err(|_| Error)?.logical_time() > now {
                return Err(Error);
            }
            // Shipping funding is only the measured private allocation. Never
            // let a separately accepted API intent borrow this activation.
            if self.gates.funding && demo_allocation.is_none() {
                return Err(Error);
            }
            if let Some(authorization) = demo_allocation.as_ref()
                && !store
                    .state()
                    .map_err(|_| Error)?
                    .funds()
                    .iter()
                    .any(|o| !o.terminal && o.attempt.is_some())
            {
                authorization.accept_next(
                    store,
                    funding,
                    gateway,
                    demo_deposit.as_ref().ok_or(Error)?,
                    now,
                )?;
            }
            let demo = demo_deposit
                .as_ref()
                .filter(|p| p.initial_setup.is_some())
                .map(|p| (&*gateway, p));
            prepare_next_ingress(store, funding, demo, now)?;
            store
                .state()
                .map_err(|_| Error)?
                .funds()
                .iter()
                .find(|o| !o.terminal && o.attempt.is_some())
                .and_then(|o| {
                    o.attempt.map(|attempt| {
                        (
                            attempt,
                            o.intent.source == cinder_kernel::ledger::Location::Venue,
                        )
                    })
                })
        };
        if let Some((attempt, native)) = next {
            if self.gates.funding {
                let active = self.active.lock().map_err(|_| Error)?;
                if !active
                    .demo_allocation
                    .as_ref()
                    .ok_or(Error)?
                    .permits(&active.store, attempt)?
                    || native
                {
                    return Err(Error);
                }
            }
            if native {
                self.tick_withdrawal(egress, attempt)?;
            } else {
                self.tick_chain(chain.ok_or(Error)?, attempt)?;
            }
        }
        Ok(())
    }

    pub(super) fn tick_chain<T: crate::chain_rpc::Transport>(
        &self,
        chain: &mut chain_funding::Port<T>,
        attempt: AttemptKey,
    ) -> Result<(), Error> {
        let (issue, reconcile) = {
            let mut active = self.active.lock().map_err(|_| Error)?;
            let now = self.active_time()?;
            let Active {
                store,
                funding,
                gateway,
                demo_deposit,
                ..
            } = &mut *active;
            let exposed = store
                .verified_state()
                .map_err(|_| Error)?
                .attempts()
                .iter()
                .find(|a| a.key == attempt)
                .ok_or(Error)?
                .possibly_exposed;
            if exposed {
                (None, chain.prepare_reconcile(store, funding, attempt, now)?)
            } else {
                let demo = demo_deposit
                    .as_ref()
                    .filter(|p| p.initial_setup.is_some())
                    .map(|p| (&*gateway, p));
                (
                    Some(chain.prepare_issue(store, funding, attempt, demo, now)?),
                    None,
                )
            }
        };
        if let Some(request) = reconcile {
            let read = chain.collect_reconcile(request)?;
            let mut active = self.active.lock().map_err(|_| Error)?;
            let now = self.active_time()?;
            let Active { store, funding, .. } = &mut *active;
            read.complete(store, funding, now)?;
        } else if let Some(request) = issue {
            let read = chain.collect_issue(request)?;
            let work = {
                let mut active = self.active.lock().map_err(|_| Error)?;
                let now = self.active_time()?;
                let Active {
                    store,
                    funding,
                    gateway,
                    demo_deposit,
                    ..
                } = &mut *active;
                let demo = demo_deposit
                    .as_ref()
                    .filter(|p| p.initial_setup.is_some())
                    .map(|p| (&*gateway, p));
                read.expose(store, funding, demo, now)?
            };
            let slot = work.slot();
            let simulation = chain.simulate_issue(&work)?;
            let delivery = {
                let mut active = self.active.lock().map_err(|_| Error)?;
                let now = self.active_time()?;
                let Active { store, funding, .. } = &mut *active;
                chain.persist_issue(store, funding, work, simulation, now)?
            };
            // This durable, one-use capability is the release linearization
            // point. Subsequent revocation cannot erase a possible effect.
            chain.submit_issue(delivery, slot)?;
        }
        Ok(())
    }

    fn tick_withdrawal<T: NativeTransport>(
        &self,
        egress: &mut T,
        attempt: AttemptKey,
    ) -> Result<(), Error> {
        let prepared = {
            let mut active = self.active.lock().map_err(|_| Error)?;
            let now = self.active_time()?;
            let Active {
                store,
                gateway,
                funding,
                ..
            } = &mut *active;
            if store
                .verified_state()
                .map_err(|_| Error)?
                .attempts()
                .iter()
                .find(|a| a.key == attempt)
                .ok_or(Error)?
                .possibly_exposed
            {
                None
            } else {
                Some(
                    funding
                        .prepare_withdrawal(
                            store,
                            gateway,
                            Dispatch {
                                attempt,
                                commit: commit_id()?,
                                at: now,
                            },
                        )
                        .map_err(|_| Error)?,
                )
            }
        };
        if let Some((request, completion)) = prepared {
            let reply = egress.post(request);
            let mut active = self.active.lock().map_err(|_| Error)?;
            let now = self.active_time()?;
            let Active {
                store,
                gateway,
                funding,
                ..
            } = &mut *active;
            funding
                .complete_withdrawal(store, gateway, completion, reply, now)
                .map_err(|_| Error)?;
        }
        // Retention alone grants neither source finality nor payout authority.
        Ok(())
    }

    pub(super) fn tick_orders<T: NativeTransport>(&self, egress: &mut T) -> Result<(), Error> {
        let prepared = {
            let mut active = self.active.lock().map_err(|_| Error)?;
            let now = self.active_time()?;
            let Active { store, gateway, .. } = &mut *active;
            let pending = store
                .verified_state()
                .map_err(|_| Error)?
                .attempts()
                .iter()
                .find(|a| {
                    !a.possibly_exposed
                        && matches!(a.kind, AttemptKind::Order | AttemptKind::Cancel)
                })
                .map(|a| a.key);
            pending
                .map(|attempt| {
                    gateway
                        .prepare_dispatch(
                            store,
                            Dispatch {
                                attempt,
                                commit: commit_id()?,
                                at: now,
                            },
                        )
                        .map_err(|_| Error)
                })
                .transpose()?
        };
        if let Some((request, completion)) = prepared {
            let reply = egress.post(request);
            let mut active = self.active.lock().map_err(|_| Error)?;
            let now = self.active_time()?;
            let Active { store, gateway, .. } = &mut *active;
            gateway
                .complete_dispatch(store, completion, reply, now)
                .map_err(|_| Error)?;
        }
        Ok(())
    }
}
