#!/usr/bin/env node
// Separate GET-only schema observation. Never reopens a funding invocation.
import { readFileSync, mkdirSync, lstatSync } from 'node:fs';
import { join, resolve, relative } from 'node:path';
import { pathToFileURL } from 'node:url';
import { setTimeout as pause } from 'node:timers/promises';
import { sha, exclusive } from './core.mjs';
import { root, checkEnvironment, tlsRoots } from './artifacts.mjs';

export const ORIGIN = 'https://test-api.pacifica.fi';
export const BOUNDS = Object.freeze({requests:12, interval_ms:12000, request_ms:12000,
  duration_ms:180000, body_bytes:8192});
const paths = ['account/deposit/history','account/balance/history','account/settings',
  'account/loan','account','positions','orders'];
export function targets(account, absent) {
  if(account===absent || [account,absent].some(v=>typeof v!=='string'||!/^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(v)))
    throw Error('Two distinct public account addresses required');
  return [...paths.map(path=>({role:'established',account,path})),
    ...paths.slice(0,5).map(path=>({role:'uninitialized-probe',account:absent,path}))].map(row=>{
      const url=new URL('/api/v1/'+row.path,ORIGIN);url.searchParams.set('account',row.account);
      if(row.path.endsWith('history'))url.searchParams.set('limit','32');
      if(row.path==='account/balance/history')url.searchParams.set('include_trades','true');
      return {...row,url:url.href};
    });
}
// No arbitrary strings, signatures, addresses or upstream error messages in stdout.
export function shape(value, depth=0) {
  if(depth>8)throw Error('Response nesting bound');
  if(value===null)return 'null';
  if(Array.isArray(value))return {rows:value.length,items:[...new Map(value.map(v=>{
    const s=shape(v,depth+1);return [JSON.stringify(s),s];
  })).values()]};
  if(typeof value==='object')return Object.fromEntries(Object.entries(value).map(([k,v])=>{
    if(!/^[a-zA-Z_][a-zA-Z0-9_]{0,63}$/.test(k))throw Error('Response field-name bound');
    return [k,shape(v,depth+1)];
  }));
  return typeof value;
}
export async function body(response) {
  if(!response.body)throw Error('Missing response body');
  const reader=response.body.getReader(),chunks=[];let length=0;
  try {
    for(;;){const {done,value}=await reader.read();if(done)break;
      length+=value.length;if(length>BOUNDS.body_bytes)throw Error('Response size bound');chunks.push(value);}
    return Buffer.concat(chunks,length);
  } finally {await reader.cancel().catch(()=>{});reader.releaseLock();}
}
export async function observe(account, absent, ports) {
  const plan=targets(account,absent),begin=ports.now();let last=begin-BOUNDS.interval_ms,count=0;
  for(const row of plan) {
    const delay=Math.max(0,last+BOUNDS.interval_ms-ports.now());await ports.wait(delay);
    const at=ports.now();
    if(at<last||at<begin||at-begin+BOUNDS.request_ms>BOUNDS.duration_ms||count>=BOUNDS.requests)
      throw Error('Observation time/request bound');
    last=at;
    // Durable request metadata precedes I/O; it contains no authorization header.
    ports.retain('request-'+(++count),{...row,method:'GET',at});
    const response=await ports.fetch(row.url,{method:'GET',redirect:'error',
      headers:{accept:'application/json'},signal:AbortSignal.timeout(BOUNDS.request_ms)});
    const bytes=await body(response),received_at=ports.now();
    ports.retain('response-'+count,{status:response.status,received_at,body:bytes.toString('utf8')});
    if(received_at<at||received_at-begin>BOUNDS.duration_ms)throw Error('Observation reply time bound');
    const value=JSON.parse(bytes.toString('utf8'));
    const report={request:count,role:row.role,path:row.path,status:response.status,
      bytes:bytes.length,sha256:sha(bytes),shape:shape(value)};
    ports.retain('summary-'+count,report);ports.report(report);
    if(response.status===429)throw Error('Rate limited; stop without retry');
    if(response.status>=500)throw Error('Upstream failure; stop without retry');
  }
  ports.retain('complete',{requests:count,financial_activity:false,aws:false,
    shipping_readiness:false,ended_at:ports.now()});
  return count;
}
export async function main(account, absent, directory) {
  checkEnvironment();targets(account,absent);
  const parent=resolve(root,'work/experiments/p23-native-schemas'),dir=resolve(directory);
  if(!/^observe-[a-zA-Z0-9-]+$/.test(relative(parent,dir)))throw Error('Fresh schema namespace required');
  let walk=root;
  for(const part of relative(root,parent).split('/')){
    walk=join(walk,part);
    try {const st=lstatSync(walk);if(!st.isDirectory()||st.isSymbolicLink())throw Error('Directory type');}
    catch(e){if(e.code!=='ENOENT')throw e;mkdirSync(walk,{mode:0o700});}
  }
  mkdirSync(dir,{mode:0o700}); // Never overwrite/restart an old observation.
  const provenance=['observe.mjs','core.mjs','artifacts.mjs'];
  exclusive(join(dir,'scope.json'),{schema:'cinder-pacifica-schema-observation-v1',account,absent,
    origin:ORIGIN,bounds:BOUNDS,node:process.versions.node,tls_roots:tlsRoots(),
    sources:Object.fromEntries(provenance.map(p=>[p,sha(readFileSync(new URL(p,import.meta.url)))])),
    authority:'2026-10-08 user go-for-it: public testnet GET/schema qualification only',
    keys:false,rpc:false,settings_writes:false,financial_activity:false,aws:false,started_at:Date.now()});
  return observe(account,absent,{now:Date.now,wait:pause,fetch,
    retain:(name,value)=>exclusive(join(dir,name+'.json'),value),report:v=>console.log(JSON.stringify(v))});
}
if(process.argv[1]&&pathToFileURL(resolve(process.argv[1])).href===import.meta.url) {
  if(process.argv.length!==5)throw Error('Usage: observe.mjs PUBLIC_ACCOUNT PUBLIC_ABSENCE_PROBE FRESH_DIRECTORY');
  main(...process.argv.slice(2)).catch(()=>{console.error('Read-only schema observation stopped; retain original artifacts.');process.exitCode=1;});
}
