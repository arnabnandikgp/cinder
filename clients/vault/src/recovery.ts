import { createHash } from 'node:crypto';
import { PublicKey } from '@solana/web3.js';
import { identity, u64 } from './index.ts';

export const MAX_RECOVERY_LEAVES = 65_536;
export const MAX_RECOVERY_PROOF = 16;
const TOKEN = new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
type Digest = Uint8Array;
export interface RecoveryContext {
  program: PublicKey; config: PublicKey; domain: Digest; pool: Digest; mint: PublicKey; decimals: number;
}
export interface RecoveryStatement {
  domain: Digest; epoch: bigint; root: Digest; treeSize: number; total: bigint;
  journalCutoff: bigint; journalHash: Digest; evidenceHash: Digest; policyHash: Digest;
  normalPaid: bigint; fundingSequence: bigint;
  qualification: { unresolvedOperations: bigint; outstandingReservations: bigint; unresolvedInputs: bigint;
    venueExposureZero: boolean; claimsAvailable: boolean };
}
export interface RecoveryClaim {
  index: number; owner: PublicKey; destination: PublicKey; amount: bigint;
  paidBase: bigint; payoutSequenceBase: bigint; claimId: Digest; salt: Digest;
}
const digest = (v: Digest) => Buffer.from(identity(v));
const hash = (...parts: Uint8Array[]) => {
  const h = createHash('sha256'); for (const part of parts) h.update(part); return h.digest();
};
const le64 = (v: bigint) => { u64(v); const b = Buffer.alloc(8); b.writeBigUInt64LE(v); return b; };
const le32 = (v: number) => {
  if (!Number.isInteger(v) || v < 0 || v > 0xffff_ffff) throw new RangeError('Expected unsigned 32-bit index');
  const b = Buffer.alloc(4); b.writeUInt32LE(v); return b;
};
function size(v: number) {
  if (!Number.isInteger(v) || v < 1 || v > MAX_RECOVERY_LEAVES) throw new RangeError('Recovery tree size');
}

/** Domain/asset/pool/final-cutoff binding. Caller supplies the authenticated chain configuration. */
export function recoveryContextHash(c: RecoveryContext, s: RecoveryStatement): Buffer {
  size(s.treeSize);
  if (!Buffer.from(c.domain).equals(Buffer.from(s.domain)) || !Number.isInteger(c.decimals) || c.decimals < 0 || c.decimals > 18
    || s.epoch <= 0n || s.total <= 0n || s.journalCutoff <= 0n) throw new RangeError('Recovery context');
  return hash(Buffer.from('CINDER_RECOVERY_CONTEXT_V1'), c.program.toBuffer(), c.config.toBuffer(),
    digest(c.domain), digest(c.pool), c.mint.toBuffer(), TOKEN.toBuffer(), Uint8Array.of(c.decimals),
    le64(s.epoch), le32(s.treeSize), le64(s.total), le64(s.journalCutoff), digest(s.journalHash),
    digest(s.evidenceHash), digest(s.policyHash), le64(s.normalPaid), le64(s.fundingSequence));
}

export function recoveryLeaf(context: Digest, claim: RecoveryClaim): Buffer {
  if (claim.amount <= 0n) throw new RangeError('Positive final unpaid claim required');
  return hash(Uint8Array.of(0), Buffer.from('CINDER_RECOVERY_CLAIM_V1'), digest(context), le32(claim.index),
    digest(claim.claimId), claim.owner.toBuffer(), claim.destination.toBuffer(), le64(claim.amount),
    le64(claim.paidBase), le64(claim.payoutSequenceBase), digest(claim.salt));
}
const parent = (l: Digest, r: Digest) => hash(Uint8Array.of(1), l, r);
const split = (n: number) => 2 ** Math.floor(Math.log2(n - 1));

/** Pure packaging, not an entitlement calculator or solvency proof. Positive claims must already
 * be finalized by the authoritative ledger; prior payouts are NOT subtracted a second time. */
export function buildRecoveryTree(c: RecoveryContext, s: RecoveryStatement, claims: readonly RecoveryClaim[]) {
  size(claims.length);
  if (s.treeSize !== claims.length) throw new RangeError('Claim count mismatch');
  const contextHash = recoveryContextHash(c, s), owners = new Set<string>(), ids = new Set<string>();
  let total = 0n;
  const leaves = claims.map((claim, index) => {
    const owner = claim.owner.toBase58(), key = digest(claim.claimId).toString('hex');
    if (claim.index !== index || owners.has(owner) || ids.has(key)) throw new RangeError('Duplicate owner/identity or noncanonical index');
    owners.add(owner); ids.add(key); total += claim.amount; u64(total);
    return recoveryLeaf(contextHash, claim);
  });
  if (total !== s.total) throw new RangeError('Final unpaid total mismatch');
  const nodes = new Map<string, Buffer>();
  function tree(start: number, n: number): Buffer {
    const key = `${start}:${n}`, old = nodes.get(key); if (old) return old;
    const k = n === 1 ? 0 : split(n);
    const result = n === 1 ? leaves[start]! : parent(tree(start, k), tree(start + k, n - k));
    nodes.set(key, result); return result;
  }
  const root = tree(0, leaves.length);
  function proof(index: number): Buffer[] {
    if (!Number.isInteger(index) || index < 0 || index >= leaves.length) throw new RangeError('Leaf index');
    function path(start: number, n: number, i: number): Buffer[] {
      if (n === 1) return [];
      const k = split(n);
      return i < k ? [...path(start, k, i), Buffer.from(tree(start + k, n - k))]
        : [...path(start + k, n - k, i - k), Buffer.from(tree(start, k))];
    }
    return path(0, leaves.length, index);
  }
  return { contextHash: Buffer.from(contextHash), root: Buffer.from(root), proof };
}

/** Strict ordered membership only. Caller must separately authenticate the active root,
 * asset/configuration, immutable statement and current lifetime paid-counter basis. */
export function verifyRecoveryProof(root: Digest, leaf: Digest, index: number, treeSize: number, proof: readonly Digest[]): boolean {
  try {
    size(treeSize); le32(index);
    if (index >= treeSize || proof.length > MAX_RECOVERY_PROOF) return false;
    let f = index, s = treeSize - 1, r = digest(leaf);
    for (const sibling of proof) {
      if (s === 0) return false;
      const p = digest(sibling);
      if ((f & 1) !== 0 || f === s) {
        r = parent(p, r);
        while ((f & 1) === 0 && f !== 0) { f >>>= 1; s >>>= 1; }
      } else r = parent(r, p);
      f >>>= 1; s >>>= 1;
    }
    return s === 0 && r.equals(digest(root));
  } catch { return false; }
}

export function recoveryStatementWire(s: RecoveryStatement) {
  size(s.treeSize);
  return { domain: identity(s.domain), epoch: u64(s.epoch), root: identity(s.root), treeSize: s.treeSize,
    total: u64(s.total), journalCutoff: u64(s.journalCutoff), journalHash: identity(s.journalHash),
    evidenceHash: identity(s.evidenceHash), policyHash: identity(s.policyHash), normalPaid: u64(s.normalPaid),
    fundingSequence: u64(s.fundingSequence), qualification: {
      unresolvedOperations: u64(s.qualification.unresolvedOperations), outstandingReservations: u64(s.qualification.outstandingReservations),
      unresolvedInputs: u64(s.qualification.unresolvedInputs), venueExposureZero: s.qualification.venueExposureZero,
      claimsAvailable: s.qualification.claimsAvailable,
    } };
}
export function recoveryClaimWire(c: RecoveryClaim) {
  le32(c.index);
  return { index: c.index, amount: u64(c.amount), paidBase: u64(c.paidBase), payoutSequenceBase: u64(c.payoutSequenceBase),
    claimId: identity(c.claimId), salt: identity(c.salt) };
}
export function recoveryAddress(program: PublicKey, config: PublicKey) {
  return PublicKey.findProgramAddressSync([Buffer.from('recovery'), config.toBuffer()], program)[0];
}
export function recoveryReceiptAddress(program: PublicKey, config: PublicKey, owner: PublicKey) {
  return PublicKey.findProgramAddressSync([Buffer.from('recovery_paid'), config.toBuffer(), owner.toBuffer()], program)[0];
}
