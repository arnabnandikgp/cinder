#![allow(dead_code)]
// Synthetic fixtures only. This reversible transform is NOT encryption. It tests
// the opaque-storage seam/context and must never protect real private material.
use cinder_journal::{model::*, sqlite::*, *};
use cinder_kernel::{amounts::*, identity::*, ledger::*, position::*};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub struct FixtureProtection;
fn context_bytes(c: RecordContext) -> Vec<u8> {
    [
        c.domain.network.bytes().as_slice(),
        c.domain.deployment.bytes().as_slice(),
        &c.sequence.to_be_bytes(),
        &c.previous,
    ]
    .concat()
}
impl Protection for FixtureProtection {
    fn seal(&self, c: RecordContext, p: &[u8]) -> Result<PrivateBytes, Error> {
        let mut bytes = context_bytes(c);
        bytes.extend(p.iter().map(|b| b ^ 0xa5));
        PrivateBytes::new(bytes)
    }
    fn open(&self, c: RecordContext, b: &PrivateBytes) -> Result<PrivateBytes, Error> {
        let prefix = context_bytes(c);
        let suffix = b
            .as_bytes()
            .strip_prefix(prefix.as_slice())
            .ok_or(Error::Protection)?;
        PrivateBytes::new(suffix.iter().map(|b| b ^ 0xa5).collect())
    }
}
pub type Store = Journal<SqliteBackend, FixtureProtection>;
pub struct Temp {
    pub root: PathBuf,
    pub db: PathBuf,
}
impl Temp {
    pub fn new() -> Self {
        static ID: AtomicU64 = AtomicU64::new(0);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "cinder-p05-test-{}-{nonce}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self {
            db: root.join("store"),
            root,
        }
    }
    pub fn create(&self) -> Store {
        Journal::create(
            SqliteBackend::create(&self.db).unwrap(),
            FixtureProtection,
            config(),
        )
        .unwrap()
    }
    pub fn open(&self) -> Store {
        open(&self.db)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}
pub fn open(path: &Path) -> Store {
    Journal::open(
        SqliteBackend::open(path, Migration::None).unwrap(),
        FixtureProtection,
        config(),
    )
    .unwrap()
}
pub fn user(n: u8) -> AccountId {
    AccountId::new([n; 32]).unwrap()
}
pub fn config() -> Config {
    let domain = Domain {
        network: NetworkId::new([1; 32]).unwrap(),
        deployment: DeploymentId::new([2; 32]).unwrap(),
    };
    let quote = AssetUnit {
        asset: AssetId::new([3; 32]).unwrap(),
        precision: PrecisionVersion::new(1).unwrap(),
    };
    let venue = VenueId::new([4; 32]).unwrap();
    let venue_account = VenueAccountId::new([5; 32]).unwrap();
    let unit = MarketUnit {
        market: MarketId::new([7; 32]).unwrap(),
        precision: PrecisionVersion::new(1).unwrap(),
        quote,
    };
    Config {
        domain,
        quote,
        venue,
        venue_account,
        policy: PolicyVersion::new(1).unwrap(),
        sources: vec![Source {
            scope: EventScope {
                domain,
                venue,
                account: venue_account,
                namespace: NamespaceId::new([6; 32]).unwrap(),
            },
            location: Location::Venue,
        }],
        markets: vec![Market::new(unit, 1, 1).unwrap()],
        customers: vec![user(1), user(2)],
    }
}
pub fn cash(n: i128) -> QuoteAtoms {
    QuoteAtoms::new(config().quote, n)
}
pub fn q(n: i64) -> QuantityLots {
    QuantityLots::new(config().markets[0].unit(), n)
}
pub fn p(n: u64) -> PriceTicks {
    PriceTicks::new(q(0).unit(), n).unwrap()
}
pub fn key(n: u64) -> EventKey {
    EventKey {
        scope: config().sources[0].scope,
        event: EconomicEventId::new(&n.to_be_bytes()).unwrap(),
        leg: 0,
    }
}
pub fn event(n: u64, change: Change) -> Event {
    Event {
        key: RecordKey::Economic(key(n)),
        policy: config().policy,
        change,
    }
}
pub fn receipt(n: u64, owner: Owner, amount: i128) -> Event {
    event(
        n,
        Change::Receipt {
            owner,
            location: Location::Venue,
            amount: cash(amount),
        },
    )
}
pub fn request(n: u8) -> RequestKey {
    RequestKey {
        domain: config().domain,
        account: user(1),
        request: RequestId::new([n; 32]).unwrap(),
    }
}
pub fn attempt(n: u8) -> AttemptKey {
    AttemptKey {
        request: request(n),
        attempt: AttemptId::new([n; 32]).unwrap(),
    }
}
pub fn raw(b: &[u8]) -> PrivateBytes {
    PrivateBytes::new(b.to_vec()).unwrap()
}
pub fn input(e: Event) -> Input {
    Input {
        source: config().sources[0].scope,
        source_cut: Some(0),
        authority_epoch: 1,
        observed_at: 1,
        raw: raw(b"synthetic-native-evidence"),
        event: Some(e),
    }
}
pub fn transaction(head: Head, n: u8, events: Vec<Event>, controls: Vec<Control>) -> Transaction {
    Transaction {
        id: CommitId::new([n; 32]).unwrap(),
        expected: head,
        at: 10,
        inputs: events.into_iter().map(input).collect(),
        order_observations: vec![],
        controls,
    }
}
pub fn reserve(n: u8, amount: i128) -> Control {
    Control::Reserve {
        request: request(n),
        reservations: vec![
            Reservation {
                resource: Resource::Customer(user(1)),
                amount: cash(amount),
            },
            Reservation {
                resource: Resource::Location(Location::Venue),
                amount: cash(amount),
            },
        ],
    }
}
pub fn prepare(n: u8) -> Control {
    Control::Prepare {
        key: attempt(n),
        message: raw(b"exact-unsigned-private-preimage"),
        authority_epoch: 1,
        expires_at: 100,
    }
}
pub fn seed(s: &mut Store) {
    let tx = transaction(
        s.head(),
        1,
        vec![
            receipt(1, Owner::Customer(user(1)), 100),
            receipt(2, Owner::House, 100),
        ],
        vec![],
    );
    s.commit(tx).unwrap();
}
pub fn hold(s: &mut Store) {
    let tx = transaction(s.head(), 2, vec![], vec![reserve(2, 80), prepare(2)]);
    assert_eq!(s.commit(tx).unwrap().receipt.controls, None);
}
