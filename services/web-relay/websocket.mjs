// Bounded RFC 6455 binary carrier ONLY. SHA-1 below is the public Upgrade
// accept digest, not application encryption or authentication.
import { createHash } from 'node:crypto';
const MAX_INPUT=1046, MAX_OUTPUT=1_050_790, MAX_BUFFER=MAX_OUTPUT+10;
const failed=()=>Error('WebSocket delivery unavailable');
function frame(opcode,body){
  const n=body.length,header=Buffer.alloc(n<126?2:n<65536?4:10);header[0]=0x80|opcode;
  if(n<126)header[1]=n;else if(n<65536){header[1]=126;header.writeUInt16BE(n,2);}else{header[1]=127;header.writeBigUInt64BE(BigInt(n),2);}
  return Buffer.concat([header,body]);
}
/** No extensions/subprotocol, text, private paths or unbounded assembly. */
export function upgrade(req,socket,head,onMessage,onClose){
  const key=req.headers['sec-websocket-key'];
  if(req.method!=='GET'||req.url!=='/v1/ws'||req.headers.upgrade?.toLowerCase()!=='websocket'
    || !req.headers.connection?.toLowerCase().split(',').map(x=>x.trim()).includes('upgrade')
    || req.headers['sec-websocket-version']!=='13'||typeof key!=='string'
    || !/^[A-Za-z0-9+/]{22}==$/.test(key)||Buffer.from(key,'base64').length!==16||Buffer.from(key,'base64').toString('base64')!==key
    || req.headers['sec-websocket-protocol']||req.headers['content-length']||req.headers['transfer-encoding'])throw failed();
  const accept=createHash('sha1').update(key+'258EAFA5-E914-47DA-95CA-C5AB0DC85B11').digest('base64');
  socket.write(`HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ${accept}\r\n\r\n`);
  let dead=false,ending=false,buffer=Buffer.alloc(0),parts=[],size=0,fragment=false,controls=0,partialTimer,closeTimer;
  const stop=()=>{if(dead)return;dead=true;clearTimeout(partialTimer);clearTimeout(closeTimer);parts=[];buffer=Buffer.alloc(0);socket.destroy();onClose();};
  socket.on('error',stop);socket.on('close',stop);
  const write=(opcode,body)=>{
    if(dead||ending||body.length>MAX_OUTPUT||socket.writableLength+body.length+10>MAX_BUFFER){stop();throw failed();}
    socket.write(frame(opcode,body),e=>{if(e)stop();});
  };
  const finish=()=>{
    if(dead||ending)return;
    // Flush complete encrypted deliveries before the orderly close frame.
    // Abort/overflow still destroys immediately; this never retries a record.
    write(8,Buffer.from([3,232]));ending=true;clearTimeout(partialTimer);
    socket.end();closeTimer=setTimeout(stop,1000);closeTimer.unref();
  };
  const feed=data=>{
    try{
      if(dead||ending)return;if(buffer.length+data.length>16_384)throw failed();
      buffer=Buffer.concat([buffer,data]);
      while(buffer.length>=2){
        const a=buffer[0],b=buffer[1],fin=!!(a&128),op=a&15;
        if(a&112||!(b&128)||![0,2,8,9,10].includes(op))throw failed();
        const short=b&127;let at=2,n=short;
        if(short===126){if(buffer.length<4)break;n=buffer.readUInt16BE(2);at=4;if(n<126)throw failed();}
        if(short===127)throw failed(); // Every accepted client message is <=1046.
        if(n>MAX_INPUT||op>=8&&(!fin||n>125))throw failed();
        if(buffer.length<at+4+n)break;
        const mask=buffer.subarray(at,at+4),payload=Buffer.from(buffer.subarray(at+4,at+4+n));
        for(let i=0;i<n;i++)payload[i]^=mask[i&3];buffer=buffer.subarray(at+4+n);
        if(op>=8){
          if(++controls>256)throw failed();
          if(op===8){if(n===1)throw failed();if(n>=2){const code=payload.readUInt16BE();if(code<1000||code>=5000||[1004,1005,1006,1015].includes(code)||code>1014&&code<3000)throw failed();new TextDecoder('utf-8',{fatal:true}).decode(payload.subarray(2));}
            finish();return;}
          if(op===9)write(10,payload);continue;
        }
        if(op===2&&fragment||op===0&&!fragment)throw failed();
        if(size+n>MAX_INPUT||parts.length>=64)throw failed();parts.push(payload);size+=n;fragment=!fin;
        if(fin){const body=Buffer.concat(parts,size);parts=[];size=0;fragment=false;onMessage(body);}
      }
      if(buffer.length||fragment){if(!partialTimer)partialTimer=setTimeout(stop,5000);}
      else{clearTimeout(partialTimer);partialTimer=undefined;}
    }catch{stop();}
  };
  socket.on('data',feed);if(head.length)feed(head);
  return {send:body=>write(2,body),close:finish};
}
