//! Bounded public-data-only native verifier bridge for the Node SDK. The client
//! distributes this executable independently; the relay never selects its path.
use crate::{
    Error,
    attestation::{self, Context, MAX_QUOTE, Policy},
};
use std::io::{Read, Write};

/// Single verification request. This is not a listener or remote trust oracle.
/// The callback is fixed in each separate binary, not selected by an input flag.
pub fn run(
    verify: impl FnOnce(&[u8], &Policy, &Context<'_>) -> Result<(), Error>,
) -> Result<(), Error> {
    let mut b = Vec::new();
    std::io::stdin()
        .take((MAX_QUOTE + 4096) as u64)
        .read_to_end(&mut b)?;
    if b.len() < 360 || b.len() >= MAX_QUOTE + 4096 {
        return Err(Error);
    }
    let policy = Policy::decode(&b[..240])?;
    let n = u32::from_be_bytes(b[352..356].try_into().map_err(|_| Error)?) as usize;
    if n == 0 || n > 1024 || b.len() < 356 + n {
        return Err(Error);
    }
    let c = Context {
        nonce: b[240..272].try_into().map_err(|_| Error)?,
        exporter: b[272..304].try_into().map_err(|_| Error)?,
        boot: b[304..336].try_into().map_err(|_| Error)?,
        expires: u64::from_be_bytes(b[336..344].try_into().map_err(|_| Error)?),
        now: u64::from_be_bytes(b[344..352].try_into().map_err(|_| Error)?),
        spki: &b[356..356 + n],
    };
    verify(&b[356 + n..], &policy, &c)?;
    std::io::stdout().write_all(&attestation::binding(&policy, &c))?;
    Ok(())
}
