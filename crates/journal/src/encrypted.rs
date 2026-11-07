//! Versioned record AEAD. Keys belong only in the trusted runtime; no key loading
//! from environment/files and no deterministic nonce derived from a journal slot.

use crate::{Error, Protection, RecordContext, model::PrivateBytes, wire::MAX_RECORD};
use chacha20poly1305::{
    KeyInit, XChaCha20Poly1305, XNonce,
    aead::{Aead, OsRng, Payload, rand_core::RngCore},
};
use zeroize::Zeroizing;

const MAGIC: &[u8; 10] = b"CINDER-AE1";
const HEADER: usize = 10 + 8 + 24;

/// One explicit storage-key generation. No Clone/Debug or key-export accessor.
/// Random 192-bit nonces are drawn for every proposal, including losing CAS
/// writes and retries. Entropy failure stops sealing. Key release/rotation and
/// approved hardware are external qualification, not supplied by this type.
pub struct RecordCipher {
    key: Zeroizing<[u8; 32]>,
    generation: u64,
    stream: [u8; 32],
}
impl RecordCipher {
    /// Move an explicitly supplied key into the trusted runtime. The caller must
    /// protect any source copies and validate its release policy independently.
    pub fn new(key: Zeroizing<[u8; 32]>, generation: u64, stream: [u8; 32]) -> Result<Self, Error> {
        if generation == 0 || stream == [0; 32] {
            return Err(Error::Invalid);
        }
        Ok(Self {
            key,
            generation,
            stream,
        })
    }
    fn aad(&self, context: RecordContext, header: &[u8]) -> Vec<u8> {
        [
            b"CINDER-STORAGE-RECORD-1\0".as_slice(),
            &self.stream,
            &context.domain.network.bytes(),
            &context.domain.deployment.bytes(),
            &context.sequence.to_be_bytes(),
            &context.previous,
            header,
        ]
        .concat()
    }
}
impl Protection for RecordCipher {
    fn seal(&self, context: RecordContext, plaintext: &[u8]) -> Result<PrivateBytes, Error> {
        if plaintext.len() > MAX_RECORD - HEADER - 16 {
            return Err(Error::Limit);
        }
        let mut nonce = [0; 24];
        OsRng
            .try_fill_bytes(&mut nonce)
            .map_err(|_| Error::Protection)?;
        let mut out = Vec::with_capacity(HEADER + plaintext.len() + 16);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&self.generation.to_be_bytes());
        out.extend_from_slice(&nonce);
        let aad = self.aad(context, &out);
        let ciphertext = XChaCha20Poly1305::new((&*self.key).into())
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: plaintext,
                    aad: &aad,
                },
            )
            .map_err(|_| Error::Protection)?;
        out.extend_from_slice(&ciphertext);
        PrivateBytes::new(out)
    }
    fn open(&self, context: RecordContext, opaque: &PrivateBytes) -> Result<PrivateBytes, Error> {
        let bytes = opaque.as_bytes();
        if bytes.len() < HEADER + 16
            || &bytes[..10] != MAGIC
            || bytes[10..18] != self.generation.to_be_bytes()
        {
            return Err(Error::Protection);
        }
        let aad = self.aad(context, &bytes[..HEADER]);
        let plaintext = XChaCha20Poly1305::new((&*self.key).into())
            .decrypt(
                XNonce::from_slice(&bytes[18..HEADER]),
                Payload {
                    msg: &bytes[HEADER..],
                    aad: &aad,
                },
            )
            .map_err(|_| Error::Protection)?;
        PrivateBytes::new(plaintext)
    }
}
