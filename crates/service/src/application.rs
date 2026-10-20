//! Shared production entrypoint/preparation selection. The measured manifest,
//! not CBOR guessing, host flags or available files, selects the private schema.
use crate::{
    Error,
    boot::{Manifest, Role},
    cloud::Credential,
    runtime,
    transport::{Clock, Handler},
    units::deployment,
};
use cinder_journal::encrypted::RecordCipher;
use std::{
    collections::BTreeMap,
    sync::{Arc, atomic::AtomicBool},
};
use zeroize::Zeroizing;

/// Actual validated application, before external storage access.
pub enum Loaded {
    /// Unchanged versions 1–6.
    Legacy(Box<runtime::Loaded>),
    /// Explicit new-history version 7.
    Units(Box<deployment::Loaded>),
}
/// Deserialize exactly the schema named by the measured version. No retry with
/// the other schema after a failure; all secrets stay enclave/trusted-operator side.
pub fn construct(
    manifest: &Manifest,
    mut keys: BTreeMap<Role, Zeroizing<Vec<u8>>>,
) -> Result<Loaded, Error> {
    manifest.validate()?;
    let bytes = keys.remove(&Role::Configuration).ok_or(Error)?;
    if manifest.version == 7 {
        let config: deployment::Configuration =
            serde_cbor::from_slice(&bytes).map_err(|_| Error)?;
        Ok(Loaded::Units(Box::new(
            config.construct_for(manifest, keys)?,
        )))
    } else {
        let config: runtime::Configuration = serde_cbor::from_slice(&bytes).map_err(|_| Error)?;
        Ok(Loaded::Legacy(Box::new(
            config.construct_for(manifest, keys)?,
        )))
    }
}
/// Trusted LOCAL preparation, also used by the operator binary. Pure validation
/// and derivation only: no files, KMS calls, network, wallets or journal writes.
/// Every plaintext is checked before the caller creates its private output.
pub fn prepare(
    manifest: &mut Manifest,
    keys: &BTreeMap<Role, Zeroizing<Vec<u8>>>,
) -> Result<(), Error> {
    if keys.len() != manifest.slots.len() {
        return Err(Error);
    }
    crate::boot::validate_keys(keys)?;
    if manifest.version == 7 {
        let p: deployment::Pacifica =
            serde_cbor::from_slice(keys.get(&Role::PacificaConfiguration).ok_or(Error)?)
                .map_err(|_| Error)?;
        let b: deployment::Bulk =
            serde_cbor::from_slice(keys.get(&Role::BulkConfiguration).ok_or(Error)?)
                .map_err(|_| Error)?;
        let registry = deployment::Configuration::registry(manifest, &p, &b)?;
        manifest.units.as_mut().ok_or(Error)?.registry = registry;
    }
    for slot in &mut manifest.slots {
        slot.plaintext_hash = openssl::sha::sha256(keys.get(&slot.role).ok_or(Error)?);
    }
    manifest.application = construct(manifest, keys.clone())?.commitment();
    manifest.validate()?;
    for (role, key) in keys {
        manifest.wrap(*role, key)?;
    }
    Ok(())
}
impl Loaded {
    /// Application component derived from actual controllers, not host metadata.
    pub fn commitment(&self) -> [u8; 32] {
        match self {
            Self::Legacy(v) => v.commitment(),
            Self::Units(v) => v.commitment(),
        }
    }
    /// Open the exact measured cloud backend before starting any listener.
    pub fn open(
        self,
        manifest: &Manifest,
        parent: Credential,
        clock: Arc<dyn Clock>,
        stop: Arc<AtomicBool>,
    ) -> Result<Instance, Error> {
        match self {
            Self::Legacy(v) => Ok(Instance::Legacy(Arc::new(
                v.open(manifest, parent, clock, stop)?,
            ))),
            Self::Units(v) => Ok(Instance::Units(Arc::new(
                v.open(manifest, parent, clock, stop)?,
            ))),
        }
    }
}
/// Same transport abstraction for both modes; not a second public API/server.
pub enum Instance {
    /// Existing single-unit deployment.
    Legacy(Arc<runtime::Runtime<runtime::Storage, RecordCipher>>),
    /// One shared unit journal and Handler.
    Units(Arc<deployment::Instance<runtime::Storage>>),
}
impl Instance {
    /// Supply the exact same handler to TLS or attested Noise.
    pub fn handler(&self) -> Arc<dyn Handler> {
        match self {
            Self::Legacy(v) => v.clone(),
            Self::Units(v) => v.handler(),
        }
    }
    /// Finite trusted supervisor; mode-specific measured gates remain in force.
    pub fn tick(&self) -> Result<(), Error> {
        match self {
            Self::Legacy(v) => v.tick(),
            Self::Units(v) => v.tick(),
        }
    }
    /// Invalidate all future private releases for this boot.
    pub fn fence(&self) {
        match self {
            Self::Legacy(v) => v.fence(),
            Self::Units(v) => v.fence(),
        }
    }
    /// Legacy-only qualified finite capture. Unit manifests refuse that policy.
    pub fn capture_once(&self) -> Result<(), Error> {
        match self {
            Self::Legacy(v) => v.capture_once(),
            Self::Units(_) => Err(Error),
        }
    }
}
