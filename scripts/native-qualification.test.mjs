import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, chmodSync, writeFileSync, symlinkSync,readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { LIMITS, STAGED_LIMITS, DELIVERY_LIMITS, deliveryPolicy, faucetRecipient, canonical, sha, validateManifest, assertApproval, Journal, atoms, nativeAck,
  setupObservations, stagedInitialization, stagedIdle, stagedDepositHistory, transfers, emptyBalanceBaseline, flatAccountObservation, depositLink, withdrawalLink, paymentDelta, tokenDelta, boundedBody, privateRead, exclusive, originalNativeReply } from '../tools/native-qualification/core.mjs';
import { qualify,qualifyStaged } from '../tools/native-qualification/run.mjs';
import { configuredRpc,configuredWallet,validateSignerLocator } from '../tools/native-qualification/artifacts.mjs';
import { targets,observe,shape,body as observationBody,BOUNDS } from '../tools/native-qualification/observe.mjs';
import {eligible as followupEligible,BOUNDS as FOLLOWUP_BOUNDS} from '../tools/native-qualification/finish-staging.mjs';

test('schema observation is fixed testnet GET-only with bounded first-page histories',()=>{
  const rows=targets('1'.repeat(32),'2'.repeat(32));assert.equal(rows.length,BOUNDS.requests);
  for(const row of rows){const u=new URL(row.url);assert.equal(u.origin,'https://test-api.pacifica.fi');
    assert.equal(u.searchParams.get('account'),row.account);assert.equal(u.pathname,'/api/v1/'+row.path);
    if(row.path.endsWith('history'))assert.equal(u.searchParams.get('limit'),'32');
    if(row.path.includes('balance/history'))assert.equal(u.searchParams.get('include_trades'),'true');}
  for(const values of [['1'.repeat(32),'1'.repeat(32)],['https://evil','2'.repeat(32)],['','2'.repeat(32)]])
    assert.throws(()=>targets(...values));
});
test('schema observation reports types without leaking values, enforces streaming/nesting bounds',async()=>{
  const secret='DO_NOT_LOG';assert.equal(JSON.stringify(shape({error:secret,data:[{transaction_id:secret}]})).includes(secret),false);
  assert.throws(()=>shape({'bad\nfield':1}));
  let deep={};for(let i=0;i<10;i++)deep={data:deep};assert.throws(()=>shape(deep),/nesting/);
  await assert.rejects(observationBody(new Response('x'.repeat(BOUNDS.body_bytes+1))),/size bound/);
  assert.equal((await observationBody(new Response('ok'))).toString(),'ok');
});
function schemaPorts(status=200,raw='{"success":true,"data":[],"has_more":false}') {
  let at=1000;const retained=[],calls=[],reports=[];
  return {retained,calls,reports,ports:{now:()=>at,wait:async ms=>{at+=ms;},
    retain:(name,value)=>retained.push({name,value}),report:v=>reports.push(v),
    fetch:async (url,options)=>{calls.push({url,options,at});return new Response(raw,{status});}}};
}
test('read-only schema qualification retains before GET, observes fixed cadence and never grants readiness',async()=>{
  const h=schemaPorts();assert.equal(await observe('1'.repeat(32),'2'.repeat(32),h.ports),12);
  assert.equal(h.calls.length,12);assert.equal(h.reports.length,12);
  for(let i=0;i<12;i++){assert.equal(h.retained[i*3].name,'request-'+(i+1));
    assert.equal(h.calls[i].options.method,'GET');assert.equal(h.calls[i].options.redirect,'error');
    assert.deepEqual(h.calls[i].options.headers,{accept:'application/json'});
    if(i)assert.equal(h.calls[i].at-h.calls[i-1].at,12000);}
  assert.equal(h.retained.at(-1).value.shipping_readiness,false);
});
test('schema qualification stops on rate limit, upstream/transport/malformed response; no retries',async()=>{
  for(const [status,raw] of [[429,'{}'],[503,'{}'],[200,'bad-json'],[200,'x'.repeat(8193)]]){
    const h=schemaPorts(status,raw);await assert.rejects(observe('1'.repeat(32),'2'.repeat(32),h.ports));
    assert.equal(h.calls.length,1);assert.equal(h.retained.some(v=>v.name==='complete'),false);
  }
  const h=schemaPorts();h.ports.fetch=async()=>{h.calls.push({});throw Error('Network unknown');};
  await assert.rejects(observe('1'.repeat(32),'2'.repeat(32),h.ports));assert.equal(h.calls.length,1);
  assert.deepEqual(h.retained.map(v=>v.name),['request-1']);
});
test('schema observation bounds elapsed/backward time and refuses I/O after retention failure',async()=>{
  const h=schemaPorts();h.ports.wait=async()=>{};h.ports.now=(()=>{let n=0;return ()=>++n===1?1000:200000;})();
  await assert.rejects(observe('1'.repeat(32),'2'.repeat(32),h.ports),/bound/);assert.equal(h.calls.length,0);
  const j=schemaPorts();j.ports.retain=()=>{throw Error('Disk unavailable');};
  await assert.rejects(observe('1'.repeat(32),'2'.repeat(32),j.ports));assert.equal(j.calls.length,0);
});
const manifest=()=>({schema:'cinder-native-qualification-v1',cluster:'devnet',aws:false,shipping:false,
  native_origin:'https://test-api.pacifica.fi',wss_origin:'wss://test-ws.pacifica.fi/ws',
  genesis:'EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG',rpc_origin:'https://devnet.helius-rpc.com',
  limits:{...LIMITS},bootstrap_exception:true,lost_native_reply:true,withdraw_uuid:'12345678-1234-4234-8234-123456789abc',
  sources:'a'.repeat(64),tls_roots:'b'.repeat(64),node:'24.21.0',owner:'owner',broker:'broker',sponsor:'sponsor'});
test('persistent diagnostic seals bounded original-wire forwarding and distinct faucet control routes',()=>{
  const old=manifest();assert.deepEqual(deliveryPolicy(old),{maxRetries:0,polls:30,pollMs:2000});
  const staged={...old,schema:'cinder-native-staged-bootstrap-v1',limits:{...STAGED_LIMITS},
    bootstrap_exception:false,lost_native_reply:false,test_capital_only:true};
  assert.deepEqual(deliveryPolicy(staged),deliveryPolicy(old));
  for(const mode of ['owner-via-broker','broker-direct']) {
    const m={...staged,schema:'cinder-native-staged-delivery-v1',limits:{...DELIVERY_LIMITS},faucet_mode:mode};
    const approval={manifest:validateManifest(m),approved:true,reference:'latest user persistent diagnostics',not_after:2000};
    assert.deepEqual(deliveryPolicy(m),{maxRetries:5,polls:60,pollMs:2000});
    assert.equal(faucetRecipient(m),mode==='broker-direct'?'broker':'owner');
    assert.doesNotThrow(()=>assertApproval(m,approval,1000));
    assert.throws(()=>assertApproval({...m,faucet_mode:mode==='broker-direct'?'owner-via-broker':'broker-direct'},approval,1000));
    for(const change of [{limits:{...DELIVERY_LIMITS,rpc_max_retries:6}},{limits:{...DELIVERY_LIMITS,status_polls:61}},
      {limits:{...DELIVERY_LIMITS,initialization_cycles:13}},{faucet_mode:'any-wallet'},{aws:true},{shipping:true}])
      assert.throws(()=>validateManifest({...m,...change}));
  }
  assert.throws(()=>validateManifest({...staged,faucet_mode:'broker-direct'}));
});
function temp(fn){const dir=mkdtempSync(join(tmpdir(),'cinder-native-offline-'));try{return fn(dir);}finally{rmSync(dir,{recursive:true,force:true});}}
test('native offline sponsor locator strips CLI display padding without reading key material',()=>{
  assert.equal(configuredWallet('Config File: other\nKeypair Path: /private/tmp/test wallet.json \nCommitment: confirmed\n'),'/private/tmp/test wallet.json');
  assert.equal(configuredWallet('Keypair Path:\t/private/tmp/test.json \r\n'),'/private/tmp/test.json');
  for(const text of ['No keypair','Keypair Path: relative.json','Keypair Path: \n','Keypair Path: /a\nKeypair Path: /b','Keypair Path: /a\u0000b'])assert.throws(()=>configuredWallet(text));
});
test('native signer metadata must pass before a run can start; sealed locators are not trimmed',()=>temp(dir=>{
  const file=join(dir,'signer.json');exclusive(file,'not-read-as-a-key');
  assert.doesNotThrow(()=>validateSignerLocator(file));
  for(const path of [file+' ',join(dir,'missing'),'relative.json',dir])assert.throws(()=>validateSignerLocator(path));
  const link=join(dir,'link');symlinkSync(file,link);assert.throws(()=>validateSignerLocator(link));
  chmodSync(file,0o644);assert.throws(()=>validateSignerLocator(file));
}));
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
test('native documented transfer object is retained rather than silently discarded; malformed envelopes refuse',()=>{
  assert.deepEqual(transfers({channel:'account_transfers',data:row},account),[row]);
  assert.deepEqual(transfers({channel:'other',data:row},account),[]);
  for(const data of [null,undefined,'bad',42,[null],[[row]]])assert.throws(()=>transfers({channel:'account_transfers',data},account));
});
test('native baseline uses an explicit empty balance page, not missing endpoints or a financial frontier',()=>{
  const account={status:404,body:{success:false}},history={status:200,body:{success:true,data:[],has_more:false}};
  assert.deepEqual(emptyBalanceBaseline(account,history),{fresh:true,empty_balance_page_observed:true,source_cut:null});
  for(const h of [{status:404,body:{success:false}}, {...history,body:{success:true,data:[]}}, {...history,body:{...history.body,has_more:true}}, {...history,body:{...history.body,data:[{}]}}])assert.throws(()=>emptyBalanceBaseline(account,h));
  assert.throws(()=>emptyBalanceBaseline({status:200,body:{success:true}},history));
  const flat={status:200,body:{success:true,data:{balance:'0',pending_balance:'0'}}};
  assert.equal(flatAccountObservation(flat),true);assert.equal(flatAccountObservation({...flat,body:{success:true,data:{balance:'0',pending_balance:'1'}}}),false);
  assert.equal(flatAccountObservation({status:404,body:{success:false}}),false);
  assert.throws(()=>flatAccountObservation({...flat,body:{success:true,data:{balance:'0'}}}));
});
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
test('original native transport/body/rate-limit/audit failures retain exposure and cannot resend',async()=>{
  for(const stage of ['transport','body','rate-limit','audit']) {
    const dir=mkdtempSync(join(tmpdir(),'cinder-native-unknown-'));
    try {
      chmodSync(dir,0o700);
      const m=manifest(),j=new Journal(dir,m,()=>1000);j.begin();let requests=0;
      const receive=async()=>{requests++;if(stage!=='audit')throw Error(stage);return {status:200,body:{success:true}};};
      await assert.rejects(originalNativeReply(j,'original',{type:'withdraw'},receive,()=>{throw Error('audit');}),new RegExp(stage));
      const unknown=j.rows.filter(r=>r.kind==='native-reply-unknown');
      assert.equal(unknown.length,1);assert.deepEqual(unknown[0].data,{identity:'original',exposed:true,controller_accepted:false});
      assert.equal(j.rows.some(r=>r.kind==='native-reply'),false);
      await assert.rejects(originalNativeReply(j,'original',{},receive,()=>{}),/no resend/);
      assert.equal(requests,1);
    } finally {rmSync(dir,{recursive:true,force:true});}
  }
});
test('original native response withholding is distinct from genuine unknown reply',async()=>{
  const dir=mkdtempSync(join(tmpdir(),'cinder-native-withheld-'));
  try {
    chmodSync(dir,0o700);const j=new Journal(dir,manifest(),()=>1000);j.begin();let retained=false;
    assert.equal(await originalNativeReply(j,'original',{},async()=>({status:200,body:{success:true}}),()=>{retained=true;},true),null);
    assert.equal(retained,true);assert.equal(j.rows.at(-1).kind,'lost-native-reply');
    assert.equal(j.rows.some(r=>r.kind==='native-reply-unknown'),false);
  } finally {rmSync(dir,{recursive:true,force:true});}
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
test('staged manifest is a distinct sealed scenario, not the old timing exception or reply-loss authority',()=>{
  const m={...manifest(),schema:'cinder-native-staged-bootstrap-v1',limits:{...STAGED_LIMITS},
    bootstrap_exception:false,lost_native_reply:false,test_capital_only:true};
  const approval={approved:true,manifest:validateManifest(m),reference:'separate staged bootstrap approval',not_after:2000};
  assert.doesNotThrow(()=>assertApproval(m,approval,1000));
  for(const change of [{bootstrap_exception:true},{lost_native_reply:true},{test_capital_only:false},
    {limits:{...STAGED_LIMITS,bootstrap_ms:300001}},{limits:{...STAGED_LIMITS,initialization_cycles:5}}])
    assert.throws(()=>assertApproval({...m,...change},approval,1000));
  assert.throws(()=>assertApproval(manifest(),approval,1000));
  assert.throws(()=>validateManifest({...manifest(),test_capital_only:true}),/Mixed/);
});
test('private RPC locator imports only fixed devnet transport, never old account/approval authority',()=>{
  const url='https://devnet.helius-rpc.com/?api-key=TEST_ONLY';
  assert.equal(configuredRpc(Buffer.from(url+'\n')),url);
  assert.equal(configuredRpc(Buffer.from(JSON.stringify({rpc:url,wallet:'/old/ignored.json'}))),url);
  for(const value of ['https://api.mainnet-beta.solana.com',url+'#fragment','https://user@devnet.helius-rpc.com/',
    'https://devnet.helius-rpc.com/other',JSON.stringify({rpc:url,wallet:'/old',approved:true}),JSON.stringify({rpc:42,wallet:'/old'})])
    assert.throws(()=>configuredRpc(Buffer.from(value)));
});
function idleResponses() {
  const r=data=>({status:200,body:{success:true,data,error:null,code:null}});
  return {settings:r({auto_lend_disabled:true,margin_settings:[],spot_settings:[]}),
    loan:r({borrowed:'0',pending_interest:'0',spot_balances:[],updated_at:1000}),
    account:r({balance:'20',account_equity:'20',pending_balance:'0',pending_interest:'0',total_margin_used:'0',
      spot_market_value:'0',spot_collateral:'0',positions_count:0,orders_count:0,stop_orders_count:0,spot_balances:[]}),positions:r([]),orders:r([])};
}
test('staged initialization waits for an explicitly absent loan cache, never interprets it as zero debt',()=>{
  const x=idleResponses(),missing={status:404,body:{success:false,data:null,code:404}};
  assert.equal(stagedInitialization(x.account,missing,1000),false);
  assert.equal(stagedInitialization(missing,missing,1000),false);
  assert.equal(stagedInitialization(x.account,x.loan,1000),true);
  for(const bad of [{status:500,body:{success:false}},{status:200,body:{success:true,data:null}},
    {...missing,body:{success:false}}, {...x.loan,body:{...x.loan.body,code:404}}])
    assert.throws(()=>stagedInitialization(x.account,bad,1000));
});
test('staged setup refuses exposures, debt, pending cash, stale/future data and incomplete cross settings',()=>{
  const check=x=>stagedIdle(x.settings,x.loan,x.account,x.positions,x.orders,1000);
  assert.equal(check(idleResponses()).shipping_complete,false);
  for(const [section,key,value] of [['account','balance','19'],['account','account_equity','19'],['account','pending_balance','1'],
    ['account','positions_count',1],['account','orders_count',1],['account','stop_orders_count',1],['account','total_margin_used','1'],
    ['account','spot_market_value','1'],['account','spot_collateral','1'],
    ['account','spot_balances',[{}]],['loan','borrowed','1'],['loan','pending_interest','1'],['loan','spot_balances',[{}]],
    ['loan','updated_at',0],['loan','updated_at',6001],['settings','auto_lend_disabled',null],
    ['settings','margin_settings',[{}]],['settings','spot_settings',[{}]]]) {
    const x=idleResponses();x[section].body.data[key]=value;assert.throws(()=>check(x));
  }
  const x=idleResponses();assert.throws(()=>stagedIdle(x.settings,x.loan,x.account,x.positions,x.orders,181001));
  x.orders.body.data=[{}];assert.throws(()=>check(x));
});
test('staged empty baseline requires positively observed zero cash and disabled/no-debt state',()=>{
  const x=idleResponses();x.account.body.data.balance='0';x.account.body.data.account_equity='0';
  assert.equal(stagedIdle(x.settings,x.loan,x.account,x.positions,x.orders,1000,'0').status,'observed-disabled-no-debt');
  assert.throws(()=>stagedIdle(x.settings,x.loan,x.account,x.positions,x.orders,1000));
  x.loan.body.data.borrowed='1';assert.throws(()=>stagedIdle(x.settings,x.loan,x.account,x.positions,x.orders,1000,'0'));
});
test('staged deposit history requires original full signature credit and latest nonpending balance, not a certificate',()=>{
  const d={status:200,body:{success:true,data:[{transaction_id:signature,amount:'20',created_at:1000}],has_more:false}},
    b={status:200,body:{success:true,data:[{event_type:'deposit',amount:'20',balance:'20',pending_balance:'0',created_at:1100}],has_more:false}};
  const result=stagedDepositHistory(d,b,signature);assert.equal(result.financial_credit,false);assert.equal(result.source_cut,null);
  for(const [section,key,value] of [['deposit','transaction_id','different'],['deposit','amount','19'],['deposit','created_at',0],
    ['balance','event_type','withdraw'],['balance','balance','0'],['balance','amount','19'],['balance','pending_balance','20'],['balance','created_at',999]]) {
    const dd=structuredClone(d),bb=structuredClone(b);(section==='deposit'?dd:bb).body.data[0][key]=value;
    assert.throws(()=>stagedDepositHistory(dd,bb,signature));
  }
  for(const change of [{has_more:true},{next_cursor:'another'},{data:[...d.body.data,...d.body.data]},{code:404}])
    assert.throws(()=>stagedDepositHistory({...d,body:{...d.body,...change}},b,signature));
});
function stagedHarness(overrides={}) {
  const h=harness();h.p.stageSetup=async()=>{h.calls.push('staged-setup');return {status:'observed-disabled-no-debt'};};
  h.p.withdrawRetained=async()=>{h.calls.push('original-withdrawal-retained');return {status:200,body:{success:true}};};
  h.p.closeout=async state=>{h.calls.push('closeout');assert.equal(state.controller_ack.status,200);return state;};
  return {calls:h.calls,p:{...h.p,...overrides}};
}
test('staged sequence initializes with test capital and retains the original ACK without legacy settings/loss injection',async()=>{
  const h=stagedHarness();await qualifyStaged(h.p);
  assert.deepEqual(h.calls,['preflight','baseline','fund','original-deposit','staged-setup','bootstrap-checked',
    'original-withdrawal-retained','returnNet','closeout']);
});
test('staged setup failure only allows the planned original withdrawal containment',async()=>{
  const h=stagedHarness({stageSetup:async()=>{throw Error('missing cache');}}),r=await qualifyStaged(h.p);
  assert.equal(r.setup.status,'unresolved');assert(h.calls.includes('staged-setup-blocker'));
  assert.equal(h.calls.filter(x=>x==='original-withdrawal-retained').length,1);
});
test('staged uncertain deposit, withdrawal ACK or payment stops without resend, return or rescue',async()=>{
  for(const key of ['deposit','withdrawRetained','reconcilePayment']) {
    const h=stagedHarness({[key]:async()=>{throw Error('original unknown');}});
    await assert.rejects(qualifyStaged(h.p),/unknown/);assert(!h.calls.includes('returnNet'));assert(!h.calls.includes('closeout'));
    assert(h.calls.filter(x=>x==='original-withdrawal-retained').length<=1);
  }
});
test('actual account schema uses account_equity, not a made-up equity alias, and includes stop-order exposure',()=>{
  const fixture=JSON.parse(readFileSync(new URL('../crates/pacifica/tests/fixtures/demo-native-shapes.json',import.meta.url))),x=idleResponses();
  x.account.body.data=fixture.account.data;
  assert.equal(stagedInitialization(x.account,x.loan,1000),true);
  const old=structuredClone(x.account);old.body.data.equity=old.body.data.account_equity;delete old.body.data.account_equity;
  assert.throws(()=>stagedInitialization(old,x.loan,1000));
  const missing=structuredClone(x.account);delete missing.body.data.stop_orders_count;
  assert.throws(()=>stagedInitialization(missing,x.loan,1000));
});
test('settings continuation requires a closed returned staged run and no earlier setting exposure',()=>{
  const m={...manifest(),schema:'cinder-native-staged-bootstrap-v1',limits:{...STAGED_LIMITS},
    bootstrap_exception:false,lost_native_reply:false,test_capital_only:true},
    result={owner_atoms:'19000000',broker_atoms:'0',payment:{net:'19000000'},pending_balance_observed:'0',source_cut:null,shipping:false,financial_completion:false},
    rows=[{kind:'start',at:1000},{kind:'http',at:2000},{kind:'closed',at:3000}],now=4000;
  assert.deepEqual(followupEligible(m,result,rows,now),{deadline:1201000,parent_http:1,last_http:2000});
  for(const change of [{broker_atoms:'1'},{owner_atoms:'20000000'},{pending_balance_observed:'1'},{shipping:true},{financial_completion:true}])
    assert.throws(()=>followupEligible(m,{...result,...change},rows,now));
  assert.throws(()=>followupEligible(m,result,[...rows,{kind:'native-intent',data:{type:'set_auto_lend_disabled'}},{kind:'closed'}],now));
  assert.throws(()=>followupEligible(m,result,rows.slice(0,-1),now));
  assert.throws(()=>followupEligible(manifest(),result,rows,now));
  assert.throws(()=>followupEligible(m,result,rows,1200000));
  assert.throws(()=>followupEligible(m,result,rows,999));
  const spent=[rows[0],...Array.from({length:191},()=>({kind:'http',at:2000})),rows[2]];
  assert.throws(()=>followupEligible(m,result,spent,now));assert.equal(FOLLOWUP_BOUNDS.posts,1);
});
