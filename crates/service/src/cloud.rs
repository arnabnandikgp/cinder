//! Closed AWS HTTPS ports. SigV4 is Amazon's pinned signer, not a home-grown
//! implementation. No SDK default credential chain, retry, endpoint discovery,
//! host TLS termination or environment override is used.
use crate::{
    Error,
    egress::Trust,
    transport::{Clock, Lifetime, Socket},
    vsock::{Target, VsockStream},
};
use aws_credential_types::Credentials;
use aws_sigv4::{
    http_request::{PayloadChecksumKind, SignableBody, SignableRequest, SigningSettings, sign},
    sign::v4,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use cinder_journal::{
    Frame, Head,
    model::PrivateBytes,
    replicated::{Anchor, Replica, Stream, Witness},
};
use openssl::sha::sha256;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    sync::Arc,
    time::{Duration, Instant, UNIX_EPOCH},
};
use zeroize::Zeroizing;

const MAX_BODY: usize = cinder_journal::wire::MAX_RECORD + 76;
const MAX_HEADER: usize = 16384;

/// Temporary scoped credentials. Never persisted, printed or discovered from
/// the enclave environment. Witness credentials must not be known by the parent.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Credential {
    /// Explicit STS access identifier, not a customer key.
    pub access: String,
    /// Secret signing key; zeroized on drop.
    pub secret: String,
    /// Required STS session token; no long-lived credential fallback.
    pub token: String,
    /// Unix millisecond expiry verified with the enclave clock.
    pub expires: u64,
}
impl Drop for Credential {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.access.zeroize();
        self.secret.zeroize();
        self.token.zeroize();
    }
}
impl Credential {
    fn identity(&self, now: u64) -> Result<aws_credential_types::Credentials, Error> {
        if self.expires <= now.saturating_add(10_000)
            || self.expires - now > 86_400_000
            || self.access.len() < 16
            || self.access.len() > 128
            || self.secret.len() < 32
            || self.secret.len() > 128
            || self.token.is_empty()
            || self.token.len() > 8192
            || [&self.access, &self.secret, &self.token]
                .iter()
                .any(|s| s.bytes().any(|b| !(33..=126).contains(&b)))
        {
            return Err(Error);
        }
        Ok(Credentials::new(
            &self.access,
            &self.secret,
            Some(self.token.clone()),
            Some(UNIX_EPOCH + Duration::from_millis(self.expires)),
            "cinder-explicit",
        ))
    }
}
/// Measured routing/trust configuration, not caller-selected endpoint URLs.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    /// Closed service: kms, s3 or dynamodb.
    pub service: String,
    /// Explicit allowed US region; no endpoint discovery.
    pub region: String,
    /// Exact key ARN, bucket or table name.
    pub resource: String,
    /// Parent CID 3 routing port, never a caller URL.
    pub port: u32,
    /// Canonical public DER trust anchor.
    pub root: Vec<u8>,
    /// Independently approved digest of the consumed trust anchor.
    pub root_hash: [u8; 32],
}
impl Endpoint {
    /// Validate the closed route and derive its exact TLS/SigV4 hostname.
    pub fn host(&self) -> Result<String, Error> {
        if self.region != "us-east-1" && self.region != "us-west-2" {
            return Err(Error);
        }
        Target::new(3, self.port)?;
        match self.service.as_str() {
            "kms" if key_arn(&self.resource, &self.region) => {
                Ok(format!("kms.{}.amazonaws.com", self.region))
            }
            "dynamodb" if valid_name(&self.resource) => {
                Ok(format!("dynamodb.{}.amazonaws.com", self.region))
            }
            "s3" if valid_name(&self.resource)
                && self.resource.len() <= 63
                && !self.resource.starts_with('-')
                && !self.resource.ends_with('-')
                && self
                    .resource
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') =>
            {
                Ok(format!(
                    "{}.s3.{}.amazonaws.com",
                    self.resource, self.region
                ))
            }
            _ => Err(Error),
        }
    }
    /// Hash every actually consumed route and trust field.
    pub fn commitment(&self) -> Result<[u8; 32], Error> {
        self.host()?;
        Trust::from_der(&self.root, self.root_hash)?;
        Ok(sha256(&serde_cbor::to_vec(self).map_err(|_| Error)?))
    }
}
fn valid_name(s: &str) -> bool {
    (3..=128).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}
fn key_arn(arn: &str, region: &str) -> bool {
    let Some(rest) = arn.strip_prefix(&format!("arn:aws:kms:{region}:")) else {
        return false;
    };
    let Some((account, id)) = rest.split_once(":key/") else {
        return false;
    };
    account.len() == 12
        && account.bytes().all(|b| b.is_ascii_digit())
        && id.len() == 36
        && id.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

/// Authenticated one-shot enclave HTTPS client with no ambient providers/retries.
pub struct Client {
    endpoint: Endpoint,
    trust: Trust,
    credentials: Credential,
    clock: Arc<dyn Clock>,
}
impl Client {
    /// Bind the measured endpoint, finite explicit credential and trusted clock.
    pub fn new(
        endpoint: Endpoint,
        credentials: Credential,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, Error> {
        endpoint.commitment()?;
        credentials.identity(clock.now()?)?;
        let trust = Trust::from_der(&endpoint.root, endpoint.root_hash)?;
        Ok(Self {
            endpoint,
            trust,
            credentials,
            clock,
        })
    }
    /// Actual loaded public endpoint contract, not a reported digest.
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }
    fn call(
        &mut self,
        method: &str,
        path: &str,
        headers: Vec<(&str, &str)>,
        body: &[u8],
    ) -> Result<Http, Error> {
        let start = Instant::now();
        let now = self.clock.now()?;
        let wire = signed(
            &self.endpoint,
            &self.credentials,
            now,
            method,
            path,
            &headers,
            body,
        )?;
        let socket = VsockStream::connect(Target::new(3, self.endpoint.port)?)?;
        let response = exchange(socket, &self.trust, &self.endpoint.host()?, now, &wire)?;
        let end = self.clock.now()?;
        if end < now || end - now > 10_000 || start.elapsed() > Duration::from_secs(10) {
            return Err(Error);
        }
        Ok(response)
    }
    pub(crate) fn json(&mut self, target: &str, body: Value) -> Result<Value, Error> {
        let b = Zeroizing::new(serde_json::to_vec(&body).map_err(|_| Error)?);
        let content_type = if self.endpoint.service == "dynamodb" {
            "application/x-amz-json-1.0"
        } else {
            "application/x-amz-json-1.1"
        };
        let r = self.call(
            "POST",
            "/",
            vec![("content-type", content_type), ("x-amz-target", target)],
            &b,
        )?;
        if r.status != 200 {
            return Err(Error);
        }
        serde_json::from_slice(&r.body).map_err(|_| Error)
    }
}
fn signed(
    e: &Endpoint,
    c: &Credential,
    now: u64,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> Result<Zeroizing<Vec<u8>>, Error> {
    if !matches!(method, "POST" | "PUT" | "GET")
        || body.len() > MAX_BODY
        || !path.starts_with('/')
        || path.len() > 512
        || path
            .bytes()
            .any(|b| !(33..=126).contains(&b) || b"?#%".contains(&b))
    {
        return Err(Error);
    }
    let hostname = e.host()?;
    let url = format!("https://{hostname}{path}");
    let mut headers = headers.to_vec();
    headers.push(("host", &hostname));
    if headers.len() > 16
        || headers.iter().any(|(k, v)| {
            k.is_empty()
                || k.bytes().any(|b| !b.is_ascii_lowercase() && b != b'-')
                || v.len() > 8192
                || v.bytes().any(|b| !(32..=126).contains(&b))
        })
    {
        return Err(Error);
    }
    let identity = c.identity(now)?.into();
    let mut settings = SigningSettings::default();
    // S3's SigV4 profile requires the exact payload digest on the wire, even
    // for an empty GET. The generic AWS signer's default omits this header.
    if e.service == "s3" {
        settings.payload_checksum_kind = PayloadChecksumKind::XAmzSha256;
    }
    let params = v4::SigningParams::builder()
        .identity(&identity)
        .region(&e.region)
        .name(&e.service)
        .time(UNIX_EPOCH + Duration::from_millis(now))
        .settings(settings)
        .build()
        .map_err(|_| Error)?
        .into();
    let request = SignableRequest::new(
        method,
        &url,
        headers.iter().copied(),
        SignableBody::Bytes(body),
    )
    .map_err(|_| Error)?;
    let (instructions, _) = sign(request, &params).map_err(|_| Error)?.into_parts();
    if !instructions.params().is_empty() {
        return Err(Error);
    }
    let mut wire = Zeroizing::new(format!("{method} {path} HTTP/1.1\r\n").into_bytes());
    for (k, v) in headers.iter().copied().chain(instructions.headers()) {
        wire.extend_from_slice(format!("{k}: {v}\r\n").as_bytes());
    }
    wire.extend_from_slice(
        format!(
            "Connection: close\r\nAccept-Encoding: identity\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )
        .as_bytes(),
    );
    wire.extend_from_slice(body);
    Ok(wire)
}
struct Http {
    status: u16,
    body: Zeroizing<Vec<u8>>,
}
fn exchange<S: Socket>(
    socket: S,
    trust: &Trust,
    host: &str,
    now: u64,
    wire: &[u8],
) -> Result<Http, Error> {
    let _life = Lifetime::new(socket.try_clone()?);
    let mut config = trust.connector()?.configure()?;
    config
        .param_mut()
        .set_time(i64::try_from(now / 1000).map_err(|_| Error)?);
    let mut tls = config.connect(host, socket).map_err(|_| Error)?;
    if tls.ssl().version_str() != "TLSv1.3" || tls.ssl().session_reused() {
        return Err(Error);
    }
    tls.write_all(wire)?;
    tls.flush()?;
    parse(&mut tls)
}
fn parse(reader: &mut impl Read) -> Result<Http, Error> {
    let mut h = Zeroizing::new(Vec::new());
    while !h.ends_with(b"\r\n\r\n") {
        if h.len() >= MAX_HEADER {
            return Err(Error);
        }
        let mut b = [0];
        reader.read_exact(&mut b)?;
        h.push(b[0]);
    }
    let t = std::str::from_utf8(&h).map_err(|_| Error)?;
    let mut lines = t.strip_suffix("\r\n\r\n").ok_or(Error)?.split("\r\n");
    let status = lines
        .next()
        .ok_or(Error)?
        .strip_prefix("HTTP/1.1 ")
        .ok_or(Error)?
        .split_once(' ')
        .ok_or(Error)?
        .0
        .parse::<u16>()
        .map_err(|_| Error)?;
    if !(200..=599).contains(&status) {
        return Err(Error);
    }
    let mut fields = BTreeMap::new();
    let mut field_count = 0;
    for line in lines {
        field_count += 1;
        let (k, v) = line.split_once(':').ok_or(Error)?;
        if field_count > 64
            || k.is_empty()
            || k.bytes().any(|b| !b.is_ascii_alphanumeric() && b != b'-')
            || !v.is_ascii()
            || v.bytes().any(|b| b < 32 || b == 127)
        {
            return Err(Error);
        }
        let name = k.to_ascii_lowercase();
        if matches!(
            name.as_str(),
            "content-length" | "transfer-encoding" | "content-encoding" | "location"
        ) && fields.insert(name, v.trim()).is_some()
        {
            return Err(Error);
        }
    }
    if fields.contains_key("transfer-encoding")
        || fields.contains_key("content-encoding")
        || fields.contains_key("location")
    {
        return Err(Error);
    }
    let n = fields.get("content-length").ok_or(Error)?;
    if n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error);
    }
    let n = n.parse::<usize>().map_err(|_| Error)?;
    if n > MAX_BODY {
        return Err(Error);
    }
    let mut body = Zeroizing::new(vec![0; n]);
    reader.read_exact(&mut body)?;
    Ok(Http { status, body })
}
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn stream_key(s: Stream) -> String {
    hex(&[
        s.domain.network.bytes().as_slice(),
        &s.domain.deployment.bytes(),
        &s.id,
    ]
    .concat())
}

/// Region-local strong read + exact conditional update. No global table,
/// eventually-consistent read, implicit registration, epoch write or retries.
pub struct DynamoWitness {
    client: Client,
    stream: Stream,
}
impl DynamoWitness {
    /// Bind a provisioned region-local stream; never creates or resets a row.
    pub fn new(client: Client, stream: Stream) -> Result<Self, Error> {
        if client.endpoint.service != "dynamodb" || stream.id == [0; 32] {
            return Err(Error);
        }
        Ok(Self { client, stream })
    }
}
fn read_request(table: &str, s: Stream) -> Value {
    json!({"TableName":table,"Key":{"Stream":{"S":stream_key(s)}},"ConsistentRead":true})
}
fn parse_anchor(v: &Value, s: Stream) -> Result<Anchor, Error> {
    let i = v.get("Item").and_then(Value::as_object).ok_or(Error)?;
    if i.get("Stream")
        .and_then(|v| v.get("S"))
        .and_then(Value::as_str)
        != Some(stream_key(s).as_str())
    {
        return Err(Error);
    }
    let integer = |name: &str| -> Result<u64, Error> {
        let text = i
            .get(name)
            .and_then(|v| v.get("N"))
            .and_then(Value::as_str)
            .ok_or(Error)?;
        let n = text.parse::<u64>().map_err(|_| Error)?;
        if n.to_string() != text {
            return Err(Error);
        }
        Ok(n)
    };
    let epoch = integer("Epoch")?;
    if epoch == 0 {
        return Err(Error);
    }
    let head = match (i.get("Sequence"), i.get("Hash")) {
        (None, None) => None,
        (Some(_), Some(v)) => Some(Head {
            sequence: integer("Sequence")?,
            hash: STANDARD
                .decode(v.get("B").and_then(Value::as_str).ok_or(Error)?)
                .map_err(|_| Error)?
                .try_into()
                .map_err(|_| Error)?,
        }),
        _ => return Err(Error),
    };
    if head.is_some_and(|h| h.hash == [0; 32]) {
        return Err(Error);
    }
    Ok(Anchor { epoch, head })
}
fn cas_request(table: &str, s: Stream, expected: Anchor, next: Head) -> Result<Value, Error> {
    if expected.epoch == 0
        || next.hash == [0; 32]
        || expected.head.map_or(next.sequence != 0, |h| {
            h.sequence.checked_add(1) != Some(next.sequence)
        })
    {
        return Err(Error);
    }
    let mut values = json!({":e":{"N":expected.epoch.to_string()},":s":{"N":next.sequence.to_string()},":h":{"B":STANDARD.encode(next.hash)}});
    let condition = if let Some(h) = expected.head {
        values[":old_s"] = json!({"N":h.sequence.to_string()});
        values[":old_h"] = json!({"B":STANDARD.encode(h.hash)});
        "#e = :e AND #s = :old_s AND #h = :old_h"
    } else {
        "#e = :e AND attribute_not_exists(#s) AND attribute_not_exists(#h)"
    };
    Ok(
        json!({"TableName":table,"Key":{"Stream":{"S":stream_key(s)}},"ConditionExpression":condition,"UpdateExpression":"SET #s = :s, #h = :h","ExpressionAttributeNames":{"#e":"Epoch","#s":"Sequence","#h":"Hash"},"ExpressionAttributeValues":values,"ReturnValues":"NONE"}),
    )
}
impl Witness for DynamoWitness {
    fn read(&mut self, s: Stream) -> Result<Anchor, cinder_journal::Error> {
        if s != self.stream {
            return Err(cinder_journal::Error::Stale);
        }
        let body = read_request(&self.client.endpoint.resource, s);
        self.client
            .json("DynamoDB_20120810.GetItem", body)
            .and_then(|v| parse_anchor(&v, s))
            .map_err(|_| cinder_journal::Error::Storage)
    }
    fn accept(
        &mut self,
        s: Stream,
        expected: Anchor,
        next: Head,
    ) -> Result<(), cinder_journal::Error> {
        if s != self.stream {
            return Err(cinder_journal::Error::Stale);
        }
        let body = cas_request(&self.client.endpoint.resource, s, expected, next)
            .map_err(|_| cinder_journal::Error::Invalid)?;
        self.client
            .json("DynamoDB_20120810.UpdateItem", body)
            .map(|_| ())
            .map_err(|_| cinder_journal::Error::Storage)
    }
}
/// Content-addressed S3 ciphertext. Two differently bound buckets are required
/// by Replicated; deployment independence is not established by distinct names.
pub struct S3Replica {
    client: Client,
    id: [u8; 32],
    prefix: String,
}
impl S3Replica {
    /// Bind the actual bucket/region identity and exact domain/stream prefix.
    pub fn new(client: Client, stream: Stream) -> Result<Self, Error> {
        if client.endpoint.service != "s3" || stream.id == [0; 32] {
            return Err(Error);
        }
        let id = sha256(
            format!(
                "cinder-s3:{}:{}",
                client.endpoint.region, client.endpoint.resource
            )
            .as_bytes(),
        );
        Ok(Self {
            client,
            id,
            prefix: stream_key(stream),
        })
    }
    fn path(&self, hash: [u8; 32]) -> String {
        format!("/{}/{}", self.prefix, hex(&hash))
    }
}
fn frame_bytes(f: &Frame) -> Result<Vec<u8>, Error> {
    if f.opaque.as_bytes().is_empty()
        || f.opaque.as_bytes().len() > cinder_journal::wire::MAX_RECORD
    {
        return Err(Error);
    }
    Ok([
        b"CFR1".as_slice(),
        &f.head.sequence.to_be_bytes(),
        &f.head.hash,
        &f.previous,
        f.opaque.as_bytes(),
    ]
    .concat())
}
fn frame_decode(bytes: &[u8], expected: [u8; 32]) -> Result<Frame, Error> {
    if bytes.len() <= 76
        || bytes.len() > MAX_BODY
        || &bytes[..4] != b"CFR1"
        || bytes[12..44] != expected
    {
        return Err(Error);
    }
    Ok(Frame {
        head: Head {
            sequence: u64::from_be_bytes(bytes[4..12].try_into().map_err(|_| Error)?),
            hash: expected,
        },
        previous: bytes[44..76].try_into().map_err(|_| Error)?,
        opaque: PrivateBytes::new(bytes[76..].to_vec()).map_err(|_| Error)?,
    })
}
impl Replica for S3Replica {
    fn identity(&self) -> [u8; 32] {
        self.id
    }
    fn get(&mut self, hash: [u8; 32]) -> Result<Frame, cinder_journal::Error> {
        let r = self
            .client
            .call("GET", &self.path(hash), vec![], &[])
            .map_err(|_| cinder_journal::Error::Storage)?;
        if r.status != 200 {
            return Err(cinder_journal::Error::Storage);
        }
        frame_decode(&r.body, hash).map_err(|_| cinder_journal::Error::Storage)
    }
    fn put(&mut self, f: &Frame) -> Result<(), cinder_journal::Error> {
        let bytes = frame_bytes(f).map_err(|_| cinder_journal::Error::Storage)?;
        let r = self
            .client
            .call(
                "PUT",
                &self.path(f.head.hash),
                vec![
                    ("content-type", "application/octet-stream"),
                    ("if-none-match", "*"),
                ],
                &bytes,
            )
            .map_err(|_| cinder_journal::Error::Storage)?;
        if r.status == 200 || r.status == 412 && self.get(f.head.hash)? == *f {
            Ok(())
        } else {
            Err(cinder_journal::Error::Storage)
        }
    }
}

// Hardware qualification needs to distinguish an authenticated AWS denial
// from a broken route. No status/body inspection API exists in production.
#[cfg(test)]
impl Client {
    pub(crate) fn qualification_json(
        &mut self,
        target: &str,
        body: Value,
    ) -> Result<(u16, Value), Error> {
        let bytes = Zeroizing::new(serde_json::to_vec(&body).map_err(|_| Error)?);
        let content_type = if self.endpoint.service == "dynamodb" {
            "application/x-amz-json-1.0"
        } else {
            "application/x-amz-json-1.1"
        };
        let response = self.call(
            "POST",
            "/",
            vec![("content-type", content_type), ("x-amz-target", target)],
            &bytes,
        )?;
        Ok((
            response.status,
            serde_json::from_slice(&response.body).map_err(|_| Error)?,
        ))
    }
}

// Exercise the actual wire builder, not a separately invented CAS query.
#[cfg(test)]
pub(crate) fn qualification_cas_request(
    table: &str,
    stream: Stream,
    expected: Anchor,
    next: Head,
) -> Result<Value, Error> {
    cas_request(table, stream, expected, next)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_unconsumed_cloud_headers_are_bounded() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nSet-Cookie: a=1\r\nset-cookie: b=2\r\nVary: Origin\r\nvary: Accept-Encoding\r\n\r\nx";
        assert_eq!(parse(&mut &response[..]).unwrap().body.as_slice(), b"x");
        assert!(
            parse(&mut &b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\ncontent-length: 0\r\n\r\n"[..])
                .is_err()
        );
        for count in [63, 64] {
            let mut response = String::from("HTTP/1.1 200 OK\r\nContent-Length: 0\r\n");
            for _ in 0..count {
                response.push_str("Vary: Origin\r\n");
            }
            response.push_str("\r\n");
            assert_eq!(parse(&mut response.as_bytes()).is_ok(), count == 63);
        }
    }
    #[test]
    fn strict_cloud_framing() {
        assert_eq!(
            parse(&mut &b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n"[..])
                .unwrap()
                .body
                .len(),
            0
        );
        for b in [
            b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nContent-Length: 1\r\n\r\nx".as_slice(),
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n",
            b"HTTP/1.1 302 Found\r\nLocation: x\r\nContent-Length: 0\r\n\r\n",
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nx",
        ] {
            assert!(parse(&mut &b[..]).is_err());
        }
    }
    #[test]
    fn witness_is_fresh_cas_never_an_epoch_reset() {
        let s = Stream {
            domain: cinder_kernel::identity::Domain {
                network: cinder_kernel::identity::NetworkId::new([1; 32]).unwrap(),
                deployment: cinder_kernel::identity::DeploymentId::new([2; 32]).unwrap(),
            },
            id: [3; 32],
        };
        assert_eq!(read_request("test", s)["ConsistentRead"], true);
        assert!(parse_anchor(&json!({}), s).is_err());
        let a = Anchor {
            epoch: 9,
            head: None,
        };
        let n = Head {
            sequence: 0,
            hash: [4; 32],
        };
        let q = cas_request("test", s, a, n).unwrap();
        assert_eq!(q["UpdateExpression"], "SET #s = :s, #h = :h");
        assert!(
            q["ConditionExpression"]
                .as_str()
                .unwrap()
                .contains("attribute_not_exists")
        );
        assert!(cas_request("test", s, a, Head { sequence: 1, ..n }).is_err());
        let a = Anchor {
            epoch: 9,
            head: Some(n),
        };
        let q = cas_request(
            "test",
            s,
            a,
            Head {
                sequence: 1,
                hash: [5; 32],
            },
        )
        .unwrap();
        assert_eq!(
            q["ExpressionAttributeValues"][":old_h"]["B"],
            STANDARD.encode(n.hash)
        );
        let item = json!({"Item":{"Stream":{"S":stream_key(s)},"Epoch":{"N":"9"},"Sequence":{"N":"0"},"Hash":{"B":STANDARD.encode(n.hash)}}});
        assert_eq!(parse_anchor(&item, s).unwrap(), a);
        for (name, value) in [
            ("Epoch", json!({"N":"09"})),
            ("Sequence", json!({"N":"-1"})),
            ("Hash", json!({"B":STANDARD.encode([0;32])})),
        ] {
            let mut bad = item.clone();
            bad["Item"][name] = value;
            assert!(parse_anchor(&bad, s).is_err());
        }
    }
    #[cfg(feature = "local-fixture")]
    #[test]
    fn explicit_credential_signs_exact_bytes_but_never_discovers_or_retries() {
        let endpoint = crate::boot::qualification_manifest().slots[0]
            .endpoint
            .clone();
        let now = 1_700_000_000_000;
        let credentials = Credential {
            access: "AKIDEXAMPLE123456".into(),
            secret: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY".into(),
            token: "disposable-session-token".into(),
            expires: now + 60_000,
        };
        let headers = [
            ("content-type", "application/x-amz-json-1.1"),
            ("x-amz-target", "TrentService.Decrypt"),
        ];
        let wire = signed(
            &endpoint,
            &credentials,
            now,
            "POST",
            "/",
            &headers,
            b"{\"test\":1}",
        )
        .unwrap();
        let text = std::str::from_utf8(&wire).unwrap();
        assert!(text.contains("AWS4-HMAC-SHA256"));
        assert!(text.contains("/us-east-1/kms/aws4_request"));
        assert!(!text.contains(&credentials.secret));
        assert!(text.ends_with("{\"test\":1}"));
        assert_ne!(
            wire,
            signed(
                &endpoint,
                &credentials,
                now,
                "POST",
                "/",
                &headers,
                b"{\"test\":2}"
            )
            .unwrap()
        );
        assert!(
            signed(
                &endpoint,
                &credentials,
                credentials.expires,
                "POST",
                "/",
                &headers,
                b"{}"
            )
            .is_err()
        );
        for path in [
            "//evil?token=x",
            "/\r\nInjected: yes",
            "/%2f",
            "https://evil",
        ] {
            assert!(signed(&endpoint, &credentials, now, "POST", path, &headers, b"{}").is_err());
        }
        let mut changed = endpoint.clone();
        changed.resource = "alias/master".into();
        assert!(changed.host().is_err());
        changed = endpoint;
        changed.region = "evil.example".into();
        assert!(changed.host().is_err());
        let s3 = crate::boot::qualification_manifest().first;
        for (method, body) in [
            ("PUT", b"opaque-ciphertext".as_slice()),
            ("GET", b"".as_slice()),
        ] {
            let wire = signed(&s3, &credentials, now, method, "/stream/digest", &[], body).unwrap();
            let text = std::str::from_utf8(&wire).unwrap();
            assert!(text.contains(&format!("x-amz-content-sha256: {}\r\n", hex(&sha256(body)))));
            assert!(text.contains("/us-east-1/s3/aws4_request"));
        }
    }
}
