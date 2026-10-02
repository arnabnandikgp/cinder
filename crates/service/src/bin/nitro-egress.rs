//! Fixed-origin opaque parent egress. No signature, payload or key parsing.
fn run() -> Result<(), cinder_service::Error> {
    use cinder_pacifica::execution::Origin;
    use cinder_service::{
        Error,
        transport::{relay_egress, stop_on_stdin},
        vsock::{Target, VsockListener},
    };
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err(Error);
    }
    let port = args[0].parse().map_err(|_| Error)?;
    let cid = args[1].parse().map_err(|_| Error)?;
    Target::new(cid, port)?;
    if cid <= 3 {
        return Err(Error);
    }
    let origin = match args[2].as_str() {
        "pacifica-testnet" => Origin::Testnet,
        "pacifica-mainnet" => Origin::Mainnet,
        _ => return Err(Error),
    };
    let listener = VsockListener::bind(port, cid)?;
    relay_egress(listener, origin, stop_on_stdin())
}
fn main() {
    if run().is_err() {
        eprintln!("nitro egress unavailable");
        std::process::exit(1);
    }
}
