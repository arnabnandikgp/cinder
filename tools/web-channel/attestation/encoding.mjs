// Closed, bounded CBOR profile matching the native verifier. Not a general CBOR
// decoder: no tags except outer Sign1, no indefinite nested values or coercions.
export const MAX_QUOTE = 16384;
export const equal = (a, b) => a.length === b.length && a.every((v, i) => v === b[i]);
export function bytes(value, length) {
  if (!(value instanceof Uint8Array) || (length !== undefined && value.length !== length)) throw Error('bytes');
  return value;
}
export function concat(...parts) {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let at = 0;
  for (const part of parts) { out.set(part, at); at += part.length; }
  return out;
}
export function uint64(value) {
  if (!Number.isSafeInteger(value) || value < 0) throw Error('integer');
  const out = new Uint8Array(8);
  new DataView(out.buffer).setBigUint64(0, BigInt(value));
  return out;
}
export function head(kind, n) {
  if (!Number.isSafeInteger(n) || n < 0) throw Error('integer');
  if (n < 24) return Uint8Array.of(kind * 32 + n);
  const width = n <= 255 ? 1 : n <= 65535 ? 2 : n <= 4294967295 ? 4 : 8;
  return concat(Uint8Array.of(kind * 32 + ({1:24,2:25,4:26,8:27})[width]), uint64(n).slice(8 - width));
}
export function cbor(value) {
  if (value instanceof Uint8Array) return concat(head(2, value.length), value);
  if (typeof value === 'string') { const b = new TextEncoder().encode(value); return concat(head(3, b.length), b); }
  if (typeof value === 'number') return head(0, value);
  if (Array.isArray(value)) return concat(head(4, value.length), ...value.map(cbor));
  if (value instanceof Map) return concat(head(5, value.size), ...[...value].flatMap(([k, v]) => [cbor(k), cbor(v)]));
  throw Error('CBOR type');
}
class Reader {
  constructor(b) { this.b = b; this.at = 0; }
  take(n) {
    if (!Number.isSafeInteger(n) || n < 0 || n > this.b.length - this.at) throw Error('CBOR bound');
    const result = this.b.subarray(this.at, this.at + n); this.at += n; return result;
  }
  head(kind) {
    const h = this.take(1)[0], extra = h & 31;
    if (h >> 5 !== kind || extra > 27) throw Error('CBOR type');
    if (extra < 24) return extra;
    const width = 2 ** (extra - 24);
    let n = 0;
    for (const b of this.take(width)) { n = n * 256 + b; if (!Number.isSafeInteger(n)) throw Error('CBOR integer'); }
    if (n < ({1:24,2:256,4:65536,8:4294967296})[width]) throw Error('CBOR minimal');
    return n;
  }
  value(kind, cap) { const n = this.head(kind); if (n > cap) throw Error('CBOR cap'); return this.take(n); }
  text(cap) { return new TextDecoder('utf-8', { fatal: true }).decode(this.value(3, cap)); }
  done() { if (this.at !== this.b.length) throw Error('CBOR trailing'); }
}
export function document(input) {
  const b = bytes(input);
  if (!b.length || b.length > MAX_QUOTE) throw Error('quote bound');
  const outer = new Reader(b);
  if (b[0] === 0xd2) outer.take(1);
  if (outer.head(4) !== 4) throw Error('Sign1');
  const protectedBytes = outer.value(2, 16);
  if (!equal(protectedBytes, Uint8Array.of(0xa1, 1, 0x38, 0x22)) || outer.head(5) !== 0) throw Error('ES384');
  const payload = outer.value(2, MAX_QUOTE);
  const signature = outer.value(2, 96);
  if (signature.length !== 96) throw Error('signature');
  outer.done();
  const r = new Reader(payload), indefinite = payload[0] === 0xbf;
  if (indefinite) r.take(1); else if (r.head(5) !== 9) throw Error('document fields');
  const seen = new Set(), out = { protectedBytes, payload, signature, chain: [], pcrs: [] };
  for (let n = 0; n < 9; n++) {
    const k = r.text(32);
    if (seen.has(k)) throw Error('duplicate document field');
    seen.add(k);
    switch (k) {
      case 'module_id': if (!r.text(256).length) throw Error('module'); break;
      case 'digest': if (r.text(16) !== 'SHA384') throw Error('digest'); break;
      case 'timestamp': out.timestamp = r.head(0); break;
      case 'certificate': out.certificate = r.value(2, 1024); break;
      case 'cabundle': {
        const count = r.head(4);
        if (!count || count > 8) throw Error('chain bound');
        for (let j = 0; j < count; j++) out.chain.push(r.value(2, 1024));
        break;
      }
      case 'pcrs': {
        const count = r.head(5), indices = new Set();
        if (count < 3 || count > 32) throw Error('PCR bound');
        for (let j = 0; j < count; j++) {
          const index = r.head(0), value = r.value(2, 48);
          if (index > 31 || indices.has(index) || value.length !== 48) throw Error('PCR');
          indices.add(index); if (index < 3) out.pcrs[index] = value;
        }
        if (![0, 1, 2].every(i => out.pcrs[i])) throw Error('required PCR');
        break;
      }
      case 'public_key': out.publicKey = r.value(2, 1024); break;
      case 'nonce': out.nonce = r.value(2, 32); break;
      case 'user_data': out.data = r.value(2, 512); break;
      default: throw Error('unknown document field');
    }
  }
  if (indefinite && r.take(1)[0] !== 0xff) throw Error('map break');
  r.done();
  if (out.timestamp === undefined || !out.certificate || !out.publicKey || !out.nonce || !out.data) throw Error('missing document field');
  return out;
}
export const signatureInput = d => cbor(['Signature1', d.protectedBytes, new Uint8Array(), d.payload]);
