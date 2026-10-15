/** Unsigned P23B instruction builders. No RPC, wallet loading, signing or sending. */
import type { Program } from '@anchor-lang/core';
import { PublicKey, SystemProgram } from '@solana/web3.js';
import type { CinderVault } from './cinder_vault.ts';
import { customerAddress, identity, movement, receiptAddress, u64, vaultAddresses } from './index.ts';

const TOKEN = new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
export const ESCROW_PENDING = 0;
export const ESCROW_SWEPT = 1;
export const ESCROW_REFUNDED = 2;

export function venueRouteAddress(program: PublicKey, config: PublicKey) {
  return PublicKey.findProgramAddressSync([Buffer.from('venue_route'), config.toBuffer()], program)[0];
}
export function venueEscrowAddresses(program: PublicKey, config: PublicKey, owner: PublicKey) {
  const route = venueRouteAddress(program, config);
  const escrow = PublicKey.findProgramAddressSync([Buffer.from('venue_escrow'), route.toBuffer(), owner.toBuffer()], program)[0];
  const escrowTokens = PublicKey.findProgramAddressSync([Buffer.from('escrow_tokens'), escrow.toBuffer()], program)[0];
  return { route, escrow, escrowTokens };
}
export function escrowBatchAddress(program: PublicKey, escrow: PublicKey, operation: Uint8Array) {
  return PublicKey.findProgramAddressSync([Buffer.from('escrow_batch'), escrow.toBuffer(), Buffer.from(identity(operation))], program)[0];
}

/** Config is the immutable pool/asset route scope. Venue registration is separately governed.
 * These are expected deployment keys, not evidence that a native credit occurred. */
export type EscrowContext = {
  domain: Uint8Array; pool: Uint8Array; mint: PublicKey; config: PublicKey; owner: PublicKey;
};
export type EscrowAuthorization = { epoch: bigint; operation: Uint8Array; expiresAtSlot: bigint };
function accounts(program: Program<CinderVault>, c: EscrowContext) {
  const derived = vaultAddresses(program.programId, c.domain, c.pool, c.mint);
  if (!derived.config.equals(c.config) || !PublicKey.isOnCurve(c.owner.toBytes())) throw new Error('Escrow route');
  return { config: c.config, mint: c.mint, ...venueEscrowAddresses(program.programId, c.config, c.owner),
    tokenProgram: TOKEN, systemProgram: SystemProgram.programId };
}
function authorization(c: EscrowContext, a: EscrowAuthorization) {
  if (a.expiresAtSlot === 0n) throw new RangeError('Empty deadline');
  return movement(c.domain, a.epoch, a.operation, a.expiresAtSlot);
}

export async function registerVenueRouteInstruction(program: Program<CinderVault>, c: EscrowContext,
  governance: PublicKey, epoch: bigint, venue: Uint8Array) {
  const a = accounts(program, c);
  if (epoch === 0n) throw new RangeError('Empty epoch');
  return program.methods.registerVenueRoute(identity(c.domain), u64(epoch), identity(venue))
    .accountsStrict({ config: a.config, route: a.route, governance, systemProgram: a.systemProgram }).instruction();
}
export async function openVenueEscrowInstruction(program: Program<CinderVault>, c: EscrowContext) {
  return program.methods.openVenueEscrow(identity(c.domain)).accountsStrict({ ...accounts(program, c),
    owner: c.owner, customer: customerAddress(program.programId, c.config, c.owner) }).instruction();
}
export async function depositEscrowInstruction(program: Program<CinderVault>, c: EscrowContext,
  a: EscrowAuthorization, amount: bigint, source: PublicKey) {
  u64(amount);
  if (amount === 0n) throw new RangeError('Empty batch');
  const keys = accounts(program, c), auth = authorization(c, a);
  return program.methods.depositEscrow(auth, u64(amount)).accountsStrict({ ...keys, source, owner: c.owner,
    customer: customerAddress(program.programId, c.config, c.owner),
    batch: escrowBatchAddress(program.programId, keys.escrow, a.operation) }).instruction();
}
export async function sweepEscrowInstruction(program: Program<CinderVault>, c: EscrowContext,
  depositOperation: Uint8Array, a: EscrowAuthorization, funds: PublicKey, brokerTokens: PublicKey, sequence: bigint) {
  const keys = accounts(program, c);
  // No amount parameter. The SBF instruction consumes the exact immutable batch.
  return program.methods.sweepEscrow(authorization(c, a), u64(sequence)).accountsStrict({ ...keys, funds, brokerTokens,
    customer: customerAddress(program.programId, c.config, c.owner),
    batch: escrowBatchAddress(program.programId, keys.escrow, depositOperation),
    receipt: receiptAddress(program.programId, c.config, a.operation) }).instruction();
}
export async function refundEscrowInstruction(program: Program<CinderVault>, c: EscrowContext,
  depositOperation: Uint8Array, destination: PublicKey) {
  const { systemProgram: _system, ...keys } = accounts(program, c);
  return program.methods.refundEscrow(identity(c.domain)).accountsStrict({ ...keys, owner: c.owner, destination,
    batch: escrowBatchAddress(program.programId, keys.escrow, depositOperation) }).instruction();
}
