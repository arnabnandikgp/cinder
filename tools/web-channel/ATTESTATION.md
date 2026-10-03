# Browser attestation and binding qualification

Candidate profile, checked 2026-10-03. **Not a shipping SDK, fresh browser Nitro
session, security audit or production dependency approval.** This tool cannot
operate a financial account. [ADR 0021](../../docs/architecture/0021-confidential-web-api.md)
owns the product contract; P21A remains in progress.

## Trust and cryptographic boundary

The browser itself parses the quote and verifies certificates and signatures
using pinned PKI.js 3.4.1, ASN1.js 3.0.10 and platform WebCrypto. It does not call
a remote Node approval service. The separately compiled existing native verifier
is an **offline test oracle only**. One Rust/Snow core supplies both native and
WASM Noise; no JavaScript handshake/cipher is introduced.

`verifyWebQuote` has only the embedded AWS root, checked against the exact native
DER SHA-256 fingerprint. It returns a redacted error on failure. The internal
`verifyWithRoot` exists solely for explicitly synthetic qualification. Never
export it as a customer SDK or let the relay choose a root/release policy.
Public inputs are copied before asynchronous verification. No native capability,
wallet signature, account ID or private request is sent before verification and
both encrypted key-confirmation records.

The policy is independently selected, exact 240 bytes: network and deployment
IDs (32 bytes each), release-manifest digest (32), and PCR0/1/2 (48 each). Each
component must be nonzero. The client generates its own unpredictable 32-byte
challenge; missing entropy and all-zero output refuse without fallback. The
client supplies its independently trusted Unix-millisecond time, not a relay time.

## Exact candidate wire context

All literal strings below are ASCII with the explicitly shown terminal NUL.
`P` is the complete policy above. `N`, `B`, `H`, `R` are respectively challenge,
boot identity, public routing handle and responder X25519 key, each 32 bytes and
nonzero. `E` is expiry as eight-byte unsigned big-endian Unix milliseconds.
The web-facing implementation uses exact safe integers within the Date range.

```text
suite = "Noise_NK_25519_ChaChaPoly_SHA256\0"
C = "CINDER-WEB-CONTEXT-1\0" || suite || P || N || B || H || R || E
user_data = SHA384(C)
nonce = N; public_key = R
Q = exact complete signed COSE quote bytes
prologue = SHA256("CINDER-WEB-PROLOGUE-1\0" || C || SHA256(Q))
binding = SHA256("CINDER-WEB-SESSION-1\0" || prologue || confirmed_Noise_handshake_hash)
```

`C` is 430 bytes. Context/prologue/binding functions have native Rust, WASM and
WebCrypto agreement tests. Fixed public hash-input vectors are in `tests/profile.rs`
and `attestation/suite.mjs`; their dummy quote bytes are **not** valid attestation.
The final binding is available through the endpoint only after both confirmation
records. It is the candidate 32-byte P18 signing binding, not a replacement for
owner/agent signatures, writer authorization or economic operation IDs.

The quote must have the selected PCRs, exact key/challenge/context digest, no
future timestamp and at most 30 seconds' age. Expiry must be later than client
time and no later than signed quote time plus 120 seconds. Old TLS, clock or
KMS-recipient quotes have different purpose data and cannot authorize this profile.
The prologue commits exact quote bytes, including signature/tag framing: another
valid quote for the same key is not an interchangeable session authorization.

## Closed parsing and certificate profile

The bounded CBOR profile mirrors the native verifier: at most 16 KiB; tagged or
untagged four-element Sign1; exact ES384 protected header; empty unprotected map;
96-byte raw ECDSA signature. Exactly nine unique recognized document fields,
minimal integers/lengths, bounded strict UTF-8, one to eight certificates in the
CA bundle, each certificate at most 1024 bytes, and unique PCR indices including
0/1/2. Only the actual NSM-shaped top-level map may be indefinite; nested
indefinite values, duplicate fields/indices, missing required PCRs and trailing
bytes refuse. COSE verifies the **original** signed payload, not a reserialized map.

PKI.js's chain engine alone is not OpenSSL `X509_STRICT`: source review found no
path-length enforcement. Explicit checks plus its full signature/path engine
therefore enforce a deliberately narrower AWS-shaped profile:

- Exact pinned root first; unique, ordered root-to-leaf chain; valid root signature.
- Every certificate, including the anchor, valid at independent client time;
  v3, positive bounded serial, canonical round-trip shape, P384 public keys and
  matching ES384 inner/outer certificate signature algorithms.
- Typed, duplicate-free basic constraints, key usage, SKI and AKI; only additionally
  noncritical CRL distribution metadata. Other extensions/algorithms fail closed.
- CA constraints and CA key usage must be critical, keyCertSign present, CA SKI
  present, non-leaf AKI present and issuer identifiers consistent. The observed
  AWS leaf legitimately has no AKI/SKI and noncritical digital-signature usage;
  it still requires the authenticated exact issuer path and valid signature.
- Explicit CA path-length enforcement; non-root self-issued shapes refuse.
  Unknown critical extensions, invalid signatures, non-CAs and unauthorized
  issuer usage are never converted into an accept-on-error fallback.

Like the existing native policy, no online CRL/OCSP fetching or revocation status
is claimed. Noncritical CRL locations are parsed metadata, not permission to
fetch a caller-controlled URL. Production revocation/release fencing is a distinct
governance requirement. The narrower profile intentionally rejects, for example,
an otherwise native-accepted SHA256 certificate signature with a P384 leaf.
This is tested and recorded, **not universal equivalence to arbitrary PKIX paths**.
Future legitimate AWS shape changes require explicit review; never relax on error.

## Evidence

The repeatable runner generates 40 fresh synthetic signed cases. Public fixtures
contain certificates/quotes and expected outcomes, never private signing keys.
An independent native executable checks the same certificates and original
document fields with TLS-purpose data; web-only boot/handle/domain substitutions
are separately tested, not falsely called native failures. Every fixture records
the expected and observed native outcome. Its fixtures feature is absent from
the default shipping verifier.

Node and real headless Chrome test that corpus, signature/payload/root tampering,
closed CBOR and every quote truncation, input-snapshot races, fixed encoding
vectors, early binding refusal, challenge entropy failure and native encrypted
round trips. The native responder owns the fresh Noise private key; the synthetic
attester signs only its public key/context. Both endpoints derive the same final
binding; the carrier sees no private fixture marker. This is targeted leakage
testing, not metadata privacy or hardware isolation proof.

`attestation/aws-certificate-capture.json` promotes **only five public DER
certificates** from P20's public-only NSM clock probe on 2026-10-02. Capture time
and original quote hash identify provenance. It contains no original quote,
release measurement, challenge, storage ciphertext, capability or private key.
Tests validate this historical AWS path at capture time and reject its expired
leaf later. That evidence verifies actual certificate shapes, not a current web
quote, running application measurement or hardware session.

The root shipping Cargo graph and P19 Node TLS are unchanged. The isolated Rust
graph still has 43 registry packages; SHA2 0.10.9 is now also an exact direct edge
for canonical binding, with no new package/feature. NPM manifest/lock SHA-256 and
all registry integrity/source/lifecycle changes are guarded. Runtime candidates:
PKI.js, ASN1.js, bytestreamjs, pvtsutils, pvutils, tslib and @noble/hashes. Build-only
esbuild 0.28.2 uses its locked platform binary; lifecycle scripts are disabled.
The lock contains 34 external packages including 26 optional platform binaries;
only the host binary is installed. No bundle or downloaded research tree is tracked.

## Remaining gates, unchanged by passing this slice

Snow 0.10.0's opaque DH/cipher/chaining state still lacks qualified zeroizing
destruction. Upstream issue 203 is open. No wrapper, Web Worker teardown, garbage
collection or successful round trip proves those secrets were erased. **No fork
or production library selection has been made here.** A separately reviewed
hardened implementation or narrowly scoped upstream/vendor patch is still needed
before service integration; qualify all secret lifetimes, temporary derivations,
entropy and build targets without changing standard Noise behavior.

Also remaining: SDK-owned verification/handshake deadlines and post-verification
expiry enforcement; server NSM web-purpose quote generation; actual enclave
entropy; current writer/lease/fence checks; bounded authenticated chunking and
session ownership; HTTP/WS/shared API integration; changed-image hardware tests
and release distribution/security review. The existing Node TLS path stays intact.
All parent-phase P21A acceptance criteria remain open.

## Primary references checked 2026-10-03

- [AWS Nitro validation](https://docs.aws.amazon.com/enclaves/latest/user/verify-root.html).
- [PKI.js chain engine](https://pkijs.org/docs/api/classes/CertificateChainValidationEngine/)
  and [upstream source](https://github.com/PeculiarVentures/PKI.js); exact installed
  3.4.1 source, not docs alone, informed the path-length and closed-policy checks.
- [Noise revision 34](https://noiseprotocol.org/noise.html) and
  [W3C WebCrypto](https://www.w3.org/TR/webcrypto/).
- [Snow's open secret-erasure issue](https://github.com/mcginty/snow/issues/203).

These are dependencies/standards, not an independent audit of Cinder's composition.
