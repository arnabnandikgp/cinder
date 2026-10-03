//! Attestation-gated TLS transport and bounded opaque relay. Real NSM, key
//! release, trusted time and independent witness ports are qualified by P20.
pub mod attestation;
pub mod boot;
pub mod cloud;
pub mod egress;
#[cfg(feature = "local-fixture")]
pub mod fixture;
pub mod nsm;
/// Sealed-journal recovery preparation and recipient-encrypted independent delivery.
pub mod recovery;
pub mod release;
pub mod runtime;
pub mod transport;
pub mod verifier;
pub mod vsock;
/// Attested Noise ingress into the same private financial handler as TLS.
pub mod web;

/// Deliberately redacted failures; neither OpenSSL nor application bodies escape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error;
impl From<openssl::error::ErrorStack> for Error {
    fn from(_: openssl::error::ErrorStack) -> Self {
        Self
    }
}
impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Self
    }
}
