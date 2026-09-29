// Independent unbounded-integer oracle. Prints tracked fixture content; no I/O
// besides stdout, no research dependency, and never used by the Rust algorithm.
const low = -(1n << 127n);
const high = (1n << 127n) - 1n;
const mask = (1n << 64n) - 1n;
let seed = 0x517cc1b727220a95n;
function next() {
  seed ^= (seed << 13n) & mask;
  seed ^= seed >> 7n;
  seed ^= (seed << 17n) & mask;
  return seed & mask;
}
function expected(value, multiplier, divisor) {
  const n = value * multiplier;
  const q = n / divisor;
  const r = n % divisor;
  const values = [q, q - (r < 0n ? 1n : 0n), q + (r > 0n ? 1n : 0n)];
  const bounded = result => result < low || result > high ? 'Overflow' : String(result);
  return [...values.map(bounded), r === 0n ? bounded(q) : 'Inexact'];
}
console.log('# P02 independent BigInt oracle; regenerate with scripts/generate-p02-math-vectors.mjs');
console.log('# value,multiplier,divisor,toward_zero,floor,ceil,exact (tab-separated)');
const cases = [
  [low, mask - 1n, mask], [high, mask - 1n, mask],
  [low, mask, mask - 1n], [high, mask, mask - 1n],
  [low, 1n, 3n], [high, 1n, 3n],
  [low, mask, 1n], [high, mask, 1n],
];
for (let i = 0; i < 56; i++) {
  const magnitude = ((next() << 64n) | next()) & high;
  const value = i % 2 ? -magnitude : magnitude;
  cases.push([value, next(), next() || 1n]);
}
for (const [value, multiplier, divisor] of cases) {
  console.log([value, multiplier, divisor, ...expected(value, multiplier, divisor)].join('\t'));
}
