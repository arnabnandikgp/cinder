//! Enclave-only NSM adapter. No quote file, parent approval, RNG fallback or
//! alternate root. The injectable exchange seam is private to this module's
//! unit tests; public construction always opens the actual Linux NSM device.
use crate::{
    Error,
    attestation::{self, Context, MAX_QUOTE, MAX_SESSION, Policy},
    transport::{Attester, Clock},
};
use aws_nitro_enclaves_nsm_api::api::{Digest, Request, Response};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

// Reject late results; an in-progress kernel ioctl is NOT cancellable here.
// The measured runtime/hardware qualification must also bound driver latency.
const MAX_NSM_OPERATION: Duration = Duration::from_secs(5);

fn timely(elapsed: Duration) -> Result<(), Error> {
    if elapsed > MAX_NSM_OPERATION {
        Err(Error)
    } else {
        Ok(())
    }
}

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

fn clock_sample(
    exchange: &mut impl FnMut(Request) -> Result<Response, Error>,
    policy: &Policy,
    verify: impl FnOnce(&[u8], &Policy, &[u8; 32]) -> Result<u64, Error>,
) -> Result<u64, Error> {
    let start = Instant::now();
    let nonce = entropy(exchange)?;
    let (key, data) = attestation::clock_binding(policy);
    let Response::Attestation { document } = exchange(Request::Attestation {
        nonce: Some(nonce.to_vec().into()),
        public_key: Some(key.to_vec().into()),
        user_data: Some(data.to_vec().into()),
    })?
    else {
        return Err(Error);
    };
    let now = verify(&document, policy, &nonce)?;
    timely(start.elapsed())?;
    Ok(now)
}

#[derive(Default)]
struct TimeState {
    last: Option<u64>,
    failed: bool,
}
impl TimeState {
    fn active(&self) -> Result<(), Error> {
        if self.failed { Err(Error) } else { Ok(()) }
    }
    fn observe(&mut self, result: Result<u64, Error>) -> Result<u64, Error> {
        let result = self.active().and(result).and_then(|at| {
            if at == 0 || self.last.is_some_and(|last| at < last) {
                Err(Error)
            } else {
                Ok(at)
            }
        });
        match result {
            Ok(at) => {
                self.last = Some(at);
                Ok(at)
            }
            Err(e) => {
                // No cached clock, parent wall-time or later "healthy" reply can
                // recover this boot. Restart goes through normal fresh fencing.
                self.failed = true;
                Err(e)
            }
        }
    }
}
struct State {
    device: Device,
    time: TimeState,
}

/// Actual NSM handle tied to one independently selected release policy.
/// Missing hardware or a debug/wrong/unlocked PCR bank fails construction.
/// Timestamp replies are nonce-bound, AWS-signed and monotone within this boot.
/// This is not durable freshness, recipient key release or hardware qualification.
pub struct Nsm {
    state: Mutex<State>,
    policy: Policy,
}
impl Nsm {
    /// Discover THIS measured image's locked PCR bank. The immutable loaded
    /// manifest is measured with the image; SDK/KMS independently approve its
    /// actual measurements. PCRs cannot be embedded in their own measured image.
    pub fn discover(domain: [u8; 64], manifest: [u8; 32]) -> Result<Self, Error> {
        let mut device = Device::open()?;
        let mut pcrs = [[0; 48]; 3];
        for (i, pcr) in pcrs.iter_mut().enumerate() {
            let Response::DescribePCR { lock: true, data } =
                device.exchange(Request::DescribePCR { index: i as u16 })?
            else {
                return Err(Error);
            };
            *pcr = data.try_into().map_err(|_| Error)?;
        }
        drop(device);
        Self::open(Policy {
            domain,
            manifest,
            pcrs,
        })
    }
    /// Public actual policy, no secrets. Client acceptance is independent.
    pub fn policy(&self) -> Policy {
        self.policy.clone()
    }
    pub(crate) fn recipient(&self, spki: &[u8], data: [u8; 48]) -> Result<Vec<u8>, Error> {
        if spki.is_empty() || spki.len() > 1024 {
            return Err(Error);
        }
        let mut state = self.state.lock().map_err(|_| Error)?;
        state.time.active()?;
        let start = Instant::now();
        let result = (|| {
            let nonce = entropy(&mut |r| state.device.exchange(r))?;
            let now = state.time.last.ok_or(Error)?;
            let Response::Attestation { document } =
                state.device.exchange(Request::Attestation {
                    nonce: Some(nonce.to_vec().into()),
                    public_key: Some(spki.to_vec().into()),
                    user_data: Some(data.to_vec().into()),
                })?
            else {
                return Err(Error);
            };
            let at =
                attestation::verify_recipient(&document, &self.policy, &nonce, spki, &data, now)?;
            timely(start.elapsed())?;
            Ok((at, document))
        })();
        match result {
            Ok((at, document)) => {
                state.time.observe(Ok(at))?;
                Ok(document)
            }
            Err(e) => {
                state.time.observe(Err(e))?;
                Err(e)
            }
        }
    }
    /// Open `/dev/nsm`, check locked PCR0/1/2 and verify a fresh NSM clock sample.
    /// NSM major version 1/SHA384 is the closed profile; future majors reject.
    pub fn open(policy: Policy) -> Result<Self, Error> {
        Policy::decode(&policy.encode())?;
        let mut device = Device::open()?;
        check_measurements(&mut |request| device.exchange(request), &policy)?;
        let at = clock_sample(
            &mut |request| device.exchange(request),
            &policy,
            attestation::verify_clock,
        )?;
        let mut time = TimeState::default();
        time.observe(Ok(at))?;
        Ok(Self {
            state: Mutex::new(State { device, time }),
            policy,
        })
    }
    /// Exactly 32 bytes obtained from NSM, zeroized on drop; no host RNG fallback.
    /// TLS/library entropy still requires its own image/runtime qualification.
    pub fn random(&self) -> Result<Zeroizing<[u8; 32]>, Error> {
        let mut state = self.state.lock().map_err(|_| Error)?;
        state.time.active()?;
        let result = entropy(&mut |request| state.device.exchange(request));
        if result.is_err() {
            state.time.failed = true;
        }
        result
    }
}
impl Clock for Nsm {
    fn now(&self) -> Result<u64, Error> {
        let mut state = self.state.lock().map_err(|_| Error)?;
        state.time.active()?;
        let result = clock_sample(
            &mut |request| state.device.exchange(request),
            &self.policy,
            attestation::verify_clock,
        );
        state.time.observe(result)
    }
}
impl Attester for Nsm {
    fn quote(&self, policy: &Policy, context: &Context<'_>) -> Result<Vec<u8>, Error> {
        if policy.encode() != self.policy.encode() {
            return Err(Error);
        }
        let request = quote_request(policy, context)?;
        let mut state = self.state.lock().map_err(|_| Error)?;
        state.time.active()?;
        let start = Instant::now();
        let result = (|| {
            let Response::Attestation { document } = state.device.exchange(request)? else {
                return Err(Error);
            };
            if document.is_empty() || document.len() > MAX_QUOTE {
                return Err(Error);
            }
            // A newly generated quote necessarily follows the initial clock
            // cut. Verify that signed timestamp locally without relaxing the
            // public SDK verifier's independently trusted client-clock rules.
            let at = attestation::verify_local(&document, policy, context)?;
            timely(start.elapsed())?;
            Ok((document, at))
        })();
        let (document, at) = result.inspect_err(|_| state.time.failed = true)?;
        state.time.observe(Ok(at))?;
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
    #[test]
    fn clock_request_uses_fresh_entropy_and_separate_purpose_bindings() {
        let p = policy();
        let (key, data) = attestation::clock_binding(&p);
        let mut calls = 0;
        assert_eq!(
            clock_sample(
                &mut |request| {
                    calls += 1;
                    match request {
                        Request::GetRandom => Ok(Response::GetRandom {
                            random: vec![6; 32],
                        }),
                        Request::Attestation {
                            nonce,
                            public_key,
                            user_data,
                        } => {
                            assert_eq!(nonce.unwrap().as_ref(), &[6; 32]);
                            assert_eq!(public_key.unwrap().as_ref(), &key);
                            assert_eq!(user_data.unwrap().as_ref(), &data);
                            Ok(Response::Attestation { document: vec![7] })
                        }
                        _ => panic!("unexpected request"),
                    }
                },
                &p,
                |document, _, nonce| {
                    assert_eq!(document, [7]);
                    assert_eq!(nonce, &[6; 32]);
                    Ok(100)
                },
            )
            .unwrap(),
            100
        );
        assert_eq!(calls, 2);
        assert!(
            clock_sample(&mut |_| Err(Error), &p, |_, _, _| panic!(
                "verification on failed entropy"
            ))
            .is_err()
        );
        let mut calls = 0;
        assert!(
            clock_sample(
                &mut |_| {
                    calls += 1;
                    if calls == 1 {
                        Ok(Response::GetRandom {
                            random: vec![6; 32],
                        })
                    } else {
                        Ok(Response::LockPCR)
                    }
                },
                &p,
                |_, _, _| panic!("verification on wrong response")
            )
            .is_err()
        );
        assert_eq!(calls, 2); // No retry, cached time or alternate clock.
    }
    #[test]
    fn clock_never_moves_backward_or_recovers_a_failed_boot() {
        let mut time = TimeState::default();
        assert_eq!(time.observe(Ok(100)), Ok(100));
        assert_eq!(time.observe(Ok(100)), Ok(100));
        assert_eq!(time.observe(Ok(101)), Ok(101));
        assert!(time.observe(Ok(100)).is_err());
        assert!(time.active().is_err());
        assert!(time.observe(Ok(102)).is_err());
        assert_eq!(time.last, Some(101));
        for result in [Ok(0), Err(Error)] {
            let mut time = TimeState::default();
            assert!(time.observe(result).is_err());
            assert!(time.observe(Ok(100)).is_err());
        }
    }
    #[test]
    fn late_nsm_results_reject_at_the_operation_budget() {
        assert!(timely(Duration::ZERO).is_ok());
        assert!(timely(MAX_NSM_OPERATION).is_ok());
        assert!(timely(MAX_NSM_OPERATION + Duration::from_nanos(1)).is_err());
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
