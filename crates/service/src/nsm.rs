//! Enclave-only NSM adapter. No quote file, parent approval, RNG fallback or
//! alternate root. The injectable exchange seam is private to this module's
//! unit tests; public construction always opens the actual Linux NSM device.
use crate::{
    Error,
    attestation::{self, Context, MAX_QUOTE, MAX_SESSION, Policy},
    transport::Attester,
};
use aws_nitro_enclaves_nsm_api::api::{Digest, Request, Response};
use std::sync::Mutex;
use zeroize::Zeroizing;

struct Device {
    #[cfg(target_os = "linux")]
    fd: i32,
}
impl Device {
    fn open() -> Result<Self, Error> {
        #[cfg(target_os = "linux")]
        {
            let fd = aws_nitro_enclaves_nsm_api::driver::nsm_init();
            if fd < 0 {
                return Err(Error);
            }
            Ok(Self { fd })
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(Error)
        }
    }
    fn exchange(&mut self, request: Request) -> Result<Response, Error> {
        #[cfg(target_os = "linux")]
        {
            Ok(aws_nitro_enclaves_nsm_api::driver::nsm_process_request(
                self.fd, request,
            ))
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = request;
            Err(Error)
        }
    }
}
impl Drop for Device {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        aws_nitro_enclaves_nsm_api::driver::nsm_exit(self.fd);
    }
}

fn check_measurements(
    exchange: &mut impl FnMut(Request) -> Result<Response, Error>,
    expected: &Policy,
) -> Result<(), Error> {
    Policy::decode(&expected.encode())?;
    match exchange(Request::DescribeNSM)? {
        Response::DescribeNSM {
            version_major: 1,
            module_id,
            max_pcrs,
            locked_pcrs,
            digest: Digest::SHA384,
            ..
        } if !module_id.is_empty()
            && module_id.len() <= 256
            && module_id.is_ascii()
            && (3..=32).contains(&max_pcrs)
            && (0..3).all(|i| locked_pcrs.contains(&i))
            && locked_pcrs.iter().all(|i| *i < max_pcrs) => {}
        _ => return Err(Error),
    }
    for (index, wanted) in expected.pcrs.iter().enumerate() {
        match exchange(Request::DescribePCR {
            index: index as u16,
        })? {
            Response::DescribePCR { lock: true, data } if data.as_slice() == wanted => {}
            _ => return Err(Error),
        }
    }
    Ok(())
}

fn entropy(
    exchange: &mut impl FnMut(Request) -> Result<Response, Error>,
) -> Result<Zeroizing<[u8; 32]>, Error> {
    let Response::GetRandom { random } = exchange(Request::GetRandom)? else {
        return Err(Error);
    };
    let random = Zeroizing::new(random);
    if !(32..=4096).contains(&random.len()) {
        return Err(Error);
    }
    let mut result = Zeroizing::new([0; 32]);
    result.copy_from_slice(&random[..32]);
    if *result == [0; 32] {
        return Err(Error);
    }
    Ok(result)
}

fn quote_request(expected: &Policy, context: &Context<'_>) -> Result<Request, Error> {
    if context.nonce == [0; 32]
        || context.exporter == [0; 32]
        || context.boot == [0; 32]
        || context.spki.is_empty()
        || context.spki.len() > 1024
        || context.expires <= context.now
        || context.expires - context.now > MAX_SESSION
    {
        return Err(Error);
    }
    let key = openssl::pkey::PKey::public_key_from_der(context.spki)?;
    if key.public_key_to_der()? != context.spki {
        return Err(Error);
    }
    Ok(Request::Attestation {
        user_data: Some(attestation::user_data(expected, context).to_vec().into()),
        nonce: Some(context.nonce.to_vec().into()),
        public_key: Some(context.spki.to_vec().into()),
    })
}

/// Actual NSM handle tied to one independently selected release policy.
/// Missing hardware or a debug/wrong/unlocked PCR bank fails construction.
/// This adapter is not itself qualified key release, clock or durable storage.
pub struct Nsm {
    device: Mutex<Device>,
    policy: Policy,
}
impl Nsm {
    /// Open `/dev/nsm`, check locked PCR0/1/2 and require NSM entropy before use.
    /// NSM major version 1/SHA384 is the closed profile; future majors reject.
    pub fn open(policy: Policy) -> Result<Self, Error> {
        Policy::decode(&policy.encode())?;
        let mut device = Device::open()?;
        check_measurements(&mut |request| device.exchange(request), &policy)?;
        let _boot_entropy = entropy(&mut |request| device.exchange(request))?;
        Ok(Self {
            device: Mutex::new(device),
            policy,
        })
    }
    /// Exactly 32 bytes obtained from NSM, zeroized on drop; no host RNG fallback.
    /// TLS/library entropy still requires its own image/runtime qualification.
    pub fn random(&self) -> Result<Zeroizing<[u8; 32]>, Error> {
        let mut device = self.device.lock().map_err(|_| Error)?;
        entropy(&mut |request| device.exchange(request))
    }
}
impl Attester for Nsm {
    fn quote(&self, policy: &Policy, context: &Context<'_>) -> Result<Vec<u8>, Error> {
        if policy.encode() != self.policy.encode() {
            return Err(Error);
        }
        let request = quote_request(policy, context)?;
        let mut device = self.device.lock().map_err(|_| Error)?;
        let Response::Attestation { document } = device.exchange(request)? else {
            return Err(Error);
        };
        if document.is_empty() || document.len() > MAX_QUOTE {
            return Err(Error);
        }
        // Reuse the production pinned-root verifier, including the exact live
        // SPKI/challenge/exporter and qualified-clock freshness checks.
        attestation::verify(&document, policy, context)?;
        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, VecDeque};

    fn policy() -> Policy {
        Policy {
            domain: [1; 64],
            manifest: [2; 32],
            pcrs: [[3; 48], [4; 48], [5; 48]],
        }
    }
    fn description() -> Response {
        Response::DescribeNSM {
            version_major: 1,
            version_minor: 0,
            version_patch: 0,
            module_id: "synthetic-unit-test".into(),
            max_pcrs: 32,
            locked_pcrs: BTreeSet::from([0, 1, 2]),
            digest: Digest::SHA384,
        }
    }
    #[test]
    fn boot_requires_exact_locked_non_debug_measurements_and_known_profile() {
        for case in 0..9 {
            let mut p = policy();
            let mut info = description();
            if let Response::DescribeNSM {
                version_major,
                digest,
                locked_pcrs,
                module_id,
                max_pcrs,
                ..
            } = &mut info
            {
                match case {
                    1 => *version_major = 2,
                    2 => *digest = Digest::SHA256,
                    3 => {
                        locked_pcrs.remove(&1);
                    }
                    4 => module_id.clear(),
                    5 => *max_pcrs = 2,
                    8 => p.pcrs[0] = [0; 48],
                    _ => {}
                }
            }
            let mut replies = VecDeque::from([info]);
            for (index, wanted) in policy().pcrs.iter().enumerate() {
                let mut data = wanted.to_vec();
                if case == 6 && index == 1 {
                    data[0] ^= 1;
                }
                replies.push_back(Response::DescribePCR {
                    lock: case != 7,
                    data,
                });
            }
            let mut requests = Vec::new();
            let result = check_measurements(
                &mut |request| {
                    requests.push(request);
                    replies.pop_front().ok_or(Error)
                },
                &p,
            );
            assert_eq!(result.is_ok(), case == 0);
            if case == 0 {
                assert!(matches!(requests[0], Request::DescribeNSM));
                for (index, request) in requests[1..].iter().enumerate() {
                    assert!(
                        matches!(request, Request::DescribePCR { index: got } if *got == index as u16)
                    );
                }
            }
        }
    }
    #[test]
    fn entropy_is_bounded_and_fails_without_fallback_or_retry() {
        for size in [0, 31, 32, 256, 4096, 4097] {
            let mut count = 0;
            let result = entropy(&mut |request| {
                count += 1;
                assert!(matches!(request, Request::GetRandom));
                Ok(Response::GetRandom {
                    random: vec![7; size],
                })
            });
            assert_eq!(count, 1);
            assert_eq!(result.is_ok(), (32..=4096).contains(&size));
            if let Ok(value) = result {
                assert_eq!(*value, [7; 32]);
            }
        }
        assert!(
            entropy(&mut |_| Ok(Response::GetRandom {
                random: vec![0; 32]
            }))
            .is_err()
        );
        assert!(entropy(&mut |_| Ok(Response::LockPCR)).is_err());
        assert!(entropy(&mut |_| Err(Error)).is_err());
    }
    #[test]
    fn native_attestation_binds_this_live_socket_and_rejects_bad_context() {
        use openssl::{
            ec::{EcGroup, EcKey},
            nid::Nid,
            pkey::PKey,
        };
        let key = PKey::from_ec_key(
            EcKey::generate(&EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap()).unwrap(),
        )
        .unwrap()
        .public_key_to_der()
        .unwrap();
        let p = policy();
        let mut c = Context {
            nonce: [6; 32],
            exporter: [7; 32],
            boot: [8; 32],
            now: 100,
            expires: 200,
            spki: &key,
        };
        let Request::Attestation {
            nonce,
            user_data,
            public_key,
        } = quote_request(&p, &c).unwrap()
        else {
            panic!("wrong request");
        };
        assert_eq!(nonce.unwrap().as_ref(), &c.nonce);
        assert_eq!(user_data.unwrap().as_ref(), &attestation::user_data(&p, &c));
        assert_eq!(public_key.unwrap().as_ref(), c.spki);
        c.expires = c.now;
        assert!(quote_request(&p, &c).is_err());
        c.expires = c.now + MAX_SESSION + 1;
        assert!(quote_request(&p, &c).is_err());
        c.expires = 200;
        c.nonce = [0; 32];
        assert!(quote_request(&p, &c).is_err());
        c.nonce = [6; 32];
        c.spki = b"unparsed parent-supplied public key";
        assert!(quote_request(&p, &c).is_err());
    }
    #[test]
    fn public_constructor_cannot_fall_back_to_the_unit_test_device() {
        // Ordinary Linux/macOS hosts have no NSM; an actual enclave also cannot
        // satisfy this synthetic release's fabricated PCR values.
        assert!(Nsm::open(policy()).is_err());
    }
}
