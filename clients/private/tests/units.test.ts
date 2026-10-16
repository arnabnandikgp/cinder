// Offline v2 contract only. No shipping service, wallet, native venue or network.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash, createPrivateKey, createPublicKey, sign, verify } from 'node:crypto';
import { signingMessage, intentDigest, unitSigningMessage, unitIntentDigest, signUnitRequest, decodeUnitResponse, decodeResponse, UnitClient,
  type UnitEnvelope, type Command } from '../src/index.ts';

const id = (n:number) => new Uint8Array(32).fill(n);
const secret = createPrivateKey({ key: Buffer.concat([Buffer.from('302e020100300506032b657004220420','hex'),Buffer.alloc(32,9)]), format:'der',type:'pkcs8' });
const publicKey = new Uint8Array(createPublicKey(secret).export({format:'der',type:'spki'}).subarray(-32));
const signer = { publicKey, async signMessage(bytes:Uint8Array){return new Uint8Array(sign(null,bytes,secret));} };
const domain = {network:id(1),deployment:id(2)};
const commands:Command[] = [{kind:'view'},{kind:'operation',target:id(40)},
  {kind:'order',market:id(7),lots:-2n,minimum:90n,maximum:9007199254740997n,fee:1n<<100n,tif:'IOC',reduceOnly:true,goodUntil:90n},
  {kind:'cancel',target:id(40),attempt:id(50),goodUntil:90n},
  {kind:'payout',net:10n,maximumFee:2n,allowPartial:true,goodUntil:90n},
  {kind:'grant',grant:{key:publicKey,methods:7,market:id(7),maximumLots:2n,maximumFee:1n,maximumOrders:1n,expiresAt:90n}},
  {kind:'revoke'},{kind:'leverage',market:id(7),leverage:20000n,goodUntil:90n}];
function envelope(command:Command=commands[2],unit=40):UnitEnvelope {
  return {domain,account:id(1),id:id(40),policy:1,epoch:1n,signer:publicKey,session:id(8),expiresAt:100n,command,unit:id(unit)};
}
const magic = Buffer.from('CINDER-API\0\x00\x01'), economicEnd = magic.length + 132, commandStart = economicEnd + 80;
const hex = (b:Uint8Array) => Buffer.from(b).toString('hex');
const wrap = (body:Uint8Array,unit=40) => new Uint8Array(Buffer.concat([Buffer.from('CINDER-API-REPLY\0\x00\x02'),Buffer.from(id(unit)),body]));
const viewReply = () => new Uint8Array(Buffer.concat([Buffer.from('CINDER-API-REPLY\0\x00\x01\x00'),Buffer.from([0,0,0,0,0,0,0,1]),Buffer.alloc(64)]));

test('UnitClient signs explicit unit on requests/updates and rejects legacy or different-unit replies without retry',async()=>{
  let calls=0,seen:Uint8Array|undefined,reply=wrap(viewReply());
  const channel={context:()=>({...domain,binding:id(8),expiresAt:100n}),async exchange(wire:Uint8Array){calls++;seen=wire.slice();return reply.slice();}};
  const client=new UnitClient(channel,signer,{domain,account:id(1),policy:1,unit:id(40)});
  const input={id:id(40),epoch:1n,expiresAt:90n,command:{kind:'view'} as Command};
  assert.equal((await client.request(input)).kind,'view');
  assert.ok(seen);assert.equal(seen[magic.length-1],2);assert.deepEqual(seen.subarray(economicEnd,economicEnd+32),id(40));
  reply=wrap(viewReply(),50);await assert.rejects(client.request(input));assert.equal(calls,2);
  reply=viewReply();await assert.rejects(client.request(input));assert.equal(calls,3);
  reply=wrap(new Uint8Array(Buffer.from('CINDER-API-REPLY\0\x00\x01\x02\x04')));
  assert.deepEqual(await client.request(input),{kind:'error',code:'unavailable'});assert.equal(calls,4);
});

test('recorded-only receipt extension is accepted only in an explicit unit envelope',()=>{
  const body=Buffer.concat([Buffer.from('CINDER-API-REPLY\0\x00\x01\x01'),Buffer.from(id(40)),Buffer.from(id(41)),Buffer.from([0,0,0,1,7]),Buffer.alloc(58)]);
  const r=decodeUnitResponse(wrap(body),id(40)).response;
  assert.equal(r.kind,'receipt');if(r.kind!=='receipt')throw Error();assert.equal(r.outcome,'recorded');
  assert.throws(()=>decodeResponse(body));
});

test('v2 unit protocol matches manual extensions of shared Rust vectors with independent Node signatures',async()=>{
  const vectors=readFileSync(new URL('../../../crates/api/tests/fixtures/vectors.tsv',import.meta.url),'utf8').trim().split('\n');
  assert.equal(vectors.length,commands.length);
  for(const [index,row] of vectors.entries())for(const which of [40,50]){
    const fields=row.split('\t'),old=Buffer.from(fields[1],'hex'),e=envelope(commands[index],which);
    const prefix=Buffer.from(old.subarray(0,economicEnd));prefix[magic.length-1]=2;
    const expected=Buffer.concat([prefix,Buffer.from(id(which)),old.subarray(economicEnd)]);
    const expectedIntent=Buffer.concat([Buffer.from('CINDER-API-INTENT\0\x00\x02'),old.subarray(magic.length,economicEnd),Buffer.from(id(which)),old.subarray(commandStart)]);
    assert.deepEqual(unitSigningMessage(e),new Uint8Array(expected));
    assert.equal(hex(await unitIntentDigest(e)),createHash('sha256').update(expectedIntent).digest('hex'));
    const result=await signUnitRequest(e,signer);
    assert.deepEqual(result.wire.subarray(0,-64),new Uint8Array(expected));
    assert.deepEqual(result.wire.subarray(-64),new Uint8Array(sign(null,expected,secret)));
    assert.equal(verify(null,result.wire.subarray(0,-64),createPublicKey(secret),result.wire.subarray(-64)),true);
    assert.notEqual(hex(unitSigningMessage(e)),fields[1]);assert.notEqual(hex(result.digest),fields[3]);
  }
});

test('stable v2 economics include unit and financial deadline but not auth refresh',async()=>{
  const e=envelope(),digest=await unitIntentDigest(e);
  const refreshed:UnitEnvelope={...e,epoch:2n,signer:id(10),session:id(11),expiresAt:110n};
  assert.deepEqual(await unitIntentDigest(refreshed),digest);
  assert.notDeepEqual(unitSigningMessage({...e,session:id(11)}),unitSigningMessage(e));
  assert.notDeepEqual(await unitIntentDigest({...e,unit:id(50)}),digest);
  assert.notDeepEqual(await intentDigest(e),digest);
  assert.equal(e.command.kind,'order');if(e.command.kind!=='order')throw Error();
  assert.notDeepEqual(await unitIntentDigest({...e,command:{...e.command,goodUntil:91n}}),digest);
});

test('v2 read command bodies preserve all existing query families and exact bounds',()=>{
  const families=['account','positions','operations','openOrders','fills','funding','movements','grants','markets','updates','ingress'] as const;
  const base=unitSigningMessage(envelope({kind:'view'})).subarray(0,-1);
  for(const [tag,family] of families.entries()){
    const message=unitSigningMessage(envelope({kind:'read',query:{family,limit:64}}));
    assert.deepEqual(message.subarray(0,base.length),base);
    assert.deepEqual(message.subarray(base.length),Uint8Array.from([8,1,tag,...new Uint8Array(40),0,64]));
  }
  for(const unit of [new Uint8Array(31),new Uint8Array(32),new Uint8Array(33)])assert.throws(()=>unitSigningMessage({...envelope(),unit}));
  for(const c of [{...commands[2],lots:1n<<63n},{...commands[2],fee:-1n},{...commands[2],tif:'market'}])
    assert.throws(()=>unitSigningMessage(envelope(c as Command)));
  assert.throws(()=>unitSigningMessage(envelope({kind:'read',query:{family:'account',limit:65}})));
});

test('signing snapshots mutable input before awaits and never aliases Node Buffer identifiers',async()=>{
  const e=envelope({kind:'order',market:id(7),lots:2n,minimum:90n,maximum:110n,fee:1n,tif:'IOC',reduceOnly:false,goodUntil:90n});
  e.unit=Buffer.from(e.unit);e.id=Buffer.from(e.id);e.session=Buffer.from(e.session);
  const expectedMessage=unitSigningMessage(e),expectedDigest=await unitIntentDigest(e);
  let scratch:Uint8Array|undefined,signs=0;
  const pending=signUnitRequest(e,{publicKey,async signMessage(bytes){signs++;scratch=bytes;return signer.signMessage(bytes);}});
  e.unit.fill(50);e.id.fill(41);e.session.fill(9);e.policy=2;
  if(e.command.kind!=='order')throw Error();e.command.lots=99n;
  const result=await pending;
  assert.equal(signs,1);assert.deepEqual(result.unit,id(40));assert.deepEqual(result.id,id(40));assert.equal(result.policy,1);
  assert.deepEqual(result.digest,expectedDigest);assert.deepEqual(result.wire.subarray(0,-64),expectedMessage);
  assert.ok(scratch?.every(x=>x===0));assert.deepEqual(new Uint8Array(e.unit),id(50));assert.deepEqual(new Uint8Array(e.id),id(41));
  result.unit.fill(0);assert.deepEqual(new Uint8Array(e.unit),id(50));
});

test('signing failures are redacted with no retry and erase the wallet scratch',async()=>{
  let calls=0,scratch:Uint8Array|undefined;
  await assert.rejects(signUnitRequest(envelope(),{publicKey,async signMessage(bytes){calls++;scratch=bytes;throw Error('private wallet detail');}}),
    e=>e instanceof Error&&!e.message.includes('detail')&&e.message==='Private unit signing unavailable');
  assert.equal(calls,1);assert.ok(scratch?.every(x=>x===0));
  await assert.rejects(signUnitRequest(envelope(),{publicKey,async signMessage(){return new Uint8Array(63);}}));
  await assert.rejects(signUnitRequest(envelope(),{publicKey:id(10),async signMessage(){calls++;return new Uint8Array(64);}}));
  assert.equal(calls,1);
});

test('unit replies reject another unit, legacy fallback, trailing/truncated and oversized bodies',()=>{
  const body=Buffer.from('CINDER-API-REPLY\0\x00\x01\x02\x01');
  const wrap=(unit:Uint8Array)=>new Uint8Array(Buffer.concat([Buffer.from('CINDER-API-REPLY\0\x00\x02'),Buffer.from(unit),body]));
  const bytes=wrap(id(40));
  assert.deepEqual(decodeUnitResponse(bytes,id(40)),{unit:id(40),response:{kind:'error',code:'unauthorized'}});
  assert.throws(()=>decodeUnitResponse(bytes,id(50)));assert.throws(()=>decodeUnitResponse(new Uint8Array(body),id(40)));
  for(let n=0;n<bytes.length;n++)assert.throws(()=>decodeUnitResponse(bytes.subarray(0,n),id(40)));
  assert.throws(()=>decodeUnitResponse(new Uint8Array([...bytes,0]),id(40)));
  assert.throws(()=>decodeUnitResponse(new Uint8Array(1_048_577),id(40)));
  assert.throws(()=>decodeUnitResponse(wrap(new Uint8Array(32)),new Uint8Array(32)));
  const parsed=decodeUnitResponse(bytes,id(40));bytes.fill(0);assert.deepEqual(parsed.unit,id(40));
});

test('v1 helpers remain byte-identical and do not implicitly select a unit',()=>{
  const e=envelope();const legacy=signingMessage(e),modern=unitSigningMessage(e);
  assert.deepEqual(legacy.subarray(0,magic.length),new Uint8Array(magic));
  assert.equal(legacy[magic.length-1],1);assert.equal(modern[magic.length-1],2);
  assert.equal(modern.length,legacy.length+32);
});
