//! Native vector-independent state-machine and hostile-wire qualification.
use cinder_web_channel_qualification::{Endpoint, MAX_PAYLOAD, MAX_RECORDS, PROFILE};
use snow::Builder;

const CONTEXT: [u8; 32] = [1; 32];

fn pending() -> (Endpoint, Endpoint) {
    let server = Endpoint::server(&CONTEXT).unwrap();
    let client = Endpoint::client(server.public_key(), &CONTEXT).unwrap();
    (client, server)
}
fn pair() -> (Endpoint, Endpoint) {
    let (mut c, mut s) = pending();
    let first = c.start().unwrap();
    assert_eq!(first.len(), 48);
    let second = s.advance(&first).unwrap();
    assert_eq!(second.len(), 48);
    let ready = c.advance(&second).unwrap();
    let ack = s.advance(&ready).unwrap();
    assert!(c.advance(&ack).unwrap().is_empty());
    assert!(c.ready() && s.ready());
    assert_eq!(c.binding().unwrap(), s.binding().unwrap());
    (c, s)
}
fn closed(c: &mut Endpoint) {
    assert!(!c.ready());
    assert!(c.binding().is_err());
    assert!(c.start().is_err());
    assert!(c.advance(&[1; 48]).is_err());
    assert!(c.seal(b"secret").is_err());
    assert!(c.open(&[1; 32]).is_err());
}

#[test]
fn no_early_private_data_or_retry() {
    let (mut c, mut s) = pending();
    assert!(c.seal(b"secret").is_err());
    closed(&mut c);
    assert!(s.open(&[1; 32]).is_err());
    closed(&mut s);
    let (mut c, mut s) = pending();
    let second = s.advance(&c.start().unwrap()).unwrap();
    let confirmation = c.advance(&second).unwrap();
    assert!(!c.ready() && !s.ready());
    assert!(c.binding().is_err());
    assert!(s.binding().is_err());
    assert!(s.seal(b"secret").is_err());
    assert!(s.advance(&confirmation).is_err());
    closed(&mut s);
    assert!(c.seal(b"secret").is_err());
    closed(&mut c);
}

#[test]
fn fresh_keys_and_session_transcripts() {
    let (_, s) = pending();
    let (_, other) = pending();
    assert_ne!(s.public_key(), other.public_key());
    let (c, _) = pair();
    let (other, _) = pair();
    assert_ne!(c.binding().unwrap(), other.binding().unwrap());
}

#[test]
fn key_and_domain_substitution_fail_closed() {
    for wrong_key in [false, true] {
        let mut s = Endpoint::server(&CONTEXT).unwrap();
        let other = Endpoint::server(&CONTEXT).unwrap();
        let key = if wrong_key {
            other.public_key()
        } else {
            s.public_key()
        };
        let context = if wrong_key { CONTEXT } else { [2; 32] };
        let mut c = Endpoint::client(key, &context).unwrap();
        assert!(s.advance(&c.start().unwrap()).is_err());
        closed(&mut s);
    }
    assert!(Endpoint::client(&[0; 32], &CONTEXT).is_err());
    assert!(Endpoint::client(&[1; 31], &CONTEXT).is_err());
    assert!(Endpoint::server(&[0; 32]).is_err());
    assert!(Endpoint::server(&[1; 33]).is_err());
}

#[test]
fn first_handshake_rejects_library_supported_early_payload() {
    let mut s = Endpoint::server(&CONTEXT).unwrap();
    let mut c = Builder::new(PROFILE.parse().unwrap())
        .prologue(&CONTEXT)
        .unwrap()
        .remote_public_key(s.public_key())
        .unwrap()
        .build_initiator()
        .unwrap();
    let mut wire = [0; 80];
    let n = c.write_message(b"early identity", &mut wire).unwrap();
    assert!(s.advance(&wire[..n]).is_err());
    closed(&mut s);
}

#[test]
fn second_handshake_rejects_early_payload_and_bad_confirmation() {
    for bad_confirmation in [false, true] {
        let builder = || {
            Builder::new(PROFILE.parse().unwrap())
                .prologue(&CONTEXT)
                .unwrap()
        };
        let key = builder().generate_keypair().unwrap();
        let mut s = builder()
            .local_private_key(&key.private)
            .unwrap()
            .build_responder()
            .unwrap();
        let mut c = Endpoint::client(&key.public, &CONTEXT).unwrap();
        s.read_message(&c.start().unwrap(), &mut []).unwrap();
        let mut wire = [0; 80];
        let body: &[u8] = if bad_confirmation {
            b""
        } else {
            b"early identity"
        };
        let n = s.write_message(body, &mut wire).unwrap();
        if bad_confirmation {
            c.advance(&wire[..n]).unwrap();
            let mut t = s.into_transport_mode().unwrap();
            let n = t.write_message(b"wrong confirmation", &mut wire).unwrap();
            assert!(c.advance(&wire[..n]).is_err());
        } else {
            assert!(c.advance(&wire[..n]).is_err());
        }
        closed(&mut c);
    }
}

#[test]
fn handshake_truncation_tampering_and_repetition() {
    for len in 0..48 {
        let (mut c, mut s) = pending();
        let wire = c.start().unwrap();
        assert!(s.advance(&wire[..len]).is_err());
        closed(&mut s);
    }
    for index in 0..48 {
        let (mut c, mut s) = pending();
        let mut wire = c.start().unwrap();
        wire[index] ^= 1;
        assert!(s.advance(&wire).is_err());
    }
    let (mut c, _) = pending();
    c.start().unwrap();
    assert!(c.start().is_err());
    closed(&mut c);
    let (mut c, mut s) = pair();
    assert!(s.advance(&c.seal(b"extra handshake").unwrap()).is_err());
    closed(&mut s);
}

#[test]
fn replay_reflection_reordering_and_other_session_fail_closed() {
    let (mut c, mut s) = pair();
    let wire = c.seal(b"private").unwrap();
    assert_eq!(s.open(&wire).unwrap().as_slice(), b"private");
    assert!(s.open(&wire).is_err());
    closed(&mut s);
    let (mut c, _) = pair();
    let wire = c.seal(b"private").unwrap();
    assert!(c.open(&wire).is_err());
    closed(&mut c);
    let (_, mut other) = pair();
    assert!(other.open(&wire).is_err());
    closed(&mut other);
    // Fresh same-session pair: distinguish ordering from another-session keys.
    let (mut c, mut s) = pair();
    c.seal(b"first").unwrap();
    assert!(s.open(&c.seal(b"second").unwrap()).is_err());
    closed(&mut s);
}

#[test]
fn record_truncation_and_tampering_never_release_partial_plaintext() {
    for len in 0..23 {
        let (mut c, mut s) = pair();
        let wire = c.seal(b"private").unwrap();
        assert!(s.open(&wire[..len]).is_err());
        closed(&mut s);
    }
    for index in 0..23 {
        let (mut c, mut s) = pair();
        let mut wire = c.seal(b"private").unwrap();
        wire[index] ^= 1;
        assert!(s.open(&wire).is_err());
        closed(&mut s);
    }
}

#[test]
fn payload_bounds_and_each_direction_record_limit() {
    let (mut c, mut s) = pair();
    let clear = vec![42; MAX_PAYLOAD];
    let wire = c.seal(&clear).unwrap();
    assert_eq!(wire.len(), MAX_PAYLOAD + 16);
    assert_eq!(s.open(&wire).unwrap().as_slice(), clear);
    for body in [vec![], vec![42; MAX_PAYLOAD + 1]] {
        let (mut c, _) = pair();
        assert!(c.seal(&body).is_err());
        closed(&mut c);
    }
    let (_, mut s) = pair();
    assert!(s.open(&vec![1; MAX_PAYLOAD + 17]).is_err());
    closed(&mut s);
    let (mut c, mut s) = pair();
    for _ in 1..MAX_RECORDS {
        let wire = c.seal(b"c").unwrap();
        assert_eq!(s.open(&wire).unwrap().as_slice(), b"c");
        let wire = s.seal(b"s").unwrap();
        assert_eq!(c.open(&wire).unwrap().as_slice(), b"s");
    }
    assert!(c.seal(b"excess").is_err());
    assert!(s.open(&[1; 17]).is_err());
    closed(&mut c);
    closed(&mut s);
}
