//! Alternate-root verifier ONLY in an explicitly enabled disposable test build.
fn run() -> Result<(), cinder_service::Error> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 1 || args[0].len() > 2048 || args[0].len() % 2 != 0 || !args[0].is_ascii() {
        return Err(cinder_service::Error);
    }
    let b: Vec<u8> = args[0]
        .as_bytes()
        .chunks(2)
        .map(|s| {
            u8::from_str_radix(
                std::str::from_utf8(s).map_err(|_| cinder_service::Error)?,
                16,
            )
            .map_err(|_| cinder_service::Error)
        })
        .collect::<Result<_, _>>()?;
    let root = openssl::x509::X509::from_der(&b)?;
    cinder_service::verifier::run(|b, p, c| {
        cinder_service::attestation::verify_fixture(b, p, c, &root)
    })
}
fn main() {
    if run().is_err() {
        eprintln!("fixture attestation rejected");
        std::process::exit(1);
    }
}
