#!/usr/bin/env node
// Explicitly compile offline SDK/service fixtures for the joined SBF recovery
// suite. No download, wallet inheritance, RPC, live venue or AWS operation.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { dirname,resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const bindgen=process.env.CINDER_WASM_BINDGEN??'wasm-bindgen';
assert.equal(process.versions.node,'24.21.0');
const setup='See programs/README.md for the full vault/recovery setup; CINDER_WASM_BINDGEN may select a separately installed binary.';
let bindgenVersion;
try { bindgenVersion=execFileSync(bindgen,['--version'],{encoding:'utf8',stdio:['ignore','pipe','pipe']}).trim(); }
catch { throw new Error(`Recovery tests require wasm-bindgen 0.2.129. ${setup}`); }
assert.equal(bindgenVersion,'wasm-bindgen 0.2.129',`Recovery tests require wasm-bindgen 0.2.129. ${setup}`);
function run(command,args,cwd=root,target=resolve(root,'target')){
  execFileSync(command,args,{cwd,env:{...process.env,CARGO_TARGET_DIR:target},stdio:'inherit'});
}
run('cargo',['build','-p','cinder-service','--features','local-fixture','--bin','cinder-service-fixture','--locked','--offline']);
const tools=resolve(root,'tools/web-channel'),target=resolve(tools,'target');
run(process.execPath,['check-dependencies.mjs'],tools,target);
run('cargo',['build','--lib','--release','--target','wasm32-unknown-unknown','--locked','--offline'],tools,target);
run(bindgen,[resolve(target,'wasm32-unknown-unknown/release/cinder_web_channel_qualification.wasm'),'--target','web','--out-dir','pkg','--out-name','channel'],tools,target);
