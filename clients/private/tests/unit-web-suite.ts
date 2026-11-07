// Same real UnitClient/Noise/Runtime/worker matrix in Node and Chrome.
// Only native ports, money, qualification and attestation are explicit fixtures.
import {UnitClient,READ,TRADE,type MessageSigner,type Command,type Response} from '../src/index.ts';
import {WebChannel,type WebCore} from '../src/web-channel-core.ts';
import {verifyWithRoot} from '../../../tools/web-channel/attestation/verifier.mjs';
import {id,policy} from './web-suite.ts';
interface Harness {
  setup(options:{units:boolean;history:number;delay_ms:number}):Promise<void>;
  current():{baseUrl:string;root:number[];account:number[]};
  control(c:Record<string,unknown>):Promise<{bulk:number;pacifica:number;reads:number;pacifica_wire?:string;bulk_wire?:string}>;
  restart():Promise<void>;
  finish():Promise<{peakRssBytes:number}>;
  record(r:Record<string,unknown>):void;
}
function check(value:unknown,message:string):asserts value {if(!value)throw Error(message);}
function receipt(r:Response){check(r.kind==='receipt',`Expected unit receipt, got ${r.kind}`);return r;}
function view(r:Response){check(r.kind==='view',`Expected unit view, got ${r.kind}`);return r;}
export async function runUnits(core:WebCore,signer:MessageSigner,h:Harness):Promise<string[]> {
  const channels:WebChannel[]=[];
  const connect=async(transport:'http'|'websocket')=>{
    const channel=await WebChannel.connect({baseUrl:h.current().baseUrl,policy,core,transport},
      (q,p,c)=>verifyWithRoot(q,p,c,Uint8Array.from(h.current().root)));
    channels.push(channel);return channel;
  };
  const close=()=>{for(const channel of channels.splice(0))channel.close();};
  const client=(channel:WebChannel,unit:number,s=signer)=>new UnitClient(channel,s,
    {domain:policy,account:Uint8Array.from(h.current().account),policy:1,unit:id(unit)});
  const request=(n:number,command:Command,epoch=1n)=>({id:id(n),epoch,expiresAt:BigInt(Date.now()+60000),command});
  const checks:string[]=[];
  try {
    await h.setup({units:true,history:0,delay_ms:0});
    let http=await connect('http'),ws=await connect('websocket');
    let a=client(http,40),b=client(ws,50);
    for(const c of [a,b])check(view(await c.request(request(10,{kind:'view'}))).cash===1000n,'Synthetic fixture opening cash');
    const keys=await crypto.subtle.generateKey('Ed25519',false,['sign','verify']);
    const agent:MessageSigner={publicKey:new Uint8Array(await crypto.subtle.exportKey('raw',keys.publicKey)),
      signMessage:async bytes=>new Uint8Array(await crypto.subtle.sign('Ed25519',keys.privateKey,Uint8Array.from(bytes).buffer))};
    receipt(await a.request(request(20,{kind:'grant',grant:{key:agent.publicKey,methods:READ|TRADE,market:id(7),maximumLots:2n,maximumFee:1n,maximumOrders:1n,expiresAt:BigInt(Date.now()+300000)}})));
    check((await client(ws,50,agent).request(request(21,{kind:'view'}))).kind==='error','Agent crossed unit');
    const order:Command={kind:'order',market:id(7),lots:2n,minimum:90n,maximum:110n,fee:1n,tif:'GTC',reduceOnly:false,goodUntil:BigInt(Date.now()+120000)};
    check(receipt(await client(http,40,agent).request(request(30,order))).outcome==='accepted','BULK financial admission');
    await h.control({op:'delay',milliseconds:700});
    const pending=h.control({op:'tick'}).catch(()=>{throw Error('First native worker cut failed');});
    await new Promise(resolve=>setTimeout(resolve,100));
    const started=performance.now();
    // A view alone uses immutable publication and would not prove writer
    // availability. This signed grant must acquire and commit through the writer.
    receipt(await b.request(request(31,{kind:'grant',grant:{key:agent.publicKey,methods:READ,market:id(7),maximumLots:1n,maximumFee:1n,maximumOrders:1n,expiresAt:BigInt(Date.now()+300000)}})));
    check(performance.now()-started<600,'Native I/O held the financial writer');
    check(view(await a.request(request(35,{kind:'view'}))).cash===1000n,'Read during native I/O');
    await pending;await h.control({op:'delay',milliseconds:0});
    check(receipt(await b.request(request(40,order))).outcome==='accepted','Pacifica financial admission');
    for(let i=0;i<4;i++)await h.control({op:'tick'}).catch(()=>{throw Error(`Worker cut ${i+2} failed`);});
    const sent=await h.control({op:'metrics'});
    check(sent.bulk===1&&sent.pacifica===1,`Round-robin native execution counts: BULK=${sent.bulk}, Pacifica=${sent.pacifica}, reads=${sent.reads}`);
    check(receipt(await a.request(request(32,{kind:'operation',target:id(30)}))).possiblyExposed,'Missing BULK original operation');
    check((await b.request(request(33,{kind:'operation',target:id(30)}))).kind==='error','Original lookup crossed unit');
    const p=JSON.parse(sent.pacifica_wire!);
    const body=JSON.stringify({channel:'account_trades',data:[{h:1,i:200,I:p.client_order_id,u:p.account,s:'BTC',a:'1',p:'100',o:'100',f:'1',n:'-1',te:'fulfill_taker',ts:'open_long',tc:'normal',t:p.timestamp,li:999,it:0}]});
    await h.control({op:'observe',unit:50,body});await h.control({op:'observe',unit:50,body});
    const partial=view(await b.request(request(41,{kind:'view'})));
    check(partial.cash===999n&&partial.positions[0].lots===1n,'Pacifica partial/duplicate fill');
    check(view(await a.request(request(42,{kind:'view'}))).cash===1000n,'Native fill affected another unit');
    checks.push('HTTP/WebSocket UnitClient, unit grants/originals, bounded native workers, writer-free I/O and exact fill attribution');
    receipt(await b.request(request(43,{kind:'revoke'})));
    check((await client(http,40,agent).request(request(44,{kind:'view'}))).kind==='error','Cross-unit revoke');
    check(receipt(await b.request(request(49,{...order,lots:1n},2n))).outcome==='accepted','Second admitted Pacifica order');
    close();await h.restart();http=await connect('http');ws=await connect('websocket');a=client(ws,40);b=client(http,50);
    check(view(await b.request(request(45,{kind:'view'},2n))).positions[0].lots===1n,'Durable encrypted restart');
    for(let i=0;i<4;i++)await h.control({op:'tick'}).catch(()=>{throw Error(`Restart worker cut ${i+1} failed`);});
    const reopened=await h.control({op:'metrics'});
    check(reopened.bulk===0&&reopened.pacifica===0,'Restart resent escaped native requests');
    check(!receipt(await b.request(request(50,{kind:'operation',target:id(49)},2n))).possiblyExposed,'Restart reset history-wide dispatch ceiling');
    check(receipt(await a.request(request(46,{kind:'operation',target:id(30)},2n))).possiblyExposed,'Restart lost unknown original');
    check((await client(ws,40,agent).request(request(47,{kind:'view'},2n))).kind==='error','Restart restored revoked grant');
    await h.control({op:'fence'});
    for(const c of [a,b]){
      let denied=false;try{const r=await c.request(request(48,{kind:'view'},2n));denied=r.kind==='error';}catch{denied=true;}
      check(denied,'Fenced unit returned private state');
    }
    close();const memory=await h.finish();h.record({kind:'unit-worker-acceptance',...memory});
    checks.push('Both units preserve epoch/unknown operations across process restart without resend; sticky shared fence');
    return checks;
  }finally{close();}
}
