// LOCAL QUALIFICATION ONLY. No service trusts this module yet.
import { document, bytes, equal, signatureInput } from './encoding.mjs';
import { policy, userData, prologue } from './profile.mjs';
import { verifiedLeaf } from './certificates.mjs';

export async function verifyWithRoot(quote, p, c, trustedRoot) {
  // Snapshot public inputs before the first asynchronous operation; caller
  // mutation must not change the approved key/context after verification.
  bytes(quote);
  if (!quote.length || quote.length > 16384) throw Error('quote bound');
  quote = Uint8Array.from(quote);
  p = Uint8Array.from(policy(p));
  if (!bytes(trustedRoot).length || trustedRoot.length > 1024) throw Error('root bound');
  trustedRoot = Uint8Array.from(trustedRoot);
  c = { now: c.now, expires: c.expires, nonce: Uint8Array.from(bytes(c.nonce,32)),
    boot: Uint8Array.from(bytes(c.boot,32)), handle: Uint8Array.from(bytes(c.handle,32)), key: Uint8Array.from(bytes(c.key,32)) };
  const d = document(quote);
  policy(p);
  if (!Number.isSafeInteger(c.now) || c.now < 0 || d.timestamp > c.now || c.now - d.timestamp > 30000
      || !Number.isSafeInteger(c.expires) || c.expires <= c.now || c.expires - d.timestamp > 120000
      || !equal(d.publicKey, bytes(c.key, 32)) || !equal(d.nonce, bytes(c.nonce, 32))
      || ![0,1,2].every(i => equal(d.pcrs[i], p.subarray(96 + i * 48, 144 + i * 48)))
      || !equal(d.data, await userData(p, c))) throw Error('quote context');
  const leaf = await verifiedLeaf(d, trustedRoot, c.now);
  const spki = leaf.subjectPublicKeyInfo.toSchema().toBER(false);
  const key = await crypto.subtle.importKey('spki', spki, { name: 'ECDSA', namedCurve: 'P-384' }, false, ['verify']);
  if (!await crypto.subtle.verify({ name: 'ECDSA', hash: 'SHA-384' }, key, d.signature, signatureInput(d))) throw Error('COSE signature');
  return { key: Uint8Array.from(c.key), prologue: await prologue(p, c, quote) };
}
