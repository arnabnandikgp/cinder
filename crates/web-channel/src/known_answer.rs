// Public deterministic Cacophony NK vector from Snow 0.10.0:
// tests/vectors/cacophony.txt, selected without generating expected ciphertext.
// Source: https://github.com/mcginty/snow/tree/v0.10.0/tests/vectors
// Snow package SHA256: 599b506ccc4aff8cf7844bc42cf783009a434c1e26c964432560fb6d6ad02d82
// MIT notice retained in THIRD_PARTY.md. These keys are public test fixtures.
use snow::Builder;

fn hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}
fn run() -> Option<()> {
    let prologue = hex("4a6f686e2047616c74")?;
    let public = hex("31e0303fd6418d2f8c0e78b91f22e8caed0fbe48656dcf4767e4834f701b8f62")?;
    let private = hex("4a3acbfdb163dec651dfa3194dece676d437029c62a408b4c5ea9114246e4893")?;
    let ie = hex("893e28b9dc6ca8d611ab664754b8ceb7bac5117349a4439a6b0569da977c464a")?;
    let re = hex("bbdb4cdbd309f1a1f2e1456967fe288cadd6f712d65dc7b7793d5e63da6b375b")?;
    let builder = || {
        Builder::new(crate::PROFILE.parse().unwrap())
            .prologue(&prologue)
            .unwrap()
    };
    let mut c = builder()
        .remote_public_key(&public)
        .ok()?
        .fixed_ephemeral_key_for_testing_only(&ie)
        .build_initiator()
        .ok()?;
    let mut s = builder()
        .local_private_key(&private)
        .ok()?
        .fixed_ephemeral_key_for_testing_only(&re)
        .build_responder()
        .ok()?;
    let messages = [
        (
            "4c756477696720766f6e204d69736573",
            "ca35def5ae56cec33dc2036731ab14896bc4c75dbb07a61f879f8e3afa4c79448134d00711fdb390a0d178fa008f6d47d2891e5ea18ae136c3b4c23ac384efb0",
        ),
        (
            "4d757272617920526f746862617264",
            "95ebc60d2b1fa672c1f46a8aa265ef51bfe38e7ccb39ec5be34069f1448088438ea16e3701bc0d77744f117bee22451c9afa7f4cdbbcff00c04a8ee0913c88",
        ),
        (
            "462e20412e20486179656b",
            "a62de29ce27cb80245d440d986ed816c156e9d757d7008df2198b0",
        ),
        (
            "4361726c204d656e676572",
            "174a35f11c689f4530d7208618e0564ae12f2f50ba8eb4df5382ff",
        ),
        (
            "4a65616e2d426170746973746520536179",
            "337e475ebb8eae60f91974c4e455a5af38d1d8628d1803b160d60442874b0a1777",
        ),
        (
            "457567656e2042f6686d20766f6e2042617765726b",
            "047e80e060b7bb08b53c5a23dfe9920cae135b9d1dc6302fc475003062723700366346ac9d",
        ),
    ];
    let mut wire = [0; 128];
    let mut clear = [0; 128];
    for (i, (payload, expected)) in messages[..2].iter().enumerate() {
        let body = hex(payload)?;
        let (writer, reader) = if i == 0 {
            (&mut c, &mut s)
        } else {
            (&mut s, &mut c)
        };
        let n = writer.write_message(&body, &mut wire).ok()?;
        if wire[..n] != hex(expected)? {
            return None;
        }
        let n = reader.read_message(&wire[..n], &mut clear).ok()?;
        if clear[..n] != body {
            return None;
        }
    }
    let hash = hex("2efa38a9c7c93ac98f3a097af25c2f58b9e7673787717bc27e98827118c2c1a5")?;
    if c.get_handshake_hash() != hash || s.get_handshake_hash() != hash {
        return None;
    }
    let mut c = c.into_transport_mode().ok()?;
    let mut s = s.into_transport_mode().ok()?;
    for (i, (payload, expected)) in messages[2..].iter().enumerate() {
        let body = hex(payload)?;
        let (writer, reader) = if i % 2 == 0 {
            (&mut c, &mut s)
        } else {
            (&mut s, &mut c)
        };
        let n = writer.write_message(&body, &mut wire).ok()?;
        if wire[..n] != hex(expected)? {
            return None;
        }
        let n = reader.read_message(&wire[..n], &mut clear).ok()?;
        if clear[..n] != body {
            return None;
        }
    }
    Some(())
}
pub(super) fn verify() -> bool {
    run().is_some()
}
