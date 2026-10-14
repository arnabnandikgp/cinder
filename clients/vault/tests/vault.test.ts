import assert from 'node:assert/strict';
import { before, test } from 'node:test';
import { readFileSync } from 'node:fs';
import { cp,mkdtemp,mkdir,readFile,rm,writeFile } from 'node:fs/promises';
import { createHash,createPrivateKey,generateKeyPairSync,randomBytes,sign } from 'node:crypto';
import { spawn,type ChildProcessWithoutNullStreams } from 'node:child_process';
import { once } from 'node:events';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { AnchorProvider, BN, type Idl, type Wallet } from '@anchor-lang/core';
import { ComputeBudgetProgram, Connection, Keypair, PublicKey, SystemProgram, Transaction, type TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID, createMint, createAccount, mintTo, getAccount,
  freezeAccount, thawAccount, approve, transfer, burn } from '@solana/spl-token';
import { customerAddress, depositReceiptAddress, identity, movement, receiptAddress, u64, vaultAddresses, vaultProgram } from '../src/index.ts';
import { fundingInstruction, verifyFundingWire } from '../src/funding.ts';
import { venueRouteAddress, venueEscrowAddresses, escrowBatchAddress, registerVenueRouteInstruction,
  openVenueEscrowInstruction, depositEscrowInstruction, sweepEscrowInstruction, refundEscrowInstruction,
  ESCROW_PENDING, ESCROW_SWEPT, ESCROW_REFUNDED } from '../src/escrow.ts';
import { buildRecoveryTree, recoveryAddress, recoveryReceiptAddress, recoveryClaimWire, recoveryStatementWire,
  recoveryContextHash, recoveryLeaf, type RecoveryClaim, type RecoveryStatement } from '../src/recovery.ts';
import { custodySnapshot,custodyWire,decryptKit,independentClaim,recoveryKeyAuthorization,retrieveKit,verifyManifest,type ExpectedCustody,type Snapshot } from '../src/claims.ts';
import { PrivateClient,type MessageSigner } from '../../private/src/index.ts';
import { WebChannel,type WebCore } from '../../private/src/web-channel-core.ts';
// Deliberately fixture-rooted verifier; never accepted by the production client.
import { verifyWithRoot } from '../../../tools/web-channel/attestation/verifier.mjs';
import { runAcceptance } from './acceptance.ts';
// @ts-expect-error parent-only JavaScript framing layer has no custody API
import { createWebRelay } from '../../../services/web-relay/server.mjs';

// In-memory disposable identities only; no wallet file, network URL or deployed key is inherited.
const RPC_URL = 'http://127.0.0.1:18899';
const connection = new Connection(RPC_URL, 'confirmed');
const keys = () => Keypair.generate();
const governance = keys();
const id = (s: string) => createHash('sha256').update(`CINDER_VAULT_TEST:${s}`).digest();
const idl: Idl = JSON.parse(readFileSync(new URL('../idl/cinder_vault.json', import.meta.url), 'utf8'));
const programId = new PublicKey(idl.address);
const loader = new PublicKey('BPFLoaderUpgradeab1e11111111111111111111111');
const programData = PublicKey.findProgramAddressSync([programId.toBuffer()], loader)[0];
// No Program.rpc()/ambient signing in the client under test: transaction signing stays in this fixture.
const wallet = { publicKey: governance.publicKey,
  signTransaction: async () => { throw new Error('No implicit signing'); },
  signAllTransactions: async () => { throw new Error('No implicit signing'); },
} as unknown as Wallet;
const program = vaultProgram(new AnchorProvider(connection, wallet, { commitment: 'confirmed' }), idl);
let number = 0;
let transactionNumber = 0;

async function rpc(method: string, params: unknown[]) {
  const response = await fetch(RPC_URL, { method: 'POST', headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }) });
  const result = await response.json() as { error?: unknown; result: unknown };
  if (result.error) throw new Error(JSON.stringify(result.error));
  return result.result;
}
async function fund(k: Keypair) {
  await rpc('surfnet_setAccount', [k.publicKey.toBase58(), { lamports: 5_000_000_000, owner: SystemProgram.programId.toBase58() }]);
}
async function send(ix: TransactionInstruction, signers: Keypair[]) {
  const unique = [...new Map(signers.map(k => [k.publicKey.toBase58(), k])).values()];
  const block = await connection.getLatestBlockhash();
  // Distinct test wires make replay tests reach the program, not RPC signature deduplication.
  const tx = new Transaction({ feePayer: unique[0].publicKey, ...block }).add(
    ComputeBudgetProgram.setComputeUnitLimit({ units: 200_000 + ++transactionNumber }), ix);
  tx.sign(...unique);
  // Preflight is deliberately off: failed instructions must actually execute in SBF,
  // not only fail in simulation. Fees are excluded from token/state rollback comparisons.
  const signature = await connection.sendRawTransaction(tx.serialize(), { skipPreflight: true });
  const status = await connection.confirmTransaction({ signature, ...block }, 'confirmed');
  if (status.value.err) {
    const detail = await connection.getTransaction(signature, { commitment: 'confirmed', maxSupportedTransactionVersion: 0 });
    throw new Error(JSON.stringify({ error: status.value.err, logs: detail?.meta?.logMessages }));
  }
  return signature;
}
before(async () => {
  await fund(governance);
  // Loading and genesis setup are test cheatcodes. All vault mutations below use signed transactions.
  await rpc('surfnet_writeProgram', [programId.toBase58(),
    readFileSync(new URL('../../../programs/target/deploy/cinder_vault.so', import.meta.url)).toString('hex'),
    0, governance.publicKey.toBase58()]);
});

async function fixture() {
  const funds = keys(), recovery = keys(), broker = keys(), owner = keys(), outsider = keys();
  await Promise.all([funds, recovery, broker, owner, outsider].map(fund));
  const mint = await createMint(connection, governance, governance.publicKey, governance.publicKey, 6);
  const brokerTokens = await createAccount(connection, governance, mint, broker.publicKey);
  const source = await createAccount(connection, governance, mint, owner.publicKey);
  const foreignTokens = await createAccount(connection, governance, mint, outsider.publicKey);
  await mintTo(connection, governance, mint, source, governance, 1_000_000n);
  const domain = id('deployment'), pool = id(`pool-${++number}`);
  const { config, vault } = vaultAddresses(programId, domain, pool, mint);
  const customer = customerAddress(programId, config, owner.publicKey);
  const initAccounts = { governance: governance.publicKey, funds: funds.publicKey, recovery: recovery.publicKey,
    program: programId, programData, mint, brokerTokens, config, vault,
    tokenProgram: TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId };
  const initialize = (overrides = {}) => program.methods.initialize(identity(domain), identity(pool), u64(500_000n), u64(200_000n))
    .accountsStrict({ ...initAccounts, ...overrides }).instruction();
  await send(await initialize(), [governance, funds, recovery]);
  await send(await program.methods.registerCustomer(identity(domain)).accountsStrict({
    config, owner: owner.publicKey, customer, systemProgram: SystemProgram.programId,
  }).instruction(), [owner]);
  const auth = (tag: string, epoch = 1n) => movement(domain, epoch, id(`${number}-${tag}`), (1n << 64n) - 1n);
  const receipt = (a: ReturnType<typeof movement>) => receiptAddress(programId, config, Uint8Array.from(a.operation));
  const depositReceipt = (a: ReturnType<typeof movement>) => depositReceiptAddress(programId, config, owner.publicKey, Uint8Array.from(a.operation));
  const common = { config, vault, mint, tokenProgram: TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId };
  const deposit = (a = auth('deposit'), amount = 400_000n, overrides = {}) => program.methods.deposit(a, u64(amount)).accountsStrict({
    ...common, customer, owner: owner.publicKey, source, receipt: depositReceipt(a), ...overrides,
  }).instruction();
  const release = (a = auth('release'), amount = 100_000n, sequence = 0n, overrides = {}) => program.methods.releaseFunding(a, u64(amount), u64(sequence)).accountsStrict({
    ...common, funds: funds.publicKey, brokerTokens, receipt: receipt(a), ...overrides,
  }).instruction();
  const payout = (a = auth('payout'), amount = 50_000n, paid = 0n, sequence = 0n, overrides = {}) => program.methods.normalPayout(a, u64(amount), u64(paid), u64(sequence)).accountsStrict({
    ...common, funds: funds.publicKey, owner: owner.publicKey, customer, destination: source, receipt: receipt(a), ...overrides,
  }).instruction();
  const returned = (a = auth('return'), amount = 50_000n, overrides = {}) => program.methods.returnFunding(a, u64(amount)).accountsStrict({
    ...common, broker: broker.publicKey, source: brokerTokens, receipt: receipt(a), ...overrides,
  }).instruction();
  const freeze = (epoch = 1n, overrides = {}) => program.methods.freeze(identity(domain), u64(epoch)).accountsStrict({
    config, recovery: recovery.publicKey, ...overrides,
  }).instruction();
  const state = async () => ({ config: await program.account.vaultConfig.fetch(config),
    customer: await program.account.customerCounter.fetch(customer),
    vault: (await getAccount(connection, vault)).amount, source: (await getAccount(connection, source)).amount,
    broker: (await getAccount(connection, brokerTokens)).amount });
  return { funds, recovery, broker, owner, outsider, mint, source, brokerTokens, foreignTokens, domain, pool, config, vault,
    customer, auth, receipt, depositReceipt, initialize, deposit, release, payout, returned, freeze, state };
}
type Fixture = Awaited<ReturnType<typeof fixture>>;

for (const carrier of ['http','websocket'] as const) test(`P22 ${carrier} complete controller lifecycle and process faults`, {timeout:120_000}, async()=>{
  await runAcceptance({program,connection,governance,programData,fund,send},carrier);
});

const signerKey=(k:Keypair)=>createPrivateKey({key:Buffer.concat([Buffer.from('302e020100300506032b657004220420','hex'),Buffer.from(k.secretKey.subarray(0,32))]),format:'der',type:'pkcs8'});
const toBytes=(p:PublicKey|Uint8Array)=>Array.from(p instanceof PublicKey?p.toBytes():p);
async function childLines(child:ChildProcessWithoutNullStreams) {
  let pending:((v:string)=>void)|undefined,failed:((e:Error)=>void)|undefined,buff='';const lines:string[]=[];
  child.stdout.on('data',b=>{buff+=b;while(buff.includes('\n')){const at=buff.indexOf('\n'),line=buff.slice(0,at);buff=buff.slice(at+1);
    if(pending){const p=pending;pending=undefined;failed=undefined;p(line);}else lines.push(line);}});
  let error='';child.stderr.on('data',b=>{error+=b;});child.on('exit',()=>failed?.(new Error(`Joined fixture exited: ${error}`)));
  return async()=>{if(lines.length)return lines.shift()!;if(child.exitCode!==null||child.signalCode!==null)throw Error(`Joined fixture exited: ${error}`);
    return new Promise<string>((resolve,reject)=>{const t=setTimeout(()=>reject(Error('Joined fixture deadline')),10_000);pending=v=>{clearTimeout(t);resolve(v);};failed=e=>{clearTimeout(t);reject(e);};});};
}
async function stopFixture(child:ChildProcessWithoutNullStreams){
  if(child.exitCode!==null||child.signalCode!==null)return;
  const ended=once(child,'exit');child.kill('SIGKILL');await ended;
}
async function chainSnapshot(e:ExpectedCustody):Promise<Snapshot>{
  const {config,vault}=vaultAddresses(e.program,e.domain,e.pool,e.mint),pd=PublicKey.findProgramAddressSync([e.program.toBuffer()],loader)[0];
  const addresses=[e.program,pd,config,vault,e.mint,e.brokerTokens,recoveryAddress(e.program,config),
    ...e.customers.flatMap(o=>[o.tokens,customerAddress(e.program,config,o.wallet),recoveryReceiptAddress(e.program,config,o.wallet)])];
  const result=await connection.getMultipleAccountsInfoAndContext(addresses,{commitment:'finalized'});
  const accounts=new Map<string,NonNullable<typeof result.value[0]>>();result.value.forEach((v,i)=>{if(v)accounts.set(addresses[i]!.toBase58(),v);});
  return {network:e.network,slot:BigInt(result.context.slot),accounts};
}
for(const carrier of ['http','websocket'] as const)test(`P21 ${carrier} accepted tail → outage → recipient package → actual independent SBF claim`,{timeout:90_000},async()=>{
  const f=await fixture(); // Original fixture mint has 6 decimals; joined profile uses exact 0-place atoms.
  // Separate zero-place classic mint/config, no reinterpretation of the existing fixture's units.
  const mint=await createMint(connection,governance,governance.publicKey,null,0);
  const source=await createAccount(connection,governance,mint,f.owner.publicKey);
  const brokerTokens=await createAccount(connection,governance,mint,f.broker.publicKey);
  const venueTokens=await createAccount(connection,governance,mint,f.outsider.publicKey);
  const domain=Buffer.alloc(32,2),pool=id(`joined-${carrier}-${number}`),{config,vault}=vaultAddresses(programId,domain,pool,mint);
  const customer=customerAddress(programId,config,f.owner.publicKey);
  await send(await program.methods.initialize(identity(domain),identity(pool),u64(10_000n),u64(10_000n)).accountsStrict({
    governance:governance.publicKey,funds:f.funds.publicKey,recovery:f.recovery.publicKey,program:programId,programData,mint,brokerTokens,
    config,vault,tokenProgram:TOKEN_PROGRAM_ID,systemProgram:SystemProgram.programId}).instruction(),[governance,f.funds,f.recovery]);
  await send(await program.methods.registerCustomer(identity(domain)).accountsStrict({config,owner:f.owner.publicKey,customer,systemProgram:SystemProgram.programId}).instruction(),[f.owner]);
  await mintTo(connection,governance,mint,source,governance,1000n);
  const deposit=movement(domain,1n,id(`joined-deposit-${carrier}`),(1n<<64n)-1n);
  await send(await program.methods.deposit(deposit,u64(1000n)).accountsStrict({config,vault,mint,customer,owner:f.owner.publicKey,source,
    receipt:depositReceiptAddress(programId,config,f.owner.publicKey,Uint8Array.from(deposit.operation)),tokenProgram:TOKEN_PROGRAM_ID,systemProgram:SystemProgram.programId}).instruction(),[f.owner]);
  // HOUSE collateral starts at the explicit synthetic native venue, not in the
  // Solana vault. A later fake native withdrawal + actual SBF return must settle it.
  await mintTo(connection,governance,mint,venueTokens,governance,1000n);
  const enc=generateKeyPairSync('rsa',{modulusLength:3072}),spki=enc.publicKey.export({format:'der',type:'spki'});
  const ownerKey=signerKey(f.owner),binding={account:f.broker.publicKey.toBase58(),broker_seed:toBytes(f.broker.secretKey.subarray(0,32)),
    keys:[{owner:toBytes(f.owner.publicKey),spki:toBytes(spki),signature:Array.from(sign(null,recoveryKeyAuthorization(domain,pool,spki),ownerKey))}],
    route:{domain:toBytes(domain),pool:toBytes(pool),funds:toBytes(f.funds.publicKey),beneficiaries:[{account:Array(32).fill(1),wallet:toBytes(f.owner.publicKey),tokens:toBytes(source)}],
      program:toBytes(programId),config:toBytes(config),vault:toBytes(vault),mint:toBytes(mint),broker:toBytes(f.broker.publicKey),broker_tokens:toBytes(brokerTokens),
      venue_program:toBytes(f.outsider.publicKey),venue_vault:toBytes(venueTokens),epoch:1,decimals:0,withdrawal:'Qualified',chain:'Qualified',settings:'Qualified',
      withdrawal_cost:10,maximum_movement:10_000,maximum_fee:0,setup_max_age:86_400_000}};
  const directory=await mkdtemp(join(tmpdir(),'cinder-p21-outage-')),store=join(directory,'store');await mkdir(store,{mode:0o700});
  const storageKey=randomBytes(32),configBytes=Buffer.from(JSON.stringify(binding)),prefix=Buffer.alloc(4);prefix.writeUInt32BE(configBytes.length);
  const binary=new URL('../../../target/debug/cinder-service-fixture',import.meta.url).pathname;
  let service:ChildProcessWithoutNullStreams|undefined,worker:ChildProcessWithoutNullStreams|undefined;
  let relay:ReturnType<typeof createWebRelay>|undefined,channel:WebChannel|undefined;
  try {
    const refuseRestart=async(path:string,bytes:Buffer)=>{
      const length=Buffer.alloc(4);length.writeUInt32BE(bytes.length);
      service=spawn(binary,['127.0.0.1:0',path,f.owner.publicKey.toBuffer().toString('hex'),'--recovery-web']);
      const exited=once(service,'exit');let output='';service.stdout.on('data',b=>{output+=b;});
      service.stdin.on('error',()=>{});service.stdin.end(Buffer.concat([storageKey,length,bytes]));
      assert.notEqual((await exited)[0],0);assert.equal(output,'');service=undefined;
    };
    service=spawn(binary,['127.0.0.1:0',store,f.owner.publicKey.toBuffer().toString('hex'),'--recovery-web']);
    const line=await childLines(service);service.stdin.write(Buffer.concat([storageKey,prefix,configBytes]));
    const [address,ca]=(await line()).split(' ');
    relay=createWebRelay({target:{host:'127.0.0.1',port:Number(address!.split(':')[1])}});relay.server.listen(0,'127.0.0.1');await once(relay.server,'listening');
    const coreModule=await import(new URL('../../../tools/web-channel/pkg/channel.js',import.meta.url).href);
    await coreModule.default({module_or_path:await readFile(new URL('../../../tools/web-channel/pkg/channel_bg.wasm',import.meta.url))});
    const core=coreModule as WebCore;
    const policy={network:Buffer.alloc(32,1),deployment:domain,manifest:Buffer.alloc(32,21),pcrs:[Buffer.alloc(48,22),Buffer.alloc(48,23),Buffer.alloc(48,24)] as const};
    channel=await WebChannel.connect({baseUrl:`http://127.0.0.1:${relay.server.address().port}`,policy,core,transport:carrier},(q,p,c)=>verifyWithRoot(q,p,c,Buffer.from(ca!,'hex')));
    const signer:MessageSigner={publicKey:f.owner.publicKey.toBytes(),signMessage:async b=>sign(null,b,ownerKey)};
    const api=new PrivateClient(channel,signer,{domain:policy,account:Buffer.alloc(32,1),policy:1});
    const req=(n:number,command:Parameters<PrivateClient['request']>[0]['command'])=>({id:Buffer.alloc(32,n),epoch:1n,expiresAt:BigInt(Date.now()+60_000),command});
    const order=await api.request(req(40,{kind:'order',market:Buffer.alloc(32,7),lots:2n,minimum:90n,maximum:110n,fee:1n,tif:'GTC',reduceOnly:false,goodUntil:BigInt(Date.now()+60_000)}));
    assert(order.kind==='receipt'&&order.outcome==='dispatched'&&order.possiblyExposed);
    const payout=await api.request(req(41,{kind:'payout',net:20n,maximumFee:0n,allowPartial:false,goodUntil:BigInt(Date.now()+60_000)}));
    assert(payout.kind==='receipt'&&payout.outcome==='accepted');
    const accepted=await readFile(join(store,'accepted'));assert.equal(accepted.length,40);
    channel.close();await relay.close();relay=undefined;await stopFixture(service);service=undefined;
    // A valid epoch-1 restart must succeed with the exact registered keys and
    // recover pending operations without a second registration or journal write.
    service=spawn(binary,['127.0.0.1:0',store,f.owner.publicKey.toBuffer().toString('hex'),'--recovery-web']);
    const resumedLine=await childLines(service);service.stdin.write(Buffer.concat([storageKey,prefix,configBytes]));
    const [resumedAddress,resumedCa]=(await resumedLine()).split(' ');
    relay=createWebRelay({target:{host:'127.0.0.1',port:Number(resumedAddress!.split(':')[1])}});
    relay.server.listen(0,'127.0.0.1');await once(relay.server,'listening');
    channel=await WebChannel.connect({baseUrl:`http://127.0.0.1:${relay.server.address().port}`,policy,core,transport:carrier},(q,p,c)=>verifyWithRoot(q,p,c,Buffer.from(resumedCa!,'hex')));
    const resumedApi=new PrivateClient(channel,signer,{domain:policy,account:Buffer.alloc(32,1),policy:1});
    assert.deepEqual(await resumedApi.request(req(42,{kind:'operation',target:Buffer.alloc(32,40)})),order);
    assert.deepEqual(await resumedApi.request(req(43,{kind:'operation',target:Buffer.alloc(32,41)})),payout);
    assert.deepEqual(await readFile(join(store,'accepted')),accepted);
    channel.close();await relay.close();relay=undefined;await stopFixture(service);service=undefined;
    await refuseRestart(store,Buffer.from(JSON.stringify({...binding,keys:[]})));
    const wrong=generateKeyPairSync('rsa',{modulusLength:3072}),changedSpki=wrong.publicKey.export({format:'der',type:'spki'});
    await refuseRestart(store,Buffer.from(JSON.stringify({...binding,keys:[{owner:binding.keys[0]!.owner,spki:toBytes(changedSpki),
      signature:Array.from(sign(null,recoveryKeyAuthorization(domain,pool,changedSpki),ownerKey))}]})));
    assert.deepEqual(await readFile(join(store,'accepted')),accepted);
    // Isolate the epoch gate: identical valid journal, route and registered keys;
    // ONLY the witness epoch changes in this disposable copy. Never roll back
    // the actual recovery store's fence to make a negative startup test pass.
    const probe=join(directory,'epoch-probe');await cp(store,probe,{recursive:true});
    const epochOnly=Buffer.concat([Buffer.alloc(8),accepted]);epochOnly.writeBigUInt64BE(2n);
    await writeFile(join(probe,'accepted'),epochOnly);
    await refuseRestart(probe,configBytes);assert.deepEqual(await readFile(join(probe,'accepted')),epochOnly);
    // Restore the SAME accepted encrypted journal, not another seeded ledger.
    worker=spawn(binary,[store,'--recovery-worker']);const response=await childLines(worker);worker.stdin.write(Buffer.concat([storageKey,prefix,configBytes]));
    const ask=async(v:unknown)=>{worker!.stdin.write(JSON.stringify(v)+'\n');return JSON.parse(await response());};
    const plan=await ask({op:'payout'}),contract=Buffer.from(plan.contract);
    const built=await fundingInstruction(program,contract),block=await connection.getLatestBlockhash();
    const tx=new Transaction({feePayer:f.funds.publicKey,...block}).add(built.instruction);tx.sign(f.funds);
    const signed=await verifyFundingWire(program,contract,tx.serialize()),signature=await connection.sendRawTransaction(signed.wire,{skipPreflight:true});
    assert.equal((await connection.confirmTransaction({signature,...block},'finalized')).value.err,null);
    const auth=JSON.parse(contract.toString()),receipt=await program.account.movementReceipt.fetch(receiptAddress(programId,config,Uint8Array.from(auth.operation)));
    assert.equal(receipt.amount.toString(),'20');assert.equal(receipt.owner.toBase58(),f.owner.publicKey.toBase58());
    const paid=(await program.account.customerCounter.fetch(customer)).paid.toString();
    await ask({op:'settle',wire:Array.from(signed.wire),signature:Array.from(tx.signature!),slot:await connection.getSlot('finalized'),paid:Number(paid),sequence:Number(receipt.sequence.toString())});
    await send(await program.methods.freeze(identity(domain),u64(1n)).accountsStrict({config,recovery:f.recovery.publicKey}).instruction(),[f.recovery]);
    const returned=await ask({op:'return'}),returnContract=Buffer.from(returned.contract);
    // Synthetic native owner-directed payout. This is NOT a venue integration
    // qualification; all subsequent vault return effects execute actual SBF.
    await transfer(connection,governance,venueTokens,brokerTokens,f.outsider,1000n);
    const returning=await fundingInstruction(program,returnContract),returnBlock=await connection.getLatestBlockhash();
    const returnTx=new Transaction({feePayer:f.broker.publicKey,...returnBlock}).add(returning.instruction);returnTx.sign(f.broker);
    const returnWire=await verifyFundingWire(program,returnContract,returnTx.serialize());
    const returnSignature=await connection.sendRawTransaction(returnWire.wire,{skipPreflight:true});
    assert.equal((await connection.confirmTransaction({signature:returnSignature,...returnBlock},'finalized')).value.err,null);
    await ask({op:'settle',wire:Array.from(returnWire.wire),signature:Array.from(returnTx.signature!),slot:await connection.getSlot('finalized'),paid:20,sequence:1});
    const expected:ExpectedCustody={program:programId,domain,pool,mint,decimals:0,governance:governance.publicKey,funds:f.funds.publicKey,recovery:f.recovery.publicKey,
      broker:f.broker.publicKey,brokerTokens,network:Buffer.alloc(32,1),customers:[{wallet:f.owner.publicKey,tokens:source}]};
    const snapshot=await chainSnapshot(expected),observed=custodySnapshot(program,snapshot,expected);
    assert.equal(observed.vault_amount,1980n);assert.equal(observed.normal_paid,20n);
    // Same-bank parser rejects owner/schema/token/finality/domain substitution.
    for(const [address,offset,value] of [[config,8,2],[vault,108,2],[vault,72,1],[source,129,1]] as const){
      const accounts=new Map(snapshot.accounts),old=accounts.get(address.toBase58())!,data=Buffer.from(old.data);data[offset]=value;
      accounts.set(address.toBase58(),{...old,data});assert.throws(()=>custodySnapshot(program,{...snapshot,accounts},expected));}
    assert.throws(()=>custodySnapshot(program,{...snapshot,network:Buffer.alloc(32,55)},expected));
    assert.throws(()=>custodySnapshot(program,{...snapshot,slot:0n},expected));
    const cmd=`{"op":"finish","custody":${custodyWire(observed).toString()},"owner":${JSON.stringify(toBytes(f.owner.publicKey))},"spki":${JSON.stringify(toBytes(spki))},"signature":${JSON.stringify(binding.keys[0]!.signature)}}\n`;
    worker.stdin.write(cmd);const publication=JSON.parse(await response()),wire=Buffer.from(publication.manifest),locator=Buffer.from(publication.locator);
    const manifest=verifyManifest(wire,sign(null,wire,signerKey(governance)),governance.publicKey);
    assert.equal(manifest.statement.total,980n);assert.equal(manifest.statement.normalPaid,20n);
    assert.throws(()=>verifyManifest(wire,sign(null,wire,ownerKey),governance.publicKey));
    await stopFixture(worker);worker=undefined;
    // Neither the trading process nor recovery worker is needed to fetch/decrypt.
    const first=join(store,'claims-first',locator.toString('hex')),second=join(store,'claims-second',locator.toString('hex'));
    const saved=await readFile(first);await rm(first);
    const kitWire=await retrieveKit(locator,manifest,[async()=>readFile(first),async()=>readFile(second)]);
    assert.deepEqual(Buffer.from(kitWire),saved);
    const kit=decryptKit(kitWire,locator,enc.privateKey,manifest);assert.equal(kit.claim.amount,980n);assert.equal(kit.claim.paidBase,20n);
    assert.equal(kit.claim.destination.toBase58(),source.toBase58());
    const altered=Buffer.from(kitWire);altered[altered.length-2]^=1;assert.throws(()=>decryptKit(altered,locator,enc.privateKey,manifest));
    assert.throws(()=>decryptKit(kitWire,locator,wrong.privateKey,manifest));
    const publicBytes=Buffer.concat([wire,saved]);for(const marker of ['"owner":','"destination":','"paid_base":','"salt":','"proof":'])assert.equal(publicBytes.includes(Buffer.from(marker)),false);
    await rm(second);await assert.rejects(retrieveKit(locator,manifest,[async()=>readFile(first),async()=>readFile(second)]));
    // Saved ciphertext remains independently verifiable even with both stores lost.
    assert.equal(decryptKit(saved,locator,enc.privateKey,manifest).claim.amount,980n);
    const root=recoveryAddress(programId,config);
    await send(await program.methods.stageRecovery(recoveryStatementWire(kit.statement)).accountsStrict({config,governance:governance.publicKey,recoveryEpoch:root,systemProgram:SystemProgram.programId}).instruction(),[governance]);
    await assert.rejects(independentClaim(program,await chainSnapshot(expected),expected,kit)); // STAGED is not ACTIVE.
    await send(await program.methods.activateRecovery(identity(domain),u64(2n),identity(kit.statement.root)).accountsStrict({config,recovery:f.recovery.publicKey,recoveryEpoch:root,vault,tokenProgram:TOKEN_PROGRAM_ID}).instruction(),[f.recovery]);
    const active=await chainSnapshot(expected);
    await assert.rejects(independentClaim(program,active,expected,{...kit,statement:{...kit.statement,epoch:1n}}));
    await send(await independentClaim(program,active,expected,kit),[f.owner]);
    assert.equal((await getAccount(connection,source)).amount,1000n); // 20 normal + 980 recovery, not 960.
    assert.equal((await getAccount(connection,vault)).amount,1000n); // House capital stays distinct.
    assert.equal((await program.account.recoveryEpoch.fetch(root)).remaining.toString(),'0');
    await assert.rejects(independentClaim(program,await chainSnapshot(expected),expected,kit));
    const fenced=await readFile(join(store,'accepted'));
    assert.equal(fenced.length,48);assert.equal(fenced.readBigUInt64BE(),2n);
    await refuseRestart(store,configBytes); // Same keys that reopened epoch 1 cannot reopen a fenced epoch 2.
    assert.deepEqual(await readFile(join(store,'accepted')),fenced);
  } finally {channel?.close();await relay?.close();if(service)await stopFixture(service);if(worker)await stopFixture(worker);
    storageKey.fill(0);binding.broker_seed.fill(0);configBytes.fill(0);await rm(directory,{recursive:true,force:true});}
});
test('funding codec executes the selected release, return and payout rails in SBF', async () => {
  const f=await fixture(); await send(await f.deposit(),[f.owner]);
  for (const rail of ['Release','Return','Payout']) {
    const auth=f.auth(`codec-${rail}`), amount=rail==='Payout'?50_000n:100_000n;
    const bytes=(p:PublicKey|Uint8Array)=>Array.from(p instanceof PublicKey?p.toBytes():p);
    const c:Record<string,unknown>={schema:'cinder-vault-funding-v1',rail,network:bytes(id('offline-sandbox')),
      program:bytes(programId),domain:bytes(f.domain),pool:bytes(f.pool),config:bytes(f.config),vault:bytes(f.vault),
      mint:bytes(f.mint),funds:bytes(f.funds.publicKey),broker:bytes(f.broker.publicKey),broker_tokens:bytes(f.brokerTokens),
      decimals:6,venue_program:bytes(f.outsider.publicKey),venue_vault:bytes(f.foreignTokens),epoch:'1',
      operation:auth.operation,customer:bytes(f.owner.publicKey),amount:amount.toString(),sequence:'0',paid:'0',
      recipient_tokens:rail==='Payout'?bytes(f.source):bytes(Buffer.alloc(32)),expires_at_slot:auth.expiresAtSlot.toString()};
    const contract=Buffer.from(JSON.stringify(Object.fromEntries(Object.keys(c).sort().map(k=>[k,c[k]]))));
    const built=await fundingInstruction(program,contract), signer=rail==='Return'?f.broker:f.funds;
    const block=await connection.getLatestBlockhash();
    const tx=new Transaction({feePayer:signer.publicKey,...block}).add(built.instruction);tx.sign(signer);
    const verified=await verifyFundingWire(program,contract,tx.serialize());
    const signature=await connection.sendRawTransaction(verified.wire,{skipPreflight:true});
    assert.equal((await connection.confirmTransaction({signature,...block},'confirmed')).value.err,null);
    const receipt=await program.account.movementReceipt.fetch(f.receipt(auth));
    assert.equal(receipt.amount.toString(),amount.toString());
    assert.equal(receipt.kind,rail==='Release'?1:rail==='Return'?2:3);
  }
  const state=await f.state();assert.equal(state.broker,0n);assert.equal(state.vault,350_000n);
  assert.equal(state.customer.paid.toString(),'50000');
});
async function rejectsAtomic(f: Fixture, ix: TransactionInstruction, signers: Keypair[], receipt?: PublicKey, error?: RegExp) {
  const before = await f.state();
  if (error) await assert.rejects(send(ix, signers), error);
  else await assert.rejects(send(ix, signers));
  assert.deepEqual(await f.state(), before);
  if (receipt) assert.equal(await connection.getAccountInfo(receipt), null);
}

test('SDK rejects inexact/out-of-domain integers and identities without network or signing', () => {
  assert.equal(u64((1n << 64n) - 1n).toString(), '18446744073709551615');
  for (const bad of [-1n, 1n << 64n, 1.2, Number.MAX_SAFE_INTEGER + 1]) assert.throws(() => u64(bad as bigint));
  assert.throws(() => identity(Buffer.alloc(32)));
  assert.throws(() => identity(Buffer.alloc(31, 1)));
  assert.throws(() => movement(id('d'), 0n, id('o'), 10n));
  const copied = identity(id('copy')); assert.equal(copied.length, 32);
  const mint = keys().publicKey, pool = id('p');
  assert.notDeepEqual(vaultAddresses(programId, id('d1'), pool, mint), vaultAddresses(programId, id('d2'), pool, mint));
});

test('deposit → pooled funding → return → partial payouts preserve public attribution', async () => {
  const f = await fixture();
  await send(await f.deposit(), [f.owner]);
  await send(await f.release(), [f.funds]);
  await send(await f.returned(), [f.broker]);
  await send(await f.payout(), [f.funds]);
  await send(await f.payout(f.auth('payout2'), 20_000n, 50_000n, 1n), [f.funds]);
  const s = await f.state();
  assert.equal(s.vault, 280_000n); assert.equal(s.broker, 50_000n);
  assert.equal(s.config.deposited.toString(), '400000'); assert.equal(s.config.returned.toString(), '50000');
  assert.equal(s.config.paid.toString(), '70000'); assert.equal(s.customer.paid.toString(), '70000');
  assert.equal(s.customer.payoutSequence.toString(), '2');
  const r = await program.account.movementReceipt.fetch(f.receipt(f.auth('payout2')));
  assert.equal(r.amount.toString(), '20000'); assert.equal(r.kind, 3); assert.equal(r.sequence.toString(), '2');
  assert.equal(r.owner.toBase58(), f.owner.publicKey.toBase58());
});

test('initialization cannot be repeated or hijacked by a non-upgrade-authority', async () => {
  const f = await fixture();
  await rejectsAtomic(f, await f.initialize(), [governance, f.funds, f.recovery]);
  const pool = id('hijack'), addresses = vaultAddresses(programId, f.domain, pool, f.mint);
  const ix = await program.methods.initialize(identity(f.domain), identity(pool), u64(100n), u64(100n)).accountsStrict({
    governance: f.outsider.publicKey, funds: f.funds.publicKey, recovery: f.recovery.publicKey,
    program: programId, programData, mint: f.mint, brokerTokens: f.brokerTokens, ...addresses,
    tokenProgram: TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId,
  }).instruction();
  await assert.rejects(send(ix, [f.outsider, f.funds, f.recovery]), /Authority/);
  assert.equal(await connection.getAccountInfo(addresses.config), null);
  assert.equal(await connection.getAccountInfo(addresses.vault), null);
});

for (const [name, modify] of [
  ['wrong domain', (a: ReturnType<typeof movement>) => ({ ...a, domain: identity(id('wrong-domain')) })],
  ['wrong epoch', (a: ReturnType<typeof movement>) => ({ ...a, epoch: new BN(2) })],
  ['expired authorization', (a: ReturnType<typeof movement>) => ({ ...a, expiresAtSlot: new BN(0) })],
] as const) test(`${name} rejects deposit atomically`, async () => {
  const f = await fixture(), a = modify(f.auth('bad'));
  await rejectsAtomic(f, await f.deposit(a), [f.owner], f.depositReceipt(a));
});

test('deposit requires the actual customer signer and source token owner', async () => {
  const f = await fixture();
  await rejectsAtomic(f, await f.deposit(f.auth('foreign-source'), 1n, { source: f.foreignTokens }), [f.owner]);
  const ix = await f.deposit();
  // Mark the required signer read-only/non-signing to exercise the Anchor Signer check itself.
  ix.keys.find(meta => meta.pubkey.equals(f.owner.publicKey))!.isSigner = false;
  await rejectsAtomic(f, ix, [f.outsider], undefined, /AccountNotSigner/);
});

test('wrong mint, token program, vault PDA, owner and discriminator all reject', async () => {
  const f = await fixture();
  const otherMint = await createMint(connection, governance, governance.publicKey, null, 6);
  for (const [name, overrides] of [
    ['mint', { mint: otherMint }], ['token2022', { tokenProgram: TOKEN_2022_PROGRAM_ID }],
    ['program', { tokenProgram: SystemProgram.programId }], ['vault', { vault: f.source }],
    ['customer', { customer: f.config }], ['config', { config: f.customer }],
  ] as const) await rejectsAtomic(f, await f.deposit(f.auth(name), 1n, overrides), [f.owner]);
  const info = (await connection.getAccountInfo(f.customer))!;
  await rpc('surfnet_setAccount', [f.customer.toBase58(), { owner: SystemProgram.programId.toBase58() }]);
  await assert.rejects(send(await f.deposit(f.auth('owner')), [f.owner]), /AccountOwnedByWrongProgram/);
  await rpc('surfnet_setAccount', [f.customer.toBase58(), { owner: info.owner.toBase58() }]);
});

test('zero amounts and delegated source accounts reject', async () => {
  const f = await fixture();
  await rejectsAtomic(f, await f.deposit(f.auth('zero'), 0n), [f.owner], f.depositReceipt(f.auth('zero')));
  await approve(connection, governance, f.source, f.outsider.publicKey, f.owner, 1n);
  await rejectsAtomic(f, await f.deposit(), [f.owner], f.depositReceipt(f.auth('deposit')), /TokenAuthority/);
});

test('deposits cannot squat operator receipts; duplicate and cross-operator-kind receipts reject', async () => {
  const f = await fixture(), a = f.auth('same');
  await send(await f.deposit(a), [f.owner]);
  await rejectsAtomic(f, await f.deposit(a), [f.owner]);
  await send(await f.release(a), [f.funds]);
  await rejectsAtomic(f, await f.payout(a), [f.funds]);
});

test('funding requires funds role, exact route and sequential bounded gross epoch budget', async () => {
  const f = await fixture(); await send(await f.deposit(), [f.owner]);
  await rejectsAtomic(f, await f.release(f.auth('bad-role'), 10n, 0n, { funds: f.recovery.publicKey }), [f.recovery]);
  await rejectsAtomic(f, await f.release(f.auth('bad-route'), 10n, 0n, { brokerTokens: f.foreignTokens }), [f.funds]);
  await rejectsAtomic(f, await f.release(f.auth('bad-seq'), 10n, 1n), [f.funds]);
  await rejectsAtomic(f, await f.release(f.auth('over-cap'), 500_001n), [f.funds]);
  await send(await f.release(f.auth('first'), 400_000n), [f.funds]);
  await send(await f.returned(f.auth('back'), 400_000n), [f.broker]);
  await rejectsAtomic(f, await f.release(f.auth('gross'), 100_001n, 1n), [f.funds]);
  assert.equal((await f.state()).config.epochReleased.toString(), '400000');
});

test('normal payouts bind customer, recipient owner, cap and both compare-and-swap counters', async () => {
  const f = await fixture(); await send(await f.deposit(), [f.owner]);
  for (const ix of [
    await f.payout(f.auth('recipient'), 1n, 0n, 0n, { destination: f.foreignTokens }),
    await f.payout(f.auth('owner'), 1n, 0n, 0n, { owner: f.outsider.publicKey }),
    await f.payout(f.auth('role'), 1n, 0n, 0n, { funds: f.recovery.publicKey }),
    await f.payout(f.auth('cap'), 200_001n), await f.payout(f.auth('paid'), 1n, 1n),
    await f.payout(f.auth('seq'), 1n, 0n, 1n),
  ]) await rejectsAtomic(f, ix, ix.keys.some(k => k.pubkey.equals(f.recovery.publicKey)) ? [f.recovery] : [f.funds]);
  await send(await f.payout(), [f.funds]);
  await rejectsAtomic(f, await f.payout(f.auth('race'), 1n), [f.funds]);
});

test('profit payouts are not incorrectly capped at historical deposits; donations alone create no entitlement', async () => {
  const f = await fixture(); await send(await f.deposit(f.auth('small'), 10n), [f.owner]);
  await transfer(connection, governance, f.source, f.vault, f.owner, 100n);
  const before = await f.state(); assert.equal(before.customer.deposited.toString(), '10');
  await send(await f.payout(f.auth('profit'), 50n), [f.funds]);
  assert.equal((await f.state()).customer.paid.toString(), '50');
});

for (const kind of ['deposit', 'release', 'return', 'payout'] as const) test(`real frozen-token CPI failure rolls back ${kind} counters and receipt`, async () => {
  const f = await fixture();
  if (kind !== 'deposit') await send(await f.deposit(), [f.owner]);
  if (kind === 'return') await send(await f.release(), [f.funds]);
  const account = kind === 'deposit' || kind === 'payout' ? f.source : f.brokerTokens;
  await freezeAccount(connection, governance, account, f.mint, governance);
  const a = f.auth(`cpi-${kind}`);
  const ix = kind === 'deposit' ? await f.deposit(a) : kind === 'release' ? await f.release(a)
    : kind === 'return' ? await f.returned(a) : await f.payout(a);
  await rejectsAtomic(f, ix, kind === 'deposit' ? [f.owner] : kind === 'return' ? [f.broker] : [f.funds],
    kind === 'deposit' ? f.depositReceipt(a) : f.receipt(a), /Tokenkeg.*invoke.*Account is frozen/);
  // Same economic identity succeeds after resolving the external token freeze: no false consumption.
  await thawAccount(connection, governance, account, f.mint, governance);
  await send(ix, kind === 'deposit' ? [f.owner] : kind === 'return' ? [f.broker] : [f.funds]);
});

test('insufficient custody assets do not advance payout/funding counters or consume receipt', async () => {
  const f = await fixture(), a = f.auth('empty');
  await rejectsAtomic(f, await f.payout(a), [f.funds], f.receipt(a));
  await rejectsAtomic(f, await f.release(a), [f.funds], f.receipt(a));
});

test('rotation fences old wires, preserves paid history and cannot reopen a frozen pool', async () => {
  const f = await fixture(); await send(await f.deposit(), [f.owner]); await send(await f.payout(), [f.funds]);
  const newFunds = keys(), newRecovery = keys(); await fund(newFunds); await fund(newRecovery);
  const rotate = (epoch: bigint, overrides = {}) => program.methods.rotate(identity(f.domain), u64(epoch), u64(500_000n), u64(200_000n))
    .accountsStrict({ config: f.config, governance: governance.publicKey, newGovernance: governance.publicKey,
      newFunds: newFunds.publicKey, newRecovery: newRecovery.publicKey, ...overrides }).instruction();
  await rejectsAtomic(f, await rotate(1n, { newRecovery: newFunds.publicKey }), [governance, newFunds], undefined, /RoleAlias/);
  await send(await rotate(1n), [governance, newFunds, newRecovery]);
  await rejectsAtomic(f, await rotate(1n), [governance, newFunds, newRecovery]);
  await rejectsAtomic(f, await f.payout(f.auth('old'), 1n, 50_000n, 1n), [f.funds]);
  await rejectsAtomic(f, await f.payout(f.auth('stale', 1n), 1n, 50_000n, 1n, { funds: newFunds.publicKey }), [newFunds]);
  await send(await f.payout(f.auth('new', 2n), 1n, 50_000n, 1n, { funds: newFunds.publicKey }), [newFunds]);
  assert.equal((await f.state()).customer.paid.toString(), '50001');
  await send(await f.freeze(2n, { recovery: newRecovery.publicKey }), [newRecovery]);
  await rejectsAtomic(f, await rotate(3n), [governance, newFunds, newRecovery]);
  assert.equal((await f.state()).config.epoch.toString(), '3');
});

test('recovery role can only fence; normal paths stop, returned assets remain receivable', async () => {
  const f = await fixture(); await send(await f.deposit(), [f.owner]); await send(await f.release(), [f.funds]);
  await rejectsAtomic(f, await f.freeze(1n, { recovery: f.outsider.publicKey }), [f.outsider]);
  await send(await f.freeze(), [f.recovery]);
  for (const [ix, signer] of [
    [await f.deposit(f.auth('frozen-deposit', 2n)), f.owner],
    [await f.release(f.auth('frozen-release', 2n), 1n, 1n), f.funds],
    [await f.payout(f.auth('frozen-pay', 2n)), f.funds],
  ] as const) await rejectsAtomic(f, ix, [signer], undefined, /Mode/);
  await send(await f.returned(f.auth('frozen-return', 2n)), [f.broker]);
  assert.equal((await f.state()).config.mode, 1);
  // Freeze alone still creates no payable root, and there is no heartbeat/resume/reset instruction.
  assert(!idl.instructions.some(ix => /heartbeat|resume|reset|checkpoint/i.test(ix.name)));
});

test('two customers share one custody vault but never each other\'s deposit or payout counters', async () => {
  const f = await fixture(), other = keys(); await fund(other);
  const otherCustomer = customerAddress(programId, f.config, other.publicKey);
  const otherTokens = await createAccount(connection, governance, f.mint, other.publicKey);
  await mintTo(connection, governance, f.mint, otherTokens, governance, 100_000n);
  await send(await program.methods.registerCustomer(identity(f.domain)).accountsStrict({
    config: f.config, owner: other.publicKey, customer: otherCustomer, systemProgram: SystemProgram.programId,
  }).instruction(), [other]);
  const a = f.auth('shared-id');
  await send(await f.deposit(a, 100_000n), [f.owner]);
  await send(await f.deposit(a, 100_000n, { owner: other.publicKey, customer: otherCustomer, source: otherTokens,
    receipt: depositReceiptAddress(programId, f.config, other.publicKey, Uint8Array.from(a.operation)) }), [other]);
  await rejectsAtomic(f, await f.payout(f.auth('wrong-counter'), 1n, 0n, 0n, { customer: otherCustomer }), [f.funds]);
  await send(await f.payout(), [f.funds]);
  assert.equal((await program.account.customerCounter.fetch(otherCustomer)).paid.toString(), '0');
  assert.equal((await f.state()).customer.paid.toString(), '50000');
  assert.equal((await f.state()).vault, 150_000n);
});

test('prefunded PDAs initialize canonically and customer registration cannot reset paid history', async () => {
  const f = await fixture(); await send(await f.deposit(), [f.owner]); await send(await f.payout(), [f.funds]);
  const before = await f.state();
  await assert.rejects(send(await program.methods.registerCustomer(identity(f.domain)).accountsStrict({
    config: f.config, owner: f.owner.publicKey, customer: f.customer, systemProgram: SystemProgram.programId,
  }).instruction(), [f.owner]));
  assert.deepEqual(await f.state(), before);
  const pool = id('prefunded'), addresses = vaultAddresses(programId, f.domain, pool, f.mint);
  // Real system transfers, not forged initialized account data.
  const systemRent = await connection.getMinimumBalanceForRentExemption(0);
  await send(SystemProgram.transfer({ fromPubkey: governance.publicKey, toPubkey: addresses.config, lamports: systemRent }), [governance]);
  await send(SystemProgram.transfer({ fromPubkey: governance.publicKey, toPubkey: addresses.vault, lamports: systemRent }), [governance]);
  await send(await program.methods.initialize(identity(f.domain), identity(pool), u64(100n), u64(100n)).accountsStrict({
    governance: governance.publicKey, funds: f.funds.publicKey, recovery: f.recovery.publicKey,
    program: programId, programData, mint: f.mint, brokerTokens: f.brokerTokens, ...addresses,
    tokenProgram: TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId,
  }).instruction(), [governance, f.funds, f.recovery]);
  assert.equal((await getAccount(connection, addresses.vault)).owner.toBase58(), addresses.config.toBase58());
});

for (const field of ['paid', 'payoutSequence'] as const) test(`checked ${field} overflow rolls back actual payout instruction`, async () => {
  const f = await fixture(); await send(await f.deposit(), [f.owner]);
  const stored = await program.account.customerCounter.fetch(f.customer);
  const max = (1n << 64n) - 1n;
  const encoded = await program.coder.accounts.encode('customerCounter', { ...stored, [field]: u64(max) });
  // Boundary-state fixture only. Subsequent failure goes through the actual compiled instruction.
  await rpc('surfnet_setAccount', [f.customer.toBase58(), { data: encoded.toString('hex') }]);
  const a = f.auth('overflow');
  await rejectsAtomic(f, await f.payout(a, 1n, field === 'paid' ? max : 0n, field === 'payoutSequence' ? max : 0n),
    [f.funds], f.receipt(a), /Arithmetic/);
});

test('failed freeze epoch increment restores mode and successful governance handoff revokes old governor', async () => {
  const f = await fixture();
  const before = await program.account.vaultConfig.fetch(f.config), max = (1n << 64n) - 1n;
  const encoded = await program.coder.accounts.encode('vaultConfig', { ...before, epoch: u64(max) });
  await rpc('surfnet_setAccount', [f.config.toBase58(), { data: encoded.toString('hex') }]);
  await rejectsAtomic(f, await f.freeze(max), [f.recovery], undefined, /Arithmetic/);
  const restored = await program.coder.accounts.encode('vaultConfig', before);
  await rpc('surfnet_setAccount', [f.config.toBase58(), { data: restored.toString('hex') }]);
  const newGovernance = keys(); await fund(newGovernance);
  const rotate = (governor: PublicKey, epoch: bigint) => program.methods.rotate(identity(f.domain), u64(epoch), u64(100n), u64(100n)).accountsStrict({
    config: f.config, governance: governor, newGovernance: newGovernance.publicKey,
    newFunds: f.funds.publicKey, newRecovery: f.recovery.publicKey,
  }).instruction();
  await send(await rotate(governance.publicKey, 1n), [governance, newGovernance, f.funds, f.recovery]);
  await rejectsAtomic(f, await rotate(governance.publicKey, 2n), [governance, newGovernance, f.funds, f.recovery]);
  await send(await rotate(newGovernance.publicKey, 2n), [newGovernance, f.funds, f.recovery]);
});

async function escrowFixture(existing?: Fixture) {
  const f=existing??await fixture();
  const context = {domain:f.domain,pool:f.pool,mint:f.mint,config:f.config,owner:f.owner.publicKey};
  const addresses = venueEscrowAddresses(programId,f.config,f.owner.publicKey);
  const common = {config:f.config,mint:f.mint,...addresses,tokenProgram:TOKEN_PROGRAM_ID,systemProgram:SystemProgram.programId};
  await send(await registerVenueRouteInstruction(program,context,governance.publicKey,1n,id('pacifica')),[governance]);
  await send(await openVenueEscrowInstruction(program,context),[f.owner]);
  const operations = new Set<string>();
  const batch = (a:ReturnType<typeof movement>) => {
    // Raw harness intentionally permits malformed IDs to exercise SBF, unlike the typed client.
    const b = PublicKey.findProgramAddressSync([Buffer.from('escrow_batch'),addresses.escrow.toBuffer(),Buffer.from(a.operation)],programId)[0];
    operations.add(b.toBase58()); return b;
  };
  const deposit = (a=f.auth('escrow-deposit'),amount=100n,overrides={}) => program.methods.depositEscrow(a,u64(amount))
    .accountsStrict({...common,owner:f.owner.publicKey,customer:f.customer,source:f.source,batch:batch(a),...overrides}).instruction();
  const sweep = (b=f.auth('escrow-deposit'),a=f.auth('escrow-sweep'),sequence=0n,overrides={}) => program.methods.sweepEscrow(a,u64(sequence))
    .accountsStrict({...common,funds:f.funds.publicKey,customer:f.customer,brokerTokens:f.brokerTokens,batch:batch(b),receipt:f.receipt(a),...overrides}).instruction();
  const refund = (b=f.auth('escrow-deposit'),overrides={},domain=f.domain) => program.methods.refundEscrow(identity(domain))
    .accountsStrict({config:f.config,mint:f.mint,...addresses,tokenProgram:TOKEN_PROGRAM_ID,owner:f.owner.publicKey,
      destination:f.source,batch:batch(b),...overrides}).instruction();
  const state = async () => ({custody:await f.state(),escrow:await program.account.venueEscrow.fetch(addresses.escrow),
    tokens:(await getAccount(connection,addresses.escrowTokens)).amount,
    batches:await Promise.all([...operations].map(b=>program.account.escrowBatch.fetchNullable(new PublicKey(b))))});
  const rejects = async (ix:TransactionInstruction,signers:Keypair[],absent?:PublicKey,error?:RegExp) => {
    const before=await state();
    if(error)await assert.rejects(send(ix,signers),error);else await assert.rejects(send(ix,signers));
    assert.deepEqual(await state(),before);
    if(absent)assert.equal(await connection.getAccountInfo(absent),null);
  };
  return {f,context,...addresses,batch,deposit,sweep,refund,state,rejects};
}

test('P23B typed builders execute exact deposit/sweep/return/payout; pending is not credited',async()=>{
  const e=await escrowFixture(),{f}=e,a=f.auth('typed-deposit'),s=f.auth('typed-sweep');
  const auth=(m:ReturnType<typeof movement>)=>({epoch:BigInt(m.epoch.toString()),operation:Uint8Array.from(m.operation),expiresAtSlot:BigInt(m.expiresAtSlot.toString())});
  await send(await depositEscrowInstruction(program,e.context,auth(a),100n,f.source),[f.owner]);
  const pending=await e.state();assert.equal(pending.tokens,100n);assert.equal(pending.escrow.pending.toString(),'100');
  assert.equal(pending.custody.customer.deposited.toString(),'0');assert.equal(pending.custody.config.deposited.toString(),'0');
  await transfer(connection,governance,f.source,e.escrowTokens,f.owner,17n);
  await send(await sweepEscrowInstruction(program,e.context,Uint8Array.from(a.operation),auth(s),f.funds.publicKey,f.brokerTokens,0n),[f.funds]);
  const swept=await program.account.escrowBatch.fetch(e.batch(a)),receipt=await program.account.movementReceipt.fetch(f.receipt(s));
  assert.equal(swept.state,ESCROW_SWEPT);assert.equal(swept.amount.toString(),'100');assert(swept.destination.equals(f.brokerTokens));
  assert.deepEqual(swept.terminalOperation,s.operation);assert.equal(swept.sequence.toString(),'1');assert(swept.terminalSlot.gtn(0));
  assert.equal(receipt.kind,4);assert(receipt.owner.equals(f.owner.publicKey));assert.equal(receipt.amount.toString(),'100');
  const state=await e.state();assert.equal(state.tokens,17n);assert.equal(state.escrow.pending.toString(),'0');
  assert.equal(state.custody.broker,100n);assert.equal(state.custody.vault,0n);assert.equal(state.custody.customer.deposited.toString(),'100');
  // Profit has independent backing; return/payout retain their original counters and recipient.
  await mintTo(connection,governance,f.mint,f.brokerTokens,governance,20n);
  await send(await f.returned(f.auth('escrow-return'),120n),[f.broker]);
  await send(await f.payout(f.auth('escrow-profit'),120n),[f.funds]);
  assert.equal((await f.state()).customer.paid.toString(),'120');
  await e.rejects(await e.refund(a),[f.owner],undefined,/EscrowConsumed/);
  await e.rejects(await e.sweep(a,f.auth('surplus-sweep'),1n),[f.funds],f.receipt(f.auth('surplus-sweep')),/EscrowConsumed/);
  assert.equal((await e.state()).tokens,17n);
});

for(const winner of ['sweep','refund'] as const)test(`P23B ${winner} wins: terminal batch cannot replay, refund, sweep or redeposit`,async()=>{
  const e=await escrowFixture(),{f}=e,a=f.auth('race-batch');await send(await e.deposit(a),[f.owner]);
  const sweep=await e.sweep(a),refund=await e.refund(a);
  await send(winner==='sweep'?sweep:refund,[winner==='sweep'?f.funds:f.owner]);
  await e.rejects(winner==='sweep'?refund:sweep,[winner==='sweep'?f.owner:f.funds],undefined,/EscrowConsumed/);
  await e.rejects(await e.refund(a),[f.owner],undefined,/EscrowConsumed/);
  await e.rejects(await e.sweep(a,f.auth('new-sweep'),winner==='sweep'?1n:0n),[f.funds],f.receipt(f.auth('new-sweep')),/EscrowConsumed/);
  await e.rejects(await e.deposit(a),[f.owner]);
  const state=await e.state(),b=await program.account.escrowBatch.fetch(e.batch(a));
  assert.equal(state.tokens,0n);assert.equal(state.escrow.pending.toString(),'0');
  assert.equal(b.state,winner==='sweep'?ESCROW_SWEPT:ESCROW_REFUNDED);
  assert.equal(state.custody.customer.paid.toString(),'0');assert.equal(state.custody.customer.payoutSequence.toString(),'0');
  if(winner==='refund'){assert(b.destination.equals(f.source));assert.equal(b.sequence.toString(),'0');assert.deepEqual(b.terminalOperation,Array(32).fill(0));}
});

test('P23B competing signed sweep/refund transactions permit exactly one winner',async()=>{
  const e=await escrowFixture(),{f}=e;await send(await e.deposit(),[f.owner]);
  const results=await Promise.allSettled([send(await e.sweep(),[f.funds]),send(await e.refund(),[f.owner])]);
  assert.equal(results.filter(r=>r.status==='fulfilled').length,1);
  const state=await e.state();assert.equal(state.tokens,0n);assert.equal(state.escrow.pending.toString(),'0');
  assert.equal(state.custody.source+state.custody.broker,1_000_000n);
});

test('P23B two owners/batches/routes cannot substitute custody or squat operator identities',async()=>{
  const e=await escrowFixture(),{f}=e,other=keys();await fund(other);
  const otherCustomer=customerAddress(programId,f.config,other.publicKey),otherSource=await createAccount(connection,governance,f.mint,other.publicKey);
  await mintTo(connection,governance,f.mint,otherSource,governance,300n);
  await send(await program.methods.registerCustomer(identity(f.domain)).accountsStrict({config:f.config,owner:other.publicKey,customer:otherCustomer,systemProgram:SystemProgram.programId}).instruction(),[other]);
  const c={...e.context,owner:other.publicKey},otherAddresses=venueEscrowAddresses(programId,f.config,other.publicKey);
  await send(await openVenueEscrowInstruction(program,c),[other]);
  const a=f.auth('one'),b=f.auth('two'),s=f.auth('same-op');
  await send(await e.deposit(a,100n),[f.owner]);await send(await e.deposit(b,200n),[f.owner]);
  await send(await depositEscrowInstruction(program,c,{epoch:1n,operation:Uint8Array.from(a.operation),expiresAtSlot:(1n<<64n)-1n},300n,otherSource),[other]);
  const otherBatch=escrowBatchAddress(programId,otherAddresses.escrow,Uint8Array.from(a.operation));
  await e.rejects(await e.refund(a,{batch:otherBatch}),[f.owner]);
  await e.rejects(await e.sweep(a,s,0n,{customer:otherCustomer}),[f.funds]);
  const second=await escrowFixture();
  await e.rejects(await e.sweep(a,s,0n,{route:second.route}),[f.funds]);
  // The same wallet also has a separate escrow/counter on another route/mint.
  const secondContext={...second.context,owner:f.owner.publicKey},secondAddresses=venueEscrowAddresses(programId,second.f.config,f.owner.publicKey);
  const secondCustomer=customerAddress(programId,second.f.config,f.owner.publicKey),secondSource=await createAccount(connection,governance,second.f.mint,f.owner.publicKey);
  await send(await program.methods.registerCustomer(identity(second.f.domain)).accountsStrict({config:second.f.config,owner:f.owner.publicKey,
    customer:secondCustomer,systemProgram:SystemProgram.programId}).instruction(),[f.owner]);
  await send(await openVenueEscrowInstruction(program,secondContext),[f.owner]);
  await mintTo(connection,governance,second.f.mint,secondSource,governance,40n);
  const secondAuth={epoch:1n,operation:Uint8Array.from(a.operation),expiresAtSlot:(1n<<64n)-1n};
  await send(await depositEscrowInstruction(program,secondContext,secondAuth,40n,secondSource),[f.owner]);
  await e.rejects(await e.refund(a,{batch:escrowBatchAddress(programId,secondAddresses.escrow,secondAuth.operation)}),[f.owner]);
  await send(await sweepEscrowInstruction(program,secondContext,secondAuth.operation,
    {...secondAuth,operation:id('second-route-sweep')},second.f.funds.publicKey,second.f.brokerTokens,0n),[second.f.funds]);
  assert.equal((await program.account.customerCounter.fetch(secondCustomer)).deposited.toString(),'40');
  assert.equal((await getAccount(connection,second.f.brokerTokens)).amount,40n);
  await e.rejects(await e.refund(a,{escrowTokens:otherAddresses.escrowTokens}),[f.owner]);
  // Same owner operation is legitimate in another batch scope, but not another
  // batch in this scope. Deposit IDs cannot consume the operator receipt namespace.
  await send(await e.deposit(s,10n),[f.owner]);await send(await e.sweep(a,s),[f.funds]);
  await e.rejects(await e.sweep(b,s,1n),[f.funds]);
  await send(await e.refund(b),[f.owner]);
  assert.equal((await program.account.venueEscrow.fetch(otherAddresses.escrow)).pending.toString(),'300');
  assert.equal((await getAccount(connection,otherAddresses.escrowTokens)).amount,300n);
  assert.equal((await e.state()).tokens,10n);assert.equal((await e.state()).custody.broker,100n);
});

test('P23B route registration is governed, immutable and cannot relabel a config',async()=>{
  const f=await fixture(),c={domain:f.domain,pool:f.pool,mint:f.mint,config:f.config,owner:f.owner.publicKey},route=venueRouteAddress(programId,f.config);
  await rejectsAtomic(f,await registerVenueRouteInstruction(program,c,f.outsider.publicKey,1n,id('pacifica')),[f.outsider],route);
  await rejectsAtomic(f,await program.methods.registerVenueRoute(identity(f.domain),u64(1n),Array(32).fill(0))
    .accountsStrict({config:f.config,route,governance:governance.publicKey,systemProgram:SystemProgram.programId}).instruction(),[governance],route,/Domain/);
  await send(await registerVenueRouteInstruction(program,c,governance.publicKey,1n,id('pacifica')),[governance]);
  const before=await program.account.venueRoute.fetch(route);
  await assert.rejects(send(await registerVenueRouteInstruction(program,c,governance.publicKey,1n,id('other-venue')),[governance]));
  assert.deepEqual(await program.account.venueRoute.fetch(route),before);assert(before.nativeAccount.equals(f.broker.publicKey));
  const ix=await openVenueEscrowInstruction(program,c);
  ix.keys.find(k=>k.pubkey.equals(f.owner.publicKey))!.isSigner=false;
  await assert.rejects(send(ix,[f.outsider]),/AccountNotSigner/);
  assert.equal(await connection.getAccountInfo(venueEscrowAddresses(programId,f.config,f.owner.publicKey).escrow),null);
});

for(const kind of ['deposit','sweep','refund'] as const)test(`P23B ${kind} rejects hostile accounts and non-signing authorities atomically`,async()=>{
  const e=await escrowFixture(),{f}=e,a=f.auth('bad-accounts');
  if(kind!=='deposit')await send(await e.deposit(a),[f.owner]);
  const signer=kind==='sweep'?f.funds:f.owner,otherMint=await createMint(connection,governance,governance.publicKey,null,6);
  const build=(overrides={})=>kind==='deposit'?e.deposit(a,100n,overrides):kind==='sweep'?e.sweep(a,f.auth('bad-sweep'),0n,overrides):e.refund(a,overrides);
  for(const overrides of [{mint:otherMint},{tokenProgram:TOKEN_2022_PROGRAM_ID},{tokenProgram:SystemProgram.programId},
    {route:f.config},{escrow:f.customer},{escrowTokens:f.source},{batch:f.customer},
    kind==='deposit'?{source:f.foreignTokens}:kind==='sweep'?{brokerTokens:f.foreignTokens}:{destination:f.foreignTokens}]){
    await e.rejects(await build(overrides),[signer]);
  }
  await e.rejects(await build(kind==='sweep'?{funds:f.recovery.publicKey}:{owner:f.outsider.publicKey}),[kind==='sweep'?f.recovery:f.outsider]);
  const ix=await build();ix.keys.find(k=>k.pubkey.equals(signer.publicKey))!.isSigner=false;
  await e.rejects(ix,[f.outsider],undefined,/AccountNotSigner/);
});

test('P23B deposit consent bounds and expiry reject without creating a batch',async()=>{
  const e=await escrowFixture(),{f}=e,a=f.auth('bad-consent');
  for(const [auth,amount] of [[a,0n],[{...a,domain:identity(id('wrong'))},1n],[{...a,epoch:u64(2n)},1n],
    [{...a,expiresAtSlot:u64(0n)},1n],[{...a,operation:Array(32).fill(0)},1n]] as const){
    // Zero identity intentionally bypasses the typed builder to reach SBF validation.
    const batch=PublicKey.findProgramAddressSync([Buffer.from('escrow_batch'),e.escrow.toBuffer(),Buffer.from(auth.operation)],programId)[0];
    await e.rejects(await e.deposit(auth,amount,{batch}),[f.owner],batch);
  }
  await approve(connection,governance,f.source,f.outsider.publicKey,f.owner,1n);
  await e.rejects(await e.deposit(),[f.owner],e.batch(f.auth('escrow-deposit')),/TokenAuthority/);
});

test('P23B sweep validates original consent plus operator deadline, gross cap and sequence',async()=>{
  const e=await escrowFixture(),{f}=e,a=f.auth('consent');await send(await e.deposit(a,400_000n),[f.owner]);
  const s=f.auth('sweep');
  await e.rejects(await e.sweep(a,{...s,expiresAtSlot:u64(0n)}),[f.funds],f.receipt(s),/Expired/);
  await e.rejects(await e.sweep(a,s,1n),[f.funds],f.receipt(s),/Counter/);
  await send(await e.sweep(a,s),[f.funds]);
  await send(await f.returned(f.auth('return-cap'),400_000n),[f.broker]);
  const b=f.auth('above-cap');await send(await e.deposit(b,100_001n),[f.owner]);
  await e.rejects(await e.sweep(b,f.auth('cap'),1n),[f.funds],f.receipt(f.auth('cap')),/Bound/);
  // Legacy release and escrow sweep share the same gross budget and sequence.
  await e.rejects(await f.release(f.auth('legacy-cap'),100_001n,1n),[f.funds],f.receipt(f.auth('legacy-cap')),/Bound/);
  await send(await e.refund(b),[f.owner]);
  const c=f.auth('expired-batch');await send(await e.deposit(c),[f.owner]);
  const stored=await program.account.escrowBatch.fetch(e.batch(c));
  // Test fixture only: isolate an expired original consent, while the new operator wire is fresh.
  const data=await program.coder.accounts.encode('escrowBatch',{...stored,expiresAtSlot:u64(0n)});
  await rpc('surfnet_setAccount',[e.batch(c).toBase58(),{data:data.toString('hex')}]);
  await e.rejects(await e.sweep(c,f.auth('fresh-but-expired'),1n),[f.funds],f.receipt(f.auth('fresh-but-expired')),/Expired/);
  await send(await e.refund(c),[f.owner]);
});

test('P23B rotation invalidates original consent; new epoch cannot revive it; refund survives active recovery',async()=>{
  const e=await escrowFixture(),{f}=e,a=f.auth('before-rotation');await send(await e.deposit(a),[f.owner]);
  await send(await program.methods.rotate(identity(f.domain),u64(1n),u64(500_000n),u64(200_000n)).accountsStrict({
    config:f.config,governance:governance.publicKey,newGovernance:governance.publicKey,newFunds:f.funds.publicKey,newRecovery:f.recovery.publicKey}).instruction(),[governance,f.funds,f.recovery]);
  await e.rejects(await e.sweep(a,f.auth('stale-wire')),[f.funds],f.receipt(f.auth('stale-wire')),/Domain/);
  await e.rejects(await e.sweep(a,f.auth('fresh-wire',2n)),[f.funds],f.receipt(f.auth('fresh-wire',2n)),/Domain/);
  const b=f.auth('new-batch',2n);await send(await e.deposit(b,200n),[f.owner]);
  await send(await f.deposit(f.auth('separate-trading-estate',2n),100n),[f.owner]);
  await send(await f.freeze(2n),[f.recovery]);
  await e.rejects(await e.deposit(f.auth('frozen',3n)),[f.owner],e.batch(f.auth('frozen',3n)),/Mode/);
  await e.rejects(await e.sweep(b,f.auth('frozen-sweep',3n)),[f.funds],f.receipt(f.auth('frozen-sweep',3n)),/Mode/);
  // Independently funded trading recovery cannot seize escrow backing or gate its refunds.
  const r=await recoveryFixture(f);await send(await r.stage(),[governance]);await send(await r.activate(),[f.recovery]);
  const recoveryBefore=await program.account.recoveryEpoch.fetch(r.address);
  assert.equal((await f.state()).vault,100n);
  await e.rejects(await e.refund(a,{},id('wrong-refund-domain')),[f.owner],undefined,/Domain/);
  await send(await refundEscrowInstruction(program,e.context,Uint8Array.from(a.operation),f.source),[f.owner]);
  await send(await e.refund(b),[f.owner]);
  assert.equal((await e.state()).tokens,0n);assert.equal((await f.state()).customer.deposited.toString(),'100');
  assert.deepEqual(await program.account.recoveryEpoch.fetch(r.address),recoveryBefore);
  await send(await r.claim(),[f.owner]);assert.equal((await f.state()).customer.paid.toString(),'100');
});

for(const kind of ['deposit','sweep','refund'] as const)test(`P23B frozen SPL CPI rolls back ${kind}, batch and all counters; same identity succeeds after thaw`,async()=>{
  const e=await escrowFixture(),{f}=e;
  if(kind!=='deposit')await send(await e.deposit(),[f.owner]);
  const target=kind==='deposit'?f.source:kind==='sweep'?f.brokerTokens:f.source;
  await freezeAccount(connection,governance,target,f.mint,governance);
  const ix=kind==='deposit'?await e.deposit():kind==='sweep'?await e.sweep():await e.refund();
  await e.rejects(ix,[kind==='sweep'?f.funds:f.owner],kind==='deposit'?e.batch(f.auth('escrow-deposit')):
    kind==='sweep'?f.receipt(f.auth('escrow-sweep')):undefined,/Tokenkeg.*invoke.*Account is frozen/);
  await thawAccount(connection,governance,target,f.mint,governance);
  await send(ix,[kind==='sweep'?f.funds:f.owner]);
});

test('P23B underbacked escrow cannot consume one claim using another batch; repair restores exact obligation',async()=>{
  const e=await escrowFixture(),{f}=e,a=f.auth('small'),b=f.auth('large');
  await send(await e.deposit(a,10n),[f.owner]);await send(await e.deposit(b,100n),[f.owner]);
  const info=(await connection.getAccountInfo(e.escrowTokens))!,data=Buffer.from(info.data);data.writeBigUInt64LE(50n,64);
  // Synthetic token impairment cannot otherwise be induced by an owner against PDA-held classic SPL.
  await rpc('surfnet_setAccount',[e.escrowTokens.toBase58(),{data:data.toString('hex')}]);
  await e.rejects(await e.sweep(a),[f.funds],f.receipt(f.auth('escrow-sweep')),/EscrowBacking/);
  await e.rejects(await e.refund(a),[f.owner],undefined,/EscrowBacking/);
  await e.rejects(await e.deposit(f.auth('repair-is-not-deposit'),1n),[f.owner],e.batch(f.auth('repair-is-not-deposit')),/EscrowBacking/);
  await rpc('surfnet_setAccount',[e.escrowTokens.toBase58(),{data:info.data.toString('hex')}]);
  await send(await e.sweep(a),[f.funds]);await send(await e.refund(b),[f.owner]);
  assert.equal((await e.state()).tokens,0n);
});

for(const field of ['fundingSequence','deposited','epochReleased'] as const)test(`P23B checked ${field} overflow rolls back terminal consumption`,async()=>{
  const e=await escrowFixture(),{f}=e;await send(await e.deposit(),[f.owner]);
  const stored=await program.account.vaultConfig.fetch(f.config),max=(1n<<64n)-1n;
  const data=await program.coder.accounts.encode('vaultConfig',{...stored,[field]:u64(max)});
  await rpc('surfnet_setAccount',[f.config.toBase58(),{data:data.toString('hex')}]);
  await e.rejects(await e.sweep(f.auth('escrow-deposit'),f.auth('overflow'),field==='fundingSequence'?max:0n),[f.funds],f.receipt(f.auth('overflow')),/Arithmetic/);
  await send(await e.refund(),[f.owner]);
});

test('P23B typed client rejects malformed routes/amounts/identities before network or signing',async()=>{
  const config=keys().publicKey,mint=keys().publicKey,owner=keys().publicKey,domain=id('client-domain'),pool=id('client-pool');
  const c={config:vaultAddresses(programId,domain,pool,mint).config,mint,owner,domain,pool};
  const a={epoch:1n,operation:id('client-op'),expiresAtSlot:100n};
  for(const value of [0n,-1n,1n<<64n,1.5])await assert.rejects(depositEscrowInstruction(program,c,a,value as bigint,owner));
  await assert.rejects(openVenueEscrowInstruction(program,{...c,config}));
  await assert.rejects(depositEscrowInstruction(program,c,{...a,epoch:0n},1n,owner));
  await assert.rejects(depositEscrowInstruction(program,c,{...a,operation:Buffer.alloc(32)},1n,owner));
  await assert.rejects(depositEscrowInstruction(program,c,{...a,expiresAtSlot:0n},1n,owner));
  const precise=await depositEscrowInstruction(program,c,a,9007199254740993n,owner);
  assert.equal(precise.data.readBigUInt64LE(88),9007199254740993n);
  const wire=await sweepEscrowInstruction(program,c,a.operation,a,owner,mint,0n);
  assert.equal(wire.data.length,96); // discriminator + Movement(80) + sequence(8); no amount argument.
  assert(wire.keys.some(k=>k.pubkey.equals(owner)&&k.isSigner));
});

test('P23B preserves all legacy account layouts and appends rather than renumbers errors',()=>{
  const layouts={CustomerCounter:'7f1b3f7a856523896c2488bb5fbc407beda9676d38e4b128bf3346855e879f61',
    MovementReceipt:'0a9efe5ef87d114ff8a47ae5c2c7f3a1b2eac71addd5d0bfd5928a712ff8c3f8',
    RecoveryClaimReceipt:'5164e901a6efe56c06a7ad233a09df2aa52ab6d5ee0452b7273930785b1a8f58',
    RecoveryEpoch:'5bdb9ce5ac9e1fd17f621d43d2789493ae48605d291599a23bc4b9c4db7c07c6',
    VaultConfig:'ad3354dc229cfffa02fef9161d3a78eca8f4952abcdb1c61c5b830c3976129c0'};
  for(const [name,hash] of Object.entries(layouts))assert.equal(createHash('sha256').update(JSON.stringify(idl.types!.find(t=>t.name===name))).digest('hex'),hash);
  const errors=['Domain','Authority','RoleAlias','Mint','TokenAuthority','Mode','Bound','Counter','Expired','Recipient','Arithmetic',
    'RecoveryQualification','RecoveryProof','RecoveryBacking','RecoveryTotal','EscrowConsumed','EscrowBacking'];
  assert.deepEqual(idl.errors!.map(e=>[e.name,e.code]),errors.map((name,index)=>[name,6000+index]));
  for(const [name,size] of [['venueRoute',106],['venueEscrow',83],['escrowBatch',179]] as const)
    assert.equal(program.coder.accounts.size(name),size);
});

test('P23B escrow open requires registration, cannot reset pending custody, and tolerates prefunded PDAs',async()=>{
  const e=await escrowFixture(),{f}=e;await send(await e.deposit(),[f.owner]);
  await e.rejects(await openVenueEscrowInstruction(program,e.context),[f.owner]);
  const other=keys();await fund(other);const c={...e.context,owner:other.publicKey},addresses=venueEscrowAddresses(programId,f.config,other.publicKey);
  await assert.rejects(send(await openVenueEscrowInstruction(program,c),[other]));
  assert.equal(await connection.getAccountInfo(addresses.escrow),null);
  await send(await program.methods.registerCustomer(identity(f.domain)).accountsStrict({config:f.config,owner:other.publicKey,
    customer:customerAddress(programId,f.config,other.publicKey),systemProgram:SystemProgram.programId}).instruction(),[other]);
  const prefund=await connection.getMinimumBalanceForRentExemption(0);
  for(const target of [addresses.escrow,addresses.escrowTokens])await send(SystemProgram.transfer({fromPubkey:other.publicKey,toPubkey:target,lamports:prefund}),[other]);
  await send(await openVenueEscrowInstruction(program,c),[other]);
  assert.equal((await program.account.venueEscrow.fetch(addresses.escrow)).pending.toString(),'0');
  assert.equal((await getAccount(connection,addresses.escrowTokens)).amount,0n);
});

test('P23B customer attribution overflow cannot consume a batch or alter paid history',async()=>{
  const e=await escrowFixture(),{f}=e;await send(await e.deposit(),[f.owner]);
  const c=await program.account.customerCounter.fetch(f.customer);
  const data=await program.coder.accounts.encode('customerCounter',{...c,deposited:u64((1n<<64n)-1n)});
  await rpc('surfnet_setAccount',[f.customer.toBase58(),{data:data.toString('hex')}]);
  await e.rejects(await e.sweep(),[f.funds],f.receipt(f.auth('escrow-sweep')),/Arithmetic/);
  await send(await e.refund(),[f.owner]);assert.equal((await f.state()).customer.paid.toString(),'0');
});

test('P23B pending counter overflow rejects a new deposit without consuming its operation',async()=>{
  const e=await escrowFixture(),{f}=e,max=(1n<<64n)-1n;
  const escrow=await program.account.venueEscrow.fetch(e.escrow),tokens=(await connection.getAccountInfo(e.escrowTokens))!;
  const data=await program.coder.accounts.encode('venueEscrow',{...escrow,pending:u64(max)}),tokenData=Buffer.from(tokens.data);tokenData.writeBigUInt64LE(max,64);
  await rpc('surfnet_setAccount',[e.escrow.toBase58(),{data:data.toString('hex')}]);
  await rpc('surfnet_setAccount',[e.escrowTokens.toBase58(),{data:tokenData.toString('hex')}]);
  await e.rejects(await e.deposit(f.auth('pending-overflow'),1n),[f.owner],e.batch(f.auth('pending-overflow')),/Arithmetic/);
});

async function recoveryFixture(f: Fixture, supplied?: RecoveryClaim[]) {
  const cfg = await program.account.vaultConfig.fetch(f.config), customer = await program.account.customerCounter.fetch(f.customer);
  const claims: RecoveryClaim[] = supplied ?? [{ index: 0, owner: f.owner.publicKey, destination: f.source, amount: 100n,
    paidBase: BigInt(customer.paid.toString()), payoutSequenceBase: BigInt(customer.payoutSequence.toString()),
    claimId: id(`claim-${f.config}`), salt: id(`salt-${f.config}`) }];
  const context = { program: programId, config: f.config, domain: f.domain, pool: f.pool, mint: f.mint, decimals: 6 };
  let statement: RecoveryStatement = { domain: f.domain, epoch: BigInt(cfg.epoch.toString()), root: id('unused-root'),
    treeSize: claims.length, total: claims.reduce((v, c) => v + c.amount, 0n), journalCutoff: 100n,
    journalHash: id('final-journal'), evidenceHash: id('qualified-evidence'), policyHash: id('recovery-policy'),
    normalPaid: BigInt(cfg.paid.toString()), fundingSequence: BigInt(cfg.fundingSequence.toString()),
    qualification: { unresolvedOperations: 0n, outstandingReservations: 0n, unresolvedInputs: 0n,
      venueExposureZero: true, claimsAvailable: true } };
  const tree = buildRecoveryTree(context, statement, claims); statement = { ...statement, root: tree.root };
  const address = recoveryAddress(programId, f.config);
  const receipt = (owner = f.owner.publicKey) => recoveryReceiptAddress(programId, f.config, owner);
  const stage = (s = statement, overrides = {}) => program.methods.stageRecovery(recoveryStatementWire(s)).accountsStrict({
    config: f.config, governance: governance.publicKey, recoveryEpoch: address, systemProgram: SystemProgram.programId, ...overrides,
  }).instruction();
  const activate = (domain = f.domain, epoch = statement.epoch, root = statement.root, overrides = {}) => program.methods
    .activateRecovery(identity(domain), u64(epoch), identity(root)).accountsStrict({ config: f.config, recovery: f.recovery.publicKey,
      recoveryEpoch: address, vault: f.vault, tokenProgram: TOKEN_PROGRAM_ID, ...overrides }).instruction();
  const claim = (c = claims[0]!, proof = tree.proof(c.index), domain = f.domain, epoch = statement.epoch, overrides = {}) => program.methods
    .claimRecovery(identity(domain), u64(epoch), recoveryClaimWire(c), proof.map(identity)).accountsStrict({
      config: f.config, recoveryEpoch: address, owner: c.owner, customer: customerAddress(programId, f.config, c.owner),
      destination: c.destination, vault: f.vault, mint: f.mint, claimReceipt: receipt(c.owner),
      tokenProgram: TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId, ...overrides,
    }).instruction();
  const state = async () => ({ custody: await f.state(), epoch: await program.account.recoveryEpoch.fetchNullable(address) });
  const rejects = async (ix: TransactionInstruction, signers: Keypair[], absent?: PublicKey, error?: RegExp) => {
    const before = await state();
    if (error) await assert.rejects(send(ix, signers), error); else await assert.rejects(send(ix, signers));
    assert.deepEqual(await state(), before);
    if (absent) assert.equal(await connection.getAccountInfo(absent), null);
  };
  return { context, statement, tree, claims, address, receipt, stage, activate, claim, state, rejects };
}

test('recovery requires an explicit fence, publisher and separate operator; heartbeat/time alone never suffices', async () => {
  const f = await fixture(), normal = await recoveryFixture(f);
  await normal.rejects(await normal.stage(), [governance], normal.address, /Mode/);
  await send(await f.freeze(), [f.recovery]);
  const r = await recoveryFixture(f);
  await r.rejects(await r.claim(), [f.owner], r.receipt());
  await r.rejects(await r.activate(), [f.recovery]);
  await r.rejects(await r.stage(undefined, { governance: f.outsider.publicKey }), [f.outsider], r.address);
  await send(await r.stage(), [governance]);
  await r.rejects(await r.activate(undefined, undefined, undefined, { recovery: governance.publicKey }), [governance]);
  await r.rejects(await r.claim(), [f.owner], r.receipt(), /Mode/);
  await r.rejects(await r.activate(), [f.recovery], undefined, /RecoveryBacking/);
  assert.equal((await f.state()).config.mode, 2);
});

test('uncertain finalization, stale counters and malformed final statements reject without publishing an epoch', async () => {
  const f = await fixture(); await send(await f.freeze(), [f.recovery]); const r = await recoveryFixture(f);
  for (const qualification of [{ unresolvedOperations: 1n }, { outstandingReservations: 1n }, { unresolvedInputs: 1n },
    { venueExposureZero: false }, { claimsAvailable: false }]) {
    await r.rejects(await r.stage({ ...r.statement, qualification: { ...r.statement.qualification, ...qualification } }),
      [governance], r.address, /RecoveryQualification/);
  }
  for (const change of [{ normalPaid: 1n }, { fundingSequence: 1n }, { domain: id('wrong-domain') }, { epoch: 1n },
    { total: 0n }, { journalCutoff: 0n }]) await r.rejects(await r.stage({ ...r.statement, ...change }), [governance], r.address);
  const zeroHash = recoveryStatementWire(r.statement); zeroHash.evidenceHash = Array(32).fill(0);
  await r.rejects(await program.methods.stageRecovery(zeroHash).accountsStrict({ config: f.config, governance: governance.publicKey,
    recoveryEpoch: r.address, systemProgram: SystemProgram.programId }).instruction(), [governance], r.address);
});

test('V08 final settled claims preserve ordinary payment history without paying stale amounts or subtracting twice', async () => {
  const f = await fixture(), bob = keys(); await fund(bob);
  const bobCustomer = customerAddress(programId, f.config, bob.publicKey), bobTokens = await createAccount(connection, governance, f.mint, bob.publicKey);
  await mintTo(connection, governance, f.mint, bobTokens, governance, 200n);
  await send(await program.methods.registerCustomer(identity(f.domain)).accountsStrict({ config: f.config, owner: bob.publicKey,
    customer: bobCustomer, systemProgram: SystemProgram.programId }).instruction(), [bob]);
  await send(await f.deposit(f.auth('alice'), 100n), [f.owner]);
  const a = f.auth('bob'); await send(await f.deposit(a, 200n, { owner: bob.publicKey, source: bobTokens, customer: bobCustomer,
    receipt: depositReceiptAddress(programId, f.config, bob.publicKey, Uint8Array.from(a.operation)) }), [bob]);
  await send(await f.payout(f.auth('ordinary'), 20n), [f.funds]);
  // Explicit loss fixture: actual working collateral release and token burn represent
  // the 35-atom settled external loss. This is NOT Pacifica execution evidence.
  await send(await f.release(f.auth('loss'), 35n), [f.funds]);
  await burn(connection, governance, f.brokerTokens, f.mint, f.broker, 35n);
  await send(await f.freeze(), [f.recovery]);
  const cs: RecoveryClaim[] = [
    { index: 0, owner: f.owner.publicKey, destination: f.source, amount: 45n, paidBase: 20n, payoutSequenceBase: 1n, claimId: id('alice'), salt: id('alice-salt') },
    { index: 1, owner: bob.publicKey, destination: bobTokens, amount: 200n, paidBase: 0n, payoutSequenceBase: 0n, claimId: id('bob'), salt: id('bob-salt') },
  ];
  const r = await recoveryFixture(f, cs); await send(await r.stage(), [governance]); await send(await r.activate(), [f.recovery]);
  await r.rejects(await r.claim({ ...cs[0]!, amount: 100n }), [f.owner], r.receipt(), /RecoveryProof/);
  await r.rejects(await r.claim({ ...cs[0]!, amount: 25n }), [f.owner], r.receipt(), /RecoveryProof/);
  await send(await r.claim(cs[0]), [f.owner]);
  await r.rejects(await r.claim(cs[0]), [f.owner]);
  await send(await r.claim(cs[1]), [bob]);
  assert.equal((await f.state()).customer.paid.toString(), '65');
  assert.equal((await program.account.customerCounter.fetch(bobCustomer)).paid.toString(), '200');
  const state = await r.state(); assert.equal(state.custody.config.paid.toString(), '265');
  assert.equal(state.custody.vault, 0n); assert.equal(state.epoch!.remaining.toString(), '0'); assert.equal(state.custody.config.mode, 4);
});

test('active root rejects wrong domain, epoch, asset, recipient, owner, counters and malformed or oversized proof', async () => {
  const f = await fixture(); await send(await f.deposit(), [f.owner]); await send(await f.freeze(), [f.recovery]);
  const r = await recoveryFixture(f); await send(await r.stage(), [governance]);
  await r.rejects(await r.activate(id('other')), [f.recovery]); await r.rejects(await r.activate(undefined, 1n), [f.recovery]);
  await r.rejects(await r.activate(undefined, undefined, id('other')), [f.recovery]); await send(await r.activate(), [f.recovery]);
  const c = r.claims[0]!, otherDestination = await createAccount(connection, governance, f.mint, f.owner.publicKey, keys());
  const otherMint = await createMint(connection, governance, governance.publicKey, null, 6);
  for (const ix of [await r.claim(c, [], id('other')), await r.claim(c, [], undefined, 1n),
    await r.claim({ ...c, destination: otherDestination }), await r.claim({ ...c, salt: id('other') }),
    await r.claim({ ...c, claimId: id('other') }), await r.claim({ ...c, paidBase: 1n }),
    await r.claim({ ...c, payoutSequenceBase: 1n }), await r.claim({ ...c, index: 1 }, []),
    await r.claim(c, [id('extra')]), await r.claim(c, Array(17).fill(id('extra'))),
    await r.claim(c, [], undefined, undefined, { mint: otherMint }),
    await r.claim(c, [], undefined, undefined, { destination: f.foreignTokens }),
    await r.claim(c, [], undefined, undefined, { tokenProgram: TOKEN_2022_PROGRAM_ID }),
    await r.claim(c, [], undefined, undefined, { vault: f.source }),
    await r.claim(c, [], undefined, undefined, { recoveryEpoch: f.customer }),
  ]) await r.rejects(ix, [f.owner], r.receipt());
  await r.rejects(await r.claim(c, [], undefined, undefined, { owner: f.outsider.publicKey }), [f.outsider]);
  const unsigned = await r.claim(); unsigned.keys.find(k => k.pubkey.equals(f.owner.publicKey))!.isSigner = false;
  await r.rejects(unsigned, [f.outsider], r.receipt(), /AccountNotSigner/);
  await approve(connection, governance, f.source, f.outsider.publicKey, f.owner, 1n);
  await r.rejects(await r.claim(), [f.owner], r.receipt(), /TokenAuthority/);
});

test('CPI failure leaves root, shared counters and receipt unconsumed; identical claim succeeds after repair', async () => {
  const f = await fixture(); await send(await f.deposit(), [f.owner]); await send(await f.freeze(), [f.recovery]); const r = await recoveryFixture(f);
  await send(await r.stage(), [governance]); await send(await r.activate(), [f.recovery]);
  await freezeAccount(connection, governance, f.source, f.mint, governance);
  await r.rejects(await r.claim(), [f.owner], r.receipt(), /Tokenkeg.*invoke.*Account is frozen/);
  await thawAccount(connection, governance, f.source, f.mint, governance); await send(await r.claim(), [f.owner]);
  assert.equal((await f.state()).customer.paid.toString(), '100');
});

test('funded but frozen custody cannot activate; repaired custody preserves the same final statement', async () => {
  const f = await fixture(); await send(await f.deposit(), [f.owner]); await send(await f.freeze(), [f.recovery]); const r = await recoveryFixture(f);
  await send(await r.stage(), [governance]); await freezeAccount(connection, governance, f.vault, f.mint, governance);
  await r.rejects(await r.activate(), [f.recovery], undefined, /RecoveryBacking/);
  await thawAccount(connection, governance, f.vault, f.mint, governance); await send(await r.activate(), [f.recovery]);
  await send(await r.claim(), [f.owner]);
});

test('backing impairment blocks even the first small claim without deleting obligations; returned backing permits retry', async () => {
  const f = await fixture(); await send(await f.deposit(), [f.owner]); await send(await f.release(), [f.funds]);
  await send(await f.freeze(), [f.recovery]); const r = await recoveryFixture(f);
  await send(await r.stage(), [governance]); await send(await r.activate(), [f.recovery]);
  const info = (await connection.getAccountInfo(f.vault))!, impaired = Buffer.from(info.data);
  // Test-only external custody impairment; not an instruction capable of spending vault tokens.
  impaired.writeBigUInt64LE(99n, 64); await rpc('surfnet_setAccount', [f.vault.toBase58(), { data: impaired.toString('hex') }]);
  await r.rejects(await r.claim(), [f.owner], r.receipt(), /RecoveryBacking/);
  assert.equal((await r.state()).epoch!.remaining.toString(), '100');
  await send(await f.returned(f.auth('repair', 2n), 1n), [f.broker]); await send(await r.claim(), [f.owner]);
});

test('root cannot be replaced or counters reset; ordinary deposits/payouts stay fenced and late returns create no new claims', async () => {
  const f = await fixture(); await send(await f.deposit(), [f.owner]); await send(await f.release(), [f.funds]);
  await send(await f.freeze(), [f.recovery]); const r = await recoveryFixture(f);
  await send(await r.stage(), [governance]); await r.rejects(await r.stage({ ...r.statement, root: id('other') }), [governance]);
  await send(await r.activate(), [f.recovery]); await r.rejects(await r.activate(), [f.recovery]);
  await r.rejects(await f.payout(f.auth('normal', 2n)), [f.funds], undefined, /Mode/);
  await r.rejects(await f.deposit(f.auth('late', 2n), 1n), [f.owner], f.depositReceipt(f.auth('late', 2n)), /Mode/);
  await r.rejects(await f.release(f.auth('outgoing', 2n), 1n, 1n), [f.funds], undefined, /Mode/);
  await send(await f.returned(f.auth('late-return', 2n), 50n), [f.broker]);
  await transfer(connection, governance, f.source, f.vault, f.owner, 1n); // Unsolicited token arrival, not admission.
  assert.equal((await r.state()).epoch!.remaining.toString(), '100');
  assert.equal((await f.state()).customer.deposited.toString(), '400000');
  await send(await r.claim(), [f.owner]); await r.rejects(await r.stage(), [governance]);
  await r.rejects(await f.freeze(2n), [f.recovery], undefined, /Mode/);
});

for (const n of [1, 3, 5]) test(`actual SBF validates every leaf of ordered ${n}-leaf tree, including odd right-edge paths`, async () => {
  const f = await fixture(), signers = [f.owner], cs: RecoveryClaim[] = [];
  await send(await f.deposit(), [f.owner]);
  for (let index = 0; index < n; index++) {
    const owner = index === 0 ? f.owner : keys();
    const destination = index === 0 ? f.source : await createAccount(connection, governance, f.mint, owner.publicKey);
    if (index !== 0) {
      await fund(owner); signers.push(owner);
      await send(await program.methods.registerCustomer(identity(f.domain)).accountsStrict({ config: f.config, owner: owner.publicKey,
        customer: customerAddress(programId, f.config, owner.publicKey), systemProgram: SystemProgram.programId }).instruction(), [owner]);
    }
    cs.push({ index, owner: owner.publicKey, destination, amount: 10n, paidBase: 0n, payoutSequenceBase: 0n,
      claimId: id(`claim${index}`), salt: id(`salt${index}`) });
  }
  await send(await f.freeze(), [f.recovery]); const r = await recoveryFixture(f, cs);
  await send(await r.stage(), [governance]); await send(await r.activate(), [f.recovery]);
  for (let i = n - 1; i >= 0; i--) {
    const p = r.tree.proof(i);
    if (p.length) await r.rejects(await r.claim(cs[i], p.slice(1)), [signers[i]!], r.receipt(cs[i]!.owner));
    await send(await r.claim(cs[i]), [signers[i]!]);
  }
  assert.equal((await f.state()).config.mode, 4); assert.equal((await r.state()).epoch!.claimedCount, n);
});

test('maximum bounded proof fits one real transaction and executes in SBF within the test compute limit', async () => {
  const f = await fixture(); await send(await f.deposit(), [f.owner]); await send(await f.freeze(), [f.recovery]);
  const cs: RecoveryClaim[] = Array.from({ length: 65_536 }, (_, index) => ({ index,
    owner: index === 0 ? f.owner.publicKey : new PublicKey(id(`max-owner${index}`)), destination: f.source,
    amount: 1n, paidBase: 0n, payoutSequenceBase: 0n, claimId: id(`max-claim${index}`), salt: id(`max-salt${index}`) }));
  const r = await recoveryFixture(f, cs); await send(await r.stage(), [governance]); await send(await r.activate(), [f.recovery]);
  const signature = await send(await r.claim(cs[0]), [f.owner]);
  const tx = (await connection.getTransaction(signature, { commitment: 'confirmed', maxSupportedTransactionVersion: 0 }))!;
  assert(tx.meta!.computeUnitsConsumed! < 200_000);
  const block = await connection.getLatestBlockhash(), wire = new Transaction({ feePayer: f.owner.publicKey, ...block }).add(await r.claim(cs[0]));
  wire.sign(f.owner); assert(wire.serialize().length <= 1232);
  console.log(`Recovery capacity fixture: ${wire.serialize().length} bytes; ${tx.meta!.computeUnitsConsumed} CU; 16 siblings`);
});

test('ordinary Merkle inclusion is not a total/completeness proof: a publisher-understated sum preserves the unpaid remainder', async () => {
  const f = await fixture(); await send(await f.deposit(), [f.owner]); await send(await f.freeze(), [f.recovery]); const r = await recoveryFixture(f);
  const c = { ...r.claims[0]!, amount: 101n }, contextHash = recoveryContextHash(r.context, r.statement);
  const statement = { ...r.statement, root: recoveryLeaf(contextHash, c) };
  // Deliberately bypass the checked packager to demonstrate the trusted publisher boundary.
  await send(await r.stage(statement), [governance]); await send(await r.activate(undefined, undefined, statement.root), [f.recovery]);
  await r.rejects(await r.claim(c, []), [f.owner], r.receipt(), /RecoveryBacking/);
  assert.equal((await r.state()).epoch!.remaining.toString(), '100');
});

test('overdeclared final sum does not erase surplus obligations or close the epoch after the last leaf', async () => {
  const f = await fixture(); await send(await f.deposit(), [f.owner]); await send(await f.freeze(), [f.recovery]); const r = await recoveryFixture(f);
  const c = { ...r.claims[0]!, amount: 99n }, statement = { ...r.statement, root: recoveryLeaf(recoveryContextHash(r.context, r.statement), c) };
  await send(await r.stage(statement), [governance]); await send(await r.activate(undefined, undefined, statement.root), [f.recovery]);
  const before = await getAccount(connection, f.source);
  await send(await r.claim(c, []), [f.owner]);
  assert.equal((await getAccount(connection, f.source)).amount - before.amount, 99n);
  const after = await r.state();
  assert.equal(after.epoch!.remaining.toString(), '1'); assert.equal(after.epoch!.claimedCount, 1);
  assert.equal(after.custody.config.mode, 3); assert.equal(after.custody.customer.paid.toString(), '99');
  assert.equal(after.custody.customer.payoutSequence.toString(), '1');
  await r.rejects(await r.claim(c, []), [f.owner]);
  await r.rejects(await f.payout(f.auth('normal', 2n)), [f.funds], undefined, /Mode/);
});

for (const order of [[0, 1], [1, 0]]) test(`overdeclared two-leaf estate pays both valid owners in order ${order} and retains the remainder`, async () => {
  const f = await fixture(), second = keys(); await fund(second);
  const destination = await createAccount(connection, governance, f.mint, second.publicKey);
  await send(await program.methods.registerCustomer(identity(f.domain)).accountsStrict({ config: f.config, owner: second.publicKey,
    customer: customerAddress(programId, f.config, second.publicKey), systemProgram: SystemProgram.programId }).instruction(), [second]);
  await send(await f.deposit(), [f.owner]); await send(await f.freeze(), [f.recovery]);
  const cs: RecoveryClaim[] = [f.owner, second].map((owner, index) => ({ index, owner: owner.publicKey,
    destination: index === 0 ? f.source : destination, amount: index === 0 ? 40n : 59n,
    paidBase: 0n, payoutSequenceBase: 0n, claimId: id(`overdeclared-claim-${index}`), salt: id(`overdeclared-salt-${index}`) }));
  const r = await recoveryFixture(f, cs), statement = { ...r.statement, total: 100n };
  // Bypass checked packaging: a valid inclusion proof does not establish the published total.
  const context = recoveryContextHash(r.context, statement), leaves = cs.map(c => recoveryLeaf(context, c));
  const root = createHash('sha256').update(Uint8Array.of(1)).update(leaves[0]!).update(leaves[1]!).digest();
  await send(await r.stage({ ...statement, root }), [governance]);
  await send(await r.activate(undefined, undefined, root), [f.recovery]);
  for (const index of order) {
    const c = cs[index]!, before = await getAccount(connection, c.destination);
    await send(await r.claim(c, [leaves[1 - index]!]), [[f.owner, second][index]!]);
    assert.equal((await getAccount(connection, c.destination)).amount - before.amount, c.amount);
  }
  const after = await r.state();
  assert.equal(after.epoch!.remaining.toString(), '1'); assert.equal(after.epoch!.claimedCount, 2);
  assert.equal(after.custody.config.mode, 3); assert.equal(after.custody.config.paid.toString(), '99');
  for (const index of order) await r.rejects(await r.claim(cs[index], [leaves[1 - index]!]), [[f.owner, second][index]!]);
});

test('a malicious duplicate-owner tree cannot pay twice even with an advanced second baseline', async () => {
  const f = await fixture(); await send(await f.deposit(), [f.owner]); await send(await f.freeze(), [f.recovery]); const r = await recoveryFixture(f);
  const s = { ...r.statement, treeSize: 2, total: 200n }, context = recoveryContextHash(r.context, s);
  const first = r.claims[0]!, second = { ...first, index: 1, claimId: id('second'), paidBase: 100n, payoutSequenceBase: 1n };
  const l = recoveryLeaf(context, first), right = recoveryLeaf(context, second);
  const root = createHash('sha256').update(Uint8Array.of(1)).update(l).update(right).digest();
  await send(await r.stage({ ...s, root }), [governance]); await send(await r.activate(undefined, undefined, root), [f.recovery]);
  await send(await r.claim(first, [right]), [f.owner]); await r.rejects(await r.claim(second, [l]), [f.owner]);
  assert.equal((await r.state()).epoch!.remaining.toString(), '100');
});

for (const field of ['paidBase', 'payoutSequenceBase'] as const) test(`recovery checked ${field} overflow rolls back proof consumption and CPI`, async () => {
  const f = await fixture(); await send(await f.deposit(), [f.owner]);
  const stored = await program.account.customerCounter.fetch(f.customer), max = 2n ** 64n - 1n;
  const encoded = await program.coder.accounts.encode('customerCounter', { ...stored, [field === 'paidBase' ? 'paid' : 'payoutSequence']: u64(max) });
  await rpc('surfnet_setAccount', [f.customer.toBase58(), { data: encoded.toString('hex') }]);
  await send(await f.freeze(), [f.recovery]); const r = await recoveryFixture(f);
  await send(await r.stage(), [governance]); await send(await r.activate(), [f.recovery]);
  await r.rejects(await r.claim(), [f.owner], r.receipt(), /Arithmetic/);
});
