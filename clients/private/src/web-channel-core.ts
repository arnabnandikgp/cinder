// Internal transport implementation, not a public caller-selectable trust root.
// Public web-channel.ts fixes AWS root; fixture trust lives in tests only.
import type { ChannelContext, ConfidentialChannel } from './index.ts';
import type { ReleasePolicy } from './node-channel.ts';
import type { QuoteContext, VerifiedQuote } from '../../../tools/web-channel/attestation/verifier.mjs';

export interface WebEndpoint {
  start(): Uint8Array; advance(bytes: Uint8Array): Uint8Array;
  ready(): boolean; binding(): Uint8Array;
  request(sequence: number, bytes: Uint8Array): Uint8Array;
  response(sequence: number, bytes: Uint8Array): Uint8Array;
  free(): void;
}
/** Load this pinned Rust/WASM build from the application's trusted distribution,
 * never a URL/module/public key selected by the relay's attestation response. */
export interface WebCore {
  BrowserEndpoint: new (key: Uint8Array, prologue: Uint8Array) => WebEndpoint;
  web_binding(prologue: Uint8Array, hash: Uint8Array): Uint8Array;
}
export interface WebOptions { baseUrl: string; policy: ReleasePolicy; core: WebCore }
type Verifier = (quote: Uint8Array, policy: Uint8Array, context: QuoteContext) => Promise<VerifiedQuote>;
const MAX_BATCH = 1048576 + 65 * 34, DEADLINE = 5000;
const failed = () => Error('Confidential web channel unavailable; reconcile on a fresh session');
const copy = (value: Uint8Array, size: number) => {
  if (!(value instanceof Uint8Array) || value.length !== size || !value.some(b => b)) throw failed();
  return Uint8Array.from(value);
};
const concat = (...parts: Uint8Array[]) => {
  const result = new Uint8Array(parts.reduce((n, p) => n + p.length, 0)); let at = 0;
  for (const p of parts) { result.set(p, at); at += p.length; } return result;
};
export class WebChannel implements ConfidentialChannel {
  #endpoint?: WebEndpoint; #context?: ChannelContext; #handle?: Uint8Array;
  #base: string; #closed = false; #busy = false; #sequence = 0;
  #active?: AbortController; #lifetime?: ReturnType<typeof setTimeout>;
  private constructor(base: string) { this.#base = base; }
  static async connect(options: WebOptions, verify: Verifier): Promise<WebChannel> {
    const url = new URL(options.baseUrl);
    if (url.username || url.password || url.search || url.hash || url.pathname !== '/'
        || (url.protocol !== 'https:' && !(url.protocol === 'http:' && ['127.0.0.1','[::1]','localhost'].includes(url.hostname)))) throw failed();
    const policy = concat(copy(options.policy.network,32), copy(options.policy.deployment,32),
      copy(options.policy.manifest,32), ...options.policy.pcrs.map(p => copy(p,48)));
    if (policy.length !== 240) throw failed();
    const core = options.core;
    const ch = new WebChannel(url.origin), abort = new AbortController();
    ch.#active = abort;
    const timer = setTimeout(() => ch.close(), DEADLINE);
    try {
      const nonce = crypto.getRandomValues(new Uint8Array(32));
      copy(nonce,32);
      const envelope = await ch.#post('/v1/attestation', nonce, 16520, abort.signal);
      if (envelope.length <= 136 || !nonce.every((b,i) => b === envelope[i])) throw failed();
      const expires = new DataView(envelope.buffer, envelope.byteOffset + 128, 8).getBigUint64(0);
      if (expires > BigInt(Number.MAX_SAFE_INTEGER)) throw failed();
      const context: QuoteContext = { now: Date.now(), expires: Number(expires), nonce,
        boot: copy(envelope.slice(32,64),32), handle: copy(envelope.slice(64,96),32), key: copy(envelope.slice(96,128),32) };
      const trusted = await verify(envelope.slice(136), policy, context);
      if (ch.#closed || abort.signal.aborted || !context.key.every((b,i) => b === trusted.key[i]) || trusted.key.length !== 32) throw failed();
      ch.#handle = context.handle;
      ch.#endpoint = new core.BrowserEndpoint(copy(trusted.key,32), copy(trusted.prologue,32));
      const first = ch.#endpoint.start();
      const second = await ch.#post('/v1/session', concat(ch.#handle, first), 16400, abort.signal);
      const confirmation = ch.#endpoint.advance(second);
      const ack = await ch.#post('/v1/session', concat(ch.#handle, confirmation), 16400, abort.signal);
      if (ch.#endpoint.advance(ack).length || !ch.#endpoint.ready() || ch.#closed || expires <= BigInt(Date.now())) throw failed();
      ch.#context = { network: policy.slice(0,32), deployment: policy.slice(32,64),
        binding: copy(core.web_binding(trusted.prologue, ch.#endpoint.binding()),32), expiresAt: expires };
      ch.#lifetime = setTimeout(() => ch.close(), Number(expires - BigInt(Date.now())));
      return ch;
    } catch { ch.close(); throw failed(); }
    finally { clearTimeout(timer); ch.#active = undefined; }
  }
  context(): ChannelContext {
    if (this.#closed || !this.#context || this.#context.expiresAt <= BigInt(Date.now())) { this.close(); throw failed(); }
    return { ...this.#context, network: Uint8Array.from(this.#context.network), deployment: Uint8Array.from(this.#context.deployment), binding: Uint8Array.from(this.#context.binding) };
  }
  close() {
    const handle = this.#handle;
    this.#closed = true; this.#active?.abort(); this.#active = undefined;
    clearTimeout(this.#lifetime); this.#lifetime = undefined;
    this.#endpoint?.free(); this.#endpoint = undefined;
    this.#handle = undefined; this.#context = undefined;
    if (handle) void fetch(this.#base + '/v1/session', { method: 'POST', headers: { 'Content-Type': 'application/octet-stream' },
      body: concat(handle, Uint8Array.of(0)).buffer, credentials: 'omit', cache: 'no-store', redirect: 'error',
      referrerPolicy: 'no-referrer', signal: AbortSignal.timeout(DEADLINE) }).then(r => r.body?.cancel()).catch(() => {});
    // free() deallocates the WASM object. It is NOT an opaque Snow/browser
    // key-erasure guarantee; that release limitation remains explicit.
  }
  async exchange(input: Uint8Array): Promise<Uint8Array> {
    if (this.#closed || this.#busy || !(input instanceof Uint8Array) || input.length === 0 || input.length > 1024) { this.close(); throw failed(); }
    this.context(); this.#busy = true;
    const clear = Uint8Array.from(input), abort = new AbortController(); this.#active = abort;
    const timer = setTimeout(() => this.close(), DEADLINE);
    try {
      const sequence = ++this.#sequence;
      const wire = this.#endpoint!.request(sequence, clear); clear.fill(0);
      const batch = await this.#post('/v1/exchange', concat(this.#handle!, wire), MAX_BATCH, abort.signal);
      this.context();
      const reply = this.#endpoint!.response(sequence, batch);
      if (this.#closed) { reply.fill(0); throw failed(); }
      return reply;
    } catch { this.close(); throw failed(); }
    finally { clear.fill(0); clearTimeout(timer); this.#active = undefined; this.#busy = false; }
  }
  async #post(path: string, body: Uint8Array, maximum: number, signal: AbortSignal): Promise<Uint8Array> {
    if (this.#closed || signal.aborted) throw failed();
    const response = await fetch(this.#base + path, { method: 'POST', headers: { 'Content-Type': 'application/octet-stream' },
      body: Uint8Array.from(body).buffer, credentials: 'omit', cache: 'no-store', redirect: 'error', referrerPolicy: 'no-referrer', signal });
    if (!response.ok || response.headers.get('content-type') !== 'application/octet-stream' || !response.body) {
      await response.body?.cancel(); throw failed();
    }
    const reader = response.body.getReader(); const chunks: Uint8Array[] = []; let total = 0;
    try {
      for (;;) {
        const { value, done } = await reader.read(); if (done) break;
        total += value.length;
        if (total > maximum || chunks.length >= 4096 || this.#closed || signal.aborted) throw failed();
        chunks.push(value);
      }
      if (!total || this.#closed || signal.aborted) throw failed();
      return concat(...chunks);
    } finally { await reader.cancel().catch(() => {}); reader.releaseLock(); }
  }
}
