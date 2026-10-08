#!/usr/bin/env node
// C4 fixed actual-client qualification. Synthetic dependencies, no live rails.
import { spawn,execFileSync } from 'node:child_process';
import { createServer } from 'node:http';
import { generateKeyPairSync,sign,createHash } from 'node:crypto';
import { readFile,mkdtemp,rm,copyFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { join,resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';
import { runtimeFixture } from './runtime-fixture.mjs';
import { runRuntime,runPackedGrowth } from '../../clients/private/tests/runtime-suite.ts';
import { stopChild } from './stop-child.mjs';

let frozenRoot;
async function main() {
const dir=fileURLToPath(new URL('.',import.meta.url)),args=process.argv.slice(2);
if(new Set(args).size!==args.length||args.some(a=>!['--node-only','--packed-only'].includes(a)))throw Error('Usage: check-runtime.mjs [--node-only] [--packed-only]');
const nodeOnly=args.includes('--node-only'),packed=args.includes('--packed-only');
const suite=packed?runPackedGrowth:runRuntime,groups=packed?2:9,cells=packed?2:24;
if(process.versions.node!==(await readFile(new URL('../../.node-version',import.meta.url),'utf8')).trim())throw Error('Use pinned Node');
const chrome=process.env.CINDER_TEST_CHROME??(process.platform==='darwin'?'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome':'/usr/bin/google-chrome');
if(!nodeOnly&&!existsSync(chrome))throw Error('Actual Chrome is required');
// Freeze one exact executable for ALL cells, even if an independent Cargo check
// replaces its target output while this longer process suite is running.
frozenRoot=await mkdtemp(join(tmpdir(),'cinder-runtime-artifact-'));
const frozenBinary=join(frozenRoot,'cinder-service-fixture');
await copyFile(resolve(dir,'../..',process.env.CARGO_TARGET_DIR??'target','debug/cinder-service-fixture'),frozenBinary);
console.log(JSON.stringify({kind:'qualification-artifact',sha256:createHash('sha256').update(await readFile(frozenBinary)).digest('hex')}));
await build({absWorkingDir:dir,entryPoints:['../../clients/private/tests/runtime-suite.ts'],outfile:'pkg/runtime-fixture.js',bundle:true,format:'esm',platform:'browser',target:'es2022',legalComments:'eof'});
const core=await import('./pkg/channel.js');await core.default({module_or_path:await readFile(join(dir,'pkg/channel_bg.wasm'))});
const keys=generateKeyPairSync('ed25519'),publicKey=new Uint8Array(keys.publicKey.export({format:'der',type:'spki'}).subarray(-32));
let node,current,nodeOptions;
const receipts=[];
const h={setup:async options=>{
  if(node){await node.assertPrivate();await node.close();}
  node=await runtimeFixture(publicKey,undefined,options,frozenBinary);current=node.current();nodeOptions=options;
},current:()=>current,control:command=>node.control(command),restart:async()=>{await node.restart();current=node.current();},
finish:async()=>{await node.assertPrivate();const memory=await node.finish();console.log(JSON.stringify({client:'Node',kind:'process-memory',...nodeOptions,...memory}));return memory;},
record:receipt=>{receipts.push(receipt);console.log(JSON.stringify({client:'Node',...receipt}));}};
let nodeChecks;
try{nodeChecks=await suite(core,{publicKey,signMessage:async bytes=>sign(null,bytes,keys.privateKey)},h);}
finally{await node?.close();}
if(receipts.length!==cells||nodeChecks.length!==groups)throw Error('Incomplete Node runtime matrix');
console.log(`Node ${process.version}: ${nodeChecks.length} joined Runtime qualification groups passed`);
if(nodeOnly)return;

const profile=await mkdtemp(join(tmpdir(),'cinder-runtime-chrome-'));
let browserHarness,browser,currentOptions,resultResolve,timer;
const result=new Promise(resolve=>{resultResolve=resolve;});
const html=`<!doctype html><meta charset="utf-8"><title>Local real Runtime qualification</title><pre id="result">Running C4 synthetic-dependency matrix</pre><script type="module">
import init,* as core from '/channel.js';import {${packed?'runPackedGrowth':'runRuntime'} as runSuite} from '/runtime-fixture.js';let result;
try{await init();const keys=await crypto.subtle.generateKey('Ed25519',false,['sign','verify']);const publicKey=new Uint8Array(await crypto.subtle.exportKey('raw',keys.publicKey));
let current;const receipts=[];async function post(path,input){const r=await fetch(path,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(input)});if(!r.ok)throw Error('Browser runtime harness '+path);return r.json();}
const h={setup:async options=>{current=await post('/test/setup',{publicKey:[...publicKey],options});},current:()=>current,
control:c=>post('/test/control',c),restart:async()=>{current=await post('/test/restart',{});},finish:()=>post('/test/finish',{}),record:r=>receipts.push(r)};
const signer={publicKey,signMessage:async b=>new Uint8Array(await crypto.subtle.sign('Ed25519',keys.privateKey,b))};
result={ok:true,checks:await runSuite(core,signer,h),receipts};}catch(error){result={ok:false,error:String(error).slice(0,512)};}
document.querySelector('#result').textContent=JSON.stringify(result);await fetch('/test/result',{method:'POST',body:JSON.stringify(result)});
</script>`;
const files=new Map([['/channel.js',['pkg/channel.js','text/javascript']],['/channel_bg.wasm',['pkg/channel_bg.wasm','application/wasm']],['/runtime-fixture.js',['pkg/runtime-fixture.js','text/javascript']]]);
async function body(req,max=32768){const parts=[];let n=0;for await(const b of req){n+=b.length;if(n>max)throw Error('Runtime fixture body bound');parts.push(b);}return JSON.parse(Buffer.concat(parts,n).toString());}
const server=createServer(async(req,res)=>{
  try{
    if(req.method==='GET'&&req.url==='/'){res.setHeader('Content-Type','text/html');res.end(html);return;}
    if(req.method==='GET'&&files.has(req.url)){const[p,type]=files.get(req.url);res.setHeader('Content-Type',type);res.end(await readFile(join(dir,p)));return;}
    if(req.method!=='POST'){res.writeHead(404).end();return;}
    const input=await body(req);let output;
    if(req.url==='/test/setup'){
      if(input.publicKey?.length!==32)throw Error('Browser public key bound');
      if(browserHarness){await browserHarness.assertPrivate();await browserHarness.close();}
      browserHarness=await runtimeFixture(Uint8Array.from(input.publicKey),`http://127.0.0.1:${server.address().port}`,input.options,frozenBinary);
      currentOptions=input.options;output=browserHarness.current();
    }else if(req.url==='/test/control'&&browserHarness)output=await browserHarness.control(input);
    else if(req.url==='/test/restart'&&browserHarness){await browserHarness.restart();output=browserHarness.current();}
    else if(req.url==='/test/finish'&&browserHarness){await browserHarness.assertPrivate();output=await browserHarness.finish();console.log(JSON.stringify({client:'Chrome',kind:'process-memory',...currentOptions,...output}));}
    else if(req.url==='/test/result'){resultResolve(input);output={};}
    else{res.writeHead(404).end();return;}
    res.setHeader('Content-Type','application/json');res.end(JSON.stringify(output));
  }catch{res.writeHead(500).end('Local runtime fixture unavailable');}
});
try{
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  browser=spawn(chrome,['--headless','--no-first-run','--disable-background-networking','--disable-component-update','--disable-sync','--disable-default-apps','--no-proxy-server',`--user-data-dir=${profile}`,`http://127.0.0.1:${server.address().port}`],{stdio:'ignore',detached:true});
  browser.on('error',()=>resultResolve({ok:false,error:'Chrome launch failed'}));browser.on('exit',()=>resultResolve({ok:false,error:'Chrome exited before result'}));
  const r=await Promise.race([result,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Runtime browser matrix timeout')),900000);})]);
  if(!r.ok||r.checks?.length!==nodeChecks.length||r.receipts?.length!==cells)throw Error(r.error??'Incomplete Chrome runtime matrix');
  for(const receipt of r.receipts)console.log(JSON.stringify({client:'Chrome',...receipt}));
  console.log(`${execFileSync(chrome,['--version'],{encoding:'utf8'}).trim()}: ${r.checks.length} joined Runtime qualification groups passed`);
  console.log(`${packed?'Retained pack candidate':'C4'}: synthetic dependencies and local OS clock/root; not Nitro, independent AWS witness, STS, native financial qualification or production memory calibration.`);
}finally{
  clearTimeout(timer);await stopChild(browser,{group:true});await browserHarness?.close();server.closeAllConnections();
  if(server.listening)await new Promise(resolve=>server.close(resolve));await rm(profile,{recursive:true,force:true,maxRetries:4,retryDelay:100});
}
}
try{await main();}finally{if(frozenRoot)await rm(frozenRoot,{recursive:true,force:true});}
