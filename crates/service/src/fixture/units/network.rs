//! Synthetic external ports, real shared Runtime/Worker/Noise/SDK. No cloud,
//! venue, RPC, wallet discovery or shipping financial initialization.
use super::*;
use crate::units::worker::{Policy, Worker};
use std::{
    io::BufRead,
    net::{SocketAddr, TcpListener},
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

#[derive(Default)]
struct Calls {
    pacifica: Vec<String>,
    bulk: Vec<String>,
    reads: usize,
}
struct Port {
    calls: Arc<Mutex<Calls>>,
    delay: Arc<AtomicU64>,
}
impl cinder_pacifica::reads::Transport for Port {
    fn get(&mut self, _: cinder_pacifica::reads::Request) -> pacifica::Reply {
        self.calls.lock().unwrap().reads += 1;
        pacifica::Reply::Unknown
    }
}
impl pacifica::Transport for Port {
    fn post(&mut self, request: pacifica::Outbound) -> pacifica::Reply {
        self.calls
            .lock()
            .unwrap()
            .pacifica
            .push(String::from_utf8(request.body().to_vec()).unwrap());
        std::thread::sleep(Duration::from_millis(self.delay.load(Ordering::SeqCst)));
        pacifica::Reply::Unknown
    }
}
impl cinder_bulk::reads::Transport for Port {
    fn query(&mut self, _: cinder_bulk::reads::Request) -> bulk::Reply {
        self.calls.lock().unwrap().reads += 1;
        bulk::Reply::Unknown
    }
}
impl bulk::Transport for Port {
    fn post(&mut self, request: bulk::Outbound) -> bulk::Reply {
        self.calls
            .lock()
            .unwrap()
            .bulk
            .push(String::from_utf8(request.body().to_vec()).unwrap());
        std::thread::sleep(Duration::from_millis(self.delay.load(Ordering::SeqCst)));
        bulk::Reply::Unknown
    }
}
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case", deny_unknown_fields)]
enum Control {
    Tick,
    Metrics,
    Delay { milliseconds: u64 },
    Observe { unit: u8, body: String },
    Fence,
}
/// Local-only fixed disposable agent identities and explicitly seeded money.
/// Unlike shipping managed startup, this deliberately supplies synthetic native
/// credit/risk facts. Do not use this constructor to qualify live collateral.
pub fn run(
    listen: SocketAddr,
    path: &Path,
    key: Zeroizing<[u8; 32]>,
    wallet: [u8; 32],
) -> Result<(), Error> {
    let mut stdin = BufReader::new(std::io::stdin());
    let mut size = [0; 4];
    stdin.read_exact(&mut size)?;
    let n = u32::from_be_bytes(size) as usize;
    if n == 0 || n > 8192 {
        return Err(Error);
    }
    let mut options = Zeroizing::new(vec![0; n]);
    stdin.read_exact(&mut options)?;
    let _: Json = serde_json::from_slice(&options).map_err(|_| Error)?;
    let fresh = !path.join("accepted").exists();
    if fresh {
        if path.read_dir()?.next().is_some() {
            return Err(Error);
        }
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path.join("accepted"))?;
        f.write_all(&[0])?;
        f.sync_all()?;
        FileReplica::create(&path.join("first")).map_err(|_| Error)?;
        FileReplica::create(&path.join("second")).map_err(|_| Error)?;
    }
    let (configuration, _) = inventory()?;
    let backend = Replicated::new(
        Stream {
            domain: config().domain,
            id: [42; 32],
        },
        1,
        FileReplica::open(&path.join("first")).map_err(|_| Error)?,
        FileReplica::open(&path.join("second")).map_err(|_| Error)?,
        LocalWitness(path.join("accepted")),
    )
    .map_err(|_| Error)?;
    let cipher = RecordCipher::new(key, 1, [42; 32]).map_err(|_| Error)?;
    let mut j = if fresh {
        UnitStore::create(backend, cipher, configuration.clone())
    } else {
        UnitStore::open(backend, cipher, configuration.clone())
    }
    .map_err(|_| Error)?;
    let account =
        cinder_journal::onboarding::account(config().domain, wallet).map_err(|_| Error)?;
    if fresh {
        j.commit(units::Transaction {
            id: CommitId::new([200; 32]).map_err(|_| Error)?,
            expected: j.head(),
            at: FixtureClock.now()?,
            change: units::Change::Enroll {
                wallet,
                recipients: configuration
                    .units()
                    .iter()
                    .map(|d| units::Recipient {
                        unit: d.id,
                        tokens: [71; 32],
                    })
                    .collect(),
            },
        })
        .map_err(|_| Error)?;
        for d in configuration.units() {
            let mut selected = j.select(d.id).map_err(|_| Error)?;
            seed_for(&mut selected, FixtureClock.now()?, false, true, account)?;
            super::super::commit(
                &mut selected,
                FixtureClock.now()?,
                vec![],
                vec![cinder_journal::model::Control::Funds(
                    cinder_journal::funds::Action::NativeCreditReady(true),
                )],
            )?;
        }
    }
    let api = api::service::Service::new(
        api::Authorizer::new(120000, 3_600_000).map_err(|_| Error)?,
        Holds(true),
    );
    let binding = Binding {
        stream: Stream {
            domain: config().domain,
            id: [42; 32],
        },
        epoch: 1,
        api: api.commitment(&configuration).map_err(|_| Error)?,
    };
    let app = Arc::new(crate::units::Runtime::open(
        j,
        api,
        binding,
        Arc::new(Fresh(path.join("accepted"))),
        Arc::new(FixtureClock),
        FixtureClock.now()?.checked_add(600000).ok_or(Error)?,
    )?);
    let calls = Arc::new(Mutex::new(Calls::default()));
    let delay = Arc::new(AtomicU64::new(0));
    let port = || Port {
        calls: calls.clone(),
        delay: delay.clone(),
    };
    let mut worker = Worker::new(
        Policy {
            poll_ms: 1000,
            pacifica_reads: 1,
            bulk_reads: 1,
            pacifica_dispatches: 1,
            bulk_dispatches: 1,
        },
        uid(50)?,
        uid(40)?,
        pacifica_gateway(&configuration)?,
        bulk_gateway(&configuration)?,
        port(),
        port(),
    )?;
    worker.initialize(&app)?;
    let attester = Arc::new(FixtureAttester::new()?);
    let root = attester.root().to_der()?;
    let listener = TcpListener::bind(listen)?;
    println!(
        "{} {} {}",
        listener.local_addr()?,
        hex(&root),
        hex(&account.bytes())
    );
    std::io::stdout().flush()?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let control_app = app.clone();
    let thread = std::thread::spawn(move || {
        loop {
            let mut line = String::new();
            let Ok(n) = stdin.by_ref().take(131073).read_line(&mut line) else {
                break;
            };
            if n == 0 || line.len() > 131072 {
                break;
            }
            let result = (|| -> Result<Json, Error> {
                match serde_json::from_str(&line).map_err(|_| Error)? {
                    Control::Tick => {
                        worker.tick(&control_app)?;
                    }
                    Control::Metrics => (),
                    Control::Delay { milliseconds } => {
                        if milliseconds > 1000 {
                            return Err(Error);
                        }
                        delay.store(milliseconds, Ordering::SeqCst);
                    }
                    Control::Fence => control_app.fence(),
                    Control::Observe { unit, body } => {
                        let mut bytes = [0; 32];
                        openssl::rand::rand_bytes(&mut bytes)?;
                        let commit = CommitId::new(bytes).map_err(|_| Error)?;
                        if unit == 40 {
                            control_app.observe_bulk(
                                uid(40)?,
                                &bulk_gateway(&configuration)?,
                                cinder_bulk::observation::Message {
                                    kind: cinder_bulk::observation::Kind::TradeStream,
                                    body: PrivateBytes::new(body.into_bytes())
                                        .map_err(|_| Error)?,
                                    received_at: FixtureClock.now()?,
                                },
                                commit,
                            )?;
                        } else if unit == 50 {
                            control_app.observe_pacifica(
                                uid(50)?,
                                &pacifica_profile(&configuration),
                                cinder_pacifica::observation::Message {
                                    kind: cinder_pacifica::observation::Kind::TradeStream,
                                    account: chain::address(
                                        configuration.units()[1].native_account,
                                    ),
                                    cursor: None,
                                    received_at: FixtureClock.now()?,
                                    body,
                                },
                                commit,
                            )?;
                        } else {
                            return Err(Error);
                        }
                    }
                }
                let calls = calls.lock().map_err(|_| Error)?;
                Ok(
                    json!({"pacifica":calls.pacifica.len(),"bulk":calls.bulk.len(),"reads":calls.reads,
                    "pacifica_wire":calls.pacifica.last(),"bulk_wire":calls.bulk.last()}),
                )
            })();
            if output(result.unwrap_or_else(|_| json!({"error":"unit-control-rejected"}))).is_err()
            {
                break;
            }
        }
        control_app.fence();
        flag.store(true, Ordering::SeqCst);
    });
    let result = crate::web::Server::new(
        policy(),
        Arc::new(FixtureClock),
        attester,
        Arc::new(FixtureClock),
        app,
    )?
    .run(listener, stop);
    thread.join().map_err(|_| Error)?;
    result
}
