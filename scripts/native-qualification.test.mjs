import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, chmodSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { LIMITS, canonical, sha, validateManifest, assertApproval, Journal, atoms, nativeAck,
  setupObservations, transfers, depositLink, withdrawalLink, paymentDelta, tokenDelta, boundedBody, privateRead, exclusive } from '../tools/native-qualification/core.mjs';
import { qualify } from '../tools/native-qualification/run.mjs';
const manifest=()=>({schema:'cinder-native-qualification-v1',cluster:'devnet',aws:false,shipping:false,
  native_origin:'https://test-api.pacifica.fi',wss_origin:'wss://test-ws.pacifica.fi/ws',
  genesis:'EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG',rpc_origin:'https://devnet.helius-rpc.com',
  limits:{...LIMITS},bootstrap_exception:true,lost_native_reply:true,withdraw_uuid:'12345678-1234-4234-8234-123456789abc',
  sources:'a'.repeat(64),tls_roots:'b'.repeat(64),node:'24.21.0',owner:'owner',broker:'broker',sponsor:'sponsor'});
function temp(fn){const dir=mkdtempSync(join(tmpdir(),'cinder-native-offline-'));try{return fn(dir);}finally{rmSync(dir,{recursive:true,force:true});}}
test('native qualification requires exact current approval, not prior scope or changed budgets',()=>{
  const m=manifest(), approval={approved:true,manifest:validateManifest(m),reference:'specific review',not_after:2000};
  assert.doesNotThrow(()=>assertApproval(m,approval,1000));
  for(const change of [{aws:true},{cluster:'mainnet'},{native_origin:'https://api.pacifica.fi'},
    {limits:{...LIMITS,http:201}},{sources:'c'.repeat(64)},{broker:'owner'}])assert.throws(()=>assertApproval({...m,...change},approval,1000));
  assert.throws(()=>assertApproval(m,approval,2000));assert.throws(()=>assertApproval(m,{...approval,not_after:90000000},1000));
});
test('native journal replays consumed budgets and original identity without permitting restart or resend',()=>temp(dir=>{
  let time=1000;const m=manifest(),j=new Journal(dir,m,()=>time);j.begin();j.consume('http');j.once('original',{wire_hash:sha('wire')});
  const replay=new Journal(dir,m,()=>time);assert.equal(replay.head,j.head);
  assert.throws(()=>replay.begin(),/already started/);assert.throws(()=>replay.once('original',{}),/no resend/);
  time+=LIMITS.duration_ms;assert.throws(()=>replay.consume('rpc'),/deadline/);
}));
test('native budgets reserve cleanup capacity and refuse backward clocks',()=>temp(dir=>{
  let time=1000;const j=new Journal(dir,manifest(),()=>time);j.begin();
  for(let i=0;i<6;i++)j.consume('wss');assert.throws(()=>j.consume('wss'),/budget/);
  j.consume('wss',{},true);j.consume('wss',{},true);assert.throws(()=>j.consume('wss',{},true),/budget/);
  time=999;assert.throws(()=>j.append('anything',{}),/backwards/);
}));
test('native private artifacts reject permissive files, overwrite, altered and torn journal',()=>temp(dir=>{
  const m=manifest(),j=new Journal(dir,m,()=>1000);j.begin();
  const f=join(dir,'private');exclusive(f,{x:1});assert.throws(()=>exclusive(f,{x:2}));
  chmodSync(f,0o644);assert.throws(()=>privateRead(f),/permissions/);
  writeFileSync(j.file,'{}',{mode:0o600});assert.throws(()=>new Journal(dir,m,()=>1000),/Torn/);
  writeFileSync(j.file,canonical({manifest:validateManifest(m),index:0,at:1000,previous:'0'.repeat(64),kind:'start',data:{},hash:'0'.repeat(64)})+'\n');
  assert.throws(()=>new Journal(dir,m,()=>1000),/integrity/);
}));
test('native amounts are exact and setup missing cache/defaults never imply readiness',()=>{
  assert.equal(atoms('20.000000'),20000000n);for(const s of [20,'1e1','-1','00','0.0000001'])assert.throws(()=>atoms(s));
  const loan={success:true,data:{borrowed:'0',pending_interest:'0',updated_at:1000}},settings={success:true,data:{auto_lend_disabled:true}};
  assert.equal(setupObservations(settings,loan).status,'observed-disabled-no-debt');
  assert.equal(setupObservations(settings,loan).shipping_complete,false);
  for(const x of [null,{success:false}, {success:true,data:{auto_lend_disabled:null}}])assert.equal(setupObservations(x,loan).status,'unresolved');
  assert.equal(setupObservations(settings,{success:false,error:'cache missing'}).status,'unresolved');
});
const signature='5'.repeat(88),account='broker';
const row={u:account,a:'USDC',e:'deposit',am:'20',t:1000,tx:signature};
test('native deposit joins only its original signature; duplicates do not create final credit',()=>{
  const rows=transfers({channel:'account_transfers',data:[row,row]},account);
  assert.equal(depositLink(rows,signature,account).status,'exact-signature-observed');
  assert.equal(depositLink(rows,signature,account).financial_credit,false);
  assert.equal(depositLink([{...row,tx:'6'.repeat(88)}],signature,account).status,'unresolved');
  assert.throws(()=>depositLink([row,{...row,t:1001}],signature,account),/conflict/);
  assert.throws(()=>transfers({channel:'account_transfers',data:[{...row,u:'other'}]},account),/account/);
});
const ack=()=>nativeAck({success:true,data:{batch_nonce:7,requested_amount:'20',fee_amount:'1'}});
test('native lost reply audit linkage is not controller ACK, UUID reconciliation, or complete native cut',()=>{
  const rows=[{...row,e:'withdrawal_confirmed',am:'19',bn:7,ra:'20',f:'1'}];
  const result=withdrawalLink(rows,ack(),null,account);
  assert.equal(result.net,'19000000');assert.equal(result.source_cut,null);assert.equal(result.financial_completion,false);
  assert.match(result.lost_reply_reconciliation,/unresolved/);
  assert.equal(withdrawalLink([{...rows[0],bn:8}],ack(),null,account).status,'unresolved');
  assert.throws(()=>withdrawalLink([{...rows[0],am:'20'}],ack(),null,account),/conservation/);
  for(const bad of [{success:false},{success:true,data:{batch_nonce:'7',requested_amount:'20',fee_amount:'1'}},
    {success:true,data:{batch_nonce:7,requested_amount:'20',fee_amount:'3'}}])assert.throws(()=>nativeAck(bad));
});
test('native payment evidence requires exact finalized signature, ATA, mint, owner and integer token delta',()=>{
  const status={confirmationStatus:'finalized',err:null},balance={accountIndex:0,mint:'mint',owner:'broker',uiTokenAmount:{decimals:6,amount:'19000000'}};
  const tx={slot:10,transaction:{signatures:[signature],message:{accountKeys:[{pubkey:'ata'}]}},meta:{err:null,preTokenBalances:[],postTokenBalances:[balance]}};
  const binding={signature,ata:'ata',mint:'mint',owner:'broker'};
  assert.equal(paymentDelta(tx,status,binding),19000000n);
  assert.throws(()=>paymentDelta(tx,{...status,confirmationStatus:'confirmed'},binding));
  for(const field of ['signature','ata','mint','owner'])assert.throws(()=>paymentDelta(tx,status,{...binding,[field]:'wrong'}));
  assert.throws(()=>paymentDelta({...tx,meta:{...tx.meta,postTokenBalances:[balance,balance]}},status,binding));
});
test('native original deposit effect uses exact signed broker debit and distinct venue-vault credit',()=>{
  const status={confirmationStatus:'finalized',err:null},balance=(index,owner,amount)=>({accountIndex:index,mint:'mint',owner,uiTokenAmount:{decimals:6,amount}});
  const tx={slot:10,transaction:{signatures:[signature],message:{accountKeys:['broker-ata','venue-ata']}},
    meta:{err:null,preTokenBalances:[balance(0,'broker','20000000'),balance(1,'central','100000000')],
      postTokenBalances:[balance(0,'broker','0'),balance(1,'central','120000000')]}};
  const binding={signature,mint:'mint'};
  assert.equal(tokenDelta(tx,status,{...binding,ata:'broker-ata',owner:'broker'}),-20000000n);
  assert.equal(tokenDelta(tx,status,{...binding,ata:'venue-ata',owner:'central'}),20000000n);
  assert.throws(()=>paymentDelta(tx,status,{...binding,ata:'broker-ata',owner:'broker'}),/delta/);
  assert.throws(()=>tokenDelta(tx,status,{...binding,ata:'venue-ata',owner:'broker'}),/asset\/owner/);
});
test('native HTTP body bound is enforced while reading, not after buffering an unbounded response',async()=>{
  const response=new Response(new ReadableStream({start(c){c.enqueue(new Uint8Array(LIMITS.payload_bytes));c.enqueue(new Uint8Array(1));c.close();}}));
  await assert.rejects(boundedBody(response),/bound/);
  assert.equal((await boundedBody(new Response('ok'))).toString(),'ok');
});
function harness(overrides={}) {
  const calls=[],p={};
  for(const name of ['preflight','baseline','fund','returnNet'])p[name]=async()=>{calls.push(name);};
  p.disable=async stage=>{calls.push(`disable-${stage}`);return stage==='predeposit'?'rejected':'disabled';};
  p.deposit=async()=>{calls.push('original-deposit');return {signature};};
  p.capture=action=>action();p.setup=async()=>({status:'observed-disabled-no-debt',shipping_complete:false});
  p.checkBootstrap=()=>calls.push('bootstrap-checked');p.note=x=>calls.push(x);
  p.withdrawLostReply=async()=>{calls.push('original-withdrawal-response-withheld');return null;};
  p.reconcilePayment=async()=>({net:'19000000',financial_completion:false});
  p.closeout=async state=>{calls.push('closeout');assert.equal(state.controller_ack,null);return state;};
  return {p:{...p,...overrides},calls};
}
test('native sequence injects loss before controller ACK and never resends the economic action',async()=>{
  const {p,calls}=harness();const result=await qualify(p);
  assert.equal(result.payment.financial_completion,false);
  assert.deepEqual(calls,['preflight','baseline','disable-predeposit','fund','original-deposit','disable-postdeposit',
    'bootstrap-checked','original-withdrawal-response-withheld','returnNet','closeout']);
});
test('native sequence unknown setup before funding stops; expired bootstrap uses withdrawal-only containment',async()=>{
  const unknown=harness({disable:async()=> 'unknown'});await assert.rejects(qualify(unknown.p),/Unknown/);
  assert(!unknown.calls.includes('fund'));
  const expired=harness({checkBootstrap:()=>{throw Error('expired');}}),r=await qualify(expired.p);
  assert.equal(r.setup.status,'unresolved');assert(expired.calls.includes('setup-blocker'));assert(expired.calls.includes('returnNet'));
  assert.equal(expired.calls.filter(x=>x==='original-withdrawal-response-withheld').length,1);
});
test('native sequence uncertain original deposit or missing original payment cannot trigger return/rescue',async()=>{
  const deposit=harness({deposit:async()=>{throw Error('uncertain original');}});await assert.rejects(qualify(deposit.p),/uncertain/);
  assert(!deposit.calls.includes('original-withdrawal-response-withheld'));
  const withdrawal=harness({reconcilePayment:async()=>null});await assert.rejects(qualify(withdrawal.p),/unresolved/);
  assert(!withdrawal.calls.includes('returnNet'));assert(!withdrawal.calls.includes('closeout'));
});
