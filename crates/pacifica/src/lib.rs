//! Pacifica-specific evidence boundary. No sockets, keys, clocks or ledger copies.
pub mod execution;
mod native;
pub mod observation;
pub mod profile;
use cinder_kernel::{codec::Canonical, identity::AttemptKey};
use sha2::{Digest, Sha256};

/// Deterministic native UUID for a domain/account/attempt, never a customer ID.
/// The full attempt remains authoritative; a native UUID collision must reject.
pub fn client_id(attempt: AttemptKey) -> String {
    let mut hash = Sha256::new();
    hash.update(b"CINDER-PACIFICA-CLOID-1\0");
    hash.update(attempt.encode());
    let bytes: [u8; 32] = hash.finalize().into();
    let hex: String = bytes[..16].iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

/// Secret-safe failure; never embeds a response, account or signing payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Malformed, ambiguous, unsupported or off-grid native data.
    Codec,
    /// Bounded work/storage limit exceeded.
    Limit,
    /// Unqualified capability, scope or changed configuration.
    Qualification,
    /// Exact durable journal error; unknown outcome requires reopen/reconciliation.
    Journal(cinder_journal::Error),
}
impl From<cinder_journal::Error> for Error {
    fn from(e: cinder_journal::Error) -> Self {
        Self::Journal(e)
    }
}
impl From<cinder_kernel::Error> for Error {
    fn from(_: cinder_kernel::Error) -> Self {
        Self::Codec
    }
}
