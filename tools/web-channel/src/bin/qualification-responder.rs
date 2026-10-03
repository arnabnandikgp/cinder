//! Local public-fixture responder. NOT a service, attester, signer or API.
use cinder_web_channel_qualification::{Endpoint, Error, MAX_PAYLOAD, MAX_RECORDS, profile};
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
    let args: Vec<_> = std::env::args().skip(1).collect();
    let attested_fixture = args.as_slice() == ["--attested-fixture"];
    if !args.is_empty() && !attested_fixture {
        return Err(());
    }
    let mut out = io::stdout().lock();
    let mut input = io::stdin().lock();
    let mut context = [1; 32];
    let mut endpoint = Endpoint::server_with_context(|public| {
        writeln!(out, "{}", hex(public)).map_err(|_| Error)?;
        out.flush().map_err(|_| Error)?;
        if attested_fixture {
            context = read_wire(&mut input, 65)
                .map_err(|_| Error)?
                .ok_or(Error)?
                .try_into()
                .map_err(|_| Error)?;
            writeln!(out, "{}", hex(&context)).map_err(|_| Error)?;
            out.flush().map_err(|_| Error)?;
        }
        Ok(context)
    })
    .map_err(|_| ())?;
    for _ in 0..MAX_RECORDS + 1 {
        let Some(wire) = read_wire(&mut input, 2 * (MAX_PAYLOAD + 16) + 1)? else {
            return Ok(());
        };
        let reply = if endpoint.ready() {
            let clear = endpoint.open(&wire).map_err(|_| ())?;
            if attested_fixture && clear.as_slice() == b"CINDER-QUALIFICATION-BINDING" {
                let h = endpoint.binding().map_err(|_| ())?;
                endpoint
                    .seal(&profile::binding(&context, &h))
                    .map_err(|_| ())?
            } else {
                // Fixture echo goes back encrypted; never write plaintext to stdout.
                endpoint.seal(&clear).map_err(|_| ())?
            }
        } else {
            endpoint.advance(&wire).map_err(|_| ())?
        };
        writeln!(out, "{}", hex(&reply)).map_err(|_| ())?;
        out.flush().map_err(|_| ())?;
    }
    Err(())
}
fn read_wire(input: &mut impl BufRead, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
    // A finite line, not an unbounded read_line allocation.
    let mut line = Vec::new();
    while !line.ends_with(b"\n") {
        let available = input.fill_buf().map_err(|_| ())?;
        if available.is_empty() {
            return if line.is_empty() { Ok(None) } else { Err(()) };
        }
        let n = available
            .iter()
            .position(|b| *b == b'\n')
            .map_or(available.len(), |n| n + 1);
        if line.len() + n > maximum {
            return Err(());
        }
        line.extend_from_slice(&available[..n]);
        input.consume(n);
    }
    line.pop();
    Ok(Some(unhex(&line).ok_or(())?))
}
fn main() {
    if run().is_err() {
        eprintln!("Qualification channel unavailable");
        std::process::exit(1);
    }
}
