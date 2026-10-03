/** Independent recovery client: no Cinder API, venue credential or wallet-secret
 * derivation. RPC authenticity/finality is a trusted source port, not a Merkle
 * proof. Public claim transactions necessarily reveal the claimed amount. */
import { constants, createHash, createPublicKey, createDecipheriv, privateDecrypt, verify, type KeyObject } from 'node:crypto';
import { type AccountInfo, PublicKey, SystemProgram, type TransactionInstruction } from '@solana/web3.js';
import type { Program, IdlAccounts } from '@anchor-lang/core';
import type { CinderVault } from './cinder_vault.ts';
import { customerAddress, identity, u64, vaultAddresses } from './index.ts';
import { recoveryAddress, recoveryReceiptAddress, recoveryClaimWire, recoveryContextHash, recoveryLeaf,
  recoveryStatementWire, verifyRecoveryProof, type RecoveryClaim, type RecoveryStatement, type RecoveryContext } from './recovery.ts';

const TOKEN=new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
type Accounts=IdlAccounts<CinderVault>;
export const MAX_PACKAGE=32_768;
const hash=(b:Uint8Array)=>createHash('sha256').update(b).digest();
const bad=()=>new Error('Recovery evidence unavailable or invalid');
function object(v:unknown,keys:readonly string[]):Record<string,unknown> {
  if (!v||typeof v!=='object'||Array.isArray(v)||Object.keys(v).sort().join('\0')!==[...keys].sort().join('\0')) throw bad();
  return v as Record<string,unknown>;
}
function bytes(v:unknown,n=32,nonzero=true):Buffer {
  if (!Array.isArray(v)||v.length!==n||v.some(x=>!Number.isInteger(x)||x<0||x>255)) throw bad();
  const b=Buffer.from(v);if(nonzero&&!b.some(x=>x!==0))throw bad();return b;
}
function number(v:unknown):bigint {
  if(typeof v!=='string'||!/^(0|[1-9][0-9]{0,19})$/.test(v))throw bad();const n=BigInt(v);u64(n);return n;
}
function integer(v:unknown,max:number):number {if(typeof v!=='number'||!Number.isInteger(v)||v<0||v>max)throw bad();return v;}
function json(b:Uint8Array,max=MAX_PACKAGE):unknown {
  if(b.length===0||b.length>max)throw bad();
  const text=new TextDecoder('utf-8',{fatal:true}).decode(b),parsed:unknown=JSON.parse(text);
  // The producer signs/emits canonical compact JSON. Reject duplicate fields,
  // lossy UTF-8 and alternate numeric encodings before interpreting evidence.
  if(JSON.stringify(parsed)!==text)throw bad();return parsed;
}
const statementKeys=['program','config','domain','pool','mint','decimals','epoch','root','tree_size','total','journal_cutoff','journal_hash','evidence_hash','policy_hash','normal_paid','funding_sequence'];
function statement(v:unknown):{context:RecoveryContext;statement:RecoveryStatement} {
  const s=object(v,statementKeys);
  const domain=bytes(s.domain),pool=bytes(s.pool);
  return {context:{program:new PublicKey(bytes(s.program)),config:new PublicKey(bytes(s.config)),domain,pool,
    mint:new PublicKey(bytes(s.mint)),decimals:integer(s.decimals,18)},
    statement:{domain,epoch:number(s.epoch),root:bytes(s.root),treeSize:integer(s.tree_size,65_536),total:number(s.total),
      journalCutoff:number(s.journal_cutoff),journalHash:bytes(s.journal_hash),evidenceHash:bytes(s.evidence_hash),
      policyHash:bytes(s.policy_hash),normalPaid:number(s.normal_paid),fundingSequence:number(s.funding_sequence),
      qualification:{unresolvedOperations:0n,outstandingReservations:0n,unresolvedInputs:0n,venueExposureZero:true,claimsAvailable:true}}};
}
/** Wallet authorizes a dedicated encryption SPKI through the attested channel.
 * It is NOT a wallet signing-key derivation. Keep the decryption key backed up. */
export function recoveryKeyAuthorization(domain:Uint8Array,pool:Uint8Array,spki:Uint8Array):Buffer {
  identity(domain);identity(pool);const p=createPublicKey({key:Buffer.from(spki),format:'der',type:'spki'});
  if(p.asymmetricKeyType!=='rsa'||p.asymmetricKeyDetails?.modulusLength!==3072||p.asymmetricKeyDetails.publicExponent!==65537n
    ||!p.export({format:'der',type:'spki'}).equals(Buffer.from(spki)))throw bad();
  return hash(Buffer.concat([Buffer.from('CINDER_RECOVERY_KEY_V1'),Buffer.from(domain),Buffer.from(pool),hash(spki)]));
}
export interface Manifest {
  context:RecoveryContext;statement:RecoveryStatement;replicas:Buffer[];entries:ReadonlyMap<string,Buffer>;
}
/** Signature belongs to the expected governed publisher read from independently
 * authenticated chain config; never accept a key from the manifest itself. */
export function verifyManifest(wire:Uint8Array,signature:Uint8Array,governance:PublicKey):Manifest {
  try {
    const key=createPublicKey({key:Buffer.concat([Buffer.from('302a300506032b6570032100','hex'),governance.toBuffer()]),format:'der',type:'spki'});
    if(signature.length!==64||!verify(null,wire,key,signature))throw bad();
    const m=object(json(wire,24_000_000),['schema','statement','replicas','entries']);
    if(m.schema!=='cinder-recovery-manifest-v1'||!Array.isArray(m.replicas)||m.replicas.length!==2
      ||!Array.isArray(m.entries)||m.entries.length===0||m.entries.length>65_536)throw bad();
    const replicas=m.replicas.map(x=>bytes(x));if(replicas[0]!.equals(replicas[1]!))throw bad();
    const entries=new Map<string,Buffer>();let previous='';
    for(const value of m.entries){const e=object(value,['locator','digest']),locator=bytes(e.locator).toString('hex');
      if(locator<=previous)throw bad();previous=locator;entries.set(locator,bytes(e.digest));}
    const parsed=statement(m.statement);recoveryContextHash(parsed.context,parsed.statement);
    if(parsed.statement.treeSize!==entries.size)throw bad();
    return {...parsed,replicas,entries};
  } catch {throw bad();}
}
export interface Kit {context:RecoveryContext;statement:RecoveryStatement;claim:RecoveryClaim;proof:Buffer[]}
/** Recipient-only decryption. The caller's dedicated key is never sent to a
 * service; corrupted envelopes, wrong keys/locators/context are redacted failures. */
export function decryptKit(wire:Uint8Array,locator:Uint8Array,key:KeyObject,manifest:Manifest):Kit {
  let aes:Buffer|undefined,plain:Buffer|undefined;
  try {
    const expected=manifest.entries.get(Buffer.from(locator).toString('hex'));
    if(!expected||!hash(wire).equals(expected))throw bad();
    const e=object(json(wire),['schema','locator','context','root','wrapped_key','nonce','ciphertext','tag']);
    const ctx=recoveryContextHash(manifest.context,manifest.statement),root=Buffer.from(manifest.statement.root);
    if(e.schema!=='cinder-recovery-envelope-v1'||!bytes(e.locator).equals(Buffer.from(locator))
      ||!bytes(e.context).equals(ctx)||!bytes(e.root).equals(root)||!Array.isArray(e.ciphertext)||e.ciphertext.length>MAX_PACKAGE)throw bad();
    aes=privateDecrypt({key,oaepHash:'sha256',padding:constants.RSA_PKCS1_OAEP_PADDING},bytes(e.wrapped_key,384));
    if(aes.length!==32)throw bad();
    const d=createDecipheriv('aes-256-gcm',aes,bytes(e.nonce,12));
    d.setAAD(Buffer.concat([Buffer.from('CINDER_RECOVERY_ENVELOPE_V1'),ctx,root,Buffer.from(locator)]));d.setAuthTag(bytes(e.tag,16));
    plain=Buffer.concat([d.update(bytes(e.ciphertext,e.ciphertext.length)),d.final()]);
    const k=object(json(plain),['schema','statement','index','owner','destination','amount','paid_base','payout_sequence_base','claim_id','salt','proof']);
    if(k.schema!=='cinder-recovery-kit-v1'||!Array.isArray(k.proof)||k.proof.length>16)throw bad();
    const parsed=statement(k.statement),kctx=recoveryContextHash(parsed.context,parsed.statement);
    if(!kctx.equals(ctx)||!Buffer.from(parsed.statement.root).equals(root))throw bad();
    const claim:RecoveryClaim={index:integer(k.index,65_535),owner:new PublicKey(bytes(k.owner)),destination:new PublicKey(bytes(k.destination)),
      amount:number(k.amount),paidBase:number(k.paid_base),payoutSequenceBase:number(k.payout_sequence_base),claimId:bytes(k.claim_id),salt:bytes(k.salt)};
    const proof=k.proof.map(p=>bytes(p));
    if(!verifyRecoveryProof(root,recoveryLeaf(ctx,claim),claim.index,manifest.statement.treeSize,proof))throw bad();
    return {...parsed,claim,proof};
  } catch {throw bad();} finally {aes?.fill(0);plain?.fill(0);}
}
/** Fail over only GET reads. No API credentials, money-moving retries or redirects.
 * Retrieval endpoints/failure-domain identities are separately governed. */
export async function retrieveKit(locator:Uint8Array,manifest:Manifest,readers:readonly ((locator:Uint8Array)=>Promise<Uint8Array>)[]):Promise<Uint8Array> {
  if(readers.length!==2)throw bad();const expected=manifest.entries.get(Buffer.from(locator).toString('hex'));if(!expected)throw bad();
  for(const read of readers){try {const b=await read(Uint8Array.from(locator));if(b.length<=MAX_PACKAGE&&hash(b).equals(expected))return Uint8Array.from(b);}catch{/* bounded independently supplied reader */}}
  throw bad();
}
export interface ExpectedCustody {
  program:PublicKey;domain:Uint8Array;pool:Uint8Array;mint:PublicKey;decimals:number;
  governance:PublicKey;funds:PublicKey;recovery:PublicKey;broker:PublicKey;brokerTokens:PublicKey;
  network:Uint8Array;customers:readonly {wallet:PublicKey;tokens:PublicKey}[];
}
export interface Snapshot {
  /** Source port must authenticate the expected genesis, request set and FINALIZED
   * context. An attacker-controlled JSON flag is not a finality certificate. */
  network:Uint8Array;slot:bigint;accounts:ReadonlyMap<string,AccountInfo<Buffer>>;
}
/** Strict public classic SPL parser; no delegate/close/native/Token-2022/frozen
 * account can masquerade as available recovery liquidity. */
function token(s:Snapshot,address:PublicKey,mint:PublicKey,owner:PublicKey):bigint {
  const a=s.accounts.get(address.toBase58());if(!a||a.executable||!a.owner.equals(TOKEN)||a.data.length!==165)throw bad();
  const b=a.data;if(!b.subarray(0,32).equals(mint.toBuffer())||!b.subarray(32,64).equals(owner.toBuffer())||b[108]!==1
    ||b.readUInt32LE(72)!==0||b.readUInt32LE(109)!==0||b.readBigUInt64LE(121)!==0n||b.readUInt32LE(129)!==0)throw bad();
  return b.readBigUInt64LE(64);
}
function decode<T>(p:Program<CinderVault>,s:Snapshot,address:PublicKey,name:string,size:number):T {
  const a=s.accounts.get(address.toBase58());if(!a||a.executable||!a.owner.equals(p.programId)||a.data.length!==size)throw bad();
  return p.coder.accounts.decode(name,a.data) as T;
}
/** Coherent raw-account observation for P16 matching. The return is a trusted
 * port input, not validator-signed proof; live RPC authentication remains G01. */
export function custodySnapshot(p:Program<CinderVault>,s:Snapshot,e:ExpectedCustody) {
  try {
    if(!p.programId.equals(e.program)||s.slot<=0n||s.slot>(1n<<64n)-1n||!Buffer.from(s.network).equals(Buffer.from(e.network))
      ||e.network.length!==32||!e.network.some(x=>x)||e.customers.length===0||e.customers.length>65_536)throw bad();
    const {config,vault}=vaultAddresses(e.program,e.domain,e.pool,e.mint);
    const loader=new PublicKey('BPFLoaderUpgradeab1e11111111111111111111111');
    const programData=PublicKey.findProgramAddressSync([e.program.toBuffer()],loader)[0];
    const executable=s.accounts.get(e.program.toBase58()),code=s.accounts.get(programData.toBase58());
    if(!executable?.executable||!executable.owner.equals(loader)||executable.data.length!==36
      ||executable.data.readUInt32LE(0)!==2||!executable.data.subarray(4).equals(programData.toBuffer())
      ||!code||code.executable||!code.owner.equals(loader)||code.data.length<45||code.data.readUInt32LE(0)!==3
      ||code.data[12]!==1||!code.data.subarray(13,45).equals(e.governance.toBuffer()))throw bad();
    const c=decode<Accounts['vaultConfig']>(p,s,config,'vaultConfig',333);
    const bump=PublicKey.findProgramAddressSync([Buffer.from('cinder_vault'),Buffer.from(e.domain),Buffer.from(e.pool),e.mint.toBuffer()],e.program)[1];
    const vb=PublicKey.findProgramAddressSync([Buffer.from('tokens'),config.toBuffer()],e.program)[1];
    if(c.schema!==1||c.bump!==bump||c.vaultBump!==vb||c.decimals!==e.decimals||!Buffer.from(c.domain).equals(Buffer.from(e.domain))
      ||!Buffer.from(c.pool).equals(Buffer.from(e.pool))||!c.mint.equals(e.mint)||!c.governance.equals(e.governance)
      ||!c.funds.equals(e.funds)||!c.recovery.equals(e.recovery)||!c.broker.equals(e.broker)||!c.brokerTokens.equals(e.brokerTokens)
      ||new Set([e.governance,e.funds,e.recovery,e.broker].map(x=>x.toBase58())).size!==4)throw bad();
    const m=s.accounts.get(e.mint.toBase58());if(!m||m.executable||!m.owner.equals(TOKEN)||m.data.length!==82||m.data[45]!==1||m.data[44]!==e.decimals)throw bad();
    token(s,e.brokerTokens,e.mint,e.broker);const vaultAmount=token(s,vault,e.mint,config);
    const seen=new Set<string>(),dest=new Set<string>();
    const customers=e.customers.map(o=>{
      if(seen.has(o.wallet.toBase58())||dest.has(o.tokens.toBase58()))throw bad();seen.add(o.wallet.toBase58());dest.add(o.tokens.toBase58());
      token(s,o.tokens,e.mint,o.wallet);const addr=customerAddress(e.program,config,o.wallet);
      const cc=decode<Accounts['customerCounter']>(p,s,addr,'customerCounter',97);
      const cb=PublicKey.findProgramAddressSync([Buffer.from('customer'),config.toBuffer(),o.wallet.toBuffer()],e.program)[1];
      if(!cc.config.equals(config)||!cc.owner.equals(o.wallet)||cc.bump!==cb)throw bad();
      return {wallet:Array.from(o.wallet.toBytes()),tokens:Array.from(o.tokens.toBytes()),paid:BigInt(cc.paid.toString()),payout_sequence:BigInt(cc.payoutSequence.toString())};
    });
    const b=(v:PublicKey|Uint8Array)=>Array.from(v instanceof PublicKey?v.toBytes():v);
    return {network:b(e.network),program:b(e.program),config:b(config),vault:b(vault),mint:b(e.mint),domain:b(e.domain),pool:b(e.pool),
      broker:b(e.broker),broker_tokens:b(e.brokerTokens),decimals:e.decimals,epoch:BigInt(c.epoch.toString()),mode:c.mode,
      normal_paid:BigInt(c.paid.toString()),funding_sequence:BigInt(c.fundingSequence.toString()),vault_amount:vaultAmount,finalized_slot:s.slot,customers};
  } catch {throw bad();}
}
/** Exact JSON u64 numbers for Rust's typed observation port, WITHOUT conversion
 * through JavaScript Number. This encoder never accepts arbitrary objects. */
export function custodyWire(value:ReturnType<typeof custodySnapshot>):Buffer {
  function encode(v:unknown):string {
    if(typeof v==='bigint'){u64(v);return v.toString();}
    if(typeof v==='number'){if(!Number.isSafeInteger(v)||v<0)throw bad();return v.toString();}
    if(Array.isArray(v))return `[${v.map(encode).join(',')}]`;
    if(v&&typeof v==='object')return `{${Object.entries(v).map(([k,x])=>`${JSON.stringify(k)}:${encode(x)}`).join(',')}}`;
    throw bad();
  }
  return Buffer.from(encode(value));
}
/** Build only after independent active-root/config/counter/remaining-backing
 * validation; signing stays in the caller wallet. The program repeats checks. */
export async function independentClaim(p:Program<CinderVault>,s:Snapshot,e:ExpectedCustody,k:Kit):Promise<TransactionInstruction> {
  const observed=custodySnapshot(p,s,e),c=k.context,st=k.statement,cl=k.claim;
  if(!c.program.equals(e.program)||!c.config.equals(new PublicKey(observed.config))||!c.mint.equals(e.mint)
    ||!Buffer.from(c.domain).equals(Buffer.from(e.domain))||!Buffer.from(c.pool).equals(Buffer.from(e.pool))||c.decimals!==e.decimals
    ||observed.mode!==3||BigInt(observed.epoch)!==st.epoch)throw bad();
  const own=e.customers.find(o=>o.wallet.equals(cl.owner)&&o.tokens.equals(cl.destination));if(!own)throw bad();
  const counter=decode<Accounts['customerCounter']>(p,s,customerAddress(e.program,c.config,cl.owner),'customerCounter',97);
  if(BigInt(counter.paid.toString())!==cl.paidBase||BigInt(counter.payoutSequence.toString())!==cl.payoutSequenceBase)throw bad();
  const rootAddress=recoveryAddress(e.program,c.config);
  // 8 + config32 + bump1 + statement(230) + context32 + remaining8 + claimed4.
  const root=decode<Accounts['recoveryEpoch']>(p,s,rootAddress,'recoveryEpoch',315);
  const rootBump=PublicKey.findProgramAddressSync([Buffer.from('recovery'),c.config.toBuffer()],e.program)[1];
  const live=root.statement,context=recoveryContextHash(c,st);
  const expect=recoveryStatementWire(st);
  const eq=(a:unknown,b:unknown)=>JSON.stringify(a)===JSON.stringify(b);
  for(const field of ['domain','root','journalHash','evidenceHash','policyHash'] as const)if(!eq(live[field],expect[field]))throw bad();
  for(const field of ['epoch','total','journalCutoff','normalPaid','fundingSequence'] as const)if(live[field].toString()!==expect[field].toString())throw bad();
  if(root.bump!==rootBump||!root.config.equals(c.config)||!eq(root.contextHash,Array.from(context))||live.treeSize!==st.treeSize
    ||live.qualification.unresolvedOperations.toString()!=='0'||live.qualification.outstandingReservations.toString()!=='0'
    ||live.qualification.unresolvedInputs.toString()!=='0'||!live.qualification.venueExposureZero||!live.qualification.claimsAvailable
    ||BigInt(root.remaining.toString())<cl.amount||observed.vault_amount<BigInt(root.remaining.toString())
    ||s.accounts.has(recoveryReceiptAddress(e.program,c.config,cl.owner).toBase58())
    ||!verifyRecoveryProof(st.root,recoveryLeaf(context,cl),cl.index,st.treeSize,k.proof))throw bad();
  return p.methods.claimRecovery(identity(st.domain),u64(st.epoch),recoveryClaimWire(cl),k.proof.map(identity)).accountsStrict({
    config:c.config,recoveryEpoch:rootAddress,customer:customerAddress(e.program,c.config,cl.owner),owner:cl.owner,
    vault:new PublicKey(observed.vault),destination:cl.destination,mint:e.mint,
    claimReceipt:recoveryReceiptAddress(e.program,c.config,cl.owner),tokenProgram:TOKEN,systemProgram:SystemProgram.programId}).instruction();
}
