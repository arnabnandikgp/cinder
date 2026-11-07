//! Offline native capture qualification. Synthetic roots and public fixture
//! accounts only; not a native venue, source-cut or Nitro hardware receipt.
use super::*;
use crate::test_support as support;
use base64::{Engine, engine::general_purpose::STANDARD};
use cinder_journal::model::CommitId;
use cinder_pacifica::{
    capture::{self, Archive, Limits, Plan},
    execution::{Gateway, Policy},
    profile::{Grid, Level, Mapping, Profile},
    reads::MIN_READ_COST,
};
use openssl::{
    asn1::Asn1Time,
    ec::{EcGroup, EcKey},
    hash::{MessageDigest, hash},
    nid::Nid,
    pkey::{PKey, Private},
    ssl::{SslAcceptor, SslMethod},
    x509::{
        X509, X509NameBuilder,
        extension::{BasicConstraints, KeyUsage, SubjectAlternativeName},
    },
};
use serde_json::Value;
use std::{
    net::{TcpListener, TcpStream},
    sync::atomic::{AtomicU64, Ordering},
    thread,
};
use zeroize::Zeroizing;

const NOW: u64 = 1_700_000_500_000;
struct Time(AtomicU64);
impl Clock for Time {
    fn now(&self) -> Result<u64, Error> {
        let n = self.0.load(Ordering::SeqCst);
        if n == 0 { Err(Error) } else { Ok(n) }
    }
}
fn id(n: u8) -> CommitId {
    CommitId::new([n; 32]).unwrap()
}
fn limits() -> Limits {
    Limits {
        maximum_ms: 1500,
        maximum_messages: 16,
        maximum_message: 1024,
        maximum_bytes: 4096,
    }
}
fn gateway() -> Gateway {
    let config = support::config();
    Gateway::new(
        Profile {
            source: config.sources[0].scope,
            config,
            account: "J2xccRtuG43drESLYznHhLhQkLTdfepcKYbiQ9BsJVaf".into(),
            environment: Origin::Testnet.url().into(),
            revision: 1,
            evidence: "synthetic native capture fixture".into(),
            precision: Level::Qualified,
            fills: Level::Qualified,
            quote_places: 0,
            perp_tag: 0,
            markets: vec![Mapping {
                symbol: "BTC".into(),
                market: 0,
                size: Grid { places: 0, step: 1 },
                price: Grid { places: 0, step: 1 },
            }],
        },
        Policy {
            revision: 1,
            evidence: "synthetic fixture only".into(),
            execution: Level::Qualified,
            origin: Origin::Testnet,
            expiry_ms: 3000,
            credits: 600,
            cleanup_reserve: 120,
            read_cost: MIN_READ_COST,
        },
        Zeroizing::new([7; 32]),
        1,
    )
    .unwrap()
}
fn prepared(temp: &support::Temp, limits: Limits) -> (support::Store, Gateway, Query, Archive) {
    let mut j = temp.create();
    let g = gateway();
    g.activate(&mut j, id(250), NOW).unwrap();
    let (request, archive) = capture::prepare(
        &mut j,
        &g,
        Plan {
            reservation: id(1),
            capture: id(2),
            at: NOW,
            limits,
        },
    )
    .unwrap();
    (j, g, request.consume(NOW).unwrap(), archive)
}
fn records(j: &support::Store) -> Vec<Value> {
    j.transactions()
        .flat_map(|t| &t.inputs)
        .filter_map(|i| serde_json::from_slice::<Value>(i.raw.as_bytes()).ok())
        .filter(|v| v["schema"] == "cinder-native-capture-record-v1")
        .collect()
}
fn certificate(key: &PKey<Private>, issuer: Option<(&X509, &PKey<Private>)>, host: &str) -> X509 {
    let mut name = X509NameBuilder::new().unwrap();
    name.append_entry_by_text("CN", host).unwrap();
    let name = name.build();
    let mut b = X509::builder().unwrap();
    b.set_version(2).unwrap();
    b.set_serial_number(
        openssl::bn::BigNum::from_u32(if issuer.is_some() { 2 } else { 1 })
            .unwrap()
            .to_asn1_integer()
            .unwrap()
            .as_ref(),
    )
    .unwrap();
    b.set_subject_name(&name).unwrap();
    b.set_issuer_name(issuer.map_or(name.as_ref(), |(c, _)| c.subject_name()))
        .unwrap();
    b.set_pubkey(key).unwrap();
    b.set_not_before(Asn1Time::from_unix(1_700_000_000).unwrap().as_ref())
        .unwrap();
    b.set_not_after(Asn1Time::from_unix(1_700_001_000).unwrap().as_ref())
        .unwrap();
    let mut constraints = BasicConstraints::new();
    constraints.critical();
    let mut usage = KeyUsage::new();
    usage.critical();
    if issuer.is_none() {
        constraints.ca().pathlen(0);
        usage.key_cert_sign();
    } else {
        usage.digital_signature();
    }
    b.append_extension(constraints.build().unwrap()).unwrap();
    b.append_extension(usage.build().unwrap()).unwrap();
    if let Some((c, _)) = issuer {
        b.append_extension(
            SubjectAlternativeName::new()
                .dns(host)
                .build(&b.x509v3_context(Some(c), None))
                .unwrap(),
        )
        .unwrap();
    }
    b.sign(issuer.map_or(key, |(_, k)| k), MessageDigest::sha256())
        .unwrap();
    b.build()
}
fn certificates(host: &str) -> (SslAcceptor, Trust) {
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
    let ca = PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap();
    let root = certificate(&ca, None, "fixture-root");
    let key = PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap();
    let leaf = certificate(&key, Some((&root, &ca)), host);
    let mut b = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls_server()).unwrap();
    b.set_min_proto_version(Some(openssl::ssl::SslVersion::TLS1_3))
        .unwrap();
    b.set_max_proto_version(Some(openssl::ssl::SslVersion::TLS1_3))
        .unwrap();
    b.set_certificate(&leaf).unwrap();
    b.set_private_key(&key).unwrap();
    let der = root.to_der().unwrap();
    (b.build(), Trust::from_der(&der, sha256(&der)).unwrap())
}
#[test]
fn loaded_capture_derives_actual_origin_route_root_and_limits_not_an_echoed_digest() {
    assert_eq!(
        connect_budget(1000, Duration::from_millis(999)),
        Some(Duration::from_millis(1))
    );
    assert_eq!(connect_budget(1000, Duration::from_millis(1000)), None);
    assert_eq!(connect_budget(1000, Duration::from_millis(1001)), None);
    assert_eq!(
        connect_budget(30000, Duration::ZERO),
        Some(Duration::from_secs(5))
    );
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
    let ca = PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap();
    let root = certificate(&ca, None, "fixture-root").to_der().unwrap();
    let policy = super::Policy {
        port: 9008,
        root_hash: sha256(&root),
        root,
        limits: limits(),
    };
    let loaded = Loaded::new(Origin::Testnet, &policy).unwrap();
    let digest = loaded.commitment().unwrap();
    assert!(loaded.matches(&policy));
    for case in 0..6 {
        let mut changed = policy.clone();
        match case {
            0 => changed.port += 1,
            1 => changed.limits.maximum_ms -= 1,
            2 => changed.limits.maximum_messages -= 1,
            3 => changed.limits.maximum_bytes -= 1,
            4 => changed.limits.maximum_message -= 1,
            _ => {
                let ca = PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap();
                changed.root = certificate(&ca, None, "different-fixture-root")
                    .to_der()
                    .unwrap();
                changed.root_hash = sha256(&changed.root);
            }
        }
        assert!(!loaded.matches(&changed));
        assert_ne!(
            Loaded::new(Origin::Testnet, &changed)
                .unwrap()
                .commitment()
                .unwrap(),
            digest
        );
    }
    assert_ne!(
        Loaded::new(Origin::Mainnet, &policy)
            .unwrap()
            .commitment()
            .unwrap(),
        digest
    );
    let (port, bounds) = loaded.open(Arc::new(Time(AtomicU64::new(NOW)))).unwrap();
    assert_eq!(
        port.commitment(),
        route_commitment(
            Origin::Testnet,
            Target::new(3, policy.port).unwrap(),
            &Trust::from_der(&policy.root, policy.root_hash).unwrap()
        )
    );
    assert!(bounds == policy.limits);
    let mut changed = policy.clone();
    changed.root_hash = [1; 32];
    assert!(Loaded::new(Origin::Testnet, &changed).is_err());
    let mut encoded = serde_json::to_value(&policy).unwrap();
    encoded["host"] = serde_json::json!("arbitrary.example");
    assert!(serde_json::from_value::<super::Policy>(encoded).is_err());
}
fn peer(
    acceptor: SslAcceptor,
    work: impl FnOnce(&mut openssl::ssl::SslStream<TcpStream>) + Send + 'static,
) -> (TcpStream, thread::JoinHandle<()>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = l.local_addr().unwrap();
    let worker = thread::spawn(move || {
        let (s, _) = l.accept().unwrap();
        s.prepare().unwrap();
        if let Ok(mut tls) = acceptor.accept(s) {
            work(&mut tls);
        }
    });
    (TcpStream::connect(address).unwrap(), worker)
}
/// Independent RFC6455 peer: no tungstenite server or shared client parser.
fn upgrade(s: &mut (impl Read + Write), extra: &str, good_key: bool) {
    let mut headers = vec![];
    while !headers.ends_with(b"\r\n\r\n") {
        let mut b = [0];
        s.read_exact(&mut b).unwrap();
        headers.push(b[0]);
        assert!(headers.len() < 8192);
    }
    let text = String::from_utf8(headers).unwrap();
    assert!(text.starts_with("GET /ws HTTP/1.1\r\n"));
    assert!(text.contains("Host: test-ws.pacifica.fi\r\n"));
    let key = text
        .lines()
        .find_map(|l| {
            l.split_once(": ")
                .filter(|(k, _)| k.eq_ignore_ascii_case("Sec-WebSocket-Key"))
                .map(|(_, v)| v)
        })
        .unwrap();
    let joined = format!("{key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    let accept = if good_key {
        STANDARD.encode(hash(MessageDigest::sha1(), joined.as_bytes()).unwrap())
    } else {
        "bad-key".into()
    };
    write!(s,"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n{extra}\r\n").unwrap();
    s.flush().unwrap();
}
fn client_frame(s: &mut impl Read) -> (u8, Vec<u8>) {
    let mut h = [0; 2];
    s.read_exact(&mut h).unwrap();
    assert_eq!(h[0] & 0x80, 0x80);
    assert_eq!(h[1] & 0x80, 0x80, "client MUST mask");
    let n = match h[1] & 127 {
        126 => {
            let mut x = [0; 2];
            s.read_exact(&mut x).unwrap();
            u16::from_be_bytes(x) as usize
        }
        n @ 0..=125 => n as usize,
        _ => panic!("oversized client frame"),
    };
    let mut mask = [0; 4];
    s.read_exact(&mut mask).unwrap();
    let mut bytes = vec![0; n];
    s.read_exact(&mut bytes).unwrap();
    for (i, b) in bytes.iter_mut().enumerate() {
        *b ^= mask[i % 4];
    }
    (h[0] & 15, bytes)
}
fn frame(s: &mut impl Write, opcode: u8, final_frame: bool, body: &[u8]) {
    s.write_all(&[opcode | if final_frame { 128 } else { 0 }])
        .unwrap();
    if body.len() < 126 {
        s.write_all(&[body.len() as u8]).unwrap();
    } else {
        s.write_all(&[126]).unwrap();
        s.write_all(&(body.len() as u16).to_be_bytes()).unwrap();
    }
    s.write_all(body).unwrap();
    s.flush().unwrap();
}
fn sink<'a>(
    j: &'a mut support::Store,
    g: &'a Gateway,
    a: &'a mut Archive,
) -> impl FnMut(Record) -> Result<(), Error> + 'a {
    let mut n = 10u8;
    move |r| {
        let id = id(n);
        n += 1;
        a.append(j, g, id, r).map(|_| ()).map_err(|_| Error)
    }
}

#[test]
fn budget_is_durable_single_use_and_limits_are_validated_before_spend() {
    let temp = support::Temp::new();
    let (mut j, g, _, _) = prepared(&temp, limits());
    let head = j.head();
    for l in [
        Limits {
            maximum_ms: 30_001,
            ..limits()
        },
        Limits {
            maximum_messages: 257,
            ..limits()
        },
        Limits {
            maximum_message: 16385,
            ..limits()
        },
        Limits {
            maximum_bytes: 1,
            ..limits()
        },
    ] {
        assert!(
            capture::prepare(
                &mut j,
                &g,
                Plan {
                    reservation: id(30),
                    capture: id(31),
                    at: NOW,
                    limits: l
                }
            )
            .is_err()
        );
        assert_eq!(j.head(), head);
    }
    assert!(
        capture::prepare(
            &mut j,
            &g,
            Plan {
                reservation: id(1),
                capture: id(32),
                at: NOW,
                limits: limits()
            }
        )
        .is_err()
    );
    let (request, _) = capture::prepare(
        &mut j,
        &g,
        Plan {
            reservation: id(33),
            capture: id(34),
            at: NOW,
            limits: limits(),
        },
    )
    .unwrap();
    assert!(request.consume(NOW + 3000).is_err());
    drop(j);
    let j = temp.open();
    assert!(j.transaction(id(1)).is_some());
    assert!(j.transaction(id(33)).is_some());
}
#[test]
fn raw_adverse_duplicate_and_unknown_bodies_survive_restart_without_economic_postings() {
    let temp = support::Temp::new();
    let (mut j, g, mut q, mut a) = prepared(&temp, limits());
    let cash = j.state().unwrap().ledger().clone();
    let readiness = j.state().unwrap().native_funding_ready();
    for (n, kind, body) in [
        (10, Kind::Opened, b"".as_slice()),
        (11, Kind::Text, b"not-json"),
        (12, Kind::Text, b"not-json"),
        (13, Kind::Binary, b"CINDER-GATEWAY-1\0private"),
        (14, Kind::Closed, b""),
    ] {
        let r = q.record(kind, body.to_vec(), NOW + 1).unwrap();
        assert!(!format!("{r:?}").contains("private"));
        a.append(&mut j, &g, id(n), r).unwrap();
    }
    assert_eq!(j.state().unwrap().ledger(), &cash);
    assert_eq!(j.state().unwrap().native_funding_ready(), readiness);
    drop(j);
    let j = temp.open();
    let rs = records(&j);
    assert_eq!(rs.len(), 5);
    assert_eq!(rs[1]["body"], rs[2]["body"]);
    for input in j.transactions().flat_map(|t| &t.inputs).filter(|i| {
        serde_json::from_slice::<Value>(i.raw.as_bytes())
            .is_ok_and(|v| v["schema"] == "cinder-native-capture-record-v1")
    }) {
        assert!(input.event.is_none());
        assert!(input.source_cut.is_none());
        assert_eq!(input.authority_epoch, 0);
    }
}
#[test]
fn journal_rejoin_accepts_an_advanced_head_but_rejects_cross_capture_and_bad_ordering() {
    let temp = support::Temp::new();
    let (mut j, g, mut q, mut a) = prepared(&temp, limits());
    let (r2, mut a2) = capture::prepare(
        &mut j,
        &g,
        Plan {
            reservation: id(30),
            capture: id(31),
            at: NOW,
            limits: limits(),
        },
    )
    .unwrap();
    let mut q2 = r2.consume(NOW).unwrap();
    assert!(
        a.append(
            &mut j,
            &g,
            id(11),
            q2.record(Kind::Opened, vec![], NOW).unwrap()
        )
        .is_err()
    );
    a.append(
        &mut j,
        &g,
        id(10),
        q.record(Kind::Opened, vec![], NOW).unwrap(),
    )
    .unwrap();
    assert!(q.record(Kind::Text, vec![0], NOW - 1).is_err());
    assert!(
        a2.append(
            &mut j,
            &g,
            id(10),
            q2.record(Kind::Text, vec![1], NOW).unwrap()
        )
        .is_err()
    );
    a.append(
        &mut j,
        &g,
        id(12),
        q.record(Kind::Interrupted, vec![], NOW).unwrap(),
    )
    .unwrap();
    assert!(q.record(Kind::Text, vec![1], NOW).is_err());
    for kind in [
        Kind::Text,
        Kind::Binary,
        Kind::Ping,
        Kind::Pong,
        Kind::Closed,
    ] {
        let temp = support::Temp::new();
        let (mut j, g, mut q, mut a) = prepared(&temp, limits());
        let ledger = j.state().unwrap().ledger().clone();
        assert!(q.record_unverified(Kind::Opened, vec![]).is_err());
        a.append(
            &mut j,
            &g,
            id(10),
            q.record(Kind::Opened, vec![], NOW).unwrap(),
        )
        .unwrap();
        assert!(
            q.record_unverified(kind, vec![0; limits().maximum_message + 1])
                .is_err()
        );
        let record = q.record_unverified(kind, vec![1, 2, 3]).unwrap();
        a.append(&mut j, &g, id(11), record).unwrap();
        assert!(q.record(Kind::Text, vec![], NOW).is_err());
        assert!(q.record_unverified(kind, vec![]).is_err());
        assert_eq!(records(&j)[1]["clock_sample"], "unverified_after_read");
        assert_eq!(records(&j)[1]["body"], serde_json::json!([1, 2, 3]));
        assert_eq!(j.state().unwrap().ledger(), &ledger);
    }
}
#[test]
fn actual_tls_peer_checks_exact_subscribe_masking_fragmentation_controls_and_raw_binary() {
    let temp = support::Temp::new();
    let (mut j, g, q, mut a) = prepared(&temp, limits());
    let (acceptor, trust) = certificates(host(Origin::Testnet));
    let (socket, worker) = peer(acceptor, |s| {
        upgrade(s, "", true);
        let (op, b) = client_frame(s);
        assert_eq!(op, 1);
        let v: Value = serde_json::from_slice(&b).unwrap();
        assert_eq!(v["method"], "subscribe");
        assert_eq!(v["params"]["source"], "account_transfers");
        assert_eq!(
            v["params"]["account"],
            "J2xccRtuG43drESLYznHhLhQkLTdfepcKYbiQ9BsJVaf"
        );
        frame(s, 1, false, b"{\"unknown\":");
        frame(s, 9, true, b"challenge");
        let (op, b) = client_frame(s);
        assert_eq!(op, 10);
        assert_eq!(b, b"challenge");
        frame(s, 0, true, b"42}");
        frame(s, 2, true, b"raw-binary");
        frame(s, 1, true, b"not-json");
        frame(s, 8, true, b"");
        let _ = client_frame(s);
    });
    run(
        socket,
        &trust,
        &Time(AtomicU64::new(NOW)),
        q,
        Instant::now(),
        &mut sink(&mut j, &g, &mut a),
    )
    .unwrap();
    worker.join().unwrap();
    let rs = records(&j);
    assert_eq!(
        rs.iter()
            .map(|v| v["kind"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["Opened", "Ping", "Text", "Binary", "Text", "Closed"]
    );
    assert_eq!(
        rs[2]["body"],
        serde_json::to_value(b"{\"unknown\":42}".as_slice()).unwrap()
    );
}
#[test]
fn wrong_tls_name_untrusted_root_and_expired_certificates_archive_only_an_uncertain_stop() {
    for case in 0..3 {
        let temp = support::Temp::new();
        let (mut j, g, q, mut a) = prepared(&temp, limits());
        let (acceptor, mut trust) = certificates(if case == 0 {
            "test-api.pacifica.fi"
        } else {
            host(Origin::Testnet)
        });
        if case == 1 {
            trust = certificates("other.invalid").1;
        }
        let (socket, worker) = peer(acceptor, |_| {});
        let now = if case == 2 { NOW + 1_500_000 } else { NOW };
        run(
            socket,
            &trust,
            &Time(AtomicU64::new(now)),
            q,
            Instant::now(),
            &mut sink(&mut j, &g, &mut a),
        )
        .unwrap();
        worker.join().unwrap();
        assert_eq!(records(&j).len(), 1);
        assert_eq!(records(&j)[0]["kind"], "Interrupted");
    }
}
#[test]
fn bad_accept_redirect_extensions_and_oversized_headers_never_subscribe_or_reconnect() {
    for case in 0..5 {
        let temp = support::Temp::new();
        let (mut j, g, q, mut a) = prepared(&temp, limits());
        let (acceptor, trust) = certificates(host(Origin::Testnet));
        let (socket, worker) = peer(acceptor, move |s| {
            if case == 1 {
                let mut b = [0; 4096];
                let _ = s.read(&mut b);
                let _=s.write_all(b"HTTP/1.1 302 Found\r\nLocation: wss://other.invalid/ws\r\nContent-Length: 0\r\n\r\n");
            } else if case == 4 {
                upgrade(
                    s,
                    &format!("X-Padding: {}\r\n", "x".repeat(MAX_HEADERS)),
                    true,
                );
            } else {
                upgrade(
                    s,
                    if case == 2 {
                        "Sec-WebSocket-Extensions: permessage-deflate\r\n"
                    } else {
                        ""
                    },
                    case != 0,
                );
                if case == 3 {
                    let _ = client_frame(s);
                    let _ = s.write_all(&[0x81, 126, 0x10, 0]);
                }
            }
        });
        run(
            socket,
            &trust,
            &Time(AtomicU64::new(NOW)),
            q,
            Instant::now(),
            &mut sink(&mut j, &g, &mut a),
        )
        .unwrap();
        worker.join().unwrap();
        let rs = records(&j);
        assert_eq!(rs.last().unwrap()["kind"], "Interrupted");
        assert_eq!(rs.len(), if case == 3 { 2 } else { 1 });
    }
    let mut metered = Metered {
        inner: std::io::Cursor::new(vec![1; MAX_HEADERS + 1]),
        remaining: MAX_HEADERS,
    };
    let mut out = vec![];
    assert!(metered.read_to_end(&mut out).is_err());
    assert_eq!(out.len(), MAX_HEADERS);
}
#[test]
fn malformed_frames_and_fragment_growth_cannot_escape_bounds_or_produce_success() {
    for wire in [
        vec![0x81, 0x80, 1, 2, 3, 4],            // server must not mask
        vec![0xc1, 0],                           // unnegotiated reserved bit
        vec![0x81, 1, 0xff],                     // invalid UTF-8
        vec![0x80, 0],                           // continuation without a first fragment
        vec![0x09, 0],                           // fragmented control
        vec![0x81, 127, 0, 0, 0, 0, 0, 1, 0, 0], // declared payload over limit
        {
            let mut v = vec![0x01, 126, 2, 88];
            v.extend(vec![b'x'; 600]);
            v.extend([0x80, 126, 2, 88]);
            v.extend(vec![b'y'; 600]);
            v
        }, // fragments total >1024
    ] {
        let temp = support::Temp::new();
        let (mut j, g, q, mut a) = prepared(&temp, limits());
        let (acceptor, trust) = certificates(host(Origin::Testnet));
        let (socket, worker) = peer(acceptor, move |s| {
            upgrade(s, "", true);
            let _ = client_frame(s);
            let _ = s.write_all(&wire);
        });
        run(
            socket,
            &trust,
            &Time(AtomicU64::new(NOW)),
            q,
            Instant::now(),
            &mut sink(&mut j, &g, &mut a),
        )
        .unwrap();
        worker.join().unwrap();
        let rs = records(&j);
        assert_eq!(rs.len(), 2);
        assert_eq!(rs[0]["kind"], "Opened");
        assert_eq!(rs[1]["kind"], "Interrupted");
    }
}
#[test]
fn resource_clock_and_sink_failures_are_bounded_and_never_relabelled_success() {
    for case in 0..5 {
        let temp = support::Temp::new();
        let l = Limits {
            maximum_ms: if case == 0 { 400 } else { 1500 },
            maximum_messages: if case == 1 { 1 } else { 16 },
            ..limits()
        };
        let (mut j, g, q, mut a) = prepared(&temp, l);
        let (acceptor, trust) = certificates(host(Origin::Testnet));
        let time = Arc::new(Time(AtomicU64::new(NOW)));
        let (socket, worker) = peer(acceptor, move |s| {
            upgrade(s, "", true);
            let _ = client_frame(s);
            if case == 0 {
                let mut x = [0];
                let _ = s.read(&mut x);
            } else {
                frame(s, 1, true, b"private-frame");
                if case == 1 {
                    let _ = s.write_all(b"\x81\x01x");
                }
            }
        });
        let mut calls = 0;
        let mut normal = sink(&mut j, &g, &mut a);
        let result = run(socket, &trust, time.as_ref(), q, Instant::now(), &mut |r| {
            calls += 1;
            if case == 3 && calls == 2 {
                Err(Error)
            } else {
                let retained = normal(r);
                if calls == 1 && matches!(case, 2 | 4) {
                    time.0
                        .store(if case == 2 { NOW - 1 } else { 0 }, Ordering::SeqCst);
                }
                retained
            }
        });
        drop(normal);
        worker.join().unwrap();
        let rs = records(&j);
        if case == 3 {
            assert!(result.is_err());
            assert_eq!(calls, 2);
            assert_eq!(rs.len(), 1);
        } else if matches!(case, 2 | 4) {
            result.unwrap();
            assert_eq!(rs.len(), 2);
            assert_eq!(rs[1]["kind"], "Text");
            assert_eq!(rs[1]["body"], serde_json::json!(b"private-frame".to_vec()));
            assert_eq!(rs[1]["received_at"], NOW);
            assert_eq!(rs[1]["clock_sample"], "unverified_after_read");
            assert!(
                j.transactions()
                    .flat_map(|t| &t.inputs)
                    .all(|i| i.event.is_none() && i.source_cut.is_none())
            );
            let reopened = temp.open();
            assert_eq!(records(&reopened), rs);
        } else {
            result.unwrap();
            assert_eq!(
                rs.last().unwrap()["kind"],
                if case == 1 { "Limited" } else { "Interrupted" }
            );
        }
    }
}
#[test]
fn transport_has_no_host_override_and_library_plaintext_logging_is_compiled_out() {
    assert_eq!(host(Origin::Testnet), "test-ws.pacifica.fi");
    assert_eq!(host(Origin::Mainnet), "ws.pacifica.fi");
    assert_eq!(log::STATIC_MAX_LEVEL, log::LevelFilter::Off);
    let trust = certificates(host(Origin::Testnet)).1;
    assert!(
        Port::new(
            Origin::Testnet,
            Target::new(16, 5000).unwrap(),
            trust,
            Arc::new(Time(AtomicU64::new(NOW)))
        )
        .is_err()
    );
}
