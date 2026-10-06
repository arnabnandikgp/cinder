//! One private financial service, one authoritative journal. No seeded balance,
//! fixture dispatch, HTTP parent backend, hidden wallet loading or replay resend.
use crate::{
    Error,
    boot::{Gates, Manifest, Role},
    chain_funding,
    cloud::{Client, Credential, DynamoWitness, S3Replica},
    egress::{Egress, Trust},
    release::ApplicationContract,
    transport::{Clock, Handler, Session},
    vsock::Target,
};
use cinder_api::{Admission, Contract, OwnerBinding, Service};
use cinder_journal::{
    Backend, Journal, Protection,
    encrypted::RecordCipher,
    model::*,
    orders,
    replicated::{Replicated, Stream, Witness},
};
use cinder_kernel::{identity::*, ledger::Config};
use cinder_pacifica::{
    execution::{Gateway, Policy},
    funding::{Controller, Route},
    profile::{Level, Mapping, Profile},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use zeroize::Zeroizing;

const REQUEST_WAIT: std::time::Duration = std::time::Duration::from_millis(250);
fn lock_bounded<T>(
    mutex: &Mutex<T>,
    timeout: std::time::Duration,
) -> Result<Option<std::sync::MutexGuard<'_, T>>, Error> {
    let start = std::time::Instant::now();
    loop {
        match mutex.try_lock() {
            Ok(guard) => return Ok(Some(guard)),
            Err(std::sync::TryLockError::Poisoned(_)) => return Err(Error),
            Err(std::sync::TryLockError::WouldBlock) => {
                let Some(remaining) = timeout.checked_sub(start.elapsed()) else {
                    return Ok(None);
                };
                if remaining.is_zero() {
                    return Ok(None);
                }
                std::thread::sleep(remaining.min(std::time::Duration::from_millis(1)));
                if start.elapsed() >= timeout {
                    return Ok(None);
                }
            }
        }
    }
}

/// A rejected original deposit must not starve subsequent configured locators.
/// Transport, freshness, current-account and journal failures still fence the worker.
pub(crate) fn poll_deposit<B: Backend, P: Protection, T: crate::chain_rpc::Transport>(
    chain: &mut chain_funding::Port<T>,
    store: &mut Journal<B, P>,
    funding: &Controller,
    index: &mut usize,
) -> Result<(), Error> {
    let count = chain.deposit_count();
    if count != 0 {
        match chain.observe_deposit(store, funding, *index)? {
            chain_funding::Outcome::Pending
            | chain_funding::Outcome::Rejected
            | chain_funding::Outcome::Settled => *index = (*index + 1) % count,
            _ => return Err(Error),
        }
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Confidential account-to-payout/auth owner binding, not a public directory.
pub struct Owner {
    /// Private accounting identity.
    pub account: [u8; 32],
    /// Registered Solana signing/payout wallet.
    pub wallet: [u8; 32],
    /// Bound token recipient, not caller-selected per withdrawal.
    pub tokens: [u8; 32],
}
/// Encrypted configuration payload. Not customer onboarding or a policy API.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    /// Existing canonical journal configuration codec, no alternate ledger.
    pub ledger: Vec<u8>,
    /// Governed private customer bindings.
    pub owners: Vec<Owner>,
    /// Maximum private authentication lifetime, milliseconds.
    pub auth_ms: u64,
    /// Maximum persisted strategy grant lifetime, milliseconds.
    pub grant_ms: u64,
    /// Native source index in the decoded configuration.
    pub source: usize,
    /// Exact native pooled wallet address.
    pub account: String,
    /// Native profile environment identifier.
    pub environment: String,
    /// Nonzero immutable qualification revision.
    pub revision: u64,
    /// Named qualification artifact, not live authority by itself.
    pub evidence: String,
    /// Exact native decimal-grid qualification.
    pub precision: Level,
    /// Exact native fill-semantic qualification.
    pub fills: Level,
    /// Venue symbols mapped to configured kernel markets.
    pub markets: Vec<Mapping>,
    /// Native quote atom decimal scale.
    pub quote_places: u8,
    /// Qualified native perp instrument tag.
    pub perp_tag: u64,
    /// Loaded execution/credit contract; construction does not activate it.
    pub execution: Policy,
    /// Loaded funds route; chain signing requires version 2's separate role.
    pub route: Route,
    /// Trading-agent epoch, distinct from witness/storage generation.
    pub trading_epoch: u64,
    /// Version 2 private chain limits/path. Omitted in old five-role profiles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain: Option<chain_funding::Configuration>,
}
impl Configuration {
    /// Consume separated secrets into actual validated controllers and API policy.
    pub fn construct(self, mut keys: BTreeMap<Role, Zeroizing<Vec<u8>>>) -> Result<Loaded, Error> {
        self.construct_using(&mut keys, None)
    }
    /// Load versioned public trust from the ACTUAL measured manifest. Secret
    /// path/limits and separate funds identity contribute to the application hash.
    pub fn construct_for(
        self,
        manifest: &Manifest,
        mut keys: BTreeMap<Role, Zeroizing<Vec<u8>>>,
    ) -> Result<Loaded, Error> {
        manifest.validate()?;
        self.construct_using(&mut keys, manifest.chain.as_ref())
    }
    fn construct_using(
        self,
        keys: &mut BTreeMap<Role, Zeroizing<Vec<u8>>>,
        peer: Option<&chain_funding::Peer>,
    ) -> Result<Loaded, Error> {
        crate::boot::validate_keys(keys)?;
        let config = cinder_journal::wire::decode_config(&self.ledger).map_err(|_| Error)?;
        let source = config.sources.get(self.source).ok_or(Error)?.scope;
        let profile = Profile {
            config: config.clone(),
            source,
            account: self.account,
            environment: self.environment,
            revision: self.revision,
            evidence: self.evidence,
            precision: self.precision,
            fills: self.fills,
            markets: self.markets,
            quote_places: self.quote_places,
            perp_tag: self.perp_tag,
        };
        let take = |keys: &mut BTreeMap<Role, Zeroizing<Vec<u8>>>,
                    role|
         -> Result<Zeroizing<[u8; 32]>, Error> {
            let bytes = keys.remove(&role).ok_or(Error)?;
            Ok(Zeroizing::new(
                bytes.as_slice().try_into().map_err(|_| Error)?,
            ))
        };
        let storage = take(keys, Role::Storage)?;
        let gateway = Gateway::new(
            profile.clone(),
            self.execution.clone(),
            take(keys, Role::Trading)?,
            self.trading_epoch,
        )
        .map_err(|_| Error)?;
        let funding =
            Controller::new(profile, self.route, take(keys, Role::Broker)?).map_err(|_| Error)?;
        let owners = self
            .owners
            .into_iter()
            .map(|o| {
                Ok(OwnerBinding {
                    account: AccountId::new(o.account).map_err(|_| Error)?,
                    wallet: o.wallet,
                    tokens: o.tokens,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let api = Service::new(
            Contract {
                owners,
                maximum_auth_lifetime: self.auth_ms,
                maximum_grant_lifetime: self.grant_ms,
            },
            RiskAdmission { enabled: false },
        )
        .map_err(|_| Error)?;
        let chain = match (self.chain, peer) {
            (None, None) => None,
            (Some(policy), Some(peer)) => {
                if self.execution.read_cost < cinder_pacifica::reads::MIN_READ_COST {
                    return Err(Error);
                }
                Some(chain_funding::Loaded::new(
                    policy,
                    peer,
                    take(keys, Role::Funds)?,
                    &funding,
                    config.domain.network.bytes(),
                )?)
            }
            _ => return Err(Error),
        };
        let application = ApplicationContract::derive(&config, &api, &gateway, &funding)?;
        let application = if let Some(chain) = &chain {
            application.with_chain(chain)
        } else {
            application
        };
        let bytes = keys.remove(&Role::Witness).ok_or(Error)?;
        let witness = serde_cbor::from_slice(&bytes).map_err(|_| Error)?;
        if !keys.is_empty() {
            return Err(Error);
        }
        Ok(Loaded {
            config,
            api,
            gateway,
            funding,
            storage,
            witness,
            application,
            origin: self.execution.origin,
            chain,
        })
    }
}
struct RiskAdmission {
    enabled: bool,
}
impl Admission for RiskAdmission {
    fn order_holds(
        &self,
        state: &State,
        intent: &orders::Intent,
    ) -> Result<Vec<Reservation>, cinder_api::Error> {
        if !self.enabled {
            return Err(cinder_api::Error::Unavailable);
        }
        state
            .order_reservations(intent)
            .map_err(|_| cinder_api::Error::Unavailable)
    }
}
/// Validated actual loaded application, with redacted non-exportable signers.
pub struct Loaded {
    config: Config,
    api: Service<RiskAdmission>,
    gateway: Gateway,
    funding: Controller,
    storage: Zeroizing<[u8; 32]>,
    witness: Credential,
    application: ApplicationContract,
    origin: cinder_pacifica::execution::Origin,
    chain: Option<chain_funding::Loaded>,
}
/// Public health is deliberately coarse; no balances, customer counts or keys.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Health {
    /// Private protocol/storage ready, not a promise native trading is enabled.
    Ready,
    /// Stop is sticky; this boot cannot accept work or reuse capabilities.
    Fenced,
}
struct Active<B: Backend, P: Protection> {
    store: Journal<B, P>,
    gateway: Gateway,
    funding: Controller,
    egress: Egress,
    chain: Option<chain_funding::Port<crate::chain_rpc::Https>>,
    poll_ms: Option<u64>,
    next_poll: u64,
    read_kind: usize,
    deposit_index: usize,
}
/// Bounded private API/scheduler composition over one journal and one writer.
pub struct Runtime<B: Backend + Send, P: Protection + Send> {
    active: Mutex<Active<B, P>>,
    api: Service<RiskAdmission>,
    clock: Arc<dyn Clock>,
    stop: Arc<AtomicBool>,
    deadline: u64,
    gates: Gates,
}
impl Loaded {
    /// Actual application component for trusted provisioning before KMS Encrypt.
    pub fn commitment(&self) -> [u8; 32] {
        self.application.digest()
    }
    /// Freshness is read from the remote pre-provisioned register, never inferred
    /// from absent host files. All restore/config checks precede the listener.
    pub fn open(
        self,
        manifest: &Manifest,
        parent: Credential,
        clock: Arc<dyn Clock>,
        stop: Arc<AtomicBool>,
    ) -> Result<Runtime<Replicated<S3Replica, S3Replica, DynamoWitness>, RecordCipher>, Error> {
        manifest.validate()?;
        if self.application.digest() != manifest.application
            || [
                self.config.domain.network.bytes().as_slice(),
                &self.config.domain.deployment.bytes(),
            ]
            .concat()
                != manifest.domain
        {
            return Err(Error);
        }
        let now = clock.now()?;
        let stream = Stream {
            domain: self.config.domain,
            id: manifest.stream,
        };
        let copy = || Credential {
            access: parent.access.clone(),
            secret: parent.secret.clone(),
            token: parent.token.clone(),
            expires: parent.expires,
        };
        // The parent credential cannot be reused as the witness credential.
        if parent.access == self.witness.access {
            return Err(Error);
        }
        let first = S3Replica::new(
            Client::new(manifest.first.clone(), copy(), clock.clone())?,
            stream,
        )?;
        let second = S3Replica::new(
            Client::new(manifest.second.clone(), copy(), clock.clone())?,
            stream,
        )?;
        let mut witness = DynamoWitness::new(
            Client::new(manifest.witness.clone(), self.witness, clock.clone())?,
            stream,
        )?;
        let anchor = witness.read(stream).map_err(|_| Error)?;
        if anchor.epoch != manifest.epoch {
            return Err(Error);
        }
        let backend =
            Replicated::new(stream, manifest.epoch, first, second, witness).map_err(|_| Error)?;
        let cipher = RecordCipher::new(self.storage, manifest.generation, manifest.stream)
            .map_err(|_| Error)?;
        let fresh = anchor.head.is_none();
        let mut store = if fresh {
            Journal::create(backend, cipher, self.config)
        } else {
            Journal::open(backend, cipher, self.config)
        }
        .map_err(|_| Error)?;
        if store.verified_state().map_err(|_| Error)?.logical_time() > now {
            return Err(Error);
        }
        self.api.initialize(&mut store, now).map_err(|_| Error)?;
        initialize_funding(&self.funding, &mut store, clock.as_ref())?;
        let deadline = now
            .checked_add(manifest.gates.maximum_boot_ms)
            .ok_or(Error)?;
        let private_clock: Arc<dyn Clock> = Arc::new(LeaseClock {
            clock: clock.clone(),
            deadline,
            stop: stop.clone(),
        });
        let poll_ms = self.chain.as_ref().map(chain_funding::Loaded::poll_ms);
        if let Some(chain) = &self.chain {
            let peer = manifest.chain.as_ref().ok_or(Error)?;
            if chain.peer() != (peer.host.as_str(), peer.port) {
                return Err(Error);
            }
        }
        if manifest.gates.native_reads {
            self.gateway
                .initialize_reads(&mut store, commit_id()?, private_clock.now()?)
                .map_err(|_| Error)?;
        }
        let chain = self
            .chain
            .map(|chain| chain.open(private_clock.clone()))
            .transpose()?;
        let egress = Egress::new(
            self.origin,
            Target::new(3, manifest.venue_port)?,
            Trust::from_der(&manifest.venue_root, manifest.venue_root_hash)?,
            private_clock,
        )?;
        Ok(Runtime {
            active: Mutex::new(Active {
                store,
                gateway: self.gateway,
                funding: self.funding,
                egress,
                chain,
                poll_ms,
                next_poll: now,
                read_kind: 0,
                deposit_index: 0,
            }),
            api: self.api,
            clock,
            stop,
            deadline,
            gates: manifest.gates.clone(),
        })
    }
}
struct LeaseClock {
    clock: Arc<dyn Clock>,
    deadline: u64,
    stop: Arc<AtomicBool>,
}
impl Clock for LeaseClock {
    fn now(&self) -> Result<u64, Error> {
        let now = self.clock.now()?;
        if self.stop.load(Ordering::SeqCst) || now >= self.deadline {
            return Err(Error);
        }
        Ok(now)
    }
}
fn initialize_funding<B: Backend, P: Protection>(
    funding: &Controller,
    store: &mut Journal<B, P>,
    clock: &dyn Clock,
) -> Result<(), Error> {
    if !funding.bound(store, clock.now()?).map_err(|_| Error)? {
        funding
            .bind(store, commit_id()?, clock.now()?)
            .map_err(|_| Error)?;
    }
    Ok(())
}
fn commit_id() -> Result<CommitId, Error> {
    let mut id = [0; 32];
    openssl::rand::rand_bytes(&mut id)?;
    CommitId::new(id).map_err(|_| Error)
}

impl<B: Backend + Send, P: Protection + Send> Runtime<B, P> {
    fn active_time(&self) -> Result<u64, Error> {
        if self.stop.load(Ordering::SeqCst) {
            return Err(Error);
        }
        let now = match self.clock.now() {
            Ok(n) => n,
            Err(e) => {
                self.fence();
                return Err(e);
            }
        };
        if now >= self.deadline {
            self.fence();
            return Err(Error);
        }
        Ok(now)
    }
    /// Stop new sessions and scheduling; shutdown drops loaded key material.
    pub fn fence(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }
    /// Coarse local status; never exposes customer state or secret errors.
    pub fn health(&self) -> Health {
        if self.stop.load(Ordering::SeqCst) {
            Health::Fenced
        } else {
            Health::Ready
        }
    }
    /// One bounded scheduler cut. No new policy, automatic funding or synthetic
    /// native evidence. Restarts do not resend any possibly-exposed attempt.
    pub fn tick(&self) -> Result<(), Error> {
        let result = (|| {
            let now = self.active_time()?;
            let mut active = self.active.lock().map_err(|_| Error)?;
            let Active {
                store,
                gateway,
                funding,
                egress,
                chain,
                poll_ms,
                next_poll,
                read_kind,
                deposit_index,
            } = &mut *active;
            funding
                .release_commitment(store.configuration())
                .map_err(|_| Error)?;
            let state = store.verified_state().map_err(|_| Error)?;
            if state.logical_time() > now {
                return Err(Error);
            }
            if self.gates.native_reads && now >= *next_poll {
                let cleanup = state.frozen();
                let interval = poll_ms.ok_or(Error)?;
                if let Some(chain) = chain.as_mut() {
                    poll_deposit(chain, store, funding, deposit_index)?;
                }
                if gateway
                    .read_available(store, now, cleanup)
                    .map_err(|_| Error)?
                {
                    use cinder_pacifica::{
                        observation::Kind,
                        reads::{self, Diagnostic, DiagnosticPoll, Poll},
                    };
                    let kinds = [
                        Kind::Trades,
                        Kind::Orders,
                        Kind::Positions,
                        Kind::Account,
                        Kind::Funding,
                    ];
                    let diagnostics = [
                        Diagnostic::Settings,
                        Diagnostic::Loan,
                        Diagnostic::BalanceHistory,
                        Diagnostic::WithdrawalPending,
                        Diagnostic::WithdrawalHistory,
                    ];
                    if *read_kind < kinds.len() {
                        let kind = kinds[*read_kind];
                        let cursor = gateway.read_cursor(store, kind).map_err(|_| Error)?;
                        reads::poll(
                            store,
                            gateway,
                            Poll {
                                kind,
                                cursor,
                                reservation: commit_id()?,
                                evidence: commit_id()?,
                                at: self.active_time()?,
                                cleanup,
                            },
                            egress,
                        )
                        .map_err(|_| Error)?;
                    } else {
                        reads::diagnostic(
                            store,
                            gateway,
                            DiagnosticPoll {
                                kind: diagnostics[*read_kind - kinds.len()],
                                reservation: commit_id()?,
                                evidence: commit_id()?,
                                at: self.active_time()?,
                                cleanup,
                            },
                            egress,
                        )
                        .map_err(|_| Error)?;
                    }
                    *read_kind = (*read_kind + 1) % (kinds.len() + diagnostics.len());
                }
                // Keep a request-service interval after slow I/O, rather than
                // starting the next poll immediately because its start-time cut expired.
                *next_poll = self.active_time()?.checked_add(interval).ok_or(Error)?;
            }
            if self.gates.funding {
                let next = store
                    .verified_state()
                    .map_err(|_| Error)?
                    .funds()
                    .iter()
                    .find(|o| !o.terminal && o.attempt.is_some())
                    .and_then(|o| o.attempt);
                if let Some(attempt) = next {
                    let exposed = store
                        .state()
                        .map_err(|_| Error)?
                        .attempts()
                        .iter()
                        .find(|a| a.key == attempt)
                        .ok_or(Error)?
                        .possibly_exposed;
                    let native = store
                        .state()
                        .map_err(|_| Error)?
                        .funds()
                        .iter()
                        .find(|o| o.attempt == Some(attempt))
                        .is_some_and(|o| o.intent.source == cinder_kernel::ledger::Location::Venue);
                    if native {
                        if !exposed {
                            funding
                                .withdraw(
                                    store,
                                    gateway,
                                    cinder_pacifica::execution::Dispatch {
                                        attempt,
                                        commit: commit_id()?,
                                        at: self.active_time()?,
                                    },
                                    egress,
                                )
                                .map_err(|_| Error)?;
                        }
                        // Native finality/causal provider remains explicitly gated.
                    } else {
                        let chain = chain.as_mut().ok_or(Error)?;
                        if exposed {
                            chain.reconcile(store, funding, attempt)?;
                        } else {
                            chain.issue(store, funding, attempt)?;
                        }
                    }
                }
            }
            // Loaded contracts stay bound during every cut; their existence is
            // not financial activation. The manifest still refuses live trading.
            if self.gates.trading {
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
                if let Some(attempt) = pending {
                    gateway
                        .dispatch(
                            store,
                            cinder_pacifica::execution::Dispatch {
                                attempt,
                                commit: commit_id()?,
                                at: self.active_time()?,
                            },
                            egress,
                        )
                        .map_err(|_| Error)?;
                }
            }
            Ok(())
        })();
        if result.is_err() {
            self.fence();
        }
        result
    }
}
impl<B: Backend + Send, P: Protection + Send> Runtime<B, P> {
    fn handle_available(
        &self,
        channel: &Session,
        request: PrivateBytes,
        now: u64,
    ) -> Result<Option<PrivateBytes>, Error> {
        let result = (|| {
            self.active_time()?;
            let Some(mut active) = lock_bounded(&self.active, REQUEST_WAIT)? else {
                return Ok(None);
            };
            // Waiting cannot reuse an expired lease or pre-wait authentication time.
            let current = self.active_time()?;
            if current < now {
                return Err(Error);
            }
            if active.store.verified_state().is_err() {
                self.fence();
                return Err(Error);
            }
            let response = self
                .api
                .handle(&mut active.store, channel, &request, current);
            match response {
                Ok(r) => r.encode(),
                Err(e) => e.encode_private(),
            }
            .map(Some)
            .map_err(|_| Error)
        })();
        // Contention is not a permanent key revocation. Journal/clock failures
        // are caught by tick and private API already refuses stale state.
        result
    }
}
impl<B: Backend + Send, P: Protection + Send> Handler for Runtime<B, P> {
    fn handle(
        &self,
        channel: &Session,
        request: PrivateBytes,
        now: u64,
    ) -> Result<PrivateBytes, Error> {
        self.handle_available(channel, request, now)?.ok_or(Error)
    }
    fn poll(
        &self,
        channel: &Session,
        request: PrivateBytes,
        now: u64,
    ) -> Result<Option<PrivateBytes>, Error> {
        if !cinder_api::wire::Request::decode(request.as_bytes())
            .is_ok_and(|r| matches!(r.command, cinder_api::wire::Command::Read(_)))
        {
            return Err(Error);
        }
        self.handle_available(channel, request, now)
    }
}

#[cfg(test)]
#[path = "../../journal/tests/support/mod.rs"]
pub(crate) mod initialization_support;

#[cfg(test)]
mod initialization_tests {
    use super::*;
    use cinder_journal::sqlite::{Migration, SqliteBackend};
    use cinder_kernel::ledger::{Location, Source};
    use cinder_pacifica::{execution::Origin, funding::Beneficiary, profile::Grid};
    use initialization_support::{FixtureProtection, Temp};
    struct Fixed(Result<u64, Error>);
    impl Clock for Fixed {
        fn now(&self) -> Result<u64, Error> {
            self.0
        }
    }
    #[test]
    fn request_wait_is_bounded_and_short_poll_contention_does_not_fail_immediately() {
        use std::{sync::mpsc, time::Duration};
        let mutex = Mutex::new(0);
        std::thread::scope(|scope| {
            let (ready_tx, ready_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            let shared = &mutex;
            scope.spawn(move || {
                let mut guard = shared.lock().unwrap();
                ready_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                *guard = 1;
            });
            ready_rx.recv().unwrap();
            assert!(lock_bounded(&mutex, Duration::ZERO).unwrap().is_none());
            scope.spawn(move || {
                std::thread::sleep(Duration::from_millis(20));
                release_tx.send(()).unwrap();
            });
            assert_eq!(
                *lock_bounded(&mutex, Duration::from_secs(2))
                    .unwrap()
                    .unwrap(),
                1
            );
        });
        let held = mutex.lock().unwrap();
        assert!(
            lock_bounded(&mutex, Duration::from_millis(5))
                .unwrap()
                .is_none()
        );
        drop(held);
        std::thread::scope(|scope| {
            assert!(
                scope
                    .spawn(|| {
                        let _held = mutex.lock().unwrap();
                        panic!("synthetic poisoned controller");
                    })
                    .join()
                    .is_err()
            );
        });
        assert!(lock_bounded(&mutex, REQUEST_WAIT).is_err());
    }
    #[test]
    fn every_egress_step_rechecks_the_finite_boot_lease_and_stop_flag() {
        let stop = Arc::new(AtomicBool::new(false));
        let clock = LeaseClock {
            clock: Arc::new(Fixed(Ok(100))),
            deadline: 101,
            stop: stop.clone(),
        };
        assert_eq!(clock.now().unwrap(), 100);
        stop.store(true, Ordering::SeqCst);
        assert!(clock.now().is_err());
        for inner in [Ok(101), Ok(102), Err(Error)] {
            let clock = LeaseClock {
                clock: Arc::new(Fixed(inner)),
                deadline: 101,
                stop: Arc::new(AtomicBool::new(false)),
            };
            assert!(clock.now().is_err());
        }
    }
    #[test]
    fn restart_repairs_binding_after_committed_genesis_without_rebinding() {
        let mut config = initialization_support::config();
        for (tag, location) in [(8, Location::Vault), (9, Location::Broker)] {
            config.sources.push(Source {
                scope: EventScope {
                    namespace: NamespaceId::new([tag; 32]).unwrap(),
                    ..config.sources[0].scope
                },
                location,
            });
        }
        let broker =
            openssl::pkey::PKey::private_key_from_raw_bytes(&[9; 32], openssl::pkey::Id::ED25519)
                .unwrap()
                .raw_public_key()
                .unwrap()
                .try_into()
                .unwrap();
        let profile = Profile {
            config: config.clone(),
            source: config.sources[0].scope,
            account: "J2xccRtuG43drESLYznHhLhQkLTdfepcKYbiQ9BsJVaf".into(),
            environment: Origin::Testnet.url().into(),
            revision: 1,
            evidence: "offline interrupted-boot fixture; not AWS evidence".into(),
            precision: Level::Qualified,
            fills: Level::Qualified,
            quote_places: 0,
            perp_tag: 0,
            markets: vec![Mapping {
                symbol: "BTC".into(),
                market: 0,
                size: Grid { places: 0, step: 1 },
                price: Grid { places: 0, step: 1 },
            }],
        };
        let route = Route {
            domain: config.domain.deployment.bytes(),
            pool: [18; 32],
            funds: [19; 32],
            beneficiaries: (1..=2)
                .map(|n| Beneficiary {
                    account: [n; 32],
                    wallet: [n + 10; 32],
                    tokens: [n + 33; 32],
                })
                .collect(),
            program: [20; 32],
            config: [21; 32],
            vault: [22; 32],
            mint: [23; 32],
            broker,
            broker_tokens: [24; 32],
            venue_program: [25; 32],
            venue_vault: [26; 32],
            epoch: 1,
            decimals: 0,
            withdrawal: Level::Qualified,
            chain: Level::Qualified,
            settings: Level::Qualified,
            withdrawal_cost: 10,
            maximum_movement: 1000,
            maximum_fee: 5,
            setup_max_age: 10_000,
        };
        let funding = Controller::new(profile, route, Zeroizing::new([9; 32])).unwrap();
        let temp = Temp::new();
        let journal = Journal::create(
            SqliteBackend::create(&temp.db).unwrap(),
            FixtureProtection,
            config.clone(),
        )
        .unwrap();
        // Genesis is durable, but the process stopped before the route was bound.
        drop(journal);
        let mut journal = Journal::open(
            SqliteBackend::open(&temp.db, Migration::None).unwrap(),
            FixtureProtection,
            config,
        )
        .unwrap();
        let head = journal.head();
        assert!(!funding.bound(&mut journal, 100).unwrap());
        assert_eq!(journal.head(), head);
        assert!(funding.funding_need(&mut journal, 100, 1).is_err());
        assert!(initialize_funding(&funding, &mut journal, &Fixed(Err(Error))).is_err());
        assert_eq!(journal.head(), head);
        initialize_funding(&funding, &mut journal, &Fixed(Ok(100))).unwrap();
        assert!(funding.bound(&mut journal, 100).unwrap());
        assert!(funding.funding_need(&mut journal, 100, 1).is_ok());
        assert!(!journal.state().unwrap().native_funding_ready());
        let head = journal.head();
        initialize_funding(&funding, &mut journal, &Fixed(Ok(100))).unwrap();
        assert!(funding.bound(&mut journal, 99).is_err());
        assert_eq!(journal.head(), head);
    }
}
