//! Closed Nitro COSE profile plus full pinned-root X.509 validation. CBOR is
//! bounded and duplicate-free BEFORE entering the upstream COSE deserializer.
use crate::Error;
use aws_nitro_enclaves_cose::{CoseSign1, crypto::Openssl};
use openssl::{
    asn1::Asn1Time,
    nid::Nid,
    sha::{sha256, sha384},
    stack::Stack,
    x509::{
        X509, X509StoreContext,
        store::X509StoreBuilder,
        verify::{X509VerifyFlags, X509VerifyParam},
    },
};
use std::collections::BTreeSet;

/// Maximum complete quote, checked before any certificate/CBOR allocation.
pub const MAX_QUOTE: usize = 16_384;
/// Fresh quote age in client-clock milliseconds.
pub const MAX_QUOTE_AGE: u64 = 30_000;
/// Short maximum connection life; reconnect requires a new quote and signature.
pub const MAX_SESSION: u64 = 120_000;
const ROOT: &[u8] = include_bytes!("aws-root.pem");
const FINGERPRINT: [u8; 32] = [
    0x64, 0x1a, 0x03, 0x21, 0xa3, 0xe2, 0x44, 0xef, 0xe4, 0x56, 0x46, 0x31, 0x95, 0xd6, 0x06, 0x31,
    0x7e, 0xd7, 0xcd, 0xcc, 0x3c, 0x17, 0x56, 0xe0, 0x98, 0x93, 0xf3, 0xc6, 0x8f, 0x79, 0xbb, 0x5b,
];

/// Independently distributed release policy. Never learned from the relay/quote.
#[derive(Clone)]
pub struct Policy {
    /// Exact network/deployment identifiers, 32 bytes each.
    pub domain: [u8; 64],
    /// SHA-256 of the approved canonical release manifest.
    pub manifest: [u8; 32],
    /// Exact SHA-384 PCR0/1/2. All-zero/debug releases reject.
    pub pcrs: [[u8; 48]; 3],
}
impl Policy {
    /// Exact 240-byte client policy, with no optional defaults.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != 240 {
            return Err(Error);
        }
        let p = Self {
            domain: bytes[..64].try_into().map_err(|_| Error)?,
            manifest: bytes[64..96].try_into().map_err(|_| Error)?,
            pcrs: [
                bytes[96..144].try_into().map_err(|_| Error)?,
                bytes[144..192].try_into().map_err(|_| Error)?,
                bytes[192..240].try_into().map_err(|_| Error)?,
            ],
        };
        if p.domain[..32] == [0; 32]
            || p.domain[32..] == [0; 32]
            || p.manifest == [0; 32]
            || p.pcrs.contains(&[0; 48])
        {
            return Err(Error);
        }
        Ok(p)
    }
    /// Public policy encoding for independently shipped client configuration.
    pub fn encode(&self) -> Vec<u8> {
        [&self.domain[..], &self.manifest, &self.pcrs.concat()].concat()
    }
}
/// Observations from THIS live TLS socket and its fresh client challenge.
pub struct Context<'a> {
    /// Fresh unpredictable challenge, never reused on reconnect.
    pub nonce: [u8; 32],
    /// TLS 1.3 exporter from the exact carrying socket.
    pub exporter: [u8; 32],
    /// Boot identity in the public reply, also signed in user_data.
    pub boot: [u8; 32],
    /// Advertised expiry, also signed in user_data.
    pub expires: u64,
    /// Independently trusted client time in Unix milliseconds.
    pub now: u64,
    /// DER SPKI obtained from the carrying socket's leaf, not from a response field.
    pub spki: &'a [u8],
}
/// Canonical user_data digest, binding release/domain/boot/expiry/live session.
pub fn user_data(p: &Policy, c: &Context<'_>) -> [u8; 48] {
    sha384(
        &[
            b"CINDER-TLS-ATTESTATION-1\0".as_slice(),
            &p.domain,
            &p.manifest,
            &c.boot,
            &c.expires.to_be_bytes(),
            &c.exporter,
        ]
        .concat(),
    )
}
/// Application auth binding, independently derived at both live endpoints.
pub fn binding(p: &Policy, c: &Context<'_>) -> [u8; 32] {
    sha256(
        &[
            b"CINDER-TLS-SESSION-1\0".as_slice(),
            &user_data(p, c),
            &c.nonce,
        ]
        .concat(),
    )
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(n).ok_or(Error)?;
        let b = self.bytes.get(self.at..end).ok_or(Error)?;
        self.at = end;
        Ok(b)
    }
    fn header(&mut self, major: u8) -> Result<u64, Error> {
        let b = self.take(1)?[0];
        if b >> 5 != major {
            return Err(Error);
        }
        let v = match b & 31 {
            n @ 0..=23 => n as u64,
            n @ 24..=27 => {
                let count = 1_usize << (n - 24);
                let mut v = 0_u64;
                for b in self.take(count)? {
                    v = (v << 8) | *b as u64;
                }
                let minimum = match count {
                    1 => 24,
                    2 => 256,
                    4 => 65536,
                    _ => 4294967296,
                };
                if v < minimum {
                    return Err(Error);
                }
                v
            }
            _ => return Err(Error),
        };
        Ok(v)
    }
    fn bytes(&mut self, max: usize) -> Result<&'a [u8], Error> {
        let n = self.header(2)?;
        if n > max as u64 {
            return Err(Error);
        }
        self.take(n as usize)
    }
    fn text(&mut self, max: usize) -> Result<&'a str, Error> {
        let n = self.header(3)?;
        if n > max as u64 {
            return Err(Error);
        }
        std::str::from_utf8(self.take(n as usize)?).map_err(|_| Error)
    }
    fn done(&self) -> Result<(), Error> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(Error)
        }
    }
}
struct Document<'a> {
    timestamp: u64,
    certificate: &'a [u8],
    chain: Vec<&'a [u8]>,
    pcrs: [[u8; 48]; 3],
    spki: &'a [u8],
    nonce: &'a [u8],
    data: &'a [u8],
}
fn profile(bytes: &[u8]) -> Result<Document<'_>, Error> {
    if bytes.is_empty() || bytes.len() > MAX_QUOTE {
        return Err(Error);
    }
    let mut r = Reader::new(bytes);
    if bytes[0] == 0xd2 {
        r.take(1)?;
    }
    if r.header(4)? != 4 {
        return Err(Error);
    }
    // One protected algorithm and an empty unprotected map: no ambiguous keys.
    if r.bytes(16)? != [0xa1, 0x01, 0x38, 0x22] || r.header(5)? != 0 {
        return Err(Error);
    }
    let payload = r.bytes(MAX_QUOTE)?;
    if r.bytes(96)?.len() != 96 {
        return Err(Error);
    }
    r.done()?;
    let mut r = Reader::new(payload);
    if r.header(5)? != 9 {
        return Err(Error);
    }
    let mut seen = BTreeSet::new();
    let mut timestamp = None;
    let mut cert = None;
    let mut chain = Vec::new();
    let mut pcrs = [[0; 48]; 3];
    let mut spki = None;
    let mut nonce = None;
    let mut data = None;
    for _ in 0..9 {
        let key = r.text(32)?;
        if !seen.insert(key) {
            return Err(Error);
        }
        match key {
            "module_id" => {
                if r.text(256)?.is_empty() {
                    return Err(Error);
                }
            }
            "digest" => {
                if r.text(16)? != "SHA384" {
                    return Err(Error);
                }
            }
            "timestamp" => timestamp = Some(r.header(0)?),
            "certificate" => cert = Some(r.bytes(1024)?),
            "cabundle" => {
                let n = r.header(4)?;
                if n == 0 || n > 8 {
                    return Err(Error);
                }
                for _ in 0..n {
                    chain.push(r.bytes(1024)?);
                }
            }
            "pcrs" => {
                let n = r.header(5)?;
                if !(3..=32).contains(&n) {
                    return Err(Error);
                }
                let mut indices = BTreeSet::new();
                for _ in 0..n {
                    let i = r.header(0)?;
                    if i > 31 || !indices.insert(i) {
                        return Err(Error);
                    }
                    let v = r.bytes(48)?;
                    if v.len() != 48 {
                        return Err(Error);
                    }
                    if i < 3 {
                        pcrs[i as usize].copy_from_slice(v);
                    }
                }
            }
            "public_key" => spki = Some(r.bytes(1024)?),
            "nonce" => nonce = Some(r.bytes(32)?),
            "user_data" => data = Some(r.bytes(512)?),
            _ => return Err(Error),
        }
    }
    r.done()?;
    Ok(Document {
        timestamp: timestamp.ok_or(Error)?,
        certificate: cert.ok_or(Error)?,
        chain,
        pcrs,
        spki: spki.ok_or(Error)?,
        nonce: nonce.ok_or(Error)?,
        data: data.ok_or(Error)?,
    })
}
fn verify_root(bytes: &[u8], p: &Policy, c: &Context<'_>, root: &X509) -> Result<(), Error> {
    Policy::decode(&p.encode())?;
    let d = profile(bytes)?;
    if c.nonce == [0; 32]
        || c.exporter == [0; 32]
        || c.boot == [0; 32]
        || c.spki.is_empty()
        || d.timestamp > c.now
        || c.now - d.timestamp > MAX_QUOTE_AGE
        || c.expires <= c.now
        || c.expires > d.timestamp.checked_add(MAX_SESSION).ok_or(Error)?
        || d.pcrs != p.pcrs
        || d.spki != c.spki
        || d.nonce != c.nonce
        || d.data != user_data(p, c)
    {
        return Err(Error);
    }
    verify_signature(bytes, &d, c.now, root)
}

// Shared strict chain/signature validation. The externally consumed TLS verifier
// supplies independently trusted client time; ONLY the local NSM clock path may
// use a nonce-bound signed timestamp as its bootstrapping time source.
fn verify_signature(bytes: &[u8], d: &Document<'_>, at: u64, root: &X509) -> Result<(), Error> {
    let leaf = X509::from_der(d.certificate)?;
    if leaf.to_der()? != d.certificate {
        return Err(Error);
    }
    let mut stack = Stack::new()?;
    let now = Asn1Time::from_unix(i64::try_from(at / 1000).map_err(|_| Error)?)?;
    // Explicitly check the trust anchor too, not just path-building intermediates.
    for cert in std::iter::once(root.clone())
        .chain(std::iter::once(leaf.clone()))
        .chain(
            d.chain
                .iter()
                .map(|b| X509::from_der(b))
                .collect::<Result<Vec<_>, _>>()?,
        )
    {
        if cert.not_before() > now || cert.not_after() <= now {
            return Err(Error);
        }
    }
    if d.chain.first().ok_or(Error)? != &root.to_der()?.as_slice() {
        return Err(Error);
    }
    for b in d.chain.iter().skip(1).rev() {
        let cert = X509::from_der(b)?;
        if cert.to_der()? != *b {
            return Err(Error);
        }
        stack.push(cert)?;
    }
    let mut store = X509StoreBuilder::new()?;
    store.add_cert(root.clone())?;
    let mut params = X509VerifyParam::new()?;
    params.set_time(i64::try_from(at / 1000).map_err(|_| Error)?);
    params.set_depth(8);
    // Fixed minimum 128-bit certificate/key strength, not host-library defaults.
    params.set_auth_level(3);
    params.set_flags(X509VerifyFlags::X509_STRICT)?;
    store.set_param(&params)?;
    let mut ctx = X509StoreContext::new()?;
    if !ctx.init(&store.build(), &leaf, &stack, |c| c.verify_cert())? {
        return Err(Error);
    }
    let key = leaf.public_key()?;
    if key.ec_key()?.group().curve_name() != Some(Nid::SECP384R1) {
        return Err(Error);
    }
    let cose = CoseSign1::from_bytes(bytes).map_err(|_| Error)?;
    if !cose.verify_signature::<Openssl>(&key).map_err(|_| Error)? {
        return Err(Error);
    }
    Ok(())
}

fn aws_root() -> Result<X509, Error> {
    let root = X509::from_pem(ROOT)?;
    if sha256(&root.to_der()?) != FINGERPRINT {
        return Err(Error);
    }
    Ok(root)
}

// These are NOT a TLS public key or a user session. Distinct purpose binding
// prevents a clock sample from being accepted as a private-channel quote.
pub(crate) fn clock_binding(p: &Policy) -> ([u8; 32], [u8; 48]) {
    let encoded = p.encode();
    (
        sha256(&[b"CINDER-NSM-CLOCK-KEY-1\0".as_slice(), &encoded].concat()),
        sha384(&[b"CINDER-NSM-CLOCK-DATA-1\0".as_slice(), &encoded].concat()),
    )
}

fn verify_clock_root(
    bytes: &[u8],
    p: &Policy,
    nonce: &[u8; 32],
    root: &X509,
) -> Result<u64, Error> {
    Policy::decode(&p.encode())?;
    let d = profile(bytes)?;
    let (key, data) = clock_binding(p);
    if *nonce == [0; 32]
        || d.timestamp == 0
        || d.pcrs != p.pcrs
        || d.spki != key
        || d.nonce != nonce
        || d.data != data
    {
        return Err(Error);
    }
    // The timestamp is not trusted until BOTH the full pinned-root certificate
    // path and COSE signature have verified. The caller generates the nonce
    // inside NSM and bounds the complete local ioctl/verification operation.
    verify_signature(bytes, &d, d.timestamp, root)?;
    Ok(d.timestamp)
}

pub(crate) fn verify_clock(bytes: &[u8], p: &Policy, nonce: &[u8; 32]) -> Result<u64, Error> {
    verify_clock_root(bytes, p, nonce, &aws_root()?)
}

fn verify_local_root(bytes: &[u8], p: &Policy, c: &Context<'_>, root: &X509) -> Result<u64, Error> {
    let d = profile(bytes)?;
    if d.timestamp < c.now || d.timestamp - c.now > MAX_QUOTE_AGE {
        return Err(Error);
    }
    // A real quote is generated AFTER the server's initial clock sample. Do not
    // incorrectly compare its timestamp with that earlier cut as client "now".
    let current = Context {
        now: d.timestamp,
        ..*c
    };
    verify_root(bytes, p, &current, root)?;
    Ok(d.timestamp)
}

pub(crate) fn verify_local(bytes: &[u8], p: &Policy, c: &Context<'_>) -> Result<u64, Error> {
    verify_local_root(bytes, p, c, &aws_root()?)
}

/// Production path: only the independently pinned commercial AWS Nitro root.
pub fn verify(bytes: &[u8], p: &Policy, c: &Context<'_>) -> Result<(), Error> {
    verify_root(bytes, p, c, &aws_root()?)
}
/// Explicit fixture-only alternate root. Absent from default production builds.
#[cfg(feature = "local-fixture")]
pub fn verify_fixture(bytes: &[u8], p: &Policy, c: &Context<'_>, root: &X509) -> Result<(), Error> {
    verify_root(bytes, p, c, root)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "local-fixture")]
    #[test]
    fn local_clock_requires_signed_timestamp_fresh_nonce_and_exact_purpose() {
        use crate::{fixture::*, transport::Clock};
        let a = FixtureAttester::new().unwrap();
        let other = FixtureAttester::new().unwrap();
        let p = policy();
        let at = FixtureClock.now().unwrap();
        let nonce = [7; 32];
        let (key, data) = clock_binding(&p);
        let quote = a.quote_fields(&p, at, &key, &nonce, &data).unwrap();
        assert_eq!(verify_clock_root(&quote, &p, &nonce, a.root()), Ok(at));
        assert!(verify_clock(&quote, &p, &nonce).is_err()); // No fixture root in production.
        assert!(verify_clock_root(&quote, &p, &nonce, other.root()).is_err());
        assert!(verify_clock_root(&quote, &p, &[8; 32], a.root()).is_err());
        assert!(verify_clock_root(&quote, &p, &[0; 32], a.root()).is_err());
        for changed in [
            Policy {
                manifest: [8; 32],
                ..p.clone()
            },
            Policy {
                domain: [8; 64],
                ..p.clone()
            },
            Policy {
                pcrs: [[8; 48]; 3],
                ..p.clone()
            },
        ] {
            assert!(verify_clock_root(&quote, &changed, &nonce, a.root()).is_err());
        }
        for (at, key, nonce, data) in [
            (0, key.to_vec(), nonce.to_vec(), data.to_vec()),
            (
                at + 2 * 86_400_000,
                key.to_vec(),
                nonce.to_vec(),
                data.to_vec(),
            ),
            (at, vec![9; 32], nonce.to_vec(), data.to_vec()),
            (at, key.to_vec(), vec![9; 32], data.to_vec()),
            (at, key.to_vec(), nonce.to_vec(), vec![9; 48]),
        ] {
            let q = a.quote_fields(&p, at, &key, &nonce, &data).unwrap();
            assert!(verify_clock_root(&q, &p, &[7; 32], a.root()).is_err());
        }
        // Changing a signed timestamp, even within a valid certificate interval,
        // is not accepted merely because the CBOR parses and nonce matches.
        use serde_cbor::Value;
        let Value::Array(mut fields) = serde_cbor::from_slice::<Value>(&quote[1..]).unwrap() else {
            panic!()
        };
        let Value::Bytes(payload) = &fields[2] else {
            panic!()
        };
        let Value::Map(mut doc) = serde_cbor::from_slice::<Value>(payload).unwrap() else {
            panic!()
        };
        doc.insert(
            Value::Text("timestamp".into()),
            Value::Integer((at + 1) as i128),
        );
        fields[2] = Value::Bytes(serde_cbor::to_vec(&Value::Map(doc)).unwrap());
        let changed = serde_cbor::to_vec(&Value::Array(fields)).unwrap();
        assert!(verify_clock_root(&changed, &p, &nonce, a.root()).is_err());
    }
    #[cfg(feature = "local-fixture")]
    #[test]
    fn local_quote_can_follow_clock_cut_without_relaxing_client_freshness() {
        use crate::{
            fixture::*,
            transport::{Attester, Clock},
        };
        let a = FixtureAttester::new().unwrap();
        let p = policy();
        let now = FixtureClock.now().unwrap();
        let c = Context {
            nonce: [1; 32],
            exporter: [2; 32],
            boot: [3; 32],
            expires: now + 60_000,
            now,
            spki: b"fixture-public-key",
        };
        let quote = a.quote(&p, &Context { now: now + 1, ..c }).unwrap();
        assert_eq!(verify_local_root(&quote, &p, &c, a.root()), Ok(now + 1));
        assert!(verify_fixture(&quote, &p, &c, a.root()).is_err());
        verify_fixture(&quote, &p, &Context { now: now + 2, ..c }, a.root()).unwrap();
        for at in [now - 1, now + MAX_QUOTE_AGE + 1, c.expires] {
            let quote = a.quote(&p, &Context { now: at, ..c }).unwrap();
            assert!(verify_local_root(&quote, &p, &c, a.root()).is_err());
        }
        let (key, data) = clock_binding(&p);
        let clock = a.quote_fields(&p, now, &key, &c.nonce, &data).unwrap();
        assert!(verify_local_root(&clock, &p, &c, a.root()).is_err());
        let tls = a.quote(&p, &c).unwrap();
        assert!(verify_clock_root(&tls, &p, &c.nonce, a.root()).is_err());
        let mut corrupt = quote;
        *corrupt.last_mut().unwrap() ^= 1;
        assert!(verify_local_root(&corrupt, &p, &c, a.root()).is_err());
    }
    #[test]
    fn pinned_aws_root_matches_published_der_fingerprint() {
        assert_eq!(
            sha256(&X509::from_pem(ROOT).unwrap().to_der().unwrap()),
            FINGERPRINT
        );
    }
    #[test]
    fn cbor_indefinite_nonminimal_and_bounds_reject() {
        assert!(Reader::new(&[0x9f]).header(4).is_err());
        assert!(Reader::new(&[0x18, 0x01]).header(0).is_err());
        assert!(
            Reader::new(&[0x5a, 0xff, 0xff, 0xff, 0xff])
                .bytes(1024)
                .is_err()
        );
    }
    #[test]
    fn duplicate_field_and_protected_algorithm_reject_before_cose() {
        use serde_cbor::Value;
        let mut payload = vec![0xa9];
        for _ in 0..9 {
            payload.extend(serde_cbor::to_vec(&Value::Text("module_id".into())).unwrap());
            payload.extend(serde_cbor::to_vec(&Value::Text("fixture".into())).unwrap());
        }
        let cose = Value::Array(vec![
            Value::Bytes(vec![0xa1, 0x01, 0x38, 0x22]),
            Value::Map(Default::default()),
            Value::Bytes(payload),
            Value::Bytes(vec![0; 96]),
        ]);
        assert!(profile(&serde_cbor::to_vec(&cose).unwrap()).is_err());
        let mut bad = vec![0x84, 0x44, 0xa1, 0x01, 0x38, 0x21, 0xa0, 0x40, 0x58, 0x60];
        bad.extend([0; 96]);
        assert!(profile(&bad).is_err());
    }

    #[cfg(feature = "local-fixture")]
    #[test]
    fn duplicate_pcr_index_rejects_before_upstream_map_deserialization() {
        use crate::{
            fixture::*,
            transport::{Attester, Clock},
        };
        use serde_cbor::Value;
        let a = FixtureAttester::new().unwrap();
        let p = policy();
        let now = FixtureClock.now().unwrap();
        let c = Context {
            nonce: [1; 32],
            exporter: [2; 32],
            boot: [3; 32],
            expires: now + 60_000,
            now,
            spki: b"fixture-public-key",
        };
        let q = a.quote(&p, &c).unwrap();
        let Value::Array(mut outer) = serde_cbor::from_slice::<Value>(&q[1..]).unwrap() else {
            panic!()
        };
        let Value::Bytes(payload) = &outer[2] else {
            panic!()
        };
        let Value::Map(fields) = serde_cbor::from_slice::<Value>(payload).unwrap() else {
            panic!()
        };
        let mut duplicate = vec![0xa9];
        for (k, v) in fields {
            duplicate.extend(serde_cbor::to_vec(&k).unwrap());
            if k == Value::Text("pcrs".into()) {
                duplicate.push(0xa3);
                for (i, pcr) in [0, 0, 2].into_iter().zip(p.pcrs) {
                    duplicate.extend(serde_cbor::to_vec(&Value::Integer(i)).unwrap());
                    duplicate.extend(serde_cbor::to_vec(&Value::Bytes(pcr.to_vec())).unwrap());
                }
            } else {
                duplicate.extend(serde_cbor::to_vec(&v).unwrap());
            }
        }
        outer[2] = Value::Bytes(duplicate);
        assert!(profile(&serde_cbor::to_vec(&Value::Array(outer)).unwrap()).is_err());
    }
}
