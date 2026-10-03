//! Measured application entrypoint. Only public manifest/ciphertext are files.
fn run() -> Result<(), cinder_service::Error> {
    use cinder_service::{
        Error,
        boot::{Bootstrap, Manifest, Role},
        nsm::Nsm,
        runtime::Configuration,
        transport::{Clock, Identity, Server, read_frame},
        vsock::{Target, VsockListener, VsockStream},
    };
    use std::{
        io::Read,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };
    let args: Vec<_> = std::env::args().skip(1).collect();
    let web = args.as_slice() == ["--web"];
    if !args.is_empty() && !web {
        return Err(Error);
    }
    let mut bytes = Vec::new();
    std::fs::File::open("/etc/cinder/manifest.cbor")?
        .take(131073)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 131072 {
        return Err(Error);
    }
    let manifest: Manifest = serde_cbor::from_slice(&bytes).map_err(|_| Error)?;
    manifest.validate()?;
    let nsm = Arc::new(Nsm::discover(
        manifest.domain.as_slice().try_into().map_err(|_| Error)?,
        manifest.digest()?,
    )?);
    let mut socket = VsockStream::connect(Target::new(3, manifest.bootstrap)?)?;
    let bytes = read_frame(&mut socket, 65536)?;
    let bootstrap: Bootstrap = serde_cbor::from_slice(&bytes).map_err(|_| Error)?;
    let parent = cinder_service::cloud::Credential {
        access: bootstrap.parent.access.clone(),
        secret: bootstrap.parent.secret.clone(),
        token: bootstrap.parent.token.clone(),
        expires: bootstrap.parent.expires,
    };
    let mut keys = cinder_service::boot::release(&manifest, nsm.clone(), bootstrap)?;
    let bytes = keys.remove(&Role::Configuration).ok_or(Error)?;
    let config: Configuration = serde_cbor::from_slice(&bytes).map_err(|_| Error)?;
    drop(bytes);
    let stop = Arc::new(AtomicBool::new(false));
    let runtime = Arc::new(config.construct(keys)?.open(
        &manifest,
        parent,
        nsm.clone(),
        stop.clone(),
    )?);
    runtime.tick()?;
    enum Ingress {
        Tls(Server),
        Web(cinder_service::web::Server),
    }
    let server = if web {
        Ingress::Web(cinder_service::web::Server::new(
            nsm.policy(),
            nsm.clone(),
            nsm.clone(),
            nsm.clone(),
            runtime.clone(),
        )?)
    } else {
        Ingress::Tls(Server::new(
            Identity::generate(nsm.as_ref())?,
            nsm.policy(),
            nsm.clone(),
            nsm.clone(),
            runtime.clone(),
        )?)
    };
    let listener = VsockListener::bind(manifest.ingress, 3)?;
    // Joinable finite supervisor. No unbounded queue; one cut each second, at
    // most one escaped dispatch. Each cut rechecks independent witness + time.
    let worker_runtime = runtime.clone();
    let worker_stop = stop.clone();
    let worker = std::thread::spawn(move || {
        while !worker_stop.load(Ordering::SeqCst) {
            if worker_runtime.tick().is_err() {
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    });
    println!("cinder runtime ready"); // No identities, balances, roots or secrets.
    let result = match server {
        Ingress::Tls(s) => s.run_vsock(listener, stop.clone()),
        Ingress::Web(s) => s.run_vsock(listener, stop.clone()),
    };
    runtime.fence();
    let _ = worker.join();
    nsm.now()?;
    result
}
fn main() {
    if run().is_err() {
        eprintln!("cinder runtime fenced");
        std::process::exit(1);
    }
}
