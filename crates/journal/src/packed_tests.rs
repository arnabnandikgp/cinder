use super::*;
use crate::model::PrivateBytes;
use cinder_kernel::identity::{DeploymentId, Domain, NetworkId};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct Objects {
    data: BTreeMap<[u8; 32], Vec<u8>>,
    gets: usize,
    puts: usize,
    fail_put: bool,
    fail_key: Option<[u8; 32]>,
    corrupt_put: bool,
    remove_on_put: Option<[u8; 32]>,
}
#[derive(Clone, Default)]
struct Memory(Arc<Mutex<Objects>>);
impl Store for Memory {
    fn identity(&self) -> [u8; 32] {
        Sha256::digest((Arc::as_ptr(&self.0) as usize).to_be_bytes()).into()
    }
    fn get(&mut self, key: [u8; 32]) -> Result<Vec<u8>, Error> {
        let mut s = self.0.lock().unwrap();
        s.gets += 1;
        s.data.get(&key).cloned().ok_or(Error::Storage)
    }
    fn put(&mut self, key: [u8; 32], bytes: &[u8]) -> Result<(), Error> {
        let mut s = self.0.lock().unwrap();
        s.puts += 1;
        if s.fail_put || s.fail_key == Some(key) {
            return Err(Error::Storage);
        }
        if let Some(old) = s.remove_on_put.take() {
            s.data.remove(&old);
        }
        let mut bytes = bytes.to_vec();
        if s.corrupt_put {
            bytes[0] ^= 1;
        }
        s.data.insert(key, bytes);
        Ok(())
    }
}
struct Authority {
    anchor: Anchor,
    reads: usize,
    cas: usize,
    lose_ack: bool,
    reject: bool,
    fail_read_after_cas: bool,
}
#[derive(Clone)]
struct Trust(Arc<Mutex<Authority>>);
impl Default for Trust {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Authority {
            anchor: Anchor {
                epoch: 1,
                head: None,
            },
            reads: 0,
            cas: 0,
            lose_ack: false,
            reject: false,
            fail_read_after_cas: false,
        })))
    }
}
impl Witness for Trust {
    fn read(&mut self, _: Stream) -> Result<Anchor, Error> {
        let mut s = self.0.lock().unwrap();
        s.reads += 1;
        if s.fail_read_after_cas && s.cas > 0 {
            return Err(Error::Storage);
        }
        Ok(s.anchor)
    }
    fn accept(&mut self, _: Stream, expected: Anchor, next: Head) -> Result<(), Error> {
        let mut s = self.0.lock().unwrap();
        if s.reject || s.anchor != expected {
            return Err(Error::Stale);
        }
        s.cas += 1;
        s.anchor.head = Some(next);
        if s.lose_ack {
            return Err(Error::Storage);
        }
        Ok(())
    }
}
fn stream() -> Stream {
    Stream {
        domain: Domain {
            network: NetworkId::new([1; 32]).unwrap(),
            deployment: DeploymentId::new([2; 32]).unwrap(),
        },
        id: [9; 32],
    }
}
fn frames(n: usize, size: usize) -> Vec<Frame> {
    let mut result = Vec::new();
    let mut previous = [0; 32];
    for sequence in 0..n {
        let frame = Frame::new(
            sequence as u64,
            previous,
            PrivateBytes::new(vec![42; size]).unwrap(),
        );
        previous = frame.head.hash;
        result.push(frame);
    }
    result
}
fn setup(
    n: usize,
) -> (
    Packed<Memory, Memory, Trust>,
    Memory,
    Memory,
    Trust,
    Vec<Frame>,
) {
    let (a, b, w) = (Memory::default(), Memory::default(), Trust::default());
    let mut backend = Packed::new(stream(), 1, a.clone(), b.clone(), w.clone()).unwrap();
    let history = frames(n + 1, 128);
    for (i, f) in history[..n].iter().enumerate() {
        backend
            .append(i.checked_sub(1).map(|i| history[i].head), f)
            .unwrap();
    }
    (backend, a, b, w, history)
}

#[test]
fn healthy_counts_scale_with_packs_and_preserve_every_original_frame() {
    for n in [1_usize, 9, 16, 17, 32, 33, 51, 65] {
        let (mut p, a, b, w, history) = setup(n);
        for store in [&a, &b] {
            let mut s = store.0.lock().unwrap();
            s.gets = 0;
            s.puts = 0;
        }
        {
            let mut s = w.0.lock().unwrap();
            s.reads = 0;
            s.cas = 0;
        }
        p.append(Some(history[n - 1].head), &history[n]).unwrap();
        let first = a.0.lock().unwrap();
        let second = b.0.lock().unwrap();
        assert_eq!(first.gets + second.gets, 3 * n.div_ceil(PACK_RECORDS) + 2);
        assert_eq!(first.puts + second.puts, 2);
        drop((first, second));
        // Four direct reads; a deployed witness may also read within its CAS.
        assert_eq!(w.0.lock().unwrap().reads, 4);
        assert_eq!(w.0.lock().unwrap().cas, 1);
        assert_eq!(p.load().unwrap(), history);
    }
}

#[test]
fn competing_same_frame_between_anchor_and_history_cannot_overwrite_accepted_tail() {
    struct BetweenReads {
        inner: Trust,
        prior: Anchor,
        first: bool,
    }
    impl Witness for BetweenReads {
        fn read(&mut self, stream: Stream) -> Result<Anchor, Error> {
            if std::mem::take(&mut self.first) {
                return Ok(self.prior);
            }
            self.inner.read(stream)
        }
        fn accept(&mut self, stream: Stream, expected: Anchor, next: Head) -> Result<(), Error> {
            self.inner.accept(stream, expected, next)
        }
    }
    for n in [15, 16, 50] {
        let (mut other, a, b, w, history) = setup(n);
        let expected = Some(history[n - 1].head);
        let prior = Anchor {
            epoch: 1,
            head: expected,
        };
        other.append(expected, &history[n]).unwrap();
        let first = a.0.lock().unwrap().data.clone();
        let second = b.0.lock().unwrap().data.clone();
        let writes = (a.0.lock().unwrap().puts, b.0.lock().unwrap().puts);
        // The losing writer observes the prior anchor, then the winner's fresh
        // accepted history. Both proposed the SAME original frame/hash.
        let witness = BetweenReads {
            inner: w.clone(),
            prior,
            first: true,
        };
        let mut loser = Packed::new(stream(), 1, a.clone(), b.clone(), witness).unwrap();
        assert_eq!(loser.append(expected, &history[n]), Err(Error::Stale));
        assert_eq!(writes, (a.0.lock().unwrap().puts, b.0.lock().unwrap().puts));
        assert_eq!(a.0.lock().unwrap().data, first);
        assert_eq!(b.0.lock().unwrap().data, second);
        assert_eq!(other.load().unwrap(), history);
    }
}

#[test]
fn canonical_codec_rejects_domains_partitions_order_hashes_lengths_and_trailing_bytes() {
    let history = frames(33, 32);
    for end in [0, 15, 16, 31, 32] {
        let start = end / PACK_RECORDS * PACK_RECORDS;
        let pack = Pack(history[start..=end].to_vec());
        let good = pack.bytes(stream()).unwrap();
        assert_eq!(
            Pack::decode(stream(), history[end].head, &good).unwrap().0,
            pack.0
        );
        for offset in [
            0,
            MAGIC.len(),
            MAGIC.len() + 32,
            MAGIC.len() + 64,
            MAGIC.len() + 96,
            MAGIC.len() + 100,
            good.len() - 1,
        ] {
            let mut bad = good.clone();
            bad[offset] ^= 1;
            assert!(Pack::decode(stream(), history[end].head, &bad).is_err());
        }
        let mut bad = good.clone();
        bad.push(0);
        assert!(Pack::decode(stream(), history[end].head, &bad).is_err());
        assert!(Pack::decode(stream(), history[end].head, &good[..good.len() - 1]).is_err());
        let mut different = stream();
        different.id[0] ^= 1;
        assert!(Pack::decode(different, history[end].head, &good).is_err());
        if end > 0 {
            assert!(Pack::decode(stream(), history[end - 1].head, &good).is_err());
        }
    }
    for invalid in [
        history[..14].to_vec(),
        history[1..16].to_vec(),
        {
            let mut v = history[..16].to_vec();
            v.swap(3, 4);
            v
        },
        {
            let mut v = history[..16].to_vec();
            v[4] = v[3].clone();
            v
        },
    ] {
        assert!(
            Pack::decode(
                stream(),
                history[15].head,
                &Pack(invalid).bytes(stream()).unwrap()
            )
            .is_err()
        );
    }
    assert!(Pack::decode(stream(), history[0].head, &vec![0; MAX_PACK_BYTES + 1]).is_err());
    let (a, b, w) = (Memory::default(), Memory::default(), Trust::default());
    assert!(Packed::new(stream(), 1, a.clone(), a.clone(), w.clone()).is_err());
    let mut p = Packed::new(stream(), 1, a, b, w.clone()).unwrap();
    w.0.lock().unwrap().anchor.head = Some(Head {
        sequence: MAX_RECORDS as u64,
        hash: [1; 32],
    });
    assert_eq!(p.load().unwrap_err(), Error::Limit);
    w.0.lock().unwrap().anchor.epoch = 2;
    assert_eq!(p.load().unwrap_err(), Error::Stale);
}

#[test]
fn repairs_each_historical_copy_including_partial_tail_and_loss_during_new_write() {
    for first in [true, false] {
        let (mut p, a, b, _, h) = setup(33);
        let damaged = if first { &a } else { &b };
        damaged.0.lock().unwrap().data.remove(&h[15].head.hash);
        damaged
            .0
            .lock()
            .unwrap()
            .data
            .insert(h[31].head.hash, vec![0]);
        // Loss occurs AFTER the history load, on the first new-tail PUT.
        damaged.0.lock().unwrap().remove_on_put = Some(h[32].head.hash);
        p.append(Some(h[32].head), &h[33]).unwrap();
        for end in [15, 31, 32] {
            assert_eq!(
                a.0.lock().unwrap().data[&h[end].head.hash],
                b.0.lock().unwrap().data[&h[end].head.hash]
            );
        }
        assert_eq!(p.load().unwrap(), h);
    }
}

#[test]
fn missing_both_copies_and_failed_repair_or_readback_never_advance_witness() {
    for fault in 0..5 {
        let (mut p, a, b, w, h) = setup(17);
        let before = w.0.lock().unwrap().anchor;
        a.0.lock().unwrap().data.remove(&h[15].head.hash);
        match fault {
            0 => {
                b.0.lock().unwrap().data.remove(&h[15].head.hash);
            }
            1 => {
                a.0.lock().unwrap().fail_put = true;
            }
            2 => {
                a.0.lock().unwrap().corrupt_put = true;
            }
            3 => {
                b.0.lock().unwrap().fail_put = true;
            }
            _ => {
                // New tail is replicated, but repair of an OLD pack fails.
                a.0.lock().unwrap().fail_key = Some(h[15].head.hash);
            }
        }
        assert!(p.append(Some(h[16].head), &h[17]).is_err());
        assert_eq!(w.0.lock().unwrap().anchor, before);
        if fault != 0 {
            assert_eq!(p.load().unwrap(), h[..17]);
        }
    }
}

#[test]
fn interrupted_cas_or_post_cas_reads_resolve_only_against_the_fresh_witness() {
    for fault in 0..3 {
        let (mut p, _, _, w, h) = setup(17);
        {
            let mut s = w.0.lock().unwrap();
            s.cas = 0;
            s.reject = fault == 0;
            s.lose_ack = fault == 1;
            s.fail_read_after_cas = fault == 2;
        }
        assert!(p.append(Some(h[16].head), &h[17]).is_err());
        {
            let mut s = w.0.lock().unwrap();
            s.reject = false;
            s.lose_ack = false;
            s.fail_read_after_cas = false;
        }
        assert_eq!(
            p.load().unwrap(),
            if fault == 0 { h[..17].to_vec() } else { h }
        );
    }
}

#[test]
fn full_ciphertext_archive_import_has_no_cas_and_requires_exact_current_head() {
    let (mut original, _, _, w, h) = setup(33);
    let archive = original.snapshot().unwrap();
    let (a, b) = (Memory::default(), Memory::default());
    let mut imported = Packed::new(stream(), 1, a.clone(), b, w.clone()).unwrap();
    let before = w.0.lock().unwrap().cas;
    let old = encode_snapshot(stream(), &h[..16]).unwrap();
    assert_eq!(imported.restore_snapshot(&old), Err(Error::Stale));
    assert!(a.0.lock().unwrap().data.is_empty());
    imported.restore_snapshot(&archive).unwrap();
    assert_eq!(imported.load().unwrap(), h[..33]);
    assert_eq!(imported.snapshot().unwrap(), archive);
    assert_eq!(w.0.lock().unwrap().cas, before);
}

#[test]
fn full_size_pack_and_global_history_limits_are_explicit() {
    let h = frames(16, MAX_RECORD);
    let bytes = Pack(h.clone()).bytes(stream()).unwrap();
    assert_eq!(bytes.len(), MAX_PACK_BYTES);
    assert_eq!(Pack::decode(stream(), h[15].head, &bytes).unwrap().0, h);
    let (a, b, w) = (Memory::default(), Memory::default(), Trust::default());
    let h = frames(65, MAX_RECORD);
    for chunk in h.chunks(16) {
        let p = Pack(chunk.to_vec());
        let bytes = p.bytes(stream()).unwrap();
        a.0.lock().unwrap().data.insert(p.head().hash, bytes);
    }
    w.0.lock().unwrap().anchor.head = Some(h[63].head);
    let mut p = Packed::new(stream(), 1, a, b, w).unwrap();
    assert_eq!(p.load().unwrap().len(), 64);
    assert_eq!(p.append(Some(h[63].head), &h[64]), Err(Error::Limit));
    assert_eq!(p.anchor().unwrap().head, Some(h[63].head));
    p.witness.0.lock().unwrap().anchor.head = Some(h[64].head);
    assert_eq!(p.load().unwrap_err(), Error::Limit);
}

#[test]
fn sequence_limit_refuses_new_acceptance_without_pruning_existing_evidence() {
    let h = frames(MAX_RECORDS + 1, 32);
    let (a, b, w) = (Memory::default(), Memory::default(), Trust::default());
    for chunk in h[..MAX_RECORDS].chunks(PACK_RECORDS) {
        let pack = Pack(chunk.to_vec());
        a.0.lock()
            .unwrap()
            .data
            .insert(pack.head().hash, pack.bytes(stream()).unwrap());
    }
    w.0.lock().unwrap().anchor.head = Some(h[MAX_RECORDS - 1].head);
    let mut p = Packed::new(stream(), 1, a, b, w.clone()).unwrap();
    assert_eq!(p.load().unwrap(), h[..MAX_RECORDS]);
    assert_eq!(
        p.append(Some(h[MAX_RECORDS - 1].head), &h[MAX_RECORDS]),
        Err(Error::Limit)
    );
    assert_eq!(w.0.lock().unwrap().cas, 0);
}

#[test]
fn maximum_legal_archive_roundtrips_with_the_exact_header_bound() {
    let history = frames(MAX_RECORDS, MAX_HISTORY_BYTES / MAX_RECORDS);
    let archive = encode_snapshot(stream(), &history).unwrap();
    assert_eq!(archive.len(), MAX_HISTORY_BYTES + MAX_RECORDS * 80 + 118);
    assert_eq!(decode_snapshot(stream(), &archive).unwrap(), history);
    let mut extra = archive;
    extra.push(0);
    assert_eq!(decode_snapshot(stream(), &extra).unwrap_err(), Error::Limit);
}
