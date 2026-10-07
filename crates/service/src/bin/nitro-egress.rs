//! Fixed-origin opaque parent egress. No signature, payload or key parsing.
fn run() -> Result<(), cinder_service::Error> {
    use cinder_pacifica::execution::Origin;
    use cinder_service::{
        Error,
        transport::{relay_chain, relay_egress, relay_native_capture, stop_on_stdin},
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
    let route = args[2].as_str();
    // No URL/path or arbitrary hostname argument. Credentials are never given
    // to this process and TLS authentication remains enclave-owned.
    let origin = match route {
        "pacifica-testnet" => Some((Origin::Testnet, false)),
        "pacifica-mainnet" => Some((Origin::Mainnet, false)),
        "pacifica-testnet-ws" => Some((Origin::Testnet, true)),
        "pacifica-mainnet-ws" => Some((Origin::Mainnet, true)),
        "solana-devnet" | "helius-devnet" => None,
        _ => return Err(Error),
    };
    let listener = VsockListener::bind(port, cid)?;
    match origin {
        Some((origin, false)) => relay_egress(listener, origin, stop_on_stdin()),
        Some((origin, true)) => relay_native_capture(listener, origin, stop_on_stdin()),
        None => relay_chain(
            listener,
            if route == "helius-devnet" {
                "devnet.helius-rpc.com"
            } else {
                "api.devnet.solana.com"
            },
            stop_on_stdin(),
        ),
    }
}
fn main() {
    if run().is_err() {
        eprintln!("nitro egress unavailable");
        std::process::exit(1);
    }
}
