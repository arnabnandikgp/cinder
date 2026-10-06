// Public routing/lifetime adversarial checks. No private financial mock server.
import test from 'node:test';
import assert from 'node:assert/strict';
import net from 'node:net';
import { once } from 'node:events';
import { createWebRelay } from './server.mjs';
const challenge=Buffer.alloc(32,1);
function envelope(){const b=Buffer.alloc(137,2);challenge.copy(b);return b;}
async function fixture(reply){
  const peers=new Set();let calls=0;
  const target=net.createServer(socket=>{calls++;peers.add(socket);socket.on('error',()=>{});socket.on('close',()=>peers.delete(socket));socket.once('data',()=>{if(reply)socket.write(reply);});});
  target.listen(0,'127.0.0.1');await once(target,'listening');
  const relay=createWebRelay({target:{host:'127.0.0.1',port:target.address().port}});
  relay.server.listen(0,'127.0.0.1');await once(relay.server,'listening');
  return{relay,base:`http://127.0.0.1:${relay.server.address().port}`,calls:()=>calls,close:async()=>{await relay.close();for(const p of peers)p.destroy();await new Promise(r=>target.close(r));}};
}
function frame(body){const h=Buffer.alloc(4);h.writeUInt32BE(body.length);return Buffer.concat([h,body]);}
const post=(h,path,body=challenge,headers={})=>fetch(h.base+path,{method:'POST',headers:{'Content-Type':'application/octet-stream',...headers},body});
test('fixed loopback target cannot become an open proxy',()=>{
  for(const target of [{host:'example.com',port:443},{host:'127.0.0.1',port:0},{host:'127.0.0.1',port:65536}])assert.throws(()=>createWebRelay({target}));
});
test('paths/private headers/oversized bodies refuse before upstream connection',async()=>{
  const h=await fixture();try{
    for(const [path,body,headers] of [['/v1/exchange?account=private',challenge,{}],['/v1/attestation',challenge,{Authorization:'Bearer never-allowed'}],['/v1/attestation',challenge,{Cookie:'private=never'}],['/v1/attestation',Buffer.alloc(33),{}]]){
      const res=await post(h,path,body,headers);assert.equal(res.status,503);await res.body.cancel();
    }
    assert.equal(h.calls(),0);assert.equal(h.relay.activeSessions(),0);
  }finally{await h.close();}
});
test('successful flush retains routing; explicit disposal releases it',async()=>{
  const b=envelope(),h=await fixture(frame(b));try{
    const res=await post(h,'/v1/attestation');assert.equal(res.status,200);assert.deepEqual(Buffer.from(await res.arrayBuffer()),b);
    assert.equal(h.relay.activeSessions(),1);
    const disposed=await post(h,'/v1/session',Buffer.concat([b.subarray(64,96),Buffer.from([0])]));assert.equal(disposed.status,204);
    assert.equal(h.relay.activeSessions(),0);
  }finally{await h.close();}
});
test('oversized/trailing/empty upstream frames never become attestation',async()=>{
  for(const wire of [Buffer.from([255,255,255,255]),Buffer.concat([frame(envelope()),Buffer.from([1])]),Buffer.alloc(4)]){
    const h=await fixture(wire);try{const r=await post(h,'/v1/attestation');assert.equal(r.status,503);await r.body.cancel();assert.equal(h.relay.activeSessions(),0);}finally{await h.close();}
  }
});
test('colliding public handle cannot erase the original routing entry',async()=>{
  const b=envelope(),h=await fixture(frame(b));try{
    let r=await post(h,'/v1/attestation');assert.equal(r.status,200);await r.body.cancel();
    r=await post(h,'/v1/attestation');assert.equal(r.status,503);await r.body.cancel();
    assert.equal(h.relay.activeSessions(),1);
    r=await post(h,'/v1/session',Buffer.concat([b.subarray(64,96),Buffer.from([0])]));assert.equal(r.status,204);
    assert.equal(h.relay.activeSessions(),0);
  }finally{await h.close();}
});
test('one absolute five-second delivery budget fences a silent upstream',{timeout:10000},async()=>{
  const h=await fixture();try{
    const start=performance.now();await assert.rejects(post(h,'/v1/attestation'));
    assert(performance.now()-start<6500);assert.equal(h.relay.activeSessions(),0);
  }finally{await h.close();}
});

async function sessionFixture(delay) {
  const peers=new Set();let calls=0;
  const target=net.createServer(socket=>{
    peers.add(socket);let pending=Buffer.alloc(0),phase=0;
    socket.on('error',()=>{});socket.on('close',()=>peers.delete(socket));
    socket.on('data',part=>{pending=Buffer.concat([pending,part]);while(pending.length>=4){const n=pending.readUInt32BE();if(pending.length<n+4)return;pending=pending.subarray(n+4);calls++;
      if(phase++===0)socket.write(frame(envelope()));
      else if(phase<=3)socket.write(frame(Buffer.from([1])));
      else if(delay!==undefined){const timer=setTimeout(()=>{if(!socket.destroyed)socket.write(frame(Buffer.from([42])));},delay);socket.once('close',()=>clearTimeout(timer));}
    }});
  });
  target.listen(0,'127.0.0.1');await once(target,'listening');
  const relay=createWebRelay({target:{host:'127.0.0.1',port:target.address().port}});
  relay.server.listen(0,'127.0.0.1');await once(relay.server,'listening');
  const h={relay,base:`http://127.0.0.1:${relay.server.address().port}`,calls:()=>calls,close:async()=>{await relay.close();for(const s of peers)s.destroy();await new Promise(r=>target.close(r));}};
  try{const quoted=await post(h,'/v1/attestation');await quoted.body.cancel();const body=Buffer.concat([envelope().subarray(64,96),Buffer.from([1])]);for(let i=0;i<2;i++){const r=await post(h,'/v1/session',body);assert.equal(r.status,200);await r.body.cancel();}return {...h,body};}catch(e){await h.close();throw e;}
}
test('application reply can exceed five seconds without extending handshake or retrying',{timeout:12000},async()=>{
  const h=await sessionFixture(6200);try{const start=performance.now(),r=await post(h,'/v1/exchange',h.body);assert.equal(r.status,200);assert.deepEqual(Buffer.from(await r.arrayBuffer()),Buffer.from([42]));assert(performance.now()-start>=6000);assert.equal(h.calls(),4);}finally{await h.close();}
});
test('silent application reply is bounded and is never resent',{timeout:20000},async()=>{
  const h=await sessionFixture();try{const start=performance.now();await assert.rejects(post(h,'/v1/exchange',h.body));const elapsed=performance.now()-start;assert(elapsed>=14000&&elapsed<17000);assert.equal(h.calls(),4);assert.equal(h.relay.activeSessions(),0);}finally{await h.close();}
});
