//! Bounded parent-known credentials and KMS ciphertext, never private keys.
fn run() -> Result<(), cinder_service::Error> {
    use cinder_service::{
        Error,
        boot::{Bootstrap, serve_bootstrap},
        vsock::VsockListener,
    };
    use std::io::Read;
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err(Error);
    }
    let port = args[0].parse().map_err(|_| Error)?;
    let cid: u32 = args[1].parse().map_err(|_| Error)?;
    if cid <= 3 {
        return Err(Error);
    }
    let mut bytes = zeroize::Zeroizing::new(Vec::new());
    std::io::stdin().take(65537).read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        return Err(Error);
    }
    let boot: Bootstrap = serde_cbor::from_slice(&bytes).map_err(|_| Error)?;
    serve_bootstrap(VsockListener::bind(port, cid)?, boot)
}
fn main() {
    if run().is_err() {
        eprintln!("bootstrap unavailable");
        std::process::exit(1);
    }
}
