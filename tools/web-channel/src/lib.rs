//! Bounded shared Noise qualification core, NOT an attestation verifier or API.
//! The fixture harness independently supplies responder-key trust and context.
//! No financial identities, signer, journal, persistence, or HTTP listener exists.
use snow::{Builder, HandshakeState, TransportState};
use zeroize::Zeroizing;

/// Canonical public attested-context encodings; qualification, not a verifier.
pub mod profile;

/// The only qualification suite; no negotiation, modifiers, or crypto fallback.
pub const PROFILE: &str = "Noise_NK_25519_ChaChaPoly_SHA256";
/// Bound one record independently of any future reply-chunking protocol.
pub const MAX_PAYLOAD: usize = 16_384;
/// Finite records per direction, including key-confirmation traffic.
pub const MAX_RECORDS: u32 = 128;
const READY: &[u8] = b"CINDER-WEB-QUALIFICATION-READY\0";
const ACK: &[u8] = b"CINDER-WEB-QUALIFICATION-ACK\0";

/// Redacted error; failure consumes the endpoint, never a retry permission.
#[derive(Debug, PartialEq, Eq)]
pub struct Error;

enum Phase {
    ClientStart(HandshakeState),
    ClientWait(HandshakeState),
    ServerWait(HandshakeState),
    ClientConfirm(TransportState),
    ServerConfirm(TransportState),
    Ready(TransportState),
    Closed,
}

/// The same state machine is compiled for native Rust and browser/WASM.
/// Key confirmation precedes private-record release; errors are sticky.
pub struct Endpoint {
    phase: Phase,
    public: Vec<u8>,
    binding: Option<[u8; 32]>,
    sent: u32,
    received: u32,
}
impl Endpoint {
    fn builder(context: &[u8]) -> Result<Builder<'_>, Error> {
        if context.len() != 32 || context.iter().all(|b| *b == 0) {
            return Err(Error);
        }
        Builder::new(PROFILE.parse().map_err(|_| Error)?)
            .prologue(context)
            .map_err(|_| Error)
    }
    /// Fixture-provided responder key must already be trusted by the caller.
    /// This method itself does NOT verify attestation; do not publish it as an SDK.
    pub fn client(public: &[u8], context: &[u8]) -> Result<Self, Error> {
        if public.len() != 32 || public.iter().all(|b| *b == 0) {
            return Err(Error);
        }
        let inner = Self::builder(context)?
            .remote_public_key(public)
            .map_err(|_| Error)?
            .build_initiator()
            .map_err(|_| Error)?;
        Ok(Self {
            phase: Phase::ClientStart(inner),
            public: public.to_vec(),
            binding: None,
            sent: 0,
            received: 0,
        })
    }
    /// Generate a fresh synthetic responder using the platform's CSPRNG.
    /// This is not an NSM attester or qualified production entropy provider.
    pub fn server(context: &[u8]) -> Result<Self, Error> {
        let context: [u8; 32] = context.try_into().map_err(|_| Error)?;
        Self::server_with_context(|_| Ok(context))
    }
    /// Disposable native fixture: generate a fresh key, then obtain the public
    /// prologue for that exact key. The callback is NOT an NSM/trust provider.
    pub fn server_with_context(
        context_for_key: impl FnOnce(&[u8]) -> Result<[u8; 32], Error>,
    ) -> Result<Self, Error> {
        let key = Self::builder(&[1; 32])?
            .generate_keypair()
            .map_err(|_| Error)?;
        let private = Zeroizing::new(key.private);
        let context = context_for_key(&key.public)?;
        let inner = Self::builder(&context)?
            .local_private_key(&private)
            .map_err(|_| Error)?
            .build_responder()
            .map_err(|_| Error)?;
        Ok(Self {
            phase: Phase::ServerWait(inner),
            public: key.public,
            binding: None,
            sent: 0,
            received: 0,
        })
    }
    /// Public fixture key only; never exports a private key or library state.
    pub fn public_key(&self) -> &[u8] {
        &self.public
    }
    /// Whether the standard handshake and both confirmation records succeeded.
    pub fn ready(&self) -> bool {
        matches!(self.phase, Phase::Ready(_))
    }
    /// Final standard Noise handshake hash, only after key confirmation.
    /// The final web profile still needs its separate attestation-bound encoding.
    pub fn binding(&self) -> Result<[u8; 32], Error> {
        if !self.ready() {
            return Err(Error);
        }
        self.binding.ok_or(Error)
    }
    /// Emit the empty-payload first handshake message exactly once.
    pub fn start(&mut self) -> Result<Vec<u8>, Error> {
        let Phase::ClientStart(mut inner) = std::mem::replace(&mut self.phase, Phase::Closed)
        else {
            return Err(Error);
        };
        let mut out = vec![0; 48];
        let n = inner.write_message(&[], &mut out).map_err(|_| Error)?;
        if n != 48 {
            return Err(Error);
        }
        self.phase = Phase::ClientWait(inner);
        Ok(out)
    }
    /// One handshake/confirmation step; no private handshake payload is accepted.
    /// Any malformed input or invalid phase permanently closes this endpoint.
    pub fn advance(&mut self, wire: &[u8]) -> Result<Vec<u8>, Error> {
        let phase = std::mem::replace(&mut self.phase, Phase::Closed);
        if wire.is_empty() || wire.len() > MAX_PAYLOAD + 16 {
            return Err(Error);
        }
        match phase {
            Phase::ServerWait(mut h) => {
                Self::empty_handshake(&mut h, wire)?;
                let mut out = vec![0; 48];
                if h.write_message(&[], &mut out).map_err(|_| Error)? != 48 {
                    return Err(Error);
                }
                self.binding = Some(h.get_handshake_hash().try_into().map_err(|_| Error)?);
                self.phase = Phase::ServerConfirm(h.into_transport_mode().map_err(|_| Error)?);
                Ok(out)
            }
            Phase::ClientWait(mut h) => {
                Self::empty_handshake(&mut h, wire)?;
                self.binding = Some(h.get_handshake_hash().try_into().map_err(|_| Error)?);
                let mut t = h.into_transport_mode().map_err(|_| Error)?;
                let out = Self::encrypt(&mut t, READY)?;
                self.sent = 1;
                self.phase = Phase::ClientConfirm(t);
                Ok(out)
            }
            Phase::ServerConfirm(mut t) => {
                if Self::decrypt(&mut t, wire)?.as_slice() != READY {
                    return Err(Error);
                }
                let out = Self::encrypt(&mut t, ACK)?;
                self.sent = 1;
                self.received = 1;
                self.phase = Phase::Ready(t);
                Ok(out)
            }
            Phase::ClientConfirm(mut t) => {
                if Self::decrypt(&mut t, wire)?.as_slice() != ACK {
                    return Err(Error);
                }
                self.received = 1;
                self.phase = Phase::Ready(t);
                Ok(Vec::new())
            }
            _ => Err(Error),
        }
    }
    fn empty_handshake(h: &mut HandshakeState, wire: &[u8]) -> Result<(), Error> {
        if wire.len() != 48 {
            return Err(Error);
        }
        // NK permits early payloads; this profile explicitly disallows them.
        if h.read_message(wire, &mut []).map_err(|_| Error)? != 0 {
            return Err(Error);
        }
        Ok(())
    }
    fn encrypt(t: &mut TransportState, body: &[u8]) -> Result<Vec<u8>, Error> {
        if body.is_empty() || body.len() > MAX_PAYLOAD {
            return Err(Error);
        }
        let mut out = vec![0; body.len() + 16];
        let n = t.write_message(body, &mut out).map_err(|_| Error)?;
        out.truncate(n);
        Ok(out)
    }
    fn decrypt(t: &mut TransportState, wire: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
        if wire.len() <= 16 || wire.len() > MAX_PAYLOAD + 16 {
            return Err(Error);
        }
        let mut clear = Zeroizing::new(vec![0; wire.len() - 16]);
        let n = t.read_message(wire, &mut clear).map_err(|_| Error)?;
        clear.truncate(n);
        Ok(clear)
    }
    /// Encrypt one bounded record only after handshake and key confirmation.
    /// Counts are not restored or reused; failure makes the state inaccessible.
    pub fn seal(&mut self, body: &[u8]) -> Result<Vec<u8>, Error> {
        let Phase::Ready(mut t) = std::mem::replace(&mut self.phase, Phase::Closed) else {
            return Err(Error);
        };
        if self.sent >= MAX_RECORDS {
            return Err(Error);
        }
        let out = Self::encrypt(&mut t, body)?;
        self.sent += 1;
        self.phase = Phase::Ready(t);
        Ok(out)
    }
    /// Authenticate/decrypt one in-order record; never return partial plaintext.
    pub fn open(&mut self, wire: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
        let Phase::Ready(mut t) = std::mem::replace(&mut self.phase, Phase::Closed) else {
            return Err(Error);
        };
        if self.received >= MAX_RECORDS {
            return Err(Error);
        }
        let out = Self::decrypt(&mut t, wire)?;
        self.received += 1;
        self.phase = Phase::Ready(t);
        Ok(out)
    }
}

#[cfg(target_arch = "wasm32")]
mod browser;

#[cfg(any(test, target_arch = "wasm32"))]
mod known_answer;

#[cfg(test)]
mod tests {
    #[test]
    fn published_vector() {
        assert!(super::known_answer::verify());
    }
}
