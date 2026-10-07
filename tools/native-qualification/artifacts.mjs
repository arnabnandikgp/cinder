import { readFileSync, lstatSync } from 'node:fs';
import { resolve, relative } from 'node:path';
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
