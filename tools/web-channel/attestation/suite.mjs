import { verifyWithRoot } from './verifier.mjs';
import { verifyWebQuote } from './client.mjs';
import { digest, context, userData, prologue, sessionBinding, challenge } from './profile.mjs';
import { root, fingerprint } from './aws-root.mjs';
import { document, equal, concat, cbor, head } from './encoding.mjs';
import { verifiedLeaf } from './certificates.mjs';
import capture from './aws-certificate-capture.json' with { type: 'json' };

const assert = (condition, name) => { if (!condition) throw Error(name); };
const hex = b => [...b].map(v => v.toString(16).padStart(2, '0')).join('');
const decode = f => ({ p: Uint8Array.from(f.policy), q: Uint8Array.from(f.quote), r: Uint8Array.from(f.root),
  c: Object.fromEntries(Object.entries(f.context).map(([k,v]) => [k, Array.isArray(v) ? Uint8Array.from(v) : v])) });
async function refusal(call, name) {
  try { await call(); } catch { return; }
  throw Error(`Accepted ${name}`);
}
export async function runAttestation(core, fixtures, base = '') {
  const checks = [];
  assert(hex(await digest('SHA-256', root())) === fingerprint, 'same pinned AWS root');
  const historical = { chain: capture.chain.map(b => Uint8Array.from(atob(b),c=>c.charCodeAt(0))),
    certificate: Uint8Array.from(atob(capture.certificate),c=>c.charCodeAt(0)) };
  await verifiedLeaf(historical,root(),capture.capturedAt);
  await refusal(() => verifiedLeaf(historical,root(),capture.capturedAt + 86400000), 'expired historical AWS leaf');
  checks.push('historical five-certificate AWS path at capture time only, not fresh web attestation');
  for (const f of fixtures) {
    const { p, q, r, c } = decode(f);
    if (f.expected) {
      const approval = await verifyWithRoot(q, p, c, r);
      assert(equal(approval.key, c.key) && approval.prologue.length === 32, f.name);
    } else await refusal(() => verifyWithRoot(q, p, c, r), f.name);
  }
  checks.push(`${fixtures.length} signed WebCrypto fixtures; independent native certificate/context oracle`);
  const f = decode(fixtures[0]), d = document(f.q);
  const changing = decode(fixtures[0]), originalKey = changing.c.key.slice();
  const pending = verifyWithRoot(changing.q,changing.p,changing.c,changing.r);
  changing.q.fill(0); changing.p.fill(0); changing.r.fill(0); changing.c.key.fill(0);
  assert(equal((await pending).key,originalKey),'approval snapshots public inputs before asynchronous verification');
  // The AWS-only entry point never accepts this separately rooted corpus.
  try { await verifyWebQuote(f.q, f.p, f.c); throw Error('fixture accepted by AWS-only entry point'); }
  catch (error) { assert(error.message === 'Web channel unavailable', 'AWS-only redaction'); }
  const badSignature = f.q.slice(); badSignature[badSignature.length - 1] ^= 1;
  await refusal(() => verifyWithRoot(badSignature, f.p, f.c, f.r), 'COSE signature tamper');
  const wrongRoot = f.r.slice(); wrongRoot[wrongRoot.length - 1] ^= 1;
  await refusal(() => verifyWithRoot(f.q, f.p, f.c, wrongRoot), 'root substitution');
  const signedPayload = d.payload.slice(); signedPayload[20] ^= 1;
  await refusal(() => verifyWithRoot(cbor([d.protectedBytes, new Map(), signedPayload, d.signature]), f.p, f.c, f.r), 'signed payload modification');
  checks.push('pinned AWS-only root, COSE signature/payload tamper and root substitution');
  const fields = [['module_id','fixture'],['digest','SHA384'],['timestamp',f.c.now],
    ['pcrs', new Map([0,1,2].map(i => [i, f.p.slice(96 + i*48, 144 + i*48)]))],
    ['certificate',d.certificate],['cabundle',d.chain],['public_key',d.publicKey],['nonce',d.nonce],['user_data',d.data]];
  const duplicate = fields.slice(); duplicate[8] = fields[7];
  const unknown = fields.slice(); unknown[0] = ['unknown_field','fixture'];
  const invalidPcr = fields.slice(); invalidPcr[3] = ['pcrs',new Map([[0,new Uint8Array(48)],[1,new Uint8Array(48)],[3,new Uint8Array(48)]])];
  const map = pairs => concat(head(5, pairs.length), ...pairs.flatMap(([k,v])=>[cbor(k),cbor(v)]));
  const raw = payload => cbor([d.protectedBytes, new Map(), payload, d.signature]);
  const duplicatePcr = concat(head(5,3),cbor(0),cbor(f.p.slice(96,144)),cbor(0),cbor(f.p.slice(96,144)),cbor(2),cbor(f.p.slice(192,240)));
  const withRawField = (key,value) => concat(head(5,9),...fields.flatMap(([k,v])=>[cbor(k),k===key?value:cbor(v)]));
  const malformed = [new Uint8Array(), new Uint8Array(16385), concat(f.q,Uint8Array.of(0)),
    cbor([Uint8Array.of(0xa1,1,0x26),new Map(),d.payload,d.signature]),
    cbor([d.protectedBytes,new Map([[4,Uint8Array.of(1)]]),d.payload,d.signature]),
    cbor([d.protectedBytes,new Map(),d.payload,new Uint8Array(95)]),
    raw(map(duplicate)),raw(map(unknown)),raw(map(invalidPcr)),raw(map([...fields,['extra',1]])),
    raw(concat(Uint8Array.of(0xb8,9),map(fields).subarray(1))), // Nonminimal map count.
    raw(concat(Uint8Array.of(0xbf),map(fields).subarray(1))), // Missing break.
    raw(withRawField('pcrs',duplicatePcr)),
    raw(withRawField('nonce',concat(Uint8Array.of(0x5f),cbor(d.nonce),Uint8Array.of(255)))),
    cbor([Uint8Array.of(0xa2,1,0x38,0x22,1,0x38,0x22),new Map(),d.payload,d.signature]),
  ];
  for (const q of malformed) await refusal(() => document(q), 'closed CBOR');
  for (let n = 0; n < f.q.length; n++) await refusal(() => document(f.q.subarray(0,n)), 'quote truncation');
  checks.push('closed CBOR, duplicates/unknowns/missing PCRs, caps and every quote truncation');
  const vector = { ...f.c, expires: 1767225720000 };
  const quoteVector = Uint8Array.from({length:16},(_,i)=>i+1), h = new Uint8Array(32).fill(42);
  const encoded = context(f.p, vector), pieces = concat(vector.nonce,vector.boot,vector.handle,vector.key);
  assert(encoded.length === 430 && equal(core.web_context(f.p,pieces,BigInt(vector.expires)),encoded), 'canonical Rust/WASM context');
  assert(hex(await userData(f.p,vector)) === '5588a4acb8961b777189416b9358ab2e1f1c9ef3a2e58174ae2d77017b3dcf982485dfef29fffedbc07d529ce989b09f', 'user_data vector');
  const proposed = await prologue(f.p,vector,quoteVector);
  assert(hex(proposed) === '58c23e98eb7c0614ebfce47d12e035753ed337868f3657fae6ef675102f5fd9e', 'prologue vector');
  const binding = await sessionBinding(f.p,vector,quoteVector,{binding:()=>h});
  assert(hex(binding) === '20b05c130638b292b20ee299cabae502c67e69362ce45041e0ceb82353cce3c1', 'binding vector');
  assert(equal(core.web_user_data(encoded), await userData(f.p,vector))
    && equal(core.web_prologue(encoded,quoteVector), proposed)
    && equal(core.web_binding(proposed,h), binding), 'native/WASM/JS digest agreement');
  checks.push('fixed context/prologue/P18-sized binding vectors in shared Rust/WASM and WebCrypto');
  async function setup() {
    const nonce = challenge();
    const response = await fetch(`${base}/attested`, { method:'POST', body:nonce });
    assert(response.ok, 'synthetic attested setup');
    const result = await response.json(), {p,q,r,c} = decode(result);
    assert(equal(c.nonce,nonce), 'own challenge');
    c.now = Date.now(); // Independent test client clock, never server-provided time.
    const approval = await verifyWithRoot(q,p,c,r);
    const client = new core.BrowserEndpoint(approval.key,approval.prologue);
    return { id: result.id, client, p,q,c };
  }
  async function relay(id, wire) {
    const response = await fetch(`${base}/wire/${id}`,{method:'POST',body:wire});
    assert(response.ok,'native delivery'); return new Uint8Array(await response.arrayBuffer());
  }
  const first = await setup(), client = first.client;
  await refusal(() => sessionBinding(first.p,first.c,first.q,client), 'binding before handshake');
  const second = await relay(first.id,client.start());
  const confirmation = client.advance(second);
  await refusal(() => sessionBinding(first.p,first.c,first.q,client), 'binding before confirmation');
  client.advance(await relay(first.id,confirmation));
  assert(client.ready(),'attested synthetic ready');
  const finalized = await sessionBinding(first.p,first.c,first.q,client);
  const nativeBinding = client.open(await relay(first.id,client.seal(new TextEncoder().encode('CINDER-QUALIFICATION-BINDING'))));
  assert(equal(finalized,nativeBinding),'native/client final binding');
  const marker = new TextEncoder().encode('PRIVATE-WEB-QUALIFICATION-NO-PARENT-PLAINTEXT');
  assert(equal(client.open(await relay(first.id,client.seal(marker))),marker),'verified-key encrypted native round trip');
  client.free();
  const other = await setup();
  other.client.advance(await relay(other.id,other.client.start()));
  // Different quotes/handles cannot authorize the same context or session.
  assert(!equal(await prologue(other.p,other.c,other.q),await prologue(first.p,first.c,first.q)), 'fresh quote/session prologue');
  other.client.free();
  const changed = first.q.slice(); changed[changed.length-1]^=1;
  assert(!equal(await prologue(first.p,first.c,changed),await prologue(first.p,first.c,first.q)), 'exact quote commitment');
  checks.push('verified synthetic key -> native NK -> bidirectional confirmation -> matching binding/private echo');
  const rng = crypto.getRandomValues;
  try {
    crypto.getRandomValues = () => { throw Error('fixture RNG unavailable'); };
    await refusal(() => challenge(),'challenge entropy failure');
    crypto.getRandomValues = b => b.fill(0);
    await refusal(() => challenge(),'all-zero challenge');
  } finally { crypto.getRandomValues = rng; }
  checks.push('fresh client challenge refuses missing entropy or an all-zero provider result');
  return checks;
}
