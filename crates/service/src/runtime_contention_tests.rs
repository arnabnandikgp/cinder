//! Offline diagnosis only: real Runtime/API/AEAD/Replicated, synthetic I/O.
//! Synthetic I/O; no AWS credentials, native dispatch or funds.
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
    replica_delay_ms: AtomicU64,
    witness_delay_ms: AtomicU64,
    entered: Mutex<Option<mpsc::Sender<()>>>,
    anchor: Mutex<Option<Arc<Mutex<Anchor>>>>,
    private_delay_ms: AtomicU64,
    private_reads: AtomicUsize,
    private_entered: Mutex<Option<mpsc::Sender<()>>>,
    private_failure: AtomicBool,
}
impl Metrics {
    fn replica_delay(&self) {
        let delay = self.replica_delay_ms.load(Ordering::SeqCst);
        if delay != 0 {
            std::thread::sleep(Duration::from_millis(delay));
        }
    }
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
        self.metrics.replica_delay();
        self.frames.insert(frame.head.hash, frame.clone());
        Ok(())
    }
    fn get(&mut self, digest: [u8; 32]) -> Result<Frame, JournalError> {
        self.metrics.gets.fetch_add(1, Ordering::SeqCst);
        self.metrics.replica_delay();
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
            credits: 600,
            cleanup_reserve: 120,
            read_cost: cinder_pacifica::reads::MIN_READ_COST,
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
            poll_ms: None,
            next_poll: NOW,
            read_kind: 0,
            deposit_index: 0,
        }),
        io: Mutex::new(NativeIo {
            egress,
            chain: None,
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
    grant_key(n, 31)
}
fn grant_key(n: u8, agent_seed: u8) -> PrivateBytes {
    request(
        n,
        Command::Grant(Grant {
            key: public(agent_seed),
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
    for padding_count in [0, 8, 24, 32, 50, 64] {
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
fn replica_latency_can_outlive_reply_budget_while_original_grant_is_durable() {
    let metrics = Arc::new(Metrics::default());
    let app = application(metrics.clone());
    let channel = session();
    // Two initialization frames, then 49 real API grants: head 50, 51 frames.
    // Unique keys avoid a duplicate grant; all economic gates remain disabled.
    for n in 1..=49 {
        success(&app.handle(&channel, grant_key(n, n + 70), NOW).unwrap(), 1);
    }
    let before = app.active.lock().unwrap().store.head();
    let ledger = app
        .active
        .lock()
        .unwrap()
        .store
        .state()
        .unwrap()
        .ledger()
        .clone();
    assert_eq!(before.sequence, 50);
    metrics.reset();
    metrics.replica_delay_ms.store(100, Ordering::SeqCst);
    let started = Instant::now();
    let reply = app.handle(&channel, grant(200), NOW).unwrap();
    let elapsed = started.elapsed();
    // Synthetic per-call latency, not an AWS measurement or an SDK/carrier test.
    // The real Runtime write exceeds the SDK's current 15-second reply budget.
    assert!(elapsed >= Duration::from_secs(15));
    success(&reply, 1);
    let counts = metrics.counts();
    assert_eq!(counts[0], 3 * 51 + 2);
    assert_eq!(counts[1], 2);
    assert_eq!(counts[3], 1);
    eprintln!(
        "growth-runtime before_head=50 replica_latency_ms=100 elapsed_ms={} replica_gets={} replica_puts={} witness_reads={} witness_cas={}",
        elapsed.as_millis(),
        counts[0],
        counts[1],
        counts[2],
        counts[3]
    );
    // Model a caller that did not obtain the late reply. Never send grant 200 again.
    // Query its original ID, then explicitly replay the encrypted journal.
    metrics.replica_delay_ms.store(0, Ordering::SeqCst);
    let lookup = |id| {
        app.handle(
            &channel,
            request(
                id,
                Command::Operation(RequestId::new([200; 32]).unwrap()),
                11,
                1,
            ),
            NOW,
        )
        .unwrap()
    };
    assert_eq!(lookup(201).as_bytes(), reply.as_bytes());
    let mut active = app.active.lock().unwrap();
    let accepted = active.store.head();
    assert_eq!(accepted.sequence, 51);
    assert_eq!(active.store.transactions().count(), 51);
    active.store.reload().unwrap();
    assert_eq!(active.store.head(), accepted);
    assert_eq!(active.store.transactions().count(), 51);
    // A fresh API must derive the receipt from replay, not its previous cache.
    let restored_api = Service::new(
        Contract {
            owners: app.api.owner_bindings().to_vec(),
            maximum_auth_lifetime: 120_000,
            maximum_grant_lifetime: 120_000,
        },
        RiskAdmission { enabled: false },
    )
    .unwrap();
    let restored = restored_api
        .handle(
            &mut active.store,
            &channel,
            &request(
                203,
                Command::Operation(RequestId::new([200; 32]).unwrap()),
                11,
                1,
            ),
            NOW,
        )
        .unwrap()
        .encode()
        .unwrap();
    assert_eq!(restored.as_bytes(), reply.as_bytes());
    assert!(active.store.state().unwrap().attempts().is_empty());
    assert!(active.store.state().unwrap().orders().is_empty());
    assert_eq!(active.store.state().unwrap().ledger(), &ledger);
    drop(active);
    assert_eq!(lookup(202).as_bytes(), reply.as_bytes());
    assert_eq!(app.active.lock().unwrap().store.head(), accepted);
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
            assert_eq!(
                app.handle(&channel, read(3, 11, 1), NOW)
                    .unwrap()
                    .as_bytes(),
                cinder_api::Error::Unavailable
                    .encode_private()
                    .unwrap()
                    .as_bytes()
            );
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

#[test]
fn ordinary_busy_and_raced_reads_return_unavailable_without_retry_or_channel_failure() {
    let metrics = Arc::new(Metrics::default());
    let app = independent(metrics.clone());
    let channel = session();
    let candidates: Vec<_> = (0..cinder_journal::read::MAX_READS)
        .map(|n| {
            app.prepare_poll(&channel, read(n as u8 + 10, 11, 1), NOW)
                .unwrap()
                .unwrap()
        })
        .collect();
    let before = metrics.private_reads.load(Ordering::SeqCst);
    let unavailable = cinder_api::Error::Unavailable.encode_private().unwrap();
    assert_eq!(
        app.handle(&channel, read(20, 11, 1), NOW)
            .unwrap()
            .as_bytes(),
        unavailable.as_bytes()
    );
    let busy = app.prepare_handle(&channel, read(21, 11, 1), NOW).unwrap();
    assert_eq!(
        app.release_reply(&channel, busy)
            .unwrap()
            .unwrap()
            .as_bytes(),
        unavailable.as_bytes()
    );
    assert!(
        app.prepare_poll(&channel, read(22, 11, 1), NOW)
            .unwrap()
            .is_none()
    );
    assert_eq!(metrics.private_reads.load(Ordering::SeqCst), before);
    drop(candidates);
    let ordinary = app.prepare_handle(&channel, read(23, 11, 1), NOW).unwrap();
    let periodic = app
        .prepare_poll(&channel, read(24, 11, 1), NOW)
        .unwrap()
        .unwrap();
    padding(&mut app.active.lock().unwrap().store, 210);
    let before = metrics.private_reads.load(Ordering::SeqCst);
    assert_eq!(
        app.release_reply(&channel, ordinary)
            .unwrap()
            .unwrap()
            .as_bytes(),
        unavailable.as_bytes()
    );
    assert!(app.release_reply(&channel, periodic).unwrap().is_none());
    assert_eq!(metrics.private_reads.load(Ordering::SeqCst), before);
    success(&app.handle(&channel, read(25, 11, 1), NOW).unwrap(), 3);
    assert!(app.health() == Health::Ready);
}

#[test]
fn queued_reads_recheck_generation_expiry_boot_and_connection_at_delivery() {
    for case in 0..5 {
        let metrics = Arc::new(Metrics::default());
        let mut app = independent(metrics);
        let time = Arc::new(AdjustableTime(AtomicU64::new(NOW)));
        app.clock = time.clone();
        let channel = session();
        let prepared = app
            .prepare_poll(&channel, read(1, 11, 1), NOW)
            .unwrap()
            .unwrap();
        match case {
            0 => {
                // The candidate waited in a completion queue across an accepted
                // unrelated commit. It cannot emit even unchanged account data.
                padding(&mut app.active.lock().unwrap().store, 210);
                assert!(app.release_reply(&channel, prepared).unwrap().is_none());
            }
            1 => {
                time.0.store(NOW + 60_000, Ordering::SeqCst);
                assert_eq!(
                    app.release_reply(&channel, prepared)
                        .unwrap()
                        .unwrap()
                        .as_bytes(),
                    cinder_api::Error::Unauthorized
                        .encode_private()
                        .unwrap()
                        .as_bytes()
                );
            }
            2 => {
                app.fence();
                assert!(app.release_reply(&channel, prepared).is_err());
            }
            3 => {
                let other = Session::established(configuration().domain, [9; 32], NOW + 120_000);
                assert!(app.release_reply(&other, prepared).is_err());
            }
            _ => {
                let other = independent(Arc::new(Metrics::default()));
                assert!(other.release_reply(&channel, prepared).is_err());
                assert!(other.health() == Health::Ready);
            }
        }
        assert!(
            app.health()
                == if case == 2 {
                    Health::Fenced
                } else {
                    Health::Ready
                }
        );
        if case != 2 {
            // Consumption (including refusal) returns the retained ticket slot.
            assert!(app.reads.as_ref().unwrap().reader.capture().is_ok());
        }
    }
}

#[test]
fn queued_agent_reply_is_discarded_after_revoke_and_tickets_are_globally_bounded() {
    let metrics = Arc::new(Metrics::default());
    let app = independent(metrics.clone());
    let channel = session();
    success(&app.handle(&channel, grant(40), NOW).unwrap(), 1);
    let prepared = app
        .prepare_poll(&channel, read(1, 31, 1), NOW)
        .unwrap()
        .unwrap();
    success(
        &app.handle(&channel, request(41, Command::Revoke, 11, 1), NOW)
            .unwrap(),
        1,
    );
    assert!(app.release_reply(&channel, prepared).unwrap().is_none());
    assert_eq!(
        app.poll(&channel, read(2, 31, 1), NOW)
            .unwrap()
            .unwrap()
            .as_bytes(),
        cinder_api::Error::Unauthorized
            .encode_private()
            .unwrap()
            .as_bytes()
    );
    let candidates: Vec<_> = (0..cinder_journal::read::MAX_READS)
        .map(|n| {
            app.prepare_poll(&channel, read(n as u8 + 10, 11, 2), NOW)
                .unwrap()
                .unwrap()
        })
        .collect();
    let before = metrics.private_reads.load(Ordering::SeqCst);
    assert!(
        app.prepare_poll(&channel, read(20, 11, 2), NOW)
            .unwrap()
            .is_none()
    );
    assert_eq!(metrics.private_reads.load(Ordering::SeqCst), before);
    drop(candidates);
    assert!(
        app.prepare_poll(&channel, read(21, 11, 2), NOW)
            .unwrap()
            .is_some()
    );
    assert!(app.health() == Health::Ready);
}

#[test]
fn inactive_scheduler_checks_signed_lease_without_writer_or_cloud_io() {
    let metrics = Arc::new(Metrics::default());
    let mut app = independent(metrics.clone());
    let time = Arc::new(AdjustableTime(AtomicU64::new(NOW)));
    app.clock = time.clone();
    metrics.reset();
    let held = app.active.lock().unwrap();
    for _ in 0..3 {
        app.tick().unwrap();
    }
    assert_eq!(metrics.counts(), [0; 4]);
    assert_eq!(metrics.private_reads.load(Ordering::SeqCst), 0);
    assert!(app.health() == Health::Ready);
    time.0.store(NOW + 120_000, Ordering::SeqCst);
    assert!(app.tick().is_err());
    assert!(app.health() == Health::Fenced);
    drop(held);
    assert!(app.handle(&session(), grant(40), NOW).is_err());
}

#[test]
fn native_observation_io_releases_writer_and_rejoins_original_reserved_completion() {
    use cinder_pacifica::{execution::Reply, reads};
    struct Stalled {
        entered: mpsc::SyncSender<()>,
        resume: mpsc::Receiver<()>,
        calls: usize,
        status: u16,
    }
    impl reads::Transport for Stalled {
        fn get(&mut self, request: reads::Request) -> Reply {
            request.consume(NOW).unwrap();
            self.calls += 1;
            self.entered.send(()).unwrap();
            self.resume.recv_timeout(Duration::from_secs(3)).unwrap();
            if self.status == 0 {
                Reply::Unknown
            } else {
                Reply::Response {
                    status: self.status,
                    body: PrivateBytes::new(br#"{"error":"rate limited"}"#.to_vec()).unwrap(),
                    received_at: NOW,
                    retry_after_ms: Some(1000),
                }
            }
        }
    }
    for status in [0, 429] {
        let metrics = Arc::new(Metrics::default());
        let mut app = independent(metrics);
        app.gates.native_reads = true;
        let before = {
            let mut active = app.active.lock().unwrap();
            let Active {
                store,
                gateway,
                poll_ms,
                ..
            } = &mut *active;
            gateway
                .initialize_reads(store, CommitId::new([210; 32]).unwrap(), NOW)
                .unwrap();
            *poll_ms = Some(100);
            store.state().unwrap().ledger().clone()
        };
        let (entered, observed) = mpsc::sync_channel(1);
        let (release, resume) = mpsc::sync_channel(1);
        let mut transport = Stalled {
            entered,
            resume,
            calls: 0,
            status,
        };
        std::thread::scope(|scope| {
            let job = scope.spawn(|| {
                // Same single I/O-owner boundary as tick_observations; fake only
                // the network exchange, not the API, budget or journal phases.
                let _io = app.io.lock().unwrap();
                app.poll_venue(&mut transport).unwrap();
            });
            observed.recv_timeout(Duration::from_secs(3)).unwrap();
            assert!(app.active.try_lock().is_ok());
            let channel = session();
            success(&app.handle(&channel, grant(40), NOW).unwrap(), 1);
            success(&app.handle(&channel, read(1, 11, 1), NOW).unwrap(), 3);
            // A second tick skips the occupied I/O port; it must not duplicate
            // the reservation, exchange, controller selection or native credit.
            let head = app.active.lock().unwrap().store.head();
            app.tick().unwrap();
            assert_eq!(app.active.lock().unwrap().store.head(), head);
            release.send(()).unwrap();
            job.join().unwrap();
        });
        assert_eq!(transport.calls, 1);
        let mut active = app.active.lock().unwrap();
        assert_eq!(active.store.state().unwrap().ledger(), &before);
        assert_eq!(active.read_kind, 1);
        assert_eq!(active.next_poll, NOW + 100);
        if status == 429 {
            let Active { store, gateway, .. } = &mut *active;
            assert!(!gateway.read_available(store, NOW, true).unwrap());
        }
        assert!(app.health() == Health::Ready);
    }
}
