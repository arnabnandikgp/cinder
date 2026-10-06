//! Loaded configuration tests with public disposable keys and synthetic venue
//! qualification. No network, credentials, signing exposure or AWS evidence.
#[path = "../../journal/tests/support/mod.rs"]
mod support;
use cinder_api::{Admission, Contract, OwnerBinding, Service};
use cinder_journal::{
    model::{Reservation, State},
    orders,
};
use cinder_kernel::{identity::*, ledger::*};
use cinder_pacifica::{execution::*, funding::*, profile::*};
use cinder_service::release::ApplicationContract;
use openssl::pkey::{Id, PKey};
use zeroize::Zeroizing;

struct NoAdmission;
impl Admission for NoAdmission {
    fn order_holds(
        &self,
        _: &State,
        _: &orders::Intent,
    ) -> Result<Vec<Reservation>, cinder_api::Error> {
        Err(cinder_api::Error::Unavailable)
    }
}
fn public(seed: u8) -> [u8; 32] {
    PKey::private_key_from_raw_bytes(&[seed; 32], Id::ED25519)
        .unwrap()
        .raw_public_key()
        .unwrap()
        .try_into()
        .unwrap()
}
fn config() -> Config {
    let mut c = support::config();
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
fn contract() -> Contract {
    Contract {
        owners: (1..=2)
            .map(|i| OwnerBinding {
                account: support::user(i),
                wallet: public(10 + i),
                tokens: [33 + i; 32],
            })
            .collect(),
        maximum_auth_lifetime: 1000,
        maximum_grant_lifetime: 10_000,
    }
}
fn profile(c: &Config) -> Profile {
    Profile {
        config: c.clone(),
        source: c.sources[0].scope,
        // Ed25519 public key of the committed, unfunded seed-9 fixture.
        account: "J2xccRtuG43drESLYznHhLhQkLTdfepcKYbiQ9BsJVaf".into(),
        environment: Origin::Testnet.url().into(),
        revision: 1,
        evidence: "synthetic release test; not live qualification".into(),
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
    }
}
fn policy() -> Policy {
    Policy {
        revision: 1,
        evidence: "synthetic release test".into(),
        execution: Level::Qualified,
        origin: Origin::Testnet,
        expiry_ms: 3000,
        credits: 100,
        cleanup_reserve: 20,
        read_cost: 10,
    }
}
fn route() -> Route {
    Route {
        domain: config().domain.deployment.bytes(),
        pool: [18; 32],
        funds: public(19),
        beneficiaries: contract()
            .owners
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
    }
}
fn loaded(
    c: &Config,
    api: Contract,
    execution: Policy,
    funding: Route,
    seed: u8,
    epoch: u64,
) -> Result<ApplicationContract, cinder_service::Error> {
    ApplicationContract::derive(
        c,
        &Service::new(api, NoAdmission).unwrap(),
        &Gateway::new(profile(c), execution, Zeroizing::new([seed; 32]), epoch).unwrap(),
        &Controller::new(profile(c), funding, Zeroizing::new([9; 32])).unwrap(),
    )
}

fn runtime_configuration() -> cinder_service::runtime::Configuration {
    let p = profile(&config());
    let c = contract();
    cinder_service::runtime::Configuration {
        ledger: cinder_journal::wire::encode_config(&config()).unwrap(),
        owners: c
            .owners
            .into_iter()
            .map(|o| cinder_service::runtime::Owner {
                account: o.account.bytes(),
                wallet: o.wallet,
                tokens: o.tokens,
            })
            .collect(),
        auth_ms: c.maximum_auth_lifetime,
        grant_ms: c.maximum_grant_lifetime,
        source: 0,
        account: p.account,
        environment: p.environment,
        revision: p.revision,
        evidence: p.evidence,
        precision: p.precision,
        fills: p.fills,
        markets: p.markets,
        quote_places: p.quote_places,
        perp_tag: p.perp_tag,
        execution: policy(),
        route: route(),
        trading_epoch: 1,
        chain: None,
    }
}
fn runtime_keys() -> std::collections::BTreeMap<cinder_service::boot::Role, Zeroizing<Vec<u8>>> {
    use cinder_service::{boot::Role, cloud::Credential};
    std::collections::BTreeMap::from([
        (Role::Storage, Zeroizing::new(vec![55; 32])),
        (Role::Trading, Zeroizing::new(vec![7; 32])),
        (Role::Broker, Zeroizing::new(vec![9; 32])),
        (
            Role::Witness,
            Zeroizing::new(
                serde_cbor::to_vec(&Credential {
                    access: "AKIDEXAMPLE123456".into(),
                    secret: "fixture witness credential only not AWS".into(),
                    token: "disposable-session-token".into(),
                    expires: 1_700_000_060_000,
                })
                .unwrap(),
            ),
        ),
    ])
}
#[test]
fn confidential_configuration_constructs_the_same_actual_controller_contract() {
    let expected = loaded(&config(), contract(), policy(), route(), 7, 1)
        .unwrap()
        .digest();
    assert_eq!(
        runtime_configuration()
            .construct(runtime_keys())
            .unwrap()
            .commitment(),
        expected
    );
    let mut c = runtime_configuration();
    c.route.beneficiaries[0].wallet = [1; 32];
    assert!(c.construct(runtime_keys()).is_err());
    let mut keys = runtime_keys();
    keys.remove(&cinder_service::boot::Role::Storage);
    assert!(runtime_configuration().construct(keys).is_err());
}

#[cfg(feature = "local-fixture")]
#[test]
fn trusted_preparation_keeps_role_plaintexts_local_and_emits_only_public_receipts() {
    use cinder_service::{
        boot::{Gates, Manifest, Role, Slot},
        cloud::Endpoint,
    };
    use std::io::Write;
    let root = cinder_service::fixture::FixtureAttester::new()
        .unwrap()
        .root()
        .to_der()
        .unwrap();
    let hash = openssl::sha::sha256(&root);
    let endpoint = |s: &str, r: String, port| Endpoint {
        service: s.into(),
        region: "us-east-1".into(),
        resource: r,
        port,
        root: root.clone(),
        root_hash: hash,
    };
    for version in [1, 2] {
        let mut manifest = Manifest {
        version: 1,
        domain: [[1; 32], [2; 32]].concat(),
        application: [1; 32],
        stream: [42; 32],
        generation: 1,
        epoch: 1,
        ingress: 9000,
        bootstrap: 9001,
        venue_port: 9002,
        venue_root: root.clone(),
        venue_root_hash: hash,
        first: endpoint("s3", "cinder-first-test".into(), 9003),
        second: endpoint("s3", "cinder-second-test".into(), 9004),
        witness: endpoint("dynamodb", "cinder-witness-test".into(), 9005),
        slots: [
            Role::Configuration,
            Role::Storage,
            Role::Trading,
            Role::Broker,
            Role::Witness,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, role)| Slot {
            role,
            endpoint: endpoint(
                "kms",
                format!(
                    "arn:aws:kms:us-east-1:123456789012:key/00000000-0000-0000-0000-00000000000{}",
                    i + 1
                ),
                9006,
            ),
            plaintext_hash: [1; 32],
        })
        .collect(),
        gates: Gates {
            trading: false,
            funding: false,
            native_reads: false,
            maximum_boot_ms: 60000,
        },
        chain: None,
    };
        let mut configuration = runtime_configuration();
        let mut input = serde_json::json!({"manifest":null,"configuration":null,"storage":vec![55;32],"trading":vec![7;32],"broker":vec![9;32],"witness":null});
        let expected = if version == 2 {
            manifest.version = 2;
            let mut slot = manifest.slots[0].clone();
            slot.role = Role::Funds;
            slot.endpoint.resource =
                "arn:aws:kms:us-east-1:123456789012:key/00000000-0000-0000-0000-000000000006"
                    .into();
            manifest.slots.push(slot);
            manifest.chain = Some(cinder_service::chain_funding::Peer {
                host: "api.devnet.solana.com".into(),
                port: 9007,
                root: root.clone(),
                root_hash: hash,
                network: [1; 32],
            });
            configuration.route.funds = public(11);
            configuration.execution.read_cost = 120;
            configuration.execution.credits = 6000;
            configuration.execution.cleanup_reserve = 1000;
            configuration.chain = Some(cinder_service::chain_funding::Configuration {
                path: "/".into(),
                custody: cinder_service::chain_receipt::Deployment {
                    program: configuration.route.program,
                    data: [60; 32],
                    length: 64,
                    hash: [1; 32],
                    authority: Some([61; 32]),
                },
                native: cinder_service::chain_receipt::Deployment {
                    program: configuration.route.venue_program,
                    data: [62; 32],
                    length: 64,
                    hash: [2; 32],
                    authority: Some([63; 32]),
                },
                limits: cinder_pacifica::funding::chain::Limits {
                    maximum_age_ms: 1000,
                    maximum_fee_lamports: 6000,
                    compute_units: 0,
                },
                maximum_calls: 1000,
                expiry_slots: 100,
                poll_ms: 1000,
                deposits: vec![],
            });
            input["funds"] = serde_json::json!(vec![11; 32]);
            let mut keys = runtime_keys();
            keys.insert(Role::Funds, Zeroizing::new(vec![11; 32]));
            let clone = serde_cbor::from_slice::<cinder_service::runtime::Configuration>(
                &serde_cbor::to_vec(&configuration).unwrap(),
            )
            .unwrap();
            clone.construct_for(&manifest, keys).unwrap().commitment()
        } else {
            loaded(&config(), contract(), policy(), route(), 7, 1)
                .unwrap()
                .digest()
        };
        let keys = runtime_keys();
        let witness: serde_cbor::Value =
            serde_cbor::from_slice(keys[&Role::Witness].as_slice()).unwrap();
        input["manifest"] = serde_json::to_value(&manifest).unwrap();
        input["configuration"] = serde_json::to_value(configuration).unwrap();
        input["witness"] = serde_json::to_value(witness).unwrap();
        let input = serde_cbor::to_vec(&input).unwrap();
        let t = support::Temp::new();
        let out = t.root.join("prepared");
        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_cinder-prepare-release"))
            .arg(&out)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&input).unwrap();
        let receipt = child.wait_with_output().unwrap();
        assert!(receipt.status.success(), "preparation refused");
        assert!(receipt.stderr.is_empty());
        assert!(receipt.stdout.len() < 128);
        let m: Manifest =
            serde_cbor::from_slice(&std::fs::read(out.join("manifest.cbor")).unwrap()).unwrap();
        assert_eq!(m.application, expected);
        m.validate().unwrap();
        for role in m.slots.iter().map(|s| s.role) {
            let path = out.join(format!("{}.plain", role.name()));
            let bytes = std::fs::read(&path).unwrap();
            assert!(bytes.starts_with(if version == 1 { b"CKR1" } else { b"CKR2" }));
            assert!(bytes.len() <= 4096);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(
                    std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                    0o600
                );
            }
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&out).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
    }
}

#[test]
fn actual_loaded_components_change_the_commitment_not_an_echoed_label() {
    let c = config();
    let baseline = loaded(&c, contract(), policy(), route(), 7, 1)
        .unwrap()
        .digest();
    assert_ne!(baseline, [0; 32]);
    assert_eq!(
        baseline,
        loaded(&c, contract(), policy(), route(), 7, 1)
            .unwrap()
            .digest()
    );
    for case in 0..9 {
        let mut c = config();
        let mut a = contract();
        let mut p = policy();
        let mut r = route();
        let mut seed = 7;
        let mut epoch = 1;
        match case {
            0 => a.maximum_auth_lifetime += 1,
            1 => a.maximum_grant_lifetime += 1,
            2 => p.credits += 1,
            3 => seed = 8,
            4 => epoch += 1,
            5 => r.maximum_movement += 1,
            6 => r.epoch += 1,
            7 => c.policy = PolicyVersion::new(2).unwrap(),
            8 => {
                a.owners[0].wallet = public(30);
                r.beneficiaries[0].wallet = public(30);
            }
            _ => unreachable!(),
        }
        assert_ne!(baseline, loaded(&c, a, p, r, seed, epoch).unwrap().digest());
    }
}
#[test]
fn contradictory_owner_bindings_and_collapsed_key_roles_reject() {
    for case in 0..4 {
        let mut a = contract();
        let mut r = route();
        let mut seed = 7;
        match case {
            0 => a.owners[0].wallet = public(30),
            1 => a.owners[0].tokens = [70; 32],
            2 => seed = 9,            // Trading key is the fund-moving broker key.
            3 => r.funds = public(7), // Trading key is the Solana funds authority.
            _ => unreachable!(),
        }
        assert!(loaded(&config(), a, policy(), r, seed, 1).is_err());
    }
}
#[test]
fn controllers_cannot_be_attached_to_another_journal_configuration() {
    let c = config();
    let mut changed = c.clone();
    changed.policy = PolicyVersion::new(2).unwrap();
    let a = Service::new(contract(), NoAdmission).unwrap();
    let g = Gateway::new(profile(&c), policy(), Zeroizing::new([7; 32]), 1).unwrap();
    let f = Controller::new(profile(&c), route(), Zeroizing::new([9; 32])).unwrap();
    assert!(ApplicationContract::derive(&changed, &a, &g, &f).is_err());
    changed = c.clone();
    changed.customers.pop();
    assert!(ApplicationContract::derive(&changed, &a, &g, &f).is_err());
}
