# ADR 0019: attestation-gated TLS and the runnable private service

Date: 2026-10-01. Status: implemented for offline review, not a qualified Nitro
deployment. Scope: [P19](../implementation/PLAN.md#p19). Economics and authority
remain [BASELINE](../implementation/BASELINE.md) and [P18](0018-private-api.md).

## Protocol and runtime selection

Select **TLS 1.3 terminated in the confidential application**, with fresh Nitro
attestation authenticating the exact carrying connection. The Node automation SDK
uses Node's TLS implementation. The Rust service uses pinned OpenSSL bindings
`0.10.81`, AWS's COSE implementation `0.5.2`, and `serde_cbor 0.11.2`. There is no
handwritten cipher, TLS implementation, wallet-derived encryption or Noise/HPKE
port in JavaScript. Historical Snow experiments remain research, not deployment
approval. This profile does not require browser/WASM cryptography for the first
automation slice. It is **not** an HTTP/REST server or literal venue URL replacement.

The runtime is a bounded synchronous TCP/TLS listener with a small worker limit,
not another server framework. `crates/service` owns transport, verification and
entrypoints; it depends on API/journal/kernel but cannot change financial policy
or sign native actions by itself. The kernel stays dependency-free. The exact
registry graph, build scripts and resolved features are in the dependency policy
and lockfile. AWS COSE brings serialization/macro dependencies; they are included
in that graph, not hidden dependencies or an independent security audit.

OpenSSL is supplied by the build platform (macOS Homebrew; Linux `libssl-dev`).
The Cargo pin does **not** pin the system C library. P20 must lock and record the
enclave image/provider/library versions, entropy source and release manifest;
ordinary macOS CI is not that qualification. TLS early data, tickets and session
resumption are disabled; ALPN is exactly `cinder-private/1`.

The policy argument is a trusted application input, not a host-supplied approval.
P20 must derive/validate the manifest against the **actual** loaded configuration
and key/authority roles before constructing this server. Merely echoing an
expected hash cannot establish that a different runtime policy matches it.

## One connection, one fresh attestation, then private authentication

1. Establish TLS 1.3 with a boot-random enclave-generated leaf key/certificate.
   Normal web-PKI is not the identity authority in this profile. No private
   identity, wallet signature, order, grant or account lookup is sent yet.
2. Derive 32 bytes using TLS exporter label `EXPORTER-Cinder-private-v1` and empty
   context on **this same socket**. Send a fresh random 32-byte client challenge.
3. The enclave obtains a fresh quote binding `public_key` to its leaf DER SPKI,
   `nonce` to the challenge and `user_data` to SHA-384 of the canonical tuple below.
   The public reply contains boot ID, expiry and quote; no account information.
4. The independent client verifier checks the pinned AWS root and full X.509
chain, ES384 COSE signature, nondebug PCR0/1/2, expected manifest/domain, quote
   age, expiry, challenge, SPKI from this socket and exporter from this socket.
5. Both sides derive the same application binding. SDK and service exchange that
   binding over TLS before the SDK releases any private P18 request. Signed P18
   authentication includes the binding, domain, epoch and expiry.

```text
U = SHA384("CINDER-TLS-ATTESTATION-1\0" || network[32] || deployment[32]
           || manifest[32] || boot[32] || expiry_be_u64 || exporter[32])
B = SHA256("CINDER-TLS-SESSION-1\0" || U[48] || challenge[32])
```

There is no verify-on-socket-A/use-socket-B step, cached approval, identity-bearing
0-RTT or plaintext fallback. Quote failure destroys the connection. Every private
success, redacted error and view travels inside this same TLS session. Views are
full snapshots, not an unbounded subscription/stream protocol. Reconnection needs
a new quote and new signed envelope; immutable economic IDs remain unchanged.
The SDK never retries an uncertain native operation automatically.

### Strict quote profile

Accept tagged Sign1 tag 18 or untagged Sign1, exactly four elements, protected
header `{1: -35}`, empty unprotected header, embedded payload and 96-byte signature.
Reject trailing bytes, indefinite/nonminimal encodings, extra fields, duplicate
payload/PCR keys and wrong types **before** upstream deserialization. Payload has
the nine Nitro fields; all three optional binding fields are mandatory here.
Digest is SHA384; PCRs are unique indices 0–31 and exactly 48 bytes each.
PCR0/1/2 must match the independently selected, nonzero release values.

Bound quote to 16 KiB, certificates/SPKI to 1 KiB each, CA bundle to eight entries,
module ID to 256 bytes, user data to 512 bytes and challenge to 32 bytes. Certificate
DER round-trips exactly; the published root's DER SHA-256 fingerprint is checked.
OpenSSL strict path validation enforces signatures, CA/key-usage/path constraints
and certificate lifetime, with explicit minimum 128-bit certificate/key strength;
the root is also explicitly lifetime-checked. CRL checks
are disabled as in AWS's attestation profile. Client clock is independently
trusted; quote age is at most 30 seconds, with no future timestamp tolerance.
Expiry is future, no later than quote time plus 120 seconds. Client clock skew
causes rejection, never an automatic weakened validation mode.

## Client distribution and rotation

`clients/private/src/node-channel.ts` supplies `AttestedNodeChannel`. Its trusted
`nativeQuoteVerifier(absolutePath)` invokes independently distributed
`cinder-verify-quote` with bounded **public-only** input. No shell, remote helper
selection, private key or order enters that subprocess. Verification has a five
second deadline and bounded output. The caller must trust the local executable,
policy and SDK; an endpoint cannot supply them. The explicit `QuoteVerifier` port
is trusted code, not a way to accept a relay-provided approval boolean.

Production verifier has no alternate-root or skip-verification flags, including
in an all-features build. Fixture verification has a different executable name
and requires `local-fixture`; default Cargo features are empty. The workspace guard
checks both feature and entrypoint boundaries. Real NSM/key-release providers are
not invented here and no production confidential-service binary silently selects
the fixture implementation. P20 supplies that measured application entrypoint.

Boot keys rotate on restart; reconnect reattests, without restoring a TLS private
key from host storage. Release/PCR/root policy changes require independently
distributed reviewed client policy/executable updates, not a server redirect.
This initial policy accepts one exact release; coordinated multi-release windows
need explicit policy versioning. Publishing a verifier does not protect a user
from malicious website JavaScript or a compromised local SDK. Browser transport,
frontend release integrity and onboarding/agent UX remain separate work.

## Resource and parent boundary

The service admits at most eight sessions, 128 requests per session, 1 KiB private
requests and 1 MiB private replies. All frames have an exact u32 BE length. The
client permits one pending exchange, rejects concurrent calls and caps buffering
at 4,096 chunks; there is no unbounded queue. Handshake/verification/framing budgets
are five seconds; socket I/O has a five-second timeout and a separate lifetime
watchdog closes stalled sockets. A frame finishing after its deadline rejects.
An in-progress blocking syscall can delay detecting its frame deadline by at most
one socket timeout; the full connection is still forcibly bounded. Application
ports must not introduce unbounded blocking; P20 qualifies actual NSM/storage and
egress deadlines. Atomic journal outcomes remain uncertain if a connection dies
after commitment, not canceled because the caller disconnected.

The local relay only connects to one explicitly configured loopback socket. It
forwards opaque bytes in bounded 16 KiB buffers, eight connections, at most 4 MiB
per direction and a bounded lifetime. It has no destination supplied by a private
request and no HTTP parsing/termination. P20 adds the actual vsock topology. Stdin
EOF shuts down local slice entrypoints and closes active sockets. Fixed startup
ports and redacted failure messages are the only entrypoint output.

Parent-visible metadata includes network endpoints, TLS handshake/ALPN, connection
timing, ciphertext sizes, object hashes/accepted sequence and availability failures.
No traffic padding, identity-hiding transfer system or immunity to traffic analysis
is claimed. Quotes/release measurements are public protocol data. Customer API
identities, signatures, orders, balances, views and financial errors are absent
from relay traffic plaintext and ciphertext-replica storage.

## Local evidence and explicit limitations

Separate processes run `cinder-service-fixture`, `cinder-relay`, a public-data quote
verifier and the SDK. Fixture service composes the real P18 handler with P06
XChaCha20-Poly1305, two ciphertext file replicas and a **local synthetic witness**.
One authorized order prepares/exposes one journaled fixture attempt, not a venue
transaction. Actual process death/restart, exact retry, changed-economics conflict,
account queries, confidentiality checks, wrong root/release/exporter, quote replay,
downgrade, plaintext rejection, bounds, witness loss and rollback refusal are tested.

Fixture accounts, source/risk/mark policies, CA, clock and witness are synthetic.
The journal key enters via trusted test-harness stdin and is retained only by that
harness for restart, never written to parent replica storage. The fixture startup
line includes its synthetic CA and its CLI takes a disposable owner's public key;
neither is a production onboarding/configuration path. A normal local process is
not a TEE: host memory inspection, stdin observation, a rollback of both replicas
**and** local witness, and hardware failures are not defeated by this harness.
P20 must supply real enclave isolation, NSM quotes, secure entropy/time, key release,
independently authenticated fresh witness, durable storage and enclave-side venue
TLS. Passing this slice does not close G01–G05, imply an AWS-signed session was
obtained, guarantee solvency or authorize real funds/deployment.

## Primary sources and promoted research

Checked 2026-10-01: [AWS attestation validation](https://docs.aws.amazon.com/enclaves/latest/user/verify-root.html),
[NSM attestation process](https://github.com/aws/aws-nitro-enclaves-nsm-api/blob/main/docs/attestation_process.md),
[AWS COSE implementation](https://docs.rs/aws-nitro-enclaves-cose/0.5.2/aws_nitro_enclaves_cose/),
[OpenSSL Rust TLS API](https://docs.rs/openssl/0.10.81/openssl/ssl/index.html).
The tracked AWS root PEM is public verification material, not a private key.
Its published DER fingerprint is checked in a tracked unit test; trust is not
learned from the peer's bundled root.

W06 private API runtime, W07 D14 verification and W08 Nitro qualification lessons
promote key-binding, no-early-identity, authenticated persistence, client-distribution
and parent-boundary requirements. Historical experimental implementations, keys,
private audits and ignored research files are not build/test dependencies. New
tests use freshly generated synthetic evidence rather than vendoring audit code.
