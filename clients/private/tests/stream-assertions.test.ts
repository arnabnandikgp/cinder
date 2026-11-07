import test from 'node:test';
import assert from 'node:assert/strict';
import {expectStreamClosed} from './stream-assertions.ts';

test('stream closure accepts actual iterator rejection or done: true',async()=>{
  await expectStreamClosed({next:async()=>{throw Error('closed');}},'rejection');
  await expectStreamClosed({next:async()=>({done:true,value:undefined})},'ended');
});

test('a delivered page is not evidence of stream closure',async()=>{
  await assert.rejects(expectStreamClosed({next:async()=>({done:false,value:1})},'still open'),/unexpected private update/);
});

test('an indefinitely open stream fails on the harness deadline',async t=>{
  t.mock.timers.enable({apis:['setTimeout']});
  let calls=0;
  const failure=assert.rejects(expectStreamClosed({next:()=>{calls++;return new Promise<IteratorResult<number>>(()=>{});}},'stalled'),/stream closure timed out/);
  t.mock.timers.tick(6000);
  await failure;
  assert.equal(calls,1);
});

test('termination after the deadline cannot rescue a failed closure assertion',async t=>{
  t.mock.timers.enable({apis:['setTimeout']});
  let end!:(result:IteratorResult<number>)=>void;
  const pending=new Promise<IteratorResult<number>>(resolve=>{end=resolve;});
  const failure=assert.rejects(expectStreamClosed({next:()=>pending},'late close'),/stream closure timed out/);
  t.mock.timers.tick(6000);
  end({done:true,value:undefined});
  await failure;
});
