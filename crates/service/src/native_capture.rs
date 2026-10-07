//! One bounded account_transfers capture over enclave-terminated TLS 1.3.
//! No automatic reconnect, ambient TLS roots, custom URL or parent plaintext.
//! The journal sink must return before the next message is consumed.
use crate::{
    Error,
    egress::Trust,
    transport::{Clock, Lifetime, Socket},
    vsock::{Target, VsockStream},
};
use cinder_pacifica::{
    capture::{Kind, Query, Record, Request},
    execution::Origin,
};
use openssl::sha::sha256;
use serde::{Deserialize, Serialize};
use std::{
    io::{self, Read, Write},
    sync::Arc,
    time::{Duration, Instant},
};
use tungstenite::{Message, client::IntoClientRequest, protocol::WebSocketConfig};

const MAX_HEADERS: usize = 8192;
/// Measured public trust and finite bounds for exactly one capture per boot.
/// It neither authorizes money movement nor promises stream completeness.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// Dedicated parent CID 3 route, distinct from every other manifest port.
    pub port: u32,
    /// Explicit canonical DER trust anchor; no ambient root discovery.
    pub root: Vec<u8>,
    /// Independently approved digest of that exact anchor.
    pub root_hash: [u8; 32],
    /// Absolute time, decoded message and byte ceilings.
    pub limits: cinder_pacifica::capture::Limits,
}
impl Policy {
    /// Validate without opening a socket or releasing a credential.
    pub fn validate(&self) -> Result<(), Error> {
        self.limits.validate().map_err(|_| Error)?;
        Target::new(3, self.port)?;
        Trust::from_der(&self.root, self.root_hash)?;
        Ok(())
    }
}
/// Actual consumed policy. Both provisioning and runtime use this derivation.
pub(crate) struct Loaded {
    origin: Origin,
    policy: Policy,
    trust: Trust,
}
impl Loaded {
    pub(crate) fn new(origin: Origin, policy: &Policy) -> Result<Self, Error> {
        policy.validate()?;
        Ok(Self {
            origin,
            policy: policy.clone(),
            trust: Trust::from_der(&policy.root, policy.root_hash)?,
        })
    }
    pub(crate) fn commitment(&self) -> Result<[u8; 32], Error> {
        Ok(sha256(
            &[
                b"CINDER-LOADED-NATIVE-CAPTURE-1\0".as_slice(),
                &route_commitment(self.origin, Target::new(3, self.policy.port)?, &self.trust),
                &serde_cbor::to_vec(&self.policy.limits).map_err(|_| Error)?,
            ]
            .concat(),
        ))
    }
    pub(crate) fn matches(&self, policy: &Policy) -> bool {
        self.policy == *policy
    }
    pub(crate) fn open(
        self,
        clock: Arc<dyn Clock>,
    ) -> Result<(Port, cinder_pacifica::capture::Limits), Error> {
        Ok((
            Port::new(
                self.origin,
                Target::new(3, self.policy.port)?,
                self.trust,
                clock,
            )?,
            self.policy.limits,
        ))
    }
}
/// Fixed native WS hosts, separate from Pacifica's REST API hosts.
pub(crate) fn host(origin: Origin) -> &'static str {
    match origin {
        Origin::Testnet => "test-ws.pacifica.fi",
        Origin::Mainnet => "ws.pacifica.fi",
    }
}
/// Loaded transport, not permission for a live experiment. Request capabilities
/// come only from the durable shared read budget and exact pooled account.
pub struct Port {
    origin: Origin,
    target: Target,
    trust: Trust,
    clock: Arc<dyn Clock>,
}
impl Port {
    /// No connection, root discovery or credential release at construction.
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
    /// Actual route/root/origin component for later loaded-manifest integration.
    pub fn commitment(&self) -> [u8; 32] {
        route_commitment(self.origin, self.target, &self.trust)
    }
    /// One request, one connection, one exact subscription. No order/withdrawal
    /// method exists. The sink briefly reacquires the writer to retain each fact.
    /// Sink failure ends capture immediately; never continue past an uncertain
    /// journal commit, and never retry or label the capture economically complete.
    pub fn capture(
        &self,
        request: Request,
        mut sink: impl FnMut(Record) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let start = Instant::now();
        let now = self.clock.now()?;
        let mut query = request.consume(now).map_err(|_| Error)?;
        if query.origin() != self.origin {
            return Err(Error);
        }
        let Some(timeout) = connect_budget(query.limits().maximum_ms, start.elapsed()) else {
            return stop(&mut query, Kind::Limited, &mut sink);
        };
        let socket = match VsockStream::connect_bounded(self.target, timeout) {
            Ok(socket) => socket,
            Err(_) => return stop(&mut query, Kind::Interrupted, &mut sink),
        };
        run(
            socket,
            &self.trust,
            self.clock.as_ref(),
            query,
            start,
            &mut sink,
        )
    }
}
fn connect_budget(maximum_ms: u64, elapsed: Duration) -> Option<Duration> {
    Duration::from_millis(maximum_ms)
        .checked_sub(elapsed)
        .filter(|d| !d.is_zero())
        .map(|d| d.min(Duration::from_secs(5)))
}
fn route_commitment(origin: Origin, target: Target, trust: &Trust) -> [u8; 32] {
    sha256(
        &[
            b"CINDER-NATIVE-WSS-1\0".as_slice(),
            host(origin).as_bytes(),
            &target.encode(),
            &trust.digest(),
        ]
        .concat(),
    )
}
/// Count plaintext wire bytes as well as decoded message limits. Before upgrade,
/// this also bounds the library's otherwise larger HTTP header allocation.
struct Metered<S> {
    inner: S,
    remaining: usize,
}
impl<S: Read> Read for Metered<S> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let n = bytes.len().min(self.remaining);
        let read = self.inner.read(&mut bytes[..n])?;
        self.remaining -= read;
        Ok(read)
    }
}
impl<S: Write> Write for Metered<S> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.inner.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
fn stop(
    query: &mut Query,
    kind: Kind,
    sink: &mut impl FnMut(Record) -> Result<(), Error>,
) -> Result<(), Error> {
    sink(
        query
            .record(kind, vec![], query.last_at())
            .map_err(|_| Error)?,
    )
}
fn run<S: Socket>(
    socket: S,
    trust: &Trust,
    clock: &dyn Clock,
    mut query: Query,
    start: Instant,
    sink: &mut impl FnMut(Record) -> Result<(), Error>,
) -> Result<(), Error> {
    let limits = query.limits();
    let duration = Duration::from_millis(limits.maximum_ms);
    let Some(remaining) = duration
        .checked_sub(start.elapsed())
        .filter(|d| !d.is_zero())
    else {
        return stop(&mut query, Kind::Limited, sink);
    };
    let deadline_socket = match socket.try_clone() {
        Ok(s) => s,
        Err(_) => return stop(&mut query, Kind::Interrupted, sink),
    };
    let _lifetime = Lifetime::bounded(deadline_socket, remaining);
    if socket.prepare().is_err() {
        return stop(&mut query, Kind::Interrupted, sink);
    }
    let opening = (|| {
        let now = clock.now()?;
        if now < query.last_at() {
            return Err(Error);
        }
        let mut settings = trust.connector()?.configure()?;
        settings
            .param_mut()
            .set_time((now / 1000).try_into().map_err(|_| Error)?);
        let tls = settings
            .connect(host(query.origin()), socket)
            .map_err(|_| Error)?;
        if tls.ssl().version_str() != "TLSv1.3"
            || tls.ssl().session_reused()
            || tls
                .ssl()
                .selected_alpn_protocol()
                .is_some_and(|p| p != b"http/1.1")
        {
            return Err(Error);
        }
        let request = format!("wss://{}/ws", host(query.origin()))
            .into_client_request()
            .map_err(|_| Error)?;
        let config = WebSocketConfig::default()
            .read_buffer_size(4096)
            .write_buffer_size(0)
            .max_write_buffer_size(4096)
            .max_message_size(Some(limits.maximum_message))
            .max_frame_size(Some(limits.maximum_message))
            .accept_unmasked_frames(false);
        let (mut ws, response) = tungstenite::client::client_with_config(
            request,
            Metered {
                inner: tls,
                remaining: MAX_HEADERS,
            },
            Some(config),
        )
        .map_err(|_| Error)?;
        // No negotiated compression, extension, cookie or subprotocol is needed.
        // Refuse extensions/subprotocols rather than parsing altered semantics.
        if response.headers().contains_key("sec-websocket-extensions")
            || response.headers().contains_key("sec-websocket-protocol")
            || response
                .headers()
                .keys()
                .any(|k| response.headers().get_all(k).iter().count() != 1)
        {
            return Err(Error);
        }
        // The wire budget also bounds empty fragments that don't yield messages.
        ws.get_mut().remaining = limits
            .maximum_bytes
            .checked_add((limits.maximum_messages as usize + 2) * 14 + 1024)
            .ok_or(Error)?;
        let subscribe = serde_json::json!({
            "method":"subscribe",
            "params":{"source":"account_transfers","account":query.account()}
        })
        .to_string();
        ws.send(Message::Text(subscribe.into()))
            .map_err(|_| Error)?;
        Ok::<_, Error>(ws)
    })();
    let mut ws = match opening {
        Ok(ws) => ws,
        Err(_) => return stop(&mut query, Kind::Interrupted, sink),
    };
    let opened_at = match clock.now() {
        Ok(at) if at >= query.last_at() => at,
        _ => return stop(&mut query, Kind::Interrupted, sink),
    };
    if start.elapsed() >= duration {
        return stop(&mut query, Kind::Limited, sink);
    }
    sink(
        query
            .record(Kind::Opened, vec![], opened_at)
            .map_err(|_| Error)?,
    )?;
    let mut messages = 0;
    let mut bytes = 0usize;
    loop {
        if start.elapsed() >= duration
            || messages >= limits.maximum_messages
            || bytes >= limits.maximum_bytes
        {
            return stop(&mut query, Kind::Limited, sink);
        }
        let message = match ws.read() {
            Ok(message) => message,
            Err(_) => return stop(&mut query, Kind::Interrupted, sink),
        };
        let (kind, body) = match message {
            Message::Text(body) => (Kind::Text, body.as_bytes().to_vec()),
            Message::Binary(body) => (Kind::Binary, body.to_vec()),
            Message::Ping(body) => (Kind::Ping, body.to_vec()),
            Message::Pong(body) => (Kind::Pong, body.to_vec()),
            Message::Close(body) => (
                Kind::Closed,
                body.map_or_else(Vec::new, |c| {
                    let mut b = u16::from(c.code).to_be_bytes().to_vec();
                    b.extend_from_slice(c.reason.as_bytes());
                    b
                }),
            ),
            Message::Frame(_) => return stop(&mut query, Kind::Interrupted, sink),
        };
        if body.len() > limits.maximum_message
            || bytes
                .checked_add(body.len())
                .is_none_or(|n| n > limits.maximum_bytes)
        {
            return stop(&mut query, Kind::Limited, sink);
        }
        let at = match clock.now() {
            Ok(at) if at >= query.last_at() && start.elapsed() < duration => at,
            _ => {
                // Bytes were already received. Retain uncertainty explicitly,
                // never timestamp them as fresh or consume another message.
                return sink(query.record_unverified(kind, body).map_err(|_| Error)?);
            }
        };
        bytes += body.len();
        messages += 1;
        sink(query.record(kind, body, at).map_err(|_| Error)?)?;
        // Flush automatic RFC pong/close responses only, never another
        // subscription or a financial action. Client masking is library-owned.
        if kind == Kind::Closed {
            let _ = ws.flush();
            return Ok(());
        }
        if ws.flush().is_err() {
            return stop(&mut query, Kind::Interrupted, sink);
        }
    }
}

#[cfg(test)]
#[path = "native_capture_tests.rs"]
mod tests;
