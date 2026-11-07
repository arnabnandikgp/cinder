//! Opaque bounded parent relay entrypoint.
fn run() -> Result<(), cinder_service::Error> {
    use std::{
        io::Write,
        net::{SocketAddr, TcpListener},
    };
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err(cinder_service::Error);
    }
    let listen: SocketAddr = args[0].parse().map_err(|_| cinder_service::Error)?;
    let target: SocketAddr = args[1].parse().map_err(|_| cinder_service::Error)?;
    if !listen.ip().is_loopback() || !target.ip().is_loopback() {
        return Err(cinder_service::Error);
    }
    let listener = TcpListener::bind(listen)?;
    println!("{}", listener.local_addr()?);
    std::io::stdout().flush()?;
    cinder_service::transport::relay(listener, target, cinder_service::transport::stop_on_stdin())
}
fn main() {
    if run().is_err() {
        eprintln!("relay unavailable");
        std::process::exit(1);
    }
}
