//! Explicitly enabled offline fixture entrypoint, never a production fallback.
fn run() -> Result<(), cinder_service::Error> {
    use cinder_service::{Error, fixture::*, transport::*};
    use std::{
        io::{Read, Write},
        net::{SocketAddr, TcpListener},
        path::Path,
        sync::Arc,
    };
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err(Error);
    }
    let listen: SocketAddr = args[0].parse().map_err(|_| Error)?;
    if !listen.ip().is_loopback() {
        return Err(Error);
    }
    let wallet = decode_hex(&args[2])?.try_into().map_err(|_| Error)?;
    let mut key = zeroize::Zeroizing::new([0; 32]);
    std::io::stdin().read_exact(&mut *key)?;
    let handler = FixtureHandler::open(Path::new(&args[1]), key, wallet)?;
    let attester = FixtureAttester::new()?;
    let root = attester.root().to_der()?;
    let server = Server::new(
        Identity::generate(&FixtureClock)?,
        policy(),
        Arc::new(attester),
        Arc::new(FixtureClock),
        Arc::new(handler),
    )?;
    let listener = TcpListener::bind(listen)?;
    println!(
        "{} {}",
        listener.local_addr()?,
        root.iter().map(|b| format!("{b:02x}")).collect::<String>()
    );
    std::io::stdout().flush()?;
    server.run(listener, stop_on_stdin())
}
fn decode_hex(s: &str) -> Result<Vec<u8>, cinder_service::Error> {
    if s.len() != 64 || !s.is_ascii() {
        return Err(cinder_service::Error);
    }
    s.as_bytes()
        .chunks(2)
        .map(|b| {
            u8::from_str_radix(
                std::str::from_utf8(b).map_err(|_| cinder_service::Error)?,
                16,
            )
            .map_err(|_| cinder_service::Error)
        })
        .collect()
}
fn main() {
    if run().is_err() {
        eprintln!("fixture service unavailable");
        std::process::exit(1);
    }
}
