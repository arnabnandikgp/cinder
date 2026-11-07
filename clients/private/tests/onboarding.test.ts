import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash,createPrivateKey,createPublicKey,sign,verify} from 'node:crypto';
import {OnboardingClient,UnitOnboardingClient,decodeResponse,type ConfidentialChannel} from '../src/index.ts';
const id=(n:number)=>new Uint8Array(32).fill(n),domain={network:id(1),deployment:id(2)};
const secret=createPrivateKey({key:Buffer.concat([Buffer.from('302e020100300506032b657004220420','hex'),Buffer.alloc(32,33)]),format:'der',type:'pkcs8'});
const publicKey=new Uint8Array(createPublicKey(secret).export({format:'der',type:'spki'}).subarray(-32));
const signer={publicKey,signMessage:async(b:Uint8Array)=>new Uint8Array(sign(null,b,secret))};
const number=(n:bigint,width:number)=>Buffer.from(n.toString(16).padStart(width*2,'0'),'hex');
const account=createHash('sha256').update(Buffer.concat([Buffer.from('CINDER-ACCOUNT-1\0'),Buffer.from(domain.network),Buffer.from(domain.deployment),Buffer.from(publicKey)])).digest();
function reply(pending=[id(44)]){return new Uint8Array(Buffer.concat([Buffer.from('CINDER-API-REPLY\0\x00\x01'),Buffer.from([4]),account,number(1n,8),Buffer.from(id(55)),number(2n,2),
  ...[2,10,4,11,12,13,14].map(n=>Buffer.from(id(n))),Buffer.from([6]),number(1n,8),number(1000n,8),number(BigInt(pending.length),8),...pending.map(x=>Buffer.from(x))]));}
const context=()=>({...domain,binding:id(8),expiresAt:1000n});
test('unit bootstrap independently signs custody contract and unit, rejects foreign replies without retry',async()=>{
  let calls=0;let wrong=false;let retained:Uint8Array|undefined;
  const unit=id(40),contract=id(70);
  const channel:ConfidentialChannel={context,async exchange(wire){calls++;retained=wire.slice();
    const bytes=Buffer.from(wire),message=bytes.subarray(0,-64);
    assert(verify(null,message,createPublicKey(secret),bytes.subarray(-64)));
    const prefix=Buffer.byteLength('CINDER-UNIT-ONBOARD\0\x00\x01');
    assert(message.subarray(0,prefix).equals(Buffer.from('CINDER-UNIT-ONBOARD\0\x00\x01')));
    assert.deepEqual(message.subarray(prefix,prefix+32),Buffer.from(id(40)));assert.deepEqual(message.subarray(prefix+32,prefix+64),Buffer.from(id(70)));
    assert(message.subarray(prefix+64,prefix+64+Buffer.byteLength('CINDER-ONBOARD\0\x00\x01')).equals(Buffer.from('CINDER-ONBOARD\0\x00\x01')));
    return new Uint8Array(Buffer.concat([Buffer.from('CINDER-API-REPLY\0\x00\x02'),Buffer.from(id(wrong?50:40)),Buffer.from(reply([]))]));
  }};
  const sdk=new UnitOnboardingClient(channel,signer,{domain,policy:1,unit,contract});unit.fill(1);contract.fill(1);
  const r=await sdk.open(900n);assert.equal(r.kind,'discovery');assert(retained?.some(x=>x));
  wrong=true;await assert.rejects(sdk.open(900n),/unavailable/);assert.equal(calls,2);
});
test('onboarding signs the purpose/domain/wallet/session envelope and never aliases identities or signature hints',async()=>{
  const operation=Buffer.from(id(44)),signature=Buffer.alloc(64,45);let calls=0;
  const channel:ConfidentialChannel={context,async exchange(wire){calls++;
    const bytes=Buffer.from(wire),message=bytes.subarray(0,-64);assert(verify(null,message,createPublicKey(secret),bytes.subarray(-64)));
    let expected=Buffer.concat([Buffer.from('CINDER-ONBOARD\0\x00\x01'),Buffer.from(id(1)),Buffer.from(id(2)),number(1n,4),Buffer.from(publicKey),Buffer.from(id(8)),number(900n,8),Buffer.from([calls===1?0:1])]);
    if(calls===2)expected=Buffer.concat([expected,number(1n,8),operation,signature]);assert.deepEqual(message,expected);return reply(calls===1?[]:[id(44)]);
  }};
  const sdk=new OnboardingClient(channel,signer,{domain,policy:1});const created=await sdk.open(900n);assert(created.kind==='discovery');assert.deepEqual(created.account,new Uint8Array(account));
  const hint=await sdk.reportDeposit({epoch:1n,expiresAt:900n,operation,signature});assert(hint.kind==='discovery');assert.deepEqual(hint.pendingDeposits,[id(44)]);
  assert.deepEqual(operation,Buffer.from(id(44)));assert.deepEqual(signature,Buffer.alloc(64,45));assert(publicKey.some(x=>x));
});
test('discovery decoding rejects truncation, trailing data, duplicate hints and wrong owner replies',async()=>{
  const bytes=reply();for(let i=0;i<bytes.length;i++)assert.throws(()=>decodeResponse(bytes.slice(0,i)));
  assert.throws(()=>decodeResponse(new Uint8Array([...bytes,0])));assert.throws(()=>decodeResponse(reply([id(44),id(44)])));
  assert.throws(()=>decodeResponse(reply(Array.from({length:129},(_,i)=>id(i+1)))));
  const wrong=reply();wrong[Buffer.byteLength('CINDER-API-REPLY\0\x00\x01')+1]^=1;
  await assert.rejects(new OnboardingClient({context,exchange:async()=>wrong},signer,{domain,policy:1}).open(900n),/unavailable/);
  const parsed=decodeResponse(bytes);assert(parsed.kind==='discovery');bytes.fill(0);assert.deepEqual(parsed.pendingDeposits,[id(44)]);
});
test('wrong channel, malformed signatures and invalid deposit notices never get an automatic retry',async()=>{
  let sends=0,signs=0;const channel:ConfidentialChannel={context:()=>({...context(),deployment:id(9)}),exchange:async()=>{sends++;return reply();}};
  const counting={publicKey,signMessage:async(b:Uint8Array)=>{signs++;return signer.signMessage(b);}};
  await assert.rejects(new OnboardingClient(channel,counting,{domain,policy:1}).open(900n));assert.equal(signs,0);assert.equal(sends,0);
  const good={...channel,context};const sdk=new OnboardingClient(good,signer,{domain,policy:1});
  for(const signature of [new Uint8Array(63),new Uint8Array(64)])await assert.rejects(sdk.reportDeposit({epoch:1n,expiresAt:900n,operation:id(44),signature}));assert.equal(sends,0);
  await assert.rejects(new OnboardingClient(good,{publicKey,signMessage:async()=>new Uint8Array(63)},{domain,policy:1}).open(900n));assert.equal(sends,0);
  let uncertain=0;await assert.rejects(new OnboardingClient({context,exchange:async()=>{uncertain++;throw Error();}},signer,{domain,policy:1}).open(900n));assert.equal(uncertain,1);
});
