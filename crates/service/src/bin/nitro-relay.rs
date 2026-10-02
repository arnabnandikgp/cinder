//! Bounded parent TCP-to-enclave opaque ingress; no key or customer-state I/O.
fn run() -> Result<(), cinder_service::Error> {
    use cinder_service::{
        Error,
        transport::{relay_vsock, stop_on_stdin},
        vsock::Target,
    };
    use std::{
        io::Write,
        net::{SocketAddr, TcpListener},
    };
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err(Error);
    }
    let address: SocketAddr = args[0].parse().map_err(|_| Error)?;
    let cid = args[1].parse().map_err(|_| Error)?;
    let port = args[2].parse().map_err(|_| Error)?;
    let target = Target::new(cid, port)?;
    if !address.ip().is_loopback() || cid <= 3 {
        return Err(Error);
    }
    let listener = TcpListener::bind(address)?;
    println!("{}", listener.local_addr()?);
    std::io::stdout().flush()?;
    relay_vsock(listener, target, stop_on_stdin())
}
fn main() {
    if run().is_err() {
        eprintln!("nitro relay unavailable");
        std::process::exit(1);
    }
}
