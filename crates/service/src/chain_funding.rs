//! Actual private chain controller composition. No hidden wallet, seeded funds,
//! guessed native credit, public signing endpoint or retry after uncertain send.
use crate::{
    Error, chain_code,
    chain_receipt::{self, Deployment},
    chain_rpc::{self, Client, Transport},
    transport::Clock,
};
use cinder_journal::{Backend, Journal, Protection, model::CommitId};
use cinder_kernel::{
    identity::AttemptKey,
    ledger::{Location, funds::Destination},
};
use cinder_pacifica::{
    execution::{Dispatch, Gateway, Reply},
    funding::{
        Controller, Rail,
        chain::{self, Limits},
    },
};
use openssl::{
    pkey::{Id, PKey},
    sha::sha256,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zeroize::Zeroizing;

mod staged;

#[cfg(test)]
#[path = "chain_funding_tests.rs"]
pub(crate) mod tests;

/// Explicit encrypted release policy. Native semantics stay independently gated.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    /// Credential-bearing target; public routing/trust stays in the measured
    /// manifest to keep recipient-KMS plaintext below its 4-KiB limit.
    pub path: String,
    /// Qualified P15 executable and upgrade governance.
    pub custody: Deployment,
    /// Qualified native deposit executable and upgrade governance.
    pub native: Deployment,
    /// Freshness, exact lamport fee cap and optional CU limit.
    pub limits: Limits,
    /// Per-boot request cap, including errors/bytecode chunks; no automatic retry.
    pub maximum_calls: u32,
    /// Explicit slot lifetime, distinct from the native last-valid block height.
    pub expiry_slots: u64,
    /// Bounded native observation cadence in milliseconds; not a complete cut.
    pub poll_ms: u64,
    /// Bounded original customer transactions to observe; never assumed credit.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deposits: Vec<crate::customer_deposit::Locator>,
}
/// Public measured parent route and enclave trust; no credential-bearing path.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Peer {
    /// Explicit devnet origin, not a general-purpose parent proxy.
    pub host: String,
    /// Fixed parent CID 3 port, distinct from ingress/native/cloud.
    pub port: u32,
    /// Loaded public CA certificate.
    pub root: Vec<u8>,
    /// Independently approved trust-anchor digest.
    pub root_hash: [u8; 32],
    /// Exact expected devnet genesis.
    pub network: [u8; 32],
}
impl Peer {
    /// Check the public route without a credential or socket.
    pub fn validate(&self) -> Result<(), Error> {
        if !matches!(
            self.host.as_str(),
            "devnet.helius-rpc.com" | "api.devnet.solana.com"
        ) {
            return Err(Error);
        }
        self.endpoint("/".into()).validate()
    }
    fn endpoint(&self, path: String) -> chain_rpc::Endpoint {
        chain_rpc::Endpoint {
            host: self.host.clone(),
            path,
            port: self.port,
            root: self.root.clone(),
            root_hash: self.root_hash,
            network: self.network,
        }
    }
}
impl Drop for Configuration {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.path.zeroize();
    }
}
/// Measured policy plus separately released Solana funds seed. The native broker
/// seed stays in the existing Controller; storage/trading seeds cannot substitute.
pub struct Loaded {
    configuration: Configuration,
    endpoint: chain_rpc::Endpoint,
    funds: Zeroizing<[u8; 32]>,
    commitment: [u8; 32],
}
impl Loaded {
    /// No I/O/exposure. Bind actual route, seed identity, RPC and limits.
    pub fn new(
        configuration: Configuration,
        peer: &Peer,
        funds: Zeroizing<[u8; 32]>,
        funding: &Controller,
        network: [u8; 32],
    ) -> Result<Self, Error> {
        peer.validate()?;
        let endpoint = peer.endpoint(configuration.path.clone());
        endpoint.validate()?;
        configuration.custody.validate()?;
        configuration.native.validate()?;
        configuration.limits.validate().map_err(|_| Error)?;
        let public: [u8; 32] = PKey::private_key_from_raw_bytes(funds.as_slice(), Id::ED25519)?
            .raw_public_key()?
            .try_into()
            .map_err(|_| Error)?;
        let route = funding.route();
        if configuration.deposits.len() > 16 {
            return Err(Error);
        }
        for (i, locator) in configuration.deposits.iter().enumerate() {
            locator.validate()?;
            if !route
                .beneficiaries
                .iter()
                .any(|b| b.account == locator.account)
                || configuration.deposits[..i].iter().any(|l| {
                    l.signature == locator.signature
                        || l.account == locator.account && l.operation == locator.operation
                })
            {
                return Err(Error);
            }
        }
        if funds.as_slice() == [0; 32]
            || public != route.funds
            || endpoint.network != network
            || configuration.custody.program != route.program
            || configuration.native.program != route.venue_program
            || configuration.maximum_calls == 0
            || configuration.maximum_calls > 100_000
            || configuration.expiry_slots == 0
            || configuration.expiry_slots > 400
            || endpoint.path.len() > 512
            || configuration.poll_ms < 1_000
            || configuration.poll_ms > 3_600_000
        {
            return Err(Error);
        }
        let commitment = sha256(
            &[
                b"CINDER-LOADED-CHAIN-1\0".as_slice(),
                &serde_cbor::to_vec(&configuration).map_err(|_| Error)?,
                &endpoint.commitment()?,
                &public,
            ]
            .concat(),
        );
        Ok(Self {
            configuration,
            endpoint,
            funds,
            commitment,
        })
    }
    /// Component derived from actually consumed policy/key identity.
    pub fn commitment(&self) -> [u8; 32] {
        self.commitment
    }
    /// The demo grant must name the sole configured original customer deposit.
    pub(crate) fn permits_demo_deposit(&self, locator: &crate::customer_deposit::Locator) -> bool {
        self.configuration.deposits.as_slice() == std::slice::from_ref(locator)
    }
    /// Public-only parent route, never the credential-bearing RPC path.
    pub fn peer(&self) -> (&str, u32) {
        (&self.endpoint.host, self.endpoint.port)
    }
    /// Cadence from the actual measured configuration.
    pub fn poll_ms(&self) -> u64 {
        self.configuration.poll_ms
    }
    /// Open the actual fixed-route enclave TLS client; no socket until a step.
    pub fn open(self, clock: Arc<dyn Clock>) -> Result<Port<chain_rpc::Https>, Error> {
        let https = chain_rpc::Https::new(self.endpoint.clone(), clock.clone())?;
        self.with_transport(https, clock)
    }
    /// Explicit trusted port injection for the SAME offline controller. This
    /// grants no production manifest/key release and no alternate wallet loader.
    pub fn with_transport<T: Transport>(
        self,
        transport: T,
        clock: Arc<dyn Clock>,
    ) -> Result<Port<T>, Error> {
        let client = Client::new(
            self.endpoint.network,
            transport,
            self.configuration.maximum_calls,
        )?;
        Ok(Port {
            rejected_deposits: vec![false; self.configuration.deposits.len()],
            loaded: self,
            client,
            clock,
        })
    }
}
/// One runtime-owned controller; all mutations use the caller's one journal.
pub struct Port<T: Transport> {
    loaded: Loaded,
    client: Client<T>,
    clock: Arc<dyn Clock>,
    // Bounded by the immutable release configuration, private and boot-local.
    // Only finalized original-transaction ineligibility is terminal here;
    // pending evidence and port/current-account/code errors are never cached.
    rejected_deposits: Vec<bool>,
}
/// Coarse result; ACK or missing evidence never means a settled customer payment.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Send acknowledgement only, not execution/finality/native credit.
    Submitted,
    /// Possibly sent; reconcile the original without a second submission.
    Unknown,
    /// Missing/uncertain final effect or native credit; holds remain.
    Pending,
    /// Finalized original owner transaction is ineligible; never customer credit.
    Rejected,
    /// Final physical effects applied to the existing funds lifecycle.
    Settled,
}
/// Unreleased original-deposit evidence from the configured, bounded chain port.
/// No host can construct it and no RPC result alone credits the customer.
pub(crate) struct DepositRead {
    locator: crate::customer_deposit::Locator,
    result: DepositResult,
}
enum DepositResult {
    Pending,
    Rejected,
    Found(Box<DepositEvidence>),
}
struct DepositEvidence {
    deposit: chain::CustomerDeposit,
    tx: crate::chain_rpc::Finalized,
    accounts: crate::chain_rpc::Accounts,
    deployment: chain_receipt::Deployment,
    proof: chain_code::Verified,
    maximum_fee: u64,
}
impl DepositRead {
    /// Rejoin the same mutation owner with fresh authority. Original receipt,
    /// code, current token accounts and configured customer binding are all
    /// checked by the existing recognizer before the one journal commit.
    pub(crate) fn complete<B: Backend, P: Protection>(
        self,
        j: &mut Journal<B, P>,
    ) -> Result<Outcome, Error> {
        j.verified_state().map_err(|_| Error)?;
        match self.result {
            DepositResult::Pending => Ok(Outcome::Pending),
            DepositResult::Rejected => Ok(Outcome::Rejected),
            DepositResult::Found(evidence) => {
                let DepositEvidence {
                    deposit,
                    tx,
                    accounts,
                    deployment,
                    proof,
                    maximum_fee,
                } = *evidence;
                crate::customer_deposit::recognize(
                    j,
                    &self.locator,
                    deposit,
                    tx,
                    accounts,
                    &deployment,
                    proof,
                    maximum_fee,
                )?;
                Ok(Outcome::Settled)
            }
        }
    }
}
fn id() -> Result<CommitId, Error> {
    let mut b = [0; 32];
    openssl::rand::rand_bytes(&mut b)?;
    CommitId::new(b).map_err(|_| Error)
}
fn rail<B: Backend, P: Protection>(
    j: &mut Journal<B, P>,
    attempt: AttemptKey,
) -> Result<(Rail, bool), Error> {
    let o = j
        .verified_state()
        .map_err(|_| Error)?
        .funds()
        .iter()
        .find(|o| o.attempt == Some(attempt) && !o.terminal)
        .ok_or(Error)?;
    let r = match (o.intent.source, o.intent.destination) {
        (Location::Vault, Destination::Location(Location::Broker)) => Rail::Release,
        (Location::Broker, Destination::Location(Location::Venue)) => Rail::Deposit,
        (Location::Broker, Destination::Location(Location::Vault)) => Rail::Return,
        (Location::Vault, Destination::Recipient(_)) => Rail::Payout,
        _ => return Err(Error),
    };
    Ok((r, o.recovery))
}
impl<T: Transport> Port<T> {
    /// Observe one configured owner-signed deposit. This is a read-only chain
    /// path, not a native funding activation or customer signing service.
    pub fn observe_deposit<B: Backend, P: Protection>(
        &mut self,
        j: &mut Journal<B, P>,
        funding: &Controller,
        index: usize,
    ) -> Result<Outcome, Error> {
        let locator = self.loaded.configuration.deposits.get(index).ok_or(Error)?;
        j.verified_state().map_err(|_| Error)?;
        if crate::customer_deposit::recorded(j, locator)? {
            return Ok(Outcome::Settled);
        }
        self.collect_deposit(funding.route(), index)?.complete(j)
    }
    /// Only configured identity metadata is handed to I/O. Native/chain signing
    /// keys never leave their existing controllers, and no journal guard is
    /// needed while fetching original transaction/code/account evidence.
    pub(crate) fn collect_deposit(
        &mut self,
        route: &cinder_pacifica::funding::Route,
        index: usize,
    ) -> Result<DepositRead, Error> {
        let locator = self
            .loaded
            .configuration
            .deposits
            .get(index)
            .ok_or(Error)?
            .clone();
        let rejected = |port: &mut Self| {
            port.rejected_deposits[index] = true;
            DepositRead {
                locator: locator.clone(),
                result: DepositResult::Rejected,
            }
        };
        let now = self.clock.now()?;
        if self.rejected_deposits[index] {
            return Ok(rejected(self));
        }
        let Some(tx) = self.client.finalized(locator.validate()?, now)? else {
            return Ok(DepositRead {
                locator,
                result: DepositResult::Pending,
            });
        };
        if tx.meta.get("err") != Some(&serde_json::Value::Null) {
            return Ok(rejected(self));
        }
        let Ok(deposit) = chain::inspect_customer_deposit(
            route,
            self.loaded.endpoint.network,
            locator.account,
            locator.operation,
            tx.wire.as_bytes(),
        ) else {
            return Ok(rejected(self));
        };
        if !crate::customer_deposit::valid_transaction(
            &locator,
            &deposit,
            &tx,
            self.loaded.configuration.limits.maximum_fee_lamports,
        )? {
            return Ok(rejected(self));
        }
        let d = &self.loaded.configuration.custody;
        let proof = chain_code::verify(
            &mut self.client,
            d,
            self.loaded.endpoint.network,
            tx.slot,
            tx.at,
        )?;
        let keys = chain_receipt::effect_keys(deposit.target(), d)?;
        let accounts = self.client.accounts(&keys, tx.slot, proof.at())?;
        Ok(DepositRead {
            locator,
            result: DepositResult::Found(Box::new(DepositEvidence {
                deposit,
                tx,
                accounts,
                deployment: d.clone(),
                proof,
                maximum_fee: self.loaded.configuration.limits.maximum_fee_lamports,
            })),
        })
    }
    pub(crate) fn deposit_locator(
        &self,
        index: usize,
    ) -> Result<&crate::customer_deposit::Locator, Error> {
        self.loaded.configuration.deposits.get(index).ok_or(Error)
    }
    /// One bounded read locator per scheduler cut; no dynamic parent input.
    pub fn deposit_count(&self) -> usize {
        self.loaded.configuration.deposits.len()
    }
    /// Issue one already-admitted, unexposed mandate. No automatic funds target,
    /// no changes to user economics, and no signature before simulation succeeds.
    pub fn issue<B: Backend, P: Protection>(
        &mut self,
        j: &mut Journal<B, P>,
        funding: &Controller,
        attempt: AttemptKey,
    ) -> Result<Outcome, Error> {
        self.issue_using(j, funding, attempt, None)
    }
    /// Explicit approved demo setup selection for an already accepted initial
    /// Release/Deposit. This does not admit an intent or activate a manifest;
    /// absent/failed demo setup never falls back to the strong issuance path.
    pub fn issue_demo_ingress<B: Backend, P: Protection>(
        &mut self,
        j: &mut Journal<B, P>,
        funding: &Controller,
        gateway: &Gateway,
        policy: &cinder_pacifica::funding::demo::Policy,
        attempt: AttemptKey,
    ) -> Result<Outcome, Error> {
        self.issue_using(j, funding, attempt, Some((gateway, policy)))
    }
    fn issue_using<B: Backend, P: Protection>(
        &mut self,
        j: &mut Journal<B, P>,
        funding: &Controller,
        attempt: AttemptKey,
        demo: Option<(&Gateway, &cinder_pacifica::funding::demo::Policy)>,
    ) -> Result<Outcome, Error> {
        let request = self.prepare_issue(j, funding, attempt, demo, self.clock.now()?)?;
        let read = self.collect_issue(request)?;
        let work = read.expose(j, funding, demo, self.clock.now()?)?;
        let simulation = self.simulate_issue(&work)?;
        let slot = work.slot();
        let delivery = self.persist_issue(j, funding, work, simulation, self.clock.now()?)?;
        self.submit_issue(delivery, slot)
    }
    /// Reconcile the ORIGINAL retained signature. No new blockhash/signature or
    /// send capability is created, including when missing/pruned/expired.
    pub fn reconcile<B: Backend, P: Protection>(
        &mut self,
        j: &mut Journal<B, P>,
        funding: &Controller,
        attempt: AttemptKey,
    ) -> Result<Outcome, Error> {
        let now = self.clock.now()?;
        let Some(request) = self.prepare_reconcile(j, funding, attempt, now)? else {
            return Ok(
                if j.state()
                    .map_err(|_| Error)?
                    .funds()
                    .iter()
                    .any(|o| o.attempt == Some(attempt) && o.terminal)
                {
                    Outcome::Settled
                } else {
                    Outcome::Pending
                },
            );
        };
        self.collect_reconcile(request)?
            .complete(j, funding, self.clock.now()?)
    }
}
