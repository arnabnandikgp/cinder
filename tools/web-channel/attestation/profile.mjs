import { bytes, concat, uint64 } from './encoding.mjs';
const text = value => new TextEncoder().encode(value);
export const PROFILE = 'Noise_NK_25519_ChaChaPoly_SHA256';
export const digest = async (name, b) => new Uint8Array(await crypto.subtle.digest(name, b));
export function policy(value) {
  const b = bytes(value, 240);
  for (const [lo, hi] of [[0,32],[32,64],[64,96],[96,144],[144,192],[192,240]]) {
    if (!b.subarray(lo, hi).some(v => v !== 0)) throw Error('debug/empty policy');
  }
  return b;
}
export function context(p, c) {
  policy(p);
  for (const name of ['nonce', 'boot', 'handle', 'key']) {
    bytes(c[name], 32); if (!c[name].some(v => v !== 0)) throw Error('empty context');
  }
  uint64(c.expires);
  if (c.expires === 0) throw Error('empty expiry');
  return concat(text('CINDER-WEB-CONTEXT-1\0'), text(PROFILE), Uint8Array.of(0), p,
    c.nonce, c.boot, c.handle, c.key, uint64(c.expires));
}
export const userData = (p, c) => digest('SHA-384', context(p, c));
export async function prologue(p, c, quote) {
  return digest('SHA-256', concat(text('CINDER-WEB-PROLOGUE-1\0'), context(p, c), await digest('SHA-256', quote)));
}
export async function sessionBinding(p, c, quote, endpoint) {
  // binding() itself refuses before both standard key-confirmation records.
  const h = bytes(endpoint.binding(), 32);
  return digest('SHA-256', concat(text('CINDER-WEB-SESSION-1\0'), await prologue(p, c, quote), h));
}
export function challenge() {
  // Caller-owned challenge, never learned from the relay; no RNG fallback.
  const b = crypto.getRandomValues(new Uint8Array(32));
  if (!b.some(v => v !== 0)) throw Error('entropy');
  return b;
}
