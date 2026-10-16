//! Exact BULK codecs, not a qualified venue adapter or financial controller.
//! No I/O, secret key, signing, ledger posting, funding or readiness authority.
pub mod execution;
pub mod fills;
pub mod financial;
pub mod observation;
pub mod precision;
mod raw;
pub mod wire;

use cinder_kernel::identity::EventScope;
use sha2::{Digest, Sha256};

/// Secret-safe codec failure, never containing account/response/order data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Current authority, evidence, immutable profile or capability is not qualified.
    Unqualified,
    /// Durability/freshness failed; outcome may be unknown, never retry implicitly.
    Unavailable,
    /// Malformed, ambiguous or off-grid data.
    Invalid,
    /// A bounded representation or work limit was exceeded.
    Limit,
    /// Native semantics outside this explicitly supported subset.
    Unsupported,
    /// Equal execution identities describe different economic facts.
    Conflict,
}

/// Explicit native signing network, not a Solana RPC/cluster selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Network {
    /// Native mainnet signature domain; encoding grants no live permission.
    Mainnet = 1,
    /// Official public testnet signature domain.
    Testnet = 2,
    /// Separately configured native development domain.
    Devnet = 3,
}

/// Trusted request/source binding; never inferred from response fields.
#[derive(Clone, PartialEq, Eq)]
pub struct Scope {
    source: EventScope,
    network: Network,
    account: [u8; 32],
}
impl Scope {
    /// Bind exact Cinder source, native network and native pooled account.
    /// This validates shape only, not upstream authority or qualification.
    pub fn new(source: EventScope, network: Network, account: [u8; 32]) -> Result<Self, Error> {
        if account == [0; 32] {
            return Err(Error::Invalid);
        }
        Ok(Self {
            source,
            network,
            account,
        })
    }
    /// Commitment for local attribution; it is NOT appended to native signatures.
    pub fn commitment(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"CINDER-BULK-SCOPE-1\0");
        h.update(self.source.domain.network.bytes());
        h.update(self.source.domain.deployment.bytes());
        h.update(self.source.venue.bytes());
        h.update(self.source.account.bytes());
        h.update(self.source.namespace.bytes());
        h.update([self.network as u8]);
        h.update(self.account);
        h.finalize().into()
    }
    /// Original native pooled account, not a private customer account.
    pub fn account(&self) -> [u8; 32] {
        self.account
    }
}
impl std::fmt::Debug for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Scope([PRIVATE])")
    }
}

pub(crate) fn address(text: &str) -> Result<[u8; 32], Error> {
    if text.len() > 44 {
        return Err(Error::Invalid);
    }
    let mut bytes = [0; 32];
    if bs58::decode(text)
        .onto(&mut bytes)
        .map_err(|_| Error::Invalid)?
        != 32
        || bs58::encode(bytes).into_string() != text
    {
        return Err(Error::Invalid);
    }
    Ok(bytes)
}

pub(crate) fn symbol(text: &str) -> Result<(), Error> {
    if text.is_empty()
        || text.len() > 32
        || !text
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
    {
        Err(Error::Invalid)
    } else {
        Ok(())
    }
}
