//! Opt-in unit handler on the existing confidential TLS/Noise carriers. Native
//! workers and a governed deployment manifest are separate activation gates.
use crate::{
    Error,
    transport::{Clock, Handler, Prepared, PreparedReply, Session},
};
use cinder_api::{
    ConfidentialChannel,
    units::{self as api, service::Service},
    wire::Command,
};
use cinder_journal::{
    Backend, Protection,
    model::PrivateBytes,
    read::{Binding, Failure, Reader, Verified, Witness},
    units::Journal,
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const RELEASE: Duration = Duration::from_millis(250);

/// One shared writer, immutable publication and trusted clock/witness. No native
/// key, parent plaintext, mutable grant database or independently funded books.
pub struct Runtime<B: Backend, P: Protection, A> {
    journal: Mutex<Journal<B, P>>,
    api: Service<A>,
    reader: Reader,
    witness: Arc<dyn Witness>,
    clock: Arc<dyn Clock>,
    stop: AtomicBool,
    deadline: u64,
}
pub(crate) struct ReadCandidate {
    view: Verified,
    request: PrivateBytes,
    encoded: PrivateBytes,
    evaluated_at: u64,
    session: [u8; 32],
    periodic: bool,
}
struct PanicFence<'a>(&'a Reader, &'a AtomicBool);
impl Drop for PanicFence<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.1.store(true, Ordering::SeqCst);
            self.0.close();
        }
    }
}
impl<B: Backend, P: Protection, A: cinder_api::Admission> Runtime<B, P, A> {
    /// Attach after trusted configuration/history verification. Constructing
    /// this handler does not qualify clocks, storage, image or native support.
    pub fn open(
        mut journal: Journal<B, P>,
        api: Service<A>,
        binding: Binding,
        witness: Arc<dyn Witness>,
        clock: Arc<dyn Clock>,
        deadline: u64,
    ) -> Result<Self, Error> {
        if binding.api != api.commitment(journal.configuration()).map_err(|_| Error)?
            || clock.now()? >= deadline
        {
            return Err(Error);
        }
        let reader = journal.attach_reader(binding).map_err(|_| Error)?;
        Ok(Self {
            journal: Mutex::new(journal),
            api,
            reader,
            witness,
            clock,
            stop: AtomicBool::new(false),
            deadline,
        })
    }
    /// Sticky boot fence invalidates all outstanding and future read tickets.
    pub fn fence(&self) {
        self.stop.store(true, Ordering::SeqCst);
        self.reader.close();
    }
    fn now(&self) -> Result<u64, Error> {
        if self.stop.load(Ordering::SeqCst) || self.reader.fenced() {
            return Err(Error);
        }
        match self.clock.now() {
            Ok(now) if now < self.deadline => Ok(now),
            _ => {
                self.fence();
                Err(Error)
            }
        }
    }
    fn failed(
        &self,
        e: Failure,
        periodic: bool,
        unit: cinder_kernel::identity::UnitId,
    ) -> Result<Option<PreparedReply>, Error> {
        match e {
            Failure::Raced | Failure::Busy if periodic => Ok(None),
            Failure::Raced | Failure::Busy => Ok(Some(PreparedReply::immediate(
                api::encode_error(unit, cinder_api::Error::Unavailable).map_err(|_| Error)?,
            ))),
            Failure::Fenced => {
                self.fence();
                Err(Error)
            }
        }
    }
    fn prepare(
        &self,
        channel: &Session,
        request: PrivateBytes,
        now: u64,
        periodic: bool,
    ) -> Result<Option<PreparedReply>, Error> {
        let _panic = PanicFence(&self.reader, &self.stop);
        let current = self.now()?;
        if current < now {
            self.fence();
            return Err(Error);
        }
        let r = api::Request::decode(request.as_bytes()).map_err(|_| Error)?;
        if periodic && !matches!(r.body.command, Command::Read(q) if q.cursor == [0;40]) {
            return Err(Error);
        }
        if matches!(
            r.body.command,
            Command::Read(_) | Command::View | Command::Operation(_)
        ) {
            let ticket = match self.reader.capture() {
                Ok(t) => t,
                Err(e) => return self.failed(e, periodic, r.unit),
            };
            self.api
                .authenticate_read(
                    ticket.unit_configuration().map_err(|_| Error)?,
                    channel,
                    &request,
                    current,
                )
                .map_err(|_| Error)?;
            let view = match self.reader.verify(ticket, &*self.witness) {
                Ok(v) => v,
                Err(e) => return self.failed(e, periodic, r.unit),
            };
            let at = self.now()?;
            if at < current {
                self.fence();
                return Err(Error);
            }
            let encoded = match self.api.read(&view, channel, &request, at) {
                Ok(response) => api::encode_response(r.unit, &response),
                Err(e) => api::encode_error(r.unit, e),
            }
            .map_err(|_| Error)?;
            return Ok(Some(PreparedReply {
                inner: Prepared::Unit(ReadCandidate {
                    view,
                    request,
                    encoded,
                    evaluated_at: at,
                    session: channel.binding(),
                    periodic,
                }),
            }));
        }
        let mut writer = match self.journal.try_lock() {
            Ok(w) => w,
            Err(std::sync::TryLockError::WouldBlock) => {
                return self.failed(Failure::Busy, false, r.unit);
            }
            Err(_) => {
                self.fence();
                return Err(Error);
            }
        };
        let at = self.now()?;
        if at < current {
            self.fence();
            return Err(Error);
        }
        let reply = match self.api.mutate(&mut writer, channel, &request, at) {
            Ok(response) => api::encode_response(r.unit, &response),
            Err(e) => api::encode_error(r.unit, e),
        }
        .map_err(|_| Error)?;
        Ok(Some(PreparedReply::immediate(reply)))
    }
    fn release(
        &self,
        channel: &Session,
        reply: PreparedReply,
    ) -> Result<Option<PrivateBytes>, Error> {
        let _panic = PanicFence(&self.reader, &self.stop);
        self.now()?;
        let candidate = match reply.inner {
            Prepared::Immediate(bytes) => return Ok(Some(bytes)),
            Prepared::Unit(c) => c,
            Prepared::Accepted(_) => return Err(Error),
        };
        let ReadCandidate {
            view,
            request,
            encoded,
            evaluated_at,
            session,
            periodic,
        } = candidate;
        if session != channel.binding() || !self.reader.owns(&view) {
            return Err(Error);
        }
        let until = Instant::now() + RELEASE;
        let now = self.now()?;
        if now < evaluated_at {
            self.fence();
            return Err(Error);
        }
        let upper = now.checked_add(RELEASE.as_millis() as u64).ok_or(Error)?;
        if upper >= self.deadline {
            self.fence();
            return Err(Error);
        }
        let unit = api::Request::decode(request.as_bytes())
            .map_err(|_| Error)?
            .unit;
        let encoded = match self.api.revalidate_read(&view, channel, &request, upper) {
            Ok(()) => encoded,
            Err(e) => api::encode_error(unit, e).map_err(|_| Error)?,
        };
        match view.release_if_running(until, &self.stop) {
            Ok(()) => Ok(Some(encoded)),
            Err(Failure::Raced | Failure::Busy) if periodic => Ok(None),
            Err(Failure::Raced | Failure::Busy) => {
                api::encode_error(unit, cinder_api::Error::Unavailable)
                    .map(Some)
                    .map_err(|_| Error)
            }
            Err(Failure::Fenced) => {
                self.fence();
                Err(Error)
            }
        }
    }
}
impl<B: Backend + Send, P: Protection + Send, A: cinder_api::Admission + Send + Sync> Handler
    for Runtime<B, P, A>
{
    fn handle(
        &self,
        channel: &Session,
        request: PrivateBytes,
        now: u64,
    ) -> Result<PrivateBytes, Error> {
        let reply = self.prepare(channel, request, now, false)?.ok_or(Error)?;
        self.release(channel, reply)?.ok_or(Error)
    }
    fn prepare_handle(
        &self,
        channel: &Session,
        request: PrivateBytes,
        now: u64,
    ) -> Result<PreparedReply, Error> {
        self.prepare(channel, request, now, false)?.ok_or(Error)
    }
    fn prepare_poll(
        &self,
        channel: &Session,
        request: PrivateBytes,
        now: u64,
    ) -> Result<Option<PreparedReply>, Error> {
        self.prepare(channel, request, now, true)
    }
    fn release_reply(
        &self,
        channel: &Session,
        reply: PreparedReply,
    ) -> Result<Option<PrivateBytes>, Error> {
        self.release(channel, reply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support as support;
    use cinder_journal::{
        Error as JournalError, Frame, Head,
        encrypted::RecordCipher,
        model::{CommitId, Reservation, State},
        orders,
        replicated::{Anchor, Stream},
        units,
    };
    use cinder_kernel::identity::*;
    use openssl::{
        pkey::{Id, PKey, Private},
        sign::Signer,
    };
    use std::sync::{atomic::AtomicU64, mpsc};
    use zeroize::Zeroizing;

    #[derive(Default)]
    struct Shared {
        frames: Mutex<Vec<Frame>>,
        fail: AtomicBool,
        unknown: AtomicBool,
        barrier: Mutex<Option<(mpsc::Sender<()>, mpsc::Receiver<()>)>>,
    }
    struct Storage(Arc<Shared>);
    impl Backend for Storage {
        fn load(&mut self) -> Result<Vec<Frame>, JournalError> {
            Ok(self.0.frames.lock().unwrap().clone())
        }
        fn check_current(&mut self, expected: Head) -> Result<(), JournalError> {
            if self.0.frames.lock().unwrap().last().map(|f| f.head) == Some(expected) {
                Ok(())
            } else {
                Err(JournalError::Stale)
            }
        }
        fn append(&mut self, expected: Option<Head>, frame: &Frame) -> Result<(), JournalError> {
            let mut frames = self.0.frames.lock().unwrap();
            if frames.last().map(|f| f.head) != expected {
                return Err(JournalError::Stale);
            }
            frames.push(frame.clone());
            if self.0.unknown.swap(false, Ordering::SeqCst) {
                Err(JournalError::Storage)
            } else {
                Ok(())
            }
        }
    }
    struct Fresh(Arc<Shared>);
    impl Witness for Fresh {
        fn read(&self, _: Stream) -> Result<Anchor, JournalError> {
            if let Some((entered, resume)) = self.0.barrier.lock().unwrap().take() {
                entered.send(()).unwrap();
                resume.recv_timeout(Duration::from_secs(5)).unwrap();
            }
            if self.0.fail.load(Ordering::SeqCst) {
                return Err(JournalError::Storage);
            }
            Ok(Anchor {
                epoch: 1,
                head: self.0.frames.lock().unwrap().last().map(|f| f.head),
            })
        }
    }
    struct Time(AtomicU64);
    impl Clock for Time {
        fn now(&self) -> Result<u64, Error> {
            Ok(self.0.load(Ordering::SeqCst))
        }
    }
    struct NoOrders;
    impl cinder_api::Admission for NoOrders {
        fn order_holds(
            &self,
            _: &State,
            _: &orders::Intent,
        ) -> Result<Vec<Reservation>, cinder_api::Error> {
            Err(cinder_api::Error::Unavailable)
        }
    }
    fn key(n: u8) -> PKey<Private> {
        PKey::private_key_from_raw_bytes(&[n; 32], Id::ED25519).unwrap()
    }
    fn public(n: u8) -> [u8; 32] {
        key(n).raw_public_key().unwrap().try_into().unwrap()
    }
    fn uid(n: u8) -> UnitId {
        UnitId::new([n; 32]).unwrap()
    }
    fn domain() -> Domain {
        support::config().domain
    }
    fn account() -> AccountId {
        cinder_journal::onboarding::account(domain(), public(9)).unwrap()
    }
    fn config() -> units::Configuration {
        units::Configuration::new(
            domain(),
            [40_u8, 50]
                .into_iter()
                .map(|n| {
                    let mut ledger = support::config();
                    ledger.customers.clear();
                    ledger.venue = VenueId::new([n + 4; 32]).unwrap();
                    ledger.venue_account = VenueAccountId::new([n + 5; 32]).unwrap();
                    ledger.sources[0].scope.venue = ledger.venue;
                    ledger.sources[0].scope.account = ledger.venue_account;
                    units::Descriptor {
                        id: uid(n),
                        environment: [90; 32],
                        native_account: [n + 1; 32],
                        funding_wallet: [n + 2; 32],
                        custody_config: [n + 3; 32],
                        payout_vault: [n + 6; 32],
                        key_policy: [n + 7; 32],
                        ledger,
                    }
                })
                .collect(),
        )
        .unwrap()
    }
    type App = Runtime<Storage, RecordCipher, NoOrders>;
    fn fixture() -> (App, Arc<Shared>, Arc<Time>) {
        let shared = Arc::new(Shared::default());
        let mut journal = Journal::create(
            Storage(shared.clone()),
            RecordCipher::new(Zeroizing::new([42; 32]), 1, [89; 32]).unwrap(),
            config(),
        )
        .unwrap();
        journal
            .commit(units::Transaction {
                id: CommitId::new([1; 32]).unwrap(),
                expected: journal.head(),
                at: 1,
                change: units::Change::Enroll {
                    wallet: public(9),
                    recipients: vec![
                        units::Recipient {
                            unit: uid(40),
                            tokens: [71; 32],
                        },
                        units::Recipient {
                            unit: uid(50),
                            tokens: [72; 32],
                        },
                    ],
                },
            })
            .unwrap();
        let api = Service::new(api::Authorizer::new(2000, 10000).unwrap(), NoOrders);
        let binding = Binding {
            stream: Stream {
                domain: domain(),
                id: [91; 32],
            },
            epoch: 1,
            api: api.commitment(journal.configuration()).unwrap(),
        };
        let time = Arc::new(Time(AtomicU64::new(10)));
        (
            Runtime::open(
                journal,
                api,
                binding,
                Arc::new(Fresh(shared.clone())),
                time.clone(),
                10000,
            )
            .unwrap(),
            shared,
            time,
        )
    }
    fn session() -> Session {
        Session::established(domain(), [8; 32], 5000)
    }
    fn request(unit: u8, id: u8, signer: u8, command: Command) -> PrivateBytes {
        let mut r = api::Request {
            unit: uid(unit),
            body: cinder_api::wire::Request {
                domain: domain(),
                account: account(),
                id: RequestId::new([id; 32]).unwrap(),
                policy: PolicyVersion::new(1).unwrap(),
                epoch: 1,
                signer: public(signer),
                session: [8; 32],
                expires_at: 2000,
                command,
                signature: [0; 64],
            },
        };
        r.body.signature = Signer::new_without_digest(&key(signer))
            .unwrap()
            .sign_oneshot_to_vec(&r.message())
            .unwrap()
            .try_into()
            .unwrap();
        r.encode().unwrap()
    }
    fn grant() -> Command {
        Command::Grant(cinder_api::wire::Grant {
            key: public(10),
            methods: 1,
            market: MarketId::new([7; 32]).unwrap(),
            maximum_lots: 1,
            maximum_fee: 0,
            maximum_orders: 1,
            expires_at: 5000,
        })
    }
    #[test]
    fn actual_unit_handler_uses_shared_publication_and_rejects_foreign_candidate() {
        let (app, _, _) = fixture();
        let (foreign, _, _) = fixture();
        for unit in [40, 50] {
            let reply = app
                .handle(&session(), request(unit, 20, 9, Command::View), 10)
                .unwrap();
            assert!(reply.as_bytes().starts_with(
                &[b"CINDER-API-REPLY\0\x00\x02".as_slice(), &uid(unit).bytes()].concat()
            ));
        }
        let prepared = app
            .prepare_handle(&session(), request(40, 20, 9, Command::View), 10)
            .unwrap();
        assert!(foreign.release_reply(&session(), prepared).is_err());
        assert!(
            app.prepare_poll(&session(), request(40, 21, 9, grant()), 10)
                .is_err()
        );
    }
    #[test]
    fn a_revoke_in_either_unit_prevents_stale_read_release() {
        let (app, _, _) = fixture();
        app.handle(&session(), request(40, 21, 9, grant()), 10)
            .unwrap();
        let prepared = app
            .prepare_handle(&session(), request(40, 22, 10, Command::View), 10)
            .unwrap();
        app.handle(&session(), request(50, 23, 9, Command::Revoke), 10)
            .unwrap();
        let reply = app.release_reply(&session(), prepared).unwrap().unwrap();
        assert_eq!(
            reply,
            api::encode_error(uid(40), cinder_api::Error::Unavailable).unwrap()
        );
        assert!(app.reader.capture().is_ok());
    }
    #[test]
    fn witness_io_does_not_hold_writer_and_known_generation_race_skips_periodic_output() {
        let (app, shared, _) = fixture();
        let (entered, wait) = mpsc::channel();
        let (resume, blocked) = mpsc::channel();
        *shared.barrier.lock().unwrap() = Some((entered, blocked));
        std::thread::scope(|scope| {
            let pending = scope.spawn(|| {
                app.prepare_poll(
                    &session(),
                    request(
                        40,
                        20,
                        9,
                        Command::Read(cinder_api::reads::Query {
                            kind: 0,
                            cursor: [0; 40],
                            limit: 1,
                        }),
                    ),
                    10,
                )
            });
            wait.recv_timeout(Duration::from_secs(5)).unwrap();
            app.handle(&session(), request(50, 21, 9, grant()), 10)
                .unwrap();
            resume.send(()).unwrap();
            assert!(pending.join().unwrap().unwrap().is_none());
        });
        assert!(!app.reader.fenced());
    }
    #[test]
    fn unknown_append_and_failed_witness_fence_all_unit_reads() {
        for unknown in [false, true] {
            let (app, shared, _) = fixture();
            if unknown {
                shared.unknown.store(true, Ordering::SeqCst);
                assert!(
                    app.handle(&session(), request(40, 21, 9, grant()), 10)
                        .is_err()
                );
                assert_eq!(shared.frames.lock().unwrap().len(), 3);
            } else {
                shared.fail.store(true, Ordering::SeqCst);
            }
            assert!(
                app.handle(&session(), request(50, 22, 9, Command::View), 10)
                    .is_err()
            );
            assert!(app.reader.fenced());
        }
    }
    #[test]
    fn read_capacity_is_shared_and_release_rechecks_expiry_and_session() {
        let (app, _, time) = fixture();
        let mut held = Vec::new();
        for unit in [40, 50, 40, 50] {
            held.push(
                app.prepare_handle(&session(), request(unit, 20, 9, Command::View), 10)
                    .unwrap(),
            );
        }
        let busy = app
            .prepare_handle(&session(), request(50, 20, 9, Command::View), 10)
            .unwrap();
        assert_eq!(
            app.release_reply(&session(), busy).unwrap().unwrap(),
            api::encode_error(uid(50), cinder_api::Error::Unavailable).unwrap()
        );
        assert!(!app.reader.fenced());
        let candidate = held.pop().unwrap();
        let foreign = Session::established(domain(), [9; 32], 5000);
        assert!(app.release_reply(&foreign, candidate).is_err());
        let candidate = app
            .prepare_handle(&session(), request(50, 20, 9, Command::View), 10)
            .unwrap();
        time.0.store(1900, Ordering::SeqCst);
        assert_eq!(
            app.release_reply(&session(), candidate).unwrap().unwrap(),
            api::encode_error(uid(50), cinder_api::Error::Unauthorized).unwrap()
        );
        drop(held);
        assert!(app.reader.capture().is_ok());
    }
}
