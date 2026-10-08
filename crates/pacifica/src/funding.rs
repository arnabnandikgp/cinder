//! P16 custody/venue orchestration over the common journal. No RPC client, wallet
//! loader, ambient clock, automatic retry or independent balance ledger.
use crate::{
    Error,
    execution::{Dispatch, Gateway, Outbound, Reply, Transport},
    profile::*,
};
use cinder_journal::{Backend, Journal, Protection, funds, model::*};
use cinder_kernel::{
    amounts::QuoteAtoms,
    codec::Canonical,
    identity::*,
    ledger::{evidence::Disposition, funds::*, *},
};
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

const MAGIC: &[u8] = b"CINDER-PACIFICA-FUNDING-1\0";
/// Narrow legacy Solana instruction/message codec; no RPC or ambient wallet.
pub mod chain;
/// Strict documented setup/transfer wire parsing; not a financial certificate.
pub mod evidence;
pub mod recovery;
/// Governed binding between an opaque private account and its public payout owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Beneficiary {
    /// Private financial identity, not assumed equal to the wallet key.
    pub account: [u8; 32],
    /// Registered P15 wallet owner; fixed across normal/recovery counters.
    pub wallet: [u8; 32],
    /// Authorized token account for the configured quote mint.
    pub tokens: [u8; 32],
}
/// Immutable custody and native qualification. No production defaults exist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Route {
    /// Explicit P15 deployment seed, bound to the Cinder deployment namespace.
    pub domain: [u8; 32],
    /// Explicit P15 pool seed, not an inherited research run identity.
    pub pool: [u8; 32],
    /// P15 funds authority, separate from the broker owner/withdrawal signer.
    pub funds: [u8; 32],
    /// Authenticated initial customer bindings; changes require an explicit migration.
    pub beneficiaries: Vec<Beneficiary>,
    /// P15 deployment identity.
    pub program: [u8; 32],
    /// Canonical P15 config address, verified by the Solana boundary.
    pub config: [u8; 32],
    /// P15 token vault.
    pub vault: [u8; 32],
    /// Classic SPL mint shared by the exact native route.
    pub mint: [u8; 32],
    /// Intermediate wallet; also the owner-directed native withdrawal recipient.
    pub broker: [u8; 32],
    /// Allowlisted broker token account.
    pub broker_tokens: [u8; 32],
    /// Qualified native deposit program, never inferred from a display symbol.
    pub venue_program: [u8; 32],
    /// Qualified native deposit token vault.
    pub venue_vault: [u8; 32],
    /// Configured P15 authority epoch; local rotation is not native key revocation.
    pub epoch: u64,
    /// Exact native/custody atom precision.
    pub decimals: u8,
    /// Explicit owner-directed native withdrawal/final-settlement qualification.
    pub withdrawal: Level,
    /// Actual Solana finality/account/instruction-verification qualification.
    pub chain: Level,
    /// Qualified settings and debt observation semantics.
    pub settings: Level,
    /// Qualified worst-case withdrawal credits, sharing P14's one pool budget.
    pub withdrawal_cost: u32,
    /// Immutable maximum gross working-collateral movement, not perp notional.
    pub maximum_movement: u64,
    /// Explicit maximum quote-asset withdrawal fee.
    pub maximum_fee: u64,
    /// Bound lifetime of source-qualified setup observations, milliseconds.
    pub setup_max_age: u64,
}
/// Supported physical route, not arbitrary CPI or HTTP access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rail {
    /// P15 program vault to configured broker token account.
    Release,
    /// Native deposit program; broker tokens become transit until venue credit.
    Deposit,
    /// Native owner-directed withdrawal; HTTP ACK is not settlement.
    Withdraw,
    /// Broker token account back into P15's vault, without customer credit.
    Return,
    /// P15 authorized payout to the registered customer's token account.
    Payout,
}
/// Exact P15 public-counter preconditions and a chain slot deadline. Slots are
/// never compared with injected millisecond time or native causal cuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counters {
    /// Expected funding sequence or customer's payout sequence.
    pub sequence: u64,
    /// Payout-only expected lifetime ordinary paid amount.
    pub paid: u64,
    /// Exact destination token account for a payout; zero for fixed internal routes.
    pub recipient_tokens: [u8; 32],
    /// Chain deadline selected by the qualified transaction builder.
    pub expires_at_slot: u64,
}
/// Source-qualified setup. This is a trusted observation port, not customer input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Setup {
    /// Exact pooled wallet.
    pub account: [u8; 32],
    /// Source observed time, not a caller's arbitrary future expiry.
    pub observed_at: u64,
    /// Auto-lending is disabled; missing-account/default settings do not imply this.
    pub lending_disabled: bool,
    /// Exact decimal borrowed balance.
    pub borrowed: String,
    /// Exact decimal pending interest.
    pub interest: String,
    /// No unresolved native collateral configuration or in-flight account setup.
    pub complete: bool,
}
/// Persisted dispatch specification. The full attempt remains authoritative even
/// if the opaque UUID/operation hash were ever to collide.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    rail: Rail,
    attempt: Vec<u8>,
    abstract_action: Vec<u8>,
    net: u64,
    gross: u64,
    operation: [u8; 32],
    epoch: u64,
    customer: [u8; 32],
    counters: Counters,
    at: u64,
    expires_at: u64,
}
impl std::fmt::Debug for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FundingPlan([PRIVATE])")
    }
}
impl Plan {
    /// Exact physical rail.
    pub fn rail(&self) -> Rail {
        self.rail
    }
    /// Actual exact gross atoms authorized for this rail.
    pub fn amount(&self) -> u64 {
        self.gross
    }
    /// P15 immutable operator receipt identity.
    pub fn operation(&self) -> [u8; 32] {
        self.operation
    }
    /// Permanent full operation/attempt identity.
    pub fn attempt(&self) -> Result<AttemptKey, Error> {
        Ok(AttemptKey::decode(&self.attempt)?)
    }
    /// Bound P15 counters/recipient/slot preconditions.
    pub fn counters(&self) -> Counters {
        self.counters
    }
    /// Registered public customer wallet for a payout; never a private position.
    pub fn customer(&self) -> [u8; 32] {
        self.customer
    }
    /// Configured authority epoch.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    /// Original local signing deadline in milliseconds, not a chain slot.
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }
    /// Durable plan time; a backwards clock cannot authorize signing.
    pub fn prepared_at(&self) -> u64 {
        self.at
    }
}
/// Non-clone custody capability. The enclave chain boundary builds/signs only
/// this instruction and persists its exact signed wire before sending once.
pub struct ChainAction {
    route: Route,
    plan: Plan,
    network: [u8; 32],
}
/// Exact chain wire produced by the qualified instruction/signature-verification
/// port inside the confidential runtime. This struct is NOT an RPC trust proof.
#[derive(Clone, PartialEq, Eq)]
pub struct VerifiedWire {
    /// Full original private operation/attempt.
    pub attempt: AttemptKey,
    /// SHA-256 of the exact unsigned client contract checked by the chain codec.
    pub binding: [u8; 32],
    /// First transaction signature, independently verified by the chain codec.
    pub signature: [u8; 64],
    /// Exact serialized signed transaction, verified against the planned instruction.
    pub wire: PrivateBytes,
}
impl std::fmt::Debug for VerifiedWire {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("VerifiedWire([PRIVATE])")
    }
}
/// Non-clone signed delivery returned ONLY after durable exact-wire persistence.
pub struct ChainDelivery {
    wire: PrivateBytes,
    network: [u8; 32],
    expires_at: u64,
    expires_at_slot: u64,
}
impl std::fmt::Debug for ChainDelivery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ChainDelivery([PRIVATE])")
    }
}
impl ChainDelivery {
    /// Consume once in the qualified cluster egress port; no redirects or retries.
    pub fn into_wire(self) -> PrivateBytes {
        self.wire
    }
    /// Original configured genesis; a generic chain wire does not encode it.
    pub fn network(&self) -> [u8; 32] {
        self.network
    }
    /// Original local delivery deadline, not reset by persistence or restart.
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }
    /// Original chain instruction deadline, distinct from millisecond time.
    pub fn expires_at_slot(&self) -> u64 {
        self.expires_at_slot
    }
}
impl std::fmt::Debug for ChainAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ChainAction([PRIVATE])")
    }
}
impl ChainAction {
    /// Read-only original plan; cloning a plan does not clone its capability.
    pub fn plan(&self) -> &Plan {
        &self.plan
    }
    /// Encode the bounded unsigned client contract. Retain this one-use capability
    /// until persist_wire consumes it. Every u64 is a decimal
    /// string; JSON/JavaScript cannot round an amount, sequence, epoch or slot.
    /// Private request IDs and abstract journal payloads never cross this seam.
    pub fn encode(&self) -> Result<PrivateBytes, Error> {
        let r = &self.route;
        let p = &self.plan;
        let body = json!({"schema":"cinder-vault-funding-v1", "rail":p.rail,
            "network":self.network,"program":r.program,"domain":r.domain,"pool":r.pool,
            "config":r.config,"vault":r.vault,"mint":r.mint,"funds":r.funds,
            "broker":r.broker,"broker_tokens":r.broker_tokens,"decimals":r.decimals,
            "venue_program":r.venue_program,"venue_vault":r.venue_vault,
            "epoch":p.epoch.to_string(),"operation":p.operation,"customer":if p.rail==Rail::Payout {p.customer} else {[0;32]},
            "amount":p.gross.to_string(),"sequence":p.counters.sequence.to_string(),
            "paid":p.counters.paid.to_string(),"recipient_tokens":p.counters.recipient_tokens,
            "expires_at_slot":p.counters.expires_at_slot.to_string()});
        Ok(PrivateBytes::new(
            serde_json::to_vec(&body).map_err(|_| Error::Codec)?,
        )?)
    }
}
/// Independently authenticated and decoded chain evidence. The chain port must
/// validate network, finalized status/error, exact wire, program/account owners,
/// canonical PDAs, token deltas and P15 receipt data before supplying this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainReceipt {
    /// Independently verified cluster/genesis identity, not an RPC URL string.
    pub network: [u8; 32],
    /// Original persisted attempt.
    pub attempt: AttemptKey,
    /// Bound token-mint identity.
    pub mint: [u8; 32],
    /// Exact bound rail program.
    pub program: [u8; 32],
    /// Exact public source token account.
    pub source: [u8; 32],
    /// Exact destination token account.
    pub destination: [u8; 32],
    /// Actual transferred atoms; zero only for qualified no-effect failure.
    pub amount: u64,
    /// Finalized successful instruction versus atomic finalized no-effect failure.
    pub succeeded: bool,
    /// Exact 64-byte transaction signature; not a history batch nonce.
    pub signature: [u8; 64],
    /// Finalized execution slot in this chain's own causal units.
    pub slot: u64,
    /// P15 receipt operation, None for native deposit/no-effect failure.
    pub operation: Option<[u8; 32]>,
    /// P15 receipt epoch, None for native deposit/no-effect failure.
    pub epoch: Option<u64>,
    /// P15 config identity, None for native deposit/no-effect failure.
    pub config: Option<[u8; 32]>,
    /// Payout-only lifetime paid counter after the actual transfer.
    pub paid: Option<u64>,
    /// Lifetime sequence after payout/release, when applicable.
    pub sequence: Option<u64>,
    /// Retained authenticated raw evidence, never secrets or a signed request.
    pub raw: PrivateBytes,
}
/// Actual source-qualified native credit linked to the exact finalized deposit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credit {
    /// Original deposit attempt.
    pub attempt: AttemptKey,
    /// Exact native wallet, independently observed.
    pub account: [u8; 32],
    /// Exact original Solana deposit transaction, not aggregate balance increase.
    pub deposit_signature: [u8; 64],
    /// Exact native economic receipt identity.
    pub event: EconomicEventId,
    /// Independent native causal cut.
    pub cut: u64,
    /// Actual credited quote amount, allowing partial settlement.
    pub amount: u64,
    /// Qualified actual fee within this gross credit leg.
    pub fee: u64,
    /// Complete operation-specific credit total/no-more-credit evidence.
    pub final_credit: bool,
    /// Original authenticated evidence.
    pub raw: PrivateBytes,
}
/// Reconciliation of an original exposed withdrawal with native debit and actual
/// finalized owner-wallet payment. History completeness/correlation must be
/// independently qualified; empty pages/timeouts cannot construct this witness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Withdrawal {
    /// Exact original attempt, also binding its immutable native idempotency UUID.
    pub attempt: AttemptKey,
    /// Native history's qualified request correlation.
    pub idempotency_key: String,
    /// Native economic withdrawal identity, independent of delivery transport.
    pub event: EconomicEventId,
    /// Native operation-specific no-later-debit/payout evidence is complete.
    pub complete: bool,
    /// Qualified native source cut, never a Solana slot.
    pub cut: u64,
    /// Actual source debit including fee, never copied from an HTTP ACK.
    pub gross: u64,
    /// Actual finalized transfer into the allowlisted broker token account.
    pub payment: ChainReceipt,
    /// Retained authenticated native history, including causal/correlation evidence.
    pub raw: PrivateBytes,
}
#[derive(Serialize, Deserialize)]
enum Record {
    Bind,
    Setup(Setup),
    Plan(Plan),
    Chain {
        attempt: Vec<u8>,
        signature: Vec<u8>,
        succeeded: bool,
    },
    Credit {
        attempt: Vec<u8>,
        final_credit: bool,
    },
    Withdrawal {
        attempt: Vec<u8>,
    },
    Reply {
        attempt: Vec<u8>,
        status: Option<u16>,
    },
    Wire {
        attempt: Vec<u8>,
        signature: Vec<u8>,
        hash: [u8; 32],
    },
    Unsent {
        attempt: Vec<u8>,
    },
}
#[derive(Serialize, Deserialize)]
struct Archive {
    contract: [u8; 32],
    record: Record,
}
// These transactions retain an opaque native reply/signed wire/failed-chain
// body after their typed header. Its bytes must never be interpreted as another
// controller or gateway record, even if they begin with an internal magic tag.
pub(crate) fn opaque_evidence_slot(tx: &Transaction) -> Result<Option<usize>, Error> {
    let Some(bytes) = tx
        .evidence
        .first()
        .and_then(|body| body.as_bytes().strip_prefix(MAGIC))
    else {
        return Ok(None);
    };
    let archive: Archive = serde_json::from_slice(bytes).map_err(|_| Error::Codec)?;
    Ok(matches!(
        archive.record,
        Record::Reply { .. }
            | Record::Wire { .. }
            | Record::Chain {
                succeeded: false,
                ..
            }
    )
    .then_some(1))
}
#[derive(Default)]
struct History {
    bound: bool,
    setup: Option<Setup>,
    plans: Vec<Plan>,
    chains: Vec<(Vec<u8>, Vec<u8>, bool)>,
    final_credits: Vec<Vec<u8>>,
    wires: Vec<(Vec<u8>, Vec<u8>, [u8; 32])>,
    unsent: Vec<Vec<u8>>,
    withdrawal_replies: Vec<(Vec<u8>, Option<evidence::WithdrawalAcknowledgment>)>,
}
/// Derived residual report. Amounts come only from the common ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Residuals {
    /// Actual program-vault tokens.
    pub vault: QuoteAtoms,
    /// Actual intermediate-wallet tokens, not venue margin.
    pub broker: QuoteAtoms,
    /// Signed native cash, excluding unrealized PnL.
    pub native: QuoteAtoms,
    /// Pending transfer receivables, not usable liquidity.
    pub transit: QuoteAtoms,
    /// Durable exposed operations still requiring reconciliation.
    pub unresolved: Vec<AttemptKey>,
}
/// Funding forecast, separate from actual venue margin/admission. Pending ingress
/// avoids double funding but is NEVER an available asset or trading credit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FundingNeed {
    /// Qualified native free collateral after existing commitments.
    pub venue_usable: u64,
    /// Already allocated, unreserved broker tokens.
    pub broker_usable: u64,
    /// Incomplete release/deposit commitments expected to enter the venue.
    pub pending_ingress: u64,
    /// Extra marginal collateral to allocate, not full perp notional.
    pub additional: u64,
}
/// Separately scoped owner-directed withdrawal signer and physical funds controller.
/// Trading Gateway is used only for its common budget/fencing, not its key.
pub struct Controller {
    profile: Profile,
    route: Route,
    key: SigningKey,
    contract: [u8; 32],
}
impl std::fmt::Debug for Controller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FundingController([PRIVATE])")
    }
}
impl Controller {
    /// Forecast a caller's qualified working-collateral target. The financial risk
    /// controller selects the target; this method does not derive leverage from
    /// notional or allow trading against the forecast.
    pub fn funding_need<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        at: u64,
        target: u64,
    ) -> Result<FundingNeed, Error> {
        let history = self.history(j, at)?;
        if !history.bound || target > self.route.maximum_movement {
            return Err(Error::Qualification);
        }
        let state = j.state()?;
        let available = |l| -> Result<u64, Error> {
            u64::try_from(
                state
                    .available_location(l)
                    .map_err(|_| Error::Qualification)?
                    .atoms()
                    .max(0),
            )
            .map_err(|_| Error::Limit)
        };
        let venue_usable = available(Location::Venue)?;
        let broker_usable = available(Location::Broker)?;
        let mut pending_ingress = 0_u64;
        // Prepared mandates already reserve physical liquidity, even before the
        // chain-specific plan is exposed. Count that allocation exactly once.
        for m in state.ledger().movements().iter().filter(|m| {
            matches!(
                (m.mandate.source, m.mandate.destination),
                (Location::Vault, Destination::Location(Location::Broker))
                    | (Location::Broker, Destination::Location(Location::Venue))
            )
        }) {
            let attempt = m.mandate.attempt;
            if state
                .funds()
                .iter()
                .any(|o| o.attempt == Some(attempt) && !o.terminal)
            {
                let remaining = m
                    .mandate
                    .net
                    .checked_sub(m.arrived)
                    .and_then(|x| x.checked_sub(m.returned))
                    .and_then(|x| x.checked_sub(m.impaired))
                    .map_err(|_| Error::Limit)?
                    .atoms()
                    .max(0);
                pending_ingress = pending_ingress
                    .checked_add(u64::try_from(remaining).map_err(|_| Error::Limit)?)
                    .ok_or(Error::Limit)?;
            }
        }
        let projected = venue_usable
            .checked_add(broker_usable)
            .and_then(|x| x.checked_add(pending_ingress))
            .ok_or(Error::Limit)?;
        Ok(FundingNeed {
            venue_usable,
            broker_usable,
            pending_ingress,
            additional: target.saturating_sub(projected),
        })
    }
    /// Explicit injected enclave signer, never a configured wallet or fallback master.
    pub fn new(profile: Profile, route: Route, seed: Zeroizing<[u8; 32]>) -> Result<Self, Error> {
        let key = SigningKey::from_bytes(&seed);
        let keys = [
            route.funds,
            route.domain,
            route.pool,
            route.program,
            route.config,
            route.vault,
            route.mint,
            route.broker,
            route.broker_tokens,
            route.venue_program,
            route.venue_vault,
        ];
        if route.epoch == 0
            || route.domain != profile.config.domain.deployment.bytes()
            || route.funds == route.broker
            || route.beneficiaries.len() != profile.config.customers.len()
            || route.beneficiaries.iter().enumerate().any(|(i, b)| {
                b.wallet == [0; 32]
                    || b.tokens == [0; 32]
                    || !profile
                        .config
                        .customers
                        .iter()
                        .any(|id| id.bytes() == b.account)
                    || route.beneficiaries[..i].iter().any(|old| {
                        old.account == b.account || old.wallet == b.wallet || old.tokens == b.tokens
                    })
            })
            || route.decimals != profile.quote_places
            || route.decimals > 18
            || route.withdrawal != Level::Qualified
            || route.chain != Level::Qualified
            || route.settings != Level::Qualified
            || route.maximum_movement == 0
            || route.maximum_fee >= route.maximum_movement
            || route.setup_max_age == 0
            || route.withdrawal_cost == 0
            || keys.contains(&[0; 32])
            || route.vault == route.broker_tokens
            || route.vault == route.venue_vault
            || route.broker_tokens == route.venue_vault
            || key.verifying_key().to_bytes() != route.broker
            || bs58::encode(route.broker).into_string() != profile.account
            || !profile
                .config
                .sources
                .iter()
                .any(|s| s.location == Location::Broker)
            || !profile
                .config
                .sources
                .iter()
                .any(|s| s.location == Location::Vault)
        {
            return Err(Error::Qualification);
        }
        let mut h = Sha256::new();
        h.update(MAGIC);
        h.update(profile.commitment()?);
        h.update(serde_json::to_vec(&route).map_err(|_| Error::Codec)?);
        Ok(Self {
            profile,
            route,
            key,
            contract: h.finalize().into(),
        })
    }
    /// Release component from the actual loaded profile, custody route and signer.
    /// Refuse a controller belonging to a different authoritative journal.
    pub fn release_commitment(&self, config: &Config) -> Result<[u8; 32], Error> {
        if &self.profile.config != config {
            return Err(Error::Qualification);
        }
        Ok(self.contract)
    }
    /// Private-runtime route access for checking loaded owner/key-role bindings.
    /// No seed is returned and this must not become a customer API response.
    pub fn route(&self) -> &Route {
        &self.route
    }
    /// Sign only this broker's exact prepared deposit/return message. No seed
    /// export, arbitrary message signing or HTTP master-key fallback is offered.
    pub fn sign_chain(
        &self,
        prepared: chain::Prepared<'_>,
        simulation: chain::Simulation,
        at: u64,
    ) -> Result<VerifiedWire, Error> {
        if prepared.signer() != self.route.broker {
            return Err(Error::Qualification);
        }
        prepared.sign(Zeroizing::new(self.key.to_bytes()), simulation, at)
    }
    fn evidence(&self, record: Record) -> Result<PrivateBytes, Error> {
        let mut bytes = MAGIC.to_vec();
        bytes.extend(
            serde_json::to_vec(&Archive {
                contract: self.contract,
                record,
            })
            .map_err(|_| Error::Codec)?,
        );
        Ok(PrivateBytes::new(bytes)?)
    }
    fn tx<B: Backend, P: Protection>(
        &self,
        j: &Journal<B, P>,
        id: CommitId,
        at: u64,
        record: Record,
    ) -> Result<Transaction, Error> {
        Ok(Transaction {
            id,
            expected: j.head(),
            at,
            evidence: vec![self.evidence(record)?],
            inputs: vec![],
            order_observations: vec![],
            funds_observations: vec![],
            controls: vec![],
        })
    }
    fn history<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        at: u64,
    ) -> Result<History, Error> {
        j.verified_state()?;
        if j.configuration() != &self.profile.config {
            return Err(Error::Qualification);
        }
        let mut result = History::default();
        for (tx, receipt) in j.transactions_with_receipts() {
            if tx.at > at {
                return Err(Error::Qualification);
            }
            let opaque = opaque_evidence_slot(tx)?;
            for (index, bytes) in tx.evidence.iter().enumerate() {
                if opaque == Some(index) {
                    continue;
                }
                if let Some(bytes) = bytes.as_bytes().strip_prefix(MAGIC) {
                    let archive: Archive =
                        serde_json::from_slice(bytes).map_err(|_| Error::Codec)?;
                    if archive.contract != self.contract {
                        return Err(Error::Qualification);
                    }
                    // Retained evidence from a rejected financial proposal is
                    // still evidence, but cannot authorize a later dispatch.
                    if receipt.controls.is_some()
                        || receipt.inputs.iter().any(|i| {
                            !matches!(
                                i,
                                InputResult::Normalized(
                                    Disposition::Applied | Disposition::Duplicate
                                )
                            )
                        })
                        || receipt.funds_observations.iter().any(|ok| !ok)
                    {
                        continue;
                    }
                    match archive.record {
                        Record::Reply { attempt, status } => {
                            if index != 0 || opaque != Some(1) {
                                return Err(Error::Qualification);
                            }
                            let raw = tx.evidence.get(1).ok_or(Error::Codec)?;
                            let ack = (status == Some(200))
                                .then(|| {
                                    evidence::withdrawal_acknowledgment(
                                        &self.profile,
                                        raw.as_bytes(),
                                    )
                                    .ok()
                                })
                                .flatten();
                            result.withdrawal_replies.push((attempt, ack));
                        }
                        Record::Unsent { attempt } => result.unsent.push(attempt),
                        Record::Wire {
                            attempt,
                            signature,
                            hash,
                        } => result.wires.push((attempt, signature, hash)),
                        Record::Bind => {
                            if result.bound {
                                return Err(Error::Qualification);
                            }
                            result.bound = true;
                        }
                        Record::Setup(setup) => result.setup = Some(setup),
                        Record::Plan(plan) => result.plans.push(plan),
                        Record::Chain {
                            attempt,
                            signature,
                            succeeded,
                        } => result.chains.push((attempt, signature, succeeded)),
                        Record::Credit {
                            attempt,
                            final_credit,
                        } if final_credit => result.final_credits.push(attempt),
                        _ => {}
                    }
                }
            }
        }
        Ok(result)
    }
    fn ready(&self, history: &History, at: u64) -> bool {
        history.bound
            && history.setup.as_ref().is_some_and(|s| {
                s.complete
                    && s.lending_disabled
                    && s.observed_at <= at
                    && at - s.observed_at <= self.route.setup_max_age
                    && Grid {
                        places: self.route.decimals,
                        step: 1,
                    }
                    .parse(&s.borrowed)
                        == Ok(0)
                    && Grid {
                        places: self.route.decimals,
                        step: 1,
                    }
                    .parse(&s.interest)
                        == Ok(0)
            })
    }
    /// Read whether this exact route is installed in verified journal history.
    pub fn bound<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        at: u64,
    ) -> Result<bool, Error> {
        Ok(self.history(j, at)?.bound)
    }
    /// Trusted route install. It fences native risk until source-qualified setup;
    /// contract changes cannot reinterpret a prior operation or reset its identity.
    pub fn bind<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        id: CommitId,
        at: u64,
    ) -> Result<(), Error> {
        if self.history(j, at)?.bound {
            return Err(Error::Qualification);
        }
        let mut tx = self.tx(j, id, at, Record::Bind)?;
        tx.controls
            .push(Control::Funds(funds::Action::NativeCreditReady(false)));
        checked(j.commit(tx)?)?;
        Ok(())
    }
    /// Record actual settings even when they fail. Fresh-account deposit-first
    /// lending exceptions are NOT automatically promoted from research permission.
    pub fn observe_setup<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        id: CommitId,
        at: u64,
        setup: Setup,
    ) -> Result<bool, Error> {
        let mut history = self.history(j, at)?;
        if !history.bound
            || setup.account != self.route.broker
            || setup.observed_at > at
            || history
                .setup
                .as_ref()
                .is_some_and(|old| setup.observed_at < old.observed_at)
        {
            return Err(Error::Qualification);
        }
        history.setup = Some(setup.clone());
        let ready = self.ready(&history, at)
            && history
                .plans
                .iter()
                .filter(|p| p.rail == Rail::Deposit)
                .all(|p| {
                    history.final_credits.contains(&p.attempt)
                        || history
                            .chains
                            .iter()
                            .any(|(a, _, ok)| a == &p.attempt && !ok)
                });
        let mut tx = self.tx(j, id, at, Record::Setup(setup))?;
        tx.controls
            .push(Control::Funds(funds::Action::NativeCreditReady(ready)));
        checked(j.commit(tx)?)?;
        Ok(ready)
    }
    fn plan<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        dispatch: Dispatch,
        rail: Rail,
        counters: Counters,
    ) -> Result<Plan, Error> {
        let history = self.history(j, dispatch.at)?;
        if !history.bound
            || matches!(rail, Rail::Release | Rail::Deposit) && !self.ready(&history, dispatch.at)
        {
            return Err(Error::Qualification);
        }
        let state = j.state()?;
        let a = state
            .attempts()
            .iter()
            .find(|a| a.key == dispatch.attempt)
            .ok_or(Error::Qualification)?;
        let movement = state
            .ledger()
            .movements()
            .iter()
            .find(|m| m.mandate.attempt == a.key)
            .ok_or(Error::Qualification)?;
        let m = &movement.mandate;
        let beneficiary = self
            .route
            .beneficiaries
            .iter()
            .find(|b| b.account == dispatch.attempt.request.account.bytes())
            .ok_or(Error::Qualification)?;
        let (source, destination) = match rail {
            Rail::Release => (Location::Vault, Destination::Location(Location::Broker)),
            Rail::Deposit => (Location::Broker, Destination::Location(Location::Venue)),
            Rail::Withdraw => (Location::Venue, Destination::Location(Location::Broker)),
            Rail::Return => (Location::Broker, Destination::Location(Location::Vault)),
            Rail::Payout => (Location::Vault, Destination::Recipient(beneficiary.tokens)),
        };
        let net = u64::try_from(m.net.atoms()).map_err(|_| Error::Limit)?;
        let fee = u64::try_from(m.maximum_fee.atoms()).map_err(|_| Error::Limit)?;
        let gross = net.checked_add(fee).ok_or(Error::Limit)?;
        if a.kind != AttemptKind::Funds
            || a.possibly_exposed
            || a.expires_at <= dispatch.at
            || m.source != source
            || m.destination != destination
            || gross > self.route.maximum_movement
            || fee > self.route.maximum_fee
            || rail != Rail::Withdraw && fee != 0
            || counters.expires_at_slot == 0
            || rail == Rail::Payout && counters.recipient_tokens != beneficiary.tokens
            || rail != Rail::Payout && (counters.recipient_tokens != [0; 32] || counters.paid != 0)
            || history.plans.iter().any(|p| p.attempt == a.key.encode())
        {
            return Err(Error::Qualification);
        }
        let mut h = Sha256::new();
        h.update(b"CINDER-CUSTODY-OPERATION-1\0");
        h.update(self.contract);
        h.update(a.key.encode());
        let operation = h.finalize().into();
        if history.plans.iter().any(|p| p.operation == operation) {
            return Err(Error::Qualification);
        }
        if rail == Rail::Payout
            && u64::try_from(
                state
                    .ledger()
                    .paid(a.key.request.account)
                    .map_err(|_| Error::Qualification)?
                    .atoms(),
            )
            .map_err(|_| Error::Limit)?
                != counters.paid
        {
            return Err(Error::Qualification);
        }
        Ok(Plan {
            rail,
            attempt: a.key.encode(),
            abstract_action: a.message.as_bytes().to_vec(),
            net,
            gross,
            operation,
            // Ordinary route stays immutable. Only a recovery-tagged return uses
            // the one frozen custody epoch; no release/payout can borrow it.
            epoch: if state
                .funds()
                .iter()
                .any(|o| o.recovery && o.attempt == Some(a.key))
                && rail == Rail::Return
            {
                self.route.epoch.checked_add(1).ok_or(Error::Limit)?
            } else {
                self.route.epoch
            },
            customer: beneficiary.wallet,
            counters,
            at: dispatch.at,
            expires_at: a.expires_at,
        })
    }
    /// Persist and expose one unsigned physical-rail capability. The receiving
    /// chain port must persist exact signed bytes before submission; after restart
    /// the plan is reconciliation-only, never permission to build a second wire.
    pub fn expose_chain<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        dispatch: Dispatch,
        rail: Rail,
        counters: Counters,
    ) -> Result<ChainAction, Error> {
        if rail == Rail::Withdraw || j.transaction(dispatch.commit).is_some() {
            return Err(Error::Qualification);
        }
        let plan = self.plan(j, dispatch, rail, counters)?;
        let mut tx = self.tx(j, dispatch.commit, dispatch.at, Record::Plan(plan.clone()))?;
        tx.controls.push(Control::Expose(dispatch.attempt));
        if rail == Rail::Deposit {
            tx.controls
                .push(Control::Funds(funds::Action::NativeCreditReady(false)));
        }
        exposure(j.commit(tx)?, &plan)?;
        j.verified_state()?;
        Ok(ChainAction {
            route: self.route.clone(),
            plan,
            network: self.profile.config.domain.network.bytes(),
        })
    }
    /// Sign/send one native withdrawal against an already prepared funds mandate.
    /// HTTP replies are retained but never post cash or release commitments.
    pub fn withdraw<B: Backend, P: Protection, T: Transport>(
        &self,
        j: &mut Journal<B, P>,
        gateway: &Gateway,
        dispatch: Dispatch,
        transport: &mut T,
    ) -> Result<(), Error> {
        let counters = Counters {
            sequence: 0,
            paid: 0,
            recipient_tokens: [0; 32],
            expires_at_slot: 1,
        };
        let mut plan = self.plan(j, dispatch, Rail::Withdraw, counters)?;
        let expiry = gateway.funding_expiry(plan.expires_at - dispatch.at);
        plan.expires_at = dispatch.at.checked_add(expiry).ok_or(Error::Limit)?;
        if dispatch.at == 0
            || dispatch
                .at
                .checked_add(expiry)
                .is_none_or(|t| t > 9_007_199_254_740_991)
        {
            return Err(Error::Qualification);
        }
        let data = json!({"amount": Grid {places: self.route.decimals,step:1}.format(plan.gross)?, "idempotency_key":withdrawal_id(dispatch.attempt)});
        let preimage = serde_json::to_string(
            &json!({"data":data,"expiry_window":expiry,"timestamp":dispatch.at,"type":"withdraw"}),
        )
        .map_err(|_| Error::Codec)?;
        // The exact timestamp/expiry and amount are immutable plan fields; no new
        // UUID or time can be minted for a possibly exposed attempt.
        let mut tx = gateway.funding_transaction(
            j,
            dispatch.commit,
            dispatch.at,
            &self.profile,
            self.route.withdrawal_cost,
        )?;
        tx.evidence.push(self.evidence(Record::Plan(plan.clone()))?);
        tx.controls.push(Control::Expose(dispatch.attempt));
        exposure(j.commit(tx)?, &plan)?;
        gateway.funding_current(j, dispatch.at)?;
        j.state()?
            .qualified_funds_delivery(dispatch.attempt)
            .map_err(|_| Error::Qualification)?;
        let mut body = data.as_object().ok_or(Error::Codec)?.clone();
        body.insert("account".into(), json!(self.profile.account));
        body.insert(
            "signature".into(),
            json!(bs58::encode(self.key.sign(preimage.as_bytes()).to_bytes()).into_string()),
        );
        body.insert("timestamp".into(), json!(dispatch.at));
        body.insert("expiry_window".into(), json!(expiry));
        let reply = transport.post(Outbound::funding(
            self.profile.environment.as_str(),
            PrivateBytes::new(serde_json::to_vec(&body).map_err(|_| Error::Codec)?)?,
        )?);
        let (status, raw, at, retry) = match reply {
            Reply::Unknown => (
                None,
                PrivateBytes::new(b"withdrawal outcome unknown".to_vec())?,
                dispatch.at,
                None,
            ),
            Reply::Response {
                status,
                body,
                received_at,
                retry_after_ms,
            } => (
                Some(status),
                if body.as_bytes().len() <= crate::observation::MAX_BODY
                    && received_at >= dispatch.at
                {
                    body
                } else {
                    PrivateBytes::new(b"withdrawal response rejected: bound or clock".to_vec())?
                },
                received_at.max(dispatch.at),
                retry_after_ms,
            ),
        };
        let mut h = Sha256::new();
        h.update(b"CINDER-FUNDING-REPLY-1\0");
        h.update(dispatch.commit.bytes());
        let mut tx = self.tx(
            j,
            CommitId::new(h.finalize().into())?,
            at,
            Record::Reply {
                attempt: plan.attempt,
                status,
            },
        )?;
        tx.evidence.push(raw);
        if status == Some(429) {
            tx.evidence.push(gateway.funding_cooldown(at, retry)?);
        }
        checked(j.commit(tx)?)?;
        Ok(())
    }
    /// Rebuild the original request -> batch binding from the durably retained
    /// authenticated response. This neither posts cash nor authorizes resending.
    /// A missing/failed/malformed reply stays unresolved, including after restart.
    pub fn withdrawal_acknowledgment<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        attempt: AttemptKey,
        at: u64,
    ) -> Result<Option<evidence::WithdrawalAcknowledgment>, Error> {
        let history = self.history(j, at)?;
        let original = attempt.encode();
        let plan = history
            .plans
            .iter()
            .find(|p| p.attempt == original && p.rail == Rail::Withdraw)
            .ok_or(Error::Qualification)?;
        let mut replies = history
            .withdrawal_replies
            .iter()
            .filter(|(a, _)| a == &original);
        let ack = replies.next().and_then(|(_, ack)| ack.as_ref());
        if replies.next().is_some() {
            return Err(Error::Qualification);
        }
        let Some(ack) = ack else { return Ok(None) };
        if ack.requested != plan.gross
            || history.withdrawal_replies.iter().any(|(other, reply)| {
                other != &original && reply.as_ref().is_some_and(|r| r.batch == ack.batch)
            })
        {
            // A batch shared by several local requests is ambiguous. Do not
            // select one by amount/time or silently assign an aggregate payment.
            return Err(Error::Qualification);
        }
        // An advertised fee above the mandate remains identifiable evidence;
        // qualification/payment must record real overruns, not erase their link.
        Ok(Some(ack.clone()))
    }
    /// Correlate an authenticated transfer event to an acknowledged original
    /// request. No source frontier, terminal debit or finalized payment is created.
    /// A changed fee remains visible in the observation, not overwritten by the ACK.
    pub fn linked_withdrawal_transfer<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        attempt: AttemptKey,
        message: evidence::TransferMessage<'_>,
    ) -> Result<evidence::TransferObservation, Error> {
        let ack = self
            .withdrawal_acknowledgment(j, attempt, message.received_at)?
            .ok_or(Error::Qualification)?;
        let plan = self.existing(j, attempt, message.received_at)?;
        let observation = evidence::transfer(
            &self.profile,
            message.quote_symbol,
            message.bytes,
            message.received_at,
            message.maximum_age_ms,
        )?;
        if !matches!(
            &observation,
            evidence::TransferObservation::Withdrawal { batch, requested, at, .. }
                if *batch == ack.batch && *requested == ack.requested && *at >= plan.at
        ) {
            return Err(Error::Qualification);
        }
        Ok(observation)
    }
    /// Correlate an authenticated deposit event to the original, successfully
    /// finalized chain deposit. A partial/whole amount is still an observation:
    /// this does not infer final credit totals, fees or a native causal frontier.
    pub fn linked_deposit_transfer<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        attempt: AttemptKey,
        message: evidence::TransferMessage<'_>,
    ) -> Result<evidence::TransferObservation, Error> {
        let history = self.history(j, message.received_at)?;
        let original = attempt.encode();
        let plan = history
            .plans
            .iter()
            .find(|p| p.attempt == original && p.rail == Rail::Deposit)
            .ok_or(Error::Qualification)?;
        let observation = evidence::transfer(
            &self.profile,
            message.quote_symbol,
            message.bytes,
            message.received_at,
            message.maximum_age_ms,
        )?;
        if !matches!(
            &observation,
            evidence::TransferObservation::Deposit { signature, amount, at }
                if *at >= plan.at && *amount <= plan.gross && history.chains.iter().any(
                    |(a, s, ok)| a == &original && s.as_slice() == signature && *ok
                )
        ) {
            return Err(Error::Qualification);
        }
        Ok(observation)
    }
    fn existing<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        attempt: AttemptKey,
        at: u64,
    ) -> Result<Plan, Error> {
        self.history(j, at)?
            .plans
            .into_iter()
            .find(|p| p.attempt == attempt.encode())
            .ok_or(Error::Qualification)
    }
    /// Read the original unsigned contract for verification/reconciliation only.
    /// This is not a delivery capability and cannot authorize a second signed wire.
    pub fn original_chain_contract<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        attempt: AttemptKey,
        at: u64,
    ) -> Result<PrivateBytes, Error> {
        let plan = self.existing(j, attempt, at)?;
        if plan.rail == Rail::Withdraw {
            return Err(Error::Qualification);
        }
        ChainAction {
            route: self.route.clone(),
            plan,
            network: self.profile.config.domain.network.bytes(),
        }
        .encode()
    }
    /// Locate and re-verify the one originally persisted wire. After restart
    /// this is evidence only, never a ChainDelivery or authority to sign again.
    pub fn retained_wire<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        attempt: AttemptKey,
        at: u64,
    ) -> Result<Option<VerifiedWire>, Error> {
        let history = self.history(j, at)?;
        let matches: Vec<_> = history
            .wires
            .iter()
            .filter(|(a, _, _)| *a == attempt.encode())
            .collect();
        if matches.len() > 1 {
            return Err(Error::Qualification);
        }
        let Some((_, signature, hash)) = matches.first().copied() else {
            return Ok(None);
        };
        let contract = self.original_chain_contract(j, attempt, at)?;
        let mut found = None;
        for tx in j.transactions() {
            for bytes in &tx.evidence {
                if <[u8; 32]>::from(Sha256::digest(bytes.as_bytes())) == *hash {
                    let verified =
                        chain::inspect(contract.as_bytes(), attempt, bytes.as_bytes())?.verified;
                    if verified.signature.as_slice() != signature || found.is_some() {
                        return Err(Error::Qualification);
                    }
                    found = Some(verified);
                }
            }
        }
        found.map(Some).ok_or(Error::Qualification)
    }
    /// Persist the verified ORIGINAL signed chain wire before any submission.
    /// Lost/uncertain commits return no delivery. Restart may only reconcile the
    /// retained wire/signature/operation, not create a different blockhash/signature.
    pub fn persist_wire<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        id: CommitId,
        at: u64,
        action: ChainAction,
        wire: VerifiedWire,
    ) -> Result<ChainDelivery, Error> {
        let p = self.existing(j, wire.attempt, at)?;
        let history = self.history(j, at)?;
        j.state()?
            .qualified_funds_delivery(wire.attempt)
            .map_err(|_| Error::Qualification)?;
        if matches!(p.rail, Rail::Release | Rail::Deposit) && !self.ready(&history, at)
            || action.plan != p
            || action.route != self.route
            || action.network != self.profile.config.domain.network.bytes()
            || p.rail == Rail::Withdraw
            || at >= p.expires_at
            || history.wires.iter().any(|(a, _, _)| *a == p.attempt)
            || wire.signature == [0; 64]
            || wire.wire.as_bytes().is_empty()
            || wire.wire.as_bytes().len() > 1232
            || j.transaction(id).is_some()
        {
            return Err(Error::Qualification);
        }
        let contract = action.encode()?;
        let binding: [u8; 32] = Sha256::digest(contract.as_bytes()).into();
        if binding != wire.binding {
            return Err(Error::Qualification);
        }
        let hash = Sha256::digest(wire.wire.as_bytes()).into();
        let mut tx = self.tx(
            j,
            id,
            at,
            Record::Wire {
                attempt: p.attempt,
                signature: wire.signature.to_vec(),
                hash,
            },
        )?;
        tx.evidence.push(wire.wire.clone());
        let result = j.commit(tx)?;
        if result.duplicate {
            return Err(Error::Qualification);
        }
        checked(result)?;
        j.verified_state()?
            .qualified_funds_delivery(wire.attempt)
            .map_err(|_| Error::Qualification)?;
        Ok(ChainDelivery {
            wire: wire.wire,
            network: action.network,
            expires_at: p.expires_at,
            expires_at_slot: p.counters.expires_at_slot,
        })
    }
    /// Close an expired physical plan only when verified journal history proves
    /// no send capability was ever retained. Expiry alone cannot close a retained
    /// wire, and this rule never applies to native HTTP withdrawals.
    pub fn close_unsent_chain<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        id: CommitId,
        at: u64,
        attempt: AttemptKey,
    ) -> Result<bool, Error> {
        let history = self.history(j, at)?;
        let plans = history
            .plans
            .iter()
            .filter(|p| p.attempt == attempt.encode())
            .collect::<Vec<_>>();
        if !history.bound || plans.len() != 1 || plans[0].rail == Rail::Withdraw {
            return Err(Error::Qualification);
        }
        let plan = plans[0];
        if at < plan.expires_at || history.wires.iter().any(|(a, _, _)| *a == plan.attempt) {
            return Ok(false);
        }
        let state = j.verified_state()?;
        let operation = state
            .funds()
            .iter()
            .find(|o| o.attempt == Some(attempt))
            .ok_or(Error::Qualification)?;
        let movement = state
            .ledger()
            .movements()
            .iter()
            .find(|m| m.mandate.attempt == attempt)
            .ok_or(Error::Qualification)?;
        if operation.faulted
            || movement.faulted
            || movement.debit.atoms() != 0
            || movement.settled.atoms() != 0
            || !movement.receipts.is_empty()
            || history.chains.iter().any(|(a, _, _)| *a == plan.attempt)
            || history.final_credits.contains(&plan.attempt)
            || operation.proof.as_ref().is_some_and(|p| {
                p.debit.atoms() != 0
                    || p.settled.atoms() != 0
                    || !p.receipts.is_empty()
                    || !p.coverage.is_empty()
            })
        {
            return Err(Error::Qualification);
        }
        if operation.terminal {
            return if history.unsent.contains(&plan.attempt) {
                Ok(true)
            } else {
                Err(Error::Qualification)
            };
        }
        if !state
            .attempts()
            .iter()
            .any(|a| a.key == attempt && a.kind == AttemptKind::Funds && a.possibly_exposed)
            || j.transaction(id).is_some()
        {
            return Err(Error::Qualification);
        }
        let mut tx = self.tx(
            j,
            id,
            at,
            Record::Unsent {
                attempt: plan.attempt.clone(),
            },
        )?;
        self.terminal(&mut tx, plan, 0, vec![], vec![])?;
        tx.funds_observations[0].raw = self.evidence(Record::Unsent {
            attempt: plan.attempt.clone(),
        })?;
        checked(j.commit(tx)?)?;
        if !j
            .verified_state()?
            .funds()
            .iter()
            .any(|o| o.attempt == Some(attempt) && o.terminal && !o.faulted)
        {
            return Err(Error::Qualification);
        }
        Ok(true)
    }
    fn scope(&self, location: Location) -> Result<EventScope, Error> {
        self.profile
            .config
            .sources
            .iter()
            .find(|s| s.location == location)
            .map(|s| s.scope)
            .ok_or(Error::Qualification)
    }
    fn input(
        &self,
        receipt: (EventKey, u64, u64),
        movement: (AttemptKey, Leg, u64, u64),
        raw: PrivateBytes,
    ) -> Input {
        let (key, cut, at) = receipt;
        let (attempt, leg, amount, fee) = movement;
        Input {
            source: key.scope,
            source_cut: Some(cut),
            authority_epoch: self.profile.revision,
            observed_at: at,
            raw,
            event: Some(Event {
                key: RecordKey::Economic(key),
                policy: self.profile.config.policy,
                change: Change::Funds(FundsChange::Observe {
                    attempt,
                    leg,
                    amount: QuoteAtoms::new(self.profile.config.quote, i128::from(amount)),
                    fee: QuoteAtoms::new(self.profile.config.quote, i128::from(fee)),
                }),
            }),
        }
    }
    fn terminal(
        &self,
        tx: &mut Transaction,
        plan: &Plan,
        debit: u64,
        receipts: Vec<EventKey>,
        coverage: Vec<funds::Coverage>,
    ) -> Result<(), Error> {
        let key = EventKey {
            scope: self.profile.source,
            event: EconomicEventId::new(
                &[b"funding-terminal:".as_slice(), &plan.operation].concat(),
            )?,
            leg: 0,
        };
        tx.funds_observations.push(funds::Observation {
            key,
            terminal: funds::Terminal {
                attempt: plan.attempt()?,
                debit: QuoteAtoms::new(self.profile.config.quote, i128::from(debit)),
                settled: QuoteAtoms::new(self.profile.config.quote, i128::from(debit)),
                receipts,
                coverage,
                no_later_execution: plan.operation,
            },
            authority_epoch: self.profile.revision,
            observed_at: tx.at,
            raw: PrivateBytes::new(b"qualified immutable operation settlement".to_vec())?,
        });
        tx.controls.push(Control::Funds(funds::Action::Finalize(
            plan.attempt()?.request,
        )));
        Ok(())
    }
    /// Record actual finalized custody effects atomically. Native deposit posts only
    /// broker debit; a later independently correlated venue credit completes it.
    pub fn observe_chain<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        id: CommitId,
        at: u64,
        r: ChainReceipt,
    ) -> Result<(), Error> {
        let plan = self.existing(j, r.attempt, at)?;
        if plan.rail == Rail::Withdraw {
            return Err(Error::Qualification);
        }
        let (source, destination, program, from, to) = match plan.rail {
            Rail::Release => (
                Location::Vault,
                Destination::Location(Location::Broker),
                self.route.program,
                self.route.vault,
                self.route.broker_tokens,
            ),
            Rail::Deposit => (
                Location::Broker,
                Destination::Location(Location::Venue),
                self.route.venue_program,
                self.route.broker_tokens,
                self.route.venue_vault,
            ),
            Rail::Return => (
                Location::Broker,
                Destination::Location(Location::Vault),
                self.route.program,
                self.route.broker_tokens,
                self.route.vault,
            ),
            Rail::Payout => (
                Location::Vault,
                Destination::Recipient(plan.counters.recipient_tokens),
                self.route.program,
                self.route.vault,
                plan.counters.recipient_tokens,
            ),
            Rail::Withdraw => return Err(Error::Qualification),
        };
        if r.network != self.profile.config.domain.network.bytes()
            || r.mint != self.route.mint
            || r.program != program
            || r.source != from
            || r.destination != to
            || r.signature == [0; 64]
            || r.succeeded && r.amount != plan.gross
            || !r.succeeded && r.amount != 0
            || r.succeeded
                && plan.rail != Rail::Deposit
                && (r.operation != Some(plan.operation)
                    || r.epoch != Some(plan.epoch)
                    || r.config != Some(self.route.config))
            || r.succeeded
                && matches!(plan.rail, Rail::Release | Rail::Payout)
                && r.sequence != plan.counters.sequence.checked_add(1)
            || r.succeeded
                && plan.rail == Rail::Payout
                && r.paid != plan.counters.paid.checked_add(r.amount)
        {
            return Err(Error::Qualification);
        }
        let history = self.history(j, at)?;
        if !history
            .wires
            .iter()
            .any(|(a, s, _)| a == &plan.attempt && s == &r.signature)
        {
            return Err(Error::Qualification);
        }
        if history
            .chains
            .iter()
            .any(|(a, s, ok)| a == &plan.attempt && (s != &r.signature || *ok != r.succeeded))
        {
            return Err(Error::Qualification);
        }
        let mut tx = self.tx(
            j,
            id,
            at,
            Record::Chain {
                attempt: plan.attempt.clone(),
                signature: r.signature.to_vec(),
                succeeded: r.succeeded,
            },
        )?;
        let scope = self.scope(source)?;
        let mut receipts = vec![];
        if r.succeeded {
            let debit = EventKey {
                scope,
                event: EconomicEventId::new(&r.signature)?,
                leg: 0,
            };
            receipts.push(debit.clone());
            tx.inputs.push(self.input(
                (debit, r.slot, at),
                (r.attempt, Leg::Debit, r.amount, 0),
                r.raw.clone(),
            ));
            if plan.rail != Rail::Deposit {
                let to_location = match destination {
                    Destination::Location(l) => l,
                    Destination::Recipient(_) => source,
                };
                let credit = EventKey {
                    scope: self.scope(to_location)?,
                    event: EconomicEventId::new(&r.signature)?,
                    leg: 1,
                };
                receipts.push(credit.clone());
                tx.inputs.push(self.input(
                    (credit, r.slot, at),
                    (r.attempt, Leg::Arrive(destination), r.amount, 0),
                    r.raw,
                ));
            }
        } else {
            tx.evidence.push(r.raw);
        }
        if plan.rail != Rail::Deposit || !r.succeeded {
            let coverage = receipts
                .iter()
                .map(|k| funds::Coverage {
                    source: k.scope,
                    through: r.slot,
                })
                .fold(vec![], |mut v, c| {
                    if !v.iter().any(|old: &funds::Coverage| old.source == c.source) {
                        v.push(c);
                    }
                    v
                });
            if !j
                .state()?
                .funds()
                .iter()
                .any(|o| o.attempt == Some(r.attempt) && o.terminal)
            {
                self.terminal(&mut tx, &plan, r.amount, receipts, coverage)?;
            }
        }
        let committed = j.commit(tx)?;
        checked(committed)?;
        Ok(())
    }
    /// Apply operation-linked actual credit, never aggregate balance/ACK credit.
    /// Partial credits retain the remaining transit and block native new risk.
    pub fn observe_credit<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        id: CommitId,
        at: u64,
        c: Credit,
    ) -> Result<(), Error> {
        let plan = self.existing(j, c.attempt, at)?;
        let history = self.history(j, at)?;
        if plan.rail != Rail::Deposit
            || c.account != self.route.broker
            || c.amount == 0
            || c.fee > c.amount
            || !history
                .chains
                .iter()
                .any(|(a, s, ok)| a == &plan.attempt && *ok && s == &c.deposit_signature)
        {
            return Err(Error::Qualification);
        }
        let key = EventKey {
            scope: self.profile.source,
            event: c.event,
            leg: 1,
        };
        let movement = j
            .state()?
            .ledger()
            .movements()
            .iter()
            .find(|m| m.mandate.attempt == c.attempt)
            .ok_or(Error::Qualification)?;
        let duplicate = movement.receipts.contains(&key);
        let settled = u64::try_from(movement.settled.atoms())
            .map_err(|_| Error::Limit)?
            .checked_add(if duplicate { 0 } else { c.amount })
            .ok_or(Error::Limit)?;
        if c.final_credit && settled != plan.gross {
            return Err(Error::Qualification);
        }
        let mut tx = self.tx(
            j,
            id,
            at,
            Record::Credit {
                attempt: plan.attempt.clone(),
                final_credit: c.final_credit,
            },
        )?;
        tx.inputs.push(self.input(
            (key.clone(), c.cut, at),
            (
                c.attempt,
                Leg::Arrive(Destination::Location(Location::Venue)),
                c.amount,
                c.fee,
            ),
            c.raw,
        ));
        if c.final_credit {
            let mut keys = movement.receipts.clone();
            if !keys.contains(&key) {
                keys.push(key.clone());
            }
            let broker_scope = self.scope(Location::Broker)?;
            let chain_key = keys
                .iter()
                .find(|k| k.scope == broker_scope)
                .ok_or(Error::Qualification)?;
            let chain_slot = j
                .transactions()
                .flat_map(|tx| tx.inputs.iter())
                .find(|i| {
                    i.event
                        .as_ref()
                        .is_some_and(|e| e.key == RecordKey::Economic(chain_key.clone()))
                })
                .and_then(|i| i.source_cut)
                .ok_or(Error::Qualification)?;
            if !j
                .state()?
                .funds()
                .iter()
                .any(|o| o.attempt == Some(c.attempt) && o.terminal)
            {
                self.terminal(
                    &mut tx,
                    &plan,
                    plan.gross,
                    keys,
                    vec![
                        funds::Coverage {
                            source: self.scope(Location::Broker)?,
                            through: chain_slot,
                        },
                        funds::Coverage {
                            source: self.profile.source,
                            through: c.cut,
                        },
                    ],
                )?;
            }
            let others_final = history
                .plans
                .iter()
                .filter(|p| p.rail == Rail::Deposit && p.attempt != plan.attempt)
                .all(|p| {
                    history.final_credits.contains(&p.attempt)
                        || history
                            .chains
                            .iter()
                            .any(|(a, _, ok)| a == &p.attempt && !ok)
                });
            tx.controls
                .push(Control::Funds(funds::Action::NativeCreditReady(
                    others_final && self.ready(&history, at),
                )));
        }
        checked(j.commit(tx)?)?;
        Ok(())
    }
    /// Reconcile the ORIGINAL withdrawn operation after timeout/restart. Fees
    /// follow P08's attributed house-cost rules; a native batch nonce is no credit.
    pub fn observe_withdrawal<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        id: CommitId,
        at: u64,
        w: Withdrawal,
    ) -> Result<(), Error> {
        let plan = self.existing(j, w.attempt, at)?;
        let p = &w.payment;
        if plan.rail != Rail::Withdraw
            || !w.complete
            || w.idempotency_key != withdrawal_id(w.attempt)
            || p.network != self.profile.config.domain.network.bytes()
            || p.attempt != w.attempt
            || p.mint != self.route.mint
            || p.program != self.route.venue_program
            || p.source != self.route.venue_vault
            || p.destination != self.route.broker_tokens
            || !p.succeeded
            || p.signature == [0; 64]
            || p.amount == 0
            || p.amount > w.gross
            || w.gross == 0
        {
            return Err(Error::Qualification);
        }
        let debit = EventKey {
            scope: self.profile.source,
            event: w.event,
            leg: 0,
        };
        let credit = EventKey {
            scope: self.scope(Location::Broker)?,
            event: EconomicEventId::new(&p.signature)?,
            leg: 1,
        };
        let mut tx = self.tx(
            j,
            id,
            at,
            Record::Withdrawal {
                attempt: plan.attempt.clone(),
            },
        )?;
        tx.inputs.push(self.input(
            (debit.clone(), w.cut, at),
            (w.attempt, Leg::Debit, w.gross, 0),
            w.raw,
        ));
        tx.inputs.push(self.input(
            (credit.clone(), p.slot, at),
            (
                w.attempt,
                Leg::Arrive(Destination::Location(Location::Broker)),
                w.gross,
                w.gross - p.amount,
            ),
            p.raw.clone(),
        ));
        if !j
            .state()?
            .funds()
            .iter()
            .any(|o| o.attempt == Some(w.attempt) && o.terminal)
        {
            self.terminal(
                &mut tx,
                &plan,
                w.gross,
                vec![debit, credit],
                vec![
                    funds::Coverage {
                        source: self.profile.source,
                        through: w.cut,
                    },
                    funds::Coverage {
                        source: self.scope(Location::Broker)?,
                        through: p.slot,
                    },
                ],
            )?;
        }
        checked(j.commit(tx)?)?;
        Ok(())
    }
    /// Inspect real residuals after interruption/partial return/cleanup, never
    /// write an unexplained balancing amount or presume transit has arrived.
    pub fn residuals<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        at: u64,
    ) -> Result<Residuals, Error> {
        let history = self.history(j, at)?;
        let state = j.state()?;
        let ledger = state.ledger();
        Ok(Residuals {
            vault: ledger.vault(),
            broker: ledger.broker(),
            native: ledger.venue().cash(),
            transit: ledger.in_transit().map_err(|_| Error::Qualification)?,
            unresolved: history
                .plans
                .into_iter()
                .filter_map(|p| p.attempt().ok())
                .filter(|a| {
                    state
                        .funds()
                        .iter()
                        .any(|o| o.attempt == Some(*a) && !o.terminal)
                })
                .collect(),
        })
    }
}
/// Domain-separated native idempotency UUID; not a random retry identifier.
pub fn withdrawal_id(attempt: AttemptKey) -> String {
    let mut h = Sha256::new();
    h.update(b"CINDER-PACIFICA-WITHDRAWAL-1\0");
    h.update(attempt.encode());
    let mut b: [u8; 32] = h.finalize().into();
    b[6] = (b[6] & 15) | 64;
    b[8] = (b[8] & 63) | 128;
    let hex: String = b[..16].iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}
fn checked(committed: cinder_journal::Committed) -> Result<(), Error> {
    if committed.receipt.controls.is_some()
        || committed.receipt.inputs.iter().any(|r| {
            !matches!(
                r,
                InputResult::Normalized(Disposition::Applied | Disposition::Duplicate)
            )
        })
        || committed.receipt.funds_observations.iter().any(|ok| !*ok)
    {
        return Err(Error::Qualification);
    }
    Ok(())
}
fn exposure(committed: cinder_journal::Committed, plan: &Plan) -> Result<(), Error> {
    if committed.duplicate || committed.receipt.controls.is_some() || committed.exposures.len() != 1
    {
        return Err(Error::Qualification);
    }
    let (key, bytes) = committed
        .exposures
        .into_iter()
        .next()
        .ok_or(Error::Qualification)?
        .into_message();
    if key != plan.attempt()? || bytes.as_bytes() != plan.abstract_action {
        return Err(Error::Qualification);
    }
    Ok(())
}
