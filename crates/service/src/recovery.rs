//! P21 recovery artifacts. This module runs on the private side, not the parent.
//! It derives claims from the SAME sealed journal and P16 history. Chain source
//! authenticity, native capability closure and publisher approval are explicit
//! qualified ports; these mathematical matches are not a proof of those ports.
use crate::Error;
use cinder_journal::model::CommitId;
use cinder_journal::{Backend, Head, Journal, Protection};
use cinder_kernel::identity::EventKey;
use cinder_pacifica::funding::{Controller, recovery::CustodyState};
use cinder_web_channel::Entropy;
use openssl::{
    encrypt::Encrypter,
    hash::MessageDigest,
    pkey::{Id, PKey},
    rsa::Padding,
    sha::sha256,
    sign::Verifier,
    symm::{Cipher, encrypt_aead},
};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

const TOKEN: [u8; 32] = [
    6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172, 28, 180, 133, 237,
    95, 91, 55, 145, 58, 140, 245, 133, 126, 255, 0, 169,
];
/// Bounded P17 proof size and publication inventory.
pub const MAX_CLAIMS: usize = 65_536;
/// Envelope/plaintext parser bound, not an unbounded JSON blob.
pub const MAX_PACKAGE: usize = 32_768;

/// Immutable dedicated RSA-3072 SPKI, authorized by the registered owner. No
/// wallet secrets, signature KDF, operator decryption or automatic key rotation.
pub struct RecipientKey {
    /// Registered claim owner, not a caller-selected payout destination.
    pub owner: [u8; 32],
    /// DER SubjectPublicKeyInfo for RSA-3072 with exponent 65537.
    pub spki: Vec<u8>,
    /// Ed25519 wallet authorization over `key_authorization`.
    pub signature: [u8; 64],
}
impl std::fmt::Debug for RecipientKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RecipientKey([PRIVATE])")
    }
}
/// Canonical wallet authorization digest. Bind a dedicated encryption key to the
/// immutable deployment/pool; reissue/rotation requires a separately approved flow.
pub fn key_authorization(domain: [u8; 32], pool: [u8; 32], spki: &[u8]) -> [u8; 32] {
    hash(&[b"CINDER_RECOVERY_KEY_V1", &domain, &pool, &sha256(spki)])
}
fn recipient(
    key: &RecipientKey,
    domain: [u8; 32],
    pool: [u8; 32],
) -> Result<PKey<openssl::pkey::Public>, Error> {
    if key.spki.len() > 1024 {
        return Err(Error);
    }
    let p = PKey::public_key_from_der(&key.spki)?;
    if p.id() != Id::RSA
        || p.bits() != 3072
        || p.rsa()?.e().to_vec() != [1, 0, 1]
        || p.public_key_to_der()? != key.spki
    {
        return Err(Error);
    }
    let wallet = PKey::public_key_from_raw_bytes(&key.owner, Id::ED25519)?;
    if !Verifier::new_without_digest(&wallet)?
        .verify_oneshot(&key.signature, &key_authorization(domain, pool, &key.spki))?
    {
        return Err(Error);
    }
    Ok(p)
}

const REGISTRY: &[u8] = b"CINDER_RECOVERY_KEYS_V1\0";
fn key_record(controller: &Controller, keys: &[RecipientKey]) -> Result<Vec<u8>, Error> {
    let r = controller.route();
    if keys.len() != r.beneficiaries.len() {
        return Err(Error);
    }
    let mut entries = std::collections::BTreeMap::new();
    for k in keys {
        recipient(k, r.domain, r.pool)?;
        if entries
            .insert(k.owner, (k.spki.clone(), k.signature.to_vec()))
            .is_some()
        {
            return Err(Error);
        }
    }
    if r.beneficiaries
        .iter()
        .any(|b| !entries.contains_key(&b.wallet))
    {
        return Err(Error);
    }
    let bytes = serde_json::to_vec(&entries.into_iter().collect::<Vec<_>>()).map_err(|_| Error)?;
    Ok([REGISTRY, &bytes].concat())
}
/// Register the complete owner-authorized key inventory INSIDE the existing
/// encrypted journal before cutover. Immutable in V1: no parent replacement,
/// stale-key substitution or undocumented operator reissue. Run through an
/// attested private configuration/onboarding port, not a public relay route.
pub fn register_keys<B: Backend, P: Protection>(
    controller: &Controller,
    j: &mut Journal<B, P>,
    id: CommitId,
    at: u64,
    keys: &[RecipientKey],
) -> Result<(), Error> {
    let state = j.verified_state().map_err(|_| Error)?;
    if state.recovery_sealed()
        || state.recovery_epoch().is_some()
        || j.transactions().any(|t| {
            t.evidence
                .iter()
                .any(|e| e.as_bytes().starts_with(REGISTRY))
        })
    {
        return Err(Error);
    }
    let record = key_record(controller, keys)?;
    let tx = cinder_journal::model::Transaction {
        id,
        expected: j.head(),
        at,
        evidence: vec![cinder_journal::model::PrivateBytes::new(record).map_err(|_| Error)?],
        inputs: vec![],
        order_observations: vec![],
        funds_observations: vec![],
        controls: vec![],
    };
    j.commit(tx).map_err(|_| Error)?;
    Ok(())
}
fn hash(parts: &[&[u8]]) -> [u8; 32] {
    let mut h = openssl::sha::Sha256::new();
    for p in parts {
        h.update(p);
    }
    h.finish()
}
fn random(e: &dyn Entropy) -> Result<[u8; 32], Error> {
    let mut b = [0; 32];
    e.fill(&mut b).map_err(|_| Error)?;
    if b == [0; 32] {
        return Err(Error);
    }
    Ok(b)
}

/// Public immutable P17 statement. Amounts are decimal strings, never JSON floats.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Statement {
    /// Canonical custody deployment.
    pub program: [u8; 32],
    /// Canonical config PDA.
    pub config: [u8; 32],
    /// Governed deployment seed.
    pub domain: [u8; 32],
    /// Immutable pool seed.
    pub pool: [u8; 32],
    /// Classic SPL collateral.
    pub mint: [u8; 32],
    /// Exact collateral atoms.
    pub decimals: u8,
    /// Frozen custody epoch.
    pub epoch: String,
    /// Ordered RFC9162 root.
    pub root: [u8; 32],
    /// Positive claims only, complete financial inventory remains in the journal.
    pub tree_size: u32,
    /// Already-net unpaid liability.
    pub total: String,
    /// Exact independent-witness accepted journal head.
    pub journal_cutoff: String,
    /// Exact accepted head digest.
    pub journal_hash: [u8; 32],
    /// Preparation evidence plus precommitted opaque delivery layout.
    pub evidence_hash: [u8; 32],
    /// Approved economic/recovery policy, not an invented zero placeholder.
    pub policy_hash: [u8; 32],
    /// Matched lifetime beneficiary payments, not another entitlement deduction.
    pub normal_paid: String,
    /// Matched lifetime funding release sequence.
    pub funding_sequence: String,
}
impl std::fmt::Debug for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RecoveryStatement([REDACTED])")
    }
}
impl Statement {
    /// Exact cross-language P17 context; root is deliberately not part of its own hash.
    pub fn context(&self) -> Result<[u8; 32], Error> {
        let number = |s: &str| -> Result<[u8; 8], Error> {
            let n: u64 = s.parse().map_err(|_| Error)?;
            if n.to_string() != s {
                return Err(Error);
            }
            Ok(n.to_le_bytes())
        };
        if self.tree_size == 0
            || self.tree_size as usize > MAX_CLAIMS
            || self.decimals > 18
            || self.epoch == "0"
            || self.total == "0"
            || self.journal_cutoff == "0"
            || [
                self.program,
                self.config,
                self.domain,
                self.pool,
                self.mint,
                self.journal_hash,
                self.evidence_hash,
                self.policy_hash,
            ]
            .contains(&[0; 32])
        {
            return Err(Error);
        }
        Ok(hash(&[
            b"CINDER_RECOVERY_CONTEXT_V1",
            &self.program,
            &self.config,
            &self.domain,
            &self.pool,
            &self.mint,
            &TOKEN,
            &[self.decimals],
            &number(&self.epoch)?,
            &self.tree_size.to_le_bytes(),
            &number(&self.total)?,
            &number(&self.journal_cutoff)?,
            &self.journal_hash,
            &self.evidence_hash,
            &self.policy_hash,
            &number(&self.normal_paid)?,
            &number(&self.funding_sequence)?,
        ]))
    }
}
/// Private leaf and bounded ordered inclusion path, no other customer's records.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Kit {
    /// Schema marker, no permissive downgrade.
    pub schema: String,
    /// Exact public statement.
    pub statement: Statement,
    /// Owner index.
    pub index: u32,
    /// Registered owner.
    pub owner: [u8; 32],
    /// Fixed registered recipient.
    pub destination: [u8; 32],
    /// Already-net amount in custody mint atoms.
    pub amount: String,
    /// Lifetime normal payout basis.
    pub paid_base: String,
    /// Lifetime normal payout sequence basis.
    pub payout_sequence_base: String,
    /// Fresh opaque one-time identity.
    pub claim_id: [u8; 32],
    /// Fresh private leaf blinding salt.
    pub salt: [u8; 32],
    /// Ordered P17 membership path.
    pub proof: Vec<[u8; 32]>,
}
impl std::fmt::Debug for Kit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RecoveryKit([PRIVATE])")
    }
}
impl Drop for Kit {
    fn drop(&mut self) {
        self.owner.zeroize();
        self.destination.zeroize();
        self.amount.zeroize();
        self.paid_base.zeroize();
        self.payout_sequence_base.zeroize();
        self.claim_id.zeroize();
        self.salt.zeroize();
        self.proof.zeroize();
    }
}
fn leaf(k: &Kit, context: [u8; 32]) -> Result<[u8; 32], Error> {
    let num = |s: &str| -> Result<[u8; 8], Error> {
        s.parse::<u64>().map(u64::to_le_bytes).map_err(|_| Error)
    };
    Ok(hash(&[
        &[0],
        b"CINDER_RECOVERY_CLAIM_V1",
        &context,
        &k.index.to_le_bytes(),
        &k.claim_id,
        &k.owner,
        &k.destination,
        &num(&k.amount)?,
        &num(&k.paid_base)?,
        &num(&k.payout_sequence_base)?,
        &k.salt,
    ]))
}
fn split(n: usize) -> usize {
    1 << ((usize::BITS - 1) - (n - 1).leading_zeros())
}
struct Tree {
    nodes: std::collections::BTreeMap<(usize, usize), [u8; 32]>,
    len: usize,
}
impl Tree {
    fn new(leaves: &[[u8; 32]]) -> Self {
        fn build(
            leaves: &[[u8; 32]],
            start: usize,
            nodes: &mut std::collections::BTreeMap<(usize, usize), [u8; 32]>,
        ) -> [u8; 32] {
            let n = leaves.len();
            let value = if n == 1 {
                leaves[0]
            } else {
                let k = split(n);
                hash(&[
                    &[1],
                    &build(&leaves[..k], start, nodes),
                    &build(&leaves[k..], start + k, nodes),
                ])
            };
            nodes.insert((start, n), value);
            value
        }
        let mut nodes = std::collections::BTreeMap::new();
        build(leaves, 0, &mut nodes);
        Self {
            nodes,
            len: leaves.len(),
        }
    }
    fn root(&self) -> [u8; 32] {
        self.nodes[&(0, self.len)]
    }
    fn proof(&self, index: usize) -> Vec<[u8; 32]> {
        fn walk(t: &Tree, start: usize, n: usize, i: usize) -> Vec<[u8; 32]> {
            if n == 1 {
                return vec![];
            }
            let k = split(n);
            let (mut p, sibling) = if i < k {
                (walk(t, start, k, i), t.nodes[&(start + k, n - k)])
            } else {
                (walk(t, start + k, n - k, i - k), t.nodes[&(start, k)])
            };
            p.push(sibling);
            p
        }
        walk(self, 0, self.len, index)
    }
}

/// Only encrypted package bytes leave the private boundary. AAD binds the exact
/// context/root/opaque locator; RSA OAEP and MGF1 both use SHA256.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    /// Fixed profile, no negotiation.
    pub schema: String,
    /// Random opaque storage locator, never wallet/account derived.
    pub locator: [u8; 32],
    /// P17 context hash.
    pub context: [u8; 32],
    /// Exact immutable root.
    pub root: [u8; 32],
    /// Wrapped per-package AES-256 key.
    pub wrapped_key: Vec<u8>,
    /// Independent GCM nonce.
    pub nonce: [u8; 12],
    /// AES-GCM ciphertext, never leaf JSON.
    pub ciphertext: Vec<u8>,
    /// Full authentication tag.
    pub tag: [u8; 16],
}
impl std::fmt::Debug for Envelope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RecoveryEnvelope([REDACTED])")
    }
}
fn seal(
    k: &Kit,
    locator: [u8; 32],
    public: &PKey<openssl::pkey::Public>,
    entropy: &dyn Entropy,
) -> Result<Envelope, Error> {
    let context = k.statement.context()?;
    let root = k.statement.root;
    let plain = Zeroizing::new(serde_json::to_vec(k).map_err(|_| Error)?);
    if plain.len() > MAX_PACKAGE {
        return Err(Error);
    }
    let aes = Zeroizing::new(random(entropy)?);
    let nonce: [u8; 12] = random(entropy)?[..12].try_into().map_err(|_| Error)?;
    let aad = [
        b"CINDER_RECOVERY_ENVELOPE_V1".as_slice(),
        &context,
        &root,
        &locator,
    ]
    .concat();
    let mut tag = [0; 16];
    let ciphertext = encrypt_aead(
        Cipher::aes_256_gcm(),
        &*aes,
        Some(&nonce),
        &aad,
        &plain,
        &mut tag,
    )?;
    let mut enc = Encrypter::new(public)?;
    enc.set_rsa_padding(Padding::PKCS1_OAEP)?;
    enc.set_rsa_oaep_md(MessageDigest::sha256())?;
    enc.set_rsa_mgf1_md(MessageDigest::sha256())?;
    // OpenSSL owns OAEP randomness/opaque buffers. NSM entropy for AES/salts
    // is explicit; full RSA RNG/erasure qualification is a separate G03 gate.
    let mut wrapped_key = vec![0; enc.encrypt_len(&*aes)?];
    let n = enc.encrypt(&*aes, &mut wrapped_key)?;
    wrapped_key.truncate(n);
    Ok(Envelope {
        schema: "cinder-recovery-envelope-v1".into(),
        locator,
        context,
        root,
        wrapped_key,
        nonce,
        ciphertext,
        tag,
    })
}
fn deliver(
    first: &mut dyn Replica,
    second: &mut dyn Replica,
    locator: [u8; 32],
    bytes: &[u8],
) -> Result<(), Error> {
    if first.identity() == [0; 32]
        || second.identity() == [0; 32]
        || first.identity() == second.identity()
    {
        return Err(Error);
    }
    first.put(locator, bytes)?;
    second.put(locator, bytes)?;
    if first.get(locator)? != bytes || second.get(locator)? != bytes {
        return Err(Error);
    }
    Ok(())
}
/// Immutable storage, with separately qualified administrative failure domains.
/// Equal identities are rejected; distinct labels alone do not prove independence.
pub trait Replica {
    /// Approved independent store identity.
    fn identity(&self) -> [u8; 32];
    /// Create immutable opaque object; conflict/missing write must fail.
    fn put(&mut self, locator: [u8; 32], bytes: &[u8]) -> Result<(), Error>;
    /// Read the exact persisted ciphertext without ordinary trading service.
    fn get(&mut self, locator: [u8; 32]) -> Result<Vec<u8>, Error>;
}
/// Public discovery entry. No owner, balance, leaf, key or per-user index.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Opaque locator supplied privately to the customer; retain a local copy.
    pub locator: [u8; 32],
    /// Digest of the exact stored envelope; signed by the governed publisher.
    pub digest: [u8; 32],
}
/// Public authenticated discovery statement. The publisher MUST sign these
/// exact `encode()` bytes with the governed P17 publishing authority after the
/// readbacks. Its signature is verified against actual chain configuration.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Fixed schema.
    pub schema: String,
    /// Immutable P17 statement.
    pub statement: Statement,
    /// Ordered independent store identities, whose retrieval configuration is governed.
    pub replicas: [[u8; 32]; 2],
    /// Sorted opaque ciphertext inventory, no plaintext customer index.
    pub entries: Vec<Entry>,
}
impl Manifest {
    /// Deterministic signed bytes. Caller cannot add arbitrary unsigned metadata.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        serde_json::to_vec(self).map_err(|_| Error)
    }
}
/// Availability-qualified result, not an automatic stage/activation permission.
pub struct Prepared {
    manifest: Manifest,
    head: Head,
    locators: Vec<([u8; 32], [u8; 32])>,
}
impl std::fmt::Debug for Prepared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RecoveryPrepared([PRIVATE])")
    }
}
impl Prepared {
    /// Public discovery bytes for an independently approved governed signature.
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    /// Private delivery mapping; never expose this as a public index.
    pub fn locators(&self) -> &[([u8; 32], [u8; 32])] {
        &self.locators
    }
    /// Recheck current authoritative head AND custody counters/backing immediately
    /// before publication. Any late fact invalidates this prepared result.
    pub fn recheck<B: Backend, P: Protection>(
        &self,
        controller: &Controller,
        j: &mut Journal<B, P>,
        check: EventKey,
        custody: &CustodyState,
    ) -> Result<(), Error> {
        if !j.verified_state().map_err(|_| Error)?.recovery_sealed() {
            return Err(Error);
        }
        let b = controller
            .recovery_backing(j, self.head, check, custody)
            .map_err(|_| Error)?;
        if b.total().to_string() != self.manifest.statement.total
            || b.epoch().to_string() != self.manifest.statement.epoch
        {
            return Err(Error);
        }
        Ok(())
    }
}
/// Derive every claim from the sealed journal; caller cannot supply amounts or
/// omit an offline customer. No package is declared available before BOTH exact
/// readbacks. Error leaves orphan ciphertext only, never stage authority.
#[allow(clippy::too_many_arguments)]
pub fn prepare<B: Backend, P: Protection>(
    controller: &Controller,
    j: &mut Journal<B, P>,
    expected: Head,
    check: EventKey,
    custody: &CustodyState,
    keys: &[RecipientKey],
    policy_hash: [u8; 32],
    preparation_evidence: [u8; 32],
    entropy: &dyn Entropy,
    first: &mut dyn Replica,
    second: &mut dyn Replica,
) -> Result<Prepared, Error> {
    if !j.verified_state().map_err(|_| Error)?.recovery_sealed()
        || policy_hash == [0; 32]
        || preparation_evidence == [0; 32]
        || first.identity() == [0; 32]
        || second.identity() == [0; 32]
        || first.identity() == second.identity()
    {
        return Err(Error);
    }
    let b = controller
        .recovery_backing(j, expected, check.clone(), custody)
        .map_err(|_| Error)?;
    let registered = key_record(controller, keys)?;
    let records = j
        .transactions()
        .flat_map(|t| &t.evidence)
        .filter(|e| e.as_bytes().starts_with(REGISTRY))
        .collect::<Vec<_>>();
    if records.len() != 1 || records[0].as_bytes() != registered {
        return Err(Error);
    }
    if b.claims().is_empty()
        || b.claims().len() > MAX_CLAIMS
        || keys.len() != b.cut().accounts().len()
    {
        return Err(Error);
    }
    let route = controller.route();
    let mut verified = std::collections::BTreeMap::new();
    for key in keys {
        if verified
            .insert(key.owner, recipient(key, route.domain, route.pool)?)
            .is_some()
        {
            return Err(Error);
        }
    }
    if route
        .beneficiaries
        .iter()
        .any(|c| !verified.contains_key(&c.wallet))
    {
        return Err(Error);
    }
    let replicas = [first.identity(), second.identity()];
    let mut locators = Vec::new();
    let mut unique = std::collections::BTreeSet::new();
    for c in b.claims() {
        let locator = random(entropy)?;
        if !unique.insert(locator) {
            return Err(Error);
        }
        locators.push((c.wallet(), locator));
    }
    // Precommit delivery layout before root construction. Ciphertext digests are
    // signed afterwards; hashing them into their own plaintext root is circular.
    let mut layout = Vec::new();
    for (_, l) in &locators {
        layout.extend_from_slice(l);
    }
    let evidence = hash(&[
        b"CINDER_RECOVERY_PREPARATION_V1",
        &preparation_evidence,
        &replicas[0],
        &replicas[1],
        &layout,
    ]);
    let mut s = Statement {
        program: route.program,
        config: route.config,
        domain: route.domain,
        pool: route.pool,
        mint: route.mint,
        decimals: route.decimals,
        epoch: b.epoch().to_string(),
        root: [0; 32],
        tree_size: b.claims().len() as u32,
        total: b.total().to_string(),
        journal_cutoff: expected.sequence.to_string(),
        journal_hash: expected.hash,
        evidence_hash: evidence,
        policy_hash,
        normal_paid: b.normal_paid().to_string(),
        funding_sequence: b.funding_sequence().to_string(),
    };
    let context = s.context()?;
    let mut kits = Vec::new();
    let mut leaves = Vec::new();
    let mut identities = std::collections::BTreeSet::new();
    let mut salts = std::collections::BTreeSet::new();
    for (index, c) in b.claims().iter().enumerate() {
        let claim_id = random(entropy)?;
        let salt = random(entropy)?;
        if !identities.insert(claim_id) || !salts.insert(salt) {
            return Err(Error);
        }
        let k = Kit {
            schema: "cinder-recovery-kit-v1".into(),
            statement: s.clone(),
            index: index as u32,
            owner: c.wallet(),
            destination: c.tokens(),
            amount: c.amount().to_string(),
            paid_base: c.paid_base().to_string(),
            payout_sequence_base: c.payout_sequence_base().to_string(),
            claim_id,
            salt,
            proof: vec![],
        };
        leaves.push(leaf(&k, context)?);
        kits.push(k);
    }
    let tree = Tree::new(&leaves);
    s.root = tree.root();
    let mut entries = Vec::new();
    for (index, mut k) in kits.into_iter().enumerate() {
        k.statement = s.clone();
        k.proof = tree.proof(index);
        let locator = locators[index].1;
        let public = verified.get(&k.owner).ok_or(Error)?;
        let envelope = seal(&k, locator, public, entropy)?;
        let bytes = serde_json::to_vec(&envelope).map_err(|_| Error)?;
        if bytes.len() > MAX_PACKAGE {
            return Err(Error);
        }
        deliver(first, second, locator, &bytes)?;
        entries.push(Entry {
            locator,
            digest: sha256(&bytes),
        });
    }
    entries.sort_by_key(|e| e.locator);
    // Catch a concurrent witness change before returning any publication result.
    controller
        .recovery_backing(j, expected, check, custody)
        .map_err(|_| Error)?;
    Ok(Prepared {
        manifest: Manifest {
            schema: "cinder-recovery-manifest-v1".into(),
            statement: s,
            replicas,
            entries,
        },
        head: expected,
        locators,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::{encrypt::Decrypter, rsa::Rsa, sign::Signer, symm::decrypt_aead};
    struct Random;
    impl Entropy for Random {
        fn fill(&self, b: &mut [u8]) -> Result<(), cinder_web_channel::Error> {
            openssl::rand::rand_bytes(b).map_err(|_| cinder_web_channel::Error)
        }
    }
    struct Refuse;
    impl Entropy for Refuse {
        fn fill(&self, _: &mut [u8]) -> Result<(), cinder_web_channel::Error> {
            Err(cinder_web_channel::Error)
        }
    }
    fn kit() -> Kit {
        Kit {
            schema: "cinder-recovery-kit-v1".into(),
            statement: Statement {
                program: [1; 32],
                config: [2; 32],
                domain: [3; 32],
                pool: [4; 32],
                mint: [5; 32],
                decimals: 6,
                epoch: "2".into(),
                root: [6; 32],
                tree_size: 1,
                total: "980".into(),
                journal_cutoff: "10".into(),
                journal_hash: [7; 32],
                evidence_hash: [8; 32],
                policy_hash: [9; 32],
                normal_paid: "20".into(),
                funding_sequence: "0".into(),
            },
            index: 0,
            owner: [10; 32],
            destination: [11; 32],
            amount: "980".into(),
            paid_base: "20".into(),
            payout_sequence_base: "1".into(),
            claim_id: [12; 32],
            salt: [13; 32],
            proof: vec![],
        }
    }
    #[test]
    fn recipient_key_is_dedicated_exact_rsa_and_owner_domain_authorized() {
        let wallet = PKey::generate_ed25519().unwrap();
        let rsa = PKey::from_rsa(Rsa::generate(3072).unwrap()).unwrap();
        let spki = rsa.public_key_to_der().unwrap();
        let owner = wallet.raw_public_key().unwrap().try_into().unwrap();
        let signature = Signer::new_without_digest(&wallet)
            .unwrap()
            .sign_oneshot_to_vec(&key_authorization([1; 32], [2; 32], &spki))
            .unwrap()
            .try_into()
            .unwrap();
        let mut key = RecipientKey {
            owner,
            spki,
            signature,
        };
        assert!(recipient(&key, [1; 32], [2; 32]).is_ok());
        assert!(recipient(&key, [1; 32], [3; 32]).is_err());
        key.signature[0] ^= 1;
        assert!(recipient(&key, [1; 32], [2; 32]).is_err());
        key.spki = PKey::from_rsa(Rsa::generate(2048).unwrap())
            .unwrap()
            .public_key_to_der()
            .unwrap();
        assert!(recipient(&key, [1; 32], [2; 32]).is_err());
        assert_eq!(format!("{key:?}"), "RecipientKey([PRIVATE])");
    }
    #[test]
    fn standard_recipient_envelope_roundtrip_binds_locator_root_context_and_refuses_rng_failure() {
        let rsa = PKey::from_rsa(Rsa::generate(3072).unwrap()).unwrap();
        let public = PKey::public_key_from_der(&rsa.public_key_to_der().unwrap()).unwrap();
        let k = kit();
        let e = seal(&k, [14; 32], &public, &Random).unwrap();
        let mut dec = Decrypter::new(&rsa).unwrap();
        dec.set_rsa_padding(Padding::PKCS1_OAEP).unwrap();
        dec.set_rsa_oaep_md(MessageDigest::sha256()).unwrap();
        dec.set_rsa_mgf1_md(MessageDigest::sha256()).unwrap();
        let mut key = Zeroizing::new(vec![0; dec.decrypt_len(&e.wrapped_key).unwrap()]);
        let n = dec.decrypt(&e.wrapped_key, &mut key).unwrap();
        key.truncate(n);
        let aad = [
            b"CINDER_RECOVERY_ENVELOPE_V1".as_slice(),
            &e.context,
            &e.root,
            &e.locator,
        ]
        .concat();
        let plain = Zeroizing::new(
            decrypt_aead(
                Cipher::aes_256_gcm(),
                &key,
                Some(&e.nonce),
                &aad,
                &e.ciphertext,
                &e.tag,
            )
            .unwrap(),
        );
        assert_eq!(*plain, serde_json::to_vec(&k).unwrap());
        let mut bad = aad.clone();
        *bad.last_mut().unwrap() ^= 1;
        assert!(
            decrypt_aead(
                Cipher::aes_256_gcm(),
                &key,
                Some(&e.nonce),
                &bad,
                &e.ciphertext,
                &e.tag
            )
            .is_err()
        );
        assert!(seal(&k, [14; 32], &public, &Refuse).is_err());
        let second = seal(&k, [14; 32], &public, &Random).unwrap();
        assert_ne!(e.nonce, second.nonce);
        assert_ne!(e.ciphertext, second.ciphertext);
        assert_eq!(format!("{k:?}"), "RecoveryKit([PRIVATE])");
        assert_eq!(format!("{e:?}"), "RecoveryEnvelope([REDACTED])");
    }
    struct Store {
        id: [u8; 32],
        writes: usize,
        bytes: Vec<u8>,
        fault: u8,
    }
    impl Replica for Store {
        fn identity(&self) -> [u8; 32] {
            self.id
        }
        fn put(&mut self, _: [u8; 32], b: &[u8]) -> Result<(), Error> {
            self.writes += 1;
            if self.fault == 1 {
                return Err(Error);
            }
            self.bytes = b.to_vec();
            Ok(())
        }
        fn get(&mut self, _: [u8; 32]) -> Result<Vec<u8>, Error> {
            if self.fault == 2 {
                return Err(Error);
            }
            if self.fault == 3 {
                return Ok(vec![0]);
            }
            Ok(self.bytes.clone())
        }
    }
    #[test]
    fn publication_requires_distinct_replicas_and_both_exact_readbacks() {
        for fault in 0..=3 {
            let mut a = Store {
                id: [1; 32],
                writes: 0,
                bytes: vec![],
                fault: 0,
            };
            let mut b = Store {
                id: [2; 32],
                writes: 0,
                bytes: vec![],
                fault,
            };
            assert_eq!(
                deliver(&mut a, &mut b, [3; 32], b"opaque ciphertext").is_ok(),
                fault == 0
            );
            assert_eq!(a.writes, 1);
            assert_eq!(b.writes, 1);
        }
        let mut a = Store {
            id: [1; 32],
            writes: 0,
            bytes: vec![],
            fault: 0,
        };
        let mut b = Store {
            id: [1; 32],
            writes: 0,
            bytes: vec![],
            fault: 0,
        };
        assert!(deliver(&mut a, &mut b, [3; 32], b"opaque ciphertext").is_err());
        assert_eq!(a.writes, 0);
        assert_eq!(b.writes, 0);
    }
    fn recursive(leaves: &[[u8; 32]]) -> [u8; 32] {
        if leaves.len() == 1 {
            leaves[0]
        } else {
            let k = split(leaves.len());
            hash(&[&[1], &recursive(&leaves[..k]), &recursive(&leaves[k..])])
        }
    }
    #[test]
    fn cached_ordered_tree_matches_recursive_roots_and_maximum_bounded_paths() {
        for n in 1..66 {
            let leaves = (0..n)
                .map(|i| hash(&[&(i as u64).to_le_bytes()]))
                .collect::<Vec<_>>();
            let t = Tree::new(&leaves);
            assert_eq!(t.root(), recursive(&leaves));
            for i in 0..n {
                assert!(t.proof(i).len() <= 16);
            }
        }
        let leaves = (0..MAX_CLAIMS)
            .map(|i| hash(&[&(i as u64).to_le_bytes()]))
            .collect::<Vec<_>>();
        let t = Tree::new(&leaves);
        assert_eq!(t.nodes.len(), 2 * MAX_CLAIMS - 1);
        for i in [0, 1, MAX_CLAIMS / 2, MAX_CLAIMS - 1] {
            assert_eq!(t.proof(i).len(), 16);
        }
    }
}
