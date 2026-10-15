/** Unsigned owner-paid setup. Explicit provider/RPC only; never signs or sends. */
import type {Program} from '@anchor-lang/core';
import {PublicKey,SystemProgram,type TransactionInstruction} from '@solana/web3.js';
import type {CinderVault} from './cinder_vault.ts';
import type {Discovery} from '../../private/src/index.ts';
import {customerAddress,identity,vaultAddresses} from './index.ts';
import {venueEscrowAddresses,openVenueEscrowInstruction,depositEscrowInstruction,refundEscrowInstruction,type EscrowAuthorization} from './escrow.ts';

/** Validated private discovery mapped onto the generated chain client's context.
 * Addresses are expectations, not RPC authentication or evidence of funding. */
export function onboardingContext(program:Program<CinderVault>,owner:PublicKey,discovery:Discovery){
  const c=discovery.custody;
  if(!program.programId.equals(new PublicKey(c.program))||!PublicKey.isOnCurve(owner.toBytes())||c.epoch<=0n)throw Error('Onboarding route');
  const mint=new PublicKey(c.mint),config=new PublicKey(c.config),route=new PublicKey(c.route);
  const expected=vaultAddresses(program.programId,c.domain,c.pool,mint),derived=venueEscrowAddresses(program.programId,config,owner);
  const recipient=PublicKey.findProgramAddressSync([owner.toBuffer(),new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA').toBuffer(),mint.toBuffer()],
    new PublicKey('ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL'))[0];
  if(!expected.config.equals(config)||!derived.route.equals(route)||!recipient.equals(new PublicKey(discovery.recipient)))throw Error('Onboarding route');
  return {domain:c.domain,pool:c.pool,mint,config,owner};
}

/** Register/open only absent accounts. Existing counters are validated and never
 * reinitialized. A stale/concurrent plan must be rebuilt after checking chain state. */
export async function prepareVenueEscrow(program:Program<CinderVault>,owner:PublicKey,discovery:Discovery):Promise<TransactionInstruction[]>{
  const c=onboardingContext(program,owner,discovery),customer=customerAddress(program.programId,c.config,owner);
  const {route,escrow,escrowTokens}=venueEscrowAddresses(program.programId,c.config,owner);
  const infos=await program.provider.connection.getMultipleAccountsInfo([c.config,route,customer,escrow,escrowTokens],'confirmed');
  const [configInfo,routeInfo,customerInfo,escrowInfo,tokensInfo]=infos;
  const decode=(name:string,info:typeof configInfo)=>{
    if(!info||!info.owner.equals(program.programId)||info.executable)throw Error('Onboarding account');
    return program.coder.accounts.decode(name,info.data);
  };
  const config=decode('vaultConfig',configInfo),registered=decode('venueRoute',routeInfo);
  if(!config.mint.equals(c.mint)||!Buffer.from(config.domain).equals(Buffer.from(c.domain))||!Buffer.from(config.pool).equals(Buffer.from(c.pool))
    ||config.epoch.toString()!==discovery.custody.epoch.toString()||config.mode!==0
    ||!registered.config.equals(c.config)||!Buffer.from(registered.venue).equals(Buffer.from(discovery.custody.venue)))throw Error('Onboarding route');
  const instructions:TransactionInstruction[]=[];
  if(customerInfo){const counter=decode('customerCounter',customerInfo);if(!counter.config.equals(c.config)||!counter.owner.equals(owner))throw Error('Onboarding owner');}
  else instructions.push(await program.methods.registerCustomer(identity(c.domain)).accountsStrict({config:c.config,owner,customer,systemProgram:SystemProgram.programId}).instruction());
  if(escrowInfo){
    const state=decode('venueEscrow',escrowInfo);
    if(!state.route.equals(route)||!state.owner.equals(owner)||!tokensInfo||!tokensInfo.owner.equals(new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'))
      ||tokensInfo.data.length!==165||!new PublicKey(tokensInfo.data.subarray(0,32)).equals(c.mint)||!new PublicKey(tokensInfo.data.subarray(32,64)).equals(escrow))throw Error('Onboarding escrow');
  }else{
    if(tokensInfo)throw Error('Onboarding escrow');
    instructions.push(await openVenueEscrowInstruction(program,c));
  }
  return instructions;
}
/** One exact owner-signed batch; no automatic send, retry or available credit. */
export async function prepareEscrowDeposit(program:Program<CinderVault>,owner:PublicKey,discovery:Discovery,auth:EscrowAuthorization,amount:bigint,source:PublicKey){
  if(auth.epoch!==discovery.custody.epoch||amount>discovery.custody.maximumDeposit)throw Error('Onboarding epoch/limit');
  return depositEscrowInstruction(program,onboardingContext(program,owner,discovery),auth,amount,source);
}
/** Eligible unswept refund returns to the immutable owner ATA, never the broker. */
export async function prepareEscrowRefund(program:Program<CinderVault>,owner:PublicKey,discovery:Discovery,operation:Uint8Array){
  return refundEscrowInstruction(program,onboardingContext(program,owner,discovery),operation,new PublicKey(discovery.recipient));
}
