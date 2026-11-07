import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createHash } from 'node:crypto';
import { PublicKey } from '@solana/web3.js';
import { buildRecoveryTree, recoveryContextHash, recoveryLeaf, recoveryClaimWire, recoveryStatementWire,
  verifyRecoveryProof, type RecoveryClaim, type RecoveryContext, type RecoveryStatement } from '../src/recovery.ts';
const id = (s: string) => createHash('sha256').update(`RECOVERY_CODEC_TEST:${s}`).digest();
const key = (s: string) => new PublicKey(id(s));
const context: RecoveryContext = { program: key('program'), config: key('config'), domain: id('domain'), pool: id('pool'), mint: key('mint'), decimals: 6 };
const statement = (n: number): RecoveryStatement => ({ domain: context.domain, epoch: 2n, root: id('placeholder'), treeSize: n,
  total: BigInt(n), journalCutoff: 100n, journalHash: id('journal'), evidenceHash: id('evidence'), policyHash: id('policy'),
  normalPaid: 20n, fundingSequence: 3n, qualification: { unresolvedOperations: 0n, outstandingReservations: 0n,
    unresolvedInputs: 0n, venueExposureZero: true, claimsAvailable: true } });
const claims = (n: number): RecoveryClaim[] => Array.from({ length: n }, (_, index) => ({ index, owner: key(`owner${index}`),
  destination: key(`tokens${index}`), amount: 1n, paidBase: 0n, payoutSequenceBase: 0n, claimId: id(`claim${index}`), salt: id(`salt${index}`) }));

test('ordered proofs agree with independently recursive tree paths for every index in sizes 1 through 65', () => {
  for (let n = 1; n <= 65; n++) {
    const s = statement(n), cs = claims(n), t = buildRecoveryTree(context, s, cs);
    for (const claim of cs) {
      const leaf = recoveryLeaf(t.contextHash, claim), proof = t.proof(claim.index);
      assert(verifyRecoveryProof(t.root, leaf, claim.index, n, proof));
      assert(!verifyRecoveryProof(t.root, leaf, claim.index, n, [...proof, id('extra')]));
      if (proof.length) assert(!verifyRecoveryProof(t.root, leaf, claim.index, n, proof.slice(1)));
      assert(!verifyRecoveryProof(t.root, leaf, n, n, proof));
      assert(!verifyRecoveryProof(t.root, recoveryLeaf(t.contextHash, { ...claim, amount: 2n }), claim.index, n, proof));
    }
  }
});

test('context and leaves bind deployment, pool, asset, precision, cutoff, totals, identity, owner, destination and counters', () => {
  const s = statement(1), claim = claims(1)[0]!, t = buildRecoveryTree(context, s, [claim]);
  for (const change of [{ program: key('other') }, { config: key('other') }, { pool: id('other') }, { mint: key('other') }, { decimals: 9 }]) {
    assert(!t.root.equals(recoveryLeaf(recoveryContextHash({ ...context, ...change }, s), claim)));
  }
  for (const change of [{ epoch: 3n }, { total: 2n }, { journalCutoff: 101n }, { journalHash: id('other') },
    { evidenceHash: id('other') }, { policyHash: id('other') }, { normalPaid: 21n }, { fundingSequence: 4n }]) {
    assert(!t.root.equals(recoveryLeaf(recoveryContextHash(context, { ...s, ...change }), claim)));
  }
  for (const change of [{ owner: key('other') }, { destination: key('other') }, { amount: 2n }, { paidBase: 1n },
    { payoutSequenceBase: 1n }, { claimId: id('other') }, { salt: id('other') }, { index: 1 }]) {
    assert(!t.root.equals(recoveryLeaf(t.contextHash, { ...claim, ...change })));
  }
  assert.throws(() => recoveryContextHash({ ...context, domain: id('other') }, s));
});

test('packager rejects duplicates, noncanonical order, total mismatch, zero claims and integer overflow', () => {
  const s = statement(2), cs = claims(2);
  for (const bad of [[cs[1]!, cs[0]!], [cs[0]!, { ...cs[1]!, owner: cs[0]!.owner }],
    [cs[0]!, { ...cs[1]!, claimId: cs[0]!.claimId }], [cs[0]!, { ...cs[1]!, amount: 0n }]]) {
    assert.throws(() => buildRecoveryTree(context, s, bad));
  }
  assert.throws(() => buildRecoveryTree(context, { ...s, total: 3n }, cs));
  assert.throws(() => buildRecoveryTree(context, { ...s, treeSize: 0 }, []));
  assert.throws(() => recoveryClaimWire({ ...cs[0]!, amount: 2n ** 64n }));
  assert.throws(() => recoveryStatementWire({ ...s, normalPaid: -1n }));
  assert.throws(() => buildRecoveryTree(context, { ...s, total: 2n ** 64n }, [
    { ...cs[0]!, amount: 2n ** 64n - 1n }, cs[1]!,
  ]));
});

test('maximum 65536-leaf tree requires exactly 16 siblings and exposes defensive copies', () => {
  const s = statement(65_536), cs = claims(s.treeSize), t = buildRecoveryTree(context, s, cs);
  for (const index of [0, 32_768, 65_535]) {
    const p = t.proof(index); assert.equal(p.length, 16);
    assert(verifyRecoveryProof(t.root, recoveryLeaf(t.contextHash, cs[index]!), index, s.treeSize, p));
    p[0]!.fill(0); assert(verifyRecoveryProof(t.root, recoveryLeaf(t.contextHash, cs[index]!), index, s.treeSize, t.proof(index)));
  }
  assert.throws(() => t.proof(-1));
  assert.throws(() => recoveryStatementWire({ ...s, treeSize: 65_537 }));
});
