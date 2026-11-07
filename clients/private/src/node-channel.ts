// Node automation transport profile. Browser fetch cannot inspect TLS exporters.
// No runtime npm dependencies, wallet loader, native credentials or retry loop.
import tls, { type TLSSocket } from 'node:tls';
import { X509Certificate, createHash, randomBytes } from 'node:crypto';
import { execFile } from 'node:child_process';
import { isAbsolute } from 'node:path';
import type { ChannelContext, ConfidentialChannel } from './index.ts';

const ALPN = 'cinder-private/1', EXPORTER = 'EXPORTER-Cinder-private-v1';
const MAX_RESPONSE = 1_048_576, DEADLINE = 5000, MAX_SESSION = 120_000n;
function failure(): Error { return new Error('Attested channel unavailable'); }
function u64(n: bigint): Buffer { const b = Buffer.alloc(8); b.writeBigUInt64BE(n); return b; }
function frame(bytes: Uint8Array): Buffer { const header = Buffer.alloc(4); header.writeUInt32BE(bytes.length); return Buffer.concat([header, bytes]); }
function nonzero(b: Uint8Array, n: number): boolean { return b instanceof Uint8Array && b.length === n && b.some(x => x !== 0); }
export interface ReleasePolicy {
  network: Uint8Array; deployment: Uint8Array; manifest: Uint8Array;
  pcrs: readonly [Uint8Array, Uint8Array, Uint8Array];
}
/** Independently selected local verifier, not a path/command from the endpoint.
 * Only public quote/key/policy input is passed to this executable. No shell. */
export interface QuoteVerifier { verify(publicInput: Uint8Array): Promise<Uint8Array> }
export function nativeQuoteVerifier(executable: string): QuoteVerifier {
  if (!isAbsolute(executable)) throw failure();
  return { verify: input => new Promise((resolve, reject) => {
    if (input.length > 20_480) { reject(failure()); return; }
    const child = execFile(executable, [], { timeout: DEADLINE, maxBuffer: 1024, encoding: 'buffer' }, (error, stdout) => {
      if (error || stdout.length !== 32) reject(failure()); else resolve(new Uint8Array(stdout));
    });
    child.stdin?.on('error', () => reject(failure())); child.stdin?.end(input);
  }) };
}
// A single pending frame, bounded before concatenation. Serialized exchange is
// backpressure: concurrent callers reject instead of an unbounded request queue.
class Framing {
  socket: TLSSocket; pending?: { maximum: number; resolve: (b: Buffer) => void; reject: (e: Error) => void; timer: NodeJS.Timeout };
  chunks: Buffer[] = []; total = 0; size?: number; closed = false;
  constructor(socket: TLSSocket) {
    this.socket = socket;
    socket.on('data', chunk => this.data(chunk));
    socket.on('error', () => this.close()); socket.on('end', () => this.close()); socket.on('close', () => this.close());
  }
  close() {
    this.closed = true; const p = this.pending; this.pending = undefined;
    if (p) { clearTimeout(p.timer); p.reject(failure()); }
    for (const b of this.chunks) b.fill(0); this.chunks = []; this.total = 0; this.size = undefined; this.socket.destroy();
  }
  data(chunk: Buffer) {
    const p = this.pending;
    if (!p || this.total + chunk.length > p.maximum + 4 || this.chunks.length >= 4096) { chunk.fill(0); this.close(); return; }
    this.chunks.push(chunk); this.total += chunk.length;
    // Avoid quadratic copies for malicious byte-by-byte bodies.
    if (this.size === undefined && this.total >= 4) {
      const header = Buffer.concat(this.chunks, Math.min(this.total, 4)); this.size = header.readUInt32BE();
      if (this.size === 0 || this.size > p.maximum) { this.close(); return; }
    }
    if (this.size !== undefined && this.total >= this.size + 4) {
      if (this.total !== this.size + 4) { this.close(); return; }
      const out = Buffer.concat(this.chunks).subarray(4); for (const b of this.chunks) b.fill(0);
      this.chunks = []; this.total = 0; this.size = undefined;
      this.pending = undefined; clearTimeout(p.timer); p.resolve(out);
    }
  }
  async exchange(body: Uint8Array, maximum: number): Promise<Buffer> {
    if (this.closed || this.pending) throw failure();
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => this.close(), DEADLINE);
      this.pending = { maximum, resolve, reject, timer };
      const out = frame(body);
      this.socket.write(out, error => { out.fill(0); if (error) this.close(); });
    });
  }
}
/** Only connect() constructs a verified channel. TLS is intentionally untrusted
 * until the pinned AWS/PCR verifier authenticates the exact socket SPKI/exporter.
 * No customer/wallet information is sent during that verification stage. */
export class AttestedNodeChannel implements ConfidentialChannel {
  #frames: Framing; #context: ChannelContext; #expiryTimer: NodeJS.Timeout;
  #busy = false; #requests = 0;
  private constructor(frames: Framing, context: ChannelContext) {
    this.#frames = frames; this.#context = context;
    this.#expiryTimer = setTimeout(() => this.close(), Number(context.expiresAt - BigInt(Date.now())));
  }
  static async connect(config: { host: string; port: number; policy: ReleasePolicy; verifier: QuoteVerifier }): Promise<AttestedNodeChannel> {
    const p = config.policy;
    if (!nonzero(p.network, 32) || !nonzero(p.deployment, 32) || !nonzero(p.manifest, 32)
      || p.pcrs.length !== 3 || p.pcrs.some(b => !nonzero(b, 48)) || !Number.isInteger(config.port) || config.port < 1 || config.port > 65535) throw failure();
    // Snapshot independently provided policy before any asynchronous operation.
    const policy = Buffer.concat([p.network, p.deployment, p.manifest, ...p.pcrs]);
    const socket = tls.connect({ host: config.host, port: config.port, ALPNProtocols: [ALPN], minVersion: 'TLSv1.3', maxVersion: 'TLSv1.3',
      rejectUnauthorized: false, ciphers: 'TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256:TLS_AES_128_GCM_SHA256' });
    const frames = new Framing(socket);
    try {
      await new Promise<void>((resolve, reject) => {
        const timer = setTimeout(() => { frames.close(); reject(failure()); }, DEADLINE);
        socket.once('secureConnect', () => { clearTimeout(timer); resolve(); });
        socket.once('error', () => { clearTimeout(timer); reject(failure()); });
        socket.once('close', () => { clearTimeout(timer); reject(failure()); });
      });
      if (socket.getProtocol() !== 'TLSv1.3' || socket.alpnProtocol !== ALPN || socket.isSessionReused()) throw failure();
      const cert = socket.getPeerCertificate(true); if (!cert.raw) throw failure();
      const spki = new X509Certificate(cert.raw).publicKey.export({ format: 'der', type: 'spki' });
      const exporter = socket.exportKeyingMaterial(32, EXPORTER, Buffer.alloc(0)); const nonce = randomBytes(32);
      const response = await frames.exchange(nonce, 16_424);
      if (response.length < 41) throw failure();
      const boot = response.subarray(0, 32), expiry = response.readBigUInt64BE(32), now = BigInt(Date.now());
      if (expiry <= now || expiry > now + MAX_SESSION) throw failure();
      const length = Buffer.alloc(4); length.writeUInt32BE(spki.length);
      const input = Buffer.concat([policy, nonce, exporter, boot, u64(expiry), u64(now), length, spki, response.subarray(40)]);
      const digest = createHash('sha384').update(Buffer.concat([Buffer.from('CINDER-TLS-ATTESTATION-1\0'), policy.subarray(0, 96), boot, u64(expiry), exporter])).digest();
      const expected = createHash('sha256').update(Buffer.concat([Buffer.from('CINDER-TLS-SESSION-1\0'), digest, nonce])).digest();
      // Bound even an injected verifier's latency; no private data on this path.
      let timer: NodeJS.Timeout | undefined;
      const binding = await Promise.race([config.verifier.verify(input), new Promise<never>((_, reject) => {
        timer = setTimeout(() => reject(failure()), DEADLINE);
      })]).finally(() => clearTimeout(timer));
      if (binding.length !== 32 || !Buffer.from(binding).equals(expected) || frames.closed || BigInt(Date.now()) >= expiry) throw failure();
      const confirmed = await frames.exchange(expected, 32); if (!confirmed.equals(expected)) throw failure();
      return new AttestedNodeChannel(frames, { network: policy.subarray(0, 32), deployment: policy.subarray(32, 64), binding: expected, expiresAt: expiry });
    } catch { frames.close(); throw failure(); }
  }
  context(): ChannelContext {
    if (this.#frames.closed || BigInt(Date.now()) >= this.#context.expiresAt) throw failure();
    return { ...this.#context, network: new Uint8Array(this.#context.network), deployment: new Uint8Array(this.#context.deployment), binding: new Uint8Array(this.#context.binding) };
  }
  async exchange(privateWire: Uint8Array): Promise<Uint8Array> {
    this.context();
    if (this.#busy) throw failure();
    if (!(privateWire instanceof Uint8Array) || privateWire.length === 0 || privateWire.length > 1024 || this.#requests >= 128) { this.close(); throw failure(); }
    this.#busy = true; this.#requests++;
    try { return await this.#frames.exchange(privateWire, MAX_RESPONSE); }
    catch { this.close(); throw failure(); } finally { this.#busy = false; }
  }
  close() { clearTimeout(this.#expiryTimer); this.#frames.close(); }
}
