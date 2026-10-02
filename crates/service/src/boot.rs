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
    /// Fund-moving native signing; P20 refuses activation.
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
        }
    }
    fn code(self) -> u8 {
        match self {
            Self::Configuration => 1,
            Self::Storage => 2,
            Self::Trading => 3,
            Self::Broker => 4,
            Self::Witness => 5,
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
/// Included verbatim in the measured image; no secrets/customer directory here.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Closed schema version (1).
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
    /// Complete five-role release policy, without plaintext secrets.
    pub slots: Vec<Slot>,
    /// Capability decisions and finite boot lease.
    pub gates: Gates,
}
impl Manifest {
    /// Refuse ambiguous routes, key-role reuse, missing trust or live activation.
    pub fn validate(&self) -> Result<(), Error> {
        if self.version!=1 || self.domain.len()!=64 || self.domain[..32]==[0;32] || self.domain[32..]==[0;32]
            || self.application==[0;32] || self.stream==[0;32] || self.generation==0 || self.epoch==0
            || self.slots.len()!=5 || self.first.service!="s3" || self.second.service!="s3" || self.witness.service!="dynamodb"
            || (self.first.resource==self.second.resource) || self.gates.maximum_boot_ms==0 || self.gates.maximum_boot_ms>3_600_000
            // P20 packages/test-gates only. Live reads/money movement need P23's
            // qualified chain/read ports and separately authorized manifest.
            || self.gates.trading || self.gates.funding || self.gates.native_reads
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
                b"CINDER-RUNTIME-MANIFEST-1\0".as_slice(),
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
        Ok([
            b"CKR1".as_slice(),
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
/// Release each role once to its own fresh recipient inside this measured boot.
/// Any error discards accumulated material; no cached key or plaintext fallback.
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
    if boot.capsules.len() != 5
        || boot.capsules.iter().enumerate().any(|(i, c)| {
            c.ciphertext.is_empty()
                || c.ciphertext.len() > 6144
                || boot.capsules[..i].iter().any(|o| o.role == c.role)
        })
    {
        return Err(Error);
    }
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
        let mut client = Client::new(slot.endpoint.clone(), credentials, nsm.clone())?;
        let response=client.json("TrentService.Decrypt",json!({"KeyId":slot.endpoint.resource,"EncryptionAlgorithm":"SYMMETRIC_DEFAULT","CiphertextBlob":STANDARD.encode(capsule.ciphertext),"EncryptionContext":context,"Recipient":{"KeyEncryptionAlgorithm":"RSAES_OAEP_SHA_256","AttestationDocument":STANDARD.encode(document)}}))?;
        let plain = recipient.decrypt(&response, &slot.endpoint.resource)?;
        keys.insert(capsule.role, manifest.unwrap(capsule.role, plain)?);
    }
    // Checked high-entropy secrets never alias across key roles.
    let secrets: Vec<_> = keys
        .iter()
        .filter(|(r, _)| matches!(r, Role::Storage | Role::Trading | Role::Broker))
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
    nsm.now()?;
    Ok(keys)
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
