import { readFileSync, lstatSync } from 'node:fs';
import { resolve, relative, isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';
import { rootCertificates } from 'node:tls';
import { sha,canonical } from './core.mjs';
export const root=fileURLToPath(new URL('../../',import.meta.url));
const files=['tools/native-qualification/core.mjs','tools/native-qualification/bindings.mjs',
  'tools/native-qualification/artifacts.mjs','tools/native-qualification/prepare.mjs',
  'tools/native-qualification/run.mjs','scripts/native-qualification.test.mjs','clients/vault/tests/native-qualification.test.ts',
  'clients/vault/package.json','clients/vault/package-lock.json','.node-version'];
export const sources=()=>sha(canonical(Object.fromEntries(files.map(f=>[f,sha(readFileSync(resolve(root,f)))]))));
export const tlsRoots=()=>sha(rootCertificates.join('\n'));
export function checkEnvironment() {
  if(process.versions.node!=='24.21.0')throw Error('Pinned Node required');
  for(const name of ['NODE_EXTRA_CA_CERTS','NODE_TLS_REJECT_UNAUTHORIZED','NODE_USE_SYSTEM_CA','NODE_OPTIONS',
    'HTTPS_PROXY','HTTP_PROXY','ALL_PROXY','https_proxy','http_proxy','all_proxy'])if(process.env[name])throw Error('Ambient transport override forbidden');
}
// CLI text pads its display values. Normalize that formatting during offline
// preparation only; an already sealed execution locator must never be rewritten.
export function configuredWallet(config) {
  const lines=config.split(/\r?\n/).filter(line=>line.startsWith('Keypair Path:'));
  if(lines.length!==1)throw Error('One configured sponsor locator required');
  const path=lines[0].slice('Keypair Path:'.length).trim();
  if(!isAbsolute(path)||/[\x00-\x1f\x7f]/.test(path))throw Error('Absolute sponsor locator required');
  return path;
}
// Accept an explicitly supplied old PRIVATE transport config as a locator only.
// Never import its accounts, keys, authority or execution approval.
export function configuredRpc(bytes) {
  const text=Buffer.from(bytes).toString().trim();
  let rpc=text;
  if(text.startsWith('{')) {
    const config=JSON.parse(text);
    if(Object.keys(config).sort().join(',')!=='rpc,wallet'||typeof config.rpc!=='string'||typeof config.wallet!=='string')throw Error('Private RPC locator shape');
    rpc=config.rpc;
  }
  const u=new URL(rpc);
  if(u.origin!=='https://devnet.helius-rpc.com'||u.username||u.password||u.hash||u.pathname!=='/')throw Error('Fixed devnet RPC required');
  return rpc;
}
export function validateSignerLocator(path) {
  if(typeof path!=='string'||path!==path.trim()||!isAbsolute(path)||/[\x00-\x1f\x7f]/.test(path))throw Error('Signer locator shape');
  const st=lstatSync(path);
  if(!st.isFile()||st.isSymbolicLink()||st.uid!==process.getuid()||st.mode&0o077||st.size<1||st.size>8192)throw Error('Signer locator permissions/size');
}
export function runDirectory(path,{exists=true}={}) {
  const dir=resolve(path),scope=resolve(root,'work/experiments/p23-native-semantics');
  if(!/^run-[a-zA-Z0-9-]+$/.test(relative(scope,dir)))throw Error('Fresh native qualification namespace required');
  let walk=root;
  for(const part of relative(root,dir).split('/')) {
    walk=resolve(walk,part);
    try { const st=lstatSync(walk);if(!st.isDirectory()||st.isSymbolicLink())throw Error('Run directory symlink/type'); }
    catch(e) {if(e.code!=='ENOENT'||exists)throw e;}
  }
  return dir;
}
