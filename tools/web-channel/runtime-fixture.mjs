// C4: actual Runtime/Journal, explicit synthetic local dependencies. No cloud,
// venue/RPC/wallet access. Controls go to private stdin, never the public relay.
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { randomBytes,createPrivateKey,createPublicKey } from 'node:crypto';
import { mkdtemp,mkdir,readFile,readdir,rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join,resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import net from 'node:net';
import { once } from 'node:events';
import { createWebRelay } from '../../services/web-relay/server.mjs';
import { stopChild } from './stop-child.mjs';

function base58(bytes) {
  const alphabet='123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
  let n=BigInt('0x'+Buffer.from(bytes).toString('hex')),out='';
  while(n){out=alphabet[Number(n%58n)]+out;n/=58n;}
  for(const b of bytes){if(b!==0)break;out='1'+out;}
  return out;
}
export async function runtimeFixture(publicKey,origin,options,binaryOverride) {
  const root=await mkdtemp(join(tmpdir(),'cinder-runtime-fixture-'));
  const store=join(root,'ciphertext');await mkdir(store,{mode:0o700});
  const storageKey=randomBytes(32),brokerSeed=randomBytes(32),captured=[],sockets=new Set();
  const privateKey=createPrivateKey({key:Buffer.concat([Buffer.from('302e020100300506032b657004220420','hex'),brokerSeed]),format:'der',type:'pkcs8'});
  const account=base58(createPublicKey(privateKey).export({format:'der',type:'spki'}).subarray(-32));
  const config=Buffer.from(JSON.stringify({...options,broker_seed:[...brokerSeed],account}));
  const binary=binaryOverride??resolve(fileURLToPath(new URL('../..',import.meta.url)),process.env.CARGO_TARGET_DIR??'target','debug/cinder-service-fixture');
  let child,proxy,relay,info,logs='',stderr='',lines,closed,controlTail=Promise.resolve(),waiting,finished,expectNotification=false,notified;
  async function start() {
    stderr='';finished=undefined;
    const args=[...(process.platform==='darwin'?['-l']:['-v']),binary,'127.0.0.1:0',store,Buffer.from(publicKey).toString('hex'),'--runtime-web'];
    child=spawn('/usr/bin/time',args,{stdio:['pipe','pipe','pipe'],detached:true});
    child.stderr.on('data',b=>{stderr+=b;logs+=b;});
    closed=new Promise((resolve,reject)=>{child.once('error',reject);child.once('close',code=>resolve(code));});
    lines=createInterface({input:child.stdout});
    const header=new Promise((resolve,reject)=>{
      const timer=setTimeout(()=>reject(Error('Runtime fixture startup timeout')),30000);
      lines.once('line',line=>{clearTimeout(timer);resolve(line);});
      child.once('exit',()=>{clearTimeout(timer);reject(Error('Runtime fixture startup failed'));});
      child.once('error',e=>{clearTimeout(timer);reject(e);});
    });
    child.stdin.write(storageKey);
    const size=Buffer.alloc(4);size.writeUInt32BE(config.length);child.stdin.write(size);child.stdin.write(config);
    const [address,ca]=(await header).split(' '),port=Number(address.split(':')[1]);
    lines.on('line',line=>{
      logs+=line+'\n';
      if(!waiting)throw Error('Unexpected runtime harness response');
      const next=waiting;waiting=undefined;next(line);
    });
    proxy=net.createServer(client=>{
      const upstream=net.connect(port,'127.0.0.1');sockets.add(client);sockets.add(upstream);
      let framed=Buffer.alloc(0);
      upstream.on('data',b=>{
        framed=Buffer.concat([framed,b]);if(framed.length>2_101_588){client.destroy();upstream.destroy();return;}
        while(framed.length>=4){const n=framed.readUInt32BE(0);if(n>1_050_790){client.destroy();upstream.destroy();return;}
          if(framed.length<n+4)break;
          if(expectNotification&&n>4&&framed.readUInt32BE(4)===1){expectNotification=false;notified?.();}
          framed=framed.subarray(n+4);
        }
      });
      for(const [from,to] of [[client,upstream],[upstream,client]]){
        from.on('data',b=>captured.push(Buffer.from(b)));
        from.on('error',()=>{client.destroy();upstream.destroy();});
        from.on('close',()=>{sockets.delete(from);client.destroy();upstream.destroy();});
        from.pipe(to);
      }
    });
    proxy.listen(0,'127.0.0.1');await once(proxy,'listening');
    relay=createWebRelay({target:{host:'127.0.0.1',port:proxy.address().port},origin});
    relay.server.listen(0,'127.0.0.1');await once(relay.server,'listening');
    info={baseUrl:`http://127.0.0.1:${relay.server.address().port}`,root:[...Buffer.from(ca,'hex')]};
  }
  function privateControl(command) {
    const work=controlTail.then(async()=>{
      if(!child||child.exitCode!==null)throw Error('Runtime fixture unavailable');
      let timer;
      try{
        const result=await new Promise((resolve,reject)=>{
          timer=setTimeout(()=>{waiting=undefined;reject(Error('Runtime control timeout'));},15000);
          waiting=line=>{try{resolve(JSON.parse(line));}catch{reject(Error('Malformed runtime control'));}};
          child.stdin.write(JSON.stringify(command)+'\n');
        });
        if(result.error)throw Error(result.error);return result;
      }finally{clearTimeout(timer);}
    });
    controlTail=work.catch(()=>{});return work;
  }
  async function control(command) {
    if(command.op==='expect-notification')expectNotification=true;
    else if(command.op==='await-notification'){
      if(expectNotification)await new Promise((resolve,reject)=>{
        const timer=setTimeout(()=>{notified=undefined;reject(Error('Runtime notification missing'));},5000);
        notified=()=>{clearTimeout(timer);notified=undefined;resolve();};
      });
    }else return privateControl(command);
    return privateControl({op:'metrics'});
  }
  async function stopNetwork() {
    await relay?.close();for(const socket of sockets)socket.destroy();
    if(proxy?.listening)await new Promise(r=>proxy.close(r));
    if(child){
      child.stdin.end();let timer;
      try{await Promise.race([closed,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Runtime fixture did not join')),12000);})]);}
      finally{clearTimeout(timer);await stopChild(child,{group:true});lines.close();}
      const match=process.platform==='darwin'?/(\d+)\s+maximum resident set size/.exec(stderr):/Maximum resident set size \(kbytes\):\s*(\d+)/.exec(stderr);
      if(!match)throw Error('Missing runtime process peak-memory receipt');
      finished={peakRssBytes:Number(match[1])*(process.platform==='darwin'?1:1024)};
      child=undefined;
    }
    return finished;
  }
  async function files(path){const out=[];for(const e of await readdir(path,{withFileTypes:true})){const p=join(path,e.name);out.push(...(e.isDirectory()?await files(p):[await readFile(p)]));}return out;}
  const h={current:()=>info,control,restart:async()=>{await stopNetwork();await start();},
    assertPrivate:async()=>{
      const packets=Buffer.concat(captured),disk=await files(store);
      if(packets.length<1000)throw Error('Runtime client did not traverse the real carrier');
      for(const marker of [Buffer.from('CINDER-API\0'),Buffer.from('CINDER-API-REPLY\0'),Buffer.from('CINDER-API-RECORD-1\0'),Buffer.from(publicKey),storageKey,brokerSeed]){
        if(packets.includes(marker)||Buffer.from(logs).includes(marker)||disk.some(b=>b.includes(marker)))throw Error('Runtime parent plaintext leakage');
      }
    },finish:stopNetwork,close:async()=>{
      try{await stopNetwork();}finally{storageKey.fill(0);brokerSeed.fill(0);config.fill(0);await rm(root,{recursive:true,force:true});}
    }};
  try{await start();return h;}catch(error){await h.close();throw error;}
}
