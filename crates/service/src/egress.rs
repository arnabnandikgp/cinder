//! One-shot venue HTTPS inside the enclave. The parent sees TLS ciphertext only.
//! Unsupported HTTP framing and all uncertain outcomes remain Unknown, not retry.
use crate::{
    Error,
    transport::{Clock, Lifetime, Socket},
    vsock::{Target, VsockStream},
};
use cinder_journal::model::PrivateBytes;
use cinder_pacifica::{
    execution::{Origin, Outbound, Reply, Transport},
    observation::MAX_BODY,
};
use openssl::{
    sha::sha256,
    ssl::{SslConnector, SslMethod, SslOptions, SslSessionCacheMode, SslVerifyMode, SslVersion},
    x509::{X509, store::X509StoreBuilder},
};
use std::{
    collections::BTreeSet,
    io::{Read, Write},
    sync::Arc,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

const MAX_HEADERS: usize = 8192;
const MAX_FIELDS: usize = 64;
const DEADLINE: Duration = Duration::from_secs(10);

/// Actual consumed CA certificate, not a system-store or environment fallback.
/// Its expected digest must come from the independently approved runtime policy.
pub struct Trust {
    root: X509,
    digest: [u8; 32],
}
impl Trust {
    /// Validate bounded canonical DER against the independently expected digest.
    pub fn from_der(der: &[u8], expected: [u8; 32]) -> Result<Self, Error> {
        if der.is_empty()
            || der.len() > MAX_HEADERS
            || expected == [0; 32]
            || sha256(der) != expected
        {
            return Err(Error);
        }
        let root = X509::from_der(der)?;
        // Reject parser-accepted trailing data and noncanonical representations.
        if root.to_der()? != der {
            return Err(Error);
        }
        Ok(Self {
            root,
            digest: expected,
        })
    }
    fn connector(&self) -> Result<SslConnector, Error> {
        let mut store = X509StoreBuilder::new()?;
        store.add_cert(self.root.clone())?;
        let mut builder = SslConnector::builder(SslMethod::tls_client())?;
        // Replace, rather than augment, OpenSSL's default trust paths.
        builder.set_cert_store(store.build());
        builder.set_verify(SslVerifyMode::PEER);
        builder.set_verify_depth(5);
        builder.set_min_proto_version(Some(SslVersion::TLS1_3))?;
        builder.set_max_proto_version(Some(SslVersion::TLS1_3))?;
        builder.set_options(SslOptions::NO_TICKET);
        builder.set_session_cache_mode(SslSessionCacheMode::OFF);
        builder.set_max_early_data(0)?;
        builder.set_alpn_protos(b"\x08http/1.1")?;
        Ok(builder.build())
    }
}

/// Fixed origin and parent vsock route. No public TCP injection or URL override.
/// Construction is not permission to call a venue; Gateway/Controller still own
/// qualification, durable exposure and the one-shot signing capability.
pub struct Egress {
    origin: Origin,
    target: Target,
    trust: Trust,
    clock: Arc<dyn Clock>,
}
impl Egress {
    /// Select the consumed native origin, parent CID 3 port, root and clock.
    /// Does not open a socket, obtain credentials or activate native capabilities.
    pub fn new(
        origin: Origin,
        target: Target,
        trust: Trust,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, Error> {
        if target.is_enclave() {
            return Err(Error);
        }
        Ok(Self {
            origin,
            target,
            trust,
            clock,
        })
    }
    /// Bind the actual socket route, native origin and loaded root. Clock and
    /// capability policy are separate required components of the runtime manifest.
    pub fn commitment(&self) -> [u8; 32] {
        sha256(
            &[
                b"CINDER-VENUE-EGRESS-1\0".as_slice(),
                self.origin.url().as_bytes(),
                &self.target.encode(),
                &self.trust.digest,
            ]
            .concat(),
        )
    }
    fn once(&self, request: Outbound) -> Result<Reply, Error> {
        if request.origin() != self.origin {
            return Err(Error);
        }
        let start = Instant::now();
        let now = self.clock.now()?;
        let wire = encode(host(self.origin), request.path(), request.body())?;
        let socket = VsockStream::connect(self.target)?;
        let result = exchange(socket, &self.trust, host(self.origin), now, &wire, start)?;
        let received_at = self.clock.now()?;
        if received_at < now
            || received_at - now > DEADLINE.as_millis() as u64
            || start.elapsed() > DEADLINE
        {
            return Err(Error);
        }
        Ok(Reply::Response {
            status: result.status,
            body: result.body,
            received_at,
            retry_after_ms: result.retry_after_ms,
        })
    }
}
impl Transport for Egress {
    fn post(&mut self, request: Outbound) -> Reply {
        self.once(request).unwrap_or(Reply::Unknown)
    }
}
pub(crate) fn host(origin: Origin) -> &'static str {
    match origin {
        Origin::Testnet => "test-api.pacifica.fi",
        Origin::Mainnet => "api.pacifica.fi",
    }
}
fn encode(host: &str, path: &str, body: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
    if !matches!(host, "test-api.pacifica.fi" | "api.pacifica.fi")
        || !matches!(
            path,
            "/api/v1/orders/create" | "/api/v1/orders/cancel" | "/api/v1/account/withdraw"
        )
        || body.is_empty()
        || body.len() > MAX_BODY
    {
        return Err(Error);
    }
    let mut wire = Zeroizing::new(format!("POST {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nAccept: application/json\r\nAccept-Encoding: identity\r\nConnection: close\r\nContent-Length: {}\r\n\r\n", body.len()).into_bytes());
    wire.extend_from_slice(body);
    Ok(wire)
}
fn exchange<S: Socket>(
    socket: S,
    trust: &Trust,
    hostname: &str,
    now: u64,
    wire: &[u8],
    start: Instant,
) -> Result<Response, Error> {
    if now == 0 || start.elapsed() > DEADLINE {
        return Err(Error);
    }
    // A peer trickling TLS bytes cannot keep the synchronous handshake alive
    // indefinitely. This one-shot socket never enters the long session phase.
    let _lifetime = Lifetime::new(socket.try_clone()?);
    let mut configuration = trust.connector()?.configure()?;
    configuration
        .param_mut()
        .set_time(i64::try_from(now / 1000).map_err(|_| Error)?);
    // Default connector verifies hostname and SNI. No permissive callback.
    let mut tls = configuration.connect(hostname, socket).map_err(|_| Error)?;
    if tls.ssl().version_str() != "TLSv1.3"
        || tls.ssl().session_reused()
        || tls
            .ssl()
            .selected_alpn_protocol()
            .is_some_and(|p| p != b"http/1.1")
        || start.elapsed() > DEADLINE
    {
        return Err(Error);
    }
    tls.write_all(wire)?;
    tls.flush()?;
    let result = response(&mut tls, start)?;
    if start.elapsed() > DEADLINE {
        return Err(Error);
    }
    Ok(result)
}
struct Response {
    status: u16,
    body: PrivateBytes,
    retry_after_ms: Option<u64>,
}
fn response(reader: &mut impl Read, start: Instant) -> Result<Response, Error> {
    let mut header = Zeroizing::new(Vec::new());
    while !header.ends_with(b"\r\n\r\n") {
        if header.len() >= MAX_HEADERS || start.elapsed() > DEADLINE {
            return Err(Error);
        }
        let mut one = [0];
        reader.read_exact(&mut one)?;
        header.push(one[0]);
    }
    let text = std::str::from_utf8(&header).map_err(|_| Error)?;
    if !text.is_ascii() {
        return Err(Error);
    }
    let mut lines = text[..text.len() - 4].split("\r\n");
    let first = lines.next().ok_or(Error)?;
    let rest = first.strip_prefix("HTTP/1.1 ").ok_or(Error)?;
    if rest.len() < 4
        || rest.as_bytes()[3] != b' '
        || !rest.as_bytes()[..3].iter().all(u8::is_ascii_digit)
        || rest.bytes().any(|b| b < 32 || b == 127)
    {
        return Err(Error);
    }
    let status: u16 = rest[..3].parse().map_err(|_| Error)?;
    if !(200..=599).contains(&status) || (300..400).contains(&status) {
        return Err(Error);
    }
    let mut names = BTreeSet::new();
    let mut length = None;
    let mut retry_after_ms = None;
    let mut json = false;
    for line in lines {
        let (name, value) = line.split_once(':').ok_or(Error)?;
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
            || value.bytes().any(|b| (b < 32 && b != b'\t') || b == 127)
        {
            return Err(Error);
        }
        let name = name.to_ascii_lowercase();
        if !names.insert(name.clone()) || names.len() > MAX_FIELDS {
            return Err(Error);
        }
        let value = value.trim_matches([' ', '\t']);
        match name.as_str() {
            "content-length" => {
                if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(Error);
                }
                let n: usize = value.parse().map_err(|_| Error)?;
                if n == 0 || n > MAX_BODY {
                    return Err(Error);
                }
                length = Some(n);
            }
            "content-type" => {
                json = value
                    .split(';')
                    .next()
                    .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"));
            }
            "transfer-encoding" => return Err(Error), // No ambiguous/chunked framing in this bounded profile.
            "content-encoding" if !value.eq_ignore_ascii_case("identity") => return Err(Error),
            "retry-after" if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) => {
                let seconds: u64 = value.parse().map_err(|_| Error)?;
                retry_after_ms = Some(seconds.min(3600) * 1000);
            }
            _ => {}
        }
    }
    if !json {
        return Err(Error);
    }
    let mut body = Zeroizing::new(vec![0; length.ok_or(Error)?]);
    let mut at = 0;
    while at < body.len() {
        if start.elapsed() > DEADLINE {
            return Err(Error);
        }
        let n = reader.read(&mut body[at..])?;
        if n == 0 {
            return Err(Error);
        }
        at += n;
    }
    if start.elapsed() > DEADLINE {
        return Err(Error);
    }
    // Never reuse the connection, interpret a second response or follow redirects.
    Ok(Response {
        status,
        body: PrivateBytes::new(body.to_vec()).map_err(|_| Error)?,
        retry_after_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(mut wire: &[u8]) -> Result<Response, Error> {
        response(&mut wire, Instant::now())
    }
    #[test]
    fn http_framing_is_bounded_unambiguous_and_redirects_never_retry() {
        let wire = b"HTTP/1.1 429 Too Many Requests\r\nContent-Length: 2\r\nContent-Type: application/json; charset=utf-8\r\nRetry-After: 6000\r\n\r\n{}";
        let reply = parse(wire).unwrap();
        assert_eq!(reply.status, 429);
        assert_eq!(reply.body.as_bytes(), b"{}");
        assert_eq!(reply.retry_after_ms, Some(3_600_000));
        for headers in [
            "Content-Length: 2\r\ncontent-length: 2",
            "Content-Length: 2\r\nTransfer-Encoding: chunked",
            "Content-Length: +2",
            "Content-Length: 16385",
            "Content-Length: 2\r\nContent-Encoding: gzip",
            "Content-Length : 2",
            "Content-Length: 2\r\n Folded: x",
            "Content-Length: 2\r\nX: a\nb",
            "X: a",
        ] {
            assert!(
                parse(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n{headers}\r\n\r\n{{}}"
                    )
                    .as_bytes()
                )
                .is_err()
            );
        }
        for wire in [
            b"HTTP/1.1 302 Found\r\nContent-Length: 2\r\nContent-Type: application/json\r\n\r\n{}"
                .as_slice(),
            b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nContent-Type: application/json\r\n\r\n{}",
            b"HTTP/1.0 200 OK\r\nContent-Length: 2\r\nContent-Type: application/json\r\n\r\n{}",
        ] {
            assert!(parse(wire).is_err());
        }
        assert!(parse(&vec![b'x'; MAX_HEADERS + 1]).is_err());
        assert!(
            response(
                &mut wire.as_slice(),
                Instant::now() - DEADLINE - Duration::from_secs(1)
            )
            .is_err()
        );
    }
    #[test]
    fn request_bytes_are_exact_and_paths_cannot_escape_the_allowlist() {
        let encoded = encode(
            "test-api.pacifica.fi",
            "/api/v1/orders/create",
            b"{\"signature\":\"PRIVATE\"}",
        )
        .unwrap();
        assert!(encoded.ends_with(b"\r\n\r\n{\"signature\":\"PRIVATE\"}"));
        assert!(encoded.windows(20).any(|w| w == b"Content-Length: 23\r\n"));
        for (host, path) in [
            ("evil.example", "/api/v1/orders/create"),
            ("api.pacifica.fi\r\nX: evil", "/api/v1/orders/create"),
            ("api.pacifica.fi", "/api/v1/orders/create?x=y"),
            ("api.pacifica.fi", "/api/v1/account/transfer"),
        ] {
            assert!(encode(host, path, b"{}").is_err());
        }
        assert!(encode("api.pacifica.fi", "/api/v1/orders/cancel", &[]).is_err());
        assert!(
            encode(
                "api.pacifica.fi",
                "/api/v1/orders/cancel",
                &vec![0; MAX_BODY + 1]
            )
            .is_err()
        );
    }
    fn certificate(
        key: &openssl::pkey::PKey<openssl::pkey::Private>,
        issuer: Option<(&X509, &openssl::pkey::PKey<openssl::pkey::Private>)>,
        hostname: &str,
    ) -> X509 {
        use openssl::{
            asn1::Asn1Time,
            hash::MessageDigest,
            x509::{
                X509NameBuilder,
                extension::{BasicConstraints, KeyUsage, SubjectAlternativeName},
            },
        };
        let mut name = X509NameBuilder::new().unwrap();
        name.append_entry_by_text("CN", hostname).unwrap();
        let name = name.build();
        let mut builder = X509::builder().unwrap();
        builder.set_version(2).unwrap();
        builder
            .set_serial_number(
                &openssl::bn::BigNum::from_u32(if issuer.is_some() { 2 } else { 1 })
                    .unwrap()
                    .to_asn1_integer()
                    .unwrap(),
            )
            .unwrap();
        builder.set_subject_name(&name).unwrap();
        builder
            .set_issuer_name(issuer.map(|(c, _)| c.subject_name()).unwrap_or(&name))
            .unwrap();
        builder.set_pubkey(key).unwrap();
        builder
            .set_not_before(Asn1Time::from_unix(1_700_000_000).unwrap().as_ref())
            .unwrap();
        builder
            .set_not_after(Asn1Time::from_unix(1_700_001_000).unwrap().as_ref())
            .unwrap();
        let mut constraints = BasicConstraints::new();
        constraints.critical();
        let mut usage = KeyUsage::new();
        usage.critical();
        if issuer.is_none() {
            constraints.ca().pathlen(0);
            usage.key_cert_sign();
        } else {
            usage.digital_signature();
        }
        builder
            .append_extension(constraints.build().unwrap())
            .unwrap();
        builder.append_extension(usage.build().unwrap()).unwrap();
        if issuer.is_some() {
            let san = SubjectAlternativeName::new()
                .dns(hostname)
                .build(&builder.x509v3_context(issuer.map(|(c, _)| c.as_ref()), None))
                .unwrap();
            builder.append_extension(san).unwrap();
        }
        builder
            .sign(
                issuer.map(|(_, key)| key).unwrap_or(key),
                MessageDigest::sha256(),
            )
            .unwrap();
        builder.build()
    }
    #[test]
    fn authenticated_tls_requires_selected_root_hostname_and_qualified_clock() {
        use openssl::{
            ec::{EcGroup, EcKey},
            nid::Nid,
            pkey::PKey,
            ssl::SslAcceptor,
        };
        use std::{
            net::{TcpListener, TcpStream},
            thread,
        };
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
        let ca_key = PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap();
        let root = certificate(&ca_key, None, "fixture-root");
        let leaf_key = PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap();
        let leaf = certificate(&leaf_key, Some((&root, &ca_key)), "test-api.pacifica.fi");
        let other_key = PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap();
        let other_root = certificate(&other_key, None, "other-fixture-root");
        let der = root.to_der().unwrap();
        assert!(Trust::from_der(&der, [99; 32]).is_err());
        let mut trailing = der.clone();
        trailing.push(0);
        assert!(Trust::from_der(&trailing, sha256(&trailing)).is_err());
        for (hostname, now, selected_root, succeeds) in [
            ("test-api.pacifica.fi", 1_700_000_500_000, &root, true),
            ("api.pacifica.fi", 1_700_000_500_000, &root, false),
            (
                "test-api.pacifica.fi",
                1_700_000_500_000,
                &other_root,
                false,
            ),
            ("test-api.pacifica.fi", 1_700_001_500_000, &root, false),
            ("test-api.pacifica.fi", 1_699_999_500_000, &root, false),
        ] {
            let mut builder =
                SslAcceptor::mozilla_intermediate_v5(SslMethod::tls_server()).unwrap();
            builder
                .set_min_proto_version(Some(SslVersion::TLS1_3))
                .unwrap();
            builder
                .set_max_proto_version(Some(SslVersion::TLS1_3))
                .unwrap();
            builder.set_certificate(&leaf).unwrap();
            builder.set_private_key(&leaf_key).unwrap();
            let acceptor = builder.build();
            let wire = encode("test-api.pacifica.fi", "/api/v1/orders/cancel", b"{}").unwrap();
            let expected = wire.to_vec();
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let worker = thread::spawn(move || {
                let (stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                if let Ok(mut tls) = acceptor.accept(stream) {
                    let mut request = Zeroizing::new(vec![0; expected.len()]);
                    tls.read_exact(&mut request).unwrap();
                    assert_eq!(request.as_slice(), expected);
                    tls.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Type: application/json\r\n\r\n{}").unwrap();
                }
            });
            let stream = TcpStream::connect(address).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let der = selected_root.to_der().unwrap();
            let trust = Trust::from_der(&der, sha256(&der)).unwrap();
            assert_eq!(
                exchange(stream, &trust, hostname, now, &wire, Instant::now()).is_ok(),
                succeeds
            );
            worker.join().unwrap();
        }
    }
}
