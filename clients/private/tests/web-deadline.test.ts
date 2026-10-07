// HTTP SDK framing only: synthetic endpoint/attestation, no cryptographic claim.
import test from 'node:test';
import assert from 'node:assert/strict';
import {WebChannel,type WebCore} from '../src/web-channel-core.ts';
const id=(n:number)=>new Uint8Array(32).fill(n);
const policy={network:id(1),deployment:id(2),manifest:id(3),pcrs:[new Uint8Array(48).fill(4),new Uint8Array(48).fill(5),new Uint8Array(48).fill(6)] as const};
const core:WebCore={BrowserEndpoint:class {
  start(){return id(7);} advance(){return new Uint8Array();} ready(){return true;}
  binding(){return id(8);} request(){return id(9);} socket_request(){return id(9);}
  response(_sequence:number,bytes:Uint8Array){return bytes;} free(){}
},web_binding(){return id(10);}};
const binary=(body:BodyInit)=>new Response(body,{headers:{'Content-Type':'application/octet-stream'}});
for(const partial of [false,true])test(`SDK ${partial?'partial body':'headers-only body'} assembly keeps its own five-second deadline`,async t=>{
  t.mock.timers.enable({apis:['setTimeout']});let exchanges=0;
  t.mock.method(globalThis,'fetch',async(input:RequestInfo|URL,options?:RequestInit)=>{
    if(String(input).endsWith('/v1/attestation')){
      const b=new Uint8Array(137).fill(1);b.set(new Uint8Array(options!.body as ArrayBuffer));
      new DataView(b.buffer).setBigUint64(128,BigInt(Date.now()+60000));return binary(b);
    }
    if(String(input).endsWith('/v1/session'))return binary(id(1));
    exchanges++;
    return binary(new ReadableStream<Uint8Array>({start(controller){
      if(partial)controller.enqueue(Uint8Array.of(1));
      const timer=setTimeout(()=>{controller.enqueue(Uint8Array.of(2));controller.close();},6200);
      this.cancel=()=>clearTimeout(timer);
    }}));
  });
  const ch=await WebChannel.connect({baseUrl:'http://127.0.0.1',policy,core},async(_q,_p,c)=>({key:c.key,prologue:id(11)}));
  const refused=assert.rejects(ch.exchange(Uint8Array.of(1)),/unavailable/);
  await new Promise<void>(resolve=>setImmediate(resolve));
  t.mock.timers.tick(6200);await refused;
  assert.equal(exchanges,1);assert.throws(()=>ch.context(),/unavailable/);
});
