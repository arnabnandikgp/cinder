//! Adversarial identity/scope/byte-level tests, not signing or durable replay.
use cinder_kernel::{Error, codec::Canonical, identity::*};

fn domain() -> Domain {
    Domain {
        network: NetworkId::new([1; 32]).unwrap(),
        deployment: DeploymentId::new([2; 32]).unwrap(),
    }
}
fn request() -> RequestKey {
    RequestKey {
        domain: domain(),
        account: AccountId::new([3; 32]).unwrap(),
        request: RequestId::new([4; 32]).unwrap(),
    }
}
fn event() -> EventKey {
    EventKey {
        scope: EventScope {
            domain: domain(),
            venue: VenueId::new([5; 32]).unwrap(),
            account: VenueAccountId::new([6; 32]).unwrap(),
            namespace: NamespaceId::new([7; 32]).unwrap(),
        },
        event: EconomicEventId::new(b"execution-1").unwrap(),
        leg: 0,
    }
}
fn payload(bytes: &[u8]) -> Payload<'_> {
    Payload {
        schema: SchemaVersion::V1,
        policy: PolicyVersion::new(1).unwrap(),
        bytes,
    }
}

#[test]
fn key_kinds_domains_and_order_attempts_never_alias() {
    let request = request();
    let attempt = AttemptKey {
        request,
        attempt: AttemptId::new(request.request.bytes()).unwrap(),
    };
    let second = AttemptKey {
        attempt: AttemptId::new([8; 32]).unwrap(),
        ..attempt
    };
    assert_ne!(request.encode(), attempt.encode());
    assert_ne!(attempt.encode(), second.encode());
    assert!(RequestKey::decode(&attempt.encode()).is_err());
    assert!(AttemptKey::decode(&request.encode()).is_err());
    let key = RecordKey::Request(request);
    assert_eq!(key.require_domain(domain()), Ok(()));
    assert_eq!(
        key.require_domain(Domain {
            network: NetworkId::new([9; 32]).unwrap(),
            ..domain()
        }),
        Err(Error::ScopeMismatch)
    );
    assert_eq!(
        key.require_domain(Domain {
            deployment: DeploymentId::new([9; 32]).unwrap(),
            ..domain()
        }),
        Err(Error::ScopeMismatch)
    );
    for key in [
        key,
        RecordKey::Attempt(attempt),
        RecordKey::Economic(event()),
    ] {
        assert_eq!(RecordKey::decode(&key.encode()), Ok(key));
    }
}

#[test]
fn event_identity_includes_every_source_coordinate_and_participant_leg() {
    let original = event();
    let mut variants = vec![original.clone(); 7];
    variants[0].scope.domain.network = NetworkId::new([9; 32]).unwrap();
    variants[1].scope.domain.deployment = DeploymentId::new([9; 32]).unwrap();
    variants[2].scope.venue = VenueId::new([9; 32]).unwrap();
    variants[3].scope.account = VenueAccountId::new([9; 32]).unwrap();
    variants[4].scope.namespace = NamespaceId::new([9; 32]).unwrap();
    variants[5].leg = 1;
    variants[6].event = EconomicEventId::new(b"execution-2").unwrap();
    for (i, variant) in variants.into_iter().enumerate() {
        assert_ne!(variant.encode(), original.encode());
        assert_ne!(variant, original);
        if i < 5 {
            assert_eq!(
                variant.require_scope(original.scope),
                Err(Error::ScopeMismatch)
            );
        }
    }
    // REST and WS use this same qualified semantic key, not a per-transport one.
    assert_eq!(EventKey::decode(&original.encode()), Ok(original));
}

#[test]
fn exact_payload_comparison_contains_conflicts_without_mutating_prior_evidence() {
    let original = RecordKey::Economic(event());
    let before = original.encode();
    assert_eq!(
        classify_replay(&original, payload(b"fill:1"), &original, payload(b"fill:1")),
        Replay::Duplicate
    );
    assert_eq!(
        classify_replay(&original, payload(b"fill:1"), &original, payload(b"fill:2")),
        Replay::Conflict
    );
    let changed_policy = Payload {
        policy: PolicyVersion::new(2).unwrap(),
        ..payload(b"fill:1")
    };
    assert_eq!(
        classify_replay(&original, payload(b"fill:1"), &original, changed_policy),
        Replay::Conflict
    );
    let attempt = RecordKey::Attempt(AttemptKey {
        request: request(),
        attempt: AttemptId::new([8; 32]).unwrap(),
    });
    assert_eq!(
        classify_replay(&original, payload(b"fill:1"), &attempt, payload(b"fill:1")),
        Replay::Distinct
    );
    assert_eq!(original.encode(), before);
    assert_eq!(
        changed_policy
            .policy
            .require(PolicyVersion::new(1).unwrap()),
        Err(Error::UnknownVersion)
    );
    assert_eq!(PrecisionVersion::new(0), Err(Error::UnknownVersion));
    assert_eq!(PolicyVersion::new(0), Err(Error::UnknownVersion));
}

#[test]
fn external_ids_are_lossless_bounded_and_length_delimited() {
    assert_eq!(EconomicEventId::new(b""), Err(Error::InvalidId));
    assert_eq!(EconomicEventId::new(&[1; 129]), Err(Error::InvalidId));
    assert_eq!(AssetId::new([0; 32]), Err(Error::InvalidId));
    let a = EventKey {
        event: EconomicEventId::new(b"01").unwrap(),
        ..event()
    };
    let b = EventKey {
        event: EconomicEventId::new(b"1").unwrap(),
        ..event()
    };
    assert_ne!(a.encode(), b.encode());
    let full = EventKey {
        event: EconomicEventId::new(&[255; 128]).unwrap(),
        leg: u32::MAX,
        ..event()
    };
    assert_eq!(EventKey::decode(&full.encode()), Ok(full));
}

#[test]
fn request_key_golden_layout_is_stable_and_not_a_display_fingerprint() {
    let mut expected = b"CINDER\0\0\x01\x10".to_vec();
    for byte in 1..=4 {
        expected.extend([byte; 32]);
    }
    assert_eq!(request().encode(), expected);
    assert_eq!(RequestKey::decode(&expected), Ok(request()));
    let mut attempt = expected.clone();
    attempt[9] = 17;
    attempt.extend([8; 32]);
    assert_eq!(
        AttemptKey {
            request: request(),
            attempt: AttemptId::new([8; 32]).unwrap()
        }
        .encode(),
        attempt
    );
    let mut economic = b"CINDER\0\0\x01\x12".to_vec();
    for byte in [1, 2, 5, 6, 7] {
        economic.extend([byte; 32]);
    }
    economic.extend([0, 11]);
    economic.extend(b"execution-1");
    economic.extend([0; 4]);
    assert_eq!(event().encode(), economic);
}

#[test]
fn malformed_frames_reject_at_every_truncation_and_header_boundary() {
    for key in [
        RecordKey::Request(request()),
        RecordKey::Attempt(AttemptKey {
            request: request(),
            attempt: AttemptId::new([8; 32]).unwrap(),
        }),
        RecordKey::Economic(event()),
    ] {
        let bytes = key.encode();
        for end in 0..bytes.len() {
            assert!(RecordKey::decode(&bytes[..end]).is_err());
        }
        let mut changed = bytes.clone();
        changed.push(0);
        assert!(RecordKey::decode(&changed).is_err());
        for (offset, value) in [(0, b'X'), (7, 1), (8, 2), (9, 255)] {
            let mut changed = bytes.clone();
            changed[offset] = value;
            assert!(RecordKey::decode(&changed).is_err());
        }
    }
    let mut zero_id = request().encode();
    zero_id[10..42].fill(0);
    assert_eq!(RequestKey::decode(&zero_id), Err(Error::InvalidId));
    for length in [0_u16, 129, u16::MAX] {
        let mut changed = event().encode();
        changed[170..172].copy_from_slice(&length.to_be_bytes());
        assert_eq!(EventKey::decode(&changed), Err(Error::InvalidId));
    }
}

#[test]
fn property_deterministic_byte_mutations_either_reject_or_reencode_identically() {
    let mut state = 0x517c_c1b7_2722_0a95_u64;
    let original = RecordKey::Economic(event()).encode();
    for _ in 0..10_000 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let mut bytes = original.clone();
        let offset = (state as usize) % bytes.len();
        bytes[offset] ^= (state >> 32) as u8;
        if let Ok(decoded) = RecordKey::decode(&bytes) {
            assert_eq!(decoded.encode(), bytes);
        }
    }
}

#[test]
fn payload_frame_binds_policy_and_rejects_ambiguous_lengths() {
    let mut expected = b"CINDER\0\0\x01\x20".to_vec();
    expected.extend([0, 0, 0, 1, 0, 0, 0, 2]);
    expected.extend(b"AB");
    assert_eq!(payload(b"AB").encode(), Ok(expected.clone()));
    assert_eq!(Payload::decode(&expected), Ok(payload(b"AB")));
    for end in 0..expected.len() {
        assert!(Payload::decode(&expected[..end]).is_err());
    }
    let mut changed = expected.clone();
    changed.push(0);
    assert_eq!(Payload::decode(&changed), Err(Error::InvalidEncoding));
    changed = expected.clone();
    changed[8] = 2;
    assert_eq!(Payload::decode(&changed), Err(Error::UnknownVersion));
    changed = expected.clone();
    changed[10..14].fill(0);
    assert_eq!(Payload::decode(&changed), Err(Error::UnknownVersion));
    for length in [0_u32, 65_537, u32::MAX] {
        let mut changed = expected.clone();
        changed[14..18].copy_from_slice(&length.to_be_bytes());
        assert_eq!(Payload::decode(&changed), Err(Error::InvalidEncoding));
    }
    assert!(payload(b"").encode().is_err());
    assert!(payload(&vec![0; 65_537]).encode().is_err());
    let boundary = vec![0; 65_536];
    let encoded = payload(&boundary).encode().unwrap();
    assert_eq!(Payload::decode(&encoded).unwrap().bytes, boundary);
    let v2 = Payload {
        policy: PolicyVersion::new(2).unwrap(),
        ..payload(b"AB")
    };
    assert_ne!(v2.encode().unwrap(), expected);
}
