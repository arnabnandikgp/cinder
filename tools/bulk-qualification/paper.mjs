// Disposable native paper test only. Not a shipping signer, enclave or ledger.
import { createPrivateKey, createPublicKey, randomBytes, sign, verify } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readdirSync, lstatSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { exclusive, privateRead, sha, canonical } from '../native-qualification/core.mjs';
import { ORIGIN, boundedBody } from './metadata.mjs';

export const CAPS = Object.freeze({signed:14, account:24, public:12, orders:5, notional:'200000000000000000', duration_ms:1800000});
const B58 = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
export function base58(bytes) {
  let n = BigInt('0x'+(Buffer.from(bytes).toString('hex') || '0')), out = '';
  while(n) { out = B58[Number(n%58n)]+out; n/=58n; }
  for(const b of bytes) { if(b)break; out='1'+out; }
  return out;
}
export function decode58(s) {
  if(typeof s!=='string'||s.length>88) throw Error('base58 shape');
  let n=0n;for(const c of s){const d=B58.indexOf(c);if(d<0)throw Error('base58 alphabet');n=n*58n+BigInt(d);}
  const hex=n.toString(16), bytes=n?Buffer.from(hex.length%2?'0'+hex:hex,'hex'):Buffer.alloc(0);
  return Buffer.concat([Buffer.alloc(s.match(/^1*/)[0].length),bytes]);
}
// Preserve every JSON numeric token as text; use BigInt for financial decisions.
export const exact = raw => JSON.parse(raw, (_k,v,c)=>typeof v==='number'?c.source:v);
export function atoms(s) {
  if(typeof s!=='string'||!/^-?(0|[1-9]\d*)(\.\d{1,8})?$/.test(s))throw Error('exact decimal');
  const neg=s.startsWith('-'),[a,b='']=s.replace(/^-/, '').split('.');
  return (BigInt(a)*100000000n+BigInt(b.padEnd(8,'0')))*(neg?-1n:1n);
}
export const decimal = n => `${n/100000000n}.${(n%100000000n).toString().padStart(8,'0')}`;
const u32=n=>{const b=Buffer.alloc(4);b.writeUInt32LE(n);return b;};
const u64=n=>{const b=Buffer.alloc(8);b.writeBigUInt64LE(BigInt(n));return b;};
export function management(account,nonce,agent,remove=false) {
  const a=decode58(account);if(a.length!==32)throw Error('account bytes');
  let action, payload;
  if(agent) {const b=decode58(agent);if(b.length!==32)throw Error('agent bytes');action={agentWalletCreation:{a:agent,d:remove}};payload=Buffer.concat([u32(17),b,Buffer.from([Number(remove)])]);}
  else {action={faucet:{u:account}};payload=Buffer.concat([u32(16),a,Buffer.from([0])]);}
  return {action,message:Buffer.concat([u64(1),payload,u64(nonce),a,Buffer.from([2])])};
}
export function budget(rows, spec, now, start) {
  if(now<start||now-start>=CAPS.duration_ms)throw Error('paper run deadline');
  const count=k=>rows.filter(r=>r.category===k).length;
  if(!['signed','account','public'].includes(spec.category)||count(spec.category)>=CAPS[spec.category])throw Error('request cap');
  if(spec.cleanup!==undefined&&(spec.kind!=='limit'||typeof spec.cleanup!=='boolean'))throw Error('cleanup is an explicit limit-only declaration');
  const limits=rows.filter(r=>r.kind==='limit'),openings=limits.filter(r=>r.cleanup!==true);
  const cleanupStarted=limits.some(r=>r.cleanup===true);
  // Cancellations and ACKs never refund reserved cleanup capacity.
  if(spec.category==='signed'&&openings.length&&!cleanupStarted&&spec.cleanup!==true&&count('signed')+2>CAPS.signed)throw Error('signed cleanup reserve');
  if(spec.kind==='limit') {
    const n=atoms(spec.price)*atoms(spec.size);
    const used=rows.reduce((a,r)=>a+BigInt(r.notional??0),0n);
    if(spec.category!=='signed'||atoms(spec.price)<=0n||atoms(spec.size)<=0n||limits.length>=CAPS.orders||used+n>BigInt(CAPS.notional))throw Error('order/notional cap');
    if(spec.cleanup===true) {
      if(!openings.length||spec.tif!=='IOC'||typeof spec.buy!=='boolean'||openings.some(r=>r.buy===spec.buy))throw Error('cleanup must oppose original openings with IOC');
      // An admitted cleanup consumes quantity even if its reply/fill is unknown.
      const consumed=limits.filter(r=>r.cleanup===true).reduce((a,r)=>a+atoms(r.size),0n);
      const quantity=openings.reduce((a,r)=>a+atoms(r.size),0n)-consumed,ceiling=openings.reduce((a,r)=>a>atoms(r.cleanupPrice)?a:atoms(r.cleanupPrice),0n);
      if(atoms(spec.size)>quantity||atoms(spec.price)>ceiling)throw Error('cleanup exceeds original quantity/price reservation');
    } else {
      if(cleanupStarted||typeof spec.buy!=='boolean'||openings.some(r=>r.buy!==spec.buy))throw Error('opening after cleanup or mixed-side opening');
      const price=atoms(spec.cleanupPrice);
      if(price<atoms(spec.price))throw Error('explicit cleanup price ceiling required');
      const quantity=openings.reduce((a,r)=>a+atoms(r.size),atoms(spec.size));
      const ceiling=openings.reduce((a,r)=>a>atoms(r.cleanupPrice)?a:atoms(r.cleanupPrice),price);
      if(limits.length+2>CAPS.orders||count('signed')+2>CAPS.signed||used+n+ceiling*quantity>BigInt(CAPS.notional))throw Error('order/notional cleanup reserve');
    }
    return n.toString();
  }
  return '0';
}
const files = ['.node-version','tools/bulk-qualification/paper.mjs','tools/bulk-qualification/metadata.mjs','tools/native-qualification/core.mjs','crates/bulk/examples/paper-codec.rs','crates/bulk/src/wire.rs','crates/bulk/src/fills.rs','crates/bulk/src/precision.rs'];
const root=resolve(import.meta.dirname,'../..'), binary=join(root,'target/debug/examples/paper-codec');
export function environment(version=process.versions.node, tls=process.env.NODE_TLS_REJECT_UNAUTHORIZED) {
  if(version!==readFileSync(join(root,'.node-version'),'utf8').trim())throw Error('Use pinned Node');
  if(tls==='0')throw Error('TLS verification required');
}
const sources=()=>sha(canonical([...files.map(p=>[p,sha(readFileSync(join(root,p)))]),['binary',sha(readFileSync(binary))]]));
function key(seed) { return createPrivateKey({key:Buffer.concat([Buffer.from('302e020100300506032b657004220420','hex'),Buffer.from(seed,'hex')]),format:'der',type:'pkcs8'}); }
function pub(k) { return base58(Buffer.from(createPublicKey(k).export({format:'jwk'}).x,'base64url')); }
function directory(d) {const s=lstatSync(d);if(!s.isDirectory()||s.isSymbolicLink()||s.uid!==process.getuid()||s.mode&0o077)throw Error('private directory');}
export function initialize(d, reference, fingerprint=sources) {
  environment();
  directory(d);if(readdirSync(d).length||typeof reference!=='string'||!reference.trim())throw Error('fresh directory/approval reference');
  const seeds=Object.fromEntries(['owner','agentA','agentB'].map(k=>[k,randomBytes(32).toString('hex')]));
  const keys=Object.fromEntries(Object.entries(seeds).map(([k,s])=>[k,pub(key(s))]));
  exclusive(join(d,'keys.json'),seeds);
  exclusive(join(d,'manifest.json'),{schema:'cinder-bulk-paper-v1',origin:ORIGIN,keys,caps:CAPS,reference,sources:fingerprint(),created_at:Date.now()});
  return {keys};
}
export class Paper {
  constructor(d, fetcher=fetch, clock=Date.now, fingerprint=sources) {
    environment();
    directory(d);this.d=d;this.fetch=fetcher;this.now=clock;
    this.m=JSON.parse(privateRead(join(d,'manifest.json')));
    if(this.m.schema!=='cinder-bulk-paper-v1'||this.m.origin!==ORIGIN||canonical(this.m.caps)!==canonical(CAPS)||this.m.sources!==fingerprint())throw Error('paper source/scope changed');
    this.rows=readdirSync(d).filter(x=>/^request-\d+\.json$/.test(x)).map(x=>JSON.parse(privateRead(join(d,x)))).sort((a,b)=>a.index-b.index);
    this.rows.forEach((r,i)=>{if(r.index!==i+1)throw Error('request history gap');});
  }
  codec(input) {return JSON.parse(execFileSync(binary,[],{input:JSON.stringify({account:this.m.keys.owner,size_step:'0.00000001',price_step:'0.01',...input}),encoding:'utf8',maxBuffer:1000000}));}
  async request(spec) {
    environment();
    const at=this.now(), index=this.rows.length+1;
    const notional=budget(this.rows,spec,at,this.m.created_at);
    if(typeof spec.id!=='string'||!spec.id.match(/^[a-z0-9-]{1,64}$/)||this.rows.some(r=>r.id===spec.id))throw Error('unique original request id');
    let path, method='POST', body, message, nonce, oid;
    const owner=this.m.keys.owner;
    if(spec.category==='public') {
      if(spec.kind==='info')path='/api/v1/exchangeInfo';
      else if(spec.kind==='book')path='/api/v1/l2book?type=l2book&coin=BTC-USD&nlevels=10';
      else throw Error('public scope');method='GET';
    } else if(spec.category==='account') {
      if(!['fullAccount','openOrders','fills','orderHistory','feeTier'].includes(spec.kind))throw Error('account scope');
      path='/api/v1/account';body={type:spec.kind,user:owner};
      if(['fills','orderHistory'].includes(spec.kind))body.limit=128;
      if(spec.kind==='feeTier')body.symbol='BTC-USD';
    } else {
      if(!['faucet','agent','limit','cancel'].includes(spec.kind))throw Error('signed scope');
      const role=spec.role??'owner';if(!['owner','agentA','agentB'].includes(role))throw Error('fresh role only');
      if(['faucet','agent'].includes(spec.kind)&&role!=='owner')throw Error('owner setup only');
      if(spec.kind==='limit'&&role==='owner')throw Error('no master trading fallback');
      nonce=(BigInt(at)*1000000n+BigInt(index)).toString();
      if(this.rows.some(r=>r.nonce&&BigInt(r.nonce)>=BigInt(nonce)))throw Error('nonce monotonicity');
      let encoded;
      if(spec.kind==='agent') {
        if(!['agentA','agentB'].includes(spec.agent)||typeof spec.remove!=='boolean')throw Error('agent scope');
        encoded=management(owner,nonce,this.m.keys[spec.agent],spec.remove);
      } else if(spec.kind==='faucet') {
        if(this.rows.some(r=>r.kind==='faucet'))throw Error('one faucet attempt');
        encoded=management(owner,nonce);
      } else if(spec.kind==='limit') {
        if(typeof spec.buy!=='boolean'||!['ALO','IOC'].includes(spec.tif))throw Error('order scope');
        encoded=this.codec({...spec,account:owner,nonce});oid=encoded.order;
      } else {
        const original=this.rows.find(r=>r.id===spec.original&&r.kind==='limit');
        if(!original?.order)throw Error('cancel must bind original persisted order');
        encoded=this.codec({kind:'cancel',order:original.order,nonce});oid=original.order;
      }
      message=Buffer.from(encoded.message);path='/api/v1/order';
      body={actions:[encoded.action],nonce,account:owner,signer:this.m.keys[role]};
    }
    const record={...spec,index,at,path,method,notional,nonce:nonce??null,order:oid??null,body:body??null,message:message?.toString('hex')??null};
    // Persist identity/action/nonce/cap consumption before any signature or I/O.
    exclusive(join(this.d,`request-${index}.json`),record);this.rows.push(record);
    if(message) {
      const seeds=JSON.parse(privateRead(join(this.d,'keys.json'))),k=key(seeds[spec.role??'owner']);
      const signature=sign(null,message,k);if(!verify(null,message,createPublicKey(k),signature))throw Error('local signature verification');
      body.signature=base58(signature);
    }
    const wire=body?JSON.stringify(body):undefined;
    // Signed wires remain private, retained before transmission. Never print them.
    exclusive(join(this.d,`wire-${index}.json`),{body:wire??null});
    let result;
    try {
      const controller=new AbortController(),timer=setTimeout(()=>controller.abort(),20000);
      try {
        const response=await this.fetch(ORIGIN+path,{method,headers:body?{'Content-Type':'application/json'}:{},body:wire,redirect:'error',credentials:'omit',signal:controller.signal});
        const bytes=await boundedBody(response);
        result={status:response.status,bytes:bytes.length,sha256:sha(bytes),raw_base64:bytes.toString('base64'),at:this.now()};
      } finally {clearTimeout(timer);}
    } catch {result={status:null,outcome:'transport-unknown',at:this.now()};}
    exclusive(join(this.d,`response-${index}.json`),result);
    return {index,id:spec.id,status:result.status,order:oid??null,bytes:result.bytes??0};
  }
}
if(process.argv[1]===new URL(import.meta.url).pathname) {
  environment();
  const [mode,d,arg]=process.argv.slice(2);
  if(mode==='--init') console.log(JSON.stringify(initialize(resolve(d),arg)));
  else if(mode==='--step') console.log(JSON.stringify(await new Paper(resolve(d)).request(JSON.parse(arg))));
  else throw Error('Usage: paper.mjs --init PRIVATE_DIR APPROVAL_REFERENCE | --step PRIVATE_DIR JSON_SPEC');
}
