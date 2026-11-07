//! Real ECDSA/X.509 verification against a disposable CA; no AWS hardware claim.
#[cfg(feature = "local-fixture")]
mod qualified {
    use cinder_service::{
        attestation::*,
        fixture::*,
        transport::{Attester, Clock},
    };
    use openssl::{
        ec::{EcGroup, EcKey},
        nid::Nid,
        pkey::PKey,
    };

    fn context(spki: &[u8]) -> Context<'_> {
        let now = FixtureClock.now().unwrap();
        Context {
            nonce: [1; 32],
            exporter: [2; 32],
            boot: [3; 32],
            expires: now + 60_000,
            now,
            spki,
        }
    }
    fn spki() -> Vec<u8> {
        PKey::from_ec_key(
            EcKey::generate(&EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap()).unwrap(),
        )
        .unwrap()
        .public_key_to_der()
        .unwrap()
    }

    #[test]
    fn signature_chain_release_freshness_and_exact_live_socket_all_required() {
        let attester = FixtureAttester::new().unwrap();
        let public_key = spki();
        let mut c = context(&public_key);
        let mut p = policy();
        let quote = attester.quote(&p, &c).unwrap();
        verify_fixture(&quote, &p, &c, attester.root()).unwrap();
        assert!(verify(&quote, &p, &c).is_err()); // Production NEVER accepts fixture CA.
        let other = FixtureAttester::new().unwrap();
        assert!(verify_fixture(&quote, &p, &c, other.root()).is_err());
        p.pcrs[0][0] ^= 1;
        assert!(verify_fixture(&quote, &p, &c, attester.root()).is_err());
        p.pcrs[0][0] ^= 1;
        p.manifest[0] ^= 1;
        assert!(verify_fixture(&quote, &p, &c, attester.root()).is_err());
        p.manifest[0] ^= 1;
        p.domain[0] ^= 1;
        assert!(verify_fixture(&quote, &p, &c, attester.root()).is_err());
        p.domain[0] ^= 1;
        for field in 0..3 {
            match field {
                0 => c.nonce[0] ^= 1,
                1 => c.exporter[0] ^= 1,
                _ => c.boot[0] ^= 1,
            };
            assert!(verify_fixture(&quote, &p, &c, attester.root()).is_err());
            match field {
                0 => c.nonce[0] ^= 1,
                1 => c.exporter[0] ^= 1,
                _ => c.boot[0] ^= 1,
            };
        }
        let other_spki = spki();
        let old = c.spki;
        c.spki = &other_spki;
        assert!(verify_fixture(&quote, &p, &c, attester.root()).is_err());
        c.spki = old;
        let now = c.now;
        c.now += MAX_QUOTE_AGE + 1;
        assert!(verify_fixture(&quote, &p, &c, attester.root()).is_err());
        c.now = now;
        c.now -= 1;
        assert!(verify_fixture(&quote, &p, &c, attester.root()).is_err());
        c.now = now;
        c.expires = c.now;
        assert!(verify_fixture(&quote, &p, &c, attester.root()).is_err());
        c.expires = now + 60_000;
        let mut corrupt = quote.clone();
        *corrupt.last_mut().unwrap() ^= 1;
        assert!(verify_fixture(&corrupt, &p, &c, attester.root()).is_err());
        let mut trailing = quote.clone();
        trailing.push(0);
        assert!(verify_fixture(&trailing, &p, &c, attester.root()).is_err());
        let mut tag = quote.clone();
        tag[0] = 0xd1;
        assert!(verify_fixture(&tag, &p, &c, attester.root()).is_err());
        verify_fixture(&quote[1..], &p, &c, attester.root()).unwrap();
        // Full X.509 lifetime still enforced when a newly signed quote is fresh.
        c.now += 2 * 86_400_000;
        c.expires = c.now + 60_000;
        let future = attester.quote(&p, &c).unwrap();
        assert!(verify_fixture(&future, &p, &c, attester.root()).is_err());
    }

    #[test]
    fn debug_release_empty_policy_and_oversize_reject() {
        let a = FixtureAttester::new().unwrap();
        let key = spki();
        let c = context(&key);
        let mut p = policy();
        p.pcrs[0] = [0; 48];
        let q = a.quote(&p, &c).unwrap();
        assert!(verify_fixture(&q, &p, &c, a.root()).is_err());
        assert!(Policy::decode(&[0; 240]).is_err());
        assert!(Policy::decode(&[0; 241]).is_err());
        assert!(verify_fixture(&vec![0; MAX_QUOTE + 1], &policy(), &c, a.root()).is_err());
    }
}
