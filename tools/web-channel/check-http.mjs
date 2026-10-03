#!/usr/bin/env node
// Actual browser AND Node SDK -> bounded public HTTP relay -> native service
// process -> the existing durable journal. Synthetic funds, no live service.
import { spawn,execFileSync } from 'node:child_process';
import { createServer } from 'node:http';
import { generateKeyPairSync,sign } from 'node:crypto';
import { readFile,mkdtemp,rm } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';
import { fixture } from './http-fixture.mjs';
import { runHttp } from '../../clients/private/tests/web-suite.ts';
import { stopChild } from './stop-child.mjs';

const dir=fileURLToPath(new URL('.',import.meta.url));
const chrome=process.env.CINDER_TEST_CHROME??(process.platform==='darwin'?'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome':'/usr/bin/google-chrome');
if(!existsSync(chrome))throw Error('Actual Chrome is required');
await build({absWorkingDir:dir,entryPoints:['../../clients/private/tests/web-suite.ts'],outfile:'pkg/http-fixture.js',bundle:true,format:'esm',platform:'browser',target:'es2022',legalComments:'eof'});
const core=await import('./pkg/channel.js');
await core.default({module_or_path:await readFile(join(dir,'pkg/channel_bg.wasm'))});
const keys=generateKeyPairSync('ed25519');
const publicKey=new Uint8Array(keys.publicKey.export({format:'der',type:'spki'}).subarray(-32));
const node=await fixture(publicKey);
let nodeChecks;
try{
  nodeChecks=await runHttp(core,{publicKey,signMessage:async b=>sign(null,b,keys.privateKey)},node);
  await node.assertPrivate();
}finally{await node.close();}
const profile=await mkdtemp(join(tmpdir(),'cinder-http-chrome-'));
let browserHarness,browser,resultResolve,timer;
const result=new Promise(resolve=>{resultResolve=resolve;});
const files=new Map([['/',['web/http.html','text/html']],['/channel.js',['pkg/channel.js','text/javascript']],['/channel_bg.wasm',['pkg/channel_bg.wasm','application/wasm']],['/http-fixture.js',['pkg/http-fixture.js','text/javascript']]]);
async function body(req,max){const parts=[];let n=0;for await(const b of req){n+=b.length;if(n>max)throw Error('Fixture body bound');parts.push(b);}return Buffer.concat(parts,n);}
const server=createServer(async(req,res)=>{
  try{
    if(req.method==='GET'&&files.has(req.url)){const[p,type]=files.get(req.url);res.setHeader('Content-Type',type);res.end(await readFile(join(dir,p)));return;}
    if(req.method==='POST'&&req.url==='/test/setup'){
      if(browserHarness)throw Error('Fixture already allocated');
      const key=await body(req,32);if(key.length!==32)throw Error('Fixture public key');
      browserHarness=await fixture(key,`http://127.0.0.1:${server.address().port}`);
      res.setHeader('Content-Type','application/json');res.end(JSON.stringify(browserHarness.current()));return;
    }
    if(req.method==='POST'&&req.url==='/test/control'&&browserHarness){
      await browserHarness.control((await body(req,32)).toString());
      res.setHeader('Content-Type','application/json');res.end(JSON.stringify(browserHarness.current()));return;
    }
    if(req.method==='POST'&&req.url==='/test/result'){
      resultResolve(JSON.parse((await body(req,4096)).toString()));res.end();return;
    }
    res.writeHead(404).end();
  }catch{res.writeHead(500).end('Offline fixture unavailable');}
});
try{
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  browser=spawn(chrome,['--headless','--no-first-run','--disable-background-networking','--disable-component-update','--disable-sync','--disable-default-apps','--no-proxy-server',`--user-data-dir=${profile}`,`http://127.0.0.1:${server.address().port}`],{stdio:'ignore',detached:true});
  browser.on('error',()=>resultResolve({ok:false,error:'Chrome launch failed'}));
  browser.on('exit',()=>resultResolve({ok:false,error:'Chrome exited before result'}));
  const r=await Promise.race([result,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('HTTP browser test timeout')),60000);})]);
  if(!r.ok||r.checks?.length!==nodeChecks.length)throw Error(r.error??'Incomplete browser HTTP acceptance');
  await browserHarness.assertPrivate();
  console.log(`Node ${process.version}: ${nodeChecks.length} HTTP/private SDK process groups passed`);
  console.log(`${execFileSync(chrome,['--version'],{encoding:'utf8'}).trim()}: ${r.checks.length} HTTP/private SDK process groups passed`);
  console.log('All eight signed commands, durable lost-reply/crash reconciliation and parent plaintext observation: synthetic offline fixtures only.');
}finally{
  clearTimeout(timer);await stopChild(browser,{group:true});await browserHarness?.close();
  server.closeAllConnections();if(server.listening)await new Promise(resolve=>server.close(resolve));
  await rm(profile,{recursive:true,force:true,maxRetries:4,retryDelay:100});
}
