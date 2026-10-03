//! Complete responses are authenticated before release. Correlation is local
//! to a session, never an economic ID or global journal sequence.
use crate::{Endpoint, Error, MAX_PAYLOAD};
use zeroize::Zeroizing;
/// Existing private command wire bound, including signature.
pub const MAX_REQUEST: usize = 1024;
/// Complete authenticated application response bound.
pub const MAX_RESPONSE: usize = 1_048_576;
const HEADER: usize = 14;
const PIECE: usize = MAX_PAYLOAD - HEADER;
/// Maximum complete ciphertext batch including record length prefixes.
pub const MAX_BATCH: usize = MAX_RESPONSE + MAX_RESPONSE.div_ceil(PIECE) * (HEADER + 20);

/// Request envelope; exact order is checked BEFORE the application handler.
pub fn request(sequence: u32, body: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
    if sequence == 0 || body.is_empty() || body.len() > MAX_REQUEST {
        return Err(Error);
    }
    Ok(Zeroizing::new(
        [&[1, 1][..], &sequence.to_be_bytes(), body].concat(),
    ))
}
/// Decode the next authenticated request; trailing data belongs to the signed
/// application's exact decoder, not a second parsing/authorization authority.
pub fn read_request(sequence: u32, body: &[u8]) -> Result<&[u8], Error> {
    if body.len() <= 6
        || body.len() > MAX_REQUEST + 6
        || body[..2] != [1, 1]
        || body[2..6] != sequence.to_be_bytes()
        || sequence == 0
    {
        return Err(Error);
    }
    Ok(&body[6..])
}
/// Encrypt every response chunk into a single bounded outer batch. Never emit a
/// prefix whose later chunks cannot fit the remaining directional record budget.
pub fn response(endpoint: &mut Endpoint, sequence: u32, body: &[u8]) -> Result<Vec<u8>, Error> {
    if sequence == 0 || body.is_empty() || body.len() > MAX_RESPONSE {
        return Err(Error);
    }
    endpoint.require_records(body.len().div_ceil(PIECE) as u32)?;
    let mut batch = Vec::with_capacity(body.len() + body.len().div_ceil(PIECE) * (HEADER + 20));
    for (i, piece) in body.chunks(PIECE).enumerate() {
        let clear = Zeroizing::new(
            [
                &[1, 2][..],
                &sequence.to_be_bytes(),
                &(body.len() as u32).to_be_bytes(),
                &((i * PIECE) as u32).to_be_bytes(),
                piece,
            ]
            .concat(),
        );
        let cipher = endpoint.seal(&clear)?;
        batch.extend_from_slice(&(cipher.len() as u32).to_be_bytes());
        batch.extend_from_slice(&cipher);
    }
    Ok(batch)
}
/// Decrypt an exact complete batch. Reject truncated, duplicated, interleaved,
/// spliced, oversized or trailing chunks before returning ANY application bytes.
pub fn read_response(
    endpoint: &mut Endpoint,
    sequence: u32,
    batch: &[u8],
) -> Result<Zeroizing<Vec<u8>>, Error> {
    let result = read_response_inner(endpoint, sequence, batch);
    if result.is_err() {
        endpoint.close();
    }
    result
}
fn read_response_inner(
    endpoint: &mut Endpoint,
    sequence: u32,
    batch: &[u8],
) -> Result<Zeroizing<Vec<u8>>, Error> {
    if sequence == 0 || batch.is_empty() || batch.len() > MAX_BATCH {
        return Err(Error);
    }
    let mut at = 0;
    let mut total = None;
    let mut output = Zeroizing::new(Vec::new());
    while at < batch.len() {
        let end = at.checked_add(4).ok_or(Error)?;
        let n = u32::from_be_bytes(
            batch
                .get(at..end)
                .ok_or(Error)?
                .try_into()
                .map_err(|_| Error)?,
        ) as usize;
        at = end;
        if !(HEADER + 17..=MAX_PAYLOAD + 16).contains(&n) {
            return Err(Error);
        }
        let end = at.checked_add(n).ok_or(Error)?;
        let clear = endpoint.open(batch.get(at..end).ok_or(Error)?)?;
        at = end;
        if clear.len() <= HEADER || clear[..2] != [1, 2] || clear[2..6] != sequence.to_be_bytes() {
            return Err(Error);
        }
        let size = u32::from_be_bytes(clear[6..10].try_into().map_err(|_| Error)?) as usize;
        let offset = u32::from_be_bytes(clear[10..14].try_into().map_err(|_| Error)?) as usize;
        if size == 0
            || size > MAX_RESPONSE
            || total.is_some_and(|t| t != size)
            || offset != output.len()
            || clear.len() - HEADER != PIECE.min(size.checked_sub(offset).ok_or(Error)?)
        {
            return Err(Error);
        }
        total = Some(size);
        output.extend_from_slice(&clear[HEADER..]);
        if output.len() == size && at != batch.len() {
            return Err(Error);
        }
    }
    if total != Some(output.len()) {
        return Err(Error);
    }
    Ok(output)
}
