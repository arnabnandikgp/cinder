//! Attestation-gated TLS transport and bounded opaque relay. Real NSM, key
//! release, trusted time and independent witness ports are qualified by P20.
pub mod attestation;
pub mod boot;
/// Streamed approved executable verification without unbounded RPC responses.
pub mod chain_code;
/// Journal-selected one-use chain funding and original-wire reconciliation.
pub mod chain_funding;
/// Exact finalized chain effects; inclusion/ACK/balance alone are insufficient.
pub mod chain_receipt;
/// Fixed authenticated Solana RPC inside the enclave; no ambient wallet/retry.
pub mod chain_rpc;
pub mod cloud;
/// Finalized owner-signed vault deposits into the one authoritative ledger.
pub mod customer_deposit;
pub mod egress;
#[cfg(feature = "local-fixture")]
pub mod fixture;
/// Bounded native transfer capture with TLS terminating inside the enclave.
pub mod native_capture;
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

#[cfg(test)]
#[path = "../../journal/tests/support/mod.rs"]
mod test_support;

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
