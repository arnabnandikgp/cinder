// Fixed operator configuration, not an open proxy. Outer TLS belongs to the
// deployment ingress; private plaintext/key material stays inside the enclave.
import { createWebRelay } from './server.mjs';
const [listen, upstream, origin] = process.argv.slice(2);
function address(text) {
  const match = /^127\.0\.0\.1:(\d{1,5})$/.exec(text ?? '');
  if (!match || Number(match[1]) > 65535) throw Error('Invalid relay configuration');
  return { host: '127.0.0.1', port: Number(match[1]) };
}
try {
  if (process.argv.length < 4 || process.argv.length > 5) throw Error();
  const relay = createWebRelay({ target: address(upstream), origin });
  relay.server.listen(address(listen));
  relay.server.once('listening', () => console.log(`127.0.0.1:${relay.server.address().port}`));
  relay.server.once('error', () => { console.error('Web relay unavailable'); process.exitCode = 1; void relay.close(); });
  for (const signal of ['SIGTERM','SIGINT']) process.once(signal, () => { void relay.close(); });
} catch { console.error('Invalid relay configuration'); process.exitCode = 1; }
