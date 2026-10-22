// Native capability evidence only. No signer, faucet, account, RPC or AWS port.
import { sha, canonical } from '../native-qualification/core.mjs';

export const ORIGIN = 'https://exchange-api.bulk.trade';
export const PATHS = Object.freeze(['/api/v1/exchangeInfo', '/api/v1/feeState']);
export const LIMITS = Object.freeze({ requests: 2, duration_ms: 120000, request_ms: 20000, body_bytes: 2097152 });
const nonfinancial = Object.freeze({ wallets: 0, signatures: 0, faucet: 0, orders: 0, transfers: 0, rpc: 0, aws: 0, shipping_ready: false });

export function manifest(source) {
  if (!/^[0-9a-f]{64}$/.test(source)) throw Error('Source commitment required');
  return { schema: 'cinder-bulk-metadata-v1', origin: ORIGIN, paths: [...PATHS], limits: { ...LIMITS }, source, ...nonfinancial };
}
export function commitment(m) {
  if (canonical(m) !== canonical(manifest(m?.source))) throw Error('Changed metadata scope');
  return sha(canonical(m));
}
export function approval(m, a, now) {
  if (a?.manifest !== commitment(m) || a.approved !== true || typeof a.reference !== 'string'
      || !a.reference.trim() || !Number.isSafeInteger(now) || !Number.isSafeInteger(a.not_after)
      || a.not_after <= now || a.not_after - now > LIMITS.duration_ms) throw Error('Current exact metadata approval required');
}
export async function boundedBody(response) {
  if (!response.body) throw Error('Missing response body');
  const reader = response.body.getReader(), chunks = [];
  let bytes = 0;
  try {
    for (;;) {
      const item = await reader.read();
      if (item.done) break;
      bytes += item.value.length;
      if (bytes > LIMITS.body_bytes) throw Error('Response byte limit');
      chunks.push(item.value);
    }
    return Buffer.concat(chunks, bytes);
  } finally { await reader.cancel().catch(() => {}); reader.releaseLock(); }
}
function observation(path, bytes) {
  // Values are archived as original bytes. This summary cannot supply prices,
  // balances, precision, native qualification or any financial readiness.
  const value = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
  if (path === PATHS[0]) {
    if (!Array.isArray(value) || !value.length || value.length > 1024
        || value.some(v => !v || typeof v.symbol !== 'string' || !/^[A-Z0-9-]{1,32}$/.test(v.symbol))) throw Error('Unexpected market schema');
    return { market_count: value.length, symbols: value.map(v => v.symbol) };
  }
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw Error('Unexpected fee schema');
  return { object_observed: true };
}

export async function qualify(m, a, ports) {
  const start = ports.now(); approval(m, a, start);
  let last = start, used = 0;
  const time = () => {
    const t = ports.now();
    if (!Number.isSafeInteger(t) || t < last || t - start >= LIMITS.duration_ms || t >= a.not_after) throw Error('Metadata clock/deadline');
    last = t; return t;
  };
  // Exclusive durable retention must fail if this run was ever started. There
  // is no live resume/reset, even though these requests are read-only.
  ports.retain('started', { manifest: commitment(m), at: time(), ...nonfinancial });
  let outcome = 'metadata-observed';
  for (const path of PATHS) {
    const at = time();
    if (++used > LIMITS.requests) throw Error('Metadata request limit');
    ports.retain(`request-${used}`, { index: used, path, method: 'GET', at });
    let response, bytes;
    try {
      response = await ports.fetch(new URL(path, ORIGIN), {
        method: 'GET', redirect: 'error', credentials: 'omit', headers: { accept: 'application/json' },
        signal: AbortSignal.timeout(Math.min(LIMITS.request_ms, a.not_after - at, LIMITS.duration_ms - (at - start))),
      });
      bytes = await boundedBody(response);
    } catch {
      ports.retain(`unavailable-${used}`, { at: ports.now(), classification: 'transport-or-body-unavailable' });
      outcome = 'unavailable'; break;
    }
    // Never archive a credential-bearing outbound envelope. These are fixed,
    // unsigned metadata requests; response bytes are retained losslessly.
    ports.retain(`response-${used}`, { status: response.status, sha256: sha(bytes), bytes: bytes.length,
      raw_base64: bytes.toString('base64'), received_at: ports.now() });
    try { time(); } catch { outcome = 'expired'; break; }
    if (response.status !== 200) { outcome = `http-${response.status}`; break; }
    let observed;
    try { observed = observation(path, bytes); }
    catch { outcome = 'unexpected-schema'; break; }
    ports.retain(`observation-${used}`, observed);
  }
  const result = { outcome, requests: used, completed_at: ports.now(), ...nonfinancial,
    next: outcome === 'metadata-observed' ? 'review-current-grid-and-paper-run-scope' : 'investigate-availability-without-signing' };
  ports.retain('result', result);
  return result;
}
