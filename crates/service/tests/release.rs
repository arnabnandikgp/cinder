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
