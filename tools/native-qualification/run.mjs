#!/usr/bin/env node
// One-shot local native-semantics probe. Never enables shipping financial gates.
import { createHash } from 'node:crypto';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { rootCertificates } from 'node:tls';
import { createRequire } from 'node:module';
import { setTimeout as pause } from 'node:timers/promises';
import { LIMITS,canonical,sha,privateRead,exclusive,validateManifest,assertApproval,Journal,
  boundedBody,decodeResponse,nativeAck,setupObservations,transfers,emptyBalanceBaseline,flatAccountObservation,depositLink,withdrawalLink,paymentDelta,tokenDelta,atoms } from './core.mjs';
import { checkEnvironment,validateSignerLocator,runDirectory,sources,tlsRoots } from './artifacts.mjs';

// The independent offline harness drives this same sequence. Native receipt loss
// is injected at the port boundary before it can become a controller ACK.
export async function qualify(p) {
  await p.preflight();await p.baseline();
  const before=await p.disable('predeposit');
  if(!['disabled','rejected'].includes(before))throw Error('Unknown predeposit setting outcome');
  await p.fund();
  const deposit=await p.capture(()=>p.deposit());
  let setup;
  try {
    if(before==='rejected'&&await p.disable('postdeposit')!=='disabled')throw Error('Postdeposit disable not established');
    setup=await p.setup();
    if(setup.status!=='observed-disabled-no-debt')throw Error('Setup observations incomplete');
    p.checkBootstrap();
  } catch(e) {p.note('setup-blocker');setup={status:'unresolved',shipping_complete:false};}
  // Withdrawal-only containment is in the approved scope. It uses the one
  // original UUID even if setup failed; never authorizes trading or new deposits.
  const withdrawal=await p.capture(()=>p.withdrawLostReply());
  const payment=await p.reconcilePayment(withdrawal);
  if(!payment)throw Error('Original withdrawal unresolved; no replacement or rescue');
  await p.returnNet(payment);
  return p.closeout({deposit,setup,payment,controller_ack:null});
}

export async function main(directory) {
  checkEnvironment();const dir=runDirectory(directory),m=JSON.parse(privateRead(join(dir,'manifest.json'))),
    config=JSON.parse(privateRead(join(dir,'config.json'))),approval=JSON.parse(privateRead(join(dir,'approval.json')));
  const seal=validateManifest(m);assertApproval(m,approval,Date.now());
  const {web3,token,base58,ROUTE,ata,pub,decodeSigner,signNative,nativeInstruction,verifyDeployment,verifyMint,verifyToken}=await import('./bindings.mjs');
  const {Transaction,SystemProgram}=web3;
  if(canonical(m.route)!==canonical(ROUTE)||canonical(m.sponsor_transfers)!==canonical({owner:'35000000',broker:'25000000'})
    ||m.sources!==sources()||m.tls_roots!==tlsRoots()||m.rpc_config_hash!==sha(config.rpc)||m.sponsor_locator_hash!==sha(config.wallet)
    ||new URL(config.rpc).origin!==m.rpc_origin||m.owner_tokens!==ata(m.owner).toBase58()||m.broker_tokens!==ata(m.broker).toBase58())throw Error('Execution record changed');
  const load=(path,expected)=>{
    return decodeSigner(JSON.parse(privateRead(path,8192)),expected);
  };
  // A fresh lock is never removed automatically, even on a clean result. Read-only
  // analysis does not clear it; an interrupted live invocation cannot restart.
  for(const path of [join(dir,'owner.key'),join(dir,'broker.key'),config.wallet])validateSignerLocator(path);
  mkdirSync(join(dir,'running.lock'),{mode:0o700});
  const j=new Journal(dir,m,Date.now);j.begin();
  let owner,broker,sponsor;
  try {owner=load(join(dir,'owner.key'),m.owner);broker=load(join(dir,'broker.key'),m.broker);sponsor=load(config.wallet,m.sponsor);}
  catch {j.append('stopped',{reason:'Signer loading refused before any network request'});throw Error('Signer loading refused');}
  let lastHttp=0,sequence=0,nativeDepositAt,programDigest,sponsorDebit=0n;const observed=[];
  const request=async(kind,url,options,cleanup=false)=>{
    assertApproval(m,approval,Date.now());
    if(kind==='http') {
      // One conservative request every 12 seconds, shared by study and cleanup;
      // no alternate account/IP, reconnect quota evasion or retry on 429.
      await pause(Math.max(0,lastHttp+12000-Date.now()));lastHttp=Date.now();
    }
    j.consume(kind,{url:kind==='rpc'?m.rpc_origin:new URL(url).pathname,method:options.method},cleanup);
    const n=++sequence;exclusive(join(dir,`request-${n}.json`),{kind,url:kind==='rpc'?m.rpc_origin:url,options});
    const response=await fetch(url,{...options,redirect:'error',signal:AbortSignal.timeout(10000)});
    const bytes=await boundedBody(response);exclusive(join(dir,`response-${n}.json`),{status:response.status,body:bytes.toString(),at:Date.now()});
    j.append('response',{request:n,status:response.status,hash:sha(bytes)});
    const body=decodeResponse(bytes);
    if(response.status===429)throw Error('Rate limited; no automatic retry');
    return {status:response.status,body};
  };
  const rpc=async(method,params=[],cleanup=false)=>{
    const r=await request('rpc',config.rpc,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({jsonrpc:'2.0',id:1,method,params})},cleanup);
    if(r.status!==200||r.body.id!==1||r.body.error||!Object.hasOwn(r.body,'result'))throw Error('RPC refused/unknown');return r.body.result;
  };
  const get=async(path,cleanup=false)=>{
    if(!['account','account/settings','account/loan','positions','orders','account/balance/history'].includes(path))throw Error('Native GET scope');
    const url=new URL(`/api/v1/${path}`,m.native_origin);url.searchParams.set('account',m.broker);
    if(path.endsWith('history'))url.searchParams.set('limit','20');
    return request('http',url.href,{method:'GET',headers:{accept:'application/json'}},cleanup);
  };
  const info=async(account,cleanup=false,dataSlice)=>(await rpc('getAccountInfo',[account,{encoding:'base64',commitment:'finalized',...(dataSlice?{dataSlice}:{})}],cleanup)).value;
  const deployment=async(cleanup=false)=>{
    if(await rpc('getGenesisHash',[],cleanup)!==m.genesis)throw Error('Wrong cluster');
    const program=await info(ROUTE.program,cleanup),data=await info(ROUTE.program_data,cleanup,{offset:0,length:45});
    verifyDeployment(program,data);
    if(!programDigest) {
      if(!Number.isSafeInteger(data.space)||data.space<46||data.space>4194304)throw Error('Native executable size bound');
      const hash=createHash('sha256');
      for(let offset=45;offset<data.space;offset+=65536) {
        const length=Math.min(65536,data.space-offset),chunk=await info(ROUTE.program_data,cleanup,{offset,length});
        const bytes=Buffer.from(chunk?.data?.[0]??'','base64');
        if(chunk?.owner!==ROUTE.loader||chunk.executable!==false||chunk.space!==data.space||bytes.length!==length)throw Error('Native executable slice');
        hash.update(bytes);
      }
      // Repeat the loader header after streamed reads. Later submissions require
      // this same explicit slot/authority; no changed deployment is accepted.
      verifyDeployment(await info(ROUTE.program,cleanup),await info(ROUTE.program_data,cleanup,{offset:0,length:45}));
      programDigest=hash.digest('hex');
    }
    j.append('native-deployment',{sha256:programDigest,loader_slot:ROUTE.slot,source_equivalence:false});
  };
  const final=async(signature,cleanup=false)=>{
    for(let i=0;i<30;i++) {
      const status=(await rpc('getSignatureStatuses',[[signature],{searchTransactionHistory:true}],cleanup)).value?.[0];
      if(status?.err)throw Error('Original transaction failed');
      if(status?.confirmationStatus==='finalized') {
        const tx=await rpc('getTransaction',[signature,{encoding:'jsonParsed',commitment:'finalized',maxSupportedTransactionVersion:0}],cleanup);
        if(tx?.meta?.err!==null||tx.transaction?.signatures?.[0]!==signature)throw Error('Original transaction receipt');
        return {signature,status,tx};
      }
      await pause(2000);
    }
    throw Error('Original signature remains unknown; no resend');
  };
  const send=async(identity,key,instructions,{sponsorAmount=0n,cleanup=false}={})=>{
    await deployment(cleanup);
    const block=(await rpc('getLatestBlockhash',[{commitment:'finalized'}],cleanup)).value;
    const tx=new Transaction({feePayer:key.publicKey,recentBlockhash:block.blockhash});tx.add(...instructions);
    const message=Buffer.from(tx.serializeMessage());
    const fee=(await rpc('getFeeForMessage',[message.toString('base64'),{commitment:'finalized'}],cleanup)).value;
    if(!Number.isSafeInteger(fee)||fee<0||fee>1000000)throw Error('Network fee bound');
    const sim=await rpc('simulateTransaction',[tx.serialize({requireAllSignatures:false,verifySignatures:false}).toString('base64'),
      {encoding:'base64',commitment:'finalized',sigVerify:false,replaceRecentBlockhash:false}],cleanup);
    j.append('simulation',{identity,payer:key.publicKey.toBase58(),fee,result:sim.value?.err??null,valid:sim.value?.err===null});
    if(sim.value?.err!==null)throw Error('Simulation refused; no signature/submission');
    console.log(JSON.stringify({simulation:identity,cluster:'devnet',passed:true,fee_lamports:fee}));
    if(key===sponsor) {
      if(sponsorDebit+sponsorAmount+BigInt(fee)>BigInt(LIMITS.sponsor_lamports))throw Error('Sponsor debit cap');
      sponsorDebit+=sponsorAmount+BigInt(fee);
    }
    j.append('chain-intent',{identity,payer:key.publicKey.toBase58(),message_hash:sha(message),fee,sponsor_debit:sponsorDebit.toString()});
    tx.sign(key);
    if(!tx.serializeMessage().equals(message)||!tx.verifySignatures())throw Error('Signature/message binding');
    const wire=tx.serialize(),signature=base58.encode(tx.signature);
    exclusive(join(dir,`wire-${identity}.json`),{wire:wire.toString('base64'),signature,last_valid_block_height:block.lastValidBlockHeight});
    const height=await rpc('getBlockHeight',[{commitment:'finalized'}],cleanup);
    if(!Number.isSafeInteger(height)||height>block.lastValidBlockHeight)throw Error('Expired original wire; no re-sign');
    j.once(identity,{signature,wire_hash:sha(wire)});
    if(identity==='native-deposit')nativeDepositAt=Date.now();
    const reported=await rpc('sendTransaction',[wire.toString('base64'),{encoding:'base64',skipPreflight:false,maxRetries:0,preflightCommitment:'finalized'}],cleanup);
    if(reported!==signature)throw Error('Submission identity mismatch');
    const receipt=await final(signature,cleanup);exclusive(join(dir,`receipt-${identity}.json`),JSON.stringify(receipt));return receipt;
  };
  const post=async(type,identity,data,cleanup=false,lose=false)=>{
    const timestamp=Date.now(),header={timestamp,expiry_window:30000,type},message=Buffer.from(canonical({...header,data}));
    j.append('native-intent',{identity,type,message_hash:sha(message)});
    const signature=signNative(message,broker),body={account:m.broker,...header,...data,signature:base58.encode(signature)};delete body.type;
    exclusive(join(dir,`native-${identity}.json`),{message:message.toString(),body});
    const path=type==='withdraw'?'/api/v1/account/withdraw':'/api/v1/account/settings/auto_lend_disabled';
    // Rate waiting precedes signature exposure; old signed requests are never
    // refreshed or replayed if the bounded expiry cannot be met.
    await pause(Math.max(0,lastHttp+12000-Date.now()));
    if(Date.now()-timestamp>=30000)throw Error('Original native request expired unsent');
    j.once(identity,{type,request_hash:sha(canonical(body))});
    const result=await request('http',m.native_origin+path,{method:'POST',headers:{'content-type':'application/json'},body:canonical(body)},cleanup);
    exclusive(join(dir,`audit-${identity}.json`),result);
    if(lose){j.append('lost-native-reply',{identity,controller_accepted:false});return null;}
    j.append('native-reply',{identity,status:result.status,success:result.body.success===true});return result;
  };
  const require=createRequire(new URL('../../clients/vault/package.json',import.meta.url)),WebSocket=require('ws');
  const capture=async(action,cleanup=false)=>{
    j.consume('wss',{},cleanup);const captureId=sequence++,begin=Date.now();let count=0,size=0,failure;
    const socket=new WebSocket(m.wss_origin,{ca:rootCertificates,rejectUnauthorized:true,handshakeTimeout:5000,maxPayload:LIMITS.message_bytes,perMessageDeflate:false,followRedirects:false});
    const timer=setTimeout(()=>socket.terminate(),LIMITS.socket_ms);
    const end=new Promise(resolve=>socket.once('close',resolve));
    socket.on('error',()=>{failure=Error('Finite WSS capture failed');socket.terminate();});
    socket.on('message',bytes=>{
      try {
        count++;size+=bytes.length;
        if(count>LIMITS.messages||size>LIMITS.payload_bytes||bytes.length>LIMITS.message_bytes||Date.now()-begin>LIMITS.socket_ms)throw Error('WSS budget');
        exclusive(join(dir,`capture-${captureId}-${count}.json`),{at:Date.now(),body:bytes.toString()});
        j.append('wss-body',{capture:captureId,count,hash:sha(bytes)});
        observed.push(...transfers(decodeResponse(bytes),m.broker));
      } catch(e){failure=e;socket.terminate();}
    });
    try {
      await new Promise((resolve,reject)=>{socket.once('open',resolve);socket.once('error',()=>reject(Error('WSS handshake failed')));socket.once('close',()=>reject(Error('WSS closed before open')));});
      socket.send(canonical({method:'subscribe',params:{source:'account_transfers',account:m.broker}}));
      const result=await action();await end;if(failure)throw failure;return result;
    } finally {clearTimeout(timer);socket.terminate();await end;}
  };
  const p={
    preflight:async()=>{await deployment();verifyMint(await info(ROUTE.mint));verifyToken(await info(ROUTE.vault),ROUTE.central);
      if(await info(m.owner)||await info(m.broker)||await info(m.owner_tokens)||await info(m.broker_tokens))throw Error('Fresh identities not empty');
      if((await rpc('getBalance',[m.sponsor,{commitment:'finalized'}])).value<100000000)throw Error('Sponsor devnet balance');},
    baseline:async()=>{
      const account=await get('account'),history=await get('account/balance/history');
      j.append('native-baseline',emptyBalanceBaseline(account,history));
    },
    disable:async stage=>{
      const result=await post('set_auto_lend_disabled',`disable-${stage}`,{disabled:true});
      if(result.status===200&&result.body.success===true){const r=await get('account/settings');if(r.body.success===true&&r.body.data?.auto_lend_disabled===true)return 'disabled';return 'unknown';}
      // The historical exact no-account rejection is a diagnostic bootstrap
      // allowance only, not a general no-later-effect financial certificate.
      if(stage==='predeposit'&&result.status===422&&result.body.success===false&&result.body.error==='Failed to set auto lend disabled')return 'rejected';
      return 'unknown';
    },
    fund:async()=>{
      for(const role of ['owner','broker'])await send(`sponsor-${role}`,sponsor,[SystemProgram.transfer({fromPubkey:sponsor.publicKey,toPubkey:pub(m[role]),lamports:BigInt(m.sponsor_transfers[role])})],{sponsorAmount:BigInt(m.sponsor_transfers[role])});
      await send('faucet',owner,[nativeInstruction('mint_test_usdc',m.owner)]);
      if(verifyToken(await info(m.owner_tokens),m.owner)!==20000000n)throw Error('Faucet amount');
      await send('allocate',owner,[token.createAssociatedTokenAccountIdempotentInstruction(owner.publicKey,pub(m.broker_tokens),broker.publicKey,pub(ROUTE.mint)),
        token.createTransferCheckedInstruction(pub(m.owner_tokens),pub(ROUTE.mint),pub(m.broker_tokens),owner.publicKey,20000000n,6)]);
      if(verifyToken(await info(m.broker_tokens),m.broker)!==20000000n)throw Error('Original allocation mismatch');
    },
    capture:action=>capture(action,!!nativeDepositAt),deposit:async()=>{
      const receipt=await send('native-deposit',broker,[nativeInstruction('deposit',m.broker)]);
      const binding={signature:receipt.signature,mint:ROUTE.mint};
      if(tokenDelta(receipt.tx,receipt.status,{...binding,ata:m.broker_tokens,owner:m.broker})!==-20000000n
        ||tokenDelta(receipt.tx,receipt.status,{...binding,ata:ROUTE.vault,owner:ROUTE.central})!==20000000n)throw Error('Original deposit token effects');
      j.append('original-deposit-effect',{broker_debit:'20000000',venue_vault_credit:'20000000',source_cut:null});
      return receipt;
    },
    setup:async()=>setupObservations((await get('account/settings')).body,(await get('account/loan')).body),
    checkBootstrap:()=>{if(!nativeDepositAt||Date.now()-nativeDepositAt>LIMITS.bootstrap_ms)throw Error('Bootstrap deadline');},
    note:reason=>j.append('blocker',{reason}),
    withdrawLostReply:async()=>{
      const account=await get('account',true);if(account.body.success!==true||atoms(account.body.data?.available_to_withdraw)<20000000n)throw Error('Original withdrawal not available');
      return post('withdraw','native-withdraw',{amount:'20',idempotency_key:m.withdraw_uuid},true,true);
    },
    reconcilePayment:async()=>{
      const audit=JSON.parse(privateRead(join(dir,'audit-native-withdraw.json')));if(audit.status!==200)return null;
      const ack=nativeAck(audit.body);j.append('audit-only-ack',{...ack,controller_accepted:false});
      for(let i=0;i<3;i++) {
        const link=withdrawalLink(observed,ack,null,m.broker);
        if(link.signature) {
          const receipt=await final(link.signature,true),delta=paymentDelta(receipt.tx,receipt.status,{signature:link.signature,ata:m.broker_tokens,mint:ROUTE.mint,owner:m.broker});
          if(delta.toString()!==link.net)throw Error('Payment/transfer mismatch');
          exclusive(join(dir,'native-payment.json'),JSON.stringify({link,receipt}));return {...link,fee:ack.fee};
        }
        // Separate planned, finite observation windows, not an economic resend.
        await capture(async()=>{await get('account',true);await get('account/balance/history',true);},true);
      }
      return null;
    },
    returnNet:async payment=>{
      const net=BigInt(payment.net);if(verifyToken(await info(m.broker_tokens,true),m.broker)!==net)throw Error('Broker payment differs from original net');
      await send('return-net',broker,[token.createTransferCheckedInstruction(pub(m.broker_tokens),pub(ROUTE.mint),pub(m.owner_tokens),broker.publicKey,net,6)],{cleanup:true});
    },
    closeout:async state=>{
      const account=await get('account',true),settings=await get('account/settings',true),loan=await get('account/loan',true),
        positions=await get('positions',true),orders=await get('orders',true),history=await get('account/balance/history',true);
      const ownerAtoms=verifyToken(await info(m.owner_tokens,true),m.owner),brokerAtoms=verifyToken(await info(m.broker_tokens,true),m.broker);
      const sol={};for(const role of ['owner','broker','sponsor']) {
        const value=(await rpc('getBalance',[m[role],{commitment:'finalized'}],true)).value;
        if(!Number.isSafeInteger(value)||value<0)throw Error('SOL balance units');sol[role]=String(value);
      }
      const clean=ownerAtoms===BigInt(state.payment.net)&&brokerAtoms===0n&&flatAccountObservation(account)
        &&[positions,orders].every(r=>r.status===200&&r.body.success===true&&Array.isArray(r.body.data)&&r.body.data.length===0)
        &&setupObservations(settings.body,loan.body).status==='observed-disabled-no-debt';
      return {status:clean?'assets-reconciled-financial-gates-unqualified':'closeout-unresolved',setup:state.setup,
        deposit:depositLink(observed,state.deposit.signature,m.broker),payment:state.payment,owner_atoms:ownerAtoms.toString(),broker_atoms:brokerAtoms.toString(),
        sponsor_debit_reserved:sponsorDebit.toString(),history_observed:history.body.success===true,source_cut:null,
        pending_balance_observed:typeof account.body.data?.pending_balance==='string'?account.body.data.pending_balance:null,pending_operations_complete:false,
        sol_balances_lamports:sol,gas_and_token_account_rent_reclaimed:false,
        lost_reply_reconciliation:'unresolved-native-UUID-query-not-established',agent_mutations:0,
        shipping:false,nitro:false,controller_credit:false,financial_completion:false};
    },
  };
  try {const result=await qualify(p);exclusive(join(dir,'result.json'),result);j.append('closed',{status:result.status});console.log(JSON.stringify({seal,status:result.status,shipping:false}));}
  catch {j.append('stopped',{reason:'Inspect private original request/receipt evidence; no automatic restart/rescue'});console.error('Native qualification stopped; original identities and budgets retained. Inspect private evidence.');process.exitCode=1;}
  // JS/SDK opaque key objects have no secure-erasure guarantee. This disposable
  // diagnostic process is not a production secret-lifetime implementation.
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
  if(process.argv.length!==3)throw Error('Usage: run.mjs EXACT_APPROVED_RUN_DIRECTORY');
  try {await main(process.argv[2]);}catch{console.error('Native qualification refused before execution; check the exact approval, source seal and private permissions.');process.exitCode=1;}
}
