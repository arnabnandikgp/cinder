//! Exact P15/P16 legacy transaction boundary. This is not a general wallet:
//! only a durable ChainAction can create a signing capability. No key loading,
//! RPC, retries, priority-price instruction, nonce, ALT or arbitrary CPI.
use super::{ChainAction, Rail, VerifiedWire};
use crate::Error;
use cinder_journal::model::PrivateBytes;
use cinder_kernel::identity::AttemptKey;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

const TOKEN: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const ASSOCIATED: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
const COMPUTE: &str = "ComputeBudget111111111111111111111111111111";
const SYSTEM: [u8; 32] = [0; 32];
/// Existing legacy packet limit; larger/new transaction versions are not enabled.
pub const MAX_WIRE: usize = 1232;

/// Independently observed blockhash context from the configured authenticated
/// RPC. This data structure alone is not a finality or RPC trust certificate.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Context {
    /// Exact genesis identity in the financial namespace.
    pub network: [u8; 32],
    /// Native recent blockhash, never an operator's made-up expiry.
    pub blockhash: [u8; 32],
    /// Observed chain slot, distinct from height and millisecond time.
    pub slot: u64,
    /// Independently read current block height.
    pub height: u64,
    /// RPC's last valid block height for this blockhash.
    pub last_valid_height: u64,
    /// Trusted receive time in milliseconds.
    pub at: u64,
}
/// Explicit release policy; no production defaults or fee-price spending.
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    /// Maximum age of both the blockhash and successful simulation observations.
    pub maximum_age_ms: u64,
    /// Maximum independently quoted transaction fee, in lamports, not quote atoms.
    pub maximum_fee_lamports: u64,
    /// Optional bounded CU limit; zero omits the ComputeBudget instruction.
    pub compute_units: u32,
}
impl Limits {
    /// Validate explicit age/fee/compute limits; never supplies spending defaults.
    pub fn validate(self) -> Result<(), Error> {
        if self.maximum_age_ms == 0
            || self.maximum_age_ms > 30_000
            || self.maximum_fee_lamports == 0
            || self.compute_units > 1_400_000
        {
            return Err(Error::Qualification);
        }
        Ok(())
    }
}
/// Qualified fee/simulation result for the EXACT unsigned message. The RPC
/// adapter, not a user request, supplies it after authenticated response parsing.
/// A successful simulation is not execution or funding-settlement evidence.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Simulation {
    /// Network independently checked against the configured genesis.
    pub network: [u8; 32],
    /// SHA-256 of exactly the unsigned message that was simulated/fee-quoted.
    pub message: [u8; 32],
    /// Native context slot at which this simulation ran.
    pub slot: u64,
    /// Trusted receive time, in milliseconds.
    pub at: u64,
    /// Independent getFeeForMessage quote for this same message and blockhash.
    pub fee_lamports: u64,
    /// Exact successful simulation (not HTTP 200 or a send acknowledgement).
    pub succeeded: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Contract {
    schema: String,
    rail: Rail,
    network: [u8; 32],
    program: [u8; 32],
    domain: [u8; 32],
    pool: [u8; 32],
    config: [u8; 32],
    vault: [u8; 32],
    mint: [u8; 32],
    funds: [u8; 32],
    broker: [u8; 32],
    broker_tokens: [u8; 32],
    decimals: u8,
    venue_program: [u8; 32],
    venue_vault: [u8; 32],
    epoch: String,
    operation: [u8; 32],
    customer: [u8; 32],
    amount: String,
    sequence: String,
    paid: String,
    recipient_tokens: [u8; 32],
    expires_at_slot: String,
}
#[derive(Clone, Copy)]
struct Meta {
    key: [u8; 32],
    writable: bool,
    signer: bool,
}
struct Instruction {
    program: [u8; 32],
    metas: Vec<Meta>,
    data: Zeroizing<Vec<u8>>,
}
impl Instruction {
    fn validate(&self) -> Result<(), Error> {
        for (i, m) in self.metas.iter().enumerate() {
            if self.metas[..i].iter().any(|old| old.key == m.key)
                || m.key == self.program && (m.signer || m.writable)
            {
                return Err(Error::Qualification);
            }
        }
        Ok(())
    }
}
/// Non-clone, exact-message signing capability. Retain its original ChainAction
/// until Controller.persist_wire consumes it; this object cannot send a wire.
pub struct Prepared<'a> {
    action: &'a ChainAction,
    signer: [u8; 32],
    binding: [u8; 32],
    message: Zeroizing<Vec<u8>>,
    context: Context,
    limits: Limits,
    at: u64,
}
impl std::fmt::Debug for Prepared<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ChainPrepared([PRIVATE])")
    }
}
impl Prepared<'_> {
    /// Exact unsigned message for getFeeForMessage; stays inside enclave egress.
    pub fn message(&self) -> &[u8] {
        &self.message
    }
    /// Exact message correlation for the authenticated simulation recognizer.
    pub fn message_hash(&self) -> [u8; 32] {
        Sha256::digest(&self.message).into()
    }
    /// Expected single fee payer/authority; never selected by the relay.
    pub fn signer(&self) -> [u8; 32] {
        self.signer
    }
    /// Bound native context for exact minContextSlot/expiry checks at RPC egress.
    pub fn context(&self) -> Context {
        self.context
    }
    /// Loaded immutable simulation/fee/CU bounds, not caller-overridable per POST.
    pub fn limits(&self) -> Limits {
        self.limits
    }
    /// Signature-free simulation packet. sigVerify=false and
    /// replaceRecentBlockhash=false must be explicit at the RPC boundary.
    pub fn simulation_wire(&self) -> Result<PrivateBytes, Error> {
        packet([0; 64], &self.message)
    }
    /// Consume the signing capability after a fresh exact-message simulation and
    /// fee check. Seed is supplied by a purpose-separated enclave release, never
    /// read from a wallet/file/environment. The signed wire still cannot be sent
    /// before Controller.persist_wire returns its one-use ChainDelivery.
    pub fn sign(
        self,
        seed: Zeroizing<[u8; 32]>,
        simulation: Simulation,
        now: u64,
    ) -> Result<VerifiedWire, Error> {
        self.fresh(now)?;
        if !simulation.succeeded
            || simulation.network != self.context.network
            || simulation.message != self.message_hash()
            || simulation.slot < self.context.slot
            || simulation.slot > self.action.plan().counters().expires_at_slot
            || simulation.at < self.at
            || simulation.at > now
            || now - simulation.at > self.limits.maximum_age_ms
            || simulation.fee_lamports == 0
            || simulation.fee_lamports > self.limits.maximum_fee_lamports
        {
            return Err(Error::Qualification);
        }
        let key = SigningKey::from_bytes(&seed);
        if key.verifying_key().to_bytes() != self.signer {
            return Err(Error::Qualification);
        }
        let signature = key.sign(&self.message).to_bytes();
        let wire = packet(signature, &self.message)?;
        self.verify(wire.as_bytes(), now)
    }
    /// Verify an original retained wire for reconciliation, never generate a
    /// replacement blockhash or sending capability. Exact-message comparison
    /// rejects alternate instruction/account flags, prefixes and trailing bytes.
    pub fn verify(&self, wire: &[u8], now: u64) -> Result<VerifiedWire, Error> {
        // Signing deadline applies to creating a wire. Reconciliation of a
        // retained wire is allowed later and must not manufacture fresh authority.
        if now < self.at
            || wire.len() != self.message.len() + 65
            || wire.len() > MAX_WIRE
            || wire.first() != Some(&1)
            || wire.get(65..) != Some(self.message.as_slice())
        {
            return Err(Error::Qualification);
        }
        let signature: [u8; 64] = wire[1..65].try_into().map_err(|_| Error::Codec)?;
        VerifyingKey::from_bytes(&self.signer)
            .map_err(|_| Error::Codec)?
            .verify_strict(&self.message, &Signature::from_bytes(&signature))
            .map_err(|_| Error::Qualification)?;
        Ok(VerifiedWire {
            attempt: self.action.plan().attempt()?,
            binding: self.binding,
            signature,
            wire: PrivateBytes::new(wire.to_vec())?,
        })
    }
    fn fresh(&self, now: u64) -> Result<(), Error> {
        if now < self.at
            || now >= self.action.plan().expires_at()
            || now < self.context.at
            || now - self.context.at > self.limits.maximum_age_ms
        {
            return Err(Error::Qualification);
        }
        Ok(())
    }
}
/// Prepare only the immutable journal-selected chain rail, without signing or
/// exposing a live RPC. Context must come from a separately qualified provider.
pub fn prepare(
    action: &ChainAction,
    context: Context,
    limits: Limits,
    now: u64,
) -> Result<Prepared<'_>, Error> {
    limits.validate()?;
    let contract = action.encode()?;
    let c = parse(contract.as_bytes())?;
    if c.network != context.network
        || context.blockhash == [0; 32]
        || context.slot == 0
        || context.height == 0
        || context.height > context.last_valid_height
        || context.slot > integer(&c.expires_at_slot)?
        || context.at == 0
        || now < context.at
        || now - context.at > limits.maximum_age_ms
        || now < action.plan().prepared_at()
        || now >= action.plan().expires_at()
    {
        return Err(Error::Qualification);
    }
    let (instruction, signer) = instruction(&c)?;
    let message = compile(
        &instruction,
        signer,
        context.blockhash,
        limits.compute_units,
    )?;
    Ok(Prepared {
        action,
        signer,
        binding: Sha256::digest(contract.as_bytes()).into(),
        message,
        context,
        limits,
        at: now,
    })
}

/// Expected finalized effects of an independently verified ORIGINAL chain wire.
/// No signing/delivery capability, native credit or finality is granted here.
pub struct Target {
    /// Exact original wire and verified signature/contract binding.
    pub verified: VerifiedWire,
    /// Physical rail; native owner HTTP withdrawal is never accepted by this codec.
    pub rail: Rail,
    /// Actual genesis namespace, independently rechecked at RPC.
    pub network: [u8; 32],
    /// Invoked program whose deployed bytecode must be qualified.
    pub program: [u8; 32],
    /// Bound quote mint.
    pub mint: [u8; 32],
    /// Bound quote atom precision.
    pub decimals: u8,
    /// Exact source SPL token account.
    pub source: [u8; 32],
    /// Exact destination SPL token account.
    pub destination: [u8; 32],
    /// Expected source token-account authority.
    pub source_owner: [u8; 32],
    /// Expected destination token-account authority.
    pub destination_owner: [u8; 32],
    /// Bound P15 configuration account.
    pub config: [u8; 32],
    /// Exact public custody deployment seed.
    pub domain: [u8; 32],
    /// Exact public custody pool seed.
    pub pool: [u8; 32],
    /// Separate Solana funds authority, not broker HTTP authority.
    pub funds: [u8; 32],
    /// Fixed broker owner.
    pub broker: [u8; 32],
    /// Fixed broker token account.
    pub broker_tokens: [u8; 32],
    /// Canonical custody token vault.
    pub vault: [u8; 32],
    /// Permanent immutable P15 movement receipt, absent for a native deposit.
    pub receipt: Option<[u8; 32]>,
    /// P15 customer counter, payout only.
    pub customer_counter: Option<[u8; 32]>,
    /// Actual requested gross amount.
    pub amount: u64,
    /// Original movement operation, not private request ID.
    pub operation: [u8; 32],
    /// Original movement epoch.
    pub epoch: u64,
    /// Original counter precondition.
    pub sequence: u64,
    /// Original lifetime paid precondition.
    pub paid: u64,
    /// Payout owner, zero for internal movement.
    pub customer: [u8; 32],
    /// Exact legacy key order for transaction-specific balance evidence.
    pub keys: Vec<[u8; 32]>,
}
impl std::fmt::Debug for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ChainTarget([PRIVATE])")
    }
}
/// Reconcile a retained wire after a restart/expired blockhash without creating
/// any fresh signature. The attempt/contract must come from the common journal;
/// the native provider must independently establish genesis/finality/effects.
pub fn inspect(contract: &[u8], attempt: AttemptKey, wire: &[u8]) -> Result<Target, Error> {
    let c = parse(contract)?;
    let (ix, signer) = instruction(&c)?;
    if attempt.request.domain.network.bytes() != c.network
        || attempt.request.domain.deployment.bytes() != c.domain
        || wire.len() < 102
        || wire.len() > MAX_WIRE
        || wire.first() != Some(&1)
        || wire[65] != 1
        || wire[66] != 0
    {
        return Err(Error::Qualification);
    }
    let n = usize::from(wire[68]);
    if !(2..=32).contains(&n) {
        return Err(Error::Codec);
    }
    let blockhash_at = 69 + n * 32;
    let blockhash: [u8; 32] = wire
        .get(blockhash_at..blockhash_at + 32)
        .ok_or(Error::Codec)?
        .try_into()
        .map_err(|_| Error::Codec)?;
    let count_at = blockhash_at + 32;
    let units = match wire.get(count_at) {
        Some(1) => 0,
        Some(2) => {
            let budget = wire.get(count_at + 1..count_at + 9).ok_or(Error::Codec)?;
            if budget[1..4] != [0, 5, 2] {
                return Err(Error::Codec);
            }
            let units = u32::from_le_bytes(budget[4..8].try_into().map_err(|_| Error::Codec)?);
            if units == 0 || units > 1_400_000 {
                return Err(Error::Qualification);
            }
            units
        }
        _ => return Err(Error::Codec),
    };
    let message = compile(&ix, signer, blockhash, units)?;
    if wire.get(65..) != Some(message.as_slice()) {
        return Err(Error::Qualification);
    }
    let signature: [u8; 64] = wire[1..65].try_into().map_err(|_| Error::Codec)?;
    VerifyingKey::from_bytes(&signer)
        .map_err(|_| Error::Codec)?
        .verify_strict(&message, &Signature::from_bytes(&signature))
        .map_err(|_| Error::Qualification)?;
    let (source, destination, source_owner, destination_owner) = match c.rail {
        Rail::Release => (c.vault, c.broker_tokens, c.config, c.broker),
        Rail::Return => (c.broker_tokens, c.vault, c.broker, c.config),
        Rail::Payout => (c.vault, c.recipient_tokens, c.config, c.customer),
        Rail::Deposit => (
            c.broker_tokens,
            c.venue_vault,
            c.broker,
            pda(c.venue_program, &[b"central_state"])?,
        ),
        _ => return Err(Error::Qualification),
    };
    Ok(Target {
        verified: VerifiedWire {
            attempt,
            binding: Sha256::digest(contract).into(),
            signature,
            wire: PrivateBytes::new(wire.to_vec())?,
        },
        rail: c.rail,
        network: c.network,
        program: ix.program,
        mint: c.mint,
        decimals: c.decimals,
        source,
        destination,
        source_owner,
        destination_owner,
        config: c.config,
        domain: c.domain,
        pool: c.pool,
        funds: c.funds,
        broker: c.broker,
        broker_tokens: c.broker_tokens,
        vault: c.vault,
        receipt: if c.rail == Rail::Deposit {
            None
        } else {
            Some(pda(c.program, &[b"receipt", &c.config, &c.operation])?)
        },
        customer_counter: if c.rail == Rail::Payout {
            Some(pda(c.program, &[b"customer", &c.config, &c.customer])?)
        } else {
            None
        },
        amount: integer(&c.amount)?,
        operation: c.operation,
        epoch: integer(&c.epoch)?,
        sequence: integer(&c.sequence)?,
        paid: integer(&c.paid)?,
        customer: c.customer,
        keys: wire[69..blockhash_at]
            .chunks_exact(32)
            .map(|b| b.try_into().map_err(|_| Error::Codec))
            .collect::<Result<_, _>>()?,
    })
}
/// Read-only authenticated customer deposit. It grants no signing capability and
/// is not a native venue deposit or a broker funds mandate.
pub struct CustomerDeposit {
    target: Target,
    account: [u8; 32],
}
impl CustomerDeposit {
    /// Verified metadata only; there is no signature or delivery constructor.
    pub fn target(&self) -> &Target {
        &self.target
    }
    /// Bound accounting identity from the actual controller's beneficiary list.
    pub fn account(&self) -> [u8; 32] {
        self.account
    }
    /// Consume validation into the trusted receipt recognizer.
    pub fn into_target(self) -> Target {
        self.target
    }
}
/// Inspect one original owner-signed P15 deposit, optionally preceded by a CU
/// limit. Accept semantic account ordering from web3.js, but no extra keys,
/// instructions, co-signers, token programs, redirects or custody destinations.
pub fn inspect_customer_deposit(
    route: &super::Route,
    network: [u8; 32],
    account: [u8; 32],
    operation: [u8; 32],
    wire: &[u8],
) -> Result<CustomerDeposit, Error> {
    let b = route
        .beneficiaries
        .iter()
        .find(|b| b.account == account)
        .ok_or(Error::Qualification)?;
    if network == [0; 32]
        || operation == [0; 32]
        || wire.len() < 102
        || wire.len() > MAX_WIRE
        || wire[0] != 1
        || wire[65] != 1
        || wire[66] != 0
    {
        return Err(Error::Qualification);
    }
    let n = usize::from(wire[68]);
    if !(10..=11).contains(&n) {
        return Err(Error::Codec);
    }
    let hash_at = 69 + n * 32;
    let keys = wire
        .get(69..hash_at)
        .ok_or(Error::Codec)?
        .chunks_exact(32)
        .map(|k| k.try_into().map_err(|_| Error::Codec))
        .collect::<Result<Vec<[u8; 32]>, _>>()?;
    if keys[0] != b.wallet || keys.iter().collect::<std::collections::BTreeSet<_>>().len() != n {
        return Err(Error::Qualification);
    }
    let count = hash_at + 32;
    let mut at = count + 1;
    let units = match wire.get(count) {
        Some(1) => 0,
        Some(2) => {
            let x = wire.get(at..at + 8).ok_or(Error::Codec)?;
            if keys.get(usize::from(x[0])) != Some(&named(COMPUTE)?) || x[1..4] != [0, 5, 2] {
                return Err(Error::Codec);
            }
            at += 8;
            let units = u32::from_le_bytes(x[4..8].try_into().map_err(|_| Error::Codec)?);
            if units == 0 || units > 1_400_000 {
                return Err(Error::Qualification);
            }
            units
        }
        _ => return Err(Error::Codec),
    };
    let ix = wire.get(at..).ok_or(Error::Codec)?;
    if ix.len() != 108
        || ix[1] != 9
        || ix[11] != 96
        || keys.get(usize::from(ix[0])) != Some(&route.program)
    {
        return Err(Error::Codec);
    }
    let d = &ix[12..];
    let epoch = u64::from_le_bytes(d[40..48].try_into().map_err(|_| Error::Codec)?);
    let amount = u64::from_le_bytes(d[88..96].try_into().map_err(|_| Error::Codec)?);
    if d[..8] != Sha256::digest(b"global:deposit")[..8]
        || d[8..40] != route.domain
        || d[48..80] != operation
        || epoch != route.epoch
        || amount == 0
        || d[80..88] == [0; 8]
    {
        return Err(Error::Qualification);
    }
    let config = pda(
        route.program,
        &[b"cinder_vault", &route.domain, &route.pool, &route.mint],
    )?;
    if config != route.config || pda(route.program, &[b"tokens", &config])? != route.vault {
        return Err(Error::Qualification);
    }
    let customer = pda(route.program, &[b"customer", &config, &b.wallet])?;
    let receipt = pda(route.program, &[b"deposit", &config, &b.wallet, &operation])?;
    let token = named(TOKEN)?;
    let expected = [
        (config, true),
        (customer, true),
        (b.wallet, true),
        (b.tokens, true),
        (route.vault, true),
        (route.mint, false),
        (receipt, true),
        (token, false),
        (SYSTEM, false),
    ];
    let readonly = usize::from(wire[67]);
    if readonly != 4 + usize::from(units != 0) {
        return Err(Error::Qualification);
    }
    for ((key, writable), index) in expected.iter().zip(&ix[2..11]) {
        let i = usize::from(*index);
        if keys.get(i) != Some(key) || (i < n - readonly) != *writable {
            return Err(Error::Qualification);
        }
    }
    let compute = named(COMPUTE)?;
    if keys.iter().any(|k| {
        !expected.iter().any(|(x, _)| x == k)
            && *k != route.program
            && !(units != 0 && *k == compute)
    }) {
        return Err(Error::Qualification);
    }
    let signature: [u8; 64] = wire[1..65].try_into().map_err(|_| Error::Codec)?;
    VerifyingKey::from_bytes(&b.wallet)
        .map_err(|_| Error::Codec)?
        .verify_strict(&wire[65..], &Signature::from_bytes(&signature))
        .map_err(|_| Error::Qualification)?;
    let attempt = AttemptKey {
        request: cinder_kernel::identity::RequestKey {
            domain: cinder_kernel::identity::Domain {
                network: cinder_kernel::identity::NetworkId::new(network)?,
                deployment: cinder_kernel::identity::DeploymentId::new(route.domain)?,
            },
            account: cinder_kernel::identity::AccountId::new(account)?,
            request: cinder_kernel::identity::RequestId::new(operation)?,
        },
        attempt: cinder_kernel::identity::AttemptId::new(operation)?,
    };
    Ok(CustomerDeposit {
        account,
        target: Target {
            verified: VerifiedWire {
                attempt,
                binding: Sha256::digest(wire).into(),
                signature,
                wire: PrivateBytes::new(wire.to_vec())?,
            },
            rail: Rail::Deposit,
            network,
            program: route.program,
            mint: route.mint,
            decimals: route.decimals,
            source: b.tokens,
            destination: route.vault,
            source_owner: b.wallet,
            destination_owner: config,
            config,
            domain: route.domain,
            pool: route.pool,
            funds: route.funds,
            broker: route.broker,
            broker_tokens: route.broker_tokens,
            vault: route.vault,
            receipt: Some(receipt),
            customer_counter: Some(customer),
            amount,
            operation,
            epoch,
            sequence: 0,
            paid: 0,
            customer: b.wallet,
            keys,
        },
    })
}
fn integer(text: &str) -> Result<u64, Error> {
    if text.is_empty()
        || text.len() > 20
        || !text.bytes().all(|b| b.is_ascii_digit())
        || text.len() > 1 && text.starts_with('0')
    {
        return Err(Error::Codec);
    }
    text.parse().map_err(|_| Error::Codec)
}
fn parse(bytes: &[u8]) -> Result<Contract, Error> {
    if bytes.is_empty() || bytes.len() > 8192 {
        return Err(Error::Limit);
    }
    let c: Contract = serde_json::from_slice(bytes).map_err(|_| Error::Codec)?;
    // The typed deserializer rejects duplicate/unknown fields; sorted re-encoding
    // rejects alternate JSON, whitespace, numeric spelling and invalid UTF-8.
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| Error::Codec)?;
    if serde_json::to_vec(&value).map_err(|_| Error::Codec)? != bytes
        || c.schema != "cinder-vault-funding-v1"
        || c.decimals > 18
        || c.rail == Rail::Withdraw
        || [
            c.network,
            c.program,
            c.domain,
            c.pool,
            c.config,
            c.vault,
            c.mint,
            c.funds,
            c.broker,
            c.broker_tokens,
            c.venue_program,
            c.venue_vault,
            c.operation,
        ]
        .contains(&[0; 32])
        || integer(&c.epoch)? == 0
        || integer(&c.amount)? == 0
        || integer(&c.expires_at_slot)? == 0
    {
        return Err(Error::Qualification);
    }
    integer(&c.sequence)?;
    integer(&c.paid)?;
    if c.rail != Rail::Payout
        && (c.customer != [0; 32] || c.recipient_tokens != [0; 32] || integer(&c.paid)? != 0)
        || c.rail == Rail::Payout && (c.customer == [0; 32] || c.recipient_tokens == [0; 32])
    {
        return Err(Error::Qualification);
    }
    Ok(c)
}
fn named(name: &str) -> Result<[u8; 32], Error> {
    bs58::decode(name)
        .into_vec()
        .map_err(|_| Error::Codec)?
        .try_into()
        .map_err(|_| Error::Codec)
}
/// Canonical base58 address/genesis/blockhash representation at the RPC boundary.
pub fn address(bytes: [u8; 32]) -> String {
    bs58::encode(bytes).into_string()
}
/// Decode exactly one canonical 32-byte base58 value, never a display alias.
pub fn parse_address(text: &str) -> Result<[u8; 32], Error> {
    if text.is_empty() || text.len() > 44 {
        return Err(Error::Codec);
    }
    let bytes = named(text)?;
    if address(bytes) != text {
        return Err(Error::Codec);
    }
    Ok(bytes)
}
/// Canonical base58 transaction signature, not a native history UUID.
pub fn signature(bytes: [u8; 64]) -> String {
    bs58::encode(bytes).into_string()
}
/// Decode exactly one canonical 64-byte transaction signature.
pub fn parse_signature(text: &str) -> Result<[u8; 64], Error> {
    if text.is_empty() || text.len() > 88 {
        return Err(Error::Codec);
    }
    let bytes: [u8; 64] = bs58::decode(text)
        .into_vec()
        .map_err(|_| Error::Codec)?
        .try_into()
        .map_err(|_| Error::Codec)?;
    if signature(bytes) != text {
        return Err(Error::Codec);
    }
    Ok(bytes)
}
/// Canonical Solana PDA algorithm. Existing pinned dalek VerifyingKey::from_bytes
/// checks exactly Edwards decompression (including small-order points), not
/// signature validity or subgroup membership; an on-curve hash is rejected.
pub fn pda(program: [u8; 32], seeds: &[&[u8]]) -> Result<[u8; 32], Error> {
    if seeds.len() >= 16 || seeds.iter().any(|s| s.len() > 32) {
        return Err(Error::Limit);
    }
    for bump in (0..=255u8).rev() {
        let mut hash = Sha256::new();
        for seed in seeds {
            hash.update(seed);
        }
        hash.update([bump]);
        hash.update(program);
        hash.update(b"ProgramDerivedAddress");
        let candidate: [u8; 32] = hash.finalize().into();
        if VerifyingKey::from_bytes(&candidate).is_err() {
            return Ok(candidate);
        }
    }
    Err(Error::Qualification)
}
fn instruction(c: &Contract) -> Result<(Instruction, [u8; 32]), Error> {
    let token = named(TOKEN)?;
    let associated = named(ASSOCIATED)?;
    let compute = named(COMPUTE)?;
    let non_authorities = [
        c.program,
        c.config,
        c.vault,
        c.mint,
        c.broker_tokens,
        c.venue_program,
        c.venue_vault,
        token,
        associated,
        compute,
        SYSTEM,
    ];
    let config = pda(c.program, &[b"cinder_vault", &c.domain, &c.pool, &c.mint])?;
    if config != c.config
        || pda(c.program, &[b"tokens", &config])? != c.vault
        || c.funds == c.broker
        || non_authorities.contains(&c.funds)
        || non_authorities.contains(&c.broker)
        || [c.vault, c.venue_vault].contains(&c.broker_tokens)
        || c.vault == c.venue_vault
        || c.program == c.venue_program
        || [token, associated, compute, SYSTEM].contains(&c.program)
        || [token, associated, compute, SYSTEM].contains(&c.venue_program)
    {
        return Err(Error::Qualification);
    }
    let signer = match c.rail {
        Rail::Return | Rail::Deposit => c.broker,
        _ => c.funds,
    };
    let meta = |key, writable| Meta {
        key,
        writable,
        signer: key == signer,
    };
    let mut data = Zeroizing::new(Vec::new());
    let (program, metas) = if c.rail == Rail::Deposit {
        let central = pda(c.venue_program, &[b"central_state"])?;
        let ata = |owner: &[u8; 32]| pda(associated, &[owner, &token, &c.mint]);
        if ata(&signer)? != c.broker_tokens || ata(&central)? != c.venue_vault {
            return Err(Error::Qualification);
        }
        data.extend_from_slice(&[242, 35, 198, 137, 82, 225, 242, 182]);
        data.extend_from_slice(&integer(&c.amount)?.to_le_bytes());
        (
            c.venue_program,
            vec![
                meta(signer, true),
                meta(c.broker_tokens, true),
                meta(central, true),
                meta(c.venue_vault, true),
                meta(token, false),
                meta(associated, false),
                meta(c.mint, false),
                meta(SYSTEM, false),
                meta(pda(c.venue_program, &[b"__event_authority"])?, false),
                meta(c.venue_program, false),
            ],
        )
    } else {
        let (name, mut metas) = match c.rail {
            Rail::Release => (
                "release_funding",
                vec![
                    meta(config, true),
                    meta(signer, true),
                    meta(c.broker_tokens, true),
                    meta(c.vault, true),
                    meta(c.mint, false),
                ],
            ),
            Rail::Return => (
                "return_funding",
                vec![
                    meta(config, true),
                    meta(signer, true),
                    meta(c.broker_tokens, true),
                    meta(c.vault, true),
                    meta(c.mint, false),
                ],
            ),
            Rail::Payout => (
                "normal_payout",
                vec![
                    meta(config, true),
                    meta(signer, true),
                    meta(c.customer, false),
                    meta(pda(c.program, &[b"customer", &config, &c.customer])?, true),
                    meta(c.recipient_tokens, true),
                    meta(c.vault, true),
                    meta(c.mint, false),
                ],
            ),
            _ => return Err(Error::Qualification),
        };
        data.extend_from_slice(&Sha256::digest(format!("global:{name}").as_bytes())[..8]);
        data.extend_from_slice(&c.domain);
        data.extend_from_slice(&integer(&c.epoch)?.to_le_bytes());
        data.extend_from_slice(&c.operation);
        data.extend_from_slice(&integer(&c.expires_at_slot)?.to_le_bytes());
        data.extend_from_slice(&integer(&c.amount)?.to_le_bytes());
        if c.rail == Rail::Payout {
            data.extend_from_slice(&integer(&c.paid)?.to_le_bytes());
        }
        if c.rail != Rail::Return {
            data.extend_from_slice(&integer(&c.sequence)?.to_le_bytes());
        }
        metas.extend([
            meta(pda(c.program, &[b"receipt", &config, &c.operation])?, true),
            meta(token, false),
            meta(SYSTEM, false),
        ]);
        (c.program, metas)
    };
    let ix = Instruction {
        program,
        metas,
        data,
    };
    ix.validate()?;
    if ix.metas.iter().filter(|m| m.signer).count() != 1
        || !ix.metas.iter().any(|m| m.key == signer && m.writable)
        || signer == program
    {
        return Err(Error::Qualification);
    }
    Ok((ix, signer))
}
fn compact(n: usize, out: &mut Vec<u8>) -> Result<(), Error> {
    let mut n = u16::try_from(n).map_err(|_| Error::Limit)?;
    loop {
        let b = (n & 127) as u8;
        n >>= 7;
        out.push(b | if n == 0 { 0 } else { 128 });
        if n == 0 {
            break;
        }
    }
    Ok(())
}
fn compile(
    ix: &Instruction,
    signer: [u8; 32],
    blockhash: [u8; 32],
    units: u32,
) -> Result<Zeroizing<Vec<u8>>, Error> {
    // Stable first-seen ordering within privilege groups, fee payer always first.
    // Instruction metas were checked unique; only native event-CPI's program
    // account may also be the invoked (read-only) program key.
    let mut keys = vec![signer];
    for writable in [true, false] {
        for meta in &ix.metas {
            if !meta.signer && meta.writable == writable && !keys.contains(&meta.key) {
                keys.push(meta.key);
            }
        }
    }
    if !keys.contains(&ix.program) {
        keys.push(ix.program);
    }
    let compute = named(COMPUTE)?;
    if units != 0 {
        if keys.contains(&compute) {
            return Err(Error::Qualification);
        }
        keys.push(compute);
    }
    let readonly = keys
        .iter()
        .skip(1)
        .filter(|key| !ix.metas.iter().any(|m| m.key == **key && m.writable))
        .count();
    let mut out = Zeroizing::new(vec![
        1,
        0,
        u8::try_from(readonly).map_err(|_| Error::Limit)?,
    ]);
    compact(keys.len(), &mut out)?;
    for key in &keys {
        out.extend_from_slice(key);
    }
    out.extend_from_slice(&blockhash);
    compact(if units == 0 { 1 } else { 2 }, &mut out)?;
    let index = |key| {
        keys.iter()
            .position(|k| *k == key)
            .and_then(|i| u8::try_from(i).ok())
            .ok_or(Error::Codec)
    };
    if units != 0 {
        out.extend_from_slice(&[index(compute)?, 0, 5, 2]);
        out.extend_from_slice(&units.to_le_bytes());
    }
    out.push(index(ix.program)?);
    compact(ix.metas.len(), &mut out)?;
    for meta in &ix.metas {
        out.push(index(meta.key)?);
    }
    compact(ix.data.len(), &mut out)?;
    out.extend_from_slice(&ix.data);
    if out.len() + 65 > MAX_WIRE {
        return Err(Error::Limit);
    }
    Ok(out)
}
fn packet(signature: [u8; 64], message: &[u8]) -> Result<PrivateBytes, Error> {
    if message.len() + 65 > MAX_WIRE {
        return Err(Error::Limit);
    }
    let mut wire = vec![1];
    wire.extend_from_slice(&signature);
    wire.extend_from_slice(message);
    Ok(PrivateBytes::new(wire)?)
}

#[cfg(test)]
mod tests {
    use super::super::{Beneficiary, Counters, Plan, Route};
    use super::*;
    use crate::profile::Level;
    use cinder_kernel::{codec::Canonical, identity::*};
    use serde_json::{Value, json};

    fn vectors() -> Vec<Value> {
        serde_json::from_str::<Value>(include_str!(
            "../../tests/fixtures/funding-codec-public.json"
        ))
        .unwrap()["vectors"]
            .as_array()
            .unwrap()
            .clone()
    }
    fn action(c: &Value) -> ChainAction {
        let c = parse(&serde_json::to_vec(c).unwrap()).unwrap();
        let attempt = AttemptKey {
            request: RequestKey {
                domain: Domain {
                    network: NetworkId::new(c.network).unwrap(),
                    deployment: DeploymentId::new(c.domain).unwrap(),
                },
                account: AccountId::new([11; 32]).unwrap(),
                request: RequestId::new([12; 32]).unwrap(),
            },
            attempt: AttemptId::new([13; 32]).unwrap(),
        };
        ChainAction {
            network: c.network,
            route: Route {
                domain: c.domain,
                pool: c.pool,
                funds: c.funds,
                beneficiaries: vec![Beneficiary {
                    account: [11; 32],
                    wallet: if c.customer == [0; 32] {
                        [17; 32]
                    } else {
                        c.customer
                    },
                    tokens: if c.recipient_tokens == [0; 32] {
                        [18; 32]
                    } else {
                        c.recipient_tokens
                    },
                }],
                program: c.program,
                config: c.config,
                vault: c.vault,
                mint: c.mint,
                broker: c.broker,
                broker_tokens: c.broker_tokens,
                venue_program: c.venue_program,
                venue_vault: c.venue_vault,
                epoch: integer(&c.epoch).unwrap(),
                decimals: c.decimals,
                withdrawal: Level::Qualified,
                chain: Level::Qualified,
                settings: Level::Qualified,
                withdrawal_cost: 120,
                maximum_movement: u64::MAX,
                maximum_fee: 0,
                setup_max_age: 1000,
            },
            plan: Plan {
                rail: c.rail,
                attempt: attempt.encode(),
                abstract_action: vec![],
                net: integer(&c.amount).unwrap(),
                gross: integer(&c.amount).unwrap(),
                operation: c.operation,
                epoch: integer(&c.epoch).unwrap(),
                customer: if c.customer == [0; 32] {
                    [17; 32]
                } else {
                    c.customer
                },
                counters: Counters {
                    sequence: integer(&c.sequence).unwrap(),
                    paid: integer(&c.paid).unwrap(),
                    recipient_tokens: c.recipient_tokens,
                    expires_at_slot: integer(&c.expires_at_slot).unwrap(),
                },
                at: 100,
                expires_at: 1000,
            },
        }
    }
    fn context() -> Context {
        Context {
            network: [1; 32],
            blockhash: [51; 32],
            slot: 100,
            height: 100,
            last_valid_height: 200,
            at: 100,
        }
    }
    fn limits(units: u32) -> Limits {
        Limits {
            maximum_age_ms: 100,
            maximum_fee_lamports: 6000,
            compute_units: units,
        }
    }
    fn simulation(p: &Prepared<'_>) -> Simulation {
        Simulation {
            network: [1; 32],
            message: p.message_hash(),
            slot: 100,
            at: 101,
            fee_lamports: 5000,
            succeeded: true,
        }
    }
    fn bytes(v: &Value) -> Vec<u8> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|x| u8::try_from(x.as_u64().unwrap()).unwrap())
            .collect()
    }

    #[test]
    fn customer_deposit_accepts_anchor_ordering_and_rejects_every_wire_substitution() {
        let mut route = action(&vectors()[6]["contract"]).route;
        route.beneficiaries[0].account = [1; 32];
        let fixtures: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/customer-deposit-public.json"
        ))
        .unwrap();
        for v in fixtures["deposits"].as_array().unwrap() {
            let wire = bytes(&v["wire"]);
            let good = inspect_customer_deposit(&route, [1; 32], [1; 32], [41; 32], &wire).unwrap();
            assert_eq!(good.account(), [1; 32]);
            assert_eq!(good.target().amount, 9007199254740993);
            assert_eq!(good.target().source_owner, route.beneficiaries[0].wallet);
            assert_eq!(good.target().destination, route.vault);
            for i in 0..wire.len() {
                let mut bad = wire.clone();
                bad[i] ^= 1;
                assert!(
                    inspect_customer_deposit(&route, [1; 32], [1; 32], [41; 32], &bad).is_err()
                );
            }
            let mut trailing = wire.clone();
            trailing.push(0);
            assert!(
                inspect_customer_deposit(&route, [1; 32], [1; 32], [41; 32], &trailing).is_err()
            );
            assert!(inspect_customer_deposit(&route, [1; 32], [1; 32], [42; 32], &wire).is_err());
            assert!(inspect_customer_deposit(&route, [1; 32], [2; 32], [41; 32], &wire).is_err());
            let mut wrong = route.clone();
            wrong.epoch += 1;
            assert!(inspect_customer_deposit(&wrong, [1; 32], [1; 32], [41; 32], &wire).is_err());
            assert!(inspect_customer_deposit(&route, [0; 32], [1; 32], [41; 32], &wire).is_err());
        }
    }

    #[test]
    fn all_rails_and_compute_variants_match_independently_verified_anchor_vectors() {
        for v in vectors() {
            let a = action(&v["contract"]);
            assert_eq!(
                serde_json::from_slice::<Value>(a.encode().unwrap().as_bytes()).unwrap(),
                v["contract"]
            );
            let p = prepare(
                &a,
                context(),
                limits(v["compute_units"].as_u64().unwrap() as u32),
                100,
            )
            .unwrap();
            assert_eq!(p.message(), bytes(&v["message"]));
            let unsigned = p.simulation_wire().unwrap();
            assert_eq!(&unsigned.as_bytes()[1..65], &[0; 64]);
            assert_eq!(&unsigned.as_bytes()[65..], p.message());
            let s = simulation(&p);
            let seed = if matches!(a.plan.rail, Rail::Deposit | Rail::Return) {
                9
            } else {
                7
            };
            let signed = p.sign(Zeroizing::new([seed; 32]), s, 102).unwrap();
            assert_eq!(signed.wire.as_bytes(), bytes(&v["wire"]));
            assert_eq!(
                signed.binding,
                Sha256::digest(a.encode().unwrap().as_bytes()).as_slice()
            );
            assert_eq!(signed.attempt, a.plan.attempt().unwrap());
        }
    }
    #[test]
    fn every_changed_wire_byte_trailing_prefix_and_extra_instruction_reject() {
        for v in vectors() {
            let a = action(&v["contract"]);
            let p = prepare(
                &a,
                context(),
                limits(v["compute_units"].as_u64().unwrap() as u32),
                100,
            )
            .unwrap();
            let good = bytes(&v["wire"]);
            for i in 0..good.len() {
                let mut bad = good.clone();
                bad[i] ^= 1;
                assert!(p.verify(&bad, 102).is_err());
            }
            let mut trailing = good.clone();
            trailing.push(0);
            assert!(p.verify(&trailing, 102).is_err());
            let mut prefix = vec![0];
            prefix.extend_from_slice(&good);
            assert!(p.verify(&prefix, 102).is_err());
            assert!(p.verify(&good[..good.len() - 1], 102).is_err());
        }
    }
    #[test]
    fn context_network_deadline_blockheight_and_clock_are_distinct_and_bounded() {
        let v = &vectors()[0];
        let a = action(&v["contract"]);
        for c in [
            Context {
                network: [2; 32],
                ..context()
            },
            Context {
                blockhash: [0; 32],
                ..context()
            },
            Context {
                slot: 0,
                ..context()
            },
            Context {
                height: 201,
                ..context()
            },
            Context {
                at: 101,
                ..context()
            },
            Context { at: 0, ..context() },
        ] {
            assert!(prepare(&a, c, limits(0), 100).is_err());
        }
        assert!(prepare(&a, context(), limits(0), 99).is_err());
        assert!(prepare(&a, context(), limits(0), 201).is_err());
        assert!(
            prepare(
                &a,
                Context {
                    at: 1000,
                    ..context()
                },
                limits(0),
                1000
            )
            .is_err()
        );
        let mut deadline = action(&v["contract"]);
        deadline.plan.counters.expires_at_slot = 99;
        assert!(prepare(&deadline, context(), limits(0), 100).is_err());
        for l in [
            Limits {
                maximum_age_ms: 0,
                ..limits(0)
            },
            Limits {
                maximum_age_ms: 30_001,
                ..limits(0)
            },
            Limits {
                maximum_fee_lamports: 0,
                ..limits(0)
            },
            limits(1_400_001),
        ] {
            assert!(prepare(&a, context(), l, 100).is_err());
        }
    }
    #[test]
    fn signing_requires_fresh_successful_exact_message_fee_and_separate_authority() {
        let v = &vectors()[0];
        let a = action(&v["contract"]);
        let valid = {
            let p = prepare(&a, context(), limits(0), 100).unwrap();
            simulation(&p)
        };
        for s in [
            Simulation {
                succeeded: false,
                ..valid
            },
            Simulation {
                network: [2; 32],
                ..valid
            },
            Simulation {
                message: [0; 32],
                ..valid
            },
            Simulation { slot: 99, ..valid },
            Simulation { at: 99, ..valid },
            Simulation { at: 103, ..valid },
            Simulation {
                fee_lamports: 0,
                ..valid
            },
            Simulation {
                fee_lamports: 6001,
                ..valid
            },
        ] {
            let p = prepare(&a, context(), limits(0), 100).unwrap();
            assert!(p.sign(Zeroizing::new([7; 32]), s, 102).is_err());
        }
        let p = prepare(&a, context(), limits(0), 100).unwrap();
        assert!(p.sign(Zeroizing::new([9; 32]), valid, 102).is_err());
        let p = prepare(&a, context(), limits(0), 100).unwrap();
        assert!(p.sign(Zeroizing::new([7; 32]), valid, 201).is_err());
        let p = prepare(&a, context(), limits(0), 100).unwrap();
        assert!(p.sign(Zeroizing::new([7; 32]), valid, 1000).is_err());
    }
    #[test]
    fn contract_schema_canonical_integers_duplicate_keys_and_no_payout_aliases() {
        let original = vectors()[0]["contract"].clone();
        for (field, value) in [
            ("amount", json!(1)),
            ("amount", json!("01")),
            ("amount", json!("-1")),
            ("amount", json!("18446744073709551616")),
            ("epoch", json!("0")),
            ("decimals", json!(19)),
            ("rail", json!("Withdraw")),
            ("schema", json!("other")),
            ("unknown", json!(true)),
            ("customer", json!(vec![1; 32])),
            ("paid", json!("1")),
            ("recipient_tokens", json!(vec![1; 32])),
        ] {
            let mut c = original.clone();
            c[field] = value;
            assert!(parse(&serde_json::to_vec(&c).unwrap()).is_err());
        }
        let canonical = serde_json::to_string(&original).unwrap();
        assert!(parse(format!(" {canonical}").as_bytes()).is_err());
        assert!(
            parse(
                canonical
                    .replace("\"amount\":", "\"amount\":\"2\",\"amount\":")
                    .as_bytes()
            )
            .is_err()
        );
        let mut payout = vectors()[6]["contract"].clone();
        payout["customer"] = json!(vec![0; 32]);
        assert!(parse(&serde_json::to_vec(&payout).unwrap()).is_err());
    }
    #[test]
    fn incorrect_pdas_native_atas_and_privilege_aliases_fail_before_signing() {
        for v in vectors() {
            for field in ["config", "vault"] {
                let mut c = v["contract"].clone();
                c[field] = json!(vec![31; 32]);
                let a = action(&c);
                assert!(prepare(&a, context(), limits(0), 100).is_err());
            }
            for (field, alias) in [
                ("broker", "funds"),
                ("broker_tokens", "vault"),
                ("venue_vault", "vault"),
                ("program", "venue_program"),
                ("funds", "mint"),
            ] {
                let mut c = v["contract"].clone();
                c[field] = c[alias].clone();
                let a = action(&c);
                assert!(prepare(&a, context(), limits(0), 100).is_err());
            }
            if v["rail"] == "Deposit" {
                for field in ["broker_tokens", "venue_vault"] {
                    let mut c = v["contract"].clone();
                    c[field] = json!(vec![31; 32]);
                    let a = action(&c);
                    assert!(prepare(&a, context(), limits(0), 100).is_err());
                }
            }
        }
    }
    #[test]
    fn public_synthetic_data_and_capabilities_are_not_exposed_by_debug() {
        let a = action(&vectors()[0]["contract"]);
        let p = prepare(&a, context(), limits(0), 100).unwrap();
        assert_eq!(format!("{p:?}"), "ChainPrepared([PRIVATE])");
        let s = simulation(&p);
        let wire = p.sign(Zeroizing::new([7; 32]), s, 102).unwrap();
        assert_eq!(format!("{wire:?}"), "VerifiedWire([PRIVATE])");
        // Existing dalek decompression treats small-order points as on-curve;
        // they must not accidentally be accepted as PDAs.
        assert!(VerifyingKey::from_bytes(&[0; 32]).is_ok());
    }
}
