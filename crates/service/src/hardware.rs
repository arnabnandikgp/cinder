//! Explicit hardware-only tests: never run by normal CI and never linked into
//! the shipping enclave. Use a separately approved measured test image, fresh
//! disposable KMS roles and bootstrap. No venue HTTP request or financial mutation.
use super::*;
use crate::{
    transport::read_frame,
    vsock::{Target, VsockStream},
};
use std::{
    io::{self, Read, Write},
    net::Shutdown,
};

struct Environment {
    manifest: Manifest,
    nsm: Arc<Nsm>,
    bootstrap: Bootstrap,
}
impl Environment {
    fn open() -> Result<Self, Error> {
        // Fixed PUBLIC inputs, not paths into trusted private preparation files.
        let mut bytes = Vec::new();
        std::fs::File::open("/etc/cinder/manifest.cbor")?
            .take(131073)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 131072 {
            return Err(Error);
        }
        let manifest: Manifest = serde_cbor::from_slice(&bytes).map_err(|_| Error)?;
        manifest.validate()?;
        // As in the application, PCRs cannot be embedded in their own measured
        // image. The trusted operator independently approves exact test-image
        // PCRs in all five KMS policies. Only actual positive key release below
        // establishes that approval; discovery alone does not.
        let nsm = Arc::new(Nsm::discover(
            manifest.domain.as_slice().try_into().map_err(|_| Error)?,
            manifest.digest()?,
        )?);
        let mut socket = VsockStream::connect(Target::new(3, manifest.bootstrap)?)?;
        let bytes = read_frame(&mut socket, 65536)?;
        let bootstrap: Bootstrap = serde_cbor::from_slice(&bytes).map_err(|_| Error)?;
        if bootstrap.capsules.len() != 5
            || manifest.slots.iter().any(|slot| {
                bootstrap
                    .capsules
                    .iter()
                    .filter(|c| c.role == slot.role)
                    .count()
                    != 1
            })
            || bootstrap
                .capsules
                .iter()
                .any(|c| c.ciphertext.is_empty() || c.ciphertext.len() > 6144)
        {
            return Err(Error);
        }
        Ok(Self {
            manifest,
            nsm,
            bootstrap,
        })
    }
    fn client(&self, endpoint: crate::cloud::Endpoint) -> Result<Client, Error> {
        let c = &self.bootstrap.parent;
        Client::new(
            endpoint,
            Credential {
                access: c.access.clone(),
                secret: c.secret.clone(),
                token: c.token.clone(),
                expires: c.expires,
            },
            self.nsm.clone(),
        )
    }
    fn request(&self, role: Role) -> Result<(Recipient, Value), Error> {
        let slot = self
            .manifest
            .slots
            .iter()
            .find(|s| s.role == role)
            .ok_or(Error)?;
        let capsule = self
            .bootstrap
            .capsules
            .iter()
            .find(|c| c.role == role)
            .ok_or(Error)?;
        let recipient = Recipient::new()?;
        let data = sha384(
            &[
                b"CINDER-KMS-RECIPIENT-1\0".as_slice(),
                &self.manifest.digest()?,
                role.name().as_bytes(),
                &sha256(&capsule.ciphertext),
            ]
            .concat(),
        );
        let document = self.nsm.recipient(&recipient.public()?, data)?;
        Ok((
            recipient,
            json!({
                "KeyId":slot.endpoint.resource, "EncryptionAlgorithm":"SYMMETRIC_DEFAULT",
                "CiphertextBlob":STANDARD.encode(&capsule.ciphertext),
                "EncryptionContext":self.manifest.context(role)?,
                "Recipient":{"KeyEncryptionAlgorithm":"RSAES_OAEP_SHA_256", "AttestationDocument":STANDARD.encode(document)}
            }),
        ))
    }
    fn body(&self, role: Role) -> Result<Zeroizing<Vec<u8>>, Error> {
        let slot = self
            .manifest
            .slots
            .iter()
            .find(|s| s.role == role)
            .ok_or(Error)?;
        let (recipient, request) = self.request(role)?;
        let (status, response) = self
            .client(slot.endpoint.clone())?
            .qualification_json("TrentService.Decrypt", request)?;
        if status != 200 {
            return Err(Error);
        }
        // Actual KMS CMS response must not open under an unrelated fresh key.
        if Recipient::new()?
            .decrypt(&response, &slot.endpoint.resource)
            .is_ok()
        {
            return Err(Error);
        }
        let plain = recipient.decrypt(&response, &slot.endpoint.resource)?;
        self.manifest.unwrap(role, plain)
    }
    fn positive(&self, role: Role) -> Result<(), Error> {
        drop(self.body(role)?);
        Ok(())
    }
    fn denied(&self, role: Role, mutate: impl FnOnce(&mut Value), code: &str) -> Result<(), Error> {
        self.matrix_denied(role, mutate, code).map_err(|_| Error)
    }
    fn matrix_denied(
        &self,
        role: Role,
        mutate: impl FnOnce(&mut Value),
        code: &str,
    ) -> Result<(), &'static str> {
        let result = (|| {
            let slot = self
                .manifest
                .slots
                .iter()
                .find(|s| s.role == role)
                .ok_or(Error)?;
            let (_recipient, mut request) = self.request(role)?;
            mutate(&mut request);
            let (status, response) = self
                .client(slot.endpoint.clone())?
                .qualification_json("TrentService.Decrypt", request)?;
            Ok::<_, Error>((status, response))
        })();
        let (status, response) = result.map_err(|_| "transport-or-local-refusal")?;
        // Only these fixed error classes enter a public diagnostic. Never copy
        // an AWS message, arbitrary response body, identifiers or key material.
        if expected_denial(status, &response, code) {
            return Ok(());
        }
        Err(
            if expected_denial(status, &response, "AccessDeniedException") {
                "unexpected-access-denied"
            } else if expected_denial(status, &response, "IncorrectKeyException") {
                "unexpected-incorrect-key"
            } else if expected_denial(status, &response, "InvalidCiphertextException") {
                "unexpected-invalid-ciphertext"
            } else {
                "unexpected-response"
            },
        )
    }
}

fn expected_denial(status: u16, response: &Value, code: &str) -> bool {
    matches!(status, 400 | 403)
        && response.get("CiphertextForRecipient").is_none()
        && response.get("Plaintext").is_none()
        && response
            .get("__type")
            .and_then(Value::as_str)
            .and_then(|s| s.rsplit('#').next())
            == Some(code)
}

#[test]
fn denial_classification_requires_specific_error_without_release_material() {
    for error in [
        "AccessDeniedException",
        "com.amazonaws.kms#AccessDeniedException",
    ] {
        let response = json!({"__type":error});
        assert!(expected_denial(400, &response, "AccessDeniedException"));
        assert!(expected_denial(403, &response, "AccessDeniedException"));
        for status in [0, 200, 302, 429, 500, 503] {
            assert!(!expected_denial(status, &response, "AccessDeniedException"));
        }
        for field in ["Plaintext", "CiphertextForRecipient"] {
            for value in [Value::Null, json!(""), json!("unexpected")] {
                let mut changed = response.clone();
                changed[field] = value;
                assert!(!expected_denial(400, &changed, "AccessDeniedException"));
            }
        }
    }
    for response in [
        json!({}),
        json!({"__type":null}),
        json!({"__type":"ExpiredTokenException"}),
        json!({"__type":"IncorrectKeyException"}),
        json!({"__type":"NotFoundException"}),
    ] {
        assert!(!expected_denial(400, &response, "AccessDeniedException"));
    }
}

fn run_recipient_matrix(env: &Environment) -> Result<(), String> {
    for slot in &env.manifest.slots {
        let role = slot.role;
        let failed =
            |case: &str, reason: &str| format!("recipient/{}/{case}/{reason}", role.name());
        env.positive(role)
            .map_err(|_| failed("before", "release-refused"))?;
        env.matrix_denied(
            role,
            |v| {
                v.as_object_mut().unwrap().remove("Recipient");
            },
            "AccessDeniedException",
        )
        .map_err(|e| failed("missing-recipient", e))?;
        for field in [
            "cinder-release",
            "cinder-role",
            "cinder-generation",
            "cinder-stream",
        ] {
            env.matrix_denied(
                role,
                |v| {
                    v["EncryptionContext"][field] = json!("not-approved");
                },
                "AccessDeniedException",
            )
            .map_err(|e| failed(field, e))?;
        }
        env.matrix_denied(
            role,
            |v| {
                v.as_object_mut().unwrap().remove("EncryptionContext");
            },
            "AccessDeniedException",
        )
        .map_err(|e| failed("missing-context", e))?;
        // A different role's ciphertext must not decrypt with this role's key.
        // In this exact-context topology, AWS denies access to the ciphertext's
        // originating key before the IncorrectKey check (observed on Nitro).
        let other = env
            .bootstrap
            .capsules
            .iter()
            .find(|c| c.role != role)
            .ok_or_else(|| failed("wrong-role", "missing-capsule"))?;
        env.matrix_denied(
            role,
            |v| {
                v["CiphertextBlob"] = json!(STANDARD.encode(&other.ciphertext));
            },
            "AccessDeniedException",
        )
        .map_err(|e| failed("wrong-role", e))?;
        env.positive(role)
            .map_err(|_| failed("after", "release-refused"))?;
    }
    Ok(())
}

#[test]
#[ignore = "explicit approved Nitro-only qualification; no ambient credentials or native actions"]
fn hardware_recipient_rejection_matrix() {
    // Never print response bodies, keys, credentials or projection on failure.
    let env = Environment::open().expect("hardware bootstrap refused");
    let result = run_recipient_matrix(&env);
    assert!(
        report(
            &env,
            result
                .as_ref()
                .err()
                .map_or("recipient-matrix", String::as_str),
            result.is_ok()
        )
        .is_ok(),
        "hardware receipt refused"
    );
    assert!(result.is_ok(), "hardware recipient matrix refused");
}

fn run_witness_race(env: &Environment) -> Result<(), Error> {
    use crate::cloud::{DynamoWitness, qualification_cas_request};
    use cinder_journal::{
        Head,
        replicated::{Stream, Witness},
    };
    use cinder_kernel::identity::{DeploymentId, Domain, NetworkId};
    use std::sync::Barrier;

    let bytes = env.body(Role::Witness)?;
    let credential: Credential = serde_cbor::from_slice(&bytes).map_err(|_| Error)?;
    drop(bytes);
    let client = || {
        Client::new(
            env.manifest.witness.clone(),
            Credential {
                access: credential.access.clone(),
                secret: credential.secret.clone(),
                token: credential.token.clone(),
                expires: credential.expires,
            },
            env.nsm.clone(),
        )
    };
    let stream = Stream {
        domain: Domain {
            network: NetworkId::new(env.manifest.domain[..32].try_into().map_err(|_| Error)?)
                .map_err(|_| Error)?,
            deployment: DeploymentId::new(env.manifest.domain[32..].try_into().map_err(|_| Error)?)
                .map_err(|_| Error)?,
        },
        id: env.manifest.stream,
    };
    let mut observer = DynamoWitness::new(client()?, stream)?;
    let before = observer.read(stream).map_err(|_| Error)?;
    // Only a freshly provisioned disposable REGISTER: no financial ledger and
    // no accepted application history may be adopted or reset by this test.
    if before.epoch != env.manifest.epoch || before.head.is_some() {
        return Err(Error);
    }
    let heads = [
        Head {
            sequence: 0,
            hash: *env.nsm.random()?,
        },
        Head {
            sequence: 0,
            hash: *env.nsm.random()?,
        },
    ];
    if heads[0].hash == heads[1].hash {
        return Err(Error);
    }
    let barrier = Arc::new(Barrier::new(2));
    let first = client()?;
    let second = client()?;
    let request =
        |head| qualification_cas_request(&env.manifest.witness.resource, stream, before, head);
    let first_request = request(heads[0])?;
    let second_request = request(heads[1])?;
    let replies = std::thread::scope(|scope| {
        let perform = |mut c: Client, body| {
            barrier.wait();
            c.qualification_json("DynamoDB_20120810.UpdateItem", body)
        };
        let a = scope.spawn(move || perform(first, first_request));
        let b = scope.spawn(move || perform(second, second_request));
        Ok::<_, Error>([a.join().map_err(|_| Error)??, b.join().map_err(|_| Error)??])
    })?;
    let successes = replies
        .iter()
        .filter(|(status, body)| *status == 200 && body.as_object().is_some_and(|v| v.is_empty()))
        .count();
    let denials = replies
        .iter()
        .filter(|(status, body)| {
            *status == 400 && expected_denial(*status, body, "ConditionalCheckFailedException")
        })
        .count();
    if successes != 1 || denials != 1 {
        return Err(Error);
    }
    let winner = usize::from(replies[1].0 == 200);
    let after = observer.read(stream).map_err(|_| Error)?;
    if after.epoch != before.epoch || after.head != Some(heads[winner]) {
        return Err(Error);
    }
    let (status, response) = client()?
        .qualification_json("DynamoDB_20120810.UpdateItem", request(heads[1 - winner])?)?;
    if status != 400
        || !expected_denial(status, &response, "ConditionalCheckFailedException")
        || observer.read(stream).map_err(|_| Error)? != after
    {
        return Err(Error);
    }
    Ok(())
}

#[test]
#[ignore = "explicit Nitro-only test; requires a fresh empty disposable witness, never an application head"]
fn hardware_competing_witness_writers() {
    assert!(
        Environment::open()
            .and_then(|env| run_witness_race(&env))
            .is_ok(),
        "hardware witness race refused"
    );
}

fn run_journal_faults(env: &Environment) -> Result<(), String> {
    use crate::cloud::{DynamoWitness, S3Replica};
    use cinder_journal::{
        Journal, Protection, RecordContext,
        encrypted::RecordCipher,
        model::{CommitId, PrivateBytes, Transaction},
        replicated::{Anchor, Replica, Replicated, Stream, Witness},
        wire,
    };
    use cinder_kernel::identity::{DeploymentId, Domain, NetworkId};
    use std::sync::{
        Mutex,
        atomic::{AtomicU8, Ordering},
    };

    // This wrapper is only in the separately measured test executable. It
    // either refuses BEFORE real CAS or suppresses acknowledgement AFTER real
    // AWS acceptance. It is not a claim of a physical network-drop experiment.
    struct FaultWitness {
        inner: DynamoWitness,
        fault: Arc<AtomicU8>,
        orphan: Arc<Mutex<Option<cinder_journal::Head>>>,
    }
    impl Witness for FaultWitness {
        fn read(&mut self, s: Stream) -> Result<Anchor, cinder_journal::Error> {
            self.inner.read(s)
        }
        fn accept(
            &mut self,
            s: Stream,
            a: Anchor,
            h: cinder_journal::Head,
        ) -> Result<(), cinder_journal::Error> {
            match self.fault.swap(0, Ordering::SeqCst) {
                1 => {
                    *self
                        .orphan
                        .lock()
                        .map_err(|_| cinder_journal::Error::Storage)? = Some(h);
                    Err(cinder_journal::Error::Storage)
                }
                2 => {
                    self.inner.accept(s, a, h)?;
                    Err(cinder_journal::Error::Storage)
                }
                _ => self.inner.accept(s, a, h),
            }
        }
    }
    let mut stage = "configuration";
    let result = (|| {
        let config: crate::runtime::Configuration =
            serde_cbor::from_slice(&env.body(Role::Configuration)?).map_err(|_| Error)?;
        let config = wire::decode_config(&config.ledger).map_err(|_| Error)?;
        let stream = Stream {
            domain: Domain {
                network: NetworkId::new(env.manifest.domain[..32].try_into().map_err(|_| Error)?)
                    .map_err(|_| Error)?,
                deployment: DeploymentId::new(
                    env.manifest.domain[32..].try_into().map_err(|_| Error)?,
                )
                .map_err(|_| Error)?,
            },
            id: env.manifest.stream,
        };
        if config.domain != stream.domain {
            return Err(Error);
        }
        let storage = env.body(Role::Storage)?;
        let storage: [u8; 32] = storage.as_slice().try_into().map_err(|_| Error)?;
        let storage = Zeroizing::new(storage);
        let credential: Credential =
            serde_cbor::from_slice(&env.body(Role::Witness)?).map_err(|_| Error)?;
        let witness = || {
            DynamoWitness::new(
                Client::new(
                    env.manifest.witness.clone(),
                    Credential {
                        access: credential.access.clone(),
                        secret: credential.secret.clone(),
                        token: credential.token.clone(),
                        expires: credential.expires,
                    },
                    env.nsm.clone(),
                )?,
                stream,
            )
        };
        let first = || S3Replica::new(env.client(env.manifest.first.clone())?, stream);
        let second = || S3Replica::new(env.client(env.manifest.second.clone())?, stream);
        let mut observer = witness()?;
        let initial = observer.read(stream).map_err(|_| Error)?;
        if initial.epoch != env.manifest.epoch || initial.head.is_some() {
            return Err(Error);
        }
        let backend = || {
            Replicated::new(stream, initial.epoch, first()?, second()?, witness()?)
                .map_err(|_| Error)
        };
        let cipher = || {
            RecordCipher::new(storage.clone(), env.manifest.generation, stream.id)
                .map_err(|_| Error)
        };
        let fault = Arc::new(AtomicU8::new(0));
        let orphan = Arc::new(Mutex::new(None));
        let wrapped = Replicated::new(
            stream,
            initial.epoch,
            first()?,
            second()?,
            FaultWitness {
                inner: witness()?,
                fault: fault.clone(),
                orphan: orphan.clone(),
            },
        )
        .map_err(|_| Error)?;
        stage = "genesis";
        let mut store = Journal::create(wrapped, cipher()?, config.clone()).map_err(|_| Error)?;
        let genesis = store.head();
        let stale_snapshot = backend()?.snapshot().map_err(|_| Error)?;
        let tx = || {
            Ok::<_, Error>(Transaction {
                id: CommitId::new(*env.nsm.random()?).map_err(|_| Error)?,
                expected: genesis,
                at: env.nsm.now()?,
                evidence: vec![
                    PrivateBytes::new(b"disposable qualification, no financial action".to_vec())
                        .map_err(|_| Error)?,
                ],
                inputs: vec![],
                order_observations: vec![],
                funds_observations: vec![],
                controls: vec![],
            })
        };
        stage = "orphan";
        let refused = tx()?;
        fault.store(1, Ordering::SeqCst);
        if store.commit(refused.clone()).err() != Some(cinder_journal::Error::Storage)
            || store.state().err() != Some(cinder_journal::Error::Poisoned)
            || observer.read(stream).map_err(|_| Error)?.head != Some(genesis)
        {
            return Err(Error);
        }
        let orphan = orphan.lock().map_err(|_| Error)?.ok_or(Error)?;
        let a = first()?.get(orphan.hash).map_err(|_| Error)?;
        let b = second()?.get(orphan.hash).map_err(|_| Error)?;
        if a != b || a.head != orphan || a.previous != genesis.hash {
            return Err(Error);
        }
        drop(
            cipher()?
                .open(
                    RecordContext {
                        domain: stream.domain,
                        sequence: orphan.sequence,
                        previous: genesis.hash,
                    },
                    &a.opaque,
                )
                .map_err(|_| Error)?,
        );
        stage = "orphan-reload";
        store.reload().map_err(|_| Error)?;
        if store.head() != genesis || store.transaction(refused.id).is_some() {
            return Err(Error);
        }
        // Actual acceptance happens, but the caller gets Storage and must reconcile.
        stage = "lost-ack";
        let accepted = tx()?;
        fault.store(2, Ordering::SeqCst);
        if store.commit(accepted.clone()).err() != Some(cinder_journal::Error::Storage)
            || store.state().err() != Some(cinder_journal::Error::Poisoned)
        {
            return Err(Error);
        }
        let anchor = observer.read(stream).map_err(|_| Error)?;
        if anchor.epoch != initial.epoch || anchor.head.ok_or(Error)?.sequence != 1 {
            return Err(Error);
        }
        stage = "accepted-reload";
        store.reload().map_err(|_| Error)?;
        if Some(store.head()) != anchor.head
            || store.transaction(accepted.id) != Some(&accepted)
            || store.transaction(refused.id).is_some()
        {
            return Err(Error);
        }
        stage = "duplicate";
        let duplicate = store.commit(accepted).map_err(|_| Error)?;
        if !duplicate.duplicate
            || !duplicate.exposures.is_empty()
            || observer.read(stream).map_err(|_| Error)? != anchor
        {
            return Err(Error);
        }
        stage = "stale-snapshot";
        if backend()?.restore_snapshot(&stale_snapshot).is_ok()
            || observer.read(stream).map_err(|_| Error)? != anchor
        {
            return Err(Error);
        }
        stage = "current-snapshot";
        let current_snapshot = backend()?.snapshot().map_err(|_| Error)?;
        stage = "current-restore";
        backend()?
            .restore_snapshot(&current_snapshot)
            .map_err(|_| Error)?;
        stage = "restart";
        let mut restarted = Journal::open(backend()?, cipher()?, config).map_err(|_| Error)?;
        if Some(restarted.head()) != anchor.head
            || restarted.transactions().count() != 1
            || restarted.verified_state().is_err()
        {
            return Err(Error);
        }
        Ok::<(), Error>(())
    })();
    result.map_err(|_| format!("journal/{stage}"))
}
#[test]
#[ignore = "Nitro-only real encrypted journal; fresh disposable stream, injected witness-ack faults, no financial actions"]
fn hardware_journal_orphan_stale_and_uncertain_commit() {
    assert!(
        Environment::open()
            .and_then(|env| run_journal_faults(&env).map_err(|_| Error))
            .is_ok(),
        "hardware journal fault matrix refused"
    );
}

// Mutate exactly the first payload byte of a TLS application-data record. In
// TLS 1.3 this includes the encrypted handshake, not the plaintext ServerHello.
// Parsing survives arbitrary read fragmentation; it does not inspect plaintext.
#[derive(Default)]
struct RecordFault {
    header: [u8; 5],
    at: usize,
    remaining: usize,
    encrypted: bool,
    changed: bool,
}
impl RecordFault {
    fn apply(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            if self.changed {
                return;
            }
            if self.remaining > 0 {
                if self.encrypted {
                    *byte ^= 1;
                    self.changed = true;
                }
                self.remaining -= 1;
            } else {
                self.header[self.at] = *byte;
                self.at += 1;
                if self.at == self.header.len() {
                    self.remaining = u16::from_be_bytes([self.header[3], self.header[4]]) as usize;
                    self.encrypted = self.header[0] == 23;
                    self.at = 0;
                }
            }
        }
    }
}
struct FaultSocket {
    socket: VsockStream,
    fault: Option<RecordFault>,
}
impl Read for FaultSocket {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let n = self.socket.read(bytes)?;
        if let Some(fault) = &mut self.fault {
            fault.apply(&mut bytes[..n]);
        }
        Ok(n)
    }
}
impl Write for FaultSocket {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.socket.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.socket.flush()
    }
}
impl crate::transport::Socket for FaultSocket {
    fn idle(&self) -> io::Result<()> {
        self.socket.idle()
    }
    fn prepare(&self) -> io::Result<()> {
        self.socket.prepare()
    }
    fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            socket: self.socket.try_clone()?,
            fault: None,
        })
    }
    fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.socket.shutdown(how)
    }
}

#[test]
fn tls_fault_changes_encrypted_payload_only_across_all_fragmentations() {
    let original = [
        22, 3, 3, 0, 2, 7, 8, 20, 3, 3, 0, 1, 1, 23, 3, 3, 0, 3, 9, 10, 11,
    ];
    for chunk in 1..=original.len() {
        let mut bytes = original;
        let mut fault = RecordFault::default();
        for part in bytes.chunks_mut(chunk) {
            fault.apply(part);
        }
        assert!(fault.changed);
        let mut expected = original;
        expected[18] ^= 1;
        assert_eq!(bytes, expected);
    }
    // Empty records and partial headers do not invent a payload byte to alter.
    let mut bytes = [23, 3, 3, 0, 0, 22, 3];
    let original = bytes;
    let mut fault = RecordFault::default();
    fault.apply(&mut bytes);
    assert_eq!(bytes, original);
    assert!(!fault.changed);
}

fn upstream_tls(
    env: &Environment,
    trust: &crate::egress::Trust,
    hostname: &str,
    corrupt: bool,
) -> Result<(bool, bool), Error> {
    use crate::transport::{Clock, Lifetime, Socket};
    let socket = FaultSocket {
        socket: VsockStream::connect(Target::new(3, env.manifest.venue_port)?)?,
        fault: corrupt.then(RecordFault::default),
    };
    let _lifetime = Lifetime::new(socket.try_clone()?);
    let mut configuration = trust.connector()?.configure()?;
    configuration
        .param_mut()
        .set_time(i64::try_from(env.nsm.now()? / 1000).map_err(|_| Error)?);
    // Preserve the real SNI/route even for the wrong-host verification test;
    // a server rejecting an unknown SNI is not a hostname-check receipt.
    let mut ssl = configuration.into_ssl(crate::egress::host(
        cinder_pacifica::execution::Origin::Testnet,
    ))?;
    ssl.param_mut().set_host(hostname)?;
    // No HTTP bytes, native credentials or action signing are ever sent.
    match ssl.connect(socket) {
        Ok(mut tls) => {
            let okay = tls.ssl().version_str() == "TLSv1.3"
                && !tls.ssl().session_reused()
                && tls
                    .ssl()
                    .selected_alpn_protocol()
                    .is_none_or(|p| p == b"http/1.1");
            let _ = tls.shutdown();
            Ok((okay, false))
        }
        Err(openssl::ssl::HandshakeError::Failure(failed)) => {
            let changed = failed.get_ref().fault.as_ref().is_some_and(|f| f.changed);
            // Outage, timeout and missing routes are not CA/hostname evidence.
            let invalid = failed.ssl().verify_result() != openssl::x509::X509VerifyResult::OK;
            let protocol = failed.error().code() == openssl::ssl::ErrorCode::SSL;
            Ok((false, protocol && if corrupt { changed } else { invalid }))
        }
        Err(_) => Err(Error),
    }
}
fn run_upstream_tls_matrix(env: &Environment) -> Result<(), Error> {
    env.positive(Role::Configuration)?;
    let trust =
        crate::egress::Trust::from_der(&env.manifest.venue_root, env.manifest.venue_root_hash)?;
    let host = crate::egress::host(cinder_pacifica::execution::Origin::Testnet);
    // AWS and Pacifica may share an ordinary Internet CA. Use the distinct
    // Nitro attestation root, which must not authenticate venue HTTPS.
    let wrong_der = openssl::x509::X509::from_pem(include_bytes!("aws-root.pem"))?.to_der()?;
    if wrong_der == env.manifest.venue_root {
        return Err(Error);
    }
    let wrong = crate::egress::Trust::from_der(&wrong_der, sha256(&wrong_der))?;
    for (root, hostname, corrupt) in [
        (&wrong, host, false),
        (&trust, "not-approved.invalid", false),
        (&trust, host, true),
    ] {
        if !upstream_tls(env, &trust, host, false)?.0 {
            return Err(Error);
        }
        if upstream_tls(env, root, hostname, corrupt)? != (false, true) {
            return Err(Error);
        }
        if !upstream_tls(env, &trust, host, false)?.0 {
            return Err(Error);
        }
    }
    Ok(())
}
#[test]
#[ignore = "explicit Nitro-only TLS handshakes; fixed Pacifica paper origin, no HTTP/actions"]
fn hardware_upstream_tls_rejection_matrix() {
    assert!(
        Environment::open()
            .and_then(|env| run_upstream_tls_matrix(&env))
            .is_ok(),
        "hardware upstream TLS matrix refused"
    );
}

// Non-debug enclaves do not expose a console. A process exiting is not a PASS
// receipt: expose only a fixed verdict over the same real attested TLS channel.
// This TEST-ONLY handler cannot read a financial projection or sign an action.
fn report(env: &Environment, name: &str, success: bool) -> Result<(), Error> {
    use crate::transport::{Handler, Identity, Server, Session};
    use cinder_journal::model::PrivateBytes;
    use std::{
        sync::atomic::{AtomicBool, Ordering},
        time::Duration,
    };
    struct Verdict {
        name: String,
        success: bool,
    }
    impl Handler for Verdict {
        fn handle(
            &self,
            _channel: &Session,
            request: PrivateBytes,
            _now: u64,
        ) -> Result<PrivateBytes, Error> {
            if request.as_bytes() != b"qualification/result" {
                return Err(Error);
            }
            PrivateBytes::new(
                format!(
                    "CINDER-P20-RESULT-1\0{}\0{}",
                    self.name,
                    if self.success { "PASS" } else { "FAIL" }
                )
                .into_bytes(),
            )
            .map_err(|_| Error)
        }
    }
    let stop = Arc::new(AtomicBool::new(false));
    let server = Server::new(
        Identity::generate(env.nsm.as_ref())?,
        env.nsm.policy(),
        env.nsm.clone(),
        env.nsm.clone(),
        Arc::new(Verdict {
            name: name.to_owned(),
            success,
        }),
    )?;
    let listener = crate::vsock::VsockListener::bind(env.manifest.ingress, 3)?;
    let deadline = stop.clone();
    let worker = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(120));
        deadline.store(true, Ordering::SeqCst);
    });
    let result = server.run_vsock(listener, stop);
    worker.join().map_err(|_| Error)?;
    result
}

#[test]
#[ignore = "actual Nitro/KMS/TLS/encrypted-journal qualification, fresh stream, no financial or native actions"]
fn hardware_security_batch() {
    let env = Environment::open().expect("hardware bootstrap refused");
    let (name, result) = match run_recipient_matrix(&env) {
        Err(e) => (e, Err(Error)),
        Ok(()) => match run_upstream_tls_matrix(&env) {
            Err(e) => ("security-batch/upstream-tls".to_owned(), Err(e)),
            Ok(()) => match run_journal_faults(&env) {
                Err(e) => (e, Err(Error)),
                Ok(()) => ("security-batch".to_owned(), Ok(())),
            },
        },
    };
    assert!(
        report(&env, &name, result.is_ok()).is_ok(),
        "hardware receipt refused"
    );
    assert!(result.is_ok(), "hardware security batch refused");
}
#[test]
#[ignore = "actual Nitro CAS race and attested verdict, separate empty disposable register"]
fn hardware_witness_race_receipt() {
    let env = Environment::open().expect("hardware bootstrap refused");
    let result = run_witness_race(&env);
    assert!(
        report(&env, "witness-race", result.is_ok()).is_ok(),
        "hardware receipt refused"
    );
    assert!(result.is_ok(), "hardware witness race refused");
}
#[test]
#[ignore = "Nitro-only wrong-measurement probe: unchanged valid contexts/capsules, KMS approves a different EIF"]
fn hardware_unapproved_measurement_receipt() {
    let env = Environment::open().expect("hardware bootstrap refused");
    let result = (|| {
        for slot in &env.manifest.slots {
            env.denied(slot.role, |_| {}, "AccessDeniedException")?;
        }
        Ok::<_, Error>(())
    })();
    assert!(
        report(&env, "unapproved-measurement", result.is_ok()).is_ok(),
        "hardware receipt refused"
    );
    assert!(
        result.is_ok(),
        "hardware unapproved measurement release refused"
    );
}
