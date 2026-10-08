//! Measured public manifest + per-role recipient KMS capsules. Neither a parent
//! supplied manifest hash nor a single unrestricted enclave master is accepted.
use crate::{
    Error,
    cloud::{Client, Credential, Endpoint, hex},
    nsm::Nsm,
    transport::Clock,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use openssl::{
    cms::CmsContentInfo,
    pkey::{PKey, Private},
    rsa::Rsa,
    sha::{sha256, sha384},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};
use zeroize::Zeroizing;

/// No default enables live financial actions. Finite disposable release lease;
/// witness epoch fences already-loaded keys independently of KMS permission.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Gates {
    /// Risk-increasing native signing; P20 refuses activation.
    pub trading: bool,
    /// Fund-moving signing. Only version 6's fixed private demo grant may enable it.
    pub funding: bool,
    /// Qualified source reads; P20 refuses activation pending P23.
    pub native_reads: bool,
    /// Finite boot key-use lease in milliseconds, at most one hour.
    pub maximum_boot_ms: u64,
}
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
/// Separate KMS authority/secret purpose; no unrestricted master derivation.
pub enum Role {
    /// Private immutable account/API/venue configuration.
    Configuration,
    /// Journal AEAD key for the declared generation/stream.
    Storage,
    /// Native trading-agent seed, never broker funds authority.
    Trading,
    /// Broker-owner withdrawal seed, never trading or Solana funds authority.
    Broker,
    /// Parent-unknown scoped witness STS credentials.
    Witness,
    /// Solana funds authority for vault release/payout; distinct sixth purpose.
    Funds,
}
impl Role {
    /// Stable encryption-context label, not a Rust debug representation.
    pub fn name(self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::Storage => "storage",
            Self::Trading => "trading",
            Self::Broker => "broker",
            Self::Witness => "witness",
            Self::Funds => "funds",
        }
    }
    fn code(self) -> u8 {
        match self {
            Self::Configuration => 1,
            Self::Storage => 2,
            Self::Trading => 3,
            Self::Broker => 4,
            Self::Witness => 5,
            Self::Funds => 6,
        }
    }
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
/// One independently governed role release under one exact KMS key ARN.
pub struct Slot {
    /// Fixed purpose authenticated in plaintext and encryption context.
    pub role: Role,
    /// Actual consumed KMS route/key/root, no key alias.
    pub endpoint: Endpoint,
    /// Approved body hash; fresh recipient release cannot substitute other data.
    pub plaintext_hash: [u8; 32],
}
/// Version 3's retained-pack resource contract. These are bounded qualification
/// ceilings, not a production throughput or retention/garbage-collection policy.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HistoryPolicy {
    /// Maximum original opaque frame bytes (at most 64 KiB).
    pub record_bytes: u32,
    /// Maximum accepted original opaque history (at most 8 MiB).
    pub history_bytes: u32,
    /// Maximum accepted frames including genesis (at most 256).
    pub records: u32,
    /// Per replica, per boot PUT attempts, charged before signing/I/O.
    pub put_requests: u32,
    /// Per replica, per boot PUT bytes, including repairs and uncertain writes.
    pub put_bytes: u64,
}
impl HistoryPolicy {
    /// Derive the actual backend limits from the measured fields.
    pub fn limits(&self) -> Result<cinder_journal::packed::Limits, Error> {
        let limits = cinder_journal::packed::Limits {
            record_bytes: self.record_bytes as usize,
            history_bytes: self.history_bytes as usize,
            records: self.records as usize,
        };
        limits.validate().map_err(|_| Error)?;
        if self.record_bytes > 65_536
            || self.history_bytes > 8_388_608
            || self.records > 256
            || self.put_requests == 0
            || self.put_requests > 1024
            || self.put_bytes < limits.pack_bytes().map_err(|_| Error)? as u64
            || self.put_bytes > 134_217_728
        {
            return Err(Error);
        }
        Ok(limits)
    }
}
/// Included verbatim in the measured image; no secrets/customer directory here.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Closed schema: 1 (five roles), 2 (chain), 3 (packs), 4 (native capture),
    /// 5 (demo confirmation), 6 (one fixed demo allocation; no withdrawal/trading).
    pub version: u32,
    /// Exact 64-byte network/deployment namespace.
    pub domain: Vec<u8>,
    /// Expected contract derived from the actual loaded financial controllers.
    pub application: [u8; 32],
    /// Stable nonzero journal identifier.
    pub stream: [u8; 32],
    /// Storage key generation, not writer epoch.
    pub generation: u64,
    /// Independently provisioned writer fencing epoch.
    pub epoch: u64,
    /// Enclave private TLS ingress port; parent CID 3 only.
    pub ingress: u32,
    /// Parent ciphertext/STS bootstrap port.
    pub bootstrap: u32,
    /// Fixed native HTTPS egress route.
    pub venue_port: u32,
    /// Loaded public venue CA certificate, canonical DER.
    pub venue_root: Vec<u8>,
    /// Independently approved venue trust-anchor digest.
    pub venue_root_hash: [u8; 32],
    /// First actual ciphertext replica contract.
    pub first: Endpoint,
    /// Second actual ciphertext replica contract.
    pub second: Endpoint,
    /// Authenticated fresh region-local witness contract.
    pub witness: Endpoint,
    /// Complete versioned purpose-separated release, without plaintext secrets.
    pub slots: Vec<Slot>,
    /// Capability decisions and finite boot lease.
    pub gates: Gates,
    /// Version 2's public chain routing/trust. Omitted in version 1 encoding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain: Option<crate::chain_funding::Peer>,
    /// Explicit pack selection, omitted from legacy version 1/2 encodings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history: Option<HistoryPolicy>,
    /// Version 4's separate fixed WSS route and one-capture-per-boot resource policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_capture: Option<crate::native_capture::Policy>,
    /// Explicit testnet-only confirmation policy, absent from versions 1–4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub demo_deposit: Option<cinder_pacifica::funding::demo::Policy>,
}
impl Manifest {
    /// Refuse ambiguous routes, key-role reuse, missing trust or live activation.
    pub fn validate(&self) -> Result<(), Error> {
        if !matches!(self.version,1..=6) || self.domain.len()!=64 || self.domain[..32]==[0;32] || self.domain[32..]==[0;32]
            || self.application==[0;32] || self.stream==[0;32] || self.generation==0 || self.epoch==0
            || self.slots.len()!=if self.version==1 {5}else{6} || self.first.service!="s3" || self.second.service!="s3" || self.witness.service!="dynamodb"
            || (self.first.resource==self.second.resource) || self.gates.maximum_boot_ms==0 || self.gates.maximum_boot_ms>3_600_000
            // P20 packages/test-gates only. Live reads/money movement need P23's
            // qualified chain/read ports and separately authorized manifest.
            || self.gates.trading || (self.gates.funding != (self.version==6)) || (self.version==1 && self.gates.native_reads)
        {
            return Err(Error);
        }
        let mut ports = vec![
            self.ingress,
            self.bootstrap,
            self.venue_port,
            self.first.port,
            self.second.port,
            self.witness.port,
        ];
        match (self.version, &self.chain) {
            (1, None) => {}
            (2..=6, Some(peer)) => {
                peer.validate()?;
                if peer.network.as_slice() != &self.domain[..32] {
                    return Err(Error);
                }
                ports.push(peer.port);
            }
            _ => return Err(Error),
        }
        match (self.version, &self.history) {
            (1 | 2, None) => {}
            (3..=6, Some(policy)) => {
                policy.limits()?;
            }
            _ => return Err(Error),
        }
        match (self.version, &self.native_capture) {
            (1..=3 | 5 | 6, None) => {}
            (4, Some(policy)) => {
                policy.validate()?;
                let history = self.history.as_ref().ok_or(Error)?;
                // Conservative raw JSON/frame overhead; storage still enforces
                // the actual remaining history and write budgets independently.
                let message = policy
                    .limits
                    .maximum_message
                    .checked_mul(4)
                    .and_then(|n| n.checked_add(8192))
                    .ok_or(Error)?;
                if !self.gates.native_reads
                    || policy.limits.maximum_ms > self.gates.maximum_boot_ms
                    || message > history.record_bytes as usize
                {
                    return Err(Error);
                }
                ports.push(policy.port);
            }
            _ => return Err(Error),
        }
        match (self.version, &self.demo_deposit) {
            (1..=4, None) => {}
            (5 | 6, Some(policy)) => {
                policy.validate().map_err(|_| Error)?;
                // Worst-case decimal-byte JSON encoding plus metadata/frame.
                let frame = cinder_pacifica::funding::demo::MAX_BODY * 4 + 8192;
                if !self.gates.native_reads
                    || self.version == 6 && policy.initial_setup.is_none()
                    || policy.lifetime_ms > self.gates.maximum_boot_ms
                    || frame > self.history.as_ref().ok_or(Error)?.record_bytes as usize
                {
                    return Err(Error);
                }
            }
            _ => return Err(Error),
        }
        for e in [&self.first, &self.second, &self.witness] {
            e.commitment()?;
        }
        crate::egress::Trust::from_der(&self.venue_root, self.venue_root_hash)?;
        let base_ports = ports.clone();
        for (i, s) in self.slots.iter().enumerate() {
            if base_ports.contains(&s.endpoint.port)
                || s.endpoint.service != "kms"
                || s.plaintext_hash == [0; 32]
                || self.slots[..i]
                    .iter()
                    .any(|old| old.role == s.role || old.endpoint.resource == s.endpoint.resource)
            {
                return Err(Error);
            }
            s.endpoint.commitment()?;
            if !ports.contains(&s.endpoint.port) {
                ports.push(s.endpoint.port);
            }
        }
        if [
            Role::Configuration,
            Role::Storage,
            Role::Trading,
            Role::Broker,
            Role::Witness,
        ]
        .iter()
        .any(|r| !self.slots.iter().any(|s| s.role == *r))
            || (self.version >= 2 && !self.slots.iter().any(|s| s.role == Role::Funds))
        {
            return Err(Error);
        }
        for port in &ports {
            crate::vsock::Target::new(3, *port)?;
        }
        let mut unique = ports.clone();
        unique.sort();
        unique.dedup();
        if unique.len() != ports.len() {
            return Err(Error);
        }
        // A shared KMS port may only route to the same region/host.
        for (i, s) in self.slots.iter().enumerate() {
            for old in &self.slots[..i] {
                if s.endpoint.port == old.endpoint.port
                    && s.endpoint.host()? != old.endpoint.host()?
                {
                    return Err(Error);
                }
            }
        }
        Ok(())
    }
    /// Release commitment to the complete consumed configuration (excluding PCRs).
    pub fn digest(&self) -> Result<[u8; 32], Error> {
        self.validate()?;
        Ok(sha256(
            &[
                match self.version {
                    1 => b"CINDER-RUNTIME-MANIFEST-1\0".as_slice(),
                    2 => b"CINDER-RUNTIME-MANIFEST-2\0".as_slice(),
                    3 => b"CINDER-RUNTIME-MANIFEST-3\0".as_slice(),
                    4 => b"CINDER-RUNTIME-MANIFEST-4\0".as_slice(),
                    5 => b"CINDER-RUNTIME-MANIFEST-5\0".as_slice(),
                    _ => b"CINDER-RUNTIME-MANIFEST-6\0".as_slice(),
                },
                &serde_cbor::to_vec(self).map_err(|_| Error)?,
            ]
            .concat(),
        ))
    }
    /// Exact non-secret authenticated KMS Encrypt/Decrypt context.
    pub fn context(&self, role: Role) -> Result<BTreeMap<String, String>, Error> {
        Ok(BTreeMap::from([
            ("cinder-release".into(), hex(&self.digest()?)),
            ("cinder-role".into(), role.name().into()),
            ("cinder-generation".into(), self.generation.to_string()),
            ("cinder-stream".into(), hex(&self.stream)),
        ]))
    }
    fn prefix(&self, role: Role) -> Result<Vec<u8>, Error> {
        if !self.slots.iter().any(|s| s.role == role) {
            return Err(Error);
        }
        Ok([
            match self.version {
                1 => b"CKR1".as_slice(),
                2 => b"CKR2".as_slice(),
                3 => b"CKR3".as_slice(),
                4 => b"CKR4".as_slice(),
                5 => b"CKR5".as_slice(),
                _ => b"CKR6".as_slice(),
            },
            &[role.code()],
            &self.generation.to_be_bytes(),
            &self.domain,
            &self.stream,
        ]
        .concat())
    }
    /// Provisioning contract: KMS Encrypt this role-tagged plaintext under the
    /// exact context(). Keep its plaintext entirely outside the hostile parent.
    pub fn wrap(&self, role: Role, secret: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
        let mut b = Zeroizing::new(self.prefix(role)?);
        b.extend_from_slice(secret);
        if b.len() > 4096 {
            return Err(Error);
        }
        Ok(b)
    }
    fn unwrap(&self, role: Role, plain: Zeroizing<Vec<u8>>) -> Result<Zeroizing<Vec<u8>>, Error> {
        let slot = self.slots.iter().find(|s| s.role == role).ok_or(Error)?;
        let body = plain
            .strip_prefix(self.prefix(role)?.as_slice())
            .ok_or(Error)?;
        if body.is_empty() || sha256(body) != slot.plaintext_hash {
            return Err(Error);
        }
        Ok(Zeroizing::new(body.to_vec()))
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Host-visible KMS ciphertext only; its bytes never authorize an action.
pub struct Capsule {
    /// Role matched against the measured slot and decrypted prefix.
    pub role: Role,
    /// Bounded KMS ciphertext blob, not plaintext.
    pub ciphertext: Vec<u8>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Bounded parent bootstrap. No journal key, user directory or native seed here.
pub struct Bootstrap {
    /// Parent-known finite KMS/S3 credential, forbidden for the witness.
    pub parent: Credential,
    /// Exactly one ciphertext for each governed role.
    pub capsules: Vec<Capsule>,
}

/// Parent-only one-shot ciphertext/STS provider. Public data has no financial
/// authority; malicious or stale bootstrap still fails enclave release/restore.
pub fn serve_bootstrap(
    listener: crate::vsock::VsockListener,
    boot: Bootstrap,
) -> Result<(), Error> {
    use crate::transport::{Lifetime, Listener, Socket, write_frame};
    use std::{
        io::ErrorKind,
        time::{Duration, Instant},
    };
    let bytes = Zeroizing::new(serde_cbor::to_vec(&boot).map_err(|_| Error)?);
    if bytes.len() > 65536 {
        return Err(Error);
    }
    let start = Instant::now();
    loop {
        if start.elapsed() > Duration::from_secs(60) {
            return Err(Error);
        }
        match listener.accept() {
            Ok(mut socket) => {
                socket.prepare()?;
                let _life = Lifetime::new(socket.try_clone()?);
                return write_frame(&mut socket, &bytes, 65536);
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50))
            }
            Err(_) => return Err(Error),
        }
    }
}

/// One ephemeral RSA recipient per one-shot KMS operation. It is never exported,
/// reused, cloned or retained after release; old recipient replies cannot open.
struct Recipient {
    key: PKey<Private>,
}
impl Recipient {
    fn new() -> Result<Self, Error> {
        Ok(Self {
            key: PKey::from_rsa(Rsa::generate(2048)?)?,
        })
    }
    fn public(&self) -> Result<Vec<u8>, Error> {
        Ok(self.key.public_key_to_der()?)
    }
    fn decrypt(self, body: &Value, key_id: &str) -> Result<Zeroizing<Vec<u8>>, Error> {
        if body.get("KeyId").and_then(Value::as_str) != Some(key_id)
            || body.get("EncryptionAlgorithm").and_then(Value::as_str) != Some("SYMMETRIC_DEFAULT")
            || body
                .get("Plaintext")
                .is_some_and(|v| !v.is_null() && v.as_str() != Some(""))
        {
            return Err(Error);
        }
        let data = body
            .get("CiphertextForRecipient")
            .and_then(Value::as_str)
            .ok_or(Error)?;
        if data.len() > 8192 {
            return Err(Error);
        }
        let der = STANDARD.decode(data).map_err(|_| Error)?;
        if der.is_empty() || der.len() > 6144 {
            return Err(Error);
        }
        // KMS returns a CMS BER envelope, not necessarily byte-canonical DER.
        // Bound the complete single object BEFORE OpenSSL parses it. Never
        // accept a valid prefix plus trailing bytes or unbounded nesting.
        cms_frame(&der)?;
        let cms = CmsContentInfo::from_der(&der)?;
        // Only a KMS TLS-authenticated response to THIS OAEP-SHA256 request
        // reaches here; this is not a caller-accessible CMS decryption oracle.
        // KMS identifies the ephemeral key, not an X.509 recipient certificate.
        let plain = Zeroizing::new(cms.decrypt_without_cert_check(&self.key)?);
        if plain.is_empty() || plain.len() > 4096 {
            return Err(Error);
        }
        Ok(plain)
    }
}

fn cms_frame(bytes: &[u8]) -> Result<(), Error> {
    fn object(
        bytes: &[u8],
        at: &mut usize,
        limit: usize,
        depth: usize,
        count: &mut usize,
    ) -> Result<(), Error> {
        if depth > 16 || *count >= 256 || *at >= limit {
            return Err(Error);
        }
        *count += 1;
        let tag = bytes[*at];
        *at += 1;
        if tag == 0 || tag & 31 == 31 || *at >= limit {
            return Err(Error);
        }
        let length = bytes[*at];
        *at += 1;
        if length == 0x80 {
            if tag & 0x20 == 0 {
                return Err(Error);
            }
            loop {
                if *at + 2 > limit {
                    return Err(Error);
                }
                if bytes[*at..*at + 2] == [0, 0] {
                    *at += 2;
                    break;
                }
                object(bytes, at, limit, depth + 1, count)?;
            }
        } else {
            let mut n = usize::from(length);
            if length & 0x80 != 0 {
                let width = usize::from(length & 0x7f);
                if width == 0 || width > 4 || at.checked_add(width).ok_or(Error)? > limit {
                    return Err(Error);
                }
                n = 0;
                for _ in 0..width {
                    n = n
                        .checked_mul(256)
                        .and_then(|v| v.checked_add(usize::from(bytes[*at])))
                        .ok_or(Error)?;
                    *at += 1;
                }
            }
            let end = at.checked_add(n).ok_or(Error)?;
            if end > limit {
                return Err(Error);
            }
            if tag & 0x20 != 0 {
                while *at < end {
                    object(bytes, at, end, depth + 1, count)?;
                }
            } else {
                *at = end;
            }
        }
        Ok(())
    }
    if bytes.first() != Some(&0x30) || bytes.len() > 6144 {
        return Err(Error);
    }
    let mut at = 0;
    object(bytes, &mut at, bytes.len(), 0, &mut 0)?;
    if at != bytes.len() {
        return Err(Error);
    }
    Ok(())
}
/// Check exact measured role coverage before any recipient-KMS release.
fn validate_capsules(manifest: &Manifest, capsules: &[Capsule]) -> Result<(), Error> {
    if capsules.len() != manifest.slots.len()
        || capsules.iter().enumerate().any(|(i, c)| {
            c.ciphertext.is_empty()
                || c.ciphertext.len() > 6144
                || capsules[..i].iter().any(|o| o.role == c.role)
                || !manifest.slots.iter().any(|s| s.role == c.role)
        })
    {
        return Err(Error);
    }
    Ok(())
}
/// Release each measured role once to its own fresh recipient. Any error discards
/// accumulated material; no cached key or plaintext fallback.
pub fn release(
    manifest: &Manifest,
    nsm: Arc<Nsm>,
    boot: Bootstrap,
) -> Result<BTreeMap<Role, Zeroizing<Vec<u8>>>, Error> {
    manifest.validate()?;
    if nsm.policy().manifest != manifest.digest()?
        || nsm.policy().domain.as_slice() != manifest.domain
    {
        return Err(Error);
    }
    validate_capsules(manifest, &boot.capsules)?;
    let mut keys = BTreeMap::new();
    for capsule in boot.capsules {
        let slot = manifest
            .slots
            .iter()
            .find(|s| s.role == capsule.role)
            .ok_or(Error)?;
        let recipient = Recipient::new()?;
        let public = recipient.public()?;
        let context = manifest.context(capsule.role)?;
        let data = sha384(
            &[
                b"CINDER-KMS-RECIPIENT-1\0".as_slice(),
                &manifest.digest()?,
                capsule.role.name().as_bytes(),
                &sha256(&capsule.ciphertext),
            ]
            .concat(),
        );
        let document = nsm.recipient(&public, data)?;
        // Parent credentials are deliberately already parent-known and may only
        // decrypt with approved recipient PCR/context; they have NO witness right.
        let credentials = Credential {
            access: boot.parent.access.clone(),
            secret: boot.parent.secret.clone(),
            token: boot.parent.token.clone(),
            expires: boot.parent.expires,
        };
        let client = Client::new(slot.endpoint.clone(), credentials, nsm.clone())?;
        let response=client.json("TrentService.Decrypt",json!({"KeyId":slot.endpoint.resource,"EncryptionAlgorithm":"SYMMETRIC_DEFAULT","CiphertextBlob":STANDARD.encode(capsule.ciphertext),"EncryptionContext":context,"Recipient":{"KeyEncryptionAlgorithm":"RSAES_OAEP_SHA_256","AttestationDocument":STANDARD.encode(document)}}))?;
        let plain = recipient.decrypt(&response, &slot.endpoint.resource)?;
        keys.insert(capsule.role, manifest.unwrap(capsule.role, plain)?);
    }
    validate_keys(&keys)?;
    nsm.now()?;
    Ok(keys)
}
/// The same key-purpose check is used by recipient release and private runtime
/// construction. Test injection must not bypass production key separation.
pub(crate) fn validate_keys(keys: &BTreeMap<Role, Zeroizing<Vec<u8>>>) -> Result<(), Error> {
    let secrets: Vec<_> = keys
        .iter()
        .filter(|(r, _)| {
            matches!(
                r,
                Role::Storage | Role::Trading | Role::Broker | Role::Funds
            )
        })
        .map(|(_, k)| k)
        .collect();
    if secrets
        .iter()
        .any(|k| k.len() != 32 || k.as_slice() == [0; 32])
        || secrets.iter().enumerate().any(|(i, k)| {
            secrets[..i]
                .iter()
                .any(|old| old.as_slice() == k.as_slice())
        })
    {
        return Err(Error);
    }
    Ok(())
}

#[cfg(all(test, feature = "local-fixture"))]
pub(crate) fn qualification_manifest() -> Manifest {
    let root = crate::fixture::FixtureAttester::new()
        .unwrap()
        .root()
        .to_der()
        .unwrap();
    let hash = sha256(&root);
    let endpoint = |service: &str, resource: String, port| Endpoint {
        service: service.into(),
        region: "us-east-1".into(),
        resource,
        port,
        root: root.clone(),
        root_hash: hash,
    };
    Manifest {
        version: 1,
        demo_deposit: None,
        domain: [[1; 32], [2; 32]].concat(),
        application: [4; 32],
        stream: [5; 32],
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
        .map(|role| Slot {
            role,
            endpoint: endpoint(
                "kms",
                format!(
                    "arn:aws:kms:us-east-1:123456789012:key/00000000-0000-0000-0000-00000000000{}",
                    role.code()
                ),
                9006,
            ),
            plaintext_hash: sha256(&[role.code(); 32]),
        })
        .collect(),
        gates: Gates {
            trading: false,
            funding: false,
            native_reads: false,
            maximum_boot_ms: 60000,
        },
        chain: None,
        history: None,
        native_capture: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn funds_storage_trading_and_broker_seeds_are_nonzero_distinct_exact_seeds() {
        let roles = [Role::Storage, Role::Trading, Role::Broker, Role::Funds];
        let keys: BTreeMap<_, _> = roles
            .into_iter()
            .enumerate()
            .map(|(i, r)| (r, Zeroizing::new(vec![i as u8 + 1; 32])))
            .collect();
        validate_keys(&keys).unwrap();
        for role in roles {
            for invalid in [vec![0; 32], vec![1; 31], vec![1; 33]] {
                let mut changed = keys.clone();
                changed.insert(role, Zeroizing::new(invalid));
                assert!(validate_keys(&changed).is_err());
            }
            for other in roles {
                if role == other {
                    continue;
                }
                let mut changed = keys.clone();
                changed.insert(role, keys[&other].clone());
                assert!(validate_keys(&changed).is_err());
            }
        }
    }
    #[cfg(feature = "local-fixture")]
    #[test]
    fn version_two_binds_chain_trust_and_sixth_role_without_enabling_money_movement() {
        let old = qualification_manifest();
        old.validate().unwrap();
        let encoded = serde_cbor::to_vec(&old).unwrap();
        assert!(!encoded.windows(5).any(|x| x == b"chain"));
        let mut m = old.clone();
        m.version = 2;
        let mut funds = m.slots[0].clone();
        funds.role = Role::Funds;
        funds.endpoint.resource =
            "arn:aws:kms:us-east-1:123456789012:key/00000000-0000-0000-0000-000000000006".into();
        m.slots.push(funds);
        m.chain = Some(crate::chain_funding::Peer {
            host: "devnet.helius-rpc.com".into(),
            port: 9007,
            root: m.venue_root.clone(),
            root_hash: m.venue_root_hash,
            network: [1; 32],
        });
        m.gates.native_reads = true;
        m.validate().unwrap();
        // This is the exact pre-KMS guard used by release, not only manifest validation.
        for manifest in [&old, &m] {
            let capsules = || {
                manifest
                    .slots
                    .iter()
                    .map(|s| Capsule {
                        role: s.role,
                        ciphertext: vec![1],
                    })
                    .collect::<Vec<_>>()
            };
            let mut valid = capsules();
            valid.reverse();
            validate_capsules(manifest, &valid).unwrap();
            valid.pop();
            assert!(validate_capsules(manifest, &valid).is_err());
            let mut duplicate = capsules();
            duplicate[1].role = duplicate[0].role;
            assert!(validate_capsules(manifest, &duplicate).is_err());
            for bytes in [vec![], vec![1; 6145]] {
                let mut invalid = capsules();
                invalid[0].ciphertext = bytes;
                assert!(validate_capsules(manifest, &invalid).is_err());
            }
        }
        let mut foreign = old
            .slots
            .iter()
            .map(|s| Capsule {
                role: s.role,
                ciphertext: vec![1],
            })
            .collect::<Vec<_>>();
        foreign[0].role = Role::Funds;
        assert!(validate_capsules(&old, &foreign).is_err());
        assert_ne!(m.digest().unwrap(), old.digest().unwrap());
        assert!(m.wrap(Role::Funds, &[6; 32]).unwrap().starts_with(b"CKR2"));
        assert!(old.wrap(Role::Funds, &[6; 32]).is_err());
        let role = m.slots.iter_mut().find(|s| s.role == Role::Funds).unwrap();
        role.plaintext_hash = sha256(&[6; 32]);
        let own = m.wrap(Role::Funds, &[6; 32]).unwrap();
        m.unwrap(Role::Funds, own).unwrap();
        assert!(
            m.unwrap(Role::Funds, m.wrap(Role::Broker, &[6; 32]).unwrap())
                .is_err()
        );
        assert!(
            m.unwrap(Role::Broker, old.wrap(Role::Broker, &[4; 32]).unwrap())
                .is_err()
        );
        for case in 0..9 {
            let mut changed = m.clone();
            match case {
                0 => changed.gates.funding = true,
                1 => changed.gates.trading = true,
                2 => changed.chain = None,
                3 => changed.chain.as_mut().unwrap().port = changed.ingress,
                4 => changed.chain.as_mut().unwrap().network = [2; 32],
                5 => changed.chain.as_mut().unwrap().host = "mainnet.helius-rpc.com".into(),
                6 => changed.chain.as_mut().unwrap().root_hash = [2; 32],
                7 => {
                    changed.slots.pop();
                }
                _ => changed.version = 1,
            }
            assert!(changed.validate().is_err(), "case {case}");
        }
        let policy = HistoryPolicy {
            record_bytes: 65536,
            history_bytes: 8388608,
            records: 256,
            put_requests: 512,
            put_bytes: 134217728,
        };
        let mut packed = m.clone();
        packed.version = 3;
        packed.history = Some(policy.clone());
        packed.validate().unwrap();
        assert_ne!(packed.digest().unwrap(), m.digest().unwrap());
        assert!(
            packed
                .wrap(Role::Funds, &[6; 32])
                .unwrap()
                .starts_with(b"CKR3")
        );
        assert!(
            packed
                .unwrap(Role::Funds, m.wrap(Role::Funds, &[6; 32]).unwrap())
                .is_err()
        );
        let own = packed.wrap(Role::Funds, &[6; 32]).unwrap();
        packed.unwrap(Role::Funds, own).unwrap();
        for case in 0..12 {
            let mut changed = packed.clone();
            match case {
                0 => changed.history = None,
                1 => changed.version = 2,
                2 => changed.gates.funding = true,
                3 => changed.gates.trading = true,
                4 => changed.history.as_mut().unwrap().record_bytes = 65537,
                5 => changed.history.as_mut().unwrap().history_bytes = 8388609,
                6 => changed.history.as_mut().unwrap().records = 257,
                7 => changed.history.as_mut().unwrap().put_requests = 1025,
                8 => changed.history.as_mut().unwrap().put_bytes = 134217729,
                9 => changed.history.as_mut().unwrap().put_bytes = 1,
                10 => changed.history.as_mut().unwrap().records = 0,
                _ => changed.chain = None,
            }
            assert!(changed.validate().is_err(), "pack policy case {case}");
        }
        for field in 0..5 {
            let mut changed = packed.clone();
            let p = changed.history.as_mut().unwrap();
            match field {
                0 => p.record_bytes -= 1,
                1 => p.history_bytes -= 1,
                2 => p.records -= 1,
                3 => p.put_requests -= 1,
                _ => p.put_bytes -= 1,
            }
            assert_ne!(packed.digest().unwrap(), changed.digest().unwrap());
        }
        let mut captured = packed.clone();
        captured.version = 4;
        captured.native_capture = Some(crate::native_capture::Policy {
            port: 9008,
            root: captured.venue_root.clone(),
            root_hash: captured.venue_root_hash,
            limits: cinder_pacifica::capture::Limits {
                maximum_ms: 5000,
                maximum_messages: 16,
                maximum_bytes: 8192,
                maximum_message: 4096,
            },
        });
        captured.validate().unwrap();
        assert_ne!(captured.digest().unwrap(), packed.digest().unwrap());
        assert!(
            captured
                .wrap(Role::Funds, &[6; 32])
                .unwrap()
                .starts_with(b"CKR4")
        );
        captured
            .unwrap(Role::Funds, captured.wrap(Role::Funds, &[6; 32]).unwrap())
            .unwrap();
        assert!(
            captured
                .unwrap(Role::Funds, packed.wrap(Role::Funds, &[6; 32]).unwrap())
                .is_err()
        );
        for legacy in [1, 2, 3] {
            let mut changed = captured.clone();
            changed.version = legacy;
            assert!(changed.validate().is_err());
        }
        for case in 0..15 {
            let mut changed = captured.clone();
            let p = changed.native_capture.as_mut().unwrap();
            match case {
                0 => p.port = changed.ingress,
                1 => p.port = changed.bootstrap,
                2 => p.port = changed.venue_port,
                3 => p.port = changed.chain.as_ref().unwrap().port,
                4 => p.port = changed.first.port,
                5 => p.port = changed.slots[0].endpoint.port,
                6 => p.root_hash = [2; 32],
                7 => p.limits.maximum_ms = 30001,
                8 => p.limits.maximum_messages = 257,
                9 => p.limits.maximum_bytes = 1048577,
                10 => p.limits.maximum_message = 16384,
                11 => changed.gates.native_reads = false,
                12 => changed.gates.maximum_boot_ms = 4999,
                13 => changed.native_capture = None,
                _ => changed.gates.funding = true,
            }
            assert!(changed.validate().is_err(), "capture case {case}");
        }
        for case in 0..5 {
            let mut changed = captured.clone();
            let p = changed.native_capture.as_mut().unwrap();
            match case {
                0 => p.port += 1,
                1 => p.limits.maximum_ms -= 1,
                2 => p.limits.maximum_messages -= 1,
                3 => p.limits.maximum_bytes -= 1,
                _ => p.limits.maximum_message -= 1,
            }
            assert_ne!(captured.digest().unwrap(), changed.digest().unwrap());
        }
    }
    #[test]
    fn recipient_rejects_plaintext_and_wrong_key() {
        let r = Recipient::new().unwrap();
        let public = r.public().unwrap();
        assert!(
            r.decrypt(&json!({"KeyId":"wrong","Plaintext":"abc"}), "right")
                .is_err()
        );
        assert_ne!(public, Recipient::new().unwrap().public().unwrap());
    }
    #[cfg(feature = "local-fixture")]
    #[test]
    fn demo_manifest_is_explicit_bounded_and_separate_from_capture_and_financial_activation() {
        let mut m = qualification_manifest();
        m.version = 5;
        m.gates.native_reads = true;
        let mut funds = m.slots[0].clone();
        funds.role = Role::Funds;
        funds.plaintext_hash = sha256(&[6; 32]);
        funds.endpoint.resource =
            "arn:aws:kms:us-east-1:123456789012:key/00000000-0000-0000-0000-000000000006".into();
        m.slots.push(funds);
        m.chain = Some(crate::chain_funding::Peer {
            host: "api.devnet.solana.com".into(),
            port: 9007,
            root: m.venue_root.clone(),
            root_hash: m.venue_root_hash,
            network: [1; 32],
        });
        m.history = Some(HistoryPolicy {
            record_bytes: 65536,
            history_bytes: 8388608,
            records: 256,
            put_requests: 512,
            put_bytes: 134217728,
        });
        m.demo_deposit = Some(cinder_pacifica::funding::demo::Policy {
            revision: 1,
            maximum_reads: 8,
            maximum_pages: 2,
            interval_ms: 1000,
            maximum_backoff_ms: 5000,
            lifetime_ms: 60000,
            initial_setup: None,
        });
        m.validate().unwrap();
        assert!(m.wrap(Role::Funds, &[6; 32]).unwrap().starts_with(b"CKR5"));
        m.unwrap(Role::Funds, m.wrap(Role::Funds, &[6; 32]).unwrap())
            .unwrap();
        for case in 0..17 {
            let mut changed = m.clone();
            match case {
                0 => changed.demo_deposit = None,
                1 => changed.version = 3,
                2 => changed.version = 4,
                3 => changed.gates.native_reads = false,
                4 => changed.gates.funding = true,
                5 => changed.gates.trading = true,
                6 => changed.gates.maximum_boot_ms = 59999,
                7 => changed.history.as_mut().unwrap().record_bytes = 40959,
                8 => changed.chain = None,
                9 => changed.demo_deposit.as_mut().unwrap().revision = 0,
                10 => changed.demo_deposit.as_mut().unwrap().maximum_reads = 65,
                11 => changed.demo_deposit.as_mut().unwrap().maximum_pages = 9,
                12 => changed.demo_deposit.as_mut().unwrap().interval_ms = 999,
                13 => changed.demo_deposit.as_mut().unwrap().maximum_backoff_ms = 999,
                14 => changed.demo_deposit.as_mut().unwrap().lifetime_ms = 999,
                15 => changed.version = 6,
                _ => {
                    changed.native_capture = Some(crate::native_capture::Policy {
                        port: 9008,
                        root: changed.venue_root.clone(),
                        root_hash: changed.venue_root_hash,
                        limits: cinder_pacifica::capture::Limits {
                            maximum_ms: 1000,
                            maximum_messages: 2,
                            maximum_bytes: 2048,
                            maximum_message: 1024,
                        },
                    })
                }
            }
            assert!(changed.validate().is_err(), "demo manifest case {case}");
        }
        let mut legacy = m.clone();
        legacy.version = 3;
        legacy.demo_deposit = None;
        assert_ne!(legacy.digest().unwrap(), m.digest().unwrap());
        let mut financial = m.clone();
        financial.version = 6;
        financial.gates.funding = true;
        financial.demo_deposit.as_mut().unwrap().initial_setup =
            Some(cinder_pacifica::funding::demo::InitialSetup {
                revision: 1,
                exclusive_control: true,
            });
        financial.validate().unwrap();
        assert!(
            financial
                .wrap(Role::Funds, &[6; 32])
                .unwrap()
                .starts_with(b"CKR6")
        );
        assert!(
            financial
                .unwrap(Role::Funds, m.wrap(Role::Funds, &[6; 32]).unwrap())
                .is_err()
        );
        for case in 0..5 {
            let mut changed = financial.clone();
            match case {
                0 => changed.gates.funding = false,
                1 => changed.gates.trading = true,
                2 => changed.gates.native_reads = false,
                3 => changed.demo_deposit.as_mut().unwrap().initial_setup = None,
                _ => changed.version = 5,
            }
            assert!(changed.validate().is_err());
        }
        assert!(
            m.unwrap(Role::Funds, legacy.wrap(Role::Funds, &[6; 32]).unwrap())
                .is_err()
        );
        let value = serde_json::to_value(&legacy).unwrap();
        assert!(value.get("demo_deposit").is_none());
        let original = m.digest().unwrap();
        let context = m.context(Role::Funds).unwrap();
        m.demo_deposit.as_mut().unwrap().initial_setup =
            Some(cinder_pacifica::funding::demo::InitialSetup {
                revision: 1,
                exclusive_control: true,
            });
        m.validate().unwrap();
        assert_ne!(m.digest().unwrap(), original);
        // KMS authenticates the full policy via encryption context; the role
        // plaintext prefix deliberately binds only domain/stream/generation.
        assert_ne!(m.context(Role::Funds).unwrap(), context);
        for case in 0..2 {
            let mut changed = m.clone();
            let setup = changed
                .demo_deposit
                .as_mut()
                .unwrap()
                .initial_setup
                .as_mut()
                .unwrap();
            if case == 0 {
                setup.revision = 0;
            } else {
                setup.exclusive_control = false;
            }
            assert!(changed.validate().is_err());
        }
    }
    #[test]
    fn cms_framing_accepts_bounded_ber_but_not_ambiguous_or_incomplete_objects() {
        for valid in [
            vec![0x30, 3, 4, 1, 7],
            vec![0x30, 0x80, 4, 1, 7, 0, 0],
            vec![0x30, 0x80, 0x30, 0x80, 4, 1, 7, 0, 0, 0, 0],
        ] {
            assert!(cms_frame(&valid).is_ok());
            for n in 0..valid.len() {
                assert!(cms_frame(&valid[..n]).is_err());
            }
            let mut trailing = valid;
            trailing.push(0);
            assert!(cms_frame(&trailing).is_err());
        }
        for invalid in [
            vec![0x30, 0x80, 4, 0x80, 0, 0, 0, 0],
            vec![0x30, 2, 0, 0],
            vec![0x30, 0x80, 0x1f, 0, 0, 0],
            vec![0x30, 0x85, 0, 0, 0, 0, 0],
            vec![0x30, 0x84, 0xff, 0xff, 0xff, 0xff],
            vec![0x30, 3, 4, 2, 7],
        ] {
            assert!(cms_frame(&invalid).is_err());
        }
        let deep = [[0x30, 0x80].repeat(18), [0, 0].repeat(18)].concat();
        assert!(cms_frame(&deep).is_err());
        let many = [vec![0x30, 0x80], [4, 0].repeat(256), vec![0, 0]].concat();
        assert!(cms_frame(&many).is_err());
    }
    #[test]
    fn recipient_is_one_shot_and_old_envelopes_do_not_open_with_a_new_key() {
        use openssl::{
            asn1::Asn1Time,
            cms::CMSOptions,
            hash::MessageDigest,
            stack::Stack,
            symm::Cipher,
            x509::{X509, X509NameBuilder},
        };
        let old = Recipient::new().unwrap();
        let mut name = X509NameBuilder::new().unwrap();
        name.append_entry_by_text("CN", "disposable recipient")
            .unwrap();
        let name = name.build();
        let mut cert = X509::builder().unwrap();
        cert.set_version(2).unwrap();
        cert.set_subject_name(&name).unwrap();
        cert.set_issuer_name(&name).unwrap();
        cert.set_pubkey(&old.key).unwrap();
        cert.set_not_before(Asn1Time::from_unix(0).unwrap().as_ref())
            .unwrap();
        cert.set_not_after(Asn1Time::from_unix(2_000_000_000).unwrap().as_ref())
            .unwrap();
        cert.sign(&old.key, MessageDigest::sha256()).unwrap();
        let mut certs = Stack::new().unwrap();
        certs.push(cert.build()).unwrap();
        // Generic synthetic CMS validates recipient/key lifetime, NOT AWS's real
        // OAEP envelope profile. Real KMS OAEP/NSM compatibility needs hardware.
        let der = CmsContentInfo::encrypt(
            &certs,
            b"private role material",
            Cipher::aes_256_cbc(),
            CMSOptions::BINARY,
        )
        .unwrap()
        .to_der()
        .unwrap();
        // A CMS envelope may use BER's indefinite outer sequence without
        // changing its cryptographic contents. Canonical re-encoding is not a
        // valid reason to reject KMS's authenticated recipient response.
        let width = if der[1] & 0x80 == 0 {
            0
        } else {
            usize::from(der[1] & 0x7f)
        };
        let ber = [&[0x30, 0x80][..], &der[2 + width..], &[0, 0][..]].concat();
        let ber_response = json!({"KeyId":"expected","EncryptionAlgorithm":"SYMMETRIC_DEFAULT","CiphertextForRecipient":STANDARD.encode(ber)});
        // Test-only clone permits checking two encodings of the same synthetic
        // envelope; the production recipient remains consumed exactly once.
        let ber_key = Recipient {
            key: old.key.clone(),
        };
        assert_eq!(
            ber_key
                .decrypt(&ber_response, "expected")
                .unwrap()
                .as_slice(),
            b"private role material"
        );
        let response = json!({"KeyId":"expected","EncryptionAlgorithm":"SYMMETRIC_DEFAULT","CiphertextForRecipient":STANDARD.encode(der)});
        assert!(
            Recipient::new()
                .unwrap()
                .decrypt(&response, "expected")
                .is_err()
        );
        assert_eq!(
            old.decrypt(&response, "expected").unwrap().as_slice(),
            b"private role material"
        );
    }
    #[cfg(feature = "local-fixture")]
    #[test]
    fn measured_manifest_cannot_echo_bad_ports_keys_roots_or_live_permissions() {
        let m = qualification_manifest();
        let digest = m.digest().unwrap();
        for i in 0..10 {
            let mut changed = m.clone();
            match i {
                0 => changed.gates.trading = true,
                1 => changed.gates.funding = true,
                2 => changed.gates.native_reads = true,
                3 => changed.epoch = 0,
                4 => changed.first = changed.second.clone(),
                5 => changed.slots[1] = changed.slots[0].clone(),
                6 => changed.slots[0].endpoint.port = changed.ingress,
                7 => changed.ingress = 0,
                8 => changed.venue_root_hash = [1; 32],
                _ => changed.slots[0].endpoint.resource = "alias/master".into(),
            };
            assert!(changed.validate().is_err());
        }
        for change in [1, 2, 3, 4] {
            let mut changed = m.clone();
            match change {
                1 => changed.epoch += 1,
                2 => changed.generation += 1,
                3 => changed.gates.maximum_boot_ms -= 1,
                _ => changed.stream[0] ^= 1,
            };
            assert_ne!(changed.digest().unwrap(), digest);
        }
        let body = vec![Role::Storage.code(); 32];
        let plain = m.wrap(Role::Storage, &body).unwrap();
        assert_eq!(
            m.unwrap(Role::Storage, plain.clone()).unwrap().as_slice(),
            body
        );
        assert!(m.unwrap(Role::Trading, plain.clone()).is_err());
        let mut changed = m.clone();
        changed.generation += 1;
        assert!(changed.unwrap(Role::Storage, plain.clone()).is_err());
        let mut corrupt = plain.clone();
        *corrupt.last_mut().unwrap() ^= 1;
        assert!(m.unwrap(Role::Storage, corrupt).is_err());
        assert_ne!(
            m.context(Role::Trading).unwrap(),
            m.context(Role::Broker).unwrap()
        );
    }
}

// Explicitly ignored hardware tests are a separate measured test executable.
// They are absent from every default-feature application/library artifact.
#[cfg(test)]
#[path = "hardware.rs"]
mod hardware;
