// Bounded diagnostic evidence, not a shipping Controller or financial provider.
import { createHash } from 'node:crypto';
import { openSync, closeSync, writeSync, fsyncSync, readFileSync, fstatSync, lstatSync, constants } from 'node:fs';
import { join } from 'node:path';

export const sha = bytes => createHash('sha256').update(bytes).digest('hex');
export const canonical = x => {
  if (Array.isArray(x)) return `[${x.map(canonical).join(',')}]`;
  if (x && typeof x === 'object') return `{${Object.keys(x).sort().map(k => `${JSON.stringify(k)}:${canonical(x[k])}`).join(',')}}`;
  if (typeof x === 'number' && !Number.isSafeInteger(x)) throw Error('Noninteger diagnostic value');
  const s = JSON.stringify(x); if (s === undefined) throw Error('Undefined diagnostic value'); return s;
};
export const LIMITS = Object.freeze({ http:200, rpc:400, wss:8, duration_ms:1200000,
  bootstrap_ms:120000, quote_atoms:'20000000', fee_atoms:'2000000', sponsor_lamports:'100000000',
  socket_ms:30000, messages:256, message_bytes:16384, payload_bytes:1048576 });
// Separate house/test-capital staging, never the old customer-timing exception.
export const STAGED_LIMITS = Object.freeze({...LIMITS,bootstrap_ms:300000,initialization_cycles:4});
// Separately sealed persistent diagnostic. Forward one original signed wire;
// do not renew an expired transaction or silently change historical scenarios.
export const DELIVERY_LIMITS = Object.freeze({...STAGED_LIMITS,bootstrap_ms:600000,
  initialization_cycles:12,rpc_max_retries:5,status_polls:60,poll_ms:2000});
export function deliveryPolicy(m) {
  validateManifest(m);
  return m.schema==='cinder-native-staged-delivery-v1'
    ? {maxRetries:m.limits.rpc_max_retries,polls:m.limits.status_polls,pollMs:m.limits.poll_ms}
    : {maxRetries:0,polls:30,pollMs:2000};
}
export function faucetRecipient(m) {
  validateManifest(m);
  return m.faucet_mode==='broker-direct'?m.broker:m.owner;
}
export function atoms(s) {
  if (typeof s !== 'string' || !/^(0|[1-9][0-9]{0,13})(\.[0-9]{1,6})?$/.test(s)) throw Error('Exact quote decimal required');
  const [a,b=''] = s.split('.'); return BigInt(a)*1000000n + BigInt(b.padEnd(6,'0'));
}
export function privateRead(path, max = 2097152) {
  const fd = openSync(path, constants.O_RDONLY | constants.O_NOFOLLOW);
  try {
    const st = fstatSync(fd);
    if (!st.isFile() || st.isSymbolicLink() || st.uid !== process.getuid() || st.mode & 0o077 || st.size > max) throw Error('Private artifact permissions/size');
    return readFileSync(fd);
  } finally { closeSync(fd); }
}
export function exclusive(path, value) {
  const bytes = Buffer.from(typeof value === 'string' ? value : canonical(value)+'\n');
  const fd = openSync(path, constants.O_WRONLY|constants.O_CREAT|constants.O_EXCL|constants.O_NOFOLLOW,0o600);
  try { let at=0; while(at<bytes.length) at+=writeSync(fd,bytes,at,bytes.length-at); fsyncSync(fd); } finally { closeSync(fd); }
  const parent = openSync(join(path,'..'),constants.O_RDONLY); try { fsyncSync(parent); } finally { closeSync(parent); }
}
export function validateManifest(m) {
  const reliable=m?.schema==='cinder-native-staged-delivery-v1',
    staged=reliable||m?.schema==='cinder-native-staged-bootstrap-v1';
  if(!staged&&m?.test_capital_only!==undefined)throw Error('Mixed diagnostic scenarios');
  if((!reliable&&m?.faucet_mode!==undefined)||(reliable&&!['owner-via-broker','broker-direct'].includes(m.faucet_mode)))throw Error('Mixed faucet scenario');
  if ((!staged && m?.schema !== 'cinder-native-qualification-v1') || m.cluster !== 'devnet' || m.aws !== false
    || m.shipping !== false || m.native_origin !== 'https://test-api.pacifica.fi'
    || m.wss_origin !== 'wss://test-ws.pacifica.fi/ws' || m.genesis !== 'EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG'
    || canonical(m.limits) !== canonical(reliable?DELIVERY_LIMITS:staged?STAGED_LIMITS:LIMITS) || m.bootstrap_exception !== !staged
    || m.lost_native_reply !== !staged || (staged && m.test_capital_only!==true)
    || !/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(m.withdraw_uuid)
    || !/^[0-9a-f]{64}$/.test(m.sources) || !/^[0-9a-f]{64}$/.test(m.tls_roots)
    || m.node !== '24.21.0' || m.rpc_origin !== 'https://devnet.helius-rpc.com') throw Error('Unapproved manifest shape');
  if (new Set([m.owner,m.broker,m.sponsor]).size !== 3 || [m.owner,m.broker,m.sponsor].some(x=>typeof x!=='string'||!x)) throw Error('Distinct identities required');
  return sha(canonical(m));
}
export function assertApproval(m, approval, now) {
  if (approval?.manifest !== validateManifest(m) || approval?.approved !== true || typeof approval?.reference !== 'string'
    || !approval.reference.trim() || !Number.isSafeInteger(approval.not_after) || now >= approval.not_after
    || approval.not_after-now > 86400000) throw Error('Exact current execution approval required');
}

// One process owns this append-only file. Torn writes/conflicts stop; no repair,
// automatic live restart, replacement operation or reset of consumed budgets.
export class Journal {
  constructor(directory, manifest, now) {
    const st=lstatSync(directory);
    if(!st.isDirectory()||st.isSymbolicLink()||st.uid!==process.getuid()||st.mode&0o077)throw Error('Private run directory required');
    this.directory=directory;this.manifest=validateManifest(manifest);this.now=now;
    this.rows=[];this.head='0'.repeat(64);this.last=0;
    this.file=join(directory,'journal.jsonl');
    try {
      const text=privateRead(this.file,16777216).toString();
      if(text && !text.endsWith('\n')) throw Error('Torn journal; reconcile manually');
      for(const line of text.trimEnd().split('\n').filter(Boolean)) {
        const {hash,...row}=JSON.parse(line);
        if(row.previous!==this.head || row.manifest!==this.manifest || row.index!==this.rows.length
          || !Number.isSafeInteger(row.at) || row.at<this.last || sha(canonical(row))!==hash) throw Error('Journal integrity');
        this.rows.push(row);this.head=hash;this.last=row.at;
      }
    } catch(e) { if(e.code!=='ENOENT') throw e; }
  }
  append(kind, data) {
    const at=this.now();if(!Number.isSafeInteger(at)||at<this.last)throw Error('Clock moved backwards');
    const row={index:this.rows.length,manifest:this.manifest,previous:this.head,at,kind,data};
    const hash=sha(canonical(row)), bytes=Buffer.from(canonical({...row,hash})+'\n');
    const fd=openSync(this.file,constants.O_WRONLY|constants.O_APPEND|constants.O_CREAT|constants.O_NOFOLLOW,0o600);
    try {
      const st=fstatSync(fd);if(!st.isFile()||st.uid!==process.getuid()||st.mode&0o077)throw Error('Private journal permissions');
      let offset=0;while(offset<bytes.length)offset+=writeSync(fd,bytes,offset,bytes.length-offset);fsyncSync(fd);
    } finally {closeSync(fd);}
    const parent=openSync(this.directory,constants.O_RDONLY);try{fsyncSync(parent);}finally{closeSync(parent);}
    this.rows.push(row);this.head=hash;this.last=at;return row;
  }
  begin() {
    if(this.rows.length)throw Error('Invocation already started; read-only reconciliation only');
    return this.append('start',{});
  }
  consume(kind, data={}, cleanup=false) {
    const started=this.rows.find(r=>r.kind==='start');if(!started)throw Error('Invocation not started');
    const elapsed=this.now()-started.at;
    if(elapsed<0||elapsed>=LIMITS.duration_ms)throw Error('Invocation deadline');
    if(!['http','rpc','wss'].includes(kind))throw Error('Unknown budget');
    const used=this.rows.filter(r=>r.kind===kind).length;
    const reserve=cleanup?0:({http:40,rpc:80,wss:2}[kind]);
    if(used>=LIMITS[kind]-reserve)throw Error('Invocation budget');
    return this.append(kind,data);
  }
  once(identity, data) {
    if(this.rows.some(r=>r.kind==='expose'&&r.data.identity===identity))throw Error('Original operation already exposed; no resend');
    return this.append('expose',{identity,...data});
  }
}
export function decodeResponse(bytes) {
  if(bytes.length>LIMITS.payload_bytes)throw Error('Response bound');
  return JSON.parse(Buffer.from(bytes).toString('utf8'));
}
// Exposure is durable before I/O. A thrown transport/body/audit error means the
// ORIGINAL native operation may have executed, never that it was rejected.
export async function originalNativeReply(journal, identity, exposure, receive, retain, lose=false) {
  journal.once(identity,exposure);
  try {
    const result=await receive();
    await retain(result);
    if(lose){journal.append('lost-native-reply',{identity,controller_accepted:false});return null;}
    journal.append('native-reply',{identity,status:result.status,success:result.body.success===true});
    return result;
  } catch(error) {
    // Do not copy arbitrary upstream exception text (URLs/wires may be private).
    journal.append('native-reply-unknown',{identity,exposed:true,controller_accepted:false});
    throw error;
  }
}
export async function boundedBody(response) {
  const chunks=[];let n=0;
  if(!response.body)throw Error('Missing response body');
  try { for await(const b of response.body){n+=b.length;if(n>LIMITS.payload_bytes)throw Error('Response bound');chunks.push(b);} }
  catch(e){await response.body.cancel().catch(()=>{});throw e;}
  return Buffer.concat(chunks,n);
}
export function nativeAck(value) {
  const d=value?.data;
  if(value?.success!==true||value.error!=null||value.code!=null||!d||!Number.isSafeInteger(d.batch_nonce)||d.batch_nonce<0
    ||atoms(d.requested_amount)!==20000000n||atoms(d.fee_amount)>2000000n)throw Error('Withdrawal ACK mismatch');
  return {batch:d.batch_nonce,gross:atoms(d.requested_amount).toString(),fee:atoms(d.fee_amount).toString()};
}
export function setupObservations(settings, loan) {
  if(settings?.success!==true||settings.error!=null||settings.code!=null||settings.data?.auto_lend_disabled!==true
    ||loan?.success!==true||loan.error!=null||loan.code!=null
    ||!Number.isSafeInteger(loan.data?.updated_at)||loan.data.updated_at<=0||atoms(loan.data.borrowed)!==0n
    ||atoms(loan.data.pending_interest)!==0n) return {status:'unresolved',shipping_complete:false};
  return {status:'observed-disabled-no-debt',shipping_complete:false};
}
const success=r=>r?.status===200&&r.body?.success===true&&r.body.error==null&&r.body.code==null;
const absent=r=>r?.status===404&&r.body?.success===false&&r.body.data===null&&r.body.code===404;
// Missing cache means wait, never zero debt. Successful but non-idle observations
// fail rather than normalize an unexpected exposure into a setup prerequisite.
export function stagedInitialization(account,loan,now,expected='20000000') {
  if(absent(account)||absent(loan)) {
    if(![account,loan].every(r=>absent(r)||success(r)))throw Error('Initialization response unknown');
    return false;
  }
  if(!success(account)||!success(loan)||!Number.isSafeInteger(now))throw Error('Initialization response unknown');
  const a=account.body.data,l=loan.body.data;
  if(!a||!l||atoms(a.balance)!==BigInt(expected)||atoms(a.pending_balance)!==0n
    ||atoms(a.account_equity)!==BigInt(expected)||atoms(a.total_margin_used)!==0n||atoms(a.pending_interest)!==0n
    ||atoms(a.spot_market_value)!==0n||atoms(a.spot_collateral)!==0n
    ||a.positions_count!==0||a.orders_count!==0||a.stop_orders_count!==0||!Array.isArray(a.spot_balances)||a.spot_balances.length
    ||atoms(l.borrowed)!==0n||atoms(l.pending_interest)!==0n||!Array.isArray(l.spot_balances)||l.spot_balances.length
    ||!Number.isSafeInteger(l.updated_at)||l.updated_at<=0||l.updated_at>now+5000||now-l.updated_at>180000)
    throw Error('Initialized account is not idle/no-debt');
  return true;
}
export function stagedIdle(settings,loan,account,positions,orders,now,expected='20000000') {
  if(![settings,loan,account,positions,orders].every(success)
    ||setupObservations(settings.body,loan.body).status!=='observed-disabled-no-debt'
    ||!Array.isArray(settings.body.data.margin_settings)||settings.body.data.margin_settings.length
    ||!Array.isArray(settings.body.data.spot_settings)||settings.body.data.spot_settings.length
    ||![positions,orders].every(r=>Array.isArray(r.body.data)&&r.body.data.length===0))throw Error('Staged idle setup incomplete');
  // Reuse the same full checks at closeout, with a zero-cash empty baseline.
  if(!stagedInitialization(account,loan,now,expected))throw Error('Staged idle setup incomplete');
  return {status:'observed-disabled-no-debt',sequential:true,shipping_complete:false};
}
export function stagedDepositHistory(deposit,balance,signature) {
  for(const r of [deposit,balance])if(!success(r)||!Array.isArray(r.body.data)||r.body.data.length!==1
    ||r.body.has_more!==false||r.body.next_cursor!=null)throw Error('Staged history incomplete/conflicting');
  const d=deposit.body.data[0],b=balance.body.data[0];
  if(d.transaction_id!==signature||atoms(d.amount)!==20000000n||!Number.isSafeInteger(d.created_at)||d.created_at<=0
    ||!['deposit','deposit_release'].includes(b.event_type)||atoms(b.amount)!==20000000n
    ||atoms(b.balance)!==20000000n||atoms(b.pending_balance)!==0n
    ||!Number.isSafeInteger(b.created_at)||b.created_at<d.created_at)throw Error('Staged original credit mismatch');
  return {status:'original-signature-full-credit-observed',gross:'20000000',financial_credit:false,source_cut:null};
}
export function transfers(value, account) {
  if(value?.channel!=='account_transfers')return [];
  // The documented live envelope carries one object. Retain bounded diagnostic
  // batch fixtures too; neither shape establishes replay or a financial cut.
  const rows=Array.isArray(value.data)?value.data:[value.data];
  if(rows.some(r=>!r||typeof r!=='object'||Array.isArray(r)))throw Error('Transfer envelope');
  return rows.map(r=>{
    if(r.u!==account||r.a!=='USDC'||!['deposit','withdrawal_pending','withdrawal_confirmed'].includes(r.e)
      ||!Number.isSafeInteger(r.t)||r.t<0)throw Error('Transfer schema/account');
    atoms(r.am);
    if(r.tx!==undefined&&(typeof r.tx!=='string'||!/^[1-9A-HJ-NP-Za-km-z]{64,88}$/.test(r.tx)))throw Error('Transfer signature');
    if(r.bn!==undefined&&(!Number.isSafeInteger(r.bn)||r.bn<0))throw Error('Transfer batch');
    if(r.ra!==undefined)atoms(r.ra);if(r.f!==undefined)atoms(r.f);
    return structuredClone(r);
  });
}
export function emptyBalanceBaseline(account, history) {
  if(account?.status!==404||account.body?.success!==false||history?.status!==200
    ||history.body?.success!==true||!Array.isArray(history.body.data)
    ||history.body.data.length!==0||history.body.has_more!==false)throw Error('Native baseline not established');
  return {fresh:true,empty_balance_page_observed:true,source_cut:null};
}
export function flatAccountObservation(account) {
  return account?.status===200&&account.body?.success===true
    &&atoms(account.body.data?.balance)===0n&&atoms(account.body.data?.pending_balance)===0n;
}
export function depositLink(rows, signature, account) {
  const found=rows.filter(r=>r.e==='deposit'&&r.u===account&&r.tx===signature);
  if(!found.length)return {status:'unresolved',reason:'original-signature-observation-missing',financial_credit:false};
  if(found.some(r=>atoms(r.am)!==20000000n)||new Set(found.map(canonical)).size!==1)throw Error('Deposit conflict');
  return {status:'exact-signature-observed',financial_credit:false,source_cut:null};
}
export function tokenDelta(tx,status,{signature,ata,mint,owner}) {
  if(status?.confirmationStatus!=='finalized'||status.err!==null||tx?.meta?.err!==null
    ||tx.transaction?.signatures?.[0]!==signature||!Number.isSafeInteger(tx.slot))throw Error('Payment not finalized');
  const keys=tx.transaction.message.accountKeys.map(k=>typeof k==='string'?k:k.pubkey),index=keys.indexOf(ata);
  if(index<0||keys.lastIndexOf(ata)!==index)throw Error('Payment recipient');
  const balance=(rows,empty)=>{
    if(!Array.isArray(rows))throw Error('Missing token balances');
    const found=rows.filter(r=>r.accountIndex===index);if(empty&&found.length===0)return 0n;
    if(found.length!==1)throw Error('Ambiguous balance');const r=found[0];
    if(r.mint!==mint||r.owner!==owner||r.uiTokenAmount?.decimals!==6||! /^(0|[1-9][0-9]*)$/.test(r.uiTokenAmount.amount))throw Error('Payment asset/owner');
    return BigInt(r.uiTokenAmount.amount);
  };
  return balance(tx.meta.postTokenBalances,false)-balance(tx.meta.preTokenBalances,true);
}
export function paymentDelta(tx,status,binding) {
  const delta=tokenDelta(tx,status,binding);
  if(delta<=0n||delta>20000000n)throw Error('Payment delta');return delta;
}
export function withdrawalLink(rows, ack, controllerAck, account) {
  const found=rows.filter(r=>r.e==='withdrawal_confirmed'&&r.u===account&&r.bn===ack.batch);
  if(!found.length)return {status:'unresolved',reason:'confirmed-batch-observation-missing',source_cut:null};
  if(new Set(found.map(canonical)).size!==1)throw Error('Withdrawal conflict');
  const r=found[0];
  if(!r.tx||atoms(r.am)+BigInt(ack.fee)!==BigInt(ack.gross)
    ||r.ra!==undefined&&atoms(r.ra)!==BigInt(ack.gross)||r.f!==undefined&&atoms(r.f)!==BigInt(ack.fee))throw Error('Withdrawal conservation');
  return {status:'audit-linked-payment-candidate',signature:r.tx,net:atoms(r.am).toString(),source_cut:null,
    lost_reply_reconciliation:controllerAck?'not-injected':'unresolved-native-UUID-query-not-established',financial_completion:false};
}
