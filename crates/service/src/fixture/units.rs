//! Offline private-pipe acceptance ONLY, behind local-fixture. Fixed public
//! disposable native seeds, synthetic collateral and same-host witness. Never
//! a listener, real enclave, authority qualification or native network client.
use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use cinder_api::{
    onboarding::Policy as Custody,
    units::{self as api, onboarding as bootstrap},
};
use cinder_bulk::{execution as bulk, financial as bf};
use cinder_journal::{read::Binding, units};
use cinder_pacifica::{execution as pacifica, funding::chain, profile as pp};
use serde::Deserialize;
use serde_json::{Value as Json, json};
use std::{io::BufReader, sync::Arc};

type UnitStore = units::Journal<Replicated<FileReplica, FileReplica, LocalWitness>, RecordCipher>;
type App =
    crate::units::Runtime<Replicated<FileReplica, FileReplica, LocalWitness>, RecordCipher, Holds>;
fn uid(n: u8) -> Result<UnitId, Error> {
    UnitId::new([n; 32]).map_err(|_| Error)
}
fn public(seed: u8) -> Result<[u8; 32], Error> {
    PKey::private_key_from_raw_bytes(&[seed; 32], openssl::pkey::Id::ED25519)?
        .raw_public_key()?
        .try_into()
        .map_err(|_| Error)
}
fn inventory() -> Result<(units::Configuration, Vec<(UnitId, Custody)>), Error> {
    let mut descriptors = vec![];
    let mut policies = vec![];
    for n in [40_u8, 50] {
        let mut ledger = config();
        ledger.customers.clear();
        ledger.venue = VenueId::new([n + 1; 32]).map_err(|_| Error)?;
        ledger.venue_account = VenueAccountId::new([n + 2; 32]).map_err(|_| Error)?;
        for s in &mut ledger.sources {
            s.scope.venue = ledger.venue;
            s.scope.account = ledger.venue_account;
        }
        let program = [20; 32];
        let pool = [n; 32];
        let mint = [23; 32];
        let domain = ledger.domain.deployment.bytes();
        let config =
            chain::pda(program, &[b"cinder_vault", &domain, &pool, &mint]).map_err(|_| Error)?;
        let route = chain::pda(program, &[b"venue_route", &config]).map_err(|_| Error)?;
        let native_account = public(if n == 40 { 31 } else { 32 })?;
        descriptors.push(units::Descriptor {
            id: uid(n)?,
            environment: if n == 40 {
                cinder_bulk::Network::Testnet.commitment()
            } else {
                pacifica::Origin::Testnet.commitment()
            },
            native_account,
            funding_wallet: native_account,
            custody_config: config,
            payout_vault: chain::pda(program, &[b"tokens", &config]).map_err(|_| Error)?,
            key_policy: [n + 7; 32],
            ledger,
        });
        policies.push((
            uid(n)?,
            Custody {
                maximum_accounts: 1,
                custody_domain: domain,
                pool,
                venue: [n + 1; 32],
                program,
                config,
                route,
                mint,
                decimals: 0,
                epoch: 1,
                maximum_deposit: 1000,
            },
        ));
    }
    Ok((
        units::Configuration::new(config().domain, descriptors).map_err(|_| Error)?,
        policies,
    ))
}
fn bulk_gateway(c: &units::Configuration) -> Result<bulk::Gateway, Error> {
    let d = &c.units()[0];
    let profile = bf::Profile::new(
        cinder_bulk::Scope::new(
            d.ledger.sources[0].scope,
            cinder_bulk::Network::Testnet,
            d.native_account,
        )
        .map_err(|_| Error)?,
        d.ledger.quote,
        0,
        bf::Target::Master,
        vec![bf::Mapping::new(
            cinder_bulk::precision::Market::new("BTC-USD", "1", "1", 0).map_err(|_| Error)?,
            d.ledger.markets[0].unit(),
        )],
    )
    .map_err(|_| Error)?;
    bulk::Gateway::new(
        d.ledger.clone(),
        profile,
        bulk::Policy {
            unit: d.id,
            revision: 1,
            evidence: [88; 32],
            qualification: bulk::Level::Qualified,
            expiry_ms: 30000,
            credits: 100,
            cleanup_reserve: 10,
        },
        Zeroizing::new([11; 32]),
        1,
    )
    .map_err(|_| Error)
}
fn pacifica_profile(c: &units::Configuration) -> pp::Profile {
    let d = &c.units()[1];
    pp::Profile {
        config: d.ledger.clone(),
        source: d.ledger.sources[0].scope,
        account: chain::address(d.native_account),
        environment: pacifica::Origin::Testnet.url().into(),
        revision: 1,
        evidence: "synthetic private-pipe two-unit acceptance".into(),
        precision: pp::Level::Qualified,
        fills: pp::Level::Qualified,
        markets: vec![pp::Mapping {
            symbol: "BTC".into(),
            market: 0,
            size: pp::Grid { places: 0, step: 1 },
            price: pp::Grid { places: 0, step: 1 },
        }],
        quote_places: 0,
        perp_tag: 0,
    }
}
fn pacifica_gateway(c: &units::Configuration) -> Result<pacifica::Gateway, Error> {
    pacifica::Gateway::new(
        pacifica_profile(c),
        pacifica::Policy {
            revision: 1,
            evidence: "synthetic pipe qualification only".into(),
            execution: pp::Level::Qualified,
            origin: pacifica::Origin::Testnet,
            expiry_ms: 30000,
            credits: 1000,
            cleanup_reserve: 100,
            read_cost: 10,
        },
        Zeroizing::new([12; 32]),
        1,
    )
    .map_err(|_| Error)
}
struct Fresh(PathBuf);
impl cinder_journal::read::Witness for Fresh {
    fn read(&self, s: Stream) -> Result<Anchor, cinder_journal::Error> {
        LocalWitness(self.0.clone()).read(s)
    }
}
fn start(
    j: UnitStore,
    api: api::service::Service<Holds>,
    path: &Path,
    deadline: u64,
) -> Result<App, Error> {
    let binding = Binding {
        stream: Stream {
            domain: config().domain,
            id: [42; 32],
        },
        epoch: 1,
        api: api.commitment(j.configuration()).map_err(|_| Error)?,
    };
    crate::units::Runtime::open_fixture(
        j,
        api,
        binding,
        Arc::new(Fresh(path.join("accepted"))),
        Arc::new(FixtureClock),
        deadline,
    )
}
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case", deny_unknown_fields)]
enum Command {
    Request { wire: String },
    Expose { unit: u8, request: u8, nonce: u64 },
    Observe { unit: u8, body: String },
    Fence,
}
fn input(reader: &mut impl Read) -> Result<Option<Command>, Error> {
    let mut size = [0; 4];
    match reader.read(&mut size[..1])? {
        0 => return Ok(None),
        1 => (),
        _ => return Err(Error),
    };
    reader.read_exact(&mut size[1..])?;
    let n = u32::from_be_bytes(size) as usize;
    if n == 0 || n > 1_048_576 {
        return Err(Error);
    }
    let mut body = Zeroizing::new(vec![0; n]);
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body).map(Some).map_err(|_| Error)
}
fn output(value: Json) -> Result<(), Error> {
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, &value).map_err(|_| Error)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}
/// Deliberate private harness mode, never reads a wallet/cloud config or socket.
/// First 32 stdin bytes are a disposable storage key; subsequent frames bounded.
pub fn run(path: &Path, wallet: [u8; 32]) -> Result<(), Error> {
    let mut reader = BufReader::new(std::io::stdin());
    let mut seed = Zeroizing::new([0; 32]);
    reader.read_exact(&mut *seed)?;
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
    let (configuration, policies) = inventory()?;
    let bootstrap = bootstrap::Contract::new(configuration.clone(), policies).map_err(|_| Error)?;
    let contract = bootstrap.commitment();
    let api = api::service::Service::new(
        api::Authorizer::new(120000, 3_600_000).map_err(|_| Error)?,
        Holds(true),
    )
    .with_onboarding(bootstrap);
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
    let cipher = RecordCipher::new(seed, 1, [42; 32]).map_err(|_| Error)?;
    let j = if fresh {
        UnitStore::create(backend, cipher, configuration.clone())
    } else {
        UnitStore::open(backend, cipher, configuration.clone())
    }
    .map_err(|_| Error)?;
    let deadline = FixtureClock.now()?.checked_add(600000).ok_or(Error)?;
    let session = Session::established(config().domain, [8; 32], deadline);
    let mut pending = Some((j, api));
    let mut app = None;
    if !fresh {
        let (j, api) = pending.take().ok_or(Error)?;
        app = Some(start(j, api, path, deadline)?);
    }
    let bulk = bulk_gateway(&configuration)?;
    let pacifica = pacifica_gateway(&configuration)?;
    let account =
        cinder_journal::onboarding::account(config().domain, wallet).map_err(|_| Error)?;
    output(json!({"ready":true,"contract":hex(&contract),"expires":deadline}))?;
    while let Some(command) = input(&mut reader)? {
        let result = (|| -> Result<Json, Error> {
            match command {
                Command::Request { wire } => {
                    let bytes = PrivateBytes::new(STANDARD.decode(wire).map_err(|_| Error)?)
                        .map_err(|_| Error)?;
                    let response = if let Some((j, api)) = &mut pending {
                        let proof =
                            bootstrap::Request::decode(bytes.as_bytes()).map_err(|_| Error)?;
                        if proof.body.wallet != wallet {
                            return Err(Error);
                        }
                        let (unit, response) = api
                            .onboard(j, &session, &bytes, FixtureClock.now()?)
                            .map_err(|_| Error)?;
                        let reply = api::encode_response(unit, &response).map_err(|_| Error)?;
                        for u in configuration.units() {
                            let mut selected = j.select(u.id).map_err(|_| Error)?;
                            seed_for(&mut selected, FixtureClock.now()?, false, true, account)?;
                            // Explicit synthetic native setup/credit port. The
                            // shipping Broker configuration starts NOT ready;
                            // initial fixture receipts alone cannot enable it.
                            super::commit(
                                &mut selected,
                                FixtureClock.now()?,
                                vec![],
                                vec![Control::Funds(
                                    cinder_journal::funds::Action::NativeCreditReady(true),
                                )],
                            )?;
                        }
                        let (j, api) = pending.take().ok_or(Error)?;
                        app = Some(start(j, api, path, deadline)?);
                        reply
                    } else {
                        app.as_ref()
                            .ok_or(Error)?
                            .handle(&session, bytes, FixtureClock.now()?)?
                    };
                    Ok(json!({"wire":STANDARD.encode(response.as_bytes())}))
                }
                Command::Expose {
                    unit,
                    request,
                    nonce,
                } => {
                    let app = app.as_ref().ok_or(Error)?;
                    let id = uid(unit)?;
                    let req = RequestKey {
                        domain: config().domain,
                        account,
                        request: RequestId::new([request; 32]).map_err(|_| Error)?,
                    };
                    let mut c = [0; 32];
                    openssl::rand::rand_bytes(&mut c)?;
                    let attempt = app.prepare_order(
                        id,
                        req,
                        AttemptId::new([request; 32]).map_err(|_| Error)?,
                        CommitId::new(c).map_err(|_| Error)?,
                    )?;
                    openssl::rand::rand_bytes(&mut c)?;
                    let commit = CommitId::new(c).map_err(|_| Error)?;
                    if unit == 40 {
                        app.initialize_bulk(
                            id,
                            &bulk,
                            CommitId::new([180; 32]).map_err(|_| Error)?,
                        )?;
                        let (out, completion) =
                            app.prepare_bulk(id, &bulk, attempt, commit, nonce)?;
                        let wire = STANDARD.encode(out.body());
                        app.complete_bulk(id, &bulk, completion, bulk::Reply::Unknown)?;
                        Ok(json!({"wire":wire,"client":cinder_pacifica::client_id(attempt)}))
                    } else if unit == 50 {
                        app.initialize_pacifica(
                            id,
                            &pacifica,
                            CommitId::new([181; 32]).map_err(|_| Error)?,
                        )?;
                        let (out, completion) =
                            app.prepare_pacifica(id, &pacifica, attempt, commit)?;
                        let wire = STANDARD.encode(out.body());
                        app.complete_pacifica(id, &pacifica, completion, pacifica::Reply::Unknown)?;
                        Ok(json!({"wire":wire,"client":cinder_pacifica::client_id(attempt)}))
                    } else {
                        Err(Error)
                    }
                }
                Command::Observe { unit, body } => {
                    let app = app.as_ref().ok_or(Error)?;
                    let mut commit = [0; 32];
                    openssl::rand::rand_bytes(&mut commit)?;
                    let commit = CommitId::new(commit).map_err(|_| Error)?;
                    if unit == 40 {
                        app.observe_bulk(
                            uid(unit)?,
                            &bulk,
                            cinder_bulk::observation::Message {
                                kind: cinder_bulk::observation::Kind::TradeStream,
                                body: PrivateBytes::new(body.into_bytes()).map_err(|_| Error)?,
                                received_at: FixtureClock.now()?,
                            },
                            commit,
                        )?;
                    } else if unit == 50 {
                        app.observe_pacifica(
                            uid(unit)?,
                            &pacifica_profile(&configuration),
                            cinder_pacifica::observation::Message {
                                kind: cinder_pacifica::observation::Kind::TradeStream,
                                account: chain::address(configuration.units()[1].native_account),
                                cursor: None,
                                received_at: FixtureClock.now()?,
                                body,
                            },
                            commit,
                        )?;
                    } else {
                        return Err(Error);
                    }
                    Ok(json!({"retained":true}))
                }
                Command::Fence => {
                    app.as_ref().ok_or(Error)?.fence();
                    Ok(json!({"fenced":true}))
                }
            }
        })();
        output(result.unwrap_or_else(|_| json!({"error":true})))?;
    }
    Ok(())
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|v| format!("{v:02x}")).collect()
}
