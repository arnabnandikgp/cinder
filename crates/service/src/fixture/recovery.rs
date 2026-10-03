//! Explicit offline joined SDK/SBF harness. Same authoritative encrypted journal,
//! synthetic independent witness/native terminal coverage, actual local chain
//! observation supplied over private harness stdin. NOT a production operator.
use super::*;
use cinder_journal::{funds, recovery::Action};
use cinder_pacifica::{
    execution::{Dispatch, Gateway, Origin, Outbound, Policy as ExecutionPolicy, Reply, Transport},
    funding::{self, ChainAction, Controller, Route},
    profile::{Grid, Level, Mapping, Profile},
};
use serde::Deserialize;

/// Private test-only input; disposable broker seed is never an argument/log/file.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    /// Explicit public offline route.
    pub route: Route,
    /// Exact public native owner encoding.
    pub account: String,
    /// Disposable matching signing seed, supplied on stdin only.
    pub broker_seed: [u8; 32],
    /// Owner-authorized dedicated public encryption keys, never private decryptors.
    pub keys: Vec<KeyInput>,
}
/// JSON representation for the explicit private fixture configuration.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyInput {
    /// Registered owner.
    pub owner: [u8; 32],
    /// Dedicated RSA-3072 public SPKI.
    pub spki: Vec<u8>,
    /// Owner signature bytes.
    pub signature: Vec<u8>,
}
impl Binding {
    /// Validated lengths; full owner/crypto authorization is checked on registration.
    pub fn keys(&self) -> Result<Vec<crate::recovery::RecipientKey>, Error> {
        self.keys
            .iter()
            .map(|k| {
                Ok(crate::recovery::RecipientKey {
                    owner: k.owner,
                    spki: k.spki.clone(),
                    signature: k.signature.as_slice().try_into().map_err(|_| Error)?,
                })
            })
            .collect()
    }
}
impl Drop for Binding {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.broker_seed.zeroize();
    }
}
fn profile(account: &str) -> Profile {
    Profile {
        config: config(),
        source: config().sources[0].scope,
        account: account.into(),
        environment: Origin::Testnet.url().into(),
        revision: 1,
        evidence: "OFFLINE synthetic native coverage; local SBF custody".into(),
        precision: Level::Qualified,
        fills: Level::Qualified,
        markets: vec![Mapping {
            symbol: "BTC".into(),
            market: 0,
            size: Grid { places: 0, step: 1 },
            price: Grid { places: 0, step: 1 },
        }],
        quote_places: 0,
        perp_tag: 1,
    }
}
/// Construct qualified FAKE ports only in the non-default fixture feature.
pub fn controller(b: &Binding) -> Result<Controller, Error> {
    Controller::new(
        profile(&b.account),
        b.route.clone(),
        Zeroizing::new(b.broker_seed),
    )
    .map_err(|_| Error)
}
/// Fresh random commit identity, not financial content.
pub fn commit_id() -> Result<CommitId, Error> {
    let mut b = [0; 32];
    openssl::rand::rand_bytes(&mut b)?;
    CommitId::new(b).map_err(|_| Error)
}
fn reopen(root: &Path, key: Zeroizing<[u8; 32]>, epoch: u64) -> Result<Store, Error> {
    let backend = Replicated::new(
        Stream {
            domain: config().domain,
            id: [42; 32],
        },
        epoch,
        FileReplica::open(&root.join("first")).map_err(|_| Error)?,
        FileReplica::open(&root.join("second")).map_err(|_| Error)?,
        LocalWitness(root.join("accepted")),
    )
    .map_err(|_| Error)?;
    Journal::open(
        backend,
        RecordCipher::new(key, 1, [42; 32]).map_err(|_| Error)?,
        config(),
    )
    .map_err(|_| Error)
}
/// Disposable worker retains the exact original pending chain capability until
/// verified wire persistence. Killing it never grants a reconstructed resend.
pub struct Worker {
    root: PathBuf,
    key: Zeroizing<[u8; 32]>,
    store: Store,
    controller: Controller,
    account: String,
    action: Option<ChainAction>,
}
impl Worker {
    /// Open only an existing accepted journal, never a new recovery ledger.
    pub fn open(
        root: &Path,
        key: Zeroizing<[u8; 32]>,
        controller: Controller,
        account: String,
    ) -> Result<Self, Error> {
        Ok(Self {
            root: root.into(),
            store: reopen(root, key.clone(), 1)?,
            key,
            controller,
            account,
            action: None,
        })
    }
    /// Actual SDK-accepted pending payout -> original P16 unsigned contract.
    pub fn payout(&mut self) -> Result<Vec<u8>, Error> {
        if self.action.is_some() {
            return Err(Error);
        }
        let now = FixtureClock.now()?;
        let op = self
            .store
            .state()
            .map_err(|_| Error)?
            .funds()
            .iter()
            .find(|o| !o.terminal)
            .ok_or(Error)?
            .clone();
        let attempt = AttemptKey {
            request: op.intent.request,
            attempt: AttemptId::new(op.intent.request.request.bytes()).map_err(|_| Error)?,
        };
        let mut tx = empty(&self.store, now);
        tx.controls = vec![Control::Funds(funds::Action::Prepare {
            attempt,
            net: op.intent.net,
        })];
        if self
            .store
            .commit(tx)
            .map_err(|_| Error)?
            .receipt
            .controls
            .is_some()
        {
            return Err(Error);
        }
        let action = self
            .controller
            .expose_chain(
                &mut self.store,
                Dispatch {
                    attempt,
                    commit: commit_id()?,
                    at: now,
                },
                funding::Rail::Payout,
                funding::Counters {
                    sequence: 0,
                    paid: 0,
                    recipient_tokens: self.controller.route().beneficiaries[0].tokens,
                    expires_at_slot: u64::MAX,
                },
            )
            .map_err(|_| Error)?;
        let bytes = action.encode().map_err(|_| Error)?.as_bytes().to_vec();
        self.action = Some(action);
        Ok(bytes)
    }
    /// Trusted fixture codec receives TS-verified original SBF wire + finalized
    /// actual receipt/counter/token effects. Signature bytes must match persistence.
    pub fn settle(
        &mut self,
        wire: Vec<u8>,
        signature: [u8; 64],
        slot: u64,
        paid: u64,
        sequence: u64,
    ) -> Result<(), Error> {
        let action = self.action.take().ok_or(Error)?;
        let plan = action.plan().clone();
        let binding = openssl::sha::sha256(action.encode().map_err(|_| Error)?.as_bytes());
        let now = FixtureClock.now()?;
        self.controller
            .persist_wire(
                &mut self.store,
                commit_id()?,
                now,
                action,
                funding::VerifiedWire {
                    attempt: plan.attempt().map_err(|_| Error)?,
                    binding,
                    signature,
                    wire: PrivateBytes::new(wire).map_err(|_| Error)?,
                },
            )
            .map_err(|_| Error)?;
        let r = self.controller.route();
        self.controller
            .observe_chain(
                &mut self.store,
                commit_id()?,
                now,
                funding::ChainReceipt {
                    network: config().domain.network.bytes(),
                    attempt: plan.attempt().map_err(|_| Error)?,
                    mint: r.mint,
                    program: r.program,
                    source: if plan.rail() == funding::Rail::Return {
                        r.broker_tokens
                    } else {
                        r.vault
                    },
                    destination: if plan.rail() == funding::Rail::Return {
                        r.vault
                    } else {
                        plan.counters().recipient_tokens
                    },
                    amount: plan.amount(),
                    succeeded: true,
                    signature,
                    slot,
                    operation: Some(plan.operation()),
                    epoch: Some(plan.epoch()),
                    config: Some(r.config),
                    paid: Some(paid),
                    sequence: Some(sequence),
                    raw: PrivateBytes::new(
                        b"actual local SBF receipt qualified by fixture TS codec".to_vec(),
                    )
                    .map_err(|_| Error)?,
                },
            )
            .map_err(|_| Error)
    }
    /// Explicit fake witness takeover rejects the still-loaded old writer. Then
    /// unknown order blocks preparation UNTIL a separate synthetic native-history
    /// certificate is supplied. No timeout or cancel ACK manufactures closure.
    pub fn return_assets(&mut self) -> Result<Vec<u8>, Error> {
        if self.action.is_some() {
            return Err(Error);
        }
        let old = self.store.head();
        let mut w = LocalWitness(self.root.join("accepted"));
        let anchor = w
            .read(Stream {
                domain: config().domain,
                id: [42; 32],
            })
            .map_err(|_| Error)?;
        if anchor.epoch != 1 || anchor.head != Some(old) {
            return Err(Error);
        }
        // Deliberate same-host fake of P20's independently authenticated takeover.
        let tmp = self.root.join("takeover.pending");
        let mut f = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        f.write_all(
            &[
                &2u64.to_be_bytes()[..],
                &old.sequence.to_be_bytes(),
                &old.hash,
            ]
            .concat(),
        )?;
        f.sync_all()?;
        std::fs::rename(tmp, self.root.join("accepted"))?;
        File::open(&self.root)?.sync_all()?;
        if self.store.verified_state().is_ok() {
            return Err(Error);
        }
        self.store = reopen(&self.root, self.key.clone(), 2)?;
        let now = FixtureClock.now()?;
        let version = self.store.state().map_err(|_| Error)?.ledger().version();
        commit(
            &mut self.store,
            now,
            vec![],
            vec![Control::Recovery(Action::Begin {
                expected_version: version,
                authority_epoch: 2,
                valid_until: now + 60_000,
                fence: [97; 32],
            })],
        )?;
        let unknown = self
            .store
            .state()
            .map_err(|_| Error)?
            .orders()
            .iter()
            .filter(|o| !o.complete())
            .cloned()
            .collect::<Vec<_>>();
        if unknown.is_empty() {
            return Err(Error);
        }
        if self
            .store
            .recovery_cut(
                self.store.head(),
                EventKey {
                    scope: config().sources[0].scope,
                    event: EconomicEventId::new(b"absent").map_err(|_| Error)?,
                    leg: 0,
                },
            )
            .is_ok()
        {
            return Err(Error);
        }
        for (index, o) in unknown.into_iter().enumerate() {
            let mut tx = empty(&self.store, now);
            tx.order_observations.push(orders::Observation {
                key: EventKey {
                    scope: config().sources[0].scope,
                    event: EconomicEventId::new(&(1000 + index as u64).to_be_bytes())
                        .map_err(|_| Error)?,
                    leg: 0,
                },
                attempt: o.attempt.ok_or(Error)?,
                status: orders::Status::Terminal(orders::Terminal {
                    filled: QuantityLots::new(o.intent.quantity.unit(), 0),
                    executions: vec![],
                    through: 1,
                }),
                authority_epoch: 1,
                observed_at: now,
                raw: PrivateBytes::new(
                    b"EXPLICIT SYNTHETIC native complete no-fill coverage".to_vec(),
                )
                .map_err(|_| Error)?,
            });
            tx.controls.push(Control::Order(orders::Action::Release {
                request: o.intent.request,
            }));
            if self
                .store
                .commit(tx)
                .map_err(|_| Error)?
                .receipt
                .controls
                .is_some()
            {
                return Err(Error);
            }
        }
        let withdrawal = self.prepare_return(101, Location::Venue, Location::Broker, now)?;
        let gateway = Gateway::new(
            profile(&self.account),
            ExecutionPolicy {
                revision: 1,
                evidence: "OFFLINE synthetic withdrawal only".into(),
                execution: Level::Qualified,
                origin: Origin::Testnet,
                expiry_ms: 30_000,
                credits: 100,
                cleanup_reserve: 20,
                read_cost: 10,
            },
            Zeroizing::new([7; 32]),
            2,
        )
        .map_err(|_| Error)?;
        gateway
            .activate(&mut self.store, commit_id()?, now)
            .map_err(|_| Error)?;
        struct Fake;
        impl Transport for Fake {
            fn post(&mut self, _: Outbound) -> Reply {
                Reply::Unknown
            }
        }
        self.controller
            .withdraw(
                &mut self.store,
                &gateway,
                Dispatch {
                    attempt: withdrawal,
                    commit: commit_id()?,
                    at: now,
                },
                &mut Fake,
            )
            .map_err(|_| Error)?;
        // Explicit synthetic, complete original UUID correlation. Never inferred
        // from the Unknown reply, a clock timeout or an empty history response.
        let r = self.controller.route();
        self.controller
            .observe_withdrawal(
                &mut self.store,
                commit_id()?,
                now,
                funding::Withdrawal {
                    attempt: withdrawal,
                    idempotency_key: funding::withdrawal_id(withdrawal),
                    event: EconomicEventId::new(b"synthetic-native-withdrawal")
                        .map_err(|_| Error)?,
                    complete: true,
                    cut: 2,
                    gross: 1000,
                    payment: funding::ChainReceipt {
                        network: config().domain.network.bytes(),
                        attempt: withdrawal,
                        mint: r.mint,
                        program: r.venue_program,
                        source: r.venue_vault,
                        destination: r.broker_tokens,
                        amount: 1000,
                        succeeded: true,
                        signature: [77; 64],
                        slot: 1,
                        operation: None,
                        epoch: None,
                        config: None,
                        paid: None,
                        sequence: None,
                        raw: PrivateBytes::new(
                            b"EXPLICIT SYNTHETIC native owner-directed payment".to_vec(),
                        )
                        .map_err(|_| Error)?,
                    },
                    raw: PrivateBytes::new(
                        b"EXPLICIT SYNTHETIC complete original UUID coverage".to_vec(),
                    )
                    .map_err(|_| Error)?,
                },
            )
            .map_err(|_| Error)?;
        let attempt = self.prepare_return(102, Location::Broker, Location::Vault, now)?;
        let action = self
            .controller
            .expose_chain(
                &mut self.store,
                Dispatch {
                    attempt,
                    commit: commit_id()?,
                    at: now,
                },
                funding::Rail::Return,
                funding::Counters {
                    sequence: 0,
                    paid: 0,
                    recipient_tokens: [0; 32],
                    expires_at_slot: u64::MAX,
                },
            )
            .map_err(|_| Error)?;
        let bytes = action.encode().map_err(|_| Error)?.as_bytes().to_vec();
        self.action = Some(action);
        Ok(bytes)
    }
    fn prepare_return(
        &mut self,
        n: u8,
        source: Location,
        destination: Location,
        now: u64,
    ) -> Result<AttemptKey, Error> {
        let attempt = AttemptKey {
            request: RequestKey {
                domain: config().domain,
                account: user(),
                request: RequestId::new([n; 32]).map_err(|_| Error)?,
            },
            attempt: AttemptId::new([n; 32]).map_err(|_| Error)?,
        };
        let intent = funds::Intent {
            request: attempt.request,
            source,
            destination: cinder_kernel::ledger::funds::Destination::Location(destination),
            net: cash(1000),
            maximum_fee: cash(0),
            fee_payer: Owner::House,
            allow_partial: false,
            policy: config().policy,
            authority_epoch: 2,
            expires_at: now + 60_000,
        };
        let approval = orders::Approval {
            account: user(),
            intent_hash: intent.digest().map_err(|_| Error)?,
            authority_epoch: 2,
        };
        commit(
            &mut self.store,
            now,
            vec![],
            vec![
                Control::Funds(funds::Action::RecoveryAccept {
                    intent: Box::new(intent),
                    approval,
                }),
                Control::Funds(funds::Action::Prepare {
                    attempt,
                    net: cash(1000),
                }),
            ],
        )?;
        Ok(attempt)
    }
    /// Seal only after the original return wire and actual local SBF receipt are
    /// settled. Never count broker/native/transit collateral as claimable vault cash.
    pub fn finish(
        &mut self,
        custody: &funding::recovery::CustodyState,
        keys: &[crate::recovery::RecipientKey],
    ) -> Result<crate::recovery::Prepared, Error> {
        if self.action.is_some()
            || self.store.state().map_err(|_| Error)?.recovery_epoch() != Some(2)
        {
            return Err(Error);
        }
        let now = FixtureClock.now()?;
        let l = self.store.state().map_err(|_| Error)?.ledger();
        let check = event(
            100,
            Change::Reconcile(NativeCheck {
                expected_version: l.version(),
                cash: Some(l.venue().cash()),
                funding: Some(l.venue().funding()),
                positions: Some(l.venue().positions().to_vec()),
                complete: true,
                resolves: vec![],
            }),
        );
        let key = if let RecordKey::Economic(k) = &check.key {
            k.clone()
        } else {
            return Err(Error);
        };
        commit(&mut self.store, now, vec![check], vec![])?;
        let version = self.store.state().map_err(|_| Error)?.ledger().version();
        commit(
            &mut self.store,
            now,
            vec![],
            vec![Control::Recovery(Action::Seal {
                expected_version: version,
            })],
        )?;
        let mut first = Objects::new(self.root.join("claims-first"), [91; 32])?;
        let mut second = Objects::new(self.root.join("claims-second"), [92; 32])?;
        let head = self.store.head();
        let prepared = crate::recovery::prepare(
            &self.controller,
            &mut self.store,
            head,
            key.clone(),
            custody,
            keys,
            [93; 32],
            [94; 32],
            &Random,
            &mut first,
            &mut second,
        )?;
        prepared.recheck(&self.controller, &mut self.store, key, custody)?;
        Ok(prepared)
    }
}
struct Random;
impl cinder_web_channel::Entropy for Random {
    fn fill(&self, b: &mut [u8]) -> Result<(), cinder_web_channel::Error> {
        openssl::rand::rand_bytes(b).map_err(|_| cinder_web_channel::Error)
    }
}
struct Objects {
    root: PathBuf,
    id: [u8; 32],
}
impl Objects {
    fn new(root: PathBuf, id: [u8; 32]) -> Result<Self, Error> {
        std::fs::create_dir(&root)?;
        Ok(Self { root, id })
    }
}
fn hex(b: [u8; 32]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
impl crate::recovery::Replica for Objects {
    fn identity(&self) -> [u8; 32] {
        self.id
    }
    fn put(&mut self, locator: [u8; 32], bytes: &[u8]) -> Result<(), Error> {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.root.join(hex(locator)))?;
        f.write_all(bytes)?;
        f.sync_all()?;
        File::open(&self.root)?.sync_all()?;
        Ok(())
    }
    fn get(&mut self, locator: [u8; 32]) -> Result<Vec<u8>, Error> {
        let mut b = Vec::new();
        File::open(self.root.join(hex(locator)))?
            .take((crate::recovery::MAX_PACKAGE + 1) as u64)
            .read_to_end(&mut b)?;
        if b.len() > crate::recovery::MAX_PACKAGE {
            return Err(Error);
        }
        Ok(b)
    }
}
