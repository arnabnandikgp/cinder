# ADR 0027 — Bounded confidential native transfer capture

Status: implemented and locally tested transport/journal boundary; the subsequent
[measured runtime continuation](0028-measured-native-capture.md) supplies local
loaded-runtime wiring. Actual Nitro/native qualification remains open. This continues
P23's existing funding-provider gate, not a new financial policy or storage model.

## Decision

Capture Pacifica `account_transfers` through a fixed native WebSocket origin with
TLS 1.3 terminating inside the enclave. The parent only copies ciphertext on a
separate fixed vsock route. It cannot subscribe, parse responses, choose accounts,
or manufacture authenticated evidence.

`cinder-pacifica::capture` prepares one non-clone request after durably spending
the existing shared pool/IP read budget. The conservative charge is the same
qualified heavy-read cost, at least 120 tenths of a credit; this is **not** a claim
that Pacifica charges that amount for a subscription. The start record binds the
exact Gateway contract, reservation, pooled account, origin and resource limits.
Preparation errors/unknown commits must be reopened, not retried with fresh IDs.

`cinder-service::native_capture::Port` consumes that request once, verifies the
configured root, hostname and certificate times, upgrades `/ws`, and writes only
the account-bound `account_transfers` subscription. No URL override, redirect,
system trust fallback, compression, application subprotocol, bearer token or
financial signing method exists. `cinder-nitro-egress` adds explicit
`pacifica-testnet-ws` / `pacifica-mainnet-ws` routes; route existence is not live
activity authorization.

The caller must prepare under the writer's mutation guard, drop it during network
I/O, then reacquire it for **each** capture result. `Archive::append` revalidates the exact
accepted reservation/start and current loaded contract, then publishes against
the current head. An intervening accepted command does not discard a valid raw
observation. A failed/uncertain sink ends capture immediately; it does not consume
another message, try another journal identity or resend the subscription.

## Bounds and lifecycle

- Explicit caller limits: 1–30,000 ms absolute capture duration, 1–256 peer
  messages, at most 16,384 bytes per complete message and 1,048,576 payload bytes.
  All decoded peer bodies and RFC controls share the budget; fragmented messages
  share one message-size bound. Raw evidence is also bounded by journal encoding.
- Upgrade headers are capped at 8,192 plaintext bytes. A separate finite wire-byte
  budget bounds fragment/control overhead, including empty-fragment floods.
- The owned socket has bounded connect/read/write operations and an absolute
  shutdown deadline covering TLS/upgrade and capture, not a sliding idle timer.
  Journal I/O has its own existing bounds; the capture deadline is not a promise
  that a blocked sink completes within that duration.
- This first qualification port is a finite capture below the documented
  60-second send-idle cutoff, **not** a continuously reconnecting feed. It adds no
  application heartbeat, resubscription or replay/frontier inference. A quiet
  socket can terminate sooner at the existing five-second read timeout.
- Retain opened, text, binary, ping/pong and close facts; unrecognized JSON and
  duplicate bodies remain available rather than being interpreted as economic
  events or silently deduplicated. Close, interruption and limits are uncertain
  capture endings, not evidence that all native effects have arrived.
- If the post-read clock is unavailable/backward or the deadline has elapsed,
  retain an already received bounded body and its original kind, then end capture.
  `clock_sample=unverified_after_read` explicitly marks its timestamp as the
  **last available trusted time**, not the body's receipt time. It cannot become
  financial evidence. Stops without a received body use `last_known`. The shipping
  sink still requires an independent current clock/witness: if they or durability
  are unavailable, it refuses and fences rather than bypassing freshness. A crash
  or failed sink can therefore leave an unresolved start; retention is not promised
  through loss of all trusted storage authority. Restart never resumes the socket.

Each archive input has `event=None`, `source_cut=None`, `authority_epoch=0`.
Capture cannot create cash, establish setup readiness, release a funding hold,
certify a native debit, prove Solana finality or complete a withdrawal. Existing
operation/signature/batch correlation and future settlement providers must check
the authenticated raw evidence separately.

## Dependency/security review

Pin `tungstenite=0.30.0`, default features off, `handshake` only, over the existing
OpenSSL `SslStream`. Do not use its connection/TLS convenience functions: those
could select routes, retry redirects or install a second trust store. Prefer the
existing tested protocol implementation to inventing a client RFC6455 parser.

The resolved addition is nine registry packages: tungstenite 0.30.0, rand 0.10.3,
rand_core 0.10.1, getrandom 0.4.3, chacha20 0.10.2, data-encoding 2.11.1,
httparse 1.10.1, sha1 0.11.0 and target-only r-efi 6.0.0. Existing package versions
and registry checksums are unchanged. Update the lock hash and resolved-feature
guard explicitly; no dependencies enter the kernel or isolated Anchor workspace.
The new build scripts only detect sanitizer support (`getrandom`) and Rust/CPU
parser features (`httparse`); neither fetches code or credentials.

WebSocket key/mask entropy comes from the library's OS-seeded `rand`/`getrandom`,
not from a caller-injected seed or a claim that it uses NSM directly. Its actual
enclave OS entropy and changed measured image still require fresh hardware
qualification. Entropy failure may panic; shipping integration must preserve the
existing panic/sticky-fence behavior and must not install a weak fallback.
SHA-1 is only the RFC6455 upgrade accept computation, not a financial signature,
attestation check or journal hash. Library-owned transient plaintext buffers are
not guaranteed erased; do not advertise complete memory erasure.

The library's debug/trace macros include HTTP requests and plaintext frames.
Pin the existing `log=0.4.34` with `max_level_off` and `release_max_level_off` so
these are compiled out across the unified graph in debug and release. Tests
assert `STATIC_MAX_LEVEL=Off`; guard tests reject feature/pin/default changes.
This is local code/dependency review, **not** an independent audit.

## Evidence and remaining work

Offline tests use an independent manually framed RFC6455 peer over actual local
OpenSSL TLS, synthetic roots and an unfunded public fixture account. They cover
exact subscription/masking, fragmented text and interleaved controls, binary and
unknown bodies, TLS root/name/time rejection, malformed upgrade/frames, finite
budgets, bad clocks, sink failure, raw replay, shared credits and head races.
They are not Pacifica or Nitro receipts.

At the original #61 component checkpoint, `runtime::Configuration` and
`boot::Manifest` were deliberately unchanged and no scheduler activation existed.
The subsequent [ADR 0028](0028-measured-native-capture.md) implements the measured
version-4 one-shot worker locally, without financial activation. Before live
capture, bind its actual route/root, limits and account to the measured loaded
contract; wire preparation/I/O/append outside the mutation guard; keep current
financial gates closed. Continuous subscription, reconnect/gap policy and live
source/settlement semantics require their own explicit qualification, not silent
extensions of this finite capture. Then obtain a fresh bounded G05 manifest before
native/AWS/devnet/financial activity. Existing closed hardware receipts do not
qualify this changed dependency graph/image.

Primary references rechecked 2026-10-07:

- [Pacifica WebSocket lifecycle/origins](https://docs.pacifica.fi/api-documentation/api/websocket).
- [Account transfer subscription and fields](https://docs.pacifica.fi/api-documentation/api/websocket/subscriptions/account-transfers).
- [tungstenite custom-stream client API](https://docs.rs/tungstenite/0.30.0/tungstenite/client/fn.client_with_config.html).
- [Pinned source/features](https://github.com/snapview/tungstenite-rs/tree/v0.30.0).
- [WebSocket framing and client masking](https://www.rfc-editor.org/rfc/rfc6455).
