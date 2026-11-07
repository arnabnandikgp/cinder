// Carrier-only RFC 6455 bounds. No financial/Noise implementation in this peer.
import test from 'node:test';
import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { upgrade } from './websocket.mjs';
class Socket extends EventEmitter {
  writableLength=0; writes=[]; destroyed=false;
  write(bytes,callback){this.writes.push(Buffer.from(bytes));callback?.();}
  end(){queueMicrotask(()=>this.destroy());}
  destroy(){if(!this.destroyed){this.destroyed=true;this.emit('close');}}
}
const headers={'upgrade':'websocket','connection':'keep-alive, Upgrade','sec-websocket-version':'13','sec-websocket-key':'AAAAAAAAAAAAAAAAAAAAAA=='};
function masked(body,opcode=2,fin=true){
  const head=Buffer.alloc(body.length<126?6:8);head[0]=(fin?128:0)|opcode;
  if(body.length<126)head[1]=128|body.length;else{head[1]=254;head.writeUInt16BE(body.length,2);}
  const mask=Buffer.from([1,2,3,4]);mask.copy(head,head.length-4);
  return Buffer.concat([head,Buffer.from(body).map((b,i)=>b^mask[i&3])]);
}
function fixture(extra={}){
  const socket=new Socket(),messages=[];let closed=0;
  const web=upgrade({method:'GET',url:'/v1/ws',headers:{...headers,...extra}},socket,Buffer.alloc(0),b=>messages.push(b),()=>closed++);
  return{socket,messages,web,closed:()=>closed};
}
test('strict public upgrade and canonical key; no query, text subprotocol or body',()=>{
  for(const request of [{method:'POST'}, {url:'/v1/ws?account=private'},
    {headers:{...headers,'sec-websocket-version':'12'}}, {headers:{...headers,'sec-websocket-key':'AAAAAAAAAAAAAAAAAAAAAB=='}},
    {headers:{...headers,'sec-websocket-protocol':'private-method'}}, {headers:{...headers,'content-length':'0'}}]){
    assert.throws(()=>upgrade({method:'GET',url:'/v1/ws',headers,...request},new Socket(),Buffer.alloc(0),()=>{},()=>{}));
  }
});
test('masked binary fragmentation and interleaved ping preserve only opaque bytes',()=>{
  const h=fixture();try{
    const wire=Buffer.concat([masked(Buffer.from('cipher'),2,false),masked(Buffer.from('ping'),9),masked(Buffer.from('text'),0)]);
    for(const byte of wire)h.socket.emit('data',Buffer.from([byte]));
    assert.deepEqual(h.messages,[Buffer.from('ciphertext')]);assert.equal(h.closed(),0);
    assert.equal(h.socket.writes[1][0],138); // public Pong, not app response
  }finally{h.socket.destroy();}
});
test('text/RSV/unmasked/large/noncanonical/invalid control records fail closed',()=>{
  const cases=[masked(Buffer.alloc(1),1),Buffer.from([0xc2,0x80,0,0,0,0]),Buffer.from([0x82,0]),
    masked(Buffer.alloc(1047)),Buffer.from([0x82,0xfe,0,1,0,0,0,0,0]),Buffer.from([0x82,0xff]),
    masked(Buffer.alloc(126),9),masked(Buffer.alloc(1),8),masked(Buffer.from([3,237]),8),masked(Buffer.alloc(0),0)];
  for(const wire of cases){const h=fixture();h.socket.emit('data',wire);assert.equal(h.closed(),1);assert.equal(h.messages.length,0);}
});
test('fragment count, aggregate assembly and write queue cannot grow without bound',()=>{
  for(const wire of [Buffer.concat(Array.from({length:65},(_,i)=>masked(Buffer.alloc(1),i?0:2,false))),
    Buffer.concat([masked(Buffer.alloc(600),2,false),masked(Buffer.alloc(500),0)])]){
    const h=fixture();h.socket.emit('data',wire);assert.equal(h.closed(),1);
  }
  const h=fixture();h.socket.writableLength=1_050_800;assert.throws(()=>h.web.send(Buffer.alloc(1)));assert.equal(h.closed(),1);
});
test('maximum output uses 64-bit RFC length and orderly close flushes it first',async()=>{
  const h=fixture();h.web.send(Buffer.alloc(1_050_790,7));h.web.close();
  assert.equal(h.socket.writes[1][1],127);assert.equal(h.socket.writes[1].readBigUInt64BE(2),1_050_790n);
  assert.equal(h.socket.writes[2][0],136);await Promise.resolve();assert.equal(h.closed(),1);
});
test('one absolute assembly deadline bounds a partially supplied masked frame',{timeout:8000},async()=>{
  const h=fixture();h.socket.emit('data',Buffer.from([0x82]));
  await new Promise(resolve=>setTimeout(resolve,5100));assert.equal(h.closed(),1);assert.equal(h.messages.length,0);
});
