//! Public-data-only quote verifier for independently distributed Node clients.
fn main() {
    if std::env::args_os().len() != 1
        || cinder_service::verifier::run(cinder_service::attestation::verify).is_err()
    {
        eprintln!("attestation rejected");
        std::process::exit(1);
    }
}
