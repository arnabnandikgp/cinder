import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, readdirSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createPublicKey, verify } from 'node:crypto';
import { base58, decode58, exact, atoms, decimal, management, budget, CAPS, environment, initialize, Paper } from '../tools/bulk-qualification/paper.mjs';

test('paper base58 and exact decimal helpers preserve keys, wide integers and atoms',()=>{
  for(const bytes of [Buffer.alloc(32),Buffer.alloc(32,1),Buffer.from([0,0,255]),Buffer.alloc(64,255)])assert.deepEqual(decode58(base58(bytes)),bytes);
  assert.throws(()=>decode58('0invalid'));assert.equal(base58(Buffer.alloc(32)),'1'.repeat(32));
  assert.equal(exact('{"nonce":1792499212510152669,"fee":-0.00000001}').nonce,'1792499212510152669');
  assert.equal(atoms('0.00000001'),1n);assert.equal(atoms('-1.2'),-120000000n);assert.equal(decimal(123n),'0.00000123');
  for(const s of ['1e-8','NaN','01','0.000000001'])assert.throws(()=>atoms(s));
});
test('paper management bytes encode only default faucet or fixed agent and testnet domain',()=>{
  const owner=base58(Buffer.alloc(32,1)),agent=base58(Buffer.alloc(32,2));
  const f=management(owner,'9007199254740993');
  assert.equal(f.message.readBigUInt64LE(),1n);assert.equal(f.message.readUInt32LE(8),16);
  assert.equal(f.message[44],0);assert.equal(f.message.readBigUInt64LE(45),9007199254740993n);assert.equal(f.message.at(-1),2);
  assert.deepEqual(f.action,{faucet:{u:owner}});
  const a=management(owner,'42',agent,true);assert.equal(a.message.readUInt32LE(8),17);assert.equal(a.message[44],1);assert.equal(a.message.at(-1),2);
  assert.deepEqual(a.action,{agentWalletCreation:{a:agent,d:true}});
});
test('paper budgets count every order and cleanup notional with finite request/deadline limits',()=>{
  const order={category:'signed',kind:'limit',buy:true,tif:'ALO',price:'100000',size:'0.00002',cleanupPrice:'120000'};
  assert.equal(budget([],order,1,0),'20000000000000000');
  assert.throws(()=>budget(Array.from({length:5},()=>({...order,notional:'1'})),order,1,0),/cap/);
  assert.throws(()=>budget([{category:'signed',notional:CAPS.notional}],order,1,0),/cap/);
  for(const c of ['signed','account','public'])assert.throws(()=>budget(Array.from({length:CAPS[c]},()=>({category:c})),{category:c},1,0),/cap/);
  assert.throws(()=>budget([],{category:'public'},CAPS.duration_ms,0),/deadline/);
  assert.throws(()=>budget([],{category:'public'},0,1),/deadline/);
});
test('paper reserves the fifth order, signed capacity and worst planned cleanup notional',()=>{
  const opening={category:'signed',kind:'limit',buy:true,tif:'ALO',price:'100000',size:'0.00002',cleanupPrice:'120000'};
  const rows=Array.from({length:4},()=>({...opening,notional:budget([],opening,1,0)}));
  assert.throws(()=>budget(rows,opening,1,0),/cleanup reserve/);
  const cleanup={...opening,buy:false,tif:'IOC',cleanup:true,size:'0.00008',price:'120000'};
  assert.equal(budget(rows,cleanup,1,0),'96000000000000000');
  assert.throws(()=>budget(rows,{...cleanup,size:'0.00009'},1,0),/quantity/);
  assert.throws(()=>budget(rows,{...cleanup,price:'120001'},1,0),/reservation/);
  assert.throws(()=>budget(rows,{...cleanup,buy:true},1,0),/oppose/);
  assert.throws(()=>budget(rows,{...cleanup,tif:'ALO'},1,0),/IOC/);
  assert.throws(()=>budget([],{...opening,size:'0.0001'},1,0),/cleanup reserve/);
  const signed=[opening,...Array.from({length:12},()=>({category:'signed',kind:'agent'}))];
  assert.throws(()=>budget(signed,{category:'signed',kind:'agent'},1,0),/signed cleanup reserve/);
  assert.equal(budget(signed,{...cleanup,size:'0.00002'},1,0),'24000000000000000');
  assert.throws(()=>budget([{...opening,cleanup:true}],opening,1,0),/after cleanup/);
  assert.throws(()=>budget([opening],{...opening,buy:false},1,0),/mixed-side/);
  assert.throws(()=>budget([],cleanup,1,0),/original openings/);
});
test('paper cleanup quantity is consumed cumulatively, including unknown and zero-fill attempts',()=>{
  const opening={category:'signed',kind:'limit',buy:true,tif:'ALO',price:'100000',size:'0.00004',cleanupPrice:'120000'};
  const cleanup={...opening,buy:false,tif:'IOC',cleanup:true,size:'0.00002',price:'120000'};
  const first={...cleanup,notional:budget([opening],cleanup,1,0),outcome:'transport-unknown'};
  assert.equal(budget([opening,first],cleanup,1,0),'24000000000000000');
  assert.throws(()=>budget([opening,first],{...cleanup,size:'0.00003'},1,0),/quantity/);
  const second={...cleanup,notional:budget([opening,first],cleanup,1,0),filled:'0'};
  const cancel={category:'signed',kind:'cancel',original:'cleanup'};
  assert.throws(()=>budget([opening,first,second,cancel],{...cleanup,size:'0.00000001'},1,0),/quantity/);
  assert.throws(()=>budget([opening,{...first,size:opening.size}],cleanup,1,0),/quantity/);
});
test('paper CLI refuses disabled TLS before initialization and runtime mismatches fail closed',t=>{
  assert.throws(()=>environment('0.0.0'),/pinned Node/);
  assert.throws(()=>environment(process.versions.node,'0'),/TLS verification/);
  const d=mkdtempSync(join(tmpdir(),'cinder-paper-guard-'));t.after(()=>rmSync(d,{recursive:true,force:true}));
  for(const mode of ['--init','--step']) {
    const result=spawnSync(process.execPath,['tools/bulk-qualification/paper.mjs',mode,d,'{}'],{encoding:'utf8',env:{...process.env,NODE_TLS_REJECT_UNAUTHORIZED:'0'}});
    assert.notEqual(result.status,0);assert.match(result.stderr,/TLS verification required/);assert.deepEqual(readdirSync(d),[]);
  }
});
function fixture(t,fetcher) {
  const d=mkdtempSync(join(tmpdir(),'cinder-bulk-paper-offline-'));t.after(()=>rmSync(d,{recursive:true,force:true}));
  initialize(d,'offline fixture only',()=> 'fixture-source');
  return {d,p:new Paper(d,fetcher,Date.now,()=> 'fixture-source')};
}
test('paper requests retain original action before signing and verify the transmitted signature offline',async t=>{
  let f,calls=0;
  f=fixture(t,async(url,options)=>{
    calls++;assert.equal(url,'https://exchange-api.bulk.trade/api/v1/order');
    assert.equal(options.redirect,'error');assert.equal(options.credentials,'omit');
    const retained=JSON.parse(readFileSync(join(f.d,'request-1.json')));
    assert.equal(retained.kind,'faucet');assert.equal(retained.body.signature,undefined);
    const wire=JSON.parse(options.body),m=Buffer.from(retained.message,'hex');
    const pub=createPublicKey({key:Buffer.concat([Buffer.from('302a300506032b6570032100','hex'),decode58(wire.signer)]),format:'der',type:'spki'});
    assert(verify(null,m,pub,decode58(wire.signature)));
    assert(readFileSync(join(f.d,'wire-1.json')).length>0);
    return new Response('{"status":"ok","wide":9007199254740993}',{status:200});
  });
  const spec={id:'faucet',category:'signed',kind:'faucet'};
  assert.equal((await f.p.request(spec)).status,200);assert.equal(calls,1);
  const response=JSON.parse(readFileSync(join(f.d,'response-1.json')));
  assert(Buffer.from(response.raw_base64,'base64').toString().includes('9007199254740993'));
  await assert.rejects(f.p.request(spec));assert.equal(calls,1);
  const reopened=new Paper(f.d,()=>{throw Error('no retry');},Date.now,()=> 'fixture-source');
  await assert.rejects(reopened.request({...spec,id:'replacement-faucet'}),/one faucet/);
});
test('paper unknown response is retained and no resend, alternate origin or arbitrary action is supplied',async t=>{
  let calls=0;const f=fixture(t,async()=>{calls++;throw Error('lost reply');});
  const s={id:'original',category:'account',kind:'fullAccount'};
  assert.equal((await f.p.request(s)).status,null);assert.equal(calls,1);
  await assert.rejects(f.p.request(s));assert.equal(calls,1);
  for(const spec of [{category:'public',kind:'url',url:'https://mainnet-api1.bulk.trade'},
    {category:'signed',kind:'transfer'},{category:'signed',kind:'limit',role:'owner',price:'100',size:'0.01'},
    {category:'signed',kind:'cancel',original:'unknown'}])await assert.rejects(f.p.request({id:'bad',...spec}));
  assert.equal(calls,1);
});
test('paper refuses changed source and duplicate initialization before I/O',t=>{
  const f=fixture(t,()=>{throw Error('must not fetch');});
  assert.throws(()=>initialize(f.d,'not a new approval',()=> 'fixture-source'));
  assert.throws(()=>new Paper(f.d,fetch,Date.now,()=> 'different-source'),/changed/);
});
