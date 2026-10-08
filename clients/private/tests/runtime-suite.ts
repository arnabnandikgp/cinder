// C4 same executable suite in actual Node AND Chrome; explicit synthetic ports.
import { PrivateClient,type MessageSigner,type Command,type ReadPage } from '../src/index.ts';
import { WebChannel,type WebCore } from '../src/web-channel-core.ts';
import { verifyWithRoot } from '../../../tools/web-channel/attestation/verifier.mjs';
import { expectStreamClosed } from './stream-assertions.ts';
import { id,policy } from './web-suite.ts';
type Metrics={gets:number;puts:number;replica_bytes:number;writer_reads:number;accepts:number;read_calls:number;live_read_io:number;peak_read_io:number;clocks:number;post_cas:boolean;head:number};
export interface RuntimeHarness {
  setup(options:{history:number;delay_ms:number;packed?:boolean}):Promise<void>;
  current():{baseUrl:string;root:number[]};
  control(command:Record<string,unknown>):Promise<Metrics>;
  restart():Promise<void>;
  finish():Promise<{peakRssBytes:number}>;
  record(receipt:Record<string,unknown>):void;
}

// Candidate-only actual carrier growth. Never selects the shipping cloud store.
export async function runPackedGrowth(core:WebCore,signer:MessageSigner,h:RuntimeHarness):Promise<string[]> {
  const checks:string[]=[],channels:WebChannel[]=[];
  const connect=async(transport:'http'|'websocket')=>{
    const ch=await WebChannel.connect({baseUrl:h.current().baseUrl,policy,core,transport},
      (q,p,c)=>verifyWithRoot(q,p,c,Uint8Array.from(h.current().root)));channels.push(ch);return ch;
  };
  const client=(ch:WebChannel,s=signer)=>new PrivateClient(ch,s,{domain:policy,account:id(1),policy:1});
  const req=(n:number,command:Command,epoch=1n)=>({id:id(n),epoch,expiresAt:BigInt(Date.now()+60000),command});
  const close=()=>{for(const ch of channels.splice(0))ch.close();};
  const metrics=()=>h.control({op:'metrics'});
  const next=async(it:AsyncIterator<ReadPage>)=>{
    const r=await bounded(it.next(),'Packed Runtime update missing');check(!r.done,'Packed Runtime stream ended');return r.value;
  };
  try {
    for(const carrier of ['http','websocket'] as const) {
      await h.setup({history:1,delay_ms:0,packed:true});
      await h.control({op:'replica-delay',milliseconds:100});
      const started=performance.now(),cuts:Record<string,unknown>[]=[],durations:number[]=[];
      let agent:MessageSigner|undefined,grant:Command|undefined;
      for(let head=2;head<=64;head++) {
        const keys=await crypto.subtle.generateKey('Ed25519',false,['sign','verify']);
        agent={publicKey:new Uint8Array(await crypto.subtle.exportKey('raw',keys.publicKey)),
          signMessage:async b=>new Uint8Array(await crypto.subtle.sign('Ed25519',keys.privateKey,Uint8Array.from(b).buffer))};
        grant={kind:'grant',grant:{key:agent.publicKey,methods:1,market:id(7),maximumLots:1n,maximumFee:1n,maximumOrders:1n,expiresAt:BigInt(Date.now()+3600000)}};
        const ch=await connect(carrier),before=await metrics(),t=performance.now();
        const reply=await client(ch).request(req(head-1,grant));
        const elapsedMs=performance.now()-t,after=await metrics();durations.push(elapsedMs);ch.close();channels.pop();
        check(reply.kind==='receipt'&&reply.outcome==='complete',`Packed grant at head ${head}: ${reply.kind}`);
        check(after.head===head&&after.accepts===before.accepts+1,'Packed growth was retried or skipped');
        check(after.gets-before.gets===3*Math.ceil(head/16)+2&&after.puts-before.puts===2,'Packed append I/O contract');
        check(elapsedMs<15000,'Packed grant exceeded unchanged reply budget');
        if([2,8,32,64].includes(head)) {
          const http=await connect('http'),ws=await connect('websocket'),readsBefore=await metrics();
          const replies=await Promise.all([client(http).request(req(100+head,{kind:'view'})),
            client(ws).request(req(101+head,{kind:'read',query:{family:'operations',limit:64}}))]);
          check(replies[0].kind==='view'&&replies[0].cash===0n&&replies[0].funding===0n&&replies[0].held===0n&&replies[0].positions.every(p=>p.lots===0n&&p.basis===0n)&&replies[1].kind==='page'&&replies[1].rows.length===head-1,'Packed cut read/financial state');
          const afterReads=await metrics();check(afterReads.gets===readsBefore.gets&&afterReads.writer_reads===readsBefore.writer_reads,'Packed read used writer storage');
          close();cuts.push({head,elapsedMs,gets:after.gets-before.gets,puts:after.puts-before.puts});
        }
      }
      check(agent&&grant&&grant.kind==='grant','Missing packed growth grant');
      await h.control({op:'writer-delay',milliseconds:50});
      await h.control({op:'delay',milliseconds:50});
      const delegated=client(await connect('websocket'),agent);
      check((await delegated.request(req(190,{kind:'view'}))).kind==='view','Packed READ grant missing');
      const watched=client(await connect('websocket'));
      const it=(await watched.subscribe({id:id(191),epoch:1n,expiresAt:BigInt(Date.now()+60000),query:{family:'operations',limit:64}}))[Symbol.asyncIterator]();
      check((await next(it)).rows.length===63,'Packed head64 watch missing operations');
      const secondWatch=client(await connect('websocket'));
      const secondIt=(await secondWatch.subscribe({id:id(192),epoch:1n,expiresAt:BigInt(Date.now()+60000),query:{family:'operations',limit:64}}))[Symbol.asyncIterator]();
      check((await next(secondIt)).rows.length===63,'Packed second watch missing operations');
      // A new key is required: granting an already authorized key is invalid,
      // not a mutation whose response can be deliberately abandoned after CAS.
      const newKeys=await crypto.subtle.generateKey('Ed25519',false,['sign','verify']);
      const lostGrant:Command={kind:'grant',grant:{...grant.grant,key:new Uint8Array(await crypto.subtle.exportKey('raw',newKeys.publicKey)),expiresAt:BigInt(Date.now()+3600000)}};
      const lostChannel=await connect(carrier),beforeLost=await metrics();
      await h.control({op:'post-cas',milliseconds:1000});
      const lost=client(lostChannel).request(req(200,lostGrant)).then(()=>false,()=>true);
      const until=performance.now()+10000;
      while(!(await metrics()).post_cas) {check(performance.now()<until,'Lost reply did not reach CAS');await new Promise(r=>setTimeout(r,20));}
      lostChannel.close();check(await lost,'Deliberately abandoned reply unexpectedly completed');
      const update=await next(it);check(update.rows.length===64,'Accepted lost reply not published');
      check((await next(secondIt)).rows.length===64,'Second watch missed accepted lost reply');
      const owner=client(await connect('http')),receipt=await owner.request(req(202,{kind:'operation',target:id(200)}));
      check(receipt.kind==='receipt'&&receipt.outcome==='complete','Original-ID packed reconciliation');
      const grown=await metrics();close();
      const growthMemory=await h.finish();await h.restart();
      check(grown.accepts===beforeLost.accepts+1&&grown.head===65,'Lost reply was resent');
      check(grown.gets-beforeLost.gets===17&&grown.puts-beforeLost.puts===2,'Head64 writer pack cost');
      const reopened=await metrics(),fresh=client(await connect('http'));
      const replayed=await fresh.request(req(203,{kind:'operation',target:id(200)}));
      check(replayed.kind==='receipt'&&replayed.outcome==='complete'&&reopened.head===65&&reopened.accepts===0,'Packed process reopen lost original receipt');
      check(replayed.id.every((b,n)=>b===receipt.id[n])&&replayed.digest.every((b,n)=>b===receipt.digest[n]),'Packed reopen changed original ID/digest');
      const operations=await fresh.request(req(204,{kind:'read',query:{family:'operations',limit:64}}));
      check(operations.kind==='page'&&operations.rows.length===64,'Packed reopen lost operation history');
      const old=client(await connect('websocket'),agent),oldIt=(await old.subscribe({id:id(205),epoch:1n,expiresAt:BigInt(Date.now()+60000),query:{family:'operations',limit:64}}))[Symbol.asyncIterator]();await next(oldIt);
      check((await fresh.request(req(206,{kind:'revoke'}))).kind==='receipt','Packed revoke failed');
      await expectStreamClosed(oldIt,'Packed revoke leaked old generation');
      check((await fresh.request(req(207,{kind:'view'},2n))).kind==='view','Packed current-epoch owner read');
      close();const replayMemory=await h.finish();
      const memory={peakRssBytes:Math.max(growthMemory.peakRssBytes,replayMemory.peakRssBytes)};
      check(memory.peakRssBytes>0&&memory.peakRssBytes<128*1024*1024&&grown.peak_read_io<=4,'Packed small-record Runtime resource envelope');
      h.record({kind:'packed-growth',carrier,replicaDelayMs:100,witnessDelayMs:50,cuts,growthMs:performance.now()-started,maxGrantMs:Math.max(...durations),acceptedHead:grown.head,metrics:grown,...memory});
      checks.push(`${carrier}: growth to64, independent reads/watch, original-ID lost-reply replay and revocation`);
    }
    return checks;
  }finally{close();}
}
function check(ok:unknown,label:string):asserts ok {if(!ok)throw Error(label);}
async function bounded<T>(promise:Promise<T>,label:string,milliseconds=15000):Promise<T> {
  let timer:ReturnType<typeof setTimeout>|undefined;
  try{return await Promise.race([promise,new Promise<never>((_,reject)=>{timer=setTimeout(()=>reject(Error(label)),milliseconds);})]);}
  finally{clearTimeout(timer);}
}
async function denied(work:()=>Promise<unknown>,label:string) {
  try{await work();}catch{return;}throw Error(label);
}
export async function runRuntime(core:WebCore,signer:MessageSigner,h:RuntimeHarness):Promise<string[]> {
  const checks:string[]=[],channels:WebChannel[]=[];
  const connect=async(transport:'http'|'websocket'='websocket')=>{
    const ch=await WebChannel.connect({baseUrl:h.current().baseUrl,policy,core,transport},
      (q,p,c)=>verifyWithRoot(q,p,c,Uint8Array.from(h.current().root)));channels.push(ch);return ch;
  };
  const client=(ch:WebChannel,s=signer)=>new PrivateClient(ch,s,{domain:policy,account:id(1),policy:1});
  const req=(n:number,command:Command,epoch=1n)=>({id:id(n),epoch,expiresAt:BigInt(Date.now()+60000),command});
  const watch=(n:number,expiresAt=BigInt(Date.now()+60000))=>({id:id(n),epoch:1n,expiresAt,query:{family:'operations' as const,limit:64}});
  const next=async(it:AsyncIterator<ReadPage>)=>{const result=await bounded(it.next(),'Runtime update missing');check(!result.done,'Runtime stream ended');return result.value;};
  const close=()=>{for(const ch of channels.splice(0))ch.close();};
  const metrics=()=>h.control({op:'metrics'});
  const waitFor=async(predicate:(m:Metrics)=>boolean)=>{
    const until=performance.now()+4000;
    do{const m=await metrics();if(predicate(m))return m;await new Promise(r=>setTimeout(r,20));}while(performance.now()<until);
    throw Error('Runtime instrumentation barrier timeout');
  };
  const keys=await crypto.subtle.generateKey('Ed25519',false,['sign','verify']);
  const agent:MessageSigner={publicKey:new Uint8Array(await crypto.subtle.exportKey('raw',keys.publicKey)),
    signMessage:async b=>new Uint8Array(await crypto.subtle.sign('Ed25519',keys.privateKey,Uint8Array.from(b).buffer))};
  const grant:Command={kind:'grant',grant:{key:agent.publicKey,methods:1,market:id(7),maximumLots:1n,maximumFee:1n,maximumOrders:1n,expiresAt:BigInt(Date.now()+3600000)}};
  try{
    for(const history of [1,8,32,64])for(const delay_ms of [0,50,400])for(const watches of [1,2]){
      await h.setup({history,delay_ms});
      const streams:AsyncIterator<ReadPage>[]=[],apis:PrivateClient[]=[];
      for(let n=0;n<watches;n++){
        const api=client(await connect());apis.push(api);
        const it=(await api.subscribe(watch(150+n)))[Symbol.asyncIterator]();
        check((await next(it)).rows.length===0,'Runtime initial history invented account operations');streams.push(it);
      }
      const a=client(await connect('http')),b=client(await connect()),before=await metrics(),started=performance.now();
      const replies=await Promise.all([a.request(req(160,{kind:'view'})),b.request(req(161,{kind:'read',query:{family:'account',limit:64}}))]);
      const parallelMs=performance.now()-started,afterReads=await metrics();
      check(replies[0].kind==='view'&&replies[0].cash===0n&&replies[1].kind==='page','Real Runtime parallel reads');
      check(afterReads.read_calls>=before.read_calls+2&&afterReads.gets===before.gets&&afterReads.writer_reads===before.writer_reads,'Reads used writer backend or cached witness');
      check(afterReads.peak_read_io<=4&&(delay_ms===0||afterReads.peak_read_io>=2),'Independent read I/O cap/concurrency');
      const ticksBefore=await metrics();await h.control({op:'tick'});const ticksAfter=await metrics();
      check(ticksAfter.writer_reads===ticksBefore.writer_reads&&ticksAfter.clocks>ticksBefore.clocks,'Inactive real Runtime tick performed cloud checks');
      if(delay_ms)await waitFor(m=>m.live_read_io>0);
      const startedMutation=performance.now(),mutation=await apis[0].request(req(162,grant));
      const mutationMs=performance.now()-startedMutation;
      check(mutation.kind==='receipt'&&mutation.outcome==='complete',`Same-socket grant during periodic preparation: ${mutation.kind}${mutation.kind==='error'?'/'+mutation.code:mutation.kind==='receipt'?'/'+mutation.outcome:''}`);
      for(const it of streams){const update=await next(it);check(update.rows.length===1,'Committed mutation missing from Runtime watch');}
      const final=await metrics();check(final.accepts===before.accepts+1,'Grant was retried or accepted more than once');
      close();const memory=await h.finish();check(memory.peakRssBytes>0&&memory.peakRssBytes<128*1024*1024,'Declared local Runtime memory envelope');
      h.record({kind:'matrix',history,delay_ms,watches,parallelMs,mutationMs,metrics:final,...memory});
    }
    checks.push('24 fixed history/delay/watch cells: independent reads, same-socket grant, tick and process peak RSS');

    await h.setup({history:8,delay_ms:400});await h.control({op:'writer-delay',milliseconds:0});
    const owner=client(await connect('http'));check((await owner.request(req(170,grant))).kind==='receipt','Runtime agent grant');
    const delegated=client(await connect(),agent),it=(await delegated.subscribe(watch(171)))[Symbol.asyncIterator]();await next(it);
    await waitFor(m=>m.live_read_io>0);
    check((await owner.request(req(172,{kind:'revoke'}))).kind==='receipt','Runtime revoke');
    await expectStreamClosed(it,'Runtime queued read escaped revocation');
    const reconnect=client(await connect());check((await reconnect.request(req(173,{kind:'view'},2n))).kind==='view','Runtime owner current-epoch reconnect');
    close();await h.finish();checks.push('Agent revoke during witness I/O discards old generation and current owner reconnects');

    await h.setup({history:8,delay_ms:400});
    const exp=client(await connect()),expIt=(await exp.subscribe(watch(174,BigInt(Date.now()+1300))))[Symbol.asyncIterator]();await next(expIt);
    await expectStreamClosed(expIt,'Runtime signed expiry during periodic I/O ignored');close();await h.finish();
    checks.push('Signed subscription expiry during periodic I/O is enforced at delivery');

    await h.setup({history:8,delay_ms:50});await h.control({op:'writer-delay',milliseconds:0});
    const writer=client(await connect()),reader=client(await connect('http'));
    await h.control({op:'post-cas',milliseconds:1000});
    const pending=writer.request(req(175,grant));await waitFor(m=>m.post_cas);
    const raced=await reader.request(req(176,{kind:'view'}));
    check(raced.kind==='error'&&raced.code==='unavailable','Unpublished new CAS head released old view');
    check((await pending).kind==='receipt','Post-CAS writer failed');
    check((await reader.request(req(177,{kind:'view'}))).kind==='view','Known race closed the ordinary read channel');
    close();await h.finish();checks.push('Post-CAS/pre-publication refuses stale reads without fencing the accepted writer');

    await h.setup({history:8,delay_ms:400});await h.control({op:'writer-delay',milliseconds:0});
    const pressured=client(await connect('http')),attempt=pressured.request(req(178,{kind:'view'}));
    const outcome=attempt.then(r=>r.kind==='error'&&r.code==='unavailable');await waitFor(m=>m.live_read_io>0);
    await h.control({op:'writes',count:8,interval_ms:25});
    check(await outcome,'Global generation gate silently became account-local');
    check((await pressured.request(req(179,{kind:'view'}))).kind==='view','Read did not recover on the same channel after the quiet window');
    close();await h.finish();checks.push('Unrelated sustained commits may refuse reads; a quiet window restores service without weakening freshness');

    await h.setup({history:8,delay_ms:0});const first=client(await connect('http'));
    check((await first.request(req(180,grant))).kind==='receipt','Restart original grant');
    close();await h.restart();const afterRestart=await metrics(),replayed=client(await connect());
    check((await replayed.request(req(181,{kind:'operation',target:id(180)}))).kind==='receipt','Runtime restart lost original mutation');
    check((await metrics()).accepts===afterRestart.accepts,'Operation reconciliation resent mutation');
    close();await h.finish();checks.push('Encrypted real Runtime reopen reconciles the original mutation without resend');

    await h.setup({history:8,delay_ms:50});await h.control({op:'writer-delay',milliseconds:0});
    const traffic=client(await connect());check((await traffic.request(req(184,grant))).kind==='receipt','Traffic original grant');
    const trafficIt=(await traffic.subscribe(watch(185,BigInt(Date.now()+1900))))[Symbol.asyncIterator]();await next(trafficIt);
    const trafficBefore=await metrics();let served=0;
    for(let n=0;n<40;n++){
      try{check((await traffic.request(req(184,grant))).kind==='receipt','Traffic exact retry changed outcome');served++;}catch{break;}
    }
    await expectStreamClosed(trafficIt,'Continuous same-socket ingress starved subscription expiry');
    check(served>3&&(await metrics()).accepts===trafficBefore.accepts,'Traffic created duplicate mutations or did not exercise ingress');
    close();await h.finish();checks.push('Continuous same-socket exact retries do not starve signed subscription expiry or repeat acceptance');

    await h.setup({history:8,delay_ms:0});const slowChannel=await connect(),slowClient=client(slowChannel);
    const slow=(await slowClient.subscribe(watch(186)))[Symbol.asyncIterator]();await next(slow);
    const producer=client(await connect('http'));
    for(let n=0;n<33;n++){
      const disposable=await crypto.subtle.generateKey('Ed25519',false,['sign','verify']);
      const key=new Uint8Array(await crypto.subtle.exportKey('raw',disposable.publicKey));
      await h.control({op:'expect-notification'});
      check((await producer.request(req(187+n,{...grant,grant:{...grant.grant,key}}))).kind==='receipt','Slow consumer distinct grant');
      await h.control({op:'await-notification'});
    }
    const closedBy=performance.now()+2000;
    while(performance.now()<closedBy){try{slowChannel.context();}catch{break;}await new Promise(r=>setTimeout(r,10));}
    await expectStreamClosed(slow,'Real Runtime slow consumer queue exceeded its bound');
    check((await metrics()).peak_read_io<=4,'Slow consumer exceeded global read I/O budget');
    close();await h.finish();checks.push('Real Runtime slow consumer overflows the existing finite 32-update SDK queue and must reconnect');

    for(const kind of ['witness','epoch','head','credential','expire','fence']){
      await h.setup({history:8,delay_ms:50});const api=client(await connect('http'));
      check((await api.request(req(182,{kind:'view'}))).kind==='view','Fault fixture baseline');
      await h.control({op:'fault',kind});await denied(()=>api.request(req(183,{kind:'view'})),`Runtime stale reply after ${kind}`);
      close();await h.finish();
    }
    checks.push('Witness/head/epoch/credential port failures and lease/boot expiry produce no stale private output');
    return checks;
  }finally{close();}
}
