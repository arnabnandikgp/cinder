#!/usr/bin/env node
// One settings-only continuation after a closed staged run returned all net.
// Does not mutate the parent's journal/result or repeat an economic operation.
import {mkdirSync,readFileSync,lstatSync} from 'node:fs';
import {join,resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {setTimeout as pause} from 'node:timers/promises';
import {canonical,sha,privateRead,exclusive,Journal,validateManifest,stagedInitialization,stagedIdle,
  decodeResponse,tokenDelta} from './core.mjs';
import {checkEnvironment,runDirectory,tlsRoots,validateSignerLocator} from './artifacts.mjs';

export const BOUNDS=Object.freeze({http:10,posts:1,interval_ms:12000,body_bytes:8192,request_ms:10000});
const source=()=>sha(canonical(Object.fromEntries(['finish-staging.mjs','core.mjs','artifacts.mjs','bindings.mjs',
  '../../clients/vault/package.json','../../clients/vault/package-lock.json'].map(p=>[p,sha(readFileSync(new URL(p,import.meta.url)))]))));
export function eligible(m,result,rows,now) {
  validateManifest(m);
  const start=rows[0]?.at,http=rows.filter(r=>r.kind==='http').length;
  if(m.schema!=='cinder-native-staged-bootstrap-v1'||rows[0]?.kind!=='start'||rows.at(-1)?.kind!=='closed'
    ||rows.some(r=>r.kind==='native-intent'&&r.data.type==='set_auto_lend_disabled')
    ||result.owner_atoms!==result.payment?.net||result.broker_atoms!=='0'||result.pending_balance_observed!=='0'
    ||result.source_cut!==null||result.shipping!==false||result.financial_completion!==false
    ||!Number.isSafeInteger(start)||now<start||now+BOUNDS.http*BOUNDS.interval_ms+BOUNDS.request_ms>=start+m.limits.duration_ms
    ||http+BOUNDS.http>m.limits.http)throw Error('Closed returned staged run and remaining budget required');
  return {deadline:start+m.limits.duration_ms,parent_http:http,last_http:rows.filter(r=>r.kind==='http').at(-1)?.at??start};
}
function parent(directory,ROUTE) {
  const dir=runDirectory(directory),m=JSON.parse(privateRead(join(dir,'manifest.json'))),
    result=JSON.parse(privateRead(join(dir,'result.json'))),journal=new Journal(dir,m,Date.now);
  const budget=eligible(m,result,journal.rows,Date.now());
  const receipt=JSON.parse(privateRead(join(dir,'receipt-return-net.json'))),net=BigInt(result.payment.net);
  if(tokenDelta(receipt.tx,receipt.status,{signature:receipt.signature,ata:m.owner_tokens,mint:ROUTE.mint,owner:m.owner})!==net
    ||tokenDelta(receipt.tx,receipt.status,{signature:receipt.signature,ata:m.broker_tokens,mint:ROUTE.mint,owner:m.broker})!==-net)
    throw Error('Original finalized return required');
  return {dir,m,result,journal,budget};
}
export async function main(mode,directory) {
  checkEnvironment();const {decodeSigner,signNative,base58,ROUTE}=await import('./bindings.mjs');
  const p=parent(directory,ROUTE),dir=join(p.dir,'settings-followup');
  const current={schema:'cinder-staged-settings-followup-v1',parent_manifest:validateManifest(p.m),parent_head:p.journal.head,
    source:source(),tls_roots:tlsRoots(),broker:p.m.broker,bounds:BOUNDS,...p.budget,
    financial_actions:0,aws:false,shipping:false};
  if(mode==='prepare') {
    mkdirSync(dir,{mode:0o700});exclusive(join(dir,'scope.json'),current);
    console.log(JSON.stringify({scope:sha(canonical(current)),deadline:current.deadline,financial_actions:0,live:false}));return;
  }
  if(mode!=='run')throw Error('Explicit prepare/run required');
  runDirectory(p.dir); // Recheck parent path before reading the private child.
  const st=lstatSync(dir);if(!st.isDirectory()||st.isSymbolicLink()||st.uid!==process.getuid()||st.mode&0o077)throw Error('Private followup directory required');
  const scope=JSON.parse(privateRead(join(dir,'scope.json'))),approval=JSON.parse(privateRead(join(dir,'approval.json')));
  if(canonical(scope)!==canonical(current)||approval.approved!==true||approval.scope!==sha(canonical(scope))
    ||typeof approval.reference!=='string'||!approval.reference.trim()||!Number.isSafeInteger(approval.not_after)
    ||Date.now()>=approval.not_after||approval.not_after>scope.deadline)throw Error('Exact settings-only approval required');
  validateSignerLocator(join(p.dir,'broker.key'));
  mkdirSync(join(dir,'running.lock'),{mode:0o700});
  let calls=0,last=scope.last_http;
  const check=()=>{if(Date.now()>=approval.not_after||Date.now()>=scope.deadline||Date.now()<last)throw Error('Followup deadline/clock');};
  const request=async(path,options)=>{
    if(calls>=BOUNDS.http)throw Error('Followup request cap');
    await pause(Math.max(0,last+BOUNDS.interval_ms-Date.now()));check();last=Date.now();
    const n=++calls;exclusive(join(dir,`request-${n}.json`),{path,options,at:last});
    const response=await fetch(p.m.native_origin+'/api/v1/'+path,{...options,redirect:'error',signal:AbortSignal.timeout(BOUNDS.request_ms)});
    const chunks=[];let size=0;if(!response.body)throw Error('Missing response body');
    for await(const chunk of response.body){size+=chunk.length;if(size>BOUNDS.body_bytes)throw Error('Followup body bound');chunks.push(chunk);}
    const bytes=Buffer.concat(chunks,size);exclusive(join(dir,`response-${n}.json`),{status:response.status,body:bytes.toString(),at:Date.now()});check();
    if(response.status===429||response.status>=500)throw Error('Upstream refused; no retry');
    return {status:response.status,body:decodeResponse(bytes)};
  };
  const get=path=>request(path+'?account='+encodeURIComponent(p.m.broker)+(path.endsWith('history')?'&limit=20':'')
    +(path==='account/balance/history'?'&include_trades=true':''),{method:'GET',headers:{accept:'application/json'}});
  try {
    if(!stagedInitialization(await get('account'),await get('account/loan'),Date.now(),'0'))throw Error('Empty account initialization incomplete');
    const timestamp=Date.now(),header={timestamp,expiry_window:30000,type:'set_auto_lend_disabled'},message=Buffer.from(canonical({...header,data:{disabled:true}})),
      key=decodeSigner(JSON.parse(privateRead(join(p.dir,'broker.key'),8192)),p.m.broker),
      body={account:p.m.broker,timestamp,expiry_window:30000,disabled:true,signature:base58.encode(signNative(message,key))};
    exclusive(join(dir,'original-setting.json'),{message:message.toString(),body});
    await pause(Math.max(0,last+BOUNDS.interval_ms-Date.now()));check();
    if(Date.now()-timestamp>=30000)throw Error('Original setting expired unsent');
    exclusive(join(dir,'exposed.json'),{identity:'disable-empty-staged',request_hash:sha(canonical(body)),at:Date.now()});
    const reply=await request('account/settings/auto_lend_disabled',{method:'POST',headers:{'content-type':'application/json'},body:canonical(body)});
    if(reply.status!==200||reply.body.success!==true||reply.body.error!=null||reply.body.code!=null)throw Error('Original setting unconfirmed; no retry');
    const settings=await get('account/settings'),loan=await get('account/loan'),account=await get('account'),positions=await get('positions'),orders=await get('orders');
    const setup=stagedIdle(settings,loan,account,positions,orders,Date.now(),'0');
    const deposit=await get('account/deposit/history'),history=await get('account/balance/history'),original=JSON.parse(privateRead(join(p.dir,'receipt-native-deposit.json')));
    const d=deposit.body?.data,h=history.body?.data;
    if(deposit.status!==200||deposit.body.success!==true||deposit.body.has_more!==false||!Array.isArray(d)||d.length!==1
      ||d[0].transaction_id!==original.signature||d[0].amount!=='20'||history.status!==200||history.body.success!==true
      ||history.body.has_more!==false||!Array.isArray(h)||h.length!==2||h[0].event_type!=='withdraw'
      ||h[0].amount!=='-20'||h[0].balance!=='0'||h[0].pending_balance!=='0')throw Error('Returned original history not corroborated');
    const result={status:'observed-empty-disabled-no-debt',setup,http:calls,total_http:scope.parent_http+calls,
      original_signature_history_observed:true,latest_balance_history:'withdrawal-zero-cash',original_result_unchanged:true,
      bootstrap_before_withdrawal_qualified:false,financial_actions:0,source_cut:null,shipping:false,nitro:false,financial_completion:false};
    exclusive(join(dir,'result.json'),result);console.log(JSON.stringify(result));
  } catch {exclusive(join(dir,'stopped.json'),{requests:calls,financial_actions:0,shipping:false});
    console.error('Settings-only continuation stopped; preserve originals, no retry.');process.exitCode=1;}
}
if(process.argv[1]&&pathToFileURL(resolve(process.argv[1])).href===import.meta.url) {
  if(process.argv.length!==4)throw Error('Usage: finish-staging.mjs prepare|run ORIGINAL_STAGED_DIRECTORY');
  main(...process.argv.slice(2)).catch(()=>{console.error('Settings-only continuation refused before exposure.');process.exitCode=1;});
}
