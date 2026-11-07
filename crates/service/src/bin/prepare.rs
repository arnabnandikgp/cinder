//! Trusted-operator LOCAL preparation. Explicit stdin only, no AWS/wallet calls.
//! Its private output directory must NEVER be copied to the parent or image.
use cinder_service::{
    Error,
    boot::{Manifest, Role},
    cloud::Credential,
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
    #[serde(default)]
    broker: Option<Vec<u8>>,
    witness: Credential,
    #[serde(default)]
    funds: Option<Vec<u8>>,
    #[serde(default)]
    bulk_trading: Option<Vec<u8>>,
    #[serde(default)]
    pacifica_configuration: Option<cinder_service::units::deployment::Pacifica>,
    #[serde(default)]
    bulk_configuration: Option<cinder_service::units::deployment::Bulk>,
}
#[derive(Deserialize, serde::Serialize)]
#[serde(untagged)]
enum Configuration {
    Legacy(Box<cinder_service::runtime::Configuration>),
    Units(Box<cinder_service::units::deployment::Configuration>),
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
        (
            Role::Witness,
            Zeroizing::new(serde_cbor::to_vec(&input.witness).map_err(|_| Error)?),
        ),
    ]);
    if let Some(funds) = input.funds {
        keys.insert(Role::Funds, Zeroizing::new(funds));
    }
    if let Some(broker) = input.broker {
        keys.insert(Role::Broker, Zeroizing::new(broker));
    }
    if let Some(bulk) = input.bulk_trading {
        keys.insert(Role::BulkTrading, Zeroizing::new(bulk));
    }
    if let Some(p) = input.pacifica_configuration {
        keys.insert(
            Role::PacificaConfiguration,
            Zeroizing::new(serde_cbor::to_vec(&p).map_err(|_| Error)?),
        );
    }
    if let Some(b) = input.bulk_configuration {
        keys.insert(
            Role::BulkConfiguration,
            Zeroizing::new(serde_cbor::to_vec(&b).map_err(|_| Error)?),
        );
    }
    cinder_service::application::prepare(&mut manifest, &keys)?;
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
