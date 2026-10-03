//! Local public-fixture responder. NOT a service, attester, signer or API.
use cinder_web_channel_qualification::{Endpoint, MAX_PAYLOAD, MAX_RECORDS};
use std::io::{self, BufRead, Write};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(bytes: &[u8]) -> Option<Vec<u8>> {
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    bytes
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(text, 16).ok()
        })
        .collect()
}
fn run() -> Result<(), ()> {
    let mut endpoint = Endpoint::server(&[1; 32]).map_err(|_| ())?;
    let mut out = io::stdout().lock();
    writeln!(out, "{}", hex(endpoint.public_key())).map_err(|_| ())?;
    out.flush().map_err(|_| ())?;
    let mut input = io::stdin().lock();
    for _ in 0..MAX_RECORDS + 1 {
        // A finite line, not an unbounded read_line allocation.
        let mut line = Vec::new();
        while !line.ends_with(b"\n") {
            let available = input.fill_buf().map_err(|_| ())?;
            if available.is_empty() {
                return if line.is_empty() { Ok(()) } else { Err(()) };
            }
            let n = available
                .iter()
                .position(|b| *b == b'\n')
                .map_or(available.len(), |n| n + 1);
            if line.len() + n > 2 * (MAX_PAYLOAD + 16) + 1 {
                return Err(());
            }
            line.extend_from_slice(&available[..n]);
            input.consume(n);
        }
        line.pop();
        let wire = unhex(&line).ok_or(())?;
        let reply = if endpoint.ready() {
            let clear = endpoint.open(&wire).map_err(|_| ())?;
            // Fixture echo goes back encrypted; never write plaintext to stdout.
            endpoint.seal(&clear).map_err(|_| ())?
        } else {
            endpoint.advance(&wire).map_err(|_| ())?
        };
        writeln!(out, "{}", hex(&reply)).map_err(|_| ())?;
        out.flush().map_err(|_| ())?;
    }
    Err(())
}
fn main() {
    if run().is_err() {
        eprintln!("Qualification channel unavailable");
        std::process::exit(1);
    }
}
