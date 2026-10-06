// Offline process harness ONLY. Disposable wallet binding, storage key, signed
// synthetic quote root and seed balance. No AWS/RPC/venue access or wallet file.
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { randomBytes } from 'node:crypto';
import { mkdtemp, mkdir, readFile, readdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import net from 'node:net';
import { createWebRelay } from '../../services/web-relay/server.mjs';
import { stopChild } from './stop-child.mjs';

export async function fixture(publicKey, origin) {
  const root = await mkdtemp(join(tmpdir(),'cinder-http-fixture-'));
  const store = join(root,'parent-storage'); await mkdir(store,{mode:0o700});
  const storageKey = randomBytes(32), captured = [], sockets = new Set();
  const binary = resolve(fileURLToPath(new URL('../..',import.meta.url)),process.env.CARGO_TARGET_DIR ?? 'target','debug/cinder-service-fixture');
  let child, proxy, relay, info, requestCorrupt=false, replyCorrupt=false, dropNotification=false, dropped,
    expectNotification=false,notified,logs='',delayReply=false;
  async function start() {
    let out='', error='';
    child=spawn(binary,['127.0.0.1:0',store,Buffer.from(publicKey).toString('hex'),'--web'],{stdio:['pipe','pipe','pipe']});
    child.stderr.on('data',b=>{error+=b;logs+=b;});
    child.stdout.on('data',b=>{out+=b;logs+=b;});
    child.stdin.write(storageKey);
    const line=await new Promise((resolve,reject)=>{
      const timer=setTimeout(()=>{child.kill('SIGKILL');reject(Error('HTTP fixture startup timeout'));},5000);
      child.stdout.on('data',()=>{if(out.includes('\n')){clearTimeout(timer);resolve(out.split('\n')[0]);}});
      child.once('exit',()=>{clearTimeout(timer);reject(Error('HTTP fixture startup failed: '+error));});
      child.once('error',e=>{clearTimeout(timer);reject(e);});
    });
    const [address,ca]=line.split(' '); const port=Number(address.split(':')[1]);
    proxy=net.createServer(client=>{
      const upstream=net.connect(port,'127.0.0.1'); sockets.add(client);sockets.add(upstream);
      let responseBuffer=Buffer.alloc(0);
      client.on('data',b=>{captured.push(Buffer.from(b));if(requestCorrupt){requestCorrupt=false;b[b.length-1]^=1;}});
      upstream.on('data',b=>{captured.push(Buffer.from(b));
        // Rust's four-byte header and payload need not arrive in one TCP chunk.
        // Fixture controls act on complete PUBLIC frames, not packet boundaries.
        responseBuffer=Buffer.concat([responseBuffer,b]);
        if(responseBuffer.length>2_101_588){client.destroy();upstream.destroy();return;}
        while(responseBuffer.length>=4){
          const n=responseBuffer.readUInt32BE(0);if(n>1_050_790){client.destroy();upstream.destroy();return;}
          if(responseBuffer.length<n+4)break;
          const framed=Buffer.from(responseBuffer.subarray(0,n+4));responseBuffer=responseBuffer.subarray(n+4);
          if(replyCorrupt){replyCorrupt=false;framed[framed.length-1]^=1;}
        // Public stream correlation only. Require one complete fixture frame;
        // never inspect/decrypt a customer payload or drop a partial record.
          if(dropNotification&&framed.length>8&&framed.readUInt32BE(4)===1){dropNotification=false;dropped?.();continue;}
          if(delayReply){delayReply=false;const timer=setTimeout(()=>{if(!client.destroyed)client.write(framed);},6200);client.once('close',()=>clearTimeout(timer));}
          else client.write(framed);
          if(expectNotification&&framed.length>8&&framed.readUInt32BE(4)===1){expectNotification=false;notified?.();}
        }
      });
      for(const s of [client,upstream]){s.on('error',()=>{client.destroy();upstream.destroy();});s.on('close',()=>{sockets.delete(s);client.destroy();upstream.destroy();});}
      client.pipe(upstream);
    });
    proxy.listen(0,'127.0.0.1');await once(proxy,'listening');
    relay=createWebRelay({target:{host:'127.0.0.1',port:proxy.address().port},origin});
    relay.server.listen(0,'127.0.0.1');await once(relay.server,'listening');
    info={baseUrl:`http://127.0.0.1:${relay.server.address().port}`,root:[...Buffer.from(ca,'hex')]};
  }
  async function stopNetwork(){
    await relay?.close(); for(const socket of sockets)socket.destroy();
    if(proxy?.listening)await new Promise(r=>proxy.close(r));
    await stopChild(child); child=undefined;
  }
  async function files(path){const output=[];for(const entry of await readdir(path,{withFileTypes:true})){const next=join(path,entry.name);if(entry.isDirectory())output.push(...await files(next));else output.push(await readFile(next));}return output;}
  const h={current:()=>info,control:async action=>{
    if(action==='restart'){await stopNetwork();await start();}
    else if(action==='corrupt-request')requestCorrupt=true;
    else if(action==='corrupt-reply')replyCorrupt=true;
    else if(action==='delay-reply')delayReply=true;
    else if(action==='drop-notification'){dropNotification=true;}
    else if(action==='await-drop'){if(dropNotification)await new Promise((resolve,reject)=>{
      const timer=setTimeout(()=>{dropped=undefined;reject(Error('Missing fixture drop'));},5000);
      dropped=()=>{clearTimeout(timer);dropped=undefined;resolve();};});}
    else if(action==='expect-notification')expectNotification=true;
    else if(action==='await-notification'){if(expectNotification)await new Promise((resolve,reject)=>{
      const timer=setTimeout(()=>{notified=undefined;reject(Error('Missing fixture notification'));},5000);
      notified=()=>{clearTimeout(timer);notified=undefined;resolve();};});}
    else if(action==='witness-loss')await rm(join(store,'accepted'));
    else throw Error('Unknown fixture control');
  }, relay:()=>relay, assertPrivate:async()=>{
    const packets=Buffer.concat(captured);
    if(packets.length<1000)throw Error('HTTP fixture did not observe real transport');
    const disk=await files(store);
    for(const marker of [Buffer.from('CINDER-API\0'),Buffer.from('CINDER-API-REPLY\0'),Buffer.from('CINDER-API-RECORD-1\0'),Buffer.from('synthetic-source-only'),Buffer.from(publicKey),storageKey]){
      if(packets.includes(marker)||Buffer.from(logs).includes(marker)||disk.some(b=>b.includes(marker)))throw Error('Parent plaintext leakage');
    }
  },close:async()=>{await stopNetwork();storageKey.fill(0);await rm(root,{recursive:true,force:true});}};
  try{await start();return h;}catch(error){await h.close();throw error;}
}
