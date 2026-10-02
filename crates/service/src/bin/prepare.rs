//! Trusted-operator LOCAL preparation. Explicit stdin only, no AWS/wallet calls.
//! Its private output directory must NEVER be copied to the parent or image.
use cinder_service::{
    Error,
    boot::{Manifest, Role},
    cloud::Credential,
    runtime::Configuration,
};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::Path,
};
use zeroize::Zeroizing;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Prepare {
    manifest: Manifest,
    configuration: Configuration,
    storage: Vec<u8>,
    trading: Vec<u8>,
    broker: Vec<u8>,
    witness: Credential,
}
fn write(root: &Path, name: &str, bytes: &[u8]) -> Result<(), Error> {
    #[cfg(unix)]
    use std::os::unix::fs::OpenOptionsExt;
    let mut options = std::fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(root.join(name))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
fn run() -> Result<(), Error> {
    #[cfg(unix)]
    use std::os::unix::fs::DirBuilderExt;
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 1 {
        return Err(Error);
    }
    let root = Path::new(&args[0]);
    if !root.is_absolute() || root.exists() {
        return Err(Error);
    }
    let mut bytes = Zeroizing::new(Vec::new());
    std::io::stdin().take(65537).read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        return Err(Error);
    }
    let input: Prepare = serde_cbor::from_slice(&bytes).map_err(|_| Error)?;
    let mut manifest = input.manifest;
    let configuration =
        Zeroizing::new(serde_cbor::to_vec(&input.configuration).map_err(|_| Error)?);
    let mut keys = BTreeMap::from([
        (Role::Configuration, configuration),
        (Role::Storage, Zeroizing::new(input.storage)),
        (Role::Trading, Zeroizing::new(input.trading)),
        (Role::Broker, Zeroizing::new(input.broker)),
        (
            Role::Witness,
            Zeroizing::new(serde_cbor::to_vec(&input.witness).map_err(|_| Error)?),
        ),
    ]);
    // Explicitly reject aliased/all-zero seed material in trusted preparation too.
    for role in [Role::Storage, Role::Trading, Role::Broker] {
        let key = keys.get(&role).ok_or(Error)?;
        if key.len() != 32 || key.as_slice() == [0; 32] {
            return Err(Error);
        }
    }
    for (i, role) in [Role::Storage, Role::Trading, Role::Broker]
        .iter()
        .enumerate()
    {
        for other in &[Role::Storage, Role::Trading, Role::Broker][..i] {
            if keys[role].as_slice() == keys[other].as_slice() {
                return Err(Error);
            }
        }
    }
    let c: Configuration = serde_cbor::from_slice(&keys.remove(&Role::Configuration).ok_or(Error)?)
        .map_err(|_| Error)?;
    manifest.application = c
        .construct(keys.iter().map(|(r, k)| (*r, k.clone())).collect())?
        .commitment();
    keys.insert(
        Role::Configuration,
        Zeroizing::new(serde_cbor::to_vec(&input.configuration).map_err(|_| Error)?),
    );
    for slot in &mut manifest.slots {
        slot.plaintext_hash = openssl::sha::sha256(keys.get(&slot.role).ok_or(Error)?);
    }
    manifest.validate()?;
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    builder.mode(0o700);
    builder.create(root)?;
    write(
        root,
        "manifest.cbor",
        &serde_cbor::to_vec(&manifest).map_err(|_| Error)?,
    )?;
    for (role, key) in keys {
        write(
            root,
            &format!("{}.plain", role.name()),
            &manifest.wrap(role, &key)?,
        )?;
        write(
            root,
            &format!("{}.context.json", role.name()),
            &serde_json::to_vec(&manifest.context(role)?).map_err(|_| Error)?,
        )?;
    }
    std::fs::File::open(root)?.sync_all()?;
    println!("release prepared; private role files must remain on the trusted operator machine");
    Ok(())
}
fn main() {
    if run().is_err() {
        eprintln!("release preparation refused");
        std::process::exit(1);
    }
}
