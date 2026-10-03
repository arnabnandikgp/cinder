// SAME signed SDK/transport suite runs in Node AND an actual browser. Fixture
// controls belong to the test harness, never the public relay/SDK.
import { PrivateClient, READ_FAMILIES, type Command, type MessageSigner, type ReadPage } from '../src/index.ts';
import { WebChannel, type WebCore } from '../src/web-channel-core.ts';
import { connectWebChannel } from '../src/web-channel.ts';
import { verifyWithRoot } from '../../../tools/web-channel/attestation/verifier.mjs';
export const id = (n: number) => new Uint8Array(32).fill(n);
export const policy = { network:id(1), deployment:id(2), manifest:id(21), pcrs:[new Uint8Array(48).fill(22),new Uint8Array(48).fill(23),new Uint8Array(48).fill(24)] as const };
interface Fixture { baseUrl: string; root: number[] }
export interface Harness {
  current(): Fixture;
  control(action: string): Promise<void>;
}
function check(ok: unknown, label: string): asserts ok { if (!ok) throw Error(label); }
async function rejects(work: () => Promise<unknown>, label: string) {
  try { await work(); } catch { return; } throw Error(label);
}
export async function runHttp(core: WebCore, signer: MessageSigner, harness: Harness, transport:'http'|'websocket'='http'): Promise<string[]> {
  const checks: string[] = [];
  const channels: WebChannel[] = [];
  const options = () => ({ baseUrl:harness.current().baseUrl, policy, core, transport });
  const connect = async () => {
    const root = Uint8Array.from(harness.current().root);
    const ch = await WebChannel.connect(options(), (q,p,c) => verifyWithRoot(q,p,c,root)); channels.push(ch); return ch;
  };
  const client = (channel: WebChannel, account = 1) => new PrivateClient(channel, signer, {domain:policy, account:id(account), policy:1});
  const request = (n:number, command:Command, epoch=1n) => ({id:id(n),epoch,expiresAt:BigInt(Date.now()+20000),command});
  const order:Command = {kind:'order',market:id(7),lots:2n,minimum:90n,maximum:110n,fee:1n,tif:'GTC',reduceOnly:false,goodUntil:BigInt(Date.now()+60000)};
  const agentKeys=await crypto.subtle.generateKey('Ed25519',false,['sign','verify']);
  const agentSigner:MessageSigner={publicKey:new Uint8Array(await crypto.subtle.exportKey('raw',agentKeys.publicKey)),
    signMessage:async b=>new Uint8Array(await crypto.subtle.sign('Ed25519',agentKeys.privateKey,Uint8Array.from(b).buffer))};
  try {
    await rejects(() => connectWebChannel(options()), 'AWS entrypoint accepted synthetic root');
    await rejects(() => WebChannel.connect({...options(),policy:{...policy,manifest:id(99)}},(q,p,c)=>verifyWithRoot(q,p,c,Uint8Array.from(harness.current().root))), 'wrong release accepted');
    checks.push('AWS-only and wrong-release fail before private authentication');
    let ch = await connect(), api = client(ch);
    const context = ch.context(); context.binding.fill(0); check(ch.context().binding.some(b=>b),'mutable session alias');
    let reply = await api.request(request(30,{kind:'view'})); check(reply.kind==='view' && reply.cash===1000n,'seeded actual account view');
    reply = await client(ch,2).request(request(31,{kind:'view'})); check(reply.kind==='error' && reply.code==='unauthorized','cross account');
    const placed = await api.request(request(40,order)); check(placed.kind==='receipt' && placed.outcome==='dispatched' && placed.possiblyExposed,'real handler retained dispatch');
    const queried = await api.request(request(41,{kind:'operation',target:id(40)})); check(queried.kind==='receipt' && queried.outcome==='dispatched','operation query');
    const conflict = await api.request(request(40,{...order,lots:1n})); check(conflict.kind==='error' && conflict.code==='conflict','stable economic ID conflict');
    reply = await api.request(request(42,{kind:'cancel',target:id(40),attempt:id(52),goodUntil:BigInt(Date.now()+20000)})); check(reply.kind==='receipt'&&reply.outcome!=='rejected','cancel handler');
    reply = await api.request(request(43,{kind:'payout',net:10n,maximumFee:2n,allowPartial:false,goodUntil:BigInt(Date.now()+20000)})); check(reply.kind==='receipt'&&reply.outcome!=='rejected','owner payout handler');
    reply = await api.request(request(44,{kind:'leverage',market:id(7),leverage:20000n,goodUntil:BigInt(Date.now()+20000)})); check(reply.kind==='receipt'&&reply.outcome!=='rejected','leverage handler');
    reply = await api.request(request(45,{kind:'grant',grant:{key:agentSigner.publicKey,methods:7,market:id(7),maximumLots:2n,maximumFee:1n,maximumOrders:1n,expiresAt:BigInt(Date.now()+20000)}})); check(reply.kind==='receipt','owner grant');
    const agent=new PrivateClient(ch,agentSigner,{domain:policy,account:id(1),policy:1});
    reply=await agent.request(request(48,{kind:'view'}));check(reply.kind==='view','scoped agent view');
    for(const [n,command] of [[49,{kind:'payout',net:1n,maximumFee:0n,allowPartial:false,goodUntil:BigInt(Date.now()+20000)}],
      [50,{kind:'revoke'}],[51,{kind:'leverage',market:id(7),leverage:20000n,goodUntil:BigInt(Date.now()+20000)}]] as [number,Command][]){
      reply=await agent.request(request(n,command));check(reply.kind==='error'&&reply.code==='unauthorized','agent got owner authority');
    }
    reply = await api.request(request(46,{kind:'revoke'})); check(reply.kind==='receipt','owner revoke');
    reply=await agent.request(request(54,{kind:'view'}));check(reply.kind==='error'&&reply.code==='unauthorized','revoked agent remained live');
    for(const family of READ_FAMILIES){
      reply=await api.request(request(55,{kind:'read',query:{family,limit:64}},2n));
      check(reply.kind==='page'&&reply.family===family,'signed read family/codec');
    }
    checks.push('all eight signed commands use one handler; private errors and ownership');
    checks.push('real Cinder agent signatures, owner-only payouts/admin and live revocation');
    checks.push('all ten bounded signed read families reach the real journal');
    ch.close();
    await harness.control('restart'); ch = await connect(); api = client(ch);
    const after = await api.request(request(47,{kind:'operation',target:id(40)},2n)); check(after.kind==='receipt' && after.outcome==='dispatched','retained journal after crash');
    checks.push('fresh boot/key/quote reconnect retains journal economic identity');
    await harness.control('corrupt-request');
    await rejects(()=>api.request(request(60,order,2n)), 'corrupt request accepted'); ch.close();
    ch = await connect(); api = client(ch);
    reply=await api.request(request(61,{kind:'operation',target:id(60)},2n)); check(reply.kind==='error' && reply.code==='not-found','corruption mutated state');
    await harness.control('corrupt-reply');
    await rejects(()=>api.request(request(60,order,2n)), 'corrupt reply accepted'); ch.close();
    await harness.control('restart'); ch=await connect(); api=client(ch);
    reply=await api.request(request(62,{kind:'operation',target:id(60)},2n)); check(reply.kind==='receipt' && reply.outcome==='dispatched','lost reply did not reconcile');
    const retried=await api.request(request(60,order,2n)); check(retried.kind==='receipt' && retried.outcome==='dispatched','exact economic retry');
    checks.push('hostile request rejects; lost committed reply reconciles after restart');
    ch.close(); ch=await connect();
    await rejects(()=>ch.exchange(new Uint8Array(1025)), 'request size not bounded');
    await rejects(()=>ch.exchange(new Uint8Array([1])), 'failed channel reused');
    ch.close(); ch=await connect();
    const first=ch.exchange(new Uint8Array([1]));
    await rejects(()=>ch.exchange(new Uint8Array([1])), 'concurrent request queued');
    await first.catch(()=>{}); check(await Promise.resolve().then(()=>{try{ch.context();return false;}catch{return true;}}),'concurrent failure not sticky');
    checks.push('bounded requests, immutable context and concurrent fail-closed backpressure');
    ch=await connect();
    for(let i=0;i<127;i++)await ch.exchange(Uint8Array.of(1));
    await rejects(()=>ch.exchange(Uint8Array.of(1)), 'directional session budget not finite');
    checks.push('127 application records plus confirmation exhaust the finite session');
    ch=await connect(); api=client(ch);
    await harness.control('witness-loss');
    reply=await api.request(request(63,{kind:'view'},2n)); check(reply.kind==='error' && reply.code==='unavailable','stale read after witness loss');
    checks.push('witness failure returns only encrypted unavailable, never stale account');
    return checks;
  } finally { for(const ch of channels) ch.close(); }
}

/** Joined command/subscription tests, not a synthetic financial WebSocket peer. */
export async function runWebsocket(core:WebCore,signer:MessageSigner,harness:Harness):Promise<string[]>{
  const checks:string[]=[],channels:WebChannel[]=[];
  const connect=async(transport:'http'|'websocket'='websocket')=>{
    const ch=await WebChannel.connect({baseUrl:harness.current().baseUrl,policy,core,transport},
      (q,p,c)=>verifyWithRoot(q,p,c,Uint8Array.from(harness.current().root)));channels.push(ch);return ch;
  };
  const client=(ch:WebChannel,s=signer)=>new PrivateClient(ch,s,{domain:policy,account:id(1),policy:1});
  const req=(n:number,command:Command,epoch=1n)=>({id:id(n),epoch,expiresAt:BigInt(Date.now()+20000),command});
  const subscription=(n:number,epoch=1n,expiresAt=BigInt(Date.now()+90000))=>({id:id(n),epoch,expiresAt,query:{family:'operations' as const,limit:64}});
  async function next(iterator:AsyncIterator<ReadPage>){
    let timer:ReturnType<typeof setTimeout>|undefined;
    try{const result=await Promise.race([iterator.next(),new Promise<never>((_,reject)=>{timer=setTimeout(()=>reject(Error('Missing private update')),6000);})]);
      check(!result.done,'subscription ended');return result.value;
    }finally{clearTimeout(timer);}
  }
  const order:Command={kind:'order',market:id(7),lots:1n,minimum:90n,maximum:110n,fee:1n,tif:'GTC',reduceOnly:false,goodUntil:BigInt(Date.now()+60000)};
  try{
    const http=await connect('http'),owner=client(http),ch=await connect(),api=client(ch);
    const stream=await api.subscribe(subscription(100)),it=stream[Symbol.asyncIterator]();
    const initial=await next(it);check(initial.rows.length===0,'initial permitted snapshot');
    const placed=await api.request(req(101,order));check(placed.kind==='receipt'&&placed.outcome==='dispatched','command during subscription');
    const update=await next(it);check(update.rows.length===1&&update.rows[0].kind==='operation'&&update.rows[0].receipt.outcome==='dispatched','committed correlated command update');
    const cancelled=await owner.request(req(102,{kind:'cancel',target:id(101),attempt:id(112),goodUntil:BigInt(Date.now()+20000)}));check(cancelled.kind==='receipt','alternate HTTP cancel');
    const after=await next(it);check(after.rows.length===2&&after.revision.some((b,i)=>b!==update.revision[i]),'HTTP commit pushed to WS');
    await it.return?.();
    const fresh=client(await connect());const receipt=await fresh.request(req(103,{kind:'operation',target:id(101)}));
    check(receipt.kind==='receipt'&&receipt.outcome==='dispatched','reconnect did not resend/cancel away unknown order');
    const page=await fresh.request(req(104,{kind:'read',query:{family:'operations',limit:1}}));
    check(page.kind==='page'&&page.next.some(x=>x),'bounded initial snapshot page');
    const last=await fresh.request(req(105,{kind:'read',query:{family:'operations',limit:1,cursor:page.next}}));
    check(last.kind==='page'&&last.rows.length===1&&!last.next.some(x=>x),'stable next permitted page');
    checks.push('command correlation, initial/update snapshots, HTTP/WS one journal and fresh pagination');

    const keys=await crypto.subtle.generateKey('Ed25519',false,['sign','verify']);
    const agentSigner:MessageSigner={publicKey:new Uint8Array(await crypto.subtle.exportKey('raw',keys.publicKey)),
      signMessage:async b=>new Uint8Array(await crypto.subtle.sign('Ed25519',keys.privateKey,Uint8Array.from(b).buffer))};
    await owner.request(req(106,{kind:'grant',grant:{key:agentSigner.publicKey,methods:1,market:id(7),maximumLots:1n,maximumFee:1n,maximumOrders:1n,expiresAt:BigInt(Date.now()+60000)}}));
    const agent=client(await connect(),agentSigner),agentIt=(await agent.subscribe(subscription(107)))[Symbol.asyncIterator]();
    const agentPage=await next(agentIt);check(agentPage.rows.every(r=>r.kind!=='operation'||r.command.kind!=='grant'),'agent directory leaked in subscription');
    const revoked=await owner.request(req(108,{kind:'revoke'}));check(revoked.kind==='receipt','owner revoke');
    await rejects(()=>next(agentIt),'revoked stream emitted stale authority');
    checks.push('current READ-agent permission checked on every server emission; revoke closes stream');

    const expiryClient=client(await connect()),exp=(await expiryClient.subscribe(subscription(109,2n,BigInt(Date.now()+1800))))[Symbol.asyncIterator]();
    await next(exp);await rejects(()=>next(exp),'expired signed subscription kept alive');
    checks.push('signed subscription expiry rechecked without client traffic');

    const gapClient=client(await connect()),gap=(await gapClient.subscribe(subscription(110,2n)))[Symbol.asyncIterator]();
    await next(gap);await harness.control('drop-notification');
    const first=await owner.request(req(111,{kind:'leverage',market:id(7),leverage:20000n,goodUntil:BigInt(Date.now()+20000)},2n));check(first.kind==='receipt','first gap commit');
    await harness.control('await-drop');
    const second=await owner.request(req(113,{kind:'leverage',market:id(7),leverage:20000n,goodUntil:BigInt(Date.now()+20000)},2n));check(second.kind==='receipt','second gap commit');
    await rejects(()=>next(gap),'missing encrypted record silently continued');
    const recovered=client(await connect()),snapshot=await recovered.request(req(114,{kind:'read',query:{family:'operations',limit:64}},2n));
    check(snapshot.kind==='page'&&snapshot.rows.filter(r=>r.kind==='operation'&&r.command.kind==='leverage').length===2,'gap recovery lost committed operations');
    checks.push('dropped delivery fails closed; fresh signed snapshot reconciles without mutation replay');

    // Leave the initial iterator consumed but stop consuming further updates.
    // One change per poll drives the real finite 32-update client queue.
    const slowChannel=await connect(),slowClient=client(slowChannel),slow=(await slowClient.subscribe(subscription(115,2n)))[Symbol.asyncIterator]();
    await next(slow);
    for(let i=0;i<33;i++){
      await harness.control('expect-notification');
      const r=await owner.request(req(150+i,{kind:'leverage',market:id(7),leverage:20000n,goodUntil:BigInt(Date.now()+20000)},2n));check(r.kind==='receipt','bounded queue fixture commit');
      await harness.control('await-notification');
    }
    const closeBy=Date.now()+2000;
    while(Date.now()<closeBy){try{slowChannel.context();}catch{break;}await new Promise(resolve=>setTimeout(resolve,10));}
    await rejects(()=>next(slow),'slow consumer queue grew without bound');
    checks.push('slow consumer overflows the bounded real queue and must resnapshot');

    const fenced=client(await connect()),fence=(await fenced.subscribe(subscription(116,2n)))[Symbol.asyncIterator]();
    await next(fence);await harness.control('witness-loss');
    await rejects(()=>next(fence),'witness loss emitted stale private snapshot');
    checks.push('idle stream revalidates witnessed freshness and refuses stale state');
    return checks;
  }finally{for(const ch of channels)ch.close();}
}
