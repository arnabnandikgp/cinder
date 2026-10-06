//! Private stdin-only offline ports. Actual controllers and AEAD journal; fake
//! upstream coverage, settings, rate and time. No network client or production keys.
use super::*;
use cinder_journal::funds;
use cinder_kernel::{
    ledger::{economics::*, funds::Destination},
    math::Rounding,
    position::Position,
};
use cinder_pacifica::{
    execution::{Dispatch, Gateway, Origin, Outbound, Policy, Reply, Transport},
    funding::{self, ChainAction, Controller, Counters, Rail},
    observation::{self, Coverage, Kind, Message},
    profile::{Level, Profile},
};
use serde::Deserialize;
use serde_json::{Value, json};

/// Bounded, explicit privileged fixture messages. Never exposed on HTTP/WS.
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case", deny_unknown_fields)]
#[allow(missing_docs)] // Test message fields mirror the documented controller ports.
pub enum Command {
    /// Qualified local SBF deposit receipt, not a synthetic opening balance.
    Deposit {
        amount: u64,
        signature: Vec<u8>,
        slot: u64,
    },
    /// Prepare a controller-owned internal rail or SDK-accepted payout.
    Prepare {
        id: u8,
        rail: Rail,
        amount: u64,
        counters: Counters,
    },
    /// Persist the checked signed chain wire BEFORE the harness sends it once.
    Wire {
        id: u8,
        signature: Vec<u8>,
        wire: Vec<u8>,
    },
    /// Actual chain codec/finality port result, including negative substitutions.
    Chain { receipt: Box<ChainInput> },
    /// Operation-specific native credit, optionally partial.
    Credit {
        id: u8,
        signature: Vec<u8>,
        event: u64,
        amount: u64,
        final_credit: bool,
    },
    /// One native owner-signed withdrawal, with unknown response.
    Withdraw { id: u8, amount: u64 },
    /// Native history plus independently observed beneficiary chain payment.
    Withdrawal {
        id: u8,
        amount: u64,
        signature: Vec<u8>,
        slot: u64,
    },
    /// Real native normalization with an explicit synthetic causal certificate.
    Trades { body: String, through: u64 },
    /// Complete native history, not cancel ACK or clock expiry.
    Terminal {
        id: u8,
        filled: i64,
        events: Vec<u64>,
        through: u64,
    },
    /// Frozen gross private funding cut, separate recognition and settlement.
    Funding {
        event: u64,
        #[serde(deserialize_with = "quote_atoms")]
        numerator: i128,
        #[serde(deserialize_with = "quote_atoms")]
        native: i128,
        settle: bool,
    },
    /// Independent fake venue values at a complete cut. Never read back our ledger
    /// and call that a proof of external reconciliation.
    Reconcile {
        event: u64,
        #[serde(deserialize_with = "quote_atoms")]
        cash: i128,
        quantity: i64,
        #[serde(deserialize_with = "quote_atoms")]
        basis: i128,
        #[serde(deserialize_with = "quote_atoms")]
        funding: i128,
    },
    /// Per-step inspection for the INDEPENDENT test oracle, not a controller input.
    Oracle,
}
// Tagged serde content does not preserve i128 JSON numbers. Use the same lossless
// decimal-string boundary as the public money codec, including in test controls.
fn quote_atoms<'de, D: serde::Deserializer<'de>>(d: D) -> Result<i128, D::Error> {
    let text = String::deserialize(d)?;
    let value = text.parse::<i128>().map_err(serde::de::Error::custom)?;
    if value.to_string() != text {
        return Err(serde::de::Error::custom("noncanonical quote atoms"));
    }
    Ok(value)
}
/// JSON representation of the trusted chain observation port.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChainInput {
    /// Original fixture intent.
    pub id: u8,
    /// Exact signed SBF transaction.
    pub signature: Vec<u8>,
    /// Finalized local slot.
    pub slot: u64,
    /// Qualified chain program, token and account identities.
    pub network: [u8; 32],
    /// Actual rail program.
    pub program: [u8; 32],
    /// Actual mint.
    pub mint: [u8; 32],
    /// Actual source.
    pub source: [u8; 32],
    /// Actual destination.
    pub destination: [u8; 32],
    /// Actual amount, never HTTP ACK.
    pub amount: u64,
    /// Original operation binding; absent for native deposit.
    pub operation: Option<[u8; 32]>,
    /// Receipt epoch.
    pub epoch: Option<u64>,
    /// Actual config.
    pub config: Option<[u8; 32]>,
    /// Durable cumulative paid counter.
    pub paid: Option<u64>,
    /// Durable sequence.
    pub sequence: Option<u64>,
}
/// Controllers are unchanged; only their external I/O is replaced.
pub struct Native {
    pub(super) controller: Controller,
    profile: Profile,
    gateway: Gateway,
    pending: BTreeMap<u8, ChainAction>,
}
struct Unknown;
impl Transport for Unknown {
    fn post(&mut self, _: Outbound) -> Reply {
        Reply::Unknown
    }
}
fn attempt(id: u8) -> Result<AttemptKey, Error> {
    Ok(AttemptKey {
        request: RequestKey {
            domain: config().domain,
            account: user(),
            request: RequestId::new([id; 32]).map_err(|_| Error)?,
        },
        attempt: AttemptId::new([id; 32]).map_err(|_| Error)?,
    })
}
fn key(id: u64) -> Result<EventKey, Error> {
    Ok(EventKey {
        scope: config().sources[0].scope,
        event: EconomicEventId::new(&id.to_be_bytes()).map_err(|_| Error)?,
        leg: 0,
    })
}
impl Native {
    pub(super) fn new(b: &recovery::Binding) -> Result<Self, Error> {
        Ok(Self {
            controller: recovery::controller(b)?,
            profile: recovery::profile(&b.account),
            gateway: Gateway::new(
                recovery::profile(&b.account),
                Policy {
                    revision: 1,
                    evidence: "P22 OFFLINE qualified fake native ports".into(),
                    execution: Level::Qualified,
                    origin: Origin::Testnet,
                    expiry_ms: 30_000,
                    credits: 10_000,
                    cleanup_reserve: 100,
                    read_cost: 10,
                },
                Zeroizing::new([7; 32]),
                1,
            )
            .map_err(|_| Error)?,
            pending: BTreeMap::new(),
        })
    }
    pub(super) fn activate(&self, j: &mut Store) -> Result<(), Error> {
        self.gateway
            .activate(j, recovery::commit_id()?, FixtureClock.now()?)
            .map_err(|_| Error)
    }
    pub(super) fn dispatch(&self, j: &mut Store, at: u64) -> Result<(), Error> {
        let keys: Vec<_> = j
            .state()
            .map_err(|_| Error)?
            .attempts()
            .iter()
            .filter(|a| {
                !a.possibly_exposed && matches!(a.kind, AttemptKind::Order | AttemptKind::Cancel)
            })
            .map(|a| a.key)
            .collect();
        for key in keys {
            self.gateway
                .dispatch(
                    j,
                    Dispatch {
                        attempt: key,
                        commit: recovery::commit_id()?,
                        at,
                    },
                    &mut Unknown,
                )
                .map_err(|_| Error)?;
        }
        Ok(())
    }
    fn prepare(
        &self,
        j: &mut Store,
        id: u8,
        rail: Rail,
        amount: u64,
        at: u64,
    ) -> Result<(), Error> {
        let (source, destination) = match rail {
            Rail::Release => (Location::Vault, Destination::Location(Location::Broker)),
            Rail::Deposit => (Location::Broker, Destination::Location(Location::Venue)),
            Rail::Withdraw => (Location::Venue, Destination::Location(Location::Broker)),
            Rail::Return => (Location::Broker, Destination::Location(Location::Vault)),
            Rail::Payout => (
                Location::Vault,
                Destination::Recipient(self.controller.route().beneficiaries[0].tokens),
            ),
        };
        let a = attempt(id)?;
        let mut tx = empty(j, at);
        if rail != Rail::Payout {
            let intent = funds::Intent {
                request: a.request,
                source,
                destination,
                net: cash(i128::from(amount)),
                maximum_fee: cash(0),
                fee_payer: Owner::House,
                allow_partial: false,
                policy: config().policy,
                authority_epoch: 1,
                expires_at: at + 60_000,
            };
            tx.controls.push(Control::Funds(funds::Action::Accept {
                approval: orders::Approval {
                    account: user(),
                    intent_hash: intent.digest().map_err(|_| Error)?,
                    authority_epoch: 1,
                },
                intent: Box::new(intent),
            }));
        }
        tx.controls.push(Control::Funds(funds::Action::Prepare {
            attempt: a,
            net: cash(i128::from(amount)),
        }));
        if j.commit(tx).map_err(|_| Error)?.receipt.controls.is_some() {
            return Err(Error);
        }
        Ok(())
    }
    fn economics(
        &self,
        j: &mut Store,
        id: u64,
        change: EconomicChange,
        at: u64,
    ) -> Result<(), Error> {
        let mut tx = empty(j, at);
        tx.inputs.push(Input {
            source: self.profile.source,
            source_cut: Some(id),
            authority_epoch: 1,
            observed_at: at,
            raw: PrivateBytes::new(
                b"P22 explicit synthetic funding boundary/rate certificate".to_vec(),
            )
            .map_err(|_| Error)?,
            event: Some(Event {
                key: RecordKey::Economic(key(id)?),
                policy: config().policy,
                change: Change::Economics(change),
            }),
        });
        let result = j.commit(tx).map_err(|_| Error)?;
        if result.receipt.inputs.iter().any(|r| {
            !matches!(
                r,
                InputResult::Normalized(Disposition::Applied | Disposition::Duplicate)
            )
        }) {
            return Err(Error);
        }
        Ok(())
    }
    /// Privileged fake external observations drive actual production controllers.
    pub(super) fn control(&mut self, j: &mut Store, command: Command) -> Result<Value, Error> {
        let at = FixtureClock.now()?;
        match command {
            Command::Deposit {
                amount,
                signature,
                slot,
            } => {
                let signature: [u8; 64] = signature.try_into().map_err(|_| Error)?;
                if signature == [0; 64] || slot == 0 {
                    return Err(Error);
                }
                let source = config().sources[1].scope;
                let mut tx = empty(j, at);
                tx.inputs.push(Input {
                    source,
                    source_cut: Some(slot),
                    authority_epoch: 1,
                    observed_at: at,
                    raw: PrivateBytes::new(
                        b"actual SBF deposit receipt qualified by private TS harness".to_vec(),
                    )
                    .map_err(|_| Error)?,
                    event: Some(Event {
                        key: RecordKey::Economic(EventKey {
                            scope: source,
                            event: EconomicEventId::new(&signature).map_err(|_| Error)?,
                            leg: 0,
                        }),
                        policy: config().policy,
                        change: Change::Receipt {
                            owner: Owner::Customer(user()),
                            location: Location::Vault,
                            amount: cash(i128::from(amount)),
                        },
                    }),
                });
                j.commit(tx).map_err(|_| Error)?;
            }
            Command::Prepare {
                id,
                rail,
                amount,
                counters,
            } => {
                self.prepare(j, id, rail, amount, at)?;
                let action = self
                    .controller
                    .expose_chain(
                        j,
                        Dispatch {
                            attempt: attempt(id)?,
                            commit: recovery::commit_id()?,
                            at,
                        },
                        rail,
                        counters,
                    )
                    .map_err(|_| Error)?;
                let contract = action.encode().map_err(|_| Error)?.as_bytes().to_vec();
                self.pending.insert(id, action);
                return Ok(json!({"contract":contract}));
            }
            Command::Wire {
                id,
                signature,
                wire,
            } => {
                let action = self.pending.remove(&id).ok_or(Error)?;
                let binding = openssl::sha::sha256(action.encode().map_err(|_| Error)?.as_bytes());
                self.controller
                    .persist_wire(
                        j,
                        recovery::commit_id()?,
                        at,
                        action,
                        funding::VerifiedWire {
                            attempt: attempt(id)?,
                            binding,
                            signature: signature.try_into().map_err(|_| Error)?,
                            wire: PrivateBytes::new(wire).map_err(|_| Error)?,
                        },
                    )
                    .map_err(|_| Error)?;
            }
            Command::Chain { receipt: r } => {
                self.controller
                    .observe_chain(
                        j,
                        recovery::commit_id()?,
                        at,
                        funding::ChainReceipt {
                            network: r.network,
                            attempt: attempt(r.id)?,
                            mint: r.mint,
                            program: r.program,
                            source: r.source,
                            destination: r.destination,
                            amount: r.amount,
                            succeeded: true,
                            signature: r.signature.try_into().map_err(|_| Error)?,
                            slot: r.slot,
                            operation: r.operation,
                            epoch: r.epoch,
                            config: r.config,
                            paid: r.paid,
                            sequence: r.sequence,
                            raw: PrivateBytes::new(
                                b"actual SBF receipt and checked wire; fake native deposit rail"
                                    .to_vec(),
                            )
                            .map_err(|_| Error)?,
                        },
                    )
                    .map_err(|_| Error)?;
            }
            Command::Credit {
                id,
                signature,
                event,
                amount,
                final_credit,
            } => {
                self.controller
                    .observe_credit(
                        j,
                        recovery::commit_id()?,
                        at,
                        funding::Credit {
                            attempt: attempt(id)?,
                            account: self.controller.route().broker,
                            deposit_signature: signature.try_into().map_err(|_| Error)?,
                            event: EconomicEventId::new(&event.to_be_bytes()).map_err(|_| Error)?,
                            cut: event,
                            amount,
                            fee: 0,
                            final_credit,
                            raw: PrivateBytes::new(
                                b"explicit synthetic operation-specific native credit".to_vec(),
                            )
                            .map_err(|_| Error)?,
                        },
                    )
                    .map_err(|_| Error)?;
            }
            Command::Withdraw { id, amount } => {
                self.prepare(j, id, Rail::Withdraw, amount, at)?;
                self.controller
                    .withdraw(
                        j,
                        &self.gateway,
                        Dispatch {
                            attempt: attempt(id)?,
                            commit: recovery::commit_id()?,
                            at,
                        },
                        &mut Unknown,
                    )
                    .map_err(|_| Error)?;
            }
            Command::Withdrawal {
                id,
                amount,
                signature,
                slot,
            } => {
                let r = self.controller.route();
                self.controller.observe_withdrawal(j, recovery::commit_id()?, at, funding::Withdrawal { attempt: attempt(id)?, idempotency_key: funding::withdrawal_id(attempt(id)?), event: EconomicEventId::new(&[id]).map_err(|_| Error)?, complete: true, cut: u64::from(id), gross: amount,
                    payment: funding::ChainReceipt { network: config().domain.network.bytes(), attempt: attempt(id)?, mint: r.mint, program: r.venue_program, source: r.venue_vault, destination: r.broker_tokens, amount, succeeded: true, signature: signature.try_into().map_err(|_| Error)?, slot, operation: None, epoch: None, config: None, paid: None, sequence: None, raw: PrivateBytes::new(b"actual local SPL beneficiary transfer for fake native withdrawal".to_vec()).map_err(|_| Error)? }, raw: PrivateBytes::new(b"explicit fake complete UUID-linked native withdrawal history".to_vec()).map_err(|_| Error)? }).map_err(|_| Error)?;
            }
            Command::Trades { body, through } => {
                let body_hash = openssl::sha::sha256(body.as_bytes());
                observation::ingest_covered(
                    j,
                    &self.profile,
                    Message {
                        kind: Kind::Trades,
                        account: self.profile.account.clone(),
                        cursor: None,
                        received_at: at,
                        body,
                    },
                    recovery::commit_id()?,
                    at,
                    Coverage {
                        profile: self.profile.commitment().map_err(|_| Error)?,
                        body: body_hash,
                        through,
                        evidence: [92; 32],
                    },
                )
                .map_err(|_| Error)?;
            }
            Command::Terminal {
                id,
                filled,
                events,
                through,
            } => {
                let mut tx = empty(j, at);
                tx.order_observations.push(orders::Observation {
                    key: key(10_000 + u64::from(id))?,
                    attempt: attempt(id)?,
                    status: orders::Status::Terminal(orders::Terminal {
                        filled: QuantityLots::new(config().markets[0].unit(), filled),
                        executions: events
                            .into_iter()
                            .map(|n| {
                                Ok(EventKey {
                                    scope: self.profile.source,
                                    event: EconomicEventId::new(format!("trade:{n}").as_bytes())
                                        .map_err(|_| Error)?,
                                    leg: 0,
                                })
                            })
                            .collect::<Result<_, Error>>()?,
                        through,
                    }),
                    authority_epoch: 1,
                    observed_at: at,
                    raw: PrivateBytes::new(
                        b"explicit fake complete economic execution set; not cancel ACK".to_vec(),
                    )
                    .map_err(|_| Error)?,
                });
                tx.controls.push(Control::Order(orders::Action::Release {
                    request: attempt(id)?.request,
                }));
                if j.commit(tx).map_err(|_| Error)?.receipt.controls.is_some() {
                    return Err(Error);
                }
            }
            Command::Funding {
                event,
                numerator,
                native,
                settle,
            } => {
                self.economics(
                    j,
                    event,
                    EconomicChange::FundingBoundary {
                        market: config().markets[0].unit(),
                        expected_version: j.state().map_err(|_| Error)?.ledger().version(),
                    },
                    at,
                )?;
                self.economics(
                    j,
                    event + 1,
                    EconomicChange::FundingInputs {
                        boundary: key(event)?,
                        rate: Some(FundingRate {
                            numerator: cash(numerator),
                            denominator: 1,
                            rounding: Rounding::TowardZero,
                        }),
                        native: Some(cash(native)),
                    },
                    at,
                )?;
                if settle {
                    self.economics(
                        j,
                        event + 2,
                        EconomicChange::FundingSettlement {
                            boundary: key(event)?,
                            native: cash(native),
                        },
                        at,
                    )?;
                }
            }
            Command::Reconcile {
                event,
                cash: value,
                quantity,
                basis,
                funding,
            } => {
                let mut tx = empty(j, at);
                tx.inputs.push(Input {
                    source: self.profile.source,
                    source_cut: Some(event),
                    authority_epoch: 1,
                    observed_at: at,
                    raw: PrivateBytes::new(
                        b"independent fake native state, not derived from Cinder".to_vec(),
                    )
                    .map_err(|_| Error)?,
                    event: Some(Event {
                        key: RecordKey::Economic(key(event)?),
                        policy: config().policy,
                        change: Change::Reconcile(NativeCheck {
                            expected_version: j.state().map_err(|_| Error)?.ledger().version(),
                            cash: Some(cash(value)),
                            funding: Some(cash(funding)),
                            positions: Some(vec![
                                Position::new(
                                    QuantityLots::new(config().markets[0].unit(), quantity),
                                    BasisAtoms::new(config().markets[0].unit(), basis),
                                )
                                .map_err(|_| Error)?,
                            ]),
                            complete: true,
                            resolves: vec![],
                        }),
                    }),
                });
                j.commit(tx).map_err(|_| Error)?;
            }
            Command::Oracle => {}
        }
        oracle(j)
    }
}
fn oracle(j: &mut Store) -> Result<Value, Error> {
    let s = j.verified_state().map_err(|_| Error)?;
    let l = s.ledger();
    l.check_bridge().map_err(|_| Error)?;
    let book = |b: &Book| json!({"cash":b.cash().atoms().to_string(),"funding":b.funding().atoms().to_string(),"quantity":b.positions()[0].quantity().lots().to_string(),"basis":b.positions()[0].basis().atoms().to_string()});
    let marks = vec![PriceTicks::new(config().markets[0].unit(), 100).map_err(|_| Error)?];
    let d = l.diagnostics(&marks).map_err(|_| Error)?;
    Ok(
        json!({"ledger_version":l.version(),"customer":book(l.book(Owner::Customer(user())).map_err(|_| Error)?),"house":book(l.book(Owner::House).map_err(|_| Error)?),"native":book(l.venue()),
        "vault":l.vault().atoms().to_string(),"broker":l.broker().atoms().to_string(),"transit":l.in_transit().map_err(|_| Error)?.atoms().to_string(),
        "customer_reserved":s.reserved(Resource::Customer(user())).map_err(|_| Error)?.atoms().to_string(),"venue_reserved":s.reserved(Resource::Location(Location::Venue)).map_err(|_| Error)?.atoms().to_string(),
        "unresolved":s.unresolved_raw(),"active_holds":s.holds().iter().filter(|h| h.active).count(),"exposed":s.attempts().iter().filter(|a| a.possibly_exposed).count(),
        "claims":d.customer_claims.atoms().to_string(),"shortfall":d.shortfall.atoms().to_string(),"frozen":s.frozen(),
        "orders":s.orders().iter().filter_map(|o| o.attempt.map(|a| json!({"id":a.request.request.bytes(),"client_id":cinder_pacifica::client_id(a),"filled":o.filled.lots(),"complete":o.complete(),"cuts":o.executions.iter().map(|(_,cut)|*cut).collect::<Vec<_>>()}))).collect::<Vec<_>>()}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn controls_decode_lossless_canonical_money() {
        let body = r#"{"op":"reconcile","event":1,"cash":"170141183460469231731687303715884105727","quantity":0,"basis":"0","funding":"-1"}"#;
        assert!(matches!(
            serde_json::from_str::<Command>(body),
            Ok(Command::Reconcile {
                cash: i128::MAX,
                funding: -1,
                ..
            })
        ));
        for wrong in [
            "\"-0\"",
            "\"01\"",
            "\"+1\"",
            "1",
            "\"170141183460469231731687303715884105728\"",
        ] {
            assert!(serde_json::from_str::<Command>(&body.replace("\"-1\"", wrong)).is_err());
        }
    }
}
