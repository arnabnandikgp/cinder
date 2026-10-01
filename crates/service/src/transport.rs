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
    net::{Shutdown, SocketAddr, TcpListener, TcpStream},
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
    /// Uses OpenSSL's entropy provider; P20 must qualify the actual enclave RNG.
    pub fn generate() -> Result<Self, Error> {
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)?;
        let key = PKey::from_ec_key(EcKey::generate(&group)?)?;
        let mut name = X509NameBuilder::new()?;
        name.append_entry_by_text("CN", "cinder-private")?;
        let name = name.build();
        let mut cert = X509::builder()?;
        cert.set_version(2)?;
        let mut boot = [0; 32];
        rand_bytes(&mut boot)?;
        let serial = openssl::bn::BigNum::from_slice(&boot[..16])?.to_asn1_integer()?;
        cert.set_serial_number(&serial)?;
        cert.set_subject_name(&name)?;
        cert.set_issuer_name(&name)?;
        cert.set_pubkey(&key)?;
        cert.set_not_before(Asn1Time::days_from_now(0)?.as_ref())?;
        cert.set_not_after(Asn1Time::days_from_now(1)?.as_ref())?;
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

struct Lifetime {
    state: Arc<(Mutex<(bool, bool)>, Condvar)>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Lifetime {
    fn new(socket: TcpStream) -> Self {
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
        let acceptor = self.identity.acceptor()?;
        let server = Arc::new(self);
        listener.set_nonblocking(true)?;
        let active = Arc::new(AtomicUsize::new(0));
        let mut workers = Vec::new();
        while !stop.load(Ordering::SeqCst) {
            workers.retain(|(_, h): &(TcpStream, thread::JoinHandle<()>)| !h.is_finished());
            match listener.accept() {
                Ok((socket, _)) => {
                    if active.load(Ordering::SeqCst) >= MAX_CONNECTIONS {
                        let _ = socket.shutdown(Shutdown::Both);
                        continue;
                    }
                    // Darwin can inherit listener O_NONBLOCK; each worker uses
                    // bounded blocking I/O, never treat a timing race as EOF.
                    socket.set_nonblocking(false)?;
                    socket.set_read_timeout(Some(IO_TIMEOUT))?;
                    socket.set_write_timeout(Some(IO_TIMEOUT))?;
                    let shutdown = socket.try_clone()?;
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
                    stop.store(true, Ordering::SeqCst);
                    break;
                }
            }
        }
        for (socket, worker) in workers {
            let _ = socket.shutdown(Shutdown::Both);
            let _ = worker.join();
        }
        Ok(())
    }
    fn connection(&self, socket: TcpStream, acceptor: &SslAcceptor) -> Result<(), Error> {
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
    listener.set_nonblocking(true)?;
    let active = Arc::new(AtomicUsize::new(0));
    let mut workers = Vec::new();
    while !stop.load(Ordering::SeqCst) {
        workers
            .retain(|(_, _, h): &(TcpStream, TcpStream, thread::JoinHandle<()>)| !h.is_finished());
        match listener.accept() {
            Ok((client, _)) => {
                if active.load(Ordering::SeqCst) >= MAX_CONNECTIONS {
                    let _ = client.shutdown(Shutdown::Both);
                    continue;
                }
                client.set_nonblocking(false)?;
                let Ok(upstream) = TcpStream::connect_timeout(&target, IO_TIMEOUT) else {
                    let _ = client.shutdown(Shutdown::Both);
                    continue;
                };
                for s in [&client, &upstream] {
                    s.set_read_timeout(Some(IO_TIMEOUT))?;
                    s.set_write_timeout(Some(IO_TIMEOUT))?;
                }
                let c = client.try_clone()?;
                let u = upstream.try_clone()?;
                let deadline_socket = client.try_clone()?;
                let mut c2 = client.try_clone()?;
                let mut u2 = upstream.try_clone()?;
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
            Err(_) => break,
        }
    }
    for (c, u, h) in workers {
        let _ = c.shutdown(Shutdown::Both);
        let _ = u.shutdown(Shutdown::Both);
        let _ = h.join();
    }
    Ok(())
}
fn copy_bounded(from: &mut TcpStream, to: &mut TcpStream) -> Result<(), Error> {
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
