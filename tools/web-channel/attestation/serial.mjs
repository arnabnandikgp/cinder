// Synthetic certificates only: minimally encode a positive ASN.1 INTEGER from
// 128 bits of public serial entropy. Never use this to generate a private key.
export function serialFromEntropy(entropy) {
  if (!(entropy instanceof Uint8Array) || entropy.length !== 16) throw Error('fixture serial entropy');
  let first = 0;
  while (first < entropy.length && entropy[first] === 0) first++;
  if (first === entropy.length) throw Error('zero fixture serial');
  const magnitude = entropy.slice(first);
  if (!(magnitude[0] & 128)) return magnitude;
  const encoded = new Uint8Array(magnitude.length + 1);
  encoded.set(magnitude, 1);
  return encoded;
}
