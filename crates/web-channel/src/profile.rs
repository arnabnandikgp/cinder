//! Canonical public context and P18-sized session-binding candidate. Distinct
//! purposes from TLS/clock/KMS. No quote/path/time verification happens here.
use crate::{Error, PROFILE};
use sha2::{Digest, Sha256, Sha384};

/// Exact independently selected release policy plus this connection's public
/// challenge, boot, routing handle, attested responder key and expiry.
pub fn context(policy: &[u8], fields: &[u8], expires: u64) -> Result<Vec<u8>, Error> {
    if policy.len() != 240 || fields.len() != 128 {
        return Err(Error);
    }
    for (lo, hi) in [
        (0, 32),
        (32, 64),
        (64, 96),
        (96, 144),
        (144, 192),
        (192, 240),
    ] {
        if policy[lo..hi].iter().all(|b| *b == 0) {
            return Err(Error);
        }
    }
    if fields.chunks_exact(32).any(|b| b.iter().all(|v| *v == 0)) || expires == 0 {
        return Err(Error);
    }
    Ok([
        b"CINDER-WEB-CONTEXT-1\0".as_slice(),
        PROFILE.as_bytes(),
        &[0],
        policy,
        fields,
        &expires.to_be_bytes(),
    ]
    .concat())
}
/// NSM user_data value for the complete canonical public context.
pub fn user_data(context: &[u8]) -> [u8; 48] {
    Sha384::digest(context).into()
}
/// Noise prologue commits exact quote bytes, not an operator's approval flag.
pub fn prologue(context: &[u8], quote: &[u8]) -> Result<[u8; 32], Error> {
    if quote.is_empty() || quote.len() > 16_384 {
        return Err(Error);
    }
    Ok(Sha256::digest(
        [
            b"CINDER-WEB-PROLOGUE-1\0".as_slice(),
            context,
            &Sha256::digest(quote),
        ]
        .concat(),
    )
    .into())
}
/// Final authentication binding, used only after bidirectional confirmation.
pub fn binding(prologue: &[u8; 32], confirmed_handshake_hash: &[u8; 32]) -> [u8; 32] {
    Sha256::digest(
        [
            b"CINDER-WEB-SESSION-1\0".as_slice(),
            prologue,
            confirmed_handshake_hash,
        ]
        .concat(),
    )
    .into()
}
