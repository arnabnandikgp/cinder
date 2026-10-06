//! Offline diagnosis only: real Runtime/API/AEAD/Replicated, synthetic I/O.
//! No sockets, AWS credentials, native dispatch, funds or production changes.
use super::*;
use crate::fixture::FixtureAttester;
use cinder_api::wire::{Command, Grant, READ, Request};
use cinder_journal::{Error as JournalError, Frame, Head, replicated::*};
use cinder_kernel::ledger::{Location, Source};
use cinder_pacifica::{execution::Origin, funding::Beneficiary, profile::Grid};
use openssl::{
    pkey::{Id, PKey},
    sign::Signer,
};
use std::{
    sync::{
        atomic::{AtomicU64, AtomicUsize},
        mpsc,
    },
    time::{Duration, Instant},
};

#[derive(Default)]
struct Metrics {
    gets: AtomicUsize,
    puts: AtomicUsize,
    reads: AtomicUsize,
    accepts: AtomicUsize,
    witness_delay_ms: AtomicU64,
    entered: Mutex<Option<mpsc::Sender<()>>>,
    anchor: Mutex<Option<Arc<Mutex<Anchor>>>>,
    private_delay_ms: AtomicU64,
    private_reads: AtomicUsize,
    private_entered: Mutex<Option<mpsc::Sender<()>>>,
    private_failure: AtomicBool,
}
impl Metrics {
    fn counts(&self) -> [usize; 4] {
        [&self.gets, &self.puts, &self.reads, &self.accepts].map(|n| n.load(Ordering::SeqCst))
    }
    fn reset(&self) {
        for n in [&self.gets, &self.puts, &self.reads, &self.accepts] {
            n.store(0, Ordering::SeqCst);
        }
    }
}
struct MemoryReplica {
    id: u8,
    frames: BTreeMap<[u8; 32], Frame>,
    metrics: Arc<Metrics>,
}
impl Replica for MemoryReplica {
    fn identity(&self) -> [u8; 32] {
        [self.id; 32]
    }
    fn put(&mut self, frame: &Frame) -> Result<(), JournalError> {
        self.metrics.puts.fetch_add(1, Ordering::SeqCst);
        self.frames.insert(frame.head.hash, frame.clone());
        Ok(())
    }
    fn get(&mut self, digest: [u8; 32]) -> Result<Frame, JournalError> {
        self.metrics.gets.fetch_add(1, Ordering::SeqCst);
        self.frames
            .get(&digest)
            .cloned()
            .ok_or(JournalError::Storage)
    }
}
struct MemoryWitness {
    stream: Stream,
    anchor: Arc<Mutex<Anchor>>,
    metrics: Arc<Metrics>,
}
impl Witness for MemoryWitness {
    fn read(&mut self, stream: Stream) -> Result<Anchor, JournalError> {
        if stream != self.stream {
            return Err(JournalError::Conflict);
        }
        self.metrics.reads.fetch_add(1, Ordering::SeqCst);
        if let Some(sender) = self.metrics.entered.lock().unwrap().take() {
            sender.send(()).unwrap();
        }
        std::thread::sleep(Duration::from_millis(
            self.metrics.witness_delay_ms.load(Ordering::SeqCst),
        ));
        Ok(*self.anchor.lock().unwrap())
    }
    fn accept(&mut self, stream: Stream, expected: Anchor, next: Head) -> Result<(), JournalError> {
        if stream != self.stream {
            return Err(JournalError::Conflict);
        }
        self.metrics.accepts.fetch_add(1, Ordering::SeqCst);
        let mut anchor = self.anchor.lock().unwrap();
        if *anchor != expected {
            return Err(JournalError::Stale);
        }
        anchor.head = Some(next);
        Ok(())
    }
}
type Backend = Replicated<MemoryReplica, MemoryReplica, MemoryWitness>;
type Store = Journal<Backend, RecordCipher>;
type Application = Runtime<Backend, RecordCipher>;
const NOW: u64 = 1_700_000_000_000;
struct Time;
impl Clock for Time {
    fn now(&self) -> Result<u64, Error> {
        Ok(NOW)
    }
}
fn key(seed: u8) -> PKey<openssl::pkey::Private> {
    PKey::private_key_from_raw_bytes(&[seed; 32], Id::ED25519).unwrap()
}
fn public(seed: u8) -> [u8; 32] {
    key(seed).raw_public_key().unwrap().try_into().unwrap()
}
fn configuration() -> Config {
    let mut c = initialization_support::config();
    for (tag, location) in [(8, Location::Vault), (9, Location::Broker)] {
        c.sources.push(Source {
            scope: EventScope {
                namespace: NamespaceId::new([tag; 32]).unwrap(),
                ..c.sources[0].scope
            },
            location,
        });
    }
    c
}
fn store(metrics: Arc<Metrics>) -> Store {
    let stream = Stream {
        domain: configuration().domain,
        id: [42; 32],
    };
    let replica = |id| MemoryReplica {
        id,
        frames: BTreeMap::new(),
        metrics: metrics.clone(),
    };
    let witness = MemoryWitness {
        stream,
        anchor: Arc::new(Mutex::new(Anchor {
            epoch: 1,
            head: None,
        })),
        metrics: metrics.clone(),
    };
    *metrics.anchor.lock().unwrap() = Some(witness.anchor.clone());
    Journal::create(
        Replicated::new(stream, 1, replica(1), replica(2), witness).unwrap(),
        RecordCipher::new(Zeroizing::new([55; 32]), 1, stream.id).unwrap(),
        configuration(),
    )
    .unwrap()
}
fn padding(store: &mut Store, n: u8) {
    let tx = Transaction {
        id: CommitId::new([n; 32]).unwrap(),
        expected: store.head(),
        at: NOW,
        evidence: vec![PrivateBytes::new(b"offline no-money history padding".to_vec()).unwrap()],
        inputs: vec![],
        controls: vec![],
        order_observations: vec![],
        funds_observations: vec![],
    };
    assert!(store.commit(tx).unwrap().receipt.controls.is_none());
}
fn application(metrics: Arc<Metrics>) -> Application {
    let config = configuration();
    let owners: Vec<_> = (1..=2)
        .map(|n| OwnerBinding {
            account: initialization_support::user(n),
            wallet: public(n + 10),
            tokens: [n + 33; 32],
        })
        .collect();
    let profile = Profile {
        config: config.clone(),
        source: config.sources[0].scope,
        account: "J2xccRtuG43drESLYznHhLhQkLTdfepcKYbiQ9BsJVaf".into(),
        environment: Origin::Testnet.url().into(),
        revision: 1,
        evidence: "offline contention diagnosis; never native qualification".into(),
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
    let gateway = Gateway::new(
        profile.clone(),
        Policy {
            revision: 1,
            evidence: profile.evidence.clone(),
            execution: Level::Qualified,
            origin: Origin::Testnet,
            expiry_ms: 3000,
            credits: 100,
            cleanup_reserve: 20,
            read_cost: 10,
        },
        Zeroizing::new([7; 32]),
        1,
    )
    .unwrap();
    let funding = Controller::new(
        profile,
        Route {
            domain: config.domain.deployment.bytes(),
            pool: [18; 32],
            funds: public(19),
            beneficiaries: owners
                .iter()
                .map(|o| Beneficiary {
                    account: o.account.bytes(),
                    wallet: o.wallet,
                    tokens: o.tokens,
                })
                .collect(),
            program: [20; 32],
            config: [21; 32],
            vault: [22; 32],
            mint: [23; 32],
            broker: public(9),
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
        },
        Zeroizing::new([9; 32]),
    )
    .unwrap();
    let api = Service::new(
        Contract {
            owners,
            maximum_auth_lifetime: 120_000,
            maximum_grant_lifetime: 120_000,
        },
        RiskAdmission { enabled: false },
    )
    .unwrap();
    let mut store = store(metrics);
    api.initialize(&mut store, NOW).unwrap();
    let root = FixtureAttester::new().unwrap().root().to_der().unwrap();
    let clock: Arc<dyn Clock> = Arc::new(Time);
    let egress = Egress::new(
        Origin::Testnet,
        Target::new(3, 9007).unwrap(),
        Trust::from_der(&root, openssl::sha::sha256(&root)).unwrap(),
        clock.clone(),
    )
    .unwrap();
    Application {
        active: Mutex::new(Active {
            store,
            gateway,
            funding,
            egress,
            chain: None,
            poll_ms: None,
            next_poll: NOW,
            read_kind: 0,
            deposit_index: 0,
        }),
        api,
        clock,
        stop: Arc::new(AtomicBool::new(false)),
        deadline: NOW + 120_000,
        gates: Gates {
            trading: false,
            funding: false,
            native_reads: false,
            maximum_boot_ms: 120_000,
        },
        reads: None,
    }
}
struct IndependentWitness {
    metrics: Arc<Metrics>,
    anchor: Arc<Mutex<Anchor>>,
}
impl ReadWitness for IndependentWitness {
    fn read(&self, stream: Stream) -> Result<Anchor, JournalError> {
        if stream
            != (Stream {
                domain: configuration().domain,
                id: [42; 32],
            })
        {
            return Err(JournalError::Stale);
        }
        self.metrics.private_reads.fetch_add(1, Ordering::SeqCst);
        if let Some(entered) = self.metrics.private_entered.lock().unwrap().take() {
            entered.send(()).unwrap();
        }
        std::thread::sleep(Duration::from_millis(
            self.metrics.private_delay_ms.load(Ordering::SeqCst),
        ));
        if self.metrics.private_failure.load(Ordering::SeqCst) {
            return Err(JournalError::Storage);
        }
        Ok(*self.anchor.lock().unwrap())
    }
}
fn independent(metrics: Arc<Metrics>) -> Application {
    let mut app = application(metrics.clone());
    let mut active = app.active.lock().unwrap();
    let binding = ReadBinding {
        stream: Stream {
            domain: configuration().domain,
            id: [42; 32],
        },
        epoch: 1,
        api: app
            .api
            .release_commitment(active.store.configuration())
            .unwrap(),
    };
    let reader = active.store.attach_reader(binding).unwrap();
    drop(active);
    let anchor = metrics.anchor.lock().unwrap().as_ref().unwrap().clone();
    app.reads = Some(ReadRuntime {
        reader,
        witness: Arc::new(IndependentWitness { metrics, anchor }),
    });
    app
}
fn session() -> Session {
    Session::established(configuration().domain, [8; 32], NOW + 120_000)
}
fn request(n: u8, command: Command, seed: u8, epoch: u64) -> PrivateBytes {
    let mut r = Request {
        domain: configuration().domain,
        account: initialization_support::user(1),
        id: RequestId::new([n; 32]).unwrap(),
        policy: configuration().policy,
        epoch,
        signer: public(seed),
        session: [8; 32],
        expires_at: NOW + 60_000,
        command,
        signature: [0; 64],
    };
    r.signature = Signer::new_without_digest(&key(seed))
        .unwrap()
        .sign_oneshot_to_vec(&r.message())
        .unwrap()
        .try_into()
        .unwrap();
    r.encode().unwrap()
}
fn read(n: u8, seed: u8, epoch: u64) -> PrivateBytes {
    request(
        n,
        Command::Read(cinder_api::reads::Query {
            kind: 2,
            cursor: [0; 40],
            limit: 8,
        }),
        seed,
        epoch,
    )
}
fn grant(n: u8) -> PrivateBytes {
    request(
        n,
        Command::Grant(Grant {
            key: public(31),
            methods: READ,
            market: configuration().markets[0].unit().market,
            maximum_lots: 1,
            maximum_fee: 0,
            maximum_orders: 1,
            expires_at: NOW + 60_000,
        }),
        11,
        1,
    )
}
fn success(reply: &PrivateBytes, kind: u8) {
    assert!(reply.as_bytes().starts_with(b"CINDER-API-REPLY\0\x00\x01"));
    assert_eq!(reply.as_bytes()[b"CINDER-API-REPLY\0\x00\x01".len()], kind);
}

#[test]
fn accepted_history_io_growth_is_measured_without_changing_durability() {
    for padding_count in [0, 8, 24] {
        let metrics = Arc::new(Metrics::default());
        let mut s = store(metrics.clone());
        for n in 0..padding_count {
            padding(&mut s, n + 10);
        }
        let history = s.head().sequence as usize + 1;
        metrics.reset();
        padding(&mut s, 200);
        let counts = metrics.counts();
        assert_eq!(counts, [3 * history + 2, 2, 5, 1]);
        eprintln!(
            "contention-history records={history} replica_gets={} replica_puts={} witness_reads={} witness_cas={}",
            counts[0], counts[1], counts[2], counts[3]
        );
    }
}

#[test]
fn real_runtime_slow_reads_and_writes_refuse_initial_reads_but_skip_busy_polls() {
    for write in [false, true] {
        let metrics = Arc::new(Metrics::default());
        let app = application(metrics.clone());
        let channel = session();
        success(&app.handle(&channel, read(1, 11, 1), NOW).unwrap(), 3);
        let head = app.active.lock().unwrap().store.head();
        metrics.reset();
        metrics.witness_delay_ms.store(400, Ordering::SeqCst);
        let (send, receive) = mpsc::channel();
        *metrics.entered.lock().unwrap() = Some(send);
        let work = if write { grant(40) } else { read(2, 11, 1) };
        std::thread::scope(|scope| {
            let worker = scope.spawn(|| {
                let start = Instant::now();
                let reply = app.handle(&channel, work, NOW).unwrap();
                (reply, start.elapsed())
            });
            receive.recv_timeout(Duration::from_secs(5)).unwrap();
            let started = Instant::now();
            // Same .handle path used for an initial subscription or ordinary read.
            assert!(app.handle(&channel, read(3, 11, 1), NOW).is_err());
            assert!(started.elapsed() >= REQUEST_WAIT);
            let started = Instant::now();
            assert!(app.poll(&channel, read(4, 11, 1), NOW).unwrap().is_none());
            assert!(started.elapsed() >= REQUEST_WAIT);
            let (reply, elapsed) = worker.join().unwrap();
            success(&reply, if write { 1 } else { 3 });
            eprintln!(
                "contention-runtime workload={} witness_latency_ms=400 work_ms={} counters={:?} initial_read=busy periodic_poll=skipped",
                if write { "grant" } else { "read" },
                elapsed.as_millis(),
                metrics.counts()
            );
            metrics.witness_delay_ms.store(0, Ordering::SeqCst);
            success(&app.handle(&channel, read(5, 11, 1), NOW).unwrap(), 3);
            assert!(app.health() == Health::Ready);
            if write {
                let accepted = app.active.lock().unwrap().store.head();
                assert_ne!(accepted, head);
                // Original lookup is safe; no resend, duplicate acceptance or new attempt.
                let found = app
                    .handle(
                        &channel,
                        request(
                            6,
                            Command::Operation(RequestId::new([40; 32]).unwrap()),
                            11,
                            1,
                        ),
                        NOW,
                    )
                    .unwrap();
                assert_eq!(found.as_bytes(), reply.as_bytes());
                assert_eq!(app.active.lock().unwrap().store.head(), accepted);
                success(&app.handle(&channel, read(7, 31, 1), NOW).unwrap(), 3);
                success(
                    &app.handle(&channel, request(8, Command::Revoke, 11, 1), NOW)
                        .unwrap(),
                    1,
                );
                let revoked = app.handle(&channel, read(9, 31, 1), NOW).unwrap();
                assert_eq!(
                    revoked.as_bytes(),
                    cinder_api::Error::Unauthorized
                        .encode_private()
                        .unwrap()
                        .as_bytes()
                );
                success(&app.handle(&channel, read(10, 11, 2), NOW).unwrap(), 3);
            } else {
                assert_eq!(app.active.lock().unwrap().store.head(), head);
            }
            assert!(
                app.active
                    .lock()
                    .unwrap()
                    .store
                    .state()
                    .unwrap()
                    .attempts()
                    .is_empty()
            );
        });
    }
}

#[test]
fn independent_runtime_reads_do_not_wait_for_a_slow_writer_or_recheck_its_backend() {
    let metrics = Arc::new(Metrics::default());
    let app = independent(metrics.clone());
    let channel = session();
    metrics.reset();
    for _ in 0..3 {
        success(&app.handle(&channel, read(1, 11, 1), NOW).unwrap(), 3);
    }
    assert_eq!(metrics.private_reads.load(Ordering::SeqCst), 3);
    assert_eq!(metrics.counts(), [0; 4]);
    metrics.witness_delay_ms.store(400, Ordering::SeqCst);
    let (entered, observed) = mpsc::channel();
    *metrics.entered.lock().unwrap() = Some(entered);
    std::thread::scope(|scope| {
        let write = scope.spawn(|| app.handle(&channel, grant(40), NOW).unwrap());
        observed.recv_timeout(Duration::from_secs(5)).unwrap();
        let start = Instant::now();
        success(&app.handle(&channel, read(2, 11, 1), NOW).unwrap(), 3);
        assert!(start.elapsed() < REQUEST_WAIT);
        assert!(app.active.try_lock().is_err());
        success(
            &app.poll(&channel, read(3, 11, 1), NOW).unwrap().unwrap(),
            3,
        );
        success(&write.join().unwrap(), 1);
    });
    metrics.witness_delay_ms.store(0, Ordering::SeqCst);
    success(&app.handle(&channel, read(4, 31, 1), NOW).unwrap(), 3);
    assert!(app.health() == Health::Ready);
}

#[test]
fn slow_independent_reads_do_not_block_each_other_or_mutations_and_revoke_refuses_old_reply() {
    let metrics = Arc::new(Metrics::default());
    let app = independent(metrics.clone());
    let channel = session();
    success(&app.handle(&channel, grant(40), NOW).unwrap(), 1);
    metrics.private_delay_ms.store(400, Ordering::SeqCst);
    let (entered, observed) = mpsc::channel();
    *metrics.private_entered.lock().unwrap() = Some(entered);
    std::thread::scope(|scope| {
        let agent = scope.spawn(|| app.poll(&channel, read(1, 31, 1), NOW).unwrap());
        observed.recv_timeout(Duration::from_secs(5)).unwrap();
        // Mutation completes while the old epoch's read is waiting on its witness.
        success(
            &app.handle(&channel, request(41, Command::Revoke, 11, 1), NOW)
                .unwrap(),
            1,
        );
        assert!(agent.join().unwrap().is_none());
    });
    assert!(app.health() == Health::Ready);
    metrics.private_delay_ms.store(0, Ordering::SeqCst);
    let denied = app.handle(&channel, read(2, 31, 1), NOW).unwrap();
    assert_eq!(
        denied.as_bytes(),
        cinder_api::Error::Unauthorized
            .encode_private()
            .unwrap()
            .as_bytes()
    );
    // Concurrent witness I/O has no shared network lock, unlike the old runtime.
    metrics.private_delay_ms.store(400, Ordering::SeqCst);
    let (entered, observed) = mpsc::channel();
    *metrics.private_entered.lock().unwrap() = Some(entered);
    std::thread::scope(|scope| {
        let first = scope.spawn(|| app.handle(&channel, read(3, 11, 2), NOW).unwrap());
        observed.recv_timeout(Duration::from_secs(5)).unwrap();
        let start = Instant::now();
        success(&app.handle(&channel, read(4, 11, 2), NOW).unwrap(), 3);
        assert!(start.elapsed() < Duration::from_millis(750));
        success(&first.join().unwrap(), 3);
    });
}
struct AdjustableTime(AtomicU64);
impl Clock for AdjustableTime {
    fn now(&self) -> Result<u64, Error> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}
#[test]
fn independent_runtime_rechecks_expiry_after_io_and_fences_failed_witnesses() {
    let metrics = Arc::new(Metrics::default());
    let mut app = independent(metrics.clone());
    let time = Arc::new(AdjustableTime(AtomicU64::new(NOW)));
    app.clock = time.clone();
    let channel = session();
    metrics.private_delay_ms.store(400, Ordering::SeqCst);
    let (entered, observed) = mpsc::channel();
    *metrics.private_entered.lock().unwrap() = Some(entered);
    std::thread::scope(|scope| {
        let work = scope.spawn(|| app.handle(&channel, read(1, 11, 1), NOW).unwrap());
        observed.recv_timeout(Duration::from_secs(5)).unwrap();
        time.0.store(NOW + 60_000, Ordering::SeqCst);
        assert_eq!(
            work.join().unwrap().as_bytes(),
            cinder_api::Error::Unauthorized
                .encode_private()
                .unwrap()
                .as_bytes()
        );
    });
    assert!(app.health() == Health::Ready);
    time.0.store(NOW, Ordering::SeqCst);
    metrics.private_delay_ms.store(0, Ordering::SeqCst);
    metrics.private_failure.store(true, Ordering::SeqCst);
    assert!(app.handle(&channel, read(2, 11, 1), NOW).is_err());
    assert!(app.health() == Health::Fenced);
    assert!(app.handle(&channel, grant(42), NOW).is_err());
    assert!(app.tick().is_err());
}
struct FinalTime {
    calls: AtomicUsize,
    final_now: u64,
    delay: u64,
    panic: bool,
}
impl Clock for FinalTime {
    fn now(&self) -> Result<u64, Error> {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 2 {
            assert!(!self.panic, "synthetic final clock panic");
            std::thread::sleep(Duration::from_millis(self.delay));
            Ok(self.final_now)
        } else {
            Ok(NOW)
        }
    }
}
#[test]
fn final_read_gate_bounds_time_work_boot_expiry_and_panics_without_stale_output() {
    for case in 0..4 {
        let metrics = Arc::new(Metrics::default());
        let mut app = independent(metrics);
        app.clock = Arc::new(FinalTime {
            calls: AtomicUsize::new(0),
            final_now: match case {
                1 => NOW + 59_800,
                2 => NOW + 120_000,
                _ => NOW,
            },
            delay: if case == 0 { 400 } else { 0 },
            panic: case == 3,
        });
        let attempted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            app.poll(&session(), read(1, 11, 1), NOW)
        }));
        match case {
            0 => {
                assert!(attempted.unwrap().unwrap().is_none());
                assert!(app.health() == Health::Ready);
            }
            1 => {
                assert_eq!(
                    attempted.unwrap().unwrap().unwrap().as_bytes(),
                    cinder_api::Error::Unauthorized
                        .encode_private()
                        .unwrap()
                        .as_bytes()
                );
                assert!(app.health() == Health::Ready);
            }
            2 => {
                assert!(attempted.unwrap().is_err());
                assert!(app.health() == Health::Fenced);
            }
            _ => {
                assert!(attempted.is_err());
                assert!(app.health() == Health::Fenced);
            }
        }
    }
}
