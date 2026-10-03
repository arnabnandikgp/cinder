//! Explicitly enabled offline fixture entrypoint, never a production fallback.
fn run() -> Result<(), cinder_service::Error> {
    use cinder_service::{Error, fixture::*, transport::*};
    use std::{
        io::{Read, Write},
        net::{SocketAddr, TcpListener},
        path::Path,
        sync::Arc,
    };
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() == 2 && args[1] == "--recovery-worker" {
        return recovery_worker(Path::new(&args[0]));
    }
    let web = args.len() == 4 && args[3] == "--web";
    let recovery_web = args.len() == 4 && args[3] == "--recovery-web";
    if args.len() != 3 && !web && !recovery_web {
        return Err(Error);
    }
    let listen: SocketAddr = args[0].parse().map_err(|_| Error)?;
    if !listen.ip().is_loopback() {
        return Err(Error);
    }
    let wallet = decode_hex(&args[2])?.try_into().map_err(|_| Error)?;
    let mut key = zeroize::Zeroizing::new([0; 32]);
    std::io::stdin().read_exact(&mut *key)?;
    let handler = if recovery_web {
        let binding = read_binding()?;
        let c = recovery::controller(&binding)?;
        FixtureHandler::open_recovery(Path::new(&args[1]), key, wallet, &c, &binding.keys()?)?
    } else {
        FixtureHandler::open(Path::new(&args[1]), key, wallet)?
    };
    let attester = FixtureAttester::new()?;
    let root = attester.root().to_der()?;
    let attester = Arc::new(attester);
    let handler = Arc::new(handler);
    let listener = TcpListener::bind(listen)?;
    println!(
        "{} {}",
        listener.local_addr()?,
        root.iter().map(|b| format!("{b:02x}")).collect::<String>()
    );
    std::io::stdout().flush()?;
    if web || recovery_web {
        cinder_service::web::Server::new(
            policy(),
            Arc::new(FixtureClock),
            attester,
            Arc::new(FixtureClock),
            handler,
        )?
        .run(listener, stop_on_stdin())
    } else {
        Server::new(
            Identity::generate(&FixtureClock)?,
            policy(),
            attester,
            Arc::new(FixtureClock),
            handler,
        )?
        .run(listener, stop_on_stdin())
    }
}
fn read_binding() -> Result<cinder_service::fixture::recovery::Binding, cinder_service::Error> {
    use std::io::Read;
    let mut size = [0; 4];
    std::io::stdin().read_exact(&mut size)?;
    let n = u32::from_be_bytes(size) as usize;
    if n == 0 || n > 65536 {
        return Err(cinder_service::Error);
    }
    let mut b = zeroize::Zeroizing::new(vec![0; n]);
    std::io::stdin().read_exact(&mut b)?;
    serde_json::from_slice(&b).map_err(|_| cinder_service::Error)
}
fn recovery_worker(root: &std::path::Path) -> Result<(), cinder_service::Error> {
    use cinder_service::{Error, fixture::recovery::*};
    use std::io::{BufRead, Read, Write};
    let mut key = zeroize::Zeroizing::new([0; 32]);
    std::io::stdin().read_exact(&mut *key)?;
    let binding = read_binding()?;
    let c = controller(&binding)?;
    let mut worker = Worker::open(root, key, c, binding.account.clone())?;
    let mut input = std::io::BufReader::new(std::io::stdin());
    loop {
        let mut line = String::new();
        if input.by_ref().take(131_073).read_line(&mut line)? == 0 {
            return Ok(());
        }
        if line.len() > 131_072 {
            return Err(Error);
        }
        let v: serde_json::Value = serde_json::from_str(&line).map_err(|_| Error)?;
        let result = match v.get("op").and_then(|x| x.as_str()) {
            Some("payout") => serde_json::json!({"contract":worker.payout()?}),
            Some("return") => serde_json::json!({"contract":worker.return_assets()?}),
            Some("settle") => {
                let wire = serde_json::from_value(v["wire"].clone()).map_err(|_| Error)?;
                let signature: Vec<u8> =
                    serde_json::from_value(v["signature"].clone()).map_err(|_| Error)?;
                worker.settle(
                    wire,
                    signature.try_into().map_err(|_| Error)?,
                    v["slot"].as_u64().ok_or(Error)?,
                    v["paid"].as_u64().ok_or(Error)?,
                    v["sequence"].as_u64().ok_or(Error)?,
                )?;
                serde_json::json!({"settled":true})
            }
            Some("finish") => {
                let custody = serde_json::from_value(v["custody"].clone()).map_err(|_| Error)?;
                let owner = serde_json::from_value(v["owner"].clone()).map_err(|_| Error)?;
                let spki = serde_json::from_value(v["spki"].clone()).map_err(|_| Error)?;
                let signature: Vec<u8> =
                    serde_json::from_value(v["signature"].clone()).map_err(|_| Error)?;
                let p = worker.finish(
                    &custody,
                    &[cinder_service::recovery::RecipientKey {
                        owner,
                        spki,
                        signature: signature.try_into().map_err(|_| Error)?,
                    }],
                )?;
                serde_json::json!({"manifest":p.manifest().encode()?,"locator":p.locators()[0].1})
            }
            _ => return Err(Error),
        };
        println!("{}", serde_json::to_string(&result).map_err(|_| Error)?);
        std::io::stdout().flush()?;
    }
}
fn decode_hex(s: &str) -> Result<Vec<u8>, cinder_service::Error> {
    if s.len() != 64 || !s.is_ascii() {
        return Err(cinder_service::Error);
    }
    s.as_bytes()
        .chunks(2)
        .map(|b| {
            u8::from_str_radix(
                std::str::from_utf8(b).map_err(|_| cinder_service::Error)?,
                16,
            )
            .map_err(|_| cinder_service::Error)
        })
        .collect()
}
fn main() {
    if run().is_err() {
        eprintln!("fixture service unavailable");
        std::process::exit(1);
    }
}
