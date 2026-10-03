// Internal transport implementation, not a public caller-selectable trust root.
// Public web-channel.ts fixes AWS root; fixture trust lives in tests only.
import type { ChannelContext, ConfidentialChannel } from './index.ts';
import type { ReleasePolicy } from './node-channel.ts';
import type { QuoteContext, VerifiedQuote } from '../../../tools/web-channel/attestation/verifier.mjs';
import { privateIterable } from './private-iteration.ts';

export interface WebEndpoint {
  start(): Uint8Array; advance(bytes: Uint8Array): Uint8Array;
  ready(): boolean; binding(): Uint8Array;
  request(sequence: number, bytes: Uint8Array): Uint8Array;
  socket_request(sequence: number, bytes: Uint8Array, subscribe: boolean): Uint8Array;
  response(sequence: number, bytes: Uint8Array): Uint8Array;
  free(): void;
}
/** Load this pinned Rust/WASM build from the application's trusted distribution,
 * never a URL/module/public key selected by the relay's attestation response. */
export interface WebCore {
  BrowserEndpoint: new (key: Uint8Array, prologue: Uint8Array) => WebEndpoint;
  web_binding(prologue: Uint8Array, hash: Uint8Array): Uint8Array;
}
export interface WebOptions { baseUrl: string; policy: ReleasePolicy; core: WebCore; transport?: 'http'|'websocket' }
type Verifier = (quote: Uint8Array, policy: Uint8Array, context: QuoteContext) => Promise<VerifiedQuote>;
const MAX_BATCH = 1048576 + 65 * 34, DEADLINE = 5000;
const failed = () => Error('Confidential web channel unavailable; reconcile on a fresh session');
const copy = (value: Uint8Array, size: number) => {
  if (!(value instanceof Uint8Array) || value.length !== size || !value.some(b => b)) throw failed();
  return Uint8Array.from(value);
};
const concat = (...parts: Uint8Array[]) => {
  const result = new Uint8Array(parts.reduce((n, p) => n + p.length, 0)); let at = 0;
  for (const p of parts) { result.set(p, at); at += p.length; } return result;
};
export class WebChannel implements ConfidentialChannel {
  #endpoint?: WebEndpoint; #context?: ChannelContext; #handle?: Uint8Array;
  #base: string; #closed = false; #busy = false; #sequence = 0;
  #active?: AbortController; #lifetime?: ReturnType<typeof setTimeout>;
  #socket?: WebSocket;
  #pending?: {sequence:number;resolve:(b:Uint8Array)=>void;reject:(e:Error)=>void};
  #watch?: {sequence:number;ordinal:bigint;queue:Uint8Array[];bytes:number;wait?:()=>void};
  #subscriptionDeadline?:ReturnType<typeof setTimeout>;
  private constructor(base: string) { this.#base = base; }
  static async connect(options: WebOptions, verify: Verifier): Promise<WebChannel> {
    const url = new URL(options.baseUrl);
    if (url.username || url.password || url.search || url.hash || url.pathname !== '/'
        || (url.protocol !== 'https:' && !(url.protocol === 'http:' && ['127.0.0.1','[::1]','localhost'].includes(url.hostname)))) throw failed();
    const policy = concat(copy(options.policy.network,32), copy(options.policy.deployment,32),
      copy(options.policy.manifest,32), ...options.policy.pcrs.map(p => copy(p,48)));
    if (policy.length !== 240) throw failed();
    const core = options.core;
    const ch = new WebChannel(url.origin), abort = new AbortController();
    ch.#active = abort;
    const timer = setTimeout(() => ch.close(), DEADLINE);
    try {
      const nonce = crypto.getRandomValues(new Uint8Array(32));
      copy(nonce,32);
      const envelope = await ch.#post('/v1/attestation', nonce, 16520, abort.signal);
      if (envelope.length <= 136 || !nonce.every((b,i) => b === envelope[i])) throw failed();
      const expires = new DataView(envelope.buffer, envelope.byteOffset + 128, 8).getBigUint64(0);
      if (expires > BigInt(Number.MAX_SAFE_INTEGER)) throw failed();
      const context: QuoteContext = { now: Date.now(), expires: Number(expires), nonce,
        boot: copy(envelope.slice(32,64),32), handle: copy(envelope.slice(64,96),32), key: copy(envelope.slice(96,128),32) };
      const trusted = await verify(envelope.slice(136), policy, context);
      if (ch.#closed || abort.signal.aborted || !context.key.every((b,i) => b === trusted.key[i]) || trusted.key.length !== 32) throw failed();
      ch.#handle = context.handle;
      ch.#endpoint = new core.BrowserEndpoint(copy(trusted.key,32), copy(trusted.prologue,32));
      const first = ch.#endpoint.start();
      const second = await ch.#post('/v1/session', concat(ch.#handle, first), 16400, abort.signal);
      const confirmation = ch.#endpoint.advance(second);
      const ack = await ch.#post('/v1/session', concat(ch.#handle, confirmation), 16400, abort.signal);
      if (ch.#endpoint.advance(ack).length || !ch.#endpoint.ready() || ch.#closed || expires <= BigInt(Date.now())) throw failed();
      ch.#context = { network: policy.slice(0,32), deployment: policy.slice(32,64),
        binding: copy(core.web_binding(trusted.prologue, ch.#endpoint.binding()),32), expiresAt: expires };
      if(options.transport!==undefined&&!['http','websocket'].includes(options.transport))throw failed();
      if(options.transport==='websocket')await ch.#openSocket();
      ch.#lifetime = setTimeout(() => ch.close(), Number(expires - BigInt(Date.now())));
      return ch;
    } catch { ch.close(); throw failed(); }
    finally { clearTimeout(timer); ch.#active = undefined; }
  }
  context(): ChannelContext {
    if (this.#closed || !this.#context || this.#context.expiresAt <= BigInt(Date.now())) { this.close(); throw failed(); }
    return { ...this.#context, network: Uint8Array.from(this.#context.network), deployment: Uint8Array.from(this.#context.deployment), binding: Uint8Array.from(this.#context.binding) };
  }
  close() {
    const handle = this.#handle;
    this.#closed = true; this.#active?.abort(); this.#active = undefined;
    this.#pending?.reject(failed());this.#pending=undefined;
    const watch=this.#watch;this.#watch=undefined;
    if(watch){for(const b of watch.queue)b.fill(0);watch.queue=[];watch.wait?.();}
    clearTimeout(this.#subscriptionDeadline);this.#subscriptionDeadline=undefined;
    this.#socket?.close();this.#socket=undefined;
    clearTimeout(this.#lifetime); this.#lifetime = undefined;
    this.#endpoint?.free(); this.#endpoint = undefined;
    this.#handle = undefined; this.#context = undefined;
    if (handle) void fetch(this.#base + '/v1/session', { method: 'POST', headers: { 'Content-Type': 'application/octet-stream' },
      body: concat(handle, Uint8Array.of(0)).buffer, credentials: 'omit', cache: 'no-store', redirect: 'error',
      referrerPolicy: 'no-referrer', signal: AbortSignal.timeout(DEADLINE) }).then(r => r.body?.cancel()).catch(() => {});
    // free() deallocates the WASM object. It is NOT an opaque Snow/browser
    // key-erasure guarantee; that release limitation remains explicit.
  }
  async exchange(input: Uint8Array): Promise<Uint8Array> {
    if (this.#closed || this.#busy || !(input instanceof Uint8Array) || input.length === 0 || input.length > 1024) { this.close(); throw failed(); }
    this.context(); this.#busy = true;
    const clear = Uint8Array.from(input), abort = new AbortController(); this.#active = abort;
    const timer = setTimeout(() => this.close(), DEADLINE);
    try {
      const sequence = ++this.#sequence;
      const wire = this.#socket?this.#endpoint!.socket_request(sequence,clear,false):this.#endpoint!.request(sequence, clear); clear.fill(0);
      if(this.#socket){const reply=await this.#exchangeSocket(sequence,wire);this.context();return reply;}
      const batch = await this.#post('/v1/exchange', concat(this.#handle!, wire), MAX_BATCH, abort.signal);
      this.context();
      const reply = this.#endpoint!.response(sequence, batch);
      if (this.#closed) { reply.fill(0); throw failed(); }
      return reply;
    } catch { this.close(); throw failed(); }
    finally { clear.fill(0); clearTimeout(timer); this.#active = undefined; this.#busy = false; }
  }
  /** One server-driven read subscription. Closing/returning it closes the
   * connection; reattest and fetch a fresh snapshot, never replay a mutation. */
  async subscribe(input:Uint8Array):Promise<AsyncIterable<Uint8Array>>{
    if(!this.#socket||this.#watch||this.#busy||this.#closed||!(input instanceof Uint8Array)||!input.length||input.length>1024){this.close();throw failed();}
    this.context();const clear=Uint8Array.from(input),sequence=++this.#sequence;
    try{const wire=this.#endpoint!.socket_request(sequence,clear,true);
      this.#watch={sequence,ordinal:0n,queue:[],bytes:0};this.#socket.send(Uint8Array.from(wire).buffer);
      this.#subscriptionDeadline=setTimeout(()=>this.close(),DEADLINE);
    }catch{this.close();throw failed();}finally{clear.fill(0);}
    const self=this;
    return privateIterable(async function*(){
      try{for(;;){
        const watch=self.#watch;if(self.#closed||!watch)throw failed();
        if(!watch.queue.length){await new Promise<void>(resolve=>{watch.wait=resolve;});continue;}
        const bytes=watch.queue.shift()!;watch.bytes-=bytes.length;
        yield bytes;
      }}finally{self.close();}
    },()=>self.close());
  }
  async #openSocket(){
    const url=new URL('/v1/ws',this.#base);url.protocol=url.protocol==='https:'?'wss:':'ws:';
    const socket=new WebSocket(url);socket.binaryType='arraybuffer';this.#socket=socket;
    await new Promise<void>((resolve,reject)=>{
      socket.onopen=()=>{if(this.#closed){reject(failed());return;}socket.send(Uint8Array.from(this.#handle!).buffer);resolve();};
      socket.onerror=()=>{reject(failed());this.close();};socket.onclose=()=>{reject(failed());this.close();};
      socket.onmessage=event=>{
        try{
          this.context();if(!(event.data instanceof ArrayBuffer)||event.data.byteLength<5||event.data.byteLength>MAX_BATCH+4)throw failed();
          const b=new Uint8Array(event.data),sequence=new DataView(event.data).getUint32(0);
          const reply=this.#endpoint!.response(sequence,b.subarray(4));
          if(this.#watch?.sequence===sequence){
            const prefix=new TextEncoder().encode('CINDER-PRIVATE-UPDATE-1\0');
            if(reply.length<=prefix.length+8||!prefix.every((b,i)=>reply[i]===b))throw failed();
            const ordinal=new DataView(reply.buffer,reply.byteOffset+prefix.length,8).getBigUint64(0);
            const watch=this.#watch;if(ordinal!==watch.ordinal+1n)throw failed();watch.ordinal=ordinal;
            clearTimeout(this.#subscriptionDeadline);this.#subscriptionDeadline=undefined;
            const payload=reply.slice(prefix.length+8);reply.fill(0);
            if(watch.queue.length>=32||watch.bytes+payload.length>1_048_576){payload.fill(0);throw failed();}
            watch.queue.push(payload);watch.bytes+=payload.length;const wake=watch.wait;watch.wait=undefined;wake?.();
          }else if(this.#pending?.sequence===sequence){const p=this.#pending;this.#pending=undefined;p.resolve(reply);}
          else{reply.fill(0);throw failed();}
        }catch{this.close();}
      };
    });
  }
  #exchangeSocket(sequence:number,wire:Uint8Array):Promise<Uint8Array>{
    return new Promise((resolve,reject)=>{if(!this.#socket||this.#pending||this.#closed){reject(failed());return;}
      this.#pending={sequence,resolve,reject};try{this.#socket.send(Uint8Array.from(wire).buffer);}catch{this.close();}});
  }
  async #post(path: string, body: Uint8Array, maximum: number, signal: AbortSignal): Promise<Uint8Array> {
    if (this.#closed || signal.aborted) throw failed();
    const response = await fetch(this.#base + path, { method: 'POST', headers: { 'Content-Type': 'application/octet-stream' },
      body: Uint8Array.from(body).buffer, credentials: 'omit', cache: 'no-store', redirect: 'error', referrerPolicy: 'no-referrer', signal });
    if (!response.ok || response.headers.get('content-type') !== 'application/octet-stream' || !response.body) {
      await response.body?.cancel(); throw failed();
    }
    const reader = response.body.getReader(); const chunks: Uint8Array[] = []; let total = 0;
    try {
      for (;;) {
        const { value, done } = await reader.read(); if (done) break;
        total += value.length;
        if (total > maximum || chunks.length >= 4096 || this.#closed || signal.aborted) throw failed();
        chunks.push(value);
      }
      if (!total || this.#closed || signal.aborted) throw failed();
      return concat(...chunks);
    } finally { await reader.cancel().catch(() => {}); reader.releaseLock(); }
  }
}
