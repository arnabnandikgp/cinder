//! C4 local-only Runtime/AEAD/FileReplica/client qualification. Synthetic clock,
//! same-host witness and delays are NOT AWS authority, credentials or attestation.
use super::*;
use crate::fixture::{FixtureAttester, FixtureClock, LocalWitness, config};
use cinder_journal::{Error as JournalError, Frame, Head, replicated::*};
use cinder_pacifica::{execution::Origin, funding::Beneficiary};
use serde_json::{Value, json};
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, Read, Write},
    net::{SocketAddr, TcpListener},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU8, AtomicU64, AtomicUsize},
    time::Duration,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Options {
    history: u64,
    delay_ms: u64,
    broker_seed: [u8; 32],
    account: String,
}
impl Drop for Options {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.broker_seed.zeroize();
    }
}
#[derive(Default)]
struct Metrics {
    gets: AtomicUsize,
    puts: AtomicUsize,
    bytes: AtomicUsize,
    writer_reads: AtomicUsize,
    accepts: AtomicUsize,
    read_calls: AtomicUsize,
    live: AtomicUsize,
    peak: AtomicUsize,
    clocks: AtomicUsize,
    delay_ms: AtomicU64,
    writer_delay_ms: AtomicU64,
    post_cas_ms: AtomicU64,
    post_cas: AtomicBool,
    advance: AtomicU64,
    fault: AtomicU8,
}
impl Metrics {
    fn report(&self) -> Value {
        json!({"gets":self.gets.load(Ordering::SeqCst),"puts":self.puts.load(Ordering::SeqCst),
            "replica_bytes":self.bytes.load(Ordering::SeqCst),"writer_reads":self.writer_reads.load(Ordering::SeqCst),
            "accepts":self.accepts.load(Ordering::SeqCst),"read_calls":self.read_calls.load(Ordering::SeqCst),
            "live_read_io":self.live.load(Ordering::SeqCst),"peak_read_io":self.peak.load(Ordering::SeqCst),
            "clocks":self.clocks.load(Ordering::SeqCst),"post_cas":self.post_cas.load(Ordering::SeqCst)})
    }
}
struct CountedReplica {
    inner: FileReplica,
    metrics: Arc<Metrics>,
}
impl Replica for CountedReplica {
    fn identity(&self) -> [u8; 32] {
        self.inner.identity()
    }
    fn get(&mut self, hash: [u8; 32]) -> Result<Frame, JournalError> {
        self.metrics.gets.fetch_add(1, Ordering::SeqCst);
        let frame = self.inner.get(hash)?;
        self.metrics
            .bytes
            .fetch_add(frame.opaque.as_bytes().len(), Ordering::SeqCst);
        Ok(frame)
    }
    fn put(&mut self, frame: &Frame) -> Result<(), JournalError> {
        self.metrics.puts.fetch_add(1, Ordering::SeqCst);
        self.metrics
            .bytes
            .fetch_add(frame.opaque.as_bytes().len(), Ordering::SeqCst);
        self.inner.put(frame)
    }
}
struct CountedWitness {
    inner: LocalWitness,
    stream: Stream,
    metrics: Arc<Metrics>,
}
impl Witness for CountedWitness {
    fn read(&mut self, stream: Stream) -> Result<Anchor, JournalError> {
        if stream != self.stream {
            return Err(JournalError::Conflict);
        }
        self.metrics.writer_reads.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(
            self.metrics.writer_delay_ms.load(Ordering::SeqCst),
        ));
        self.inner.read(stream)
    }
    fn accept(&mut self, stream: Stream, expected: Anchor, next: Head) -> Result<(), JournalError> {
        if stream != self.stream {
            return Err(JournalError::Conflict);
        }
        self.inner.accept(stream, expected, next)?;
        self.metrics.accepts.fetch_add(1, Ordering::SeqCst);
        let pause = self.metrics.post_cas_ms.swap(0, Ordering::SeqCst);
        if pause != 0 {
            self.metrics.post_cas.store(true, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(pause));
            self.metrics.post_cas.store(false, Ordering::SeqCst);
        }
        Ok(())
    }
}
struct Independent {
    file: PathBuf,
    stream: Stream,
    metrics: Arc<Metrics>,
}
struct Live<'a>(&'a AtomicUsize);
impl Drop for Live<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
impl ReadWitness for Independent {
    fn read(&self, stream: Stream) -> Result<Anchor, JournalError> {
        if stream != self.stream {
            return Err(JournalError::Conflict);
        }
        self.metrics.read_calls.fetch_add(1, Ordering::SeqCst);
        let live = self.metrics.live.fetch_add(1, Ordering::SeqCst) + 1;
        let _live = Live(&self.metrics.live);
        self.metrics.peak.fetch_max(live, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(
            self.metrics.delay_ms.load(Ordering::SeqCst),
        ));
        let mut anchor = LocalWitness(self.file.clone()).read(stream)?;
        match self.metrics.fault.load(Ordering::SeqCst) {
            0 => {}
            1 | 4 => return Err(JournalError::Storage),
            2 => anchor.epoch += 1,
            3 => anchor.head.as_mut().ok_or(JournalError::Storage)?.hash[0] ^= 1,
            _ => return Err(JournalError::Storage),
        }
        Ok(anchor)
    }
}
struct Time(Arc<Metrics>);
impl Clock for Time {
    fn now(&self) -> Result<u64, Error> {
        self.0.clocks.fetch_add(1, Ordering::SeqCst);
        FixtureClock
            .now()?
            .checked_add(self.0.advance.load(Ordering::SeqCst))
            .ok_or(Error)
    }
}
type Storage = Replicated<CountedReplica, CountedReplica, CountedWitness>;
type Application = Runtime<Storage, RecordCipher>;
fn padding(store: &mut Journal<Storage, RecordCipher>, at: u64) -> Result<(), Error> {
    let result = store
        .commit(Transaction {
            id: commit_id()?,
            expected: store.head(),
            at,
            evidence: vec![PrivateBytes::new(vec![91; 16 * 1024]).map_err(|_| Error)?],
            inputs: vec![],
            controls: vec![],
            order_observations: vec![],
            funds_observations: vec![],
        })
        .map_err(|_| Error)?;
    if result.duplicate || result.receipt.controls.is_some() {
        return Err(Error);
    }
    Ok(())
}
fn application(
    root: &Path,
    key: Zeroizing<[u8; 32]>,
    wallet: [u8; 32],
    options: &Options,
    metrics: Arc<Metrics>,
    trust: &[u8],
) -> Result<Application, Error> {
    if !matches!(options.history, 1 | 8 | 32 | 64) || !matches!(options.delay_ms, 0 | 50 | 400) {
        return Err(Error);
    }
    let fresh = !root.join("accepted").exists();
    if fresh {
        if root.read_dir()?.next().is_some() {
            return Err(Error);
        }
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join("accepted"))?;
        f.write_all(&[0])?;
        f.sync_all()?;
        File::open(root)?.sync_all()?;
    }
    let stream = Stream {
        domain: config().domain,
        id: [42; 32],
    };
    let replica = |name| -> Result<CountedReplica, Error> {
        Ok(CountedReplica {
            inner: if fresh {
                FileReplica::create(&root.join(name))
            } else {
                FileReplica::open(&root.join(name))
            }
            .map_err(|_| Error)?,
            metrics: metrics.clone(),
        })
    };
    let backend = Replicated::new(
        stream,
        1,
        replica("first")?,
        replica("second")?,
        CountedWitness {
            inner: LocalWitness(root.join("accepted")),
            stream,
            metrics: metrics.clone(),
        },
    )
    .map_err(|_| Error)?;
    let cipher = RecordCipher::new(key, 1, stream.id).map_err(|_| Error)?;
    let mut store = if fresh {
        Journal::create(backend, cipher, config())
    } else {
        Journal::open(backend, cipher, config())
    }
    .map_err(|_| Error)?;
    let api = Service::new(
        Contract {
            owners: vec![OwnerBinding {
                account: config().customers[0],
                wallet,
                tokens: [81; 32],
            }],
            maximum_auth_lifetime: 120_000,
            maximum_grant_lifetime: 3_600_000,
        },
        RiskAdmission { enabled: false },
    )
    .map_err(|_| Error)?;
    let clock: Arc<dyn Clock> = Arc::new(Time(metrics.clone()));
    let now = clock.now()?;
    api.initialize(&mut store, now).map_err(|_| Error)?;
    if fresh {
        for _ in 1..options.history {
            padding(&mut store, now)?;
        }
    }
    if (fresh && store.head().sequence != options.history) || store.head().sequence > 128 {
        return Err(Error);
    }
    let profile = Profile {
        config: config(),
        source: config().sources[0].scope,
        account: options.account.clone(),
        environment: Origin::Testnet.url().into(),
        revision: 1,
        evidence: "C4 OFFLINE Runtime; no native qualification or capital".into(),
        precision: Level::Qualified,
        fills: Level::Qualified,
        markets: vec![Mapping {
            symbol: "BTC".into(),
            market: 0,
            size: cinder_pacifica::profile::Grid { places: 0, step: 1 },
            price: cinder_pacifica::profile::Grid { places: 0, step: 1 },
        }],
        quote_places: 0,
        perp_tag: 1,
    };
    let broker = openssl::pkey::PKey::private_key_from_raw_bytes(
        &options.broker_seed,
        openssl::pkey::Id::ED25519,
    )?;
    let broker: [u8; 32] = broker.raw_public_key()?.try_into().map_err(|_| Error)?;
    let funding = Controller::new(
        profile.clone(),
        Route {
            domain: config().domain.deployment.bytes(),
            pool: [18; 32],
            funds: [19; 32],
            beneficiaries: vec![Beneficiary {
                account: config().customers[0].bytes(),
                wallet,
                tokens: [81; 32],
            }],
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
            withdrawal_cost: 120,
            maximum_movement: 1000,
            maximum_fee: 5,
            setup_max_age: 10_000,
        },
        Zeroizing::new(options.broker_seed),
    )
    .map_err(|_| Error)?;
    let gateway = Gateway::new(
        profile,
        Policy {
            revision: 1,
            evidence: "C4 OFFLINE ONLY".into(),
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
    .map_err(|_| Error)?;
    let reader = store
        .attach_reader(ReadBinding {
            stream,
            epoch: 1,
            api: api
                .release_commitment(store.configuration())
                .map_err(|_| Error)?,
        })
        .map_err(|_| Error)?;
    let egress = Egress::new(
        Origin::Testnet,
        Target::new(3, 9007)?,
        Trust::from_der(trust, openssl::sha::sha256(trust))?,
        clock.clone(),
    )?;
    metrics.delay_ms.store(options.delay_ms, Ordering::SeqCst);
    metrics
        .writer_delay_ms
        .store(options.delay_ms, Ordering::SeqCst);
    Ok(Runtime {
        active: Mutex::new(Active {
            store,
            gateway,
            funding,
            poll_ms: None,
            next_poll: now,
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
        deadline: now + 120_000,
        gates: Gates {
            trading: false,
            funding: false,
            native_reads: false,
            maximum_boot_ms: 120_000,
        },
        reads: Some(ReadRuntime {
            reader,
            witness: Arc::new(Independent {
                file: root.join("accepted"),
                stream,
                metrics,
            }),
        }),
    })
}
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case", deny_unknown_fields)]
enum Control {
    Metrics {},
    Tick {},
    Writes { count: u64, interval_ms: u64 },
    Delay { milliseconds: u64 },
    WriterDelay { milliseconds: u64 },
    PostCas { milliseconds: u64 },
    Fault { kind: String },
}
fn control(app: &Application, metrics: &Metrics, command: Control) -> Result<Value, Error> {
    match command {
        Control::Metrics {} => {}
        Control::Tick {} => app.tick()?,
        Control::Writes { count, interval_ms } => {
            if count == 0 || count > 16 || interval_ms > 500 {
                return Err(Error);
            }
            for _ in 0..count {
                let mut active = app.active.lock().map_err(|_| Error)?;
                let at = app.active_time()?;
                padding(&mut active.store, at)?;
                drop(active);
                std::thread::sleep(Duration::from_millis(interval_ms));
            }
        }
        Control::Delay { milliseconds } | Control::WriterDelay { milliseconds }
            if milliseconds > 400 =>
        {
            return Err(Error);
        }
        Control::Delay { milliseconds } => metrics.delay_ms.store(milliseconds, Ordering::SeqCst),
        Control::WriterDelay { milliseconds } => metrics
            .writer_delay_ms
            .store(milliseconds, Ordering::SeqCst),
        Control::PostCas { milliseconds } => {
            if milliseconds == 0 || milliseconds > 2000 {
                return Err(Error);
            }
            metrics.post_cas_ms.store(milliseconds, Ordering::SeqCst);
        }
        Control::Fault { kind } => match kind.as_str() {
            "witness" => metrics.fault.store(1, Ordering::SeqCst),
            "epoch" => metrics.fault.store(2, Ordering::SeqCst),
            "head" => metrics.fault.store(3, Ordering::SeqCst),
            "credential" => metrics.fault.store(4, Ordering::SeqCst),
            "expire" => metrics.advance.store(120_001, Ordering::SeqCst),
            "fence" => app.fence(),
            _ => return Err(Error),
        },
    }
    Ok(metrics.report())
}
/// Run only explicit loopback fixture mode. Configuration/controls arrive on
/// private harness stdin; stdout contains a disposable root and aggregate counts.
/// No HTTP control endpoint, live I/O, wallet loading or seeded customer credit.
pub fn run(
    listen: SocketAddr,
    root: &Path,
    key: Zeroizing<[u8; 32]>,
    wallet: [u8; 32],
) -> Result<(), Error> {
    if !listen.ip().is_loopback() {
        return Err(Error);
    }
    let mut size = [0; 4];
    std::io::stdin().read_exact(&mut size)?;
    let n = u32::from_be_bytes(size) as usize;
    if n == 0 || n > 4096 {
        return Err(Error);
    }
    let mut input = Zeroizing::new(vec![0; n]);
    std::io::stdin().read_exact(&mut input)?;
    let options: Options = serde_json::from_slice(&input).map_err(|_| Error)?;
    let attester = Arc::new(FixtureAttester::new()?);
    let trust = attester.root().to_der()?;
    let metrics = Arc::new(Metrics::default());
    let app = Arc::new(application(
        root,
        key,
        wallet,
        &options,
        metrics.clone(),
        &trust,
    )?);
    let listener = TcpListener::bind(listen)?;
    println!(
        "{} {}",
        listener.local_addr()?,
        trust.iter().map(|b| format!("{b:02x}")).collect::<String>()
    );
    std::io::stdout().flush()?;
    let stop = app.stop.clone();
    std::thread::scope(|scope| {
        let port = app.clone();
        let flag = stop.clone();
        scope.spawn(move || {
            let mut input = std::io::BufReader::new(std::io::stdin());
            loop {
                let mut line = Zeroizing::new(String::new());
                let Ok(n) = input.by_ref().take(4097).read_line(&mut line) else {
                    break;
                };
                if n == 0 || n > 4096 {
                    break;
                }
                let response = serde_json::from_str(&line)
                    .ok()
                    .and_then(|c| control(&port, &metrics, c).ok())
                    .unwrap_or_else(|| json!({"error":"runtime-fixture-control-rejected"}));
                if writeln!(std::io::stdout(), "{response}")
                    .and_then(|_| std::io::stdout().flush())
                    .is_err()
                {
                    break;
                }
            }
            port.fence();
            flag.store(true, Ordering::SeqCst);
        });
        crate::web::Server::new(
            crate::fixture::policy(),
            Arc::new(FixtureClock),
            attester,
            app.clock.clone(),
            app,
        )?
        .run(listener, stop)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let id = commit_id()
                .unwrap()
                .bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            let root = std::env::temp_dir().join(format!("cinder-runtime-qualification-{id}"));
            std::fs::create_dir(&root).unwrap();
            Self(root)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn wallet(seed: u8) -> [u8; 32] {
        openssl::pkey::PKey::private_key_from_raw_bytes(&[seed; 32], openssl::pkey::Id::ED25519)
            .unwrap()
            .raw_public_key()
            .unwrap()
            .try_into()
            .unwrap()
    }
    fn options(history: u64) -> Options {
        Options {
            history,
            delay_ms: 0,
            broker_seed: [9; 32],
            account: "J2xccRtuG43drESLYznHhLhQkLTdfepcKYbiQ9BsJVaf".into(),
        }
    }
    #[test]
    fn joined_runtime_fixture_has_exact_history_and_reopens_without_accepting_or_crediting() {
        let trust = FixtureAttester::new().unwrap().root().to_der().unwrap();
        for history in [1, 8, 32, 64] {
            let root = Temp::new();
            let metrics = Arc::new(Metrics::default());
            let app = application(
                &root.0,
                Zeroizing::new([55; 32]),
                wallet(11),
                &options(history),
                metrics,
                &trust,
            )
            .unwrap();
            let active = app.active.lock().unwrap();
            let head = active.store.head();
            assert_eq!(head.sequence, history);
            assert_eq!(
                active
                    .store
                    .state()
                    .unwrap()
                    .ledger()
                    .book(cinder_kernel::ledger::Owner::Customer(
                        config().customers[0]
                    ))
                    .unwrap()
                    .cash()
                    .atoms(),
                0
            );
            drop(active);
            drop(app);
            let metrics = Arc::new(Metrics::default());
            let reopened = application(
                &root.0,
                Zeroizing::new([55; 32]),
                wallet(11),
                &options(history),
                metrics.clone(),
                &trust,
            )
            .unwrap();
            assert_eq!(reopened.active.lock().unwrap().store.head(), head);
            assert_eq!(metrics.accepts.load(Ordering::SeqCst), 0);
            reopened.tick().unwrap();
            assert_eq!(metrics.clocks.load(Ordering::SeqCst), 2);
            drop(reopened);
            assert!(
                application(
                    &root.0,
                    Zeroizing::new([55; 32]),
                    wallet(12),
                    &options(history),
                    Arc::new(Metrics::default()),
                    &trust
                )
                .is_err()
            );
        }
    }
    #[test]
    fn joined_runtime_fixture_rejects_dirty_initialization_and_out_of_envelope_controls() {
        let root = Temp::new();
        let trust = FixtureAttester::new().unwrap().root().to_der().unwrap();
        let metrics = Arc::new(Metrics::default());
        assert!(
            application(
                &root.0,
                Zeroizing::new([55; 32]),
                wallet(11),
                &options(2),
                metrics.clone(),
                &trust
            )
            .is_err()
        );
        assert!(root.0.read_dir().unwrap().next().is_none());
        std::fs::create_dir(root.0.join("unaccepted")).unwrap();
        assert!(
            application(
                &root.0,
                Zeroizing::new([55; 32]),
                wallet(11),
                &options(1),
                metrics.clone(),
                &trust
            )
            .is_err()
        );
        assert_eq!(metrics.accepts.load(Ordering::SeqCst), 0);
        assert!(
            serde_json::from_str::<Control>(r#"{"op":"metrics","seed":"unexpected"}"#).is_err()
        );
        assert!(serde_json::from_str::<Options>(r#"{"history":1,"delay_ms":0}"#).is_err());
    }
}
