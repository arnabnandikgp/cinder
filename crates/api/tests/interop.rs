//! Canonical bytes/signatures generated using Node's independent Ed25519 binding.
use cinder_api::{wire::*, *};
use cinder_kernel::identity::*;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};
fn hex(b: &[u8]) -> String {
    b.iter().map(|n| format!("{n:02x}")).collect()
}
fn decode(s: &str) -> Vec<u8> {
    s.as_bytes()
        .chunks_exact(2)
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn signed_read_schema_has_the_same_p18_envelope_and_typescript_suffix() {
    let signer = SigningKey::from_bytes(&[9; 32]);
    let mut req = Request {
        domain: Domain {
            network: NetworkId::new([1; 32]).unwrap(),
            deployment: DeploymentId::new([2; 32]).unwrap(),
        },
        account: AccountId::new([1; 32]).unwrap(),
        id: RequestId::new([40; 32]).unwrap(),
        policy: PolicyVersion::new(1).unwrap(),
        epoch: 1,
        signer: signer.verifying_key().to_bytes(),
        session: [8; 32],
        expires_at: 100,
        command: Command::View,
        signature: [0; 64],
    };
    let original = req.message();
    let prefix = &original[..original.len() - 1];
    for kind in 0..10 {
        req.command = Command::Read(cinder_api::reads::Query {
            kind,
            cursor: [0; 40],
            limit: 64,
        });
        let message = req.message();
        assert_eq!(&message[..prefix.len()], prefix);
        let mut suffix = vec![8, 1, kind];
        suffix.extend_from_slice(&[0; 40]);
        suffix.extend_from_slice(&64u16.to_be_bytes());
        assert_eq!(&message[prefix.len()..], suffix);
        req.signature = signer.sign(&message).to_bytes();
        assert!(Request::decode(req.encode().unwrap().as_bytes()).is_ok());
    }
}
#[test]
fn all_request_methods_match_typescript_bytes_digests_and_signatures() {
    let signer = SigningKey::from_bytes(&[9; 32]);
    let market = MarketId::new([7; 32]).unwrap();
    let id = RequestId::new([40; 32]).unwrap();
    let commands = [
        Command::View,
        Command::Operation(id),
        Command::Order {
            market,
            lots: -2,
            minimum: 90,
            maximum: 9_007_199_254_740_997,
            fee: 1_i128 << 100,
            tif: 2,
            reduce_only: true,
            good_until: 90,
        },
        Command::Cancel {
            target: id,
            attempt: AttemptId::new([50; 32]).unwrap(),
            good_until: 90,
        },
        Command::Payout {
            net: 10,
            maximum_fee: 2,
            allow_partial: true,
            good_until: 90,
        },
        Command::Grant(Grant {
            key: signer.verifying_key().to_bytes(),
            methods: 7,
            market,
            maximum_lots: 2,
            maximum_fee: 1,
            maximum_orders: 1,
            expires_at: 90,
        }),
        Command::Revoke,
        Command::Leverage {
            market,
            leverage: 20_000,
            good_until: 90,
        },
    ];
    let vectors: Vec<_> = include_str!("fixtures/vectors.tsv").lines().collect();
    assert_eq!(vectors.len(), commands.len());
    for (line, command) in vectors.iter().zip(commands) {
        let row: Vec<_> = line.split('\t').collect();
        assert_eq!(row.len(), 5);
        let mut req = Request {
            domain: Domain {
                network: NetworkId::new([1; 32]).unwrap(),
                deployment: DeploymentId::new([2; 32]).unwrap(),
            },
            account: AccountId::new([1; 32]).unwrap(),
            id,
            policy: PolicyVersion::new(1).unwrap(),
            epoch: 1,
            signer: signer.verifying_key().to_bytes(),
            session: [8; 32],
            expires_at: 100,
            command,
            signature: [0; 64],
        };
        req.signature = signer.sign(&req.message()).to_bytes();
        assert_eq!(hex(&req.message()), row[1]);
        assert_eq!(hex(&Sha256::digest(&*req.message())), row[2]);
        assert_eq!(hex(&req.digest()), row[3]);
        assert_eq!(hex(&req.signature), row[4]);
        VerifyingKey::from_bytes(&req.signer)
            .unwrap()
            .verify_strict(&decode(row[1]), &Signature::from_bytes(&req.signature))
            .unwrap();
        assert_eq!(
            Request::decode(req.encode().unwrap().as_bytes()).unwrap(),
            req
        );
    }
}
#[test]
fn reply_schema_matches_shared_synthetic_fixture() {
    let r = Response::Receipt(Receipt {
        id: RequestId::new([40; 32]).unwrap(),
        digest: [6; 32],
        policy: PolicyVersion::new(1).unwrap(),
        outcome: Outcome::Unknown,
        filled: -1,
        paid: 4,
        rail_fees: 1,
        fee_cap: 10,
        possibly_exposed: true,
        allow_partial: true,
    });
    assert_eq!(
        hex(r.encode().unwrap().as_bytes()),
        include_str!("fixtures/reply.hex").trim()
    );
    assert_eq!(
        Error::Unauthorized.encode_private().unwrap().as_bytes(),
        b"CINDER-API-REPLY\0\x00\x01\x02\x01"
    );
}
