use crate::{Error, PROFILE};
use snow::{
    Builder,
    params::{CipherChoice, DHChoice, HashChoice},
    resolvers::{CryptoResolver, DefaultResolver},
    types::{Cipher, Dh, Hash, Random},
};
use std::sync::Arc;

/// Responders provide entropy inside the confidential runtime. An error must
/// fail closed; implementations may not substitute parent entropy or cached keys.
pub trait Entropy: Send + Sync {
    /// Fill the complete destination with fresh random bytes.
    fn fill(&self, output: &mut [u8]) -> Result<(), Error>;
}
struct Source(Arc<dyn Entropy>);
impl Random for Source {
    fn try_fill_bytes(&mut self, output: &mut [u8]) -> Result<(), snow::Error> {
        self.0.fill(output).map_err(|_| snow::Error::Rng)
    }
}
struct Resolver(Arc<dyn Entropy>);
impl CryptoResolver for Resolver {
    fn resolve_rng(&self) -> Option<Box<dyn Random>> {
        Some(Box::new(Source(self.0.clone())))
    }
    fn resolve_dh(&self, c: &DHChoice) -> Option<Box<dyn Dh>> {
        DefaultResolver.resolve_dh(c)
    }
    fn resolve_hash(&self, c: &HashChoice) -> Option<Box<dyn Hash>> {
        DefaultResolver.resolve_hash(c)
    }
    fn resolve_cipher(&self, c: &CipherChoice) -> Option<Box<dyn Cipher>> {
        DefaultResolver.resolve_cipher(c)
    }
}
pub(crate) fn builder(context: &[u8], source: Arc<dyn Entropy>) -> Result<Builder<'_>, Error> {
    if context.len() != 32 || context.iter().all(|b| *b == 0) {
        return Err(Error);
    }
    Builder::with_resolver(
        PROFILE.parse().map_err(|_| Error)?,
        Box::new(Resolver(source)),
    )
    .prologue(context)
    .map_err(|_| Error)
}
