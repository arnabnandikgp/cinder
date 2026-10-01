// Exact private application protocol. No network defaults, RPC, native signer or retries.
export type Id = Uint8Array;
export interface Domain { network: Id; deployment: Id }
export interface Grant {
  key: Id; methods: number; market: Id; maximumLots: bigint; maximumFee: bigint;
  maximumOrders: bigint; expiresAt: bigint;
}
export const READ = 1, TRADE = 2, CANCEL = 4;
export type Command =
  | { kind: 'view' }
  | { kind: 'operation'; target: Id }
  | { kind: 'order'; market: Id; lots: bigint; minimum: bigint; maximum: bigint;
      fee: bigint; tif: 'GTC' | 'ALO' | 'IOC'; reduceOnly: boolean; goodUntil: bigint }
  | { kind: 'cancel'; target: Id; attempt: Id; goodUntil: bigint }
  | { kind: 'payout'; net: bigint; maximumFee: bigint; allowPartial: boolean; goodUntil: bigint }
  | { kind: 'grant'; grant: Grant }
  | { kind: 'revoke' }
  | { kind: 'leverage'; market: Id; leverage: bigint; goodUntil: bigint };
export interface Intent { domain: Domain; account: Id; id: Id; policy: number; command: Command }
export interface Envelope extends Intent { epoch: bigint; signer: Id; session: Id; expiresAt: bigint }
export type Outcome = 'rejected' | 'accepted' | 'dispatched' | 'acknowledged' | 'partial' | 'complete' | 'unknown';
export interface Receipt {
  kind: 'receipt'; id: Id; digest: Id; policy: number; outcome: Outcome; filled: bigint;
  paid: bigint; railFees: bigint; feeCap: bigint; possiblyExposed: boolean; allowPartial: boolean;
}
export interface View {
  kind: 'view'; epoch: bigint; cash: bigint; funding: bigint; held: bigint;
  positions: { market: Id; precision: number; lots: bigint; basis: bigint }[]; operations: Id[];
}
export type ErrorCode = 'invalid' | 'unauthorized' | 'not-found' | 'conflict' | 'unavailable';
export type Response = Receipt | View | { kind: 'error'; code: ErrorCode };
export interface MessageSigner { publicKey: Id; signMessage(message: Uint8Array): Promise<Uint8Array> }
export interface ChannelContext extends Domain { binding: Id; expiresAt: bigint }
/** Trusted port, NOT evidence of encryption/attestation. P19 supplies the verified
 * implementation. Never obtain this object from an untrusted parent/relay. */
export interface ConfidentialChannel {
  context(): ChannelContext;
  exchange(privateWire: Uint8Array): Promise<Uint8Array>;
}
const utf8 = new TextEncoder();
function fail(): never { throw new Error('Invalid private protocol'); }
function id(v: Id): Uint8Array {
  if (!(v instanceof Uint8Array) || v.length !== 32 || v.every(b => b === 0)) fail();
  return v.slice();
}
function equal(a: Uint8Array, b: Uint8Array) { return a.length === b.length && a.every((v, i) => v === b[i]); }
class Writer {
  chunks: Uint8Array[];
  constructor(prefix: string) { this.chunks = [utf8.encode(prefix)]; }
  key(v: Id) { this.chunks.push(id(v)); }
  n(n: bigint, bytes: number, signed = false) {
    if (typeof n !== 'bigint') fail();
    const bits = BigInt(bytes * 8), min = signed ? -(1n << (bits - 1n)) : 0n;
    const max = signed ? (1n << (bits - 1n)) - 1n : (1n << bits) - 1n;
    if (n < min || n > max) fail();
    const out = new Uint8Array(bytes); let u = BigInt.asUintN(bytes * 8, n);
    for (let i = bytes - 1; i >= 0; i--) { out[i] = Number(u & 255n); u >>= 8n; }
    this.chunks.push(out);
  }
  boolean(v: boolean) { if (typeof v !== 'boolean') fail(); this.n(v ? 1n : 0n, 1); }
  end() {
    const result = new Uint8Array(this.chunks.reduce((n, c) => n + c.length, 0));
    let at = 0; for (const c of this.chunks) { result.set(c, at); at += c.length; c.fill(0); }
    return result;
  }
}
function prefix(w: Writer, i: Intent) {
  w.key(i.domain.network); w.key(i.domain.deployment); w.key(i.account); w.key(i.id);
  if (!Number.isInteger(i.policy) || i.policy < 1 || i.policy > 0xffffffff) fail();
  w.n(BigInt(i.policy), 4);
}
function command(w: Writer, c: Command) {
  switch (c.kind) {
    case 'view': w.n(0n, 1); break;
    case 'operation': w.n(1n, 1); w.key(c.target); break;
    case 'order': {
      const tif = { GTC: 0n, ALO: 1n, IOC: 2n }[c.tif];
      if (tif === undefined || c.lots === 0n || c.minimum <= 0n || c.maximum < c.minimum || c.fee < 0n
        || c.fee * (c.lots < 0n ? -c.lots : c.lots) > (1n << 127n) - 1n) fail();
      w.n(2n, 1); w.key(c.market); w.n(c.lots, 8, true); w.n(c.minimum, 8); w.n(c.maximum, 8);
      w.n(c.fee, 16, true); w.n(tif, 1); w.boolean(c.reduceOnly); w.n(c.goodUntil, 8); break;
    }
    case 'cancel': w.n(3n, 1); w.key(c.target); w.key(c.attempt); w.n(c.goodUntil, 8); break;
    case 'payout':
      if (c.net <= 0n || c.maximumFee < 0n || c.net + c.maximumFee > (1n << 127n) - 1n) fail();
      w.n(4n, 1); w.n(c.net, 16, true); w.n(c.maximumFee, 16, true); w.boolean(c.allowPartial); w.n(c.goodUntil, 8); break;
    case 'grant': {
      const g = c.grant;
      if (!Number.isInteger(g.methods) || g.methods < 1 || g.methods > 7 || g.maximumLots <= 0n || g.maximumFee < 0n || g.maximumOrders <= 0n) fail();
      w.n(5n, 1); w.key(g.key); w.n(BigInt(g.methods), 1); w.key(g.market); w.n(g.maximumLots, 8);
      w.n(g.maximumFee, 16, true); w.n(g.maximumOrders, 8); w.n(g.expiresAt, 8); break;
    }
    case 'revoke': w.n(6n, 1); break;
    case 'leverage': if (c.leverage <= 0n) fail(); w.n(7n, 1); w.key(c.market); w.n(c.leverage, 8); w.n(c.goodUntil, 8); break;
    default: fail();
  }
}
/** Canonical Ed25519 preimage, not a native venue signature. */
export function signingMessage(e: Envelope): Uint8Array {
  const w = new Writer('CINDER-API\0\x00\x01'); prefix(w, e);
  if (e.epoch <= 0n) fail();
  w.n(e.epoch, 8); w.key(e.signer); w.key(e.session); w.n(e.expiresAt, 8); command(w, e.command);
  return w.end();
}
/** Excludes session/epoch/auth expiry; includes immutable financial deadlines. */
export async function intentDigest(i: Intent): Promise<Uint8Array> {
  const w = new Writer('CINDER-API-INTENT\0\x00\x01'); prefix(w, i); command(w, i.command);
  const bytes = w.end();
  try { return new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)); }
  finally { bytes.fill(0); }
}
class Reader {
  at: number; bytes: Uint8Array;
  constructor(bytes: Uint8Array) {
    const p = utf8.encode('CINDER-API-REPLY\0\x00\x01');
    if (!(bytes instanceof Uint8Array) || bytes.length > 1_048_576 || !equal(bytes.slice(0, p.length), p)) fail();
    this.at = p.length; this.bytes = bytes;
  }
  take(n: number) { if (this.at + n > this.bytes.length) fail(); const b = this.bytes.slice(this.at, this.at + n); this.at += n; return b; }
  n(bytes: number, signed = false) { let n = 0n; for (const x of this.take(bytes)) n = (n << 8n) | BigInt(x); return signed ? BigInt.asIntN(bytes * 8, n) : n; }
  boolean() { const n = this.n(1); if (n > 1n) fail(); return n === 1n; }
  count() { const n = this.n(8); if (n > 4096n) fail(); return Number(n); }
  done() { if (this.at !== this.bytes.length) fail(); }
}
/** Strict bounded replies; cash/funding/margin/fee cap are not conflated. */
export function decodeResponse(bytes: Uint8Array): Response {
  const r = new Reader(bytes); const tag = r.n(1); let out: Response;
  if (tag === 0n) {
    const epoch = r.n(8), cash = r.n(16, true), funding = r.n(16, true), held = r.n(16, true);
    const positions = Array.from({ length: r.count() }, () => ({ market: r.take(32), precision: Number(r.n(4)), lots: r.n(8, true), basis: r.n(16, true) }));
    const operations = Array.from({ length: r.count() }, () => r.take(32));
    if (epoch === 0n || held < 0n || positions.some(p => p.precision === 0 || p.market.every(x => x === 0))) fail();
    out = { kind: 'view', epoch, cash, funding, held, positions, operations };
  } else if (tag === 1n) {
    const requestId = r.take(32), digest = r.take(32), policy = Number(r.n(4));
    const outcomes: Outcome[] = ['rejected', 'accepted', 'dispatched', 'acknowledged', 'partial', 'complete', 'unknown'];
    const outcome = outcomes[Number(r.n(1))]; if (!outcome || policy === 0 || requestId.every(x => x === 0)) fail();
    out = { kind: 'receipt', id: requestId, digest, policy, outcome, filled: r.n(8, true), paid: r.n(16, true),
      railFees: r.n(16, true), feeCap: r.n(16, true), possiblyExposed: r.boolean(), allowPartial: r.boolean() };
    if (out.paid < 0n || out.railFees < 0n || out.feeCap < 0n) fail();
  } else if (tag === 2n) {
    const codes: ErrorCode[] = ['invalid', 'unauthorized', 'not-found', 'conflict', 'unavailable'];
    const code = codes[Number(r.n(1))]; if (!code) fail(); out = { kind: 'error', code };
  } else fail();
  r.done(); return out;
}
export class PrivateClient {
  readonly #domain: Domain; readonly #account: Id; readonly #policy: number;
  readonly #channel: ConfidentialChannel; readonly #signer: MessageSigner;
  constructor(channel: ConfidentialChannel, signer: MessageSigner, config: { domain: Domain; account: Id; policy: number }) {
    this.#domain = { network: id(config.domain.network), deployment: id(config.domain.deployment) };
    this.#account = id(config.account); this.#policy = config.policy; this.#channel = channel; this.#signer = signer;
  }
  /** No retries or native nonce generation. On uncertainty query the original ID.
   * Caller supplies current epoch and re-signs on a new confidential session. */
  async request(input: { id: Id; epoch: bigint; expiresAt: bigint; command: Command }): Promise<Response> {
    const ctx = this.#channel.context();
    if (!equal(ctx.network, this.#domain.network) || !equal(ctx.deployment, this.#domain.deployment)
      || input.expiresAt > ctx.expiresAt) throw new Error('Confidential session mismatch');
    const env: Envelope = { domain: this.#domain, account: this.#account, id: id(input.id), policy: this.#policy,
      command: input.command, epoch: input.epoch, signer: id(this.#signer.publicKey), session: id(ctx.binding), expiresAt: input.expiresAt };
    const kind = input.command.kind, target = kind === 'operation' ? id((input.command as { target: Id }).target) : env.id.slice();
    const message = signingMessage(env), wire = new Uint8Array(message.length + 64);
    const digest = await intentDigest(env);
    try {
      wire.set(message); const toSign = message.slice(); let signature: Uint8Array;
      try { signature = await this.#signer.signMessage(toSign); } finally { toSign.fill(0); }
      if (!(signature instanceof Uint8Array) || signature.length !== 64) fail();
      wire.set(signature, message.length);
      const reply = await this.#channel.exchange(wire);
      try {
        const response = decodeResponse(reply);
        if (response.kind === 'error') return response;
        if (kind === 'view') { if (response.kind !== 'view') fail(); }
        else if (response.kind !== 'receipt' || response.policy !== this.#policy || !equal(response.id, target)
          || (kind !== 'operation' && !equal(response.digest, digest))) fail();
        return response;
      } finally { reply.fill(0); }
    } catch {
      throw new Error('Private request unavailable; reconcile the stable operation before retry');
    } finally { message.fill(0); wire.fill(0); digest.fill(0); }
  }
}
