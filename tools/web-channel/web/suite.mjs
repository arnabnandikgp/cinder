// Synthetic fixture-key trust ONLY. This does not verify an NSM quote.
export async function run({ BrowserEndpoint, standard_vector }, base = '') {
  const checks = [];
  const assert = (value, name) => { if (!value) throw Error(name); };
  const equal = (a, b) => a.length === b.length && a.every((v, i) => v === b[i]);
  const text = new TextEncoder();
  const marker = text.encode('PRIVATE-WEB-QUALIFICATION-NO-PARENT-PLAINTEXT');
  assert(standard_vector(), 'published standard vector');
  checks.push('published standard vector');
  async function endpoint() {
    const response = await fetch(`${base}/new`, { method: 'POST' });
    assert(response.ok, 'fixture setup');
    const { id, key } = await response.json();
    return { id, client: new BrowserEndpoint(Uint8Array.from(key), new Uint8Array(32).fill(1)) };
  }
  async function relay(id, wire) {
    const response = await fetch(`${base}/wire/${id}`, { method: 'POST', body: wire });
    assert(response.ok, 'fixture delivery');
    return new Uint8Array(await response.arrayBuffer());
  }
  async function ready() {
    const { id, client } = await endpoint();
    assert(!client.ready(), 'not ready before handshake');
    const first = client.start();
    assert(first.length === 48, 'empty first handshake');
    const second = await relay(id, first);
    assert(second.length === 48, 'empty second handshake');
    const confirmation = client.advance(second);
    assert(!client.ready(), 'not ready before confirmation');
    assert(client.advance(await relay(id, confirmation)).length === 0, 'confirmation complete');
    assert(client.ready() && client.binding().length === 32, 'ready binding');
    return { id, client };
  }
  function refused(client, call, name) {
    let failure;
    try { call(); } catch (error) { failure = String(error); }
    assert(failure === 'Web channel unavailable' && !client.ready(), name);
    try { client.seal(marker); } catch (error) {
      assert(String(error) === 'Web channel unavailable', 'sticky redaction');
      client.free();
      return;
    }
    throw Error('failed endpoint reused');
  }
  {
    const { id, client } = await ready();
    const wire = client.seal(marker);
    assert(!new TextDecoder().decode(wire).includes(new TextDecoder().decode(marker)), 'ciphertext marker');
    const reply = await relay(id, wire);
    assert(equal(client.open(reply), marker), 'native/browser private round trip');
    const large = new Uint8Array(16384).fill(42);
    assert(equal(client.open(await relay(id, client.seal(large))), large), 'bounded large record');
    refused(client, () => client.open(reply), 'replay refuses');
    checks.push('native round trip, exact bound, replay and sticky failure');
  }
  {
    const { client } = await endpoint();
    refused(client, () => client.seal(marker), 'no early private record');
    checks.push('no early private record');
  }
  {
    const { id, client } = await ready();
    const reply = await relay(id, client.seal(marker));
    reply[0] ^= 1;
    refused(client, () => client.open(reply), 'tampered record');
    checks.push('tamper refuses before plaintext release');
  }
  {
    const { client } = await ready();
    refused(client, () => client.open(client.seal(marker)), 'reflection');
    checks.push('direction reflection refuses');
  }
  {
    const { client } = await ready();
    refused(client, () => client.seal(new Uint8Array(16385)), 'oversize');
    checks.push('oversize refuses');
  }
  {
    // Platform RNG refusal must fail, never switch to Math.random or static keys.
    const { client } = await endpoint();
    const original = globalThis.crypto.getRandomValues;
    let calls = 0;
    try {
      globalThis.crypto.getRandomValues = () => { calls++; throw Error('fixture entropy failure'); };
      refused(client, () => client.start(), 'entropy failure');
      assert(calls > 0, 'platform entropy exercised');
    } finally { globalThis.crypto.getRandomValues = original; }
    checks.push('platform entropy failure without fallback');
  }
  return checks;
}
