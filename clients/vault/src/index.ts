import { BN, Program, type AnchorProvider, type Idl } from '@anchor-lang/core';
import { PublicKey } from '@solana/web3.js';
import type { CinderVault } from './cinder_vault.ts';
export { CinderVaultErrorCode, type CinderVaultErrorName } from './cinder_vault_errors.ts';

const U64_MAX = (1n << 64n) - 1n;

/** Exact custody atoms, counters and slots. Never accepts floating-point money. */
export function u64(value: bigint): BN {
  if (typeof value !== 'bigint' || value < 0n || value > U64_MAX) {
    throw new RangeError('Expected unsigned 64-bit bigint');
  }
  return new BN(value.toString());
}

/** Copies nonempty 32-byte deployment/operation identities before constructing an instruction. */
export function identity(value: Uint8Array): number[] {
  if (value.length !== 32 || !value.some(byte => byte !== 0)) {
    throw new RangeError('Expected nonzero 32-byte identity');
  }
  return Array.from(value);
}

/** Public instruction authorization, NOT a private entitlement or finality certificate. */
export function movement(domain: Uint8Array, epoch: bigint, operation: Uint8Array, expiresAtSlot: bigint) {
  if (epoch === 0n) throw new RangeError('Authority epoch starts at one');
  return { domain: identity(domain), epoch: u64(epoch), operation: identity(operation), expiresAtSlot: u64(expiresAtSlot) };
}

/** Explicit caller-provided provider and deployment IDL: no ambient wallet, RPC or signing. */
export function vaultProgram(provider: AnchorProvider, deploymentIdl: Idl): Program<CinderVault> {
  if (deploymentIdl.metadata.name !== 'cinder_vault' || deploymentIdl.metadata.version !== '0.1.0') {
    throw new Error('Unsupported vault IDL');
  }
  return new Program<CinderVault>(deploymentIdl as CinderVault, provider);
}

export function vaultAddresses(program: PublicKey, domain: Uint8Array, pool: Uint8Array, mint: PublicKey) {
  const [config] = PublicKey.findProgramAddressSync([
    Buffer.from('cinder_vault'), Buffer.from(identity(domain)), Buffer.from(identity(pool)), mint.toBuffer(),
  ], program);
  const [vault] = PublicKey.findProgramAddressSync([Buffer.from('tokens'), config.toBuffer()], program);
  return { config, vault };
}

export function customerAddress(program: PublicKey, config: PublicKey, owner: PublicKey) {
  return PublicKey.findProgramAddressSync([Buffer.from('customer'), config.toBuffer(), owner.toBuffer()], program)[0];
}

/** Authority-controlled movements share a permanent namespace across rotations. */
export function receiptAddress(program: PublicKey, config: PublicKey, operation: Uint8Array) {
  return PublicKey.findProgramAddressSync([Buffer.from('receipt'), config.toBuffer(), Buffer.from(identity(operation))], program)[0];
}

/** Customer deposits cannot squat a pending operator operation's public receipt address. */
export function depositReceiptAddress(program: PublicKey, config: PublicKey, owner: PublicKey, operation: Uint8Array) {
  return PublicKey.findProgramAddressSync([
    Buffer.from('deposit'), config.toBuffer(), owner.toBuffer(), Buffer.from(identity(operation)),
  ], program)[0];
}
