//! TLS 1.3 terminates inside the confidential service, not in the byte relay.
use crate::{
    Error,
    attestation::{self, Context, Policy},
};
use cinder_api::ConfidentialChannel;
use cinder_journal::model::PrivateBytes;
use cinder_kernel::identity::{DeploymentId, Domain, NetworkId};
use openssl::{
    asn1::Asn1Time,
    ec::{EcGroup, EcKey},
    hash::MessageDigest,
    nid::Nid,
    pkey::{PKey, Private},
    rand::rand_bytes,
    ssl::{SslAcceptor, SslMethod, SslOptions, SslSessionCacheMode, SslVersion},
    x509::{X509, X509NameBuilder},
};
use std::{
    io::{Read, Write},
    net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

/// Fixed profile identifier negotiated as ALPN, never a private customer ID.
pub const ALPN: &[u8] = b"cinder-private/1";
/// Exporter label shared by the verified SDK and confidential service.
pub const EXPORTER: &str = "EXPORTER-Cinder-private-v1";
/// Maximum concurrent sessions; no unbounded connection/task queue.
pub const MAX_CONNECTIONS: usize = 8;
/// Maximum private requests on a connection (views are full snapshots, no stream).
pub const MAX_REQUESTS: usize = 128;
/// Maximum private response; checked before emitting or receiving a frame.
pub const MAX_RESPONSE: usize = 1_048_576;
const IO_TIMEOUT: Duration = Duration::from_secs(5);

// Closed implementation seam: only TCP and this crate's AF_VSOCK adapter. The
// public server never accepts a caller-supplied raw descriptor or fake socket.
pub(crate) trait Socket: Read + Write + Send + Sized + 'static {
    fn prepare(&self) -> std::io::Result<()>;
    fn try_clone(&self) -> std::io::Result<Self>;
    fn shutdown(&self, how: Shutdown) -> std::io::Result<()>;
}
pub(crate) trait Listener {
    type Stream: Socket;
    fn nonblocking(&self) -> std::io::Result<()>;
    fn accept(&self) -> std::io::Result<Self::Stream>;
}
impl Socket for TcpStream {
    fn prepare(&self) -> std::io::Result<()> {
        self.set_nonblocking(false)?;
        self.set_read_timeout(Some(IO_TIMEOUT))?;
        self.set_write_timeout(Some(IO_TIMEOUT))
    }
    fn try_clone(&self) -> std::io::Result<Self> {
        TcpStream::try_clone(self)
    }
    fn shutdown(&self, how: Shutdown) -> std::io::Result<()> {
        TcpStream::shutdown(self, how)
    }
}
impl Listener for TcpListener {
    type Stream = TcpStream;
    fn nonblocking(&self) -> std::io::Result<()> {
        self.set_nonblocking(true)
    }
    fn accept(&self) -> std::io::Result<TcpStream> {
        TcpListener::accept(self).map(|(s, _)| s)
    }
}

/// Qualified clock port. P19 uses a clearly identified local fixture clock only.
pub trait Clock: Send + Sync {
    /// Unix milliseconds from a P20-qualified source in the enclave.
    fn now(&self) -> Result<u64, Error>;
}
/// Attester must obtain a fresh NSM quote over THESE exact inputs in production.
pub trait Attester: Send + Sync {
    /// No host boolean or caller-supplied quote substitutes for real NSM.
    fn quote(&self, policy: &Policy, context: &Context<'_>) -> Result<Vec<u8>, Error>;
}
/// Synchronous bounded application handler. It owns the one protected journal;
/// all successes/errors return over the same TLS connection. Never logs inputs.
pub trait Handler: Send + Sync {
    /// Handle one authenticated private frame at the fresh qualified clock cut.
    fn handle(
        &self,
        channel: &Session,
        request: PrivateBytes,
        now: u64,
    ) -> Result<PrivateBytes, Error>;
}
/// Live established session, constructed only by the TLS server handshake.
pub struct Session {
    domain: Domain,
    binding: [u8; 32],
    expires: u64,
}
impl ConfidentialChannel for Session {
    fn domain(&self) -> Domain {
        self.domain
    }
    fn binding(&self) -> [u8; 32] {
        self.binding
    }
    fn expires_at(&self) -> u64 {
        self.expires
    }
}
/// Ephemeral boot key and certificate, generated inside the private runtime.
/// No key serialization, env loader, static key, file persistence or Debug impl.
pub struct Identity {
    key: PKey<Private>,
    certificate: X509,
    spki: Vec<u8>,
    boot: [u8; 32],
}
impl Identity {
    /// Certificate times come from the runtime's qualified clock, never the host
    /// wall clock. OpenSSL entropy still needs measured-image qualification.
    pub fn generate(clock: &dyn Clock) -> Result<Self, Error> {
        let now = clock.now()?;
        if now == 0 {
            return Err(Error);
        }
        let start = i64::try_from(now / 1000).map_err(|_| Error)?;
        let end = start.checked_add(86_400).ok_or(Error)?;
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)?;
        let key = PKey::from_ec_key(EcKey::generate(&group)?)?;
        let mut name = X509NameBuilder::new()?;
        name.append_entry_by_text("CN", "cinder-private")?;
        let name = name.build();
        let mut cert = X509::builder()?;
        cert.set_version(2)?;
        let mut boot = [0; 32];
        rand_bytes(&mut boot)?;
        if boot == [0; 32] {
            return Err(Error);
        }
        let serial = openssl::bn::BigNum::from_slice(&boot[..16])?.to_asn1_integer()?;
        cert.set_serial_number(&serial)?;
        cert.set_subject_name(&name)?;
        cert.set_issuer_name(&name)?;
        cert.set_pubkey(&key)?;
        cert.set_not_before(Asn1Time::from_unix(start)?.as_ref())?;
        cert.set_not_after(Asn1Time::from_unix(end)?.as_ref())?;
        cert.sign(&key, MessageDigest::sha256())?;
        let spki = key.public_key_to_der()?;
        Ok(Self {
            key,
            certificate: cert.build(),
            spki,
            boot,
        })
    }
    fn acceptor(&self) -> Result<SslAcceptor, Error> {
        let mut b = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls_server())?;
        b.set_min_proto_version(Some(SslVersion::TLS1_3))?;
        b.set_max_proto_version(Some(SslVersion::TLS1_3))?;
        b.set_options(SslOptions::NO_TICKET);
        b.set_session_cache_mode(SslSessionCacheMode::OFF);
        b.set_num_tickets(0)?;
        b.set_max_early_data(0)?;
        b.set_ciphersuites(
            "TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256:TLS_AES_128_GCM_SHA256",
        )?;
        b.set_certificate(&self.certificate)?;
        b.set_private_key(&self.key)?;
        b.check_private_key()?;
        b.set_alpn_select_callback(|_, offered| {
            openssl::ssl::select_next_proto(b"\x10cinder-private/1", offered)
                .ok_or(openssl::ssl::AlpnError::ALERT_FATAL)
        });
        Ok(b.build())
    }
}

/// Exact length-prefix framing, rejecting oversize/trailing/empty frames.
pub fn read_frame(stream: &mut impl Read, maximum: usize) -> Result<Zeroizing<Vec<u8>>, Error> {
    let deadline = Instant::now() + IO_TIMEOUT;
    fn exact(r: &mut impl Read, out: &mut [u8], deadline: Instant) -> Result<(), Error> {
        let mut at = 0;
        while at < out.len() {
            if Instant::now() >= deadline {
                return Err(Error);
            }
            match r.read(&mut out[at..]) {
                Ok(0) => return Err(Error),
                Ok(n) => at += n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => return Err(Error),
            }
        }
        if Instant::now() >= deadline {
            return Err(Error);
        }
        Ok(())
    }
    let mut size = [0; 4];
    exact(stream, &mut size, deadline)?;
    let size = u32::from_be_bytes(size) as usize;
    if size == 0 || size > maximum {
        return Err(Error);
    }
    let mut body = Zeroizing::new(vec![0; size]);
    exact(stream, &mut body, deadline)?;
    Ok(body)
}
/// Writes at most one bounded frame; partial failure closes the whole connection.
pub fn write_frame(stream: &mut impl Write, body: &[u8], maximum: usize) -> Result<(), Error> {
    if body.is_empty() || body.len() > maximum {
        return Err(Error);
    }
    stream.write_all(&(body.len() as u32).to_be_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;
    Ok(())
}

pub(crate) struct Lifetime {
    state: Arc<(Mutex<(bool, bool)>, Condvar)>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Lifetime {
    pub(crate) fn new(socket: impl Socket) -> Self {
        let state = Arc::new((Mutex::new((false, false)), Condvar::new()));
        let c = state.clone();
        let worker = thread::spawn(move || {
            let (lock, cv) = &*c;
            let Ok(s) = lock.lock() else {
                let _ = socket.shutdown(Shutdown::Both);
                return;
            };
            let Ok((s, _)) = cv.wait_timeout_while(s, IO_TIMEOUT, |s| !s.0 && !s.1) else {
                let _ = socket.shutdown(Shutdown::Both);
                return;
            };
            if s.0 {
                return;
            }
            if !s.1 {
                let _ = socket.shutdown(Shutdown::Both);
                return;
            }
            let Ok((s, _)) =
                cv.wait_timeout_while(s, Duration::from_millis(attestation::MAX_SESSION), |s| !s.0)
            else {
                let _ = socket.shutdown(Shutdown::Both);
                return;
            };
            if !s.0 {
                let _ = socket.shutdown(Shutdown::Both);
            }
        });
        Self {
            state,
            worker: Some(worker),
        }
    }
    fn ready(&self) {
        let (l, c) = &*self.state;
        if let Ok(mut s) = l.lock() {
            s.1 = true;
            c.notify_all();
        }
    }
}
impl Drop for Lifetime {
    fn drop(&mut self) {
        let (l, c) = &*self.state;
        if let Ok(mut s) = l.lock() {
            s.0 = true;
            c.notify_all();
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Actual confidential TLS application slice. Ports, not fixture implementations,
/// are accepted here; P20 supplies NSM/time/key-release/witness implementations.
pub struct Server {
    identity: Identity,
    policy: Policy,
    attester: Arc<dyn Attester>,
    clock: Arc<dyn Clock>,
    handler: Arc<dyn Handler>,
}
impl Server {
    /// Validates the independently provisioned release policy before listening.
    pub fn new(
        identity: Identity,
        policy: Policy,
        attester: Arc<dyn Attester>,
        clock: Arc<dyn Clock>,
        handler: Arc<dyn Handler>,
    ) -> Result<Self, Error> {
        Policy::decode(&policy.encode())?;
        Ok(Self {
            identity,
            policy,
            attester,
            clock,
            handler,
        })
    }
    /// Serve an already bound listener; shutdown fences new sessions and closes
    /// all accepted sockets. No plaintext HTTP endpoint or fallback is installed.
    pub fn run(self, listener: TcpListener, stop: Arc<AtomicBool>) -> Result<(), Error> {
        self.run_listener(listener, stop)
    }
    /// Identical attestation/TLS/API contract directly over AF_VSOCK. The parent
    /// relays encrypted bytes; it never terminates this TLS session.
    pub fn run_vsock(
        self,
        listener: crate::vsock::VsockListener,
        stop: Arc<AtomicBool>,
    ) -> Result<(), Error> {
        self.run_listener(listener, stop)
    }
    fn run_listener<L: Listener>(self, listener: L, stop: Arc<AtomicBool>) -> Result<(), Error> {
        let acceptor = self.identity.acceptor()?;
        let server = Arc::new(self);
        listener.nonblocking()?;
        let active = Arc::new(AtomicUsize::new(0));
        let mut workers: Vec<(L::Stream, thread::JoinHandle<()>)> = Vec::new();
        let mut failed = false;
        while !stop.load(Ordering::SeqCst) {
            workers.retain(|(_, h)| !h.is_finished());
            match listener.accept() {
                Ok(socket) => {
                    if active.load(Ordering::SeqCst) >= MAX_CONNECTIONS {
                        let _ = socket.shutdown(Shutdown::Both);
                        continue;
                    }
                    // Darwin can inherit listener O_NONBLOCK; each worker uses
                    // bounded blocking I/O, never treat a timing race as EOF.
                    if socket.prepare().is_err() {
                        continue;
                    }
                    let Ok(shutdown) = socket.try_clone() else {
                        continue;
                    };
                    let a = active.clone();
                    let server = server.clone();
                    let acceptor = acceptor.clone();
                    active.fetch_add(1, Ordering::SeqCst);
                    let worker = thread::spawn(move || {
                        let _ = server.connection(socket, &acceptor);
                        a.fetch_sub(1, Ordering::SeqCst);
                    });
                    workers.push((shutdown, worker));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10))
                }
                Err(_) => {
                    failed = true;
                    stop.store(true, Ordering::SeqCst);
                    break;
                }
            }
        }
        for (socket, worker) in workers {
            let _ = socket.shutdown(Shutdown::Both);
            let _ = worker.join();
        }
        if failed { Err(Error) } else { Ok(()) }
    }
    fn connection(&self, socket: impl Socket, acceptor: &SslAcceptor) -> Result<(), Error> {
        let lifetime = Lifetime::new(socket.try_clone()?);
        let mut tls = acceptor.accept(socket).map_err(|_| Error)?;
        if tls.ssl().version_str() != "TLSv1.3"
            || tls.ssl().selected_alpn_protocol() != Some(ALPN)
            || tls.ssl().session_reused()
        {
            return Err(Error);
        }
        let challenge = read_frame(&mut tls, 32)?;
        let nonce: [u8; 32] = challenge.as_slice().try_into().map_err(|_| Error)?;
        if nonce == [0; 32] {
            return Err(Error);
        }
        let mut exporter = [0; 32];
        tls.ssl()
            .export_keying_material(&mut exporter, EXPORTER, None)?;
        let now = self.clock.now()?;
        let expires = now.checked_add(attestation::MAX_SESSION).ok_or(Error)?;
        let context = Context {
            nonce,
            exporter,
            boot: self.identity.boot,
            expires,
            now,
            spki: &self.identity.spki,
        };
        let quote = self.attester.quote(&self.policy, &context)?;
        if quote.len() > attestation::MAX_QUOTE {
            return Err(Error);
        }
        let public = [
            self.identity.boot.as_slice(),
            &expires.to_be_bytes(),
            &quote,
        ]
        .concat();
        write_frame(&mut tls, &public, attestation::MAX_QUOTE + 40)?;
        let session = Session {
            domain: Domain {
                network: NetworkId::new(self.policy.domain[..32].try_into().map_err(|_| Error)?)
                    .map_err(|_| Error)?,
                deployment: DeploymentId::new(
                    self.policy.domain[32..].try_into().map_err(|_| Error)?,
                )
                .map_err(|_| Error)?,
            },
            binding: attestation::binding(&self.policy, &context),
            expires,
        };
        // No private request is processed until the SDK confirms successful quote
        // verification. The token is derived from THIS TLS exporter, not a header.
        let ready = read_frame(&mut tls, 32)?;
        if ready.as_slice() != session.binding {
            return Err(Error);
        }
        write_frame(&mut tls, &session.binding, 32)?;
        lifetime.ready();
        for _ in 0..MAX_REQUESTS {
            let request = read_frame(&mut tls, cinder_api::wire::MAX_REQUEST)?;
            let now = self.clock.now()?;
            if now >= expires {
                return Err(Error);
            }
            let request = PrivateBytes::new(request.to_vec()).map_err(|_| Error)?;
            let reply = self.handler.handle(&session, request, now)?;
            if self.clock.now()? >= expires {
                return Err(Error);
            }
            write_frame(&mut tls, reply.as_bytes(), MAX_RESPONSE)?;
        }
        Ok(())
    }
}

/// Local-loopback relay for P19's process slice. Only a configured exact socket
/// target is reachable; P20 adds the qualified vsock port, not an open proxy.
pub fn relay(
    listener: TcpListener,
    target: SocketAddr,
    stop: Arc<AtomicBool>,
) -> Result<(), Error> {
    if !target.ip().is_loopback() || !listener.local_addr()?.ip().is_loopback() {
        return Err(Error);
    }
    relay_socket(
        listener,
        || TcpStream::connect_timeout(&target, IO_TIMEOUT).map_err(|_| Error),
        stop,
    )
}
/// Parent ingress to one fixed enclave CID/port. The TCP listener is loopback
/// only in this qualification profile; deployment ingress exposure is separate.
/// Does not parse private frames, terminate TLS, retry connects or change targets.
pub fn relay_vsock(
    listener: TcpListener,
    target: crate::vsock::Target,
    stop: Arc<AtomicBool>,
) -> Result<(), Error> {
    if !target.is_enclave() || !listener.local_addr()?.ip().is_loopback() {
        return Err(Error);
    }
    relay_socket(
        listener,
        || crate::vsock::VsockStream::connect(target),
        stop,
    )
}
/// Parent egress to one native origin's HTTPS port, not a general-purpose proxy.
/// DNS chooses routing only; the enclave verifies origin/certificates. There is
/// one TCP attempt per accepted socket, no address fallback or TLS termination.
pub fn relay_egress(
    listener: crate::vsock::VsockListener,
    origin: cinder_pacifica::execution::Origin,
    stop: Arc<AtomicBool>,
) -> Result<(), Error> {
    let host = crate::egress::host(origin);
    // Fail fast at startup, but do not retain an address beyond that check.
    first_ipv4((host, 443).to_socket_addrs()?)?;
    relay_socket(listener, || connect_https(host), stop)
}
/// Measured cloud endpoint only. TLS/SigV4 terminate in the enclave, never here.
pub fn relay_cloud(
    listener: crate::vsock::VsockListener,
    endpoint: &crate::cloud::Endpoint,
    stop: Arc<AtomicBool>,
) -> Result<(), Error> {
    let host = endpoint.host()?;
    first_ipv4((host.as_str(), 443).to_socket_addrs()?)?;
    relay_socket(listener, || connect_https(&host), stop)
}
fn first_ipv4(mut addresses: impl Iterator<Item = SocketAddr>) -> Result<SocketAddr, Error> {
    addresses.find(SocketAddr::is_ipv4).ok_or(Error)
}
fn connect_https(host: &str) -> Result<TcpStream, Error> {
    connect_https_with(
        || (host, 443).to_socket_addrs().map_err(|_| Error),
        |address| TcpStream::connect_timeout(&address, IO_TIMEOUT).map_err(|_| Error),
    )
}
fn connect_https_with<I: Iterator<Item = SocketAddr>, S>(
    resolve: impl FnOnce() -> Result<I, Error>,
    connect: impl FnOnce(SocketAddr) -> Result<S, Error>,
) -> Result<S, Error> {
    connect(first_ipv4(resolve()?)?)
}
fn relay_socket<L: Listener, S: Socket>(
    listener: L,
    connect: impl Fn() -> Result<S, Error>,
    stop: Arc<AtomicBool>,
) -> Result<(), Error> {
    listener.nonblocking()?;
    let active = Arc::new(AtomicUsize::new(0));
    let mut workers: Vec<(L::Stream, S, thread::JoinHandle<()>)> = Vec::new();
    let mut failed = false;
    while !stop.load(Ordering::SeqCst) {
        workers.retain(|(_, _, h)| !h.is_finished());
        match listener.accept() {
            Ok(client) => {
                if active.load(Ordering::SeqCst) >= MAX_CONNECTIONS {
                    let _ = client.shutdown(Shutdown::Both);
                    continue;
                }
                if client.prepare().is_err() {
                    continue;
                }
                let Ok(upstream) = connect() else {
                    let _ = client.shutdown(Shutdown::Both);
                    continue;
                };
                if upstream.prepare().is_err() {
                    let _ = client.shutdown(Shutdown::Both);
                    let _ = upstream.shutdown(Shutdown::Both);
                    continue;
                }
                let (Ok(c), Ok(u), Ok(deadline_socket), Ok(mut c2), Ok(mut u2)) = (
                    client.try_clone(),
                    upstream.try_clone(),
                    client.try_clone(),
                    client.try_clone(),
                    upstream.try_clone(),
                ) else {
                    let _ = client.shutdown(Shutdown::Both);
                    let _ = upstream.shutdown(Shutdown::Both);
                    continue;
                };
                let a = active.clone();
                active.fetch_add(1, Ordering::SeqCst);
                let h = thread::spawn(move || {
                    let lifetime = Lifetime::new(deadline_socket);
                    lifetime.ready();
                    let second = thread::spawn(move || {
                        let _ = copy_bounded(&mut u2, &mut c2);
                        let _ = c2.shutdown(Shutdown::Both);
                        let _ = u2.shutdown(Shutdown::Both);
                    });
                    let mut client = client;
                    let mut upstream = upstream;
                    let _ = copy_bounded(&mut client, &mut upstream);
                    let _ = client.shutdown(Shutdown::Both);
                    let _ = upstream.shutdown(Shutdown::Both);
                    let _ = second.join();
                    a.fetch_sub(1, Ordering::SeqCst);
                });
                workers.push((c, u, h));
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10))
            }
            Err(_) => {
                failed = true;
                break;
            }
        }
    }
    for (c, u, h) in workers {
        let _ = c.shutdown(Shutdown::Both);
        let _ = u.shutdown(Shutdown::Both);
        let _ = h.join();
    }
    if failed { Err(Error) } else { Ok(()) }
}
fn copy_bounded(from: &mut impl Read, to: &mut impl Write) -> Result<(), Error> {
    let mut b = [0; 16_384];
    let mut total = 0;
    loop {
        let n = from.read(&mut b)?;
        if n == 0 {
            return Ok(());
        }
        total += n;
        if total > 4 * MAX_RESPONSE {
            return Err(Error);
        }
        to.write_all(&b[..n])?;
    }
}

/// CLI harness shutdown: public startup port only; stdin EOF stops the slice.
/// Real supervision/health/vsock belongs to the P20 deployment entrypoint.
pub fn stop_on_stdin() -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));
    let s = stop.clone();
    thread::spawn(move || {
        let mut b = [0; 1];
        let _ = std::io::stdin().read(&mut b);
        s.store(true, Ordering::SeqCst);
    });
    stop
}

#[cfg(test)]
mod identity_tests {
    use super::*;
    #[test]
    fn https_connections_resolve_fresh_ipv4_once_without_address_retry() {
        let ipv6: SocketAddr = "[::1]:443".parse().unwrap();
        let first: SocketAddr = "127.0.0.1:443".parse().unwrap();
        let second: SocketAddr = "127.0.0.2:443".parse().unwrap();
        let resolutions = std::cell::Cell::new(0);
        let attempts = std::cell::Cell::new(0);
        let dial = || {
            connect_https_with(
                || {
                    let n = resolutions.get();
                    resolutions.set(n + 1);
                    Ok([ipv6, if n == 0 { first } else { second }].into_iter())
                },
                |address| {
                    attempts.set(attempts.get() + 1);
                    Ok(address)
                },
            )
        };
        assert_eq!(dial().unwrap(), first);
        assert_eq!(dial().unwrap(), second);
        assert_eq!((resolutions.get(), attempts.get()), (2, 2));
        let calls = std::cell::Cell::new(0);
        assert!(
            connect_https_with(
                || Ok([first, second].into_iter()),
                |_| {
                    calls.set(calls.get() + 1);
                    Err::<(), _>(Error)
                }
            )
            .is_err()
        );
        assert_eq!(calls.get(), 1);
        assert!(first_ipv4([ipv6].into_iter()).is_err());
        assert!(
            connect_https_with(
                || Ok([ipv6].into_iter()),
                |_| {
                    calls.set(calls.get() + 1);
                    Ok(())
                }
            )
            .is_err()
        );
        assert!(
            connect_https_with(
                || Err::<std::vec::IntoIter<SocketAddr>, _>(Error),
                |_| {
                    calls.set(calls.get() + 1);
                    Ok(())
                }
            )
            .is_err()
        );
        assert_eq!(calls.get(), 1);
    }
    struct Fixed(Result<u64, Error>);
    impl Clock for Fixed {
        fn now(&self) -> Result<u64, Error> {
            self.0
        }
    }
    #[test]
    fn certificate_uses_only_selected_clock_and_failed_clock_cannot_boot() {
        let now = 1_600_000_000_123;
        let identity = Identity::generate(&Fixed(Ok(now))).unwrap();
        assert!(
            identity.certificate.not_before() == Asn1Time::from_unix((now / 1000) as i64).unwrap()
        );
        assert!(
            identity.certificate.not_after()
                == Asn1Time::from_unix((now / 1000) as i64 + 86_400).unwrap()
        );
        assert_ne!(identity.boot, [0; 32]);
        assert!(Identity::generate(&Fixed(Err(Error))).is_err());
        assert!(Identity::generate(&Fixed(Ok(0))).is_err());
    }
    #[test]
    fn parent_ingress_rejects_parent_loop_and_bounds_opaque_copy() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        assert!(
            relay_vsock(
                listener,
                crate::vsock::Target::new(3, 5000).unwrap(),
                Arc::new(AtomicBool::new(false))
            )
            .is_err()
        );
        let input = vec![7; 4 * MAX_RESPONSE];
        let mut output = Vec::new();
        copy_bounded(&mut input.as_slice(), &mut output).unwrap();
        assert_eq!(output, input);
        output.clear();
        let oversized = vec![7; 4 * MAX_RESPONSE + 1];
        assert!(copy_bounded(&mut oversized.as_slice(), &mut output).is_err());
        assert_eq!(output.len(), 4 * MAX_RESPONSE);
    }
}
