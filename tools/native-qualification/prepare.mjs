#!/usr/bin/env node
// Offline only: create two fresh disposable identities, never use old wallets.
// Exact live approval remains a separately created, hash-bound private artifact.
import { mkdirSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { join } from 'node:path';
import { web3,ROUTE,ata } from './bindings.mjs';
import { LIMITS,privateRead,exclusive,validateManifest,sha } from './core.mjs';
import { checkEnvironment,configuredWallet,validateSignerLocator,runDirectory,sources,tlsRoots } from './artifacts.mjs';
async function main() {
checkEnvironment();
const [directory,rpcFile]=process.argv.slice(2);if(!directory||!rpcFile||process.argv.length!==4)throw Error('Usage: prepare.mjs RUN_DIRECTORY PRIVATE_RPC_FILE');
const dir=runDirectory(directory,{exists:false}),rpc=privateRead(rpcFile,4096).toString().trim(),u=new URL(rpc);
try { const {lstatSync}=await import('node:fs');lstatSync(dir);throw Error('Run namespace already exists'); }catch(e){if(e.code!=='ENOENT')throw e;}
if(u.origin!=='https://devnet.helius-rpc.com'||u.username||u.password||u.hash||u.pathname!=='/')throw Error('Fixed devnet RPC required');
const config=execFileSync('solana',['config','get'],{encoding:'utf8',env:{...process.env,NO_DNA:'1'}});
const wallet=configuredWallet(config);validateSignerLocator(wallet);
const sponsor=execFileSync('solana',['address'],{encoding:'utf8',env:{...process.env,NO_DNA:'1'}}).trim();
new web3.PublicKey(sponsor);
mkdirSync(dir,{mode:0o700,recursive:true});
const roles={};for(const role of ['owner','broker']) {
  const key=web3.Keypair.generate();roles[role]=key.publicKey.toBase58();
  exclusive(join(dir,`${role}.key`),JSON.stringify([...key.secretKey]));key.secretKey.fill(0);
}
const manifest={schema:'cinder-native-qualification-v1',created:new Date().toISOString(),cluster:'devnet',aws:false,shipping:false,
  native_origin:'https://test-api.pacifica.fi',wss_origin:'wss://test-ws.pacifica.fi/ws',rpc_origin:u.origin,
  genesis:'EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG',owner:roles.owner,broker:roles.broker,sponsor,
  owner_tokens:ata(roles.owner).toBase58(),broker_tokens:ata(roles.broker).toBase58(),route:ROUTE,
  limits:LIMITS,bootstrap_exception:true,lost_native_reply:true,withdraw_uuid:randomUUID(),
  sponsor_transfers:{owner:'35000000',broker:'25000000'},sources:sources(),tls_roots:tlsRoots(),node:process.versions.node,
  rpc_config_hash:sha(rpc),sponsor_locator_hash:sha(wallet),source_bytecode_equivalence:false,
  scope_approval:'User approved 20 faucet USDP, 0.10 devnet SOL, no AWS and bounded bootstrap on 2026-10-08',
  execution_approval:'pending'};
const seal=validateManifest(manifest);
exclusive(join(dir,'config.json'),{rpc,wallet});exclusive(join(dir,'manifest.json'),manifest);
exclusive(join(dir,'REVIEW.md'),`# Exact native-semantics execution record\n\nSeal: ${seal}\n\nCluster: Solana devnet / Pacifica testnet. No AWS or program deployment.\n\n- Sponsor ${sponsor}: 0.035 SOL to owner ${roles.owner}, 0.025 SOL to broker ${roles.broker}. Total sponsor debit (including network fees) must remain below 0.10 SOL.\n- Faucet mints 20 USDP to the fresh owner; transfer exactly 20 to broker; one native deposit of 20; one original UUID withdrawal of 20; return the confirmed net USDP to owner. No other asset route.\n- The documented native withdrawal fee is 1 USDP. The 2-USDP ceiling is a receipt acceptance limit, NOT a signed maximum fee: the native API provides no max-fee field. An unexpected fee stops qualification and remains in the actual evidence.\n- Fee payers: sponsor for the two SOL funding transactions, owner for faucet/token allocation, broker for native deposit/net return. No priority fees; simulation and fee/remaining-balance checks precede signing/exposure.\n- Native program ${ROUTE.program}; mint ${ROUTE.mint} (6 decimals); venue vault ${ROUTE.vault}. Historical deployment slot/upgrade authority must revalidate. The initial executable digest is retained; subsequent sends require unchanged loader slot/authority. This does not prove source equivalence.\n- 20 minutes; 200 native HTTP / 400 RPC / eight finite WSS connections, including cleanup. No trading, agents, borrowing, subaccounts, recovery activation or shipping gate change.\n- Pre-deposit lending-disable check, approved 120-second deposit-first exception for the exact fresh-account rejection. Unknown settings outcomes stop rather than trigger another POST.\n- Original request/wire before exposure; callback-level loss withholds the withdrawal response from controller acceptance. Separate private audit evidence may contain the experiment, never fabricate controller recovery or a complete source cut. No automatic retries, live restart or replacement UUID/signature.\n- Public-code/lock/TLS digests are in manifest.json. Private keys, signed wires, RPC URL and raw traces remain local, mode 0600. JS keys have no secure-erasure guarantee. Unused devnet gas/token-account rent remains in the fresh test accounts; no unapproved account closure or rescue transfer.\n\nExact execution approval is pending. A separate approval.json bound to this seal and a <=24-hour expiry is required before any live request. Preflight rejects cluster/layout/deployment drift and displays/retains simulation evidence before submission; a failed prerequisite stops this invocation.\n`);
console.log(JSON.stringify({prepared:dir,seal,owner:roles.owner,broker:roles.broker,sponsor,live:false}));
}
try { await main(); } catch {console.error('Offline preparation failed. Inspect the local configuration/permissions; no live request was made.');process.exitCode=1;}
