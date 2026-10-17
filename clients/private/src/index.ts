// Exact private application protocol. No network defaults, RPC, native signer or retries.
import { privateIterable } from './private-iteration.ts';
export type Id = Uint8Array;
export interface Domain { network: Id; deployment: Id }
export interface Grant {
  key: Id; methods: number; market: Id; maximumLots: bigint; maximumFee: bigint;
  maximumOrders: bigint; expiresAt: bigint;
}
export const READ = 1, TRADE = 2, CANCEL = 4;
export const READ_FAMILIES = ['account','positions','operations','openOrders','fills','funding','movements','grants','markets','updates','ingress'] as const;
export type ReadFamily = typeof READ_FAMILIES[number];
export interface ReadQuery { family: ReadFamily; cursor?: Uint8Array; limit: number }
export type Command =
  | { kind: 'read'; query: ReadQuery }
  | { kind: 'view' }
  | { kind: 'operation'; target: Id }
  | { kind: 'order'; market: Id; lots: bigint; minimum: bigint; maximum: bigint;
      fee: bigint; tif: 'GTC' | 'ALO' | 'IOC'; reduceOnly: boolean; goodUntil: bigint }
  | { kind: 'cancel'; target: Id; attempt: Id; goodUntil: bigint }
  | { kind: 'payout'; net: bigint; maximumFee: bigint; allowPartial: boolean; goodUntil: bigint }
  | { kind: 'grant'; grant: Grant }
  | { kind: 'revoke' }
  | { kind: 'leverage'; market: Id; leverage: bigint; goodUntil: bigint };
export interface Intent { domain: Domain; account: Id; id: Id; policy: number; command: Command }
export interface Envelope extends Intent { epoch: bigint; signer: Id; session: Id; expiresAt: bigint }
/** Opt-in v2 contract; current deployed/server profiles are not upgraded by this type. */
export interface UnitIntent extends Intent { unit: Id }
export interface UnitEnvelope extends Envelope { unit: Id }
/** Private caller-owned result. Erase wire/digest after use in the verified channel. */
export interface SignedUnitRequest { unit: Id; id: Id; policy: number; digest: Id; wire: Uint8Array }
export type Outcome = 'rejected' | 'accepted' | 'dispatched' | 'acknowledged' | 'partial' | 'complete' | 'unknown' | 'recorded';
export interface Receipt {
  kind: 'receipt'; id: Id; digest: Id; policy: number; outcome: Outcome; filled: bigint;
  paid: bigint; railFees: bigint; feeCap: bigint; possiblyExposed: boolean; allowPartial: boolean;
}
export interface View {
  kind: 'view'; epoch: bigint; cash: bigint; funding: bigint; held: bigint;
  positions: { market: Id; precision: number; lots: bigint; basis: bigint }[]; operations: Id[];
}
export type ErrorCode = 'invalid' | 'unauthorized' | 'not-found' | 'conflict' | 'unavailable';
export interface Position { market: Id; precision: number; lots: bigint; basis: bigint }
export interface AccountRow {
  kind: 'account'; epoch: bigint; cash: bigint; funding: bigint; held: bigint;
  risk?: { equity: bigint; initial: bigint; maintenance: bigint; outcomeRequirement: bigint; otherHeld: bigint; free: bigint };
  accountingEquity?: bigint;
}
export type ReadRow = AccountRow
  | {kind:'ingress';request:Id;route:Id;status:'pending'|'prepared'|'exposed'|'failed'|'refunded'|'funding'|'credited'|'returned';amount:bigint;recognized:boolean;restricted:bigint}
  | (Position & { kind: 'position'; valuation?: { mark: bigint; pnl: bigint }; leverage?: { cap: bigint; selected: bigint } })
  | { kind: 'operation'; committedAt: bigint; command: Command; receipt: Receipt }
  | { kind: 'fill'; request: Id; market: Id; precision: number; lots: bigint; price: bigint; fee: bigint; observedAt?: bigint }
  | { kind: 'funding'; market: Id; precision: number; lots: bigint; payment?: bigint; settled: boolean; observedAt?: bigint }
  | { kind: 'movement'; committedAt: bigint; before: { cash: bigint; funding: bigint; positions: Position[] }; after: { cash: bigint; funding: bigint; positions: Position[] } }
  | { kind: 'grant'; grant: Grant; usedOrders: bigint }
  | { kind: 'market'; market: Id; precision: number; quote: Id; quotePrecision: number; numerator: bigint; denominator: bigint; modes: number; leverageCap?: bigint }
  | { kind: 'receipt'; receipt: Receipt };
export interface ReadPage { kind: 'page'; family: ReadFamily; revision: Id; next: Uint8Array; evaluatedAt: bigint; rows: ReadRow[] }
/** Owner-only bootstrap metadata. None of these fields is a balance or credit. */
export interface Discovery {
  kind: 'discovery'; account: Id; epoch: bigint; recipient: Id; pendingDeposits: Id[];
  custody: { maximumAccounts: number; domain: Id; pool: Id; venue: Id; program: Id;
    config: Id; route: Id; mint: Id; decimals: number; epoch: bigint; maximumDeposit: bigint };
}
export type Response = Receipt | View | ReadPage | Discovery | { kind: 'error'; code: ErrorCode };
export interface MessageSigner { publicKey: Id; signMessage(message: Uint8Array): Promise<Uint8Array> }
export interface ChannelContext extends Domain { binding: Id; expiresAt: bigint }
/** Trusted port, NOT evidence of encryption/attestation. P19 supplies the verified
 * implementation. Never obtain this object from an untrusted parent/relay. */
export interface ConfidentialChannel {
  context(): ChannelContext;
  exchange(privateWire: Uint8Array): Promise<Uint8Array>;
  /** Optional attested WebSocket read stream, never an authority bypass. */
  subscribe?(privateWire: Uint8Array): Promise<AsyncIterable<Uint8Array>>;
}
const utf8 = new TextEncoder();
function fail(): never { throw new Error('Invalid private protocol'); }
function id(v: Id): Uint8Array {
  if (!(v instanceof Uint8Array) || v.length !== 32 || v.every(b => b === 0)) fail();
  // Buffer is a Uint8Array too, but Buffer.slice() aliases its input. Always
  // copy before zeroizing wire scratch buffers or retaining private identities.
  return new Uint8Array(v);
}
function equal(a: Uint8Array, b: Uint8Array) { return a.length === b.length && a.every((v, i) => v === b[i]); }
class Writer {
  chunks: Uint8Array[];
  constructor(prefix: string) { this.chunks = [utf8.encode(prefix)]; }
  key(v: Id) { this.chunks.push(id(v)); }
  n(n: bigint, bytes: number, signed = false) {
    if (typeof n !== 'bigint') fail();
    const bits = BigInt(bytes * 8), min = signed ? -(1n << (bits - 1n)) : 0n;
    const max = signed ? (1n << (bits - 1n)) - 1n : (1n << bits) - 1n;
    if (n < min || n > max) fail();
    const out = new Uint8Array(bytes); let u = BigInt.asUintN(bytes * 8, n);
    for (let i = bytes - 1; i >= 0; i--) { out[i] = Number(u & 255n); u >>= 8n; }
    this.chunks.push(out);
  }
  boolean(v: boolean) { if (typeof v !== 'boolean') fail(); this.n(v ? 1n : 0n, 1); }
  end() {
    const result = new Uint8Array(this.chunks.reduce((n, c) => n + c.length, 0));
    let at = 0; for (const c of this.chunks) { result.set(c, at); at += c.length; c.fill(0); }
    return result;
  }
}
function prefix(w: Writer, i: Intent) {
  w.key(i.domain.network); w.key(i.domain.deployment); w.key(i.account); w.key(i.id);
  if (!Number.isInteger(i.policy) || i.policy < 1 || i.policy > 0xffffffff) fail();
  w.n(BigInt(i.policy), 4);
}
function command(w: Writer, c: Command) {
  switch (c.kind) {
    case 'read': {
      const tag = READ_FAMILIES.indexOf(c.query.family), cursor = c.query.cursor ?? new Uint8Array(40);
      if (tag < 0 || !Number.isInteger(c.query.limit) || c.query.limit < 1 || c.query.limit > 64
        || !(cursor instanceof Uint8Array) || cursor.length !== 40
        || cursor.subarray(0,32).some(x=>x!==0) !== cursor.subarray(32).some(x=>x!==0)) fail();
      w.n(8n,1); w.n(1n,1); w.n(BigInt(tag),1); w.chunks.push(Uint8Array.from(cursor)); w.n(BigInt(c.query.limit),2); break;
    }
    case 'view': w.n(0n, 1); break;
    case 'operation': w.n(1n, 1); w.key(c.target); break;
    case 'order': {
      const tif = { GTC: 0n, ALO: 1n, IOC: 2n }[c.tif];
      if (tif === undefined || c.lots === 0n || c.minimum <= 0n || c.maximum < c.minimum || c.fee < 0n
        || c.fee * (c.lots < 0n ? -c.lots : c.lots) > (1n << 127n) - 1n) fail();
      w.n(2n, 1); w.key(c.market); w.n(c.lots, 8, true); w.n(c.minimum, 8); w.n(c.maximum, 8);
      w.n(c.fee, 16, true); w.n(tif, 1); w.boolean(c.reduceOnly); w.n(c.goodUntil, 8); break;
    }
    case 'cancel': w.n(3n, 1); w.key(c.target); w.key(c.attempt); w.n(c.goodUntil, 8); break;
    case 'payout':
      if (c.net <= 0n || c.maximumFee < 0n || c.net + c.maximumFee > (1n << 127n) - 1n) fail();
      w.n(4n, 1); w.n(c.net, 16, true); w.n(c.maximumFee, 16, true); w.boolean(c.allowPartial); w.n(c.goodUntil, 8); break;
    case 'grant': {
      const g = c.grant;
      if (!Number.isInteger(g.methods) || g.methods < 1 || g.methods > 7 || g.maximumLots <= 0n || g.maximumFee < 0n || g.maximumOrders <= 0n) fail();
      w.n(5n, 1); w.key(g.key); w.n(BigInt(g.methods), 1); w.key(g.market); w.n(g.maximumLots, 8);
      w.n(g.maximumFee, 16, true); w.n(g.maximumOrders, 8); w.n(g.expiresAt, 8); break;
    }
    case 'revoke': w.n(6n, 1); break;
    case 'leverage': if (c.leverage <= 0n) fail(); w.n(7n, 1); w.key(c.market); w.n(c.leverage, 8); w.n(c.goodUntil, 8); break;
    default: fail();
  }
}
/** Canonical Ed25519 preimage, not a native venue signature. */
export function signingMessage(e: Envelope): Uint8Array {
  const w = new Writer('CINDER-API\0\x00\x01'); prefix(w, e);
  if (e.epoch <= 0n) fail();
  w.n(e.epoch, 8); w.key(e.signer); w.key(e.session); w.n(e.expiresAt, 8); command(w, e.command);
  return w.end();
}
/** Excludes session/epoch/auth expiry; includes immutable financial deadlines. */
export async function intentDigest(i: Intent): Promise<Uint8Array> {
  const w = new Writer('CINDER-API-INTENT\0\x00\x01'); prefix(w, i); command(w, i.command);
  const bytes = w.end();
  try { return new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)); }
  finally { bytes.fill(0); }
}
function unitIntentBytes(i: UnitIntent): Uint8Array<ArrayBuffer> {
  const w = new Writer('CINDER-API-INTENT\0\x00\x02'); prefix(w, i); w.key(i.unit); command(w, i.command);
  return w.end();
}
/** V2 signature preimage includes UnitId; never wrap or reuse a v1 signature. */
export function unitSigningMessage(e: UnitEnvelope): Uint8Array {
  const w = new Writer('CINDER-API\0\x00\x02'); prefix(w, e); w.key(e.unit);
  if (e.epoch <= 0n) fail();
  w.n(e.epoch, 8); w.key(e.signer); w.key(e.session); w.n(e.expiresAt, 8); command(w, e.command);
  const bytes = w.end();
  if (bytes.length + 64 > 1024) { bytes.fill(0); fail(); }
  return bytes;
}
/** Same-unit retry survives auth refresh; changing the unit changes economics. */
export async function unitIntentDigest(i: UnitIntent): Promise<Uint8Array> {
  const bytes = unitIntentBytes(i);
  try { return new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)); }
  finally { bytes.fill(0); }
}
/** Pure signing helper, not a network client. Snapshot all bytes/metadata before
 * awaiting a wallet. No channel fallback, native signing, nonce generation or retry. */
export async function signUnitRequest(e: UnitEnvelope, signer: MessageSigner): Promise<SignedUnitRequest> {
  const unit = id(e.unit), requestId = id(e.id), policy = e.policy;
  if (!equal(id(signer.publicKey), id(e.signer))) fail();
  const message = unitSigningMessage(e), intent = unitIntentBytes(e);
  const scratch = message.slice();
  let wire: Uint8Array | undefined, digest: Uint8Array | undefined;
  try {
    digest = new Uint8Array(await crypto.subtle.digest('SHA-256', intent));
    const signature = await signer.signMessage(scratch);
    if (!(signature instanceof Uint8Array) || signature.length !== 64) fail();
    wire = new Uint8Array(message.length + 64); wire.set(message); wire.set(signature, message.length);
    return { unit, id: requestId, policy, digest, wire };
  } catch {
    wire?.fill(0); digest?.fill(0);
    throw new Error('Private unit signing unavailable');
  } finally { message.fill(0); intent.fill(0); scratch.fill(0); }
}
class Reader {
  at: number; bytes: Uint8Array;
  constructor(bytes: Uint8Array, prefix = 'CINDER-API-REPLY\0\x00\x01') {
    const p = utf8.encode(prefix);
    if (!(bytes instanceof Uint8Array) || bytes.length > 1_048_576 || !equal(bytes.slice(0, p.length), p)) fail();
    this.at = p.length; this.bytes = bytes;
  }
  take(n: number) { if (this.at + n > this.bytes.length) fail(); const b = new Uint8Array(this.bytes.subarray(this.at, this.at + n)); this.at += n; return b; }
  n(bytes: number, signed = false) { let n = 0n; for (const x of this.take(bytes)) n = (n << 8n) | BigInt(x); return signed ? BigInt.asIntN(bytes * 8, n) : n; }
  boolean() { const n = this.n(1); if (n > 1n) fail(); return n === 1n; }
  count() { const n = this.n(8); if (n > 4096n) fail(); return Number(n); }
  done() { if (this.at !== this.bytes.length) fail(); }
  rest() { return this.take(this.bytes.length-this.at); }
}
function option(r: Reader, bytes = 16, signed = true) { return r.boolean() ? r.n(bytes,signed) : undefined; }
/** Strict v2 wrapper. Unit identity is required even for a redacted error; a v1
 * reply or another unit is never interpreted as success/fallback. */
export function decodeUnitResponse(bytes: Uint8Array, expectedUnit: Id): { unit: Id; response: Response } {
  const r = new Reader(bytes, 'CINDER-API-REPLY\0\x00\x02'), unit = r.take(32);
  if (!equal(unit, id(expectedUnit))) fail();
  const body=r.rest();
  try{return { unit, response: decodeBody(body, true) };}finally{body.fill(0);}
}
function unit(r: Reader) { const market=r.take(32), precision=Number(r.n(4)); if(!precision||!market.some(x=>x))fail();return {market,precision}; }
function position(r: Reader): Position { return {...unit(r),lots:r.n(8,true),basis:r.n(16,true)}; }
function grant(r: Reader): Grant { return {key:r.take(32),methods:Number(r.n(1)),market:r.take(32),maximumLots:r.n(8),maximumFee:r.n(16,true),maximumOrders:r.n(8),expiresAt:r.n(8)}; }
function readCommand(r: Reader): Command {
  const tag=Number(r.n(1));
  switch(tag){
    case 0:return {kind:'view'};
    case 1:return {kind:'operation',target:r.take(32)};
    case 2:{const market=r.take(32),lots=r.n(8,true),minimum=r.n(8),maximum=r.n(8),fee=r.n(16,true),tif=['GTC','ALO','IOC'][Number(r.n(1))] as 'GTC'|'ALO'|'IOC';if(!tif)fail();return {kind:'order',market,lots,minimum,maximum,fee,tif,reduceOnly:r.boolean(),goodUntil:r.n(8)};}
    case 3:return {kind:'cancel',target:r.take(32),attempt:r.take(32),goodUntil:r.n(8)};
    case 4:return {kind:'payout',net:r.n(16,true),maximumFee:r.n(16,true),allowPartial:r.boolean(),goodUntil:r.n(8)};
    case 5:return {kind:'grant',grant:grant(r)};
    case 6:return {kind:'revoke'};
    case 7:return {kind:'leverage',market:r.take(32),leverage:r.n(8),goodUntil:r.n(8)};
    default:fail();
  }
}
function accountRow(r: Reader): AccountRow {
  const out:AccountRow={kind:'account',epoch:r.n(8),cash:r.n(16,true),funding:r.n(16,true),held:r.n(16,true)};
  if(r.boolean())out.risk={equity:r.n(16,true),initial:r.n(16,true),maintenance:r.n(16,true),outcomeRequirement:r.n(16,true),otherHeld:r.n(16,true),free:r.n(16,true)};
  out.accountingEquity=option(r);if(!out.epoch||out.held<0n)fail();return out;
}
function readRow(family: ReadFamily, bytes: Uint8Array, scoped = false): ReadRow {
  const r=new Reader(bytes,'');let out:ReadRow;
  switch(family){
    case 'ingress':{
      const request=r.take(32),route=r.take(32),status=(['pending','prepared','exposed','failed','refunded','funding','credited','returned'] as const)[Number(r.n(1))];
      const amount=r.n(16,true),recognized=r.boolean(),restricted=r.n(16,true);
      if(!status||!request.some(x=>x)||!route.some(x=>x)||amount<=0n||restricted<0n||restricted>amount
        ||recognized!==(status==='funding'||status==='credited'||status==='returned')||status!=='funding'&&restricted!==0n)fail();
      out={kind:'ingress',request,route,status,amount,recognized,restricted};break;
    }
    case 'account':out=accountRow(r);break;
    case 'positions':{const p=position(r);const valuation=r.boolean()?{mark:r.n(8),pnl:r.n(16,true)}:undefined;
      const leverage=r.boolean()?{cap:r.n(8),selected:r.n(8)}:undefined;
      if(valuation?.mark===0n||leverage&&(leverage.cap===0n||leverage.selected===0n||leverage.selected>leverage.cap))fail();
      out={kind:'position',...p,valuation,leverage};break;}
    case 'operations':case 'openOrders':{const committedAt=r.n(8),c=readCommand(r),reply=decodeBody(r.rest(),scoped);if(reply.kind!=='receipt')fail();out={kind:'operation',committedAt,command:c,receipt:reply};break;}
    case 'fills':out={kind:'fill',request:r.take(32),...unit(r),lots:r.n(8,true),price:r.n(8),fee:r.n(16,true),observedAt:option(r,8,false)};break;
    case 'funding':out={kind:'funding',...unit(r),lots:r.n(8,true),payment:option(r),settled:r.boolean(),observedAt:option(r,8,false)};break;
    case 'movements':{const committedAt=r.n(8);const book=()=>{const cash=r.n(16,true),funding=r.n(16,true),n=r.count();if(n>64)fail();return {cash,funding,positions:Array.from({length:n},()=>position(r))};};out={kind:'movement',committedAt,before:book(),after:book()};break;}
    case 'grants':if(r.n(1)!==5n)fail();out={kind:'grant',grant:grant(r),usedOrders:r.n(8)};
      if(!out.grant.key.some(x=>x)||!out.grant.market.some(x=>x)||out.grant.methods<1||out.grant.methods>7
        ||out.grant.maximumLots===0n||out.grant.maximumFee<0n||out.grant.maximumOrders===0n||out.usedOrders>out.grant.maximumOrders)fail();break;
    case 'markets':out={kind:'market',...unit(r),quote:r.take(32),quotePrecision:Number(r.n(4)),numerator:r.n(8),denominator:r.n(8),modes:Number(r.n(1)),leverageCap:option(r,8,false)};
      if(!out.quote.some(x=>x)||!out.quotePrecision||out.numerator<=0n||out.denominator<=0n||out.modes!==7||out.leverageCap===0n)fail();break;
    case 'updates':{const tag=r.n(1);if(tag===2n)out=accountRow(r);else if(tag===0n)out={kind:'position',...position(r)};
      else if(tag===1n){const reply=decodeBody(r.rest(),scoped);if(reply.kind!=='receipt')fail();out={kind:'receipt',receipt:reply};}else fail();break;}
    default:fail();
  }
  r.done();return out;
}
/** Strict bounded replies; cash/funding/margin/fee cap are not conflated. */
export function decodeResponse(bytes: Uint8Array): Response {
  return decodeBody(bytes, false);
}
function decodeBody(bytes: Uint8Array, scoped: boolean): Response {
  const r = new Reader(bytes); const tag = r.n(1); let out: Response;
  if (tag === 0n) {
    const epoch = r.n(8), cash = r.n(16, true), funding = r.n(16, true), held = r.n(16, true);
    const positions = Array.from({ length: r.count() }, () => ({ market: r.take(32), precision: Number(r.n(4)), lots: r.n(8, true), basis: r.n(16, true) }));
    const operations = Array.from({ length: r.count() }, () => r.take(32));
    if (epoch === 0n || held < 0n || positions.some(p => p.precision === 0 || p.market.every(x => x === 0))) fail();
    out = { kind: 'view', epoch, cash, funding, held, positions, operations };
  } else if (tag === 1n) {
    const requestId = r.take(32), digest = r.take(32), policy = Number(r.n(4));
    const outcomes: Outcome[] = ['rejected', 'accepted', 'dispatched', 'acknowledged', 'partial', 'complete', 'unknown'];
    if(scoped)outcomes.push('recorded');
    const outcome = outcomes[Number(r.n(1))]; if (!outcome || policy === 0 || requestId.every(x => x === 0)) fail();
    out = { kind: 'receipt', id: requestId, digest, policy, outcome, filled: r.n(8, true), paid: r.n(16, true),
      railFees: r.n(16, true), feeCap: r.n(16, true), possiblyExposed: r.boolean(), allowPartial: r.boolean() };
    if (out.paid < 0n || out.railFees < 0n || out.feeCap < 0n) fail();
  } else if (tag === 4n) {
    const account=r.take(32),epoch=r.n(8),recipient=r.take(32),maximumAccounts=Number(r.n(2));
    const domain=r.take(32),pool=r.take(32),venue=r.take(32),program=r.take(32),config=r.take(32),route=r.take(32),mint=r.take(32);
    const decimals=Number(r.n(1)),custodyEpoch=r.n(8),maximumDeposit=r.n(8);
    if(!epoch||!custodyEpoch||!maximumDeposit||maximumAccounts<1||maximumAccounts>1024||decimals>18
      ||[account,recipient,domain,pool,venue,program,config,route,mint].some(x=>!x.some(b=>b)))fail();
    const count=r.count();if(count>128)fail();const pendingDeposits=Array.from({length:count},()=>id(r.take(32)));
    if(new Set(pendingDeposits.map(x=>Array.from(x).join(','))).size!==count)fail();
    out={kind:'discovery',account,epoch,recipient,pendingDeposits,custody:{maximumAccounts,domain,pool,venue,program,config,route,mint,decimals,epoch:custodyEpoch,maximumDeposit}};
  } else if (tag === 3n) {
    if(r.n(1)!==1n)fail();const family=READ_FAMILIES[Number(r.n(1))];if(!family)fail();
    const revision=r.take(32),next=r.take(40),evaluatedAt=r.n(8),count=r.count();if(count>64||!revision.some(x=>x))fail();
    if(next.subarray(0,32).some(x=>x)!==next.subarray(32).some(x=>x)||next.some(x=>x)&&!equal(next.subarray(0,32),revision))fail();
    const rows=Array.from({length:count},()=>{const size=Number(r.n(4));if(size<1||size>1_048_576)fail();return readRow(family,r.take(size),scoped);});
    out={kind:'page',family,revision,next,evaluatedAt,rows};
  } else if (tag === 2n) {
    const codes: ErrorCode[] = ['invalid', 'unauthorized', 'not-found', 'conflict', 'unavailable'];
    const code = codes[Number(r.n(1))]; if (!code) fail(); out = { kind: 'error', code };
  } else fail();
  r.done(); return out;
}
/** Wallet-only bootstrap; an agent signer cannot act on an owner's account.
 * Requests use the same attested encrypted channel, never a public signup API. */
class BootstrapClient {
  readonly #channel: ConfidentialChannel; readonly #signer: MessageSigner;
  readonly #domain: Domain; readonly #policy: number;
  readonly #unit?:Id; readonly #contract?:Id;
  constructor(channel: ConfidentialChannel, signer: MessageSigner, config: {domain: Domain; policy: number}, scoped?:{unit:Id;contract:Id}) {
    this.#channel=channel;this.#signer=signer;
    this.#domain={network:id(config.domain.network),deployment:id(config.domain.deployment)};
    if(!Number.isInteger(config.policy)||config.policy<1||config.policy>0xffffffff)fail();
    this.#policy=config.policy;
    if(scoped){this.#unit=id(scoped.unit);this.#contract=id(scoped.contract);}
  }
  /** Create once or retrieve current epoch. Does not reset grants or paid counters. */
  open(expiresAt:bigint):Promise<Discovery|{kind:'error';code:ErrorCode}>{return this.#exchange(expiresAt);}
  /** Record an original signature for qualification; this is NOT a credit ACK. */
  reportDeposit(input:{expiresAt:bigint;epoch:bigint;operation:Id;signature:Uint8Array}) {
    return this.#exchange(input.expiresAt,input);
  }
  async #exchange(expiresAt:bigint,notice?:{epoch:bigint;operation:Id;signature:Uint8Array}):Promise<Discovery|{kind:'error';code:ErrorCode}>{
    const ctx=this.#channel.context(),wallet=id(this.#signer.publicKey);
    if(!equal(ctx.network,this.#domain.network)||!equal(ctx.deployment,this.#domain.deployment)||expiresAt>ctx.expiresAt)fail();
    const w=new Writer('CINDER-ONBOARD\0\x00\x01');w.key(this.#domain.network);w.key(this.#domain.deployment);w.n(BigInt(this.#policy),4);
    w.key(wallet);w.key(ctx.binding);w.n(expiresAt,8);
    if(notice){
      if(notice.epoch<=0n||!(notice.signature instanceof Uint8Array)||notice.signature.length!==64||!notice.signature.some(x=>x))fail();
      w.n(1n,1);w.n(notice.epoch,8);w.key(notice.operation);w.chunks.push(new Uint8Array(notice.signature));
    }else w.n(0n,1);
    const original=w.end();
    const message=this.#unit?new Uint8Array([...utf8.encode('CINDER-UNIT-ONBOARD\0\x00\x01'),...this.#unit,...this.#contract!,...original]):original;
    if(this.#unit)original.fill(0);
    const wire=new Uint8Array(message.length+64);
    try{
      const toSign=message.slice();let signature:Uint8Array;
      try{signature=await this.#signer.signMessage(toSign);}finally{toSign.fill(0);}
      if(!(signature instanceof Uint8Array)||signature.length!==64)fail();wire.set(message);wire.set(signature,message.length);
      const reply=await this.#channel.exchange(wire);
      try{
        const result=this.#unit?decodeUnitResponse(reply,this.#unit).response:decodeResponse(reply);if(result.kind==='error')return result;if(result.kind!=='discovery')fail();
        const identity=new Uint8Array([...utf8.encode('CINDER-ACCOUNT-1\0'),...this.#domain.network,...this.#domain.deployment,...wallet]);
        try{const expected=new Uint8Array(await crypto.subtle.digest('SHA-256',identity));if(!equal(result.account,expected))fail();}finally{identity.fill(0);}
        return result;
      }finally{reply.fill(0);}
    }catch{throw new Error('Private onboarding unavailable; reconnect and retrieve the same owner account before proceeding');}
    finally{message.fill(0);wire.fill(0);wallet.fill(0);}
  }
}
/** Legacy single-route owner bootstrap. Never defaults into a shared unit. */
export class OnboardingClient extends BootstrapClient {
  constructor(channel:ConfidentialChannel,signer:MessageSigner,config:{domain:Domain;policy:number}){super(channel,signer,config);}
}
/** Explicit owner bootstrap for a shared deployment. Pin the independently
 * approved custody contract; never copy an unauthenticated relay hint. */
export class UnitOnboardingClient extends BootstrapClient {
  constructor(channel:ConfidentialChannel,signer:MessageSigner,config:{domain:Domain;policy:number;unit:Id;contract:Id}){super(channel,signer,config,config);}
}

class Client {
  readonly #domain: Domain; readonly #account: Id; readonly #policy: number;
  readonly #unit?: Id;
  readonly #channel: ConfidentialChannel; readonly #signer: MessageSigner;
  constructor(channel: ConfidentialChannel, signer: MessageSigner, config: { domain: Domain; account: Id; policy: number }, unit?: Id) {
    this.#domain = { network: id(config.domain.network), deployment: id(config.domain.deployment) };
    this.#account = id(config.account); this.#policy = config.policy; this.#channel = channel; this.#signer = signer;
    this.#unit = unit === undefined ? undefined : id(unit);
  }
  /** Server-driven replacement snapshots of a signed read projection. Page
   * cursors fetch the remaining retained range; no automatic trade retry. */
  async subscribe(input:{id:Id;epoch:bigint;expiresAt:bigint;query:ReadQuery}):Promise<AsyncIterable<ReadPage>>{
    if(!this.#channel.subscribe||input.query.cursor?.some(x=>x))throw new Error('Subscription unavailable');
    const ctx=this.#channel.context();if(!equal(ctx.network,this.#domain.network)||!equal(ctx.deployment,this.#domain.deployment)||input.expiresAt>ctx.expiresAt)fail();
    const query:ReadQuery={...input.query,cursor:input.query.cursor?.slice()};
    const env:Envelope={domain:this.#domain,account:this.#account,id:id(input.id),policy:this.#policy,epoch:input.epoch,
      signer:id(this.#signer.publicKey),session:id(ctx.binding),expiresAt:input.expiresAt,command:{kind:'read',query}};
    const message=this.#unit?unitSigningMessage({...env,unit:this.#unit}):signingMessage(env),wire=new Uint8Array(message.length+64);let stream:AsyncIterable<Uint8Array>;
    try{wire.set(message);const scratch=message.slice();let signature;
      try{signature=await this.#signer.signMessage(scratch);}finally{scratch.fill(0);}
      if(!(signature instanceof Uint8Array)||signature.length!==64)fail();wire.set(signature,message.length);
      stream=await this.#channel.subscribe(wire);
    }finally{message.fill(0);wire.fill(0);}
    const family=query.family;
    const unit=this.#unit?.slice();
    const iterator=stream[Symbol.asyncIterator]();
    const source:AsyncIterable<Uint8Array>={[Symbol.asyncIterator]:()=>iterator};
    return privateIterable(async function*(){for await(const bytes of source){try{
      const reply=unit?decodeUnitResponse(bytes,unit).response:decodeResponse(bytes);if(reply.kind!=='page'||reply.family!==family)throw new Error('Private subscription unavailable; resynchronize');yield reply;
    }finally{bytes.fill(0);}}},()=>iterator.return?.());
  }
  /** No retries or native nonce generation. On uncertainty query the original ID.
   * Caller supplies current epoch and re-signs on a new confidential session. */
  async request(input: { id: Id; epoch: bigint; expiresAt: bigint; command: Command }): Promise<Response> {
    const ctx = this.#channel.context();
    if (!equal(ctx.network, this.#domain.network) || !equal(ctx.deployment, this.#domain.deployment)
      || input.expiresAt > ctx.expiresAt) throw new Error('Confidential session mismatch');
    const env: Envelope = { domain: this.#domain, account: this.#account, id: id(input.id), policy: this.#policy,
      command: input.command, epoch: input.epoch, signer: id(this.#signer.publicKey), session: id(ctx.binding), expiresAt: input.expiresAt };
    const kind = input.command.kind, family=kind==='read'?input.command.query.family:undefined,
      target = kind === 'operation' ? id((input.command as { target: Id }).target) : env.id.slice();
    const scoped=this.#unit?{...env,unit:this.#unit}:undefined;
    const message = scoped?unitSigningMessage(scoped):signingMessage(env), wire = new Uint8Array(message.length + 64);
    const digest = scoped?await unitIntentDigest(scoped):await intentDigest(env);
    try {
      wire.set(message); const toSign = message.slice(); let signature: Uint8Array;
      try { signature = await this.#signer.signMessage(toSign); } finally { toSign.fill(0); }
      if (!(signature instanceof Uint8Array) || signature.length !== 64) fail();
      wire.set(signature, message.length);
      const reply = await this.#channel.exchange(wire);
      try {
        const response = this.#unit?decodeUnitResponse(reply,this.#unit).response:decodeResponse(reply);
        if (response.kind === 'error') return response;
        if (kind === 'view') { if (response.kind !== 'view') fail(); }
        else if(kind==='read'){if(response.kind!=='page'||response.family!==family)fail();}
        else if (response.kind !== 'receipt' || response.policy !== this.#policy || !equal(response.id, target)
          || (kind !== 'operation' && !equal(response.digest, digest))) fail();
        return response;
      } finally { reply.fill(0); }
    } catch {
      throw new Error('Private request unavailable; reconcile the stable operation before retry');
    } finally { message.fill(0); wire.fill(0); digest.fill(0); }
  }
}

/** Existing single-unit v1 protocol. Never falls back to or infers a v2 unit. */
export class PrivateClient extends Client {
  constructor(channel:ConfidentialChannel,signer:MessageSigner,config:{domain:Domain;account:Id;policy:number}){
    super(channel,signer,config);
  }
}
/** Explicit opt-in shared-journal unit protocol. All private replies and update
 * pages must repeat this unit; legacy or other-unit responses are refused. */
export class UnitClient extends Client {
  constructor(channel:ConfidentialChannel,signer:MessageSigner,config:{domain:Domain;account:Id;policy:number;unit:Id}){
    super(channel,signer,config,config.unit);
  }
}
