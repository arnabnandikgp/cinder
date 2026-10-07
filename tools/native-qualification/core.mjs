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
  if (m?.schema !== 'cinder-native-qualification-v1' || m.cluster !== 'devnet' || m.aws !== false
    || m.shipping !== false || m.native_origin !== 'https://test-api.pacifica.fi'
    || m.wss_origin !== 'wss://test-ws.pacifica.fi/ws' || m.genesis !== 'EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG'
    || canonical(m.limits) !== canonical(LIMITS) || m.bootstrap_exception !== true
    || m.lost_native_reply !== true || !/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(m.withdraw_uuid)
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
export async function boundedBody(response) {
  const chunks=[];let n=0;
  if(!response.body)throw Error('Missing response body');
  try { for await(const b of response.body){n+=b.length;if(n>LIMITS.payload_bytes)throw Error('Response bound');chunks.push(b);} }
  catch(e){await response.body.cancel().catch(()=>{});throw e;}
  return Buffer.concat(chunks,n);
}
export function nativeAck(value) {
  const d=value?.data;
  if(value?.success!==true||!d||!Number.isSafeInteger(d.batch_nonce)||d.batch_nonce<0
    ||atoms(d.requested_amount)!==20000000n||atoms(d.fee_amount)>2000000n)throw Error('Withdrawal ACK mismatch');
  return {batch:d.batch_nonce,gross:atoms(d.requested_amount).toString(),fee:atoms(d.fee_amount).toString()};
}
export function setupObservations(settings, loan) {
  if(settings?.success!==true||settings.data?.auto_lend_disabled!==true||loan?.success!==true
    ||!Number.isSafeInteger(loan.data?.updated_at)||atoms(loan.data.borrowed)!==0n
    ||atoms(loan.data.pending_interest)!==0n) return {status:'unresolved',shipping_complete:false};
  return {status:'observed-disabled-no-debt',shipping_complete:false};
}
export function transfers(value, account) {
  if(value?.channel!=='account_transfers'||!Array.isArray(value.data))return [];
  return value.data.map(r=>{
    if(r.u!==account||r.a!=='USDC'||!['deposit','withdrawal_pending','withdrawal_confirmed'].includes(r.e)
      ||!Number.isSafeInteger(r.t)||r.t<0)throw Error('Transfer schema/account');
    atoms(r.am);
    if(r.tx!==undefined&&(typeof r.tx!=='string'||!/^[1-9A-HJ-NP-Za-km-z]{64,88}$/.test(r.tx)))throw Error('Transfer signature');
    if(r.bn!==undefined&&(!Number.isSafeInteger(r.bn)||r.bn<0))throw Error('Transfer batch');
    if(r.ra!==undefined)atoms(r.ra);if(r.f!==undefined)atoms(r.f);
    return structuredClone(r);
  });
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
