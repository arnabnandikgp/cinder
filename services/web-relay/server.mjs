// Parent-side public HTTP framing ONLY. No signer, decryption, wallet, customer
// method, journal or caller-controlled upstream. Logs must remain coarse.
import { createServer } from 'node:http';
import { connect } from 'node:net';

export const MAX_BATCH = 1048576 + 65 * 34;
const MAX_RECORD = 16400, MAX_ENVELOPE = 16520, DEADLINE = 5000;
const unavailable = () => Error('Web delivery unavailable; reconcile on a fresh session');

class Peer {
  constructor(target, onClose) {
    this.socket = connect(target); this.dead = false; this.chunks = []; this.total = 0; this.size = undefined;
    this.pending = undefined;
    this.socket.setNoDelay(true);
    this.socket.on('error', () => this.close());
    this.socket.on('close', () => this.close());
    this.onClose = onClose;
    this.socket.on('data', data => {
      const p = this.pending;
      if (!p || this.total + data.length > p.maximum + 4 || this.chunks.length >= 4096) return this.close();
      this.chunks.push(data); this.total += data.length;
      if (this.size === undefined && this.total >= 4) {
        this.size = Buffer.concat(this.chunks, 4).readUInt32BE();
        if (!this.size || this.size > p.maximum) return this.close();
      }
      if (this.size !== undefined && this.total > this.size + 4) return this.close();
      if (this.size !== undefined && this.total === this.size + 4) {
        const body = Buffer.concat(this.chunks, this.total).subarray(4);
        this.chunks = []; this.total = 0; this.size = undefined;
        this.pending = undefined; p.resolve(body);
      }
    });
  }
  close() {
    if (this.dead) return;
    this.dead = true; this.socket.destroy(); this.onClose?.();
    this.pending?.reject(unavailable()); this.pending = undefined;
    this.chunks = []; this.total = 0; this.size = undefined;
  }
  exchange(body, maximum) {
    if (this.dead || this.pending) { this.close(); return Promise.reject(unavailable()); }
    return new Promise((resolve, reject) => {
      this.pending = { resolve, reject, maximum };
      const frame = Buffer.alloc(4 + body.length); frame.writeUInt32BE(body.length); body.copy(frame, 4);
      this.socket.write(frame, error => { if (error) this.close(); });
    });
  }
}

/** Fixed upstream should be a loopback opaque Rust relay -> enclave AF_VSOCK.
 * No route accepts an upstream, wallet, bearer token, private ID or method. */
export function createWebRelay({ target, origin }) {
  if (!target || target.host !== '127.0.0.1' || !Number.isInteger(target.port) || target.port < 1 || target.port > 65535) throw unavailable();
  target = Object.freeze({ host: target.host, port: target.port });
  if (origin !== undefined && (new URL(origin).origin !== origin || !/^https?:/.test(origin))) throw unavailable();
  const peers = new Set(), sessions = new Map();
  let stopping = false;
  const server = createServer({ maxHeaderSize: 8192, connectionsCheckingInterval: 1000 }, async (req, res) => {
    let peer, timer, finished = false;
    const abort = () => { if (!finished && !res.writableFinished) peer?.close(); };
    req.on('aborted', abort); res.on('close', abort);
    timer = setTimeout(() => { abort(); req.destroy(); res.destroy(); }, DEADLINE);
    try {
      if (stopping || req.headers.authorization || req.headers.cookie || req.headers['content-encoding']) throw unavailable();
      if (req.headers.origin) {
        if (!origin || req.headers.origin !== origin) throw unavailable();
        res.setHeader('Access-Control-Allow-Origin', origin);
        res.setHeader('Vary', 'Origin');
      }
      if (req.method === 'OPTIONS' && ['/v1/attestation','/v1/session','/v1/exchange'].includes(req.url)) {
        res.setHeader('Access-Control-Allow-Methods', 'POST');
        res.setHeader('Access-Control-Allow-Headers', 'Content-Type');
        res.writeHead(204).end(); finished = true; return;
      }
      if (req.method !== 'POST' || req.headers['content-type'] !== 'application/octet-stream'
          || !['/v1/attestation','/v1/session','/v1/exchange'].includes(req.url)) throw unavailable();
      const cap = req.url === '/v1/attestation' ? 32 : req.url === '/v1/session' ? 32 + MAX_RECORD : 32 + 1046;
      if (req.headers['content-length'] !== undefined && (!/^\d{1,6}$/.test(req.headers['content-length']) || Number(req.headers['content-length']) > cap)) throw unavailable();
      const chunks = []; let n = 0;
      for await (const chunk of req) { n += chunk.length; if (n > cap) throw unavailable(); chunks.push(chunk); }
      const body = Buffer.concat(chunks, n);
      let reply;
      if (req.url === '/v1/attestation') {
        if (body.length !== 32 || !body.some(b => b) || peers.size >= 8) throw unavailable();
        peer = new Peer(target, () => { peers.delete(peer); if (peer.handle && sessions.get(peer.handle) === peer) sessions.delete(peer.handle); clearTimeout(peer.lifetime); });
        peers.add(peer); peer.phase = 0;
        peer.lifetime = setTimeout(() => peer.close(), 120000);
        reply = await peer.exchange(body, MAX_ENVELOPE);
        if (peer.dead) throw unavailable();
        if (reply.length <= 136 || !reply.subarray(0,32).equals(body)) throw unavailable();
        peer.handle = reply.subarray(64,96).toString('hex');
        if (!reply.subarray(64,96).some(b => b) || sessions.has(peer.handle)) throw unavailable();
        sessions.set(peer.handle, peer);
      } else {
        if (body.length <= 32) throw unavailable();
        peer = sessions.get(body.subarray(0,32).toString('hex'));
        // Public routing disposal only; a malicious parent can already drop a
        // socket. This grants no customer authority or financial action.
        if (peer && req.url === '/v1/session' && body.length === 33 && body[32] === 0) {
          peer.close(); finished = true; res.writeHead(204, { 'Cache-Control': 'no-store' }).end(); return;
        }
        if (!peer || peer.busy || peer.dead) throw unavailable();
        peer.busy = true;
        if (req.url === '/v1/session') {
          if (peer.phase > 1) throw unavailable();
          reply = await peer.exchange(body.subarray(32), MAX_RECORD); peer.phase++;
        } else {
          if (peer.phase !== 2) throw unavailable();
          reply = await peer.exchange(body.subarray(32), MAX_BATCH);
        }
      }
      // A complete last encrypted reply may precede the enclave's orderly
      // record-budget shutdown. Its framing is complete; disposal prevents
      // future requests but must not erase this already received response.
      if (res.destroyed) throw unavailable();
      res.setHeader('Content-Type', 'application/octet-stream');
      res.setHeader('Cache-Control', 'no-store');
      res.setHeader('X-Content-Type-Options', 'nosniff');
      // Mark complete only after the reply is flushed. Client disconnect closes
      // the session; it never authorizes retransmitting a ciphertext record.
      await new Promise((resolve, reject) => {
        res.once('error', reject); res.once('close', () => { if (!res.writableFinished) reject(unavailable()); });
        res.end(reply, resolve);
      });
      finished = true; peer.busy = false;
    } catch {
      peer?.close();
      if (!res.destroyed && !res.headersSent) res.writeHead(503, { 'Content-Type': 'text/plain', 'Cache-Control': 'no-store' }).end('Web delivery unavailable');
      else if (!res.writableFinished) res.destroy();
    } finally { clearTimeout(timer); }
  });
  server.headersTimeout = DEADLINE; server.requestTimeout = DEADLINE;
  server.keepAliveTimeout = DEADLINE; server.maxConnections = 32;
  server.maxRequestsPerSocket = 256;
  server.on('clientError', (_error, socket) => socket.destroy());
  return { server, activeSessions: () => peers.size,
    close: async () => {
      stopping = true; for (const p of peers) p.close(); server.closeAllConnections();
      if (server.listening) await new Promise(resolve => server.close(resolve));
    } };
}
