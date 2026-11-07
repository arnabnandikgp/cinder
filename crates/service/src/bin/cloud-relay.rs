//! Fixed opaque AWS route. Public manifest only; no credentials or TLS keys.
fn run() -> Result<(), cinder_service::Error> {
    use cinder_service::{
        Error,
        boot::Manifest,
        transport::{relay_cloud, stop_on_stdin},
        vsock::VsockListener,
    };
    use std::io::Read;
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err(Error);
    }
    let mut bytes = Vec::new();
    std::fs::File::open(&args[0])?
        .take(131073)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 131072 {
        return Err(Error);
    }
    let manifest: Manifest = serde_cbor::from_slice(&bytes).map_err(|_| Error)?;
    manifest.validate()?;
    let cid: u32 = args[1].parse().map_err(|_| Error)?;
    if cid <= 3 {
        return Err(Error);
    }
    let port = args[2].parse().map_err(|_| Error)?;
    let endpoint = [&manifest.first, &manifest.second, &manifest.witness]
        .into_iter()
        .chain(manifest.slots.iter().map(|s| &s.endpoint))
        .find(|e| e.port == port)
        .ok_or(Error)?;
    relay_cloud(VsockListener::bind(port, cid)?, endpoint, stop_on_stdin())
}
fn main() {
    if run().is_err() {
        eprintln!("cloud relay unavailable");
        std::process::exit(1);
    }
}
