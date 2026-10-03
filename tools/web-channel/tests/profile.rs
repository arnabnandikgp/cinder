//! Canonical public attestation-context and session-binding fixtures.
use cinder_web_channel_qualification::profile;
fn hex(b: &[u8]) -> String {
    b.iter().map(|v| format!("{v:02x}")).collect()
}
fn policy() -> Vec<u8> {
    [(32, 1), (32, 2), (32, 21), (48, 22), (48, 23), (48, 24)]
        .into_iter()
        .flat_map(|(n, v)| vec![v; n])
        .collect()
}
#[test]
fn canonical_public_encoding_vectors() {
    let p = policy();
    let fields: Vec<_> = [31, 32, 33, 34].into_iter().flat_map(|v| [v; 32]).collect();
    let c = profile::context(&p, &fields, 1_767_225_720_000).unwrap();
    assert_eq!(c.len(), 430);
    assert_eq!(
        hex(&profile::user_data(&c)),
        "5588a4acb8961b777189416b9358ab2e1f1c9ef3a2e58174ae2d77017b3dcf982485dfef29fffedbc07d529ce989b09f"
    );
    let q: Vec<_> = (1..=16).collect(); // Public hash input, not a verified quote.
    let prologue = profile::prologue(&c, &q).unwrap();
    assert_eq!(
        hex(&prologue),
        "58c23e98eb7c0614ebfce47d12e035753ed337868f3657fae6ef675102f5fd9e"
    );
    assert_eq!(
        hex(&profile::binding(&prologue, &[42; 32])),
        "20b05c130638b292b20ee299cabae502c67e69362ce45041e0ceb82353cce3c1"
    );
    for n in 0..c.len() {
        let mut changed = c.clone();
        changed[n] ^= 1;
        assert_ne!(profile::user_data(&changed), profile::user_data(&c));
        assert_ne!(profile::prologue(&changed, &q).unwrap(), prologue);
    }
}
#[test]
fn context_requires_exact_nonzero_policy_and_fields() {
    let p = policy();
    let fields = [1; 128];
    for (lo, hi) in [
        (0, 32),
        (32, 64),
        (64, 96),
        (96, 144),
        (144, 192),
        (192, 240),
    ] {
        let mut bad = p.clone();
        bad[lo..hi].fill(0);
        assert!(profile::context(&bad, &fields, 1).is_err());
    }
    for n in 0..4 {
        let mut bad = fields;
        bad[n * 32..(n + 1) * 32].fill(0);
        assert!(profile::context(&p, &bad, 1).is_err());
    }
    assert!(profile::context(&p[..239], &fields, 1).is_err());
    assert!(profile::context(&p, &fields[..127], 1).is_err());
    assert!(profile::context(&p, &fields, 0).is_err());
    assert!(profile::prologue(b"public", &[]).is_err());
    assert!(profile::prologue(b"public", &[0; 16385]).is_err());
}
