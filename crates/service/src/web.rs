//! Attested Noise ingress. It accepts no financial identifiers before key
//! confirmation and invokes the SAME Handler/Session/PrivateBytes API as TLS.
use crate::{
    Error,
    attestation::{MAX_QUOTE, MAX_SESSION, Policy},
    transport::{
        Clock, Handler, Lifetime, Listener, MAX_CONNECTIONS, Session, Socket, read_frame,
        write_frame,
    },
};
use cinder_journal::model::PrivateBytes;
use cinder_kernel::identity::{DeploymentId, Domain, NetworkId};
use cinder_web_channel::{Endpoint, Entropy, profile, records};
use std::{
    io::{Cursor, Read},
    net::{Shutdown, TcpListener},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

/// Authenticated application prefix; public outer sequence is only routing.
pub const UPDATE: &[u8] = b"CINDER-PRIVATE-UPDATE-1\0";

/// Public connection fields: nonce, boot, opaque handle, responder X25519 key.
pub const FIELDS: usize = 128;
/// Public quote envelope bound: fields, expiry, bounded signed quote.
pub const MAX_ENVELOPE: usize = FIELDS + 8 + MAX_QUOTE;
/// Purpose-separated web quote provider; no parent approval/trust callback.
pub trait Attester: Send + Sync {
    /// Quote exact canonical context. Qualified implementations locally validate
    /// NSM signature/measurements/time; clients independently verify it too.
    fn web_quote(
        &self,
        policy: &Policy,
        fields: &[u8; FIELDS],
        expires: u64,
        now: u64,
    ) -> Result<Vec<u8>, Error>;
}
/// Enclave-owned bounded web ingress. Policy and boot identity never come from
/// relay requests. There is no second authorization, replay or financial store.
pub struct Server {
    policy: Policy,
    boot: [u8; 32],
    entropy: Arc<dyn Entropy>,
    attester: Arc<dyn Attester>,
    clock: Arc<dyn Clock>,
    handler: Arc<dyn Handler>,
}
impl Server {
    /// Construct with qualified in-enclave entropy/attestation/time providers.
    pub fn new(
        policy: Policy,
        entropy: Arc<dyn Entropy>,
        attester: Arc<dyn Attester>,
        clock: Arc<dyn Clock>,
        handler: Arc<dyn Handler>,
    ) -> Result<Self, Error> {
        Policy::decode(&policy.encode())?;
        let mut boot = [0; 32];
        entropy.fill(&mut boot).map_err(|_| Error)?;
        if boot == [0; 32] {
            return Err(Error);
        }
        Ok(Self {
            policy,
            boot,
            entropy,
            attester,
            clock,
            handler,
        })
    }
    /// Offline TCP ingress; exposed public bodies remain encrypted end to end.
    pub fn run(self, listener: TcpListener, stop: Arc<AtomicBool>) -> Result<(), Error> {
        self.run_listener(listener, stop)
    }
    /// Identical framing over AF_VSOCK; fixed-target parent relay stays opaque.
    pub fn run_vsock(
        self,
        listener: crate::vsock::VsockListener,
        stop: Arc<AtomicBool>,
    ) -> Result<(), Error> {
        self.run_listener(listener, stop)
    }
    fn run_listener<L: Listener>(self, listener: L, stop: Arc<AtomicBool>) -> Result<(), Error> {
        listener.nonblocking()?;
        let server = Arc::new(self);
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
                    if socket.prepare().is_err() {
                        continue;
                    }
                    let Ok(shutdown) = socket.try_clone() else {
                        continue;
                    };
                    let server = server.clone();
                    let active = active.clone();
                    active.fetch_add(1, Ordering::SeqCst);
                    let worker = thread::spawn(move || {
                        let _ = server.connection(socket);
                        active.fetch_sub(1, Ordering::SeqCst);
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
    fn connection(&self, mut socket: impl Socket) -> Result<(), Error> {
        let lifetime = Lifetime::new(socket.try_clone()?);
        let nonce = read_frame(&mut socket, 32)?;
        if nonce.len() != 32 || nonce.iter().all(|b| *b == 0) {
            return Err(Error);
        }
        let now = self.clock.now()?;
        if now == 0 {
            return Err(Error);
        }
        let expires = now.checked_add(MAX_SESSION).ok_or(Error)?;
        let mut fields = [0; FIELDS];
        fields[..32].copy_from_slice(&nonce);
        fields[32..64].copy_from_slice(&self.boot);
        self.entropy.fill(&mut fields[64..96]).map_err(|_| Error)?;
        let mut envelope = Vec::new();
        let mut prologue = [0; 32];
        let mut endpoint = Endpoint::server_with_entropy(self.entropy.clone(), |key| {
            fields[96..].copy_from_slice(key);
            let context = profile::context(&self.policy.encode(), &fields, expires)?;
            let quote = self
                .attester
                .web_quote(&self.policy, &fields, expires, now)
                .map_err(|_| cinder_web_channel::Error)?;
            prologue = profile::prologue(&context, &quote)?;
            envelope.extend_from_slice(&fields);
            envelope.extend_from_slice(&expires.to_be_bytes());
            envelope.extend_from_slice(&quote);
            Ok(prologue)
        })
        .map_err(|_| Error)?;
        write_frame(&mut socket, &envelope, MAX_ENVELOPE)?;
        for _ in 0..2 {
            let request = read_frame(&mut socket, cinder_web_channel::MAX_PAYLOAD + 16)?;
            let reply = endpoint.advance(&request).map_err(|_| Error)?;
            write_frame(&mut socket, &reply, cinder_web_channel::MAX_PAYLOAD + 16)?;
        }
        if !endpoint.ready() || self.clock.now()? >= expires {
            return Err(Error);
        }
        let session = Session::established(
            Domain {
                network: NetworkId::new(self.policy.domain[..32].try_into().map_err(|_| Error)?)
                    .map_err(|_| Error)?,
                deployment: DeploymentId::new(
                    self.policy.domain[32..].try_into().map_err(|_| Error)?,
                )
                .map_err(|_| Error)?,
            },
            profile::binding(&prologue, &endpoint.binding().map_err(|_| Error)?),
            expires,
        );
        lifetime.ready();
        // One bounded reader, one cipher/handler writer. The idle first byte
        // does not turn a partial frame into a 120-second delivery budget.
        let mut reader = socket.try_clone()?;
        let shutdown = socket.try_clone()?;
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        thread::scope(|scope| {
            let worker = scope.spawn(move || {
                let result = (|| -> Result<(), Error> {
                    loop {
                        reader.idle()?;
                        let mut first = [0];
                        reader.read_exact(&mut first)?;
                        reader.prepare()?;
                        let delivery = Lifetime::new(reader.try_clone()?);
                        let wire = read_frame(
                            &mut Cursor::new(first).chain(&mut reader),
                            records::MAX_REQUEST + 22,
                        )?;
                        drop(delivery);
                        send.try_send(wire).map_err(|_| Error)?;
                    }
                })();
                if result.is_err() {
                    let _ = shutdown.shutdown(Shutdown::Both);
                }
            });
            let result = self.application(&mut socket, &mut endpoint, &session, expires, receive);
            let _ = socket.shutdown(Shutdown::Both);
            let _ = worker.join();
            result
        })
    }
    fn application(
        &self,
        socket: &mut impl Socket,
        endpoint: &mut Endpoint,
        session: &Session,
        expires: u64,
        receive: std::sync::mpsc::Receiver<Zeroizing<Vec<u8>>>,
    ) -> Result<(), Error> {
        let mut sequence = 1;
        let mut socket_mode = false;
        let mut watch: Option<(u32, PrivateBytes, Option<[u8; 32]>, u64)> = None;
        let mut next_poll = Instant::now();
        loop {
            if self.clock.now()? >= expires {
                return Err(Error);
            }
            // Poll independently of ingress activity: a stream of valid commands
            // must not starve updates, expiry or current READ authorization.
            if Instant::now() >= next_poll {
                next_poll = Instant::now() + Duration::from_millis(500);
                if let Some((correlation, request, revision, ordinal)) = &mut watch {
                    let reply = self
                        .handler
                        .handle(session, request.clone(), self.clock.now()?)?;
                    if self.clock.now()? >= expires {
                        return Err(Error);
                    }
                    let fresh = revision_of(reply.as_bytes());
                    if fresh.is_none() || fresh != *revision {
                        *ordinal = ordinal.checked_add(1).ok_or(Error)?;
                        self.deliver(socket, endpoint, *correlation, &reply, true, Some(*ordinal))?;
                        *revision = fresh;
                        if fresh.is_none() {
                            return Err(Error);
                        }
                    }
                }
            }
            let wire = match receive.recv_timeout(Duration::from_millis(250)) {
                Ok(wire) => wire,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return Err(Error),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            };
            if sequence >= cinder_web_channel::MAX_RECORDS {
                return Err(Error);
            }
            let clear = endpoint.open(&wire).map_err(|_| Error)?;
            let (delivery, request) =
                records::read_delivery(sequence, &clear).map_err(|_| Error)?;
            if socket_mode && delivery == 1 {
                return Err(Error);
            }
            socket_mode |= delivery != 1;
            let now = self.clock.now()?;
            if now >= expires {
                return Err(Error);
            }
            let request = PrivateBytes::new(request.to_vec()).map_err(|_| Error)?;
            if delivery == 3 {
                // Never dispatch a mutation disguised as a subscription.
                let valid = cinder_api::wire::Request::decode(request.as_bytes()).is_ok_and(|r|
                    matches!(r.command,cinder_api::wire::Command::Read(q) if q.cursor == [0;40]));
                if !valid || watch.is_some() {
                    return Err(Error);
                }
            }
            let reply = self.handler.handle(session, request.clone(), now)?;
            if self.clock.now()? >= expires {
                return Err(Error);
            }
            self.deliver(
                socket,
                endpoint,
                sequence,
                &reply,
                socket_mode,
                (delivery == 3).then_some(1),
            )?;
            if delivery == 3 {
                let revision = revision_of(reply.as_bytes());
                if revision.is_none() {
                    return Err(Error);
                }
                watch = Some((sequence, request, revision, 1));
                next_poll = Instant::now() + Duration::from_millis(500);
            }
            sequence += 1;
            if sequence == cinder_web_channel::MAX_RECORDS && watch.is_none() {
                return Ok(());
            }
        }
    }
    fn deliver(
        &self,
        socket: &mut impl Socket,
        endpoint: &mut Endpoint,
        sequence: u32,
        reply: &PrivateBytes,
        websocket: bool,
        update: Option<u64>,
    ) -> Result<(), Error> {
        let clear = if let Some(ordinal) = update {
            Zeroizing::new([UPDATE, &ordinal.to_be_bytes(), reply.as_bytes()].concat())
        } else {
            Zeroizing::new(reply.as_bytes().to_vec())
        };
        let batch = records::response(endpoint, sequence, &clear).map_err(|_| Error)?;
        let _delivery = Lifetime::new(socket.try_clone()?);
        if websocket {
            write_frame(
                socket,
                &[&sequence.to_be_bytes()[..], &batch].concat(),
                records::MAX_BATCH + 4,
            )
        } else {
            write_frame(socket, &batch, records::MAX_BATCH)
        }
    }
}
fn revision_of(bytes: &[u8]) -> Option<[u8; 32]> {
    let prefix = b"CINDER-API-REPLY\0\x00\x01";
    if !bytes.starts_with(prefix) || bytes.get(prefix.len()..prefix.len() + 2)? != [3, 1] {
        return None;
    }
    bytes
        .get(prefix.len() + 3..prefix.len() + 35)?
        .try_into()
        .ok()
}
#[cfg(all(test, feature = "local-fixture"))]
mod tests {
    use super::*;
    use crate::fixture::{FixtureAttester, FixtureClock, policy};
    use cinder_api::ConfidentialChannel;
    use std::{
        net::TcpStream,
        sync::{Mutex, atomic::AtomicU64},
    };
    struct Time(AtomicU64);
    impl Clock for Time {
        fn now(&self) -> Result<u64, Error> {
            let n = self.0.load(Ordering::SeqCst);
            if n == 0 { Err(Error) } else { Ok(n) }
        }
    }
    #[derive(Default)]
    struct Application(Mutex<Vec<[u8; 32]>>);
    impl Handler for Application {
        fn handle(
            &self,
            channel: &Session,
            request: PrivateBytes,
            _now: u64,
        ) -> Result<PrivateBytes, Error> {
            self.0.lock().unwrap().push(channel.binding());
            Ok(request)
        }
    }
    fn client(port: u16) -> (TcpStream, Endpoint, u64, [u8; 32]) {
        let mut socket = TcpStream::connect(("127.0.0.1", port)).unwrap();
        socket.prepare().unwrap();
        write_frame(&mut socket, &[9; 32], 32).unwrap();
        let envelope = read_frame(&mut socket, MAX_ENVELOPE).unwrap();
        let expires = u64::from_be_bytes(envelope[128..136].try_into().unwrap());
        let context = profile::context(&policy().encode(), &envelope[..128], expires).unwrap();
        let prologue = profile::prologue(&context, &envelope[136..]).unwrap();
        let mut endpoint = Endpoint::client(&envelope[96..128], &prologue).unwrap();
        write_frame(&mut socket, &endpoint.start().unwrap(), 16400).unwrap();
        let second = read_frame(&mut socket, 16400).unwrap();
        write_frame(&mut socket, &endpoint.advance(&second).unwrap(), 16400).unwrap();
        let ack = read_frame(&mut socket, 16400).unwrap();
        assert!(endpoint.advance(&ack).unwrap().is_empty());
        let binding = profile::binding(&prologue, &endpoint.binding().unwrap());
        (socket, endpoint, expires, binding)
    }
    type Fixture = (
        u16,
        Arc<Time>,
        Arc<Application>,
        Arc<AtomicBool>,
        thread::JoinHandle<Result<(), Error>>,
    );
    fn start() -> Fixture {
        let clock = Arc::new(Time(AtomicU64::new(FixtureClock.now().unwrap())));
        let application = Arc::new(Application::default());
        let server = Server::new(
            policy(),
            Arc::new(FixtureClock),
            Arc::new(FixtureAttester::new().unwrap()),
            clock.clone(),
            application.clone(),
        )
        .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        (
            port,
            clock,
            application,
            stop,
            thread::spawn(move || server.run(listener, flag)),
        )
    }
    fn send(socket: &mut TcpStream, endpoint: &mut Endpoint, sequence: u32) {
        let clear = records::request(sequence, b"PRIVATE-APPLICATION").unwrap();
        write_frame(socket, &endpoint.seal(&clear).unwrap(), 1046).unwrap();
    }
    #[test]
    fn confirmed_binding_reaches_same_session_and_expiry_blocks_handler() {
        let (port, clock, application, stop, worker) = start();
        let (mut socket, mut endpoint, expires, binding) = client(port);
        send(&mut socket, &mut endpoint, 1);
        let batch = read_frame(&mut socket, records::MAX_BATCH).unwrap();
        assert_eq!(
            records::read_response(&mut endpoint, 1, &batch)
                .unwrap()
                .as_slice(),
            b"PRIVATE-APPLICATION"
        );
        assert_eq!(*application.0.lock().unwrap(), vec![binding]);
        clock.0.store(expires, Ordering::SeqCst);
        send(&mut socket, &mut endpoint, 2);
        assert!(read_frame(&mut socket, records::MAX_BATCH).is_err());
        assert_eq!(application.0.lock().unwrap().len(), 1);
        stop.store(true, Ordering::SeqCst);
        assert!(worker.join().unwrap().is_ok());
    }
    #[test]
    fn fence_closes_live_sessions_without_an_application_call() {
        let (port, _clock, application, stop, worker) = start();
        let (mut socket, _endpoint, _expires, _binding) = client(port);
        stop.store(true, Ordering::SeqCst);
        assert!(worker.join().unwrap().is_ok());
        assert!(read_frame(&mut socket, records::MAX_BATCH).is_err());
        assert!(application.0.lock().unwrap().is_empty());
    }
    #[test]
    fn signed_clock_failure_closes_session_instead_of_cached_time() {
        let (port, clock, application, stop, worker) = start();
        let (mut socket, mut endpoint, _expires, _binding) = client(port);
        clock.0.store(0, Ordering::SeqCst);
        send(&mut socket, &mut endpoint, 1);
        assert!(read_frame(&mut socket, records::MAX_BATCH).is_err());
        assert!(application.0.lock().unwrap().is_empty());
        stop.store(true, Ordering::SeqCst);
        assert!(worker.join().unwrap().is_ok());
    }
    #[test]
    fn subscription_delivery_cannot_disguise_a_command_or_invoke_handler() {
        let (port, _clock, application, stop, worker) = start();
        let (mut socket, mut endpoint, _expires, _binding) = client(port);
        let clear = records::socket_request(1, b"PRIVATE-APPLICATION", true).unwrap();
        write_frame(&mut socket, &endpoint.seal(&clear).unwrap(), 1046).unwrap();
        assert!(read_frame(&mut socket, records::MAX_BATCH + 4).is_err());
        assert!(application.0.lock().unwrap().is_empty());
        stop.store(true, Ordering::SeqCst);
        assert!(worker.join().unwrap().is_ok());
    }
    #[test]
    fn socket_correlation_is_authenticated_and_http_cannot_mix_after_upgrade() {
        let (port, _clock, application, stop, worker) = start();
        let (mut socket, mut endpoint, _expires, binding) = client(port);
        let clear = records::socket_request(1, b"PRIVATE-APPLICATION", false).unwrap();
        write_frame(&mut socket, &endpoint.seal(&clear).unwrap(), 1046).unwrap();
        let batch = read_frame(&mut socket, records::MAX_BATCH + 4).unwrap();
        assert_eq!(&batch[..4], &1u32.to_be_bytes());
        assert_eq!(
            records::read_response(&mut endpoint, 1, &batch[4..])
                .unwrap()
                .as_slice(),
            b"PRIVATE-APPLICATION"
        );
        assert_eq!(*application.0.lock().unwrap(), vec![binding]);
        send(&mut socket, &mut endpoint, 2);
        assert!(read_frame(&mut socket, records::MAX_BATCH + 4).is_err());
        assert_eq!(application.0.lock().unwrap().len(), 1);
        stop.store(true, Ordering::SeqCst);
        assert!(worker.join().unwrap().is_ok());
    }
}
