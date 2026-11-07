// Node-only synthetic certificate/quote generator. No private key is exported,
// written to disk or returned with this PUBLIC fixture corpus. Not an NSM device.
import * as asn1js from 'asn1js';
import * as pki from 'pkijs';
import { execFileSync } from 'node:child_process';
import { join } from 'node:path';
import { cbor, concat, uint64 } from './encoding.mjs';
import { digest, userData, prologue } from './profile.mjs';
import capture from './aws-certificate-capture.json' with { type: 'json' };
import { serialFromEntropy } from './serial.mjs';

const cryptoEngine = new pki.CryptoEngine({ name: 'synthetic-fixture', crypto: globalThis.crypto, subtle: crypto.subtle });
const text = b => new TextEncoder().encode(b);
const now = Math.floor(Date.now() / 1000) * 1000;
const policy = concat(new Uint8Array(32).fill(1), new Uint8Array(32).fill(2), new Uint8Array(32).fill(21),
  new Uint8Array(48).fill(22), new Uint8Array(48).fill(23), new Uint8Array(48).fill(24));
const context = { nonce: new Uint8Array(32).fill(31), boot: new Uint8Array(32).fill(32),
  handle: new Uint8Array(32).fill(33), key: new Uint8Array(32).fill(34), now, expires: now + 120000 };
const exporter = new Uint8Array(32).fill(35);

function extension(oid, value, critical = false) {
  return new pki.Extension({ extnID: oid, critical, extnValue: value.toBER(false) });
}
async function certificate(name, parent, { ca = false, pathLen = ca ? 2 : undefined,
  aki = true, usage = ca ? 6 : 128, before = now - 3600000, after = now + 3600000,
  extra = [], duplicate = false, badAki = false, curve = 'P-384', hash = 'SHA-384', skiPresent = true, kuCritical = true, bcCritical = true } = {}) {
  const keys = await crypto.subtle.generateKey({ name: 'ECDSA', namedCurve: curve }, false, ['sign', 'verify']);
  const cert = new pki.Certificate();
  cert.version = 2;
  const serial = serialFromEntropy(crypto.getRandomValues(new Uint8Array(16)));
  cert.serialNumber = new asn1js.Integer({ valueHex: serial.buffer });
  cert.subject = new pki.RelativeDistinguishedNames({ typesAndValues: [
    new pki.AttributeTypeAndValue({ type: '2.5.4.3', value: new asn1js.Utf8String({ value: name }) }),
  ] });
  cert.issuer = parent?.cert.subject ?? cert.subject;
  cert.notBefore = new pki.Time({ type: 0, value: new Date(before) });
  cert.notAfter = new pki.Time({ type: 0, value: new Date(after) });
  await cert.subjectPublicKeyInfo.importKey(keys.publicKey, cryptoEngine);
  const ski = (await digest('SHA-256', new Uint8Array(await crypto.subtle.exportKey('spki', keys.publicKey)))).slice(0, 20);
  const bc = new pki.BasicConstraints({ cA: ca, ...(pathLen === undefined ? {} : { pathLenConstraint: pathLen }) });
  const unusedBits = usage ? Math.min(7, Math.log2(usage & -usage)) : 0;
  cert.extensions = [extension('2.5.29.19', bc.toSchema(), bcCritical),
    extension('2.5.29.15', new asn1js.BitString({ valueHex: Uint8Array.of(usage).buffer, unusedBits }), kuCritical)];
  if (skiPresent) cert.extensions.push(extension('2.5.29.14', new asn1js.OctetString({ valueHex: ski.buffer })));
  if (aki) cert.extensions.push(extension('2.5.29.35', new pki.AuthorityKeyIdentifier({
    keyIdentifier: new asn1js.OctetString({ valueHex: (badAki ? new Uint8Array(20).fill(99) : parent?.ski ?? ski).buffer }),
  }).toSchema()));
  cert.extensions.push(...extra);
  if (duplicate) cert.extensions.push(cert.extensions[0]);
  await cert.sign(parent?.keys.privateKey ?? keys.privateKey, hash, cryptoEngine);
  return { cert, keys, ski, der: new Uint8Array(cert.toSchema(true).toBER(false)) };
}
function fields(path, at, key, nonce, data) {
  return new Map([['module_id', 'LOCAL-FIXTURE-NOT-NITRO'], ['digest', 'SHA384'], ['timestamp', at],
    ['pcrs', new Map([0,1,2].map(i => [i, policy.slice(96 + i * 48, 144 + i * 48)]))],
    ['certificate', path.at(-1).der], ['cabundle', path.slice(0, -1).map(p => p.der)],
    ['public_key', key], ['nonce', nonce], ['user_data', data]]);
}
async function quote(path, map, { indefinite = true, tagged = true } = {}) {
  let payload = cbor(map);
  if (indefinite) { payload[0] = 0xbf; payload = concat(payload, Uint8Array.of(255)); }
  const protectedBytes = Uint8Array.of(0xa1,1,0x38,0x22);
  const input = cbor(['Signature1', protectedBytes, new Uint8Array(), payload]);
  let signature = new Uint8Array(await crypto.subtle.sign({ name: 'ECDSA', hash: 'SHA-384' }, path.at(-1).keys.privateKey, input));
  // Wrong-curve negative is deliberately not a valid ES384 signature.
  if (signature.length !== 96) signature = concat(signature, new Uint8Array(96 - signature.length));
  return concat(tagged ? Uint8Array.of(0xd2) : new Uint8Array(), cbor([protectedBytes, new Map(), payload, signature]));
}
function serialize(p, c) {
  return { policy: [...p], context: Object.fromEntries(Object.entries(c).map(([k,v]) => [k, v instanceof Uint8Array ? [...v] : v])) };
}
async function nativeAccept(path, map, c, p, oracle) {
  // Same synthetic certificate and COSE payload, but TLS-purpose user_data for
  // the existing independently compiled native verifier's exact input contract.
  const data = await digest('SHA-384', concat(text('CINDER-TLS-ATTESTATION-1\0'), p.slice(0,64), p.slice(64,96), c.boot, uint64(c.expires), exporter));
  const nativeMap = new Map(map); nativeMap.set('user_data', data);
  const wire = await quote(path, nativeMap);
  const header = concat(p, c.nonce, exporter, c.boot, uint64(c.expires), uint64(c.now), Uint8Array.of(0,0,0,c.key.length), c.key, wire);
  try {
    const result = execFileSync(oracle, [Buffer.from(path[0].der).toString('hex')], { input: header, timeout: 5000, stdio: ['pipe', 'pipe', 'ignore'] });
    return result.length === 32;
  } catch (error) { if (error.status === 1) return false; throw error; }
}
export async function fixtures(repo) {
  const oracle = join(repo, 'target/debug/cinder-verify-fixture');
  const root = await certificate('fixture-root', undefined, { ca: true });
  const leaf = await certificate('fixture-leaf', root);
  const cases = [];
  async function add(name, path = [root, leaf], { expected = false, native = expected,
    ctx = context, p = policy, mutate = () => {}, framing = {} } = {}) {
    const map = fields(path, now, context.key, context.nonce, await userData(policy, context));
    mutate(map);
    const observed = await nativeAccept(path, map, ctx, p, oracle);
    if (observed !== native) throw Error(`Native fixture oracle disagrees: ${name} expected ${native}, observed ${observed}`);
    cases.push({ name, expected, native: observed, root: [...path[0].der], quote: [...await quote(path, map, framing)], ...serialize(p, ctx) });
  }
  await add('fresh signed quote', undefined, { expected: true });
  await add('definite map without outer tag', undefined, { expected: true, framing: { indefinite: false, tagged: false } });
  await add('AWS-shaped leaf without AKI', [root, await certificate('fixture-no-aki', root, { aki: false })], { expected: true });
  await add('AWS-shaped leaf without identifiers/noncritical KU', [root, await certificate('fixture-no-identifiers', root,
    { aki: false, skiPresent: false, kuCritical: false, usage: 192 })], { expected: true });
  const intermediate = await certificate('fixture-intermediate', root, { ca: true, pathLen: 0 });
  const chain = [root, intermediate, await certificate('fixture-chain-leaf', intermediate, { aki: false })];
  await add('intermediate CA path', chain, { expected: true });
  const crlMetadata = pki.Certificate.fromBER(Buffer.from(capture.chain[1],'base64')).extensions.find(e=>e.extnID==='2.5.29.31');
  const withCrlMetadata = await certificate('fixture-CRL-metadata',root,{ca:true,pathLen:0,extra:[crlMetadata]});
  await add('noncritical AWS CRL distribution metadata', [root,withCrlMetadata,await certificate('leaf',withCrlMetadata)], {expected:true});
  for (const key of ['nonce', 'key', 'boot', 'handle']) {
    const ctx = { ...context, [key]: new Uint8Array(32).fill(90) };
    // TLS oracle has no outer handle; changing a context boot changes its TLS
    // quote too. These are WEB-purpose substitution tests, not native failures.
    await add(`substituted ${key}`, undefined, { ctx, native: key === 'nonce' || key === 'key' ? false : true });
  }
  for (const [name, ctx] of [['expired session', { ...context, expires: now }],
    ['excess lifetime', { ...context, expires: now + 120001 }], ['future quote', { ...context, now: now - 1 }],
    ['stale quote', { ...context, now: now + 30001 }]]) await add(name, undefined, { ctx });
  for (const [name, at, native] of [['network',0,true],['deployment',32,true],['manifest',64,true],['PCR',96,false]]) {
    const p = policy.slice(); p[at] ^= 1;
    // Oracle's TLS-purpose data is built from its supplied policy; the original
    // Web-purpose data is intentionally fixed, so only browser accepts neither.
    await add(`wrong ${name}`, undefined, { p, native });
  }
  const debug = policy.slice(); debug.fill(0,96,144);
  await add('debug policy', undefined, { p: debug });
  await add('wrong signed nonce', undefined, { mutate: m => m.set('nonce', new Uint8Array(32).fill(99)) });
  await add('wrong signed PCR', undefined, { mutate: m => m.get('pcrs').set(1, new Uint8Array(48).fill(99)) });
  await add('wrong signed key', undefined, { mutate: m => m.set('public_key', new Uint8Array(32).fill(99)) });
  for (const [name, options] of [
    ['expired leaf', { after: now }], ['future leaf', { before: now + 1000 }],
    ['leaf unknown critical extension', { extra: [extension('1.2.3.4', new asn1js.Null(), true)] }],
    ['leaf duplicate extension', { duplicate: true }], ['leaf AKI mismatch', { badAki: true }],
    ['wrong leaf curve', { curve: 'P-256' }],
  ]) await add(name, [root, await certificate(name, root, options)]);
  const restrictedRoot = await certificate('path-zero-root', undefined, { ca: true, pathLen: 0 });
  const restrictedIntermediate = await certificate('path-zero-intermediate', restrictedRoot, { ca: true, pathLen: 0 });
  await add('root path length exceeded', [restrictedRoot, restrictedIntermediate, await certificate('path-zero-leaf', restrictedIntermediate)]);
  const childCa = await certificate('child-CA',intermediate,{ca:true,pathLen:0});
  await add('intermediate path length exceeded',[root,intermediate,childCa,await certificate('leaf',childCa)]);
  for (const [name, options] of [['issuer not CA', { ca: false }], ['issuer lacks keyCertSign', { ca: true, usage: 128 }],
    ['expired issuer', { ca: true, after: now }], ['issuer missing AKI', { ca: true, aki: false }],
    ['issuer noncritical CA constraints',{ca:true,bcCritical:false}]]) {
    const parent = await certificate(name, root, options);
    await add(name, [root, parent, await certificate('leaf', parent)]);
  }
  const expiredRoot = await certificate('expired-root', undefined, { ca: true, after: now });
  await add('expired root', [expiredRoot, await certificate('leaf', expiredRoot)]);
  const futureRoot = await certificate('future-root', undefined, { ca: true, before: now + 1000 });
  await add('future root', [futureRoot, await certificate('leaf', futureRoot)]);
  const corruptLeaf = {...leaf,der:leaf.der.slice()}; corruptLeaf.der[corruptLeaf.der.length-1]^=1;
  await add('invalid leaf certificate signature',[root,corruptLeaf]);
  const corruptIssuer = {...intermediate,der:intermediate.der.slice()}; corruptIssuer.der[corruptIssuer.der.length-1]^=1;
  await add('invalid intermediate certificate signature',[root,corruptIssuer,chain.at(-1)]);
  // Explicit stricter subset: Node accepts ES256 certificate signatures with a
  // P384 attestation leaf; this browser candidate permits ES384 signatures only.
  await add('unsupported certificate signature (stricter subset)', [root, await certificate('SHA256-cert', root, { hash: 'SHA-256' })], { native: true });
  return cases;
}
export async function attestedFixture() {
  const root = await certificate('fresh-channel-fixture-root', undefined, { ca: true });
  const leaf = await certificate('fresh-channel-fixture-leaf', root, { aki: false });
  return async (key, nonce) => {
    const at = Date.now();
    const c = { key, nonce, now: at, expires: at + 120000,
      boot: crypto.getRandomValues(new Uint8Array(32)), handle: crypto.getRandomValues(new Uint8Array(32)) };
    const wire = await quote([root, leaf], fields([root, leaf], at, key, nonce, await userData(policy, c)));
    return { root: [...root.der], quote: [...wire], ...serialize(policy, c), prologue: await prologue(policy, c, wire) };
  };
}
