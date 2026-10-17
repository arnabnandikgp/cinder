// Real SDK -> Rust unit service -> native codecs/signatures -> one journal.
// PRIVATE fixture IPC bypasses attestation/carrier on purpose; separate carrier
// suites own that qualification. No sockets, venue/RPC/AWS or real funds.
import test from 'node:test';
import assert from 'node:assert/strict';
import {spawn,type ChildProcessWithoutNullStreams} from 'node:child_process';
import {once} from 'node:events';
import {createInterface} from 'node:readline';
import {mkdtemp,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
import {createHash,createPrivateKey,createPublicKey,sign,verify} from 'node:crypto';
import {UnitClient,UnitOnboardingClient,READ,TRADE,CANCEL,type ConfidentialChannel,type MessageSigner,type Command,type Response} from '../src/index.ts';
const id=(n:number)=>new Uint8Array(32).fill(n),domain={network:id(1),deployment:id(2)};
const bytes=(x:Uint8Array)=>Buffer.from(x);
function signer(seed:number):MessageSigner {
  const key=createPrivateKey({key:Buffer.concat([Buffer.from('302e020100300506032b657004220420','hex'),Buffer.alloc(32,seed)]),format:'der',type:'pkcs8'});
  return {publicKey:new Uint8Array(createPublicKey(key).export({format:'der',type:'spki'}).subarray(-32)),signMessage:async b=>new Uint8Array(sign(null,b,key))};
}
const alphabet='123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
function from58(s:string):Buffer{let n=0n;for(const c of s){const i=alphabet.indexOf(c);assert(i>=0);n=n*58n+BigInt(i);}const h=n.toString(16);const body=n===0n?Buffer.alloc(0):Buffer.from(h.length%2?'0'+h:h,'hex');return Buffer.concat([Buffer.alloc(s.match(/^1*/)?.[0].length??0),body]);}
function to58(b:Uint8Array):string{let n=BigInt('0x'+bytes(b).toString('hex')),s='';while(n){s=alphabet[Number(n%58n)]+s;n/=58n;}let z=0;while(z<b.length&&b[z]===0)z++;return '1'.repeat(z)+s;}
const le=(n:bigint,width:number)=>{const b=Buffer.alloc(width);for(let i=0;i<width;i++){b[i]=Number(n&255n);n>>=8n;}assert.equal(n,0n);return b;};
type Json=Record<string,unknown>;
interface Pipe {child:ChildProcessWithoutNullStreams;contract:Uint8Array;expiresAt:bigint;call:(v:Json)=>Promise<Json>;close:()=>Promise<void>}
async function pipe(root:string,wallet:Uint8Array):Promise<Pipe>{
  const child=spawn(join(resolve(process.env.CARGO_TARGET_DIR??'target'),'debug','cinder-service-fixture'),[root,'--unit-pipe',bytes(wallet).toString('hex')],{stdio:['pipe','pipe','pipe']});
  const queue:Json[]=[],waiting:Array<{resolve:(j:Json)=>void;reject:(e:Error)=>void}>=[];let failed=false;
  const reject=()=>{failed=true;for(const w of waiting.splice(0))w.reject(new Error('unit fixture exited'));};
  child.on('error',reject);child.on('exit',reject);child.stderr.resume();
  const lines=createInterface({input:child.stdout});lines.on('line',line=>{
    if(line.length>2_097_152){child.kill('SIGKILL');return;}
    let value:Json;try{value=JSON.parse(line) as Json;}catch{child.kill('SIGKILL');return;}
    const w=waiting.shift();if(w)w.resolve(value);else queue.push(value);
  });
  const next=()=>new Promise<Json>((resolve,reject)=>{
    const ready=queue.shift();if(ready){resolve(ready);return;}if(failed){reject(new Error('unit fixture stopped'));return;}
    const timer=setTimeout(()=>{child.kill('SIGKILL');reject(new Error('unit fixture response deadline'));},10000);
    waiting.push({resolve:j=>{clearTimeout(timer);resolve(j);},reject:e=>{clearTimeout(timer);reject(e);}});
  });
  child.stdin.write(Buffer.alloc(32,66));
  const ready=await next();assert.equal(ready.ready,true);assert.equal(typeof ready.contract,'string');assert.equal(typeof ready.expires,'number');
  return {child,contract:new Uint8Array(Buffer.from(ready.contract as string,'hex')),expiresAt:BigInt(ready.expires as number),
    call:async value=>{const result=next();const body=Buffer.from(JSON.stringify(value));assert(body.length<=1_048_576);child.stdin.write(Buffer.concat([le(BigInt(body.length),4).reverse(),body]));return result;},
    close:async()=>{if(child.exitCode!==null||child.signalCode!==null)return;const done=once(child,'exit');child.stdin.end();const timer=setTimeout(()=>child.kill('SIGKILL'),3000);const [code,signal]=await done;clearTimeout(timer);lines.close();assert.equal(code,0);assert.equal(signal,null);}};
}
function receipt(r:Response){assert.equal(r.kind,'receipt',r.kind==='error'?r.code:r.kind);if(r.kind!=='receipt')throw Error('receipt');return r;}
function view(r:Response){assert.equal(r.kind,'view');if(r.kind!=='view')throw Error('view');return r;}
function native(r:Json):Record<string,unknown>{assert.equal(typeof r.wire,'string');return JSON.parse(Buffer.from(r.wire as string,'base64').toString()) as Record<string,unknown>;}

test('joined SDK unit lifecycle preserves native attribution, shared revoke and restart without resend',{timeout:45000},async()=>{
  const root=await mkdtemp(join(tmpdir(),'cinder-units-acceptance-'));const owner=signer(9),agent=signer(10);let current:Pipe|undefined;
  try{
    current=await pipe(root,owner.publicKey);
    const channel:ConfidentialChannel={context:()=>({...domain,binding:id(8),expiresAt:current!.expiresAt}),exchange:async wire=>{const r=await current!.call({op:'request',wire:bytes(wire).toString('base64')});if(typeof r.wire!=='string')throw Error('fixture exchange unavailable');return new Uint8Array(Buffer.from(r.wire,'base64'));}};
    const boot=new UnitOnboardingClient(channel,owner,{domain,policy:1,unit:id(40),contract:current.contract});
    const discovered=await boot.open(BigInt(Date.now()+60000));assert.equal(discovered.kind,'discovery');if(discovered.kind!=='discovery')throw Error('bootstrap');
    const client=(unit:number,s:MessageSigner=owner)=>new UnitClient(channel,s,{domain,account:discovered.account,policy:1,unit:id(unit)});
    const request=(unit:number,n:number,command:Command,epoch=1n,s:MessageSigner=owner)=>client(unit,s).request({id:id(n),epoch,expiresAt:BigInt(Date.now()+60000),command});
    for(const unit of [40,50]){const r=view(await request(unit,10,{kind:'view'}));assert.equal(r.cash,1000n);assert(r.positions.every(p=>p.lots===0n));}
    receipt(await request(40,20,{kind:'grant',grant:{key:agent.publicKey,methods:READ|TRADE|CANCEL,market:id(7),maximumLots:2n,maximumFee:1n,maximumOrders:2n,expiresAt:BigInt(Date.now()+300000)}}));
    assert.equal((await request(50,21,{kind:'view'},1n,agent)).kind,'error');
    const order:Command={kind:'order',market:id(7),lots:2n,minimum:90n,maximum:110n,fee:1n,tif:'GTC',reduceOnly:false,goodUntil:BigInt(Date.now()+120000)};
    assert.equal(receipt(await request(40,30,order,1n,agent)).outcome,'accepted');
    const out=native(await current.call({op:'expose',unit:40,request:30,nonce:123}));
    assert.equal(out.nonce,'123');assert.equal(out.signer,to58(signer(11).publicKey));assert.equal(out.account,to58(signer(31).publicKey));
    const action=(out.actions as Array<{l:{c:string;b:boolean;px:string;sz:string;tif:string;r:boolean;i:boolean}}>)[0].l;
    assert.equal(action.r,false);assert.equal(action.i,false);assert.equal(action.px,'110.00000000');assert.equal(action.sz,'2.00000000');
    const symbol=Buffer.from(action.c),scale=(s:string)=>{assert.match(s,/^\d+\.\d{8}$/);return BigInt(s.replace('.',''));};
    const encoded=Buffer.concat([le(1n,4),le(BigInt(symbol.length),8),symbol,Buffer.from([action.b?1:0]),le(scale(action.px),8),le(scale(action.sz),8),le(0n,4),Buffer.from([0,0])]);
    const signed=Buffer.concat([le(1n,8),encoded,le(123n,8),from58(out.account as string),Buffer.from([2])]);
    const spki=Buffer.concat([Buffer.from('302a300506032b6570032100','hex'),from58(out.signer as string)]);
    assert(verify(null,signed,createPublicKey({key:spki,format:'der',type:'spki'}),from58(out.signature as string)));
    const oid=to58(createHash('sha256').update(Buffer.concat([le(0n,4),encoded,from58(out.account as string),le(123n,8)])).digest());
    assert.equal(receipt(await request(40,31,{kind:'operation',target:id(30)})).possiblyExposed,true);
    assert.deepEqual(await current.call({op:'expose',unit:50,request:30,nonce:124}),{error:true});
    const fill=(sequence:number,price:number)=>JSON.stringify({type:'account',topic:`account.${out.account}`,data:{type:'fill',tradeId:`100:${sequence}`,symbol:'BTC-USD',orderId:oid,price,size:1,fee:1,isBuy:true,maker:false,reasonCode:0,timestamp:10000000000}});
    for(let i=0;i<2;i++)assert.deepEqual(await current.call({op:'observe',unit:40,body:fill(1,100)}),{retained:true});
    const partial=view(await request(40,32,{kind:'view'}));assert.equal(partial.cash,999n);assert.equal(partial.positions[0].lots,1n);assert.equal(partial.positions[0].basis,100n);
    assert.equal(view(await request(50,33,{kind:'view'})).cash,1000n);
    assert.equal(receipt(await request(40,30,order,1n,agent)).filled,1n);
    // A retained gap contains A's new risk, not B's inventory or real late facts.
    assert.deepEqual(await current.call({op:'observe',unit:40,body:'{}'}),{retained:true});
    const restricted=await request(40,34,order,1n,agent);
    assert(restricted.kind==='error'||restricted.kind==='receipt'&&restricted.outcome==='rejected');
    assert.equal(receipt(await request(50,40,order)).outcome,'accepted');
    const p=native(await current.call({op:'expose',unit:50,request:40,nonce:1}));
    assert.equal(p.agent_wallet,to58(signer(12).publicKey));assert.equal(p.account,to58(signer(32).publicKey));
    const fields=['amount','client_order_id','price','reduce_only','side','symbol','tif'];const data=Object.fromEntries(fields.map(k=>[k,p[k]]));
    const preimage=Buffer.from(JSON.stringify({data,expiry_window:p.expiry_window,timestamp:p.timestamp,type:'create_order'}));
    const pkey=createPublicKey({key:Buffer.concat([Buffer.from('302a300506032b6570032100','hex'),from58(p.agent_wallet as string)]),format:'der',type:'spki'});
    assert(verify(null,preimage,pkey,from58(p.signature as string)));
    receipt(await request(50,41,{kind:'revoke'}));
    assert.equal((await request(40,42,{kind:'view'},1n,agent)).kind,'error');
    assert.deepEqual(await current.call({op:'observe',unit:40,body:fill(2,101)}),{retained:true});
    const nativeFill=JSON.stringify({channel:'account_trades',data:[{h:1,i:200,I:p.client_order_id,u:p.account,s:'BTC',a:'1',p:'100',o:'100',f:'1',n:'-1',te:'fulfill_taker',ts:'open_long',tc:'normal',t:p.timestamp,li:999,it:0}]});
    assert.deepEqual(await current.call({op:'observe',unit:50,body:nativeFill}),{retained:true});
    const a=view(await request(40,43,{kind:'view'},2n)),b=view(await request(50,44,{kind:'view'},2n));
    assert.equal(a.cash,998n);assert.equal(a.positions[0].lots,2n);assert.equal(a.positions[0].basis,201n);assert(a.held>0n);
    assert.equal(b.cash,999n);assert.equal(b.positions[0].lots,1n);assert.equal(b.positions[0].basis,100n);
    await current.close();current=await pipe(root,owner.publicKey);
    const restored=view(await request(40,45,{kind:'view'},2n));assert.equal(restored.cash,a.cash);assert.deepEqual(restored.positions,a.positions);
    assert.deepEqual(await current.call({op:'expose',unit:40,request:30,nonce:125}),{error:true});
    assert.equal(receipt(await request(40,46,{kind:'operation',target:id(30)},2n)).filled,2n);
    await current.call({op:'fence'});await assert.rejects(request(40,47,{kind:'view'},2n));await assert.rejects(request(50,48,{kind:'view'},2n));
  }finally{if(current)await current.close();await rm(root,{recursive:true,force:true});}
});
