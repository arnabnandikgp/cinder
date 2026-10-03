//! Bounded authenticated assembly and explicit entropy failure regressions.
use cinder_web_channel::{Endpoint, Entropy, Error, records::*};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn pair() -> (Endpoint, Endpoint) {
    let mut server = Endpoint::server(&[5; 32]).unwrap();
    let mut client = Endpoint::client(server.public_key(), &[5; 32]).unwrap();
    let msg = client.start().unwrap();
    let msg = server.advance(&msg).unwrap();
    let msg = client.advance(&msg).unwrap();
    let msg = server.advance(&msg).unwrap();
    assert!(client.advance(&msg).unwrap().is_empty());
    (client, server)
}
#[test]
fn complete_reply_and_maximum_bound_round_trip() {
    for size in [1, 16370, 16371, MAX_RESPONSE] {
        let (mut c, mut s) = pair();
        let clear = request(1, &[7; 1024]).unwrap();
        let wire = c.seal(&clear).unwrap();
        let clear = s.open(&wire).unwrap();
        assert_eq!(read_request(1, &clear).unwrap(), &[7; 1024]);
        let reply = vec![33; size];
        let batch = response(&mut s, 1, &reply).unwrap();
        assert!(batch.len() <= MAX_BATCH);
        assert_eq!(read_response(&mut c, 1, &batch).unwrap().as_slice(), reply);
    }
}
#[test]
fn truncated_trailing_reordered_corrupt_and_wrong_correlation_fail_closed() {
    for mode in 0..6 {
        let (mut c, mut s) = pair();
        let mut batch = response(&mut s, 7, &[9; 32768]).unwrap();
        match mode {
            0 => {
                batch.pop();
            }
            1 => batch.push(0),
            2 => {
                let n = u32::from_be_bytes(batch[..4].try_into().unwrap()) as usize;
                batch.drain(..n + 4);
            }
            3 => batch[30] ^= 1,
            4 => {}
            _ => batch[..4].copy_from_slice(&u32::MAX.to_be_bytes()),
        }
        assert!(read_response(&mut c, if mode == 4 { 8 } else { 7 }, &batch).is_err());
        assert!(!c.ready());
        assert!(c.seal(&[1]).is_err());
    }
}
#[test]
fn authenticated_but_inconsistent_chunk_headers_do_not_release_a_prefix() {
    for offset in [1u32, 16370] {
        let (mut c, mut s) = pair();
        let clear = [
            &[1, 2][..],
            &1u32.to_be_bytes(),
            &10u32.to_be_bytes(),
            &offset.to_be_bytes(),
            &[7; 10],
        ]
        .concat();
        let wire = s.seal(&clear).unwrap();
        let batch = [&(wire.len() as u32).to_be_bytes()[..], &wire].concat();
        assert!(read_response(&mut c, 1, &batch).is_err());
        assert!(!c.ready());
    }
}
#[test]
fn limits_and_sequence_checked_before_application() {
    assert!(request(0, &[1]).is_err());
    assert!(request(1, &[]).is_err());
    assert!(request(1, &[1; 1025]).is_err());
    let clear = request(1, &[1]).unwrap();
    assert!(read_request(2, &clear).is_err());
    let (_c, mut s) = pair();
    assert!(response(&mut s, 1, &vec![1; MAX_RESPONSE + 1]).is_err());
}
#[test]
fn whole_response_record_budget_is_checked_before_emission() {
    let (mut c, mut s) = pair();
    let first = response(&mut s, 1, &vec![1; MAX_RESPONSE]).unwrap();
    assert_eq!(
        read_response(&mut c, 1, &first).unwrap().len(),
        MAX_RESPONSE
    );
    assert!(response(&mut s, 2, &vec![2; MAX_RESPONSE]).is_err());
    assert!(!s.ready());
}
struct Failing {
    calls: AtomicUsize,
    succeeds: usize,
}
impl Entropy for Failing {
    fn fill(&self, out: &mut [u8]) -> Result<(), Error> {
        if self.calls.fetch_add(1, Ordering::SeqCst) >= self.succeeds {
            return Err(Error);
        }
        out.fill(9);
        Ok(()) // Explicit deterministic test double, NEVER runtime.
    }
}
#[test]
fn explicit_entropy_failure_never_uses_platform_rng() {
    assert!(
        Endpoint::server_with_entropy(
            Arc::new(Failing {
                calls: AtomicUsize::new(0),
                succeeds: 0
            }),
            |_| Ok([5; 32])
        )
        .is_err()
    );
    let source = Arc::new(Failing {
        calls: AtomicUsize::new(0),
        succeeds: 1,
    });
    let mut s = Endpoint::server_with_entropy(source.clone(), |_| Ok([5; 32])).unwrap();
    let mut c = Endpoint::client(s.public_key(), &[5; 32]).unwrap();
    assert!(s.advance(&c.start().unwrap()).is_err());
    assert!(!s.ready());
    assert_eq!(source.calls.load(Ordering::SeqCst), 2);
}
