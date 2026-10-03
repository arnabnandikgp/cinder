import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, rm, access } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { stopChild } from './stop-child.mjs';

function ready(child) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(Error('Fixture did not start')), 5000);
    child.once('error', error => { clearTimeout(timer); reject(error); });
    child.stdout.once('data', () => { clearTimeout(timer); resolve(); });
  });
}
test('reaps a child that refuses SIGTERM within a bounded wait', async () => {
  const child = spawn(process.execPath, ['-e', `
    process.on('SIGTERM', () => {});
    setInterval(() => {}, 1000);
    console.log('ready');
  `], { stdio: ['ignore', 'pipe', 'ignore'] });
  try {
    await ready(child);
    await stopChild(child, { graceMs: 100 });
    assert.equal(child.signalCode, 'SIGKILL');
    await stopChild(child, { graceMs: 100 }); // Already reaped, no late timer.
  } finally { await stopChild(child, { graceMs: 100 }); }
});
test('stops profile-writing descendants even when the group leader exits first', async () => {
  const profile = await mkdtemp(join(tmpdir(), 'cinder-channel-teardown-'));
  const path = join(profile, 'helper-write');
  const helper = `
    const fs = require('node:fs');
    process.on('SIGTERM', () => {});
    setInterval(() => { fs.mkdirSync(${JSON.stringify(profile)}, {recursive:true});
      fs.writeFileSync(${JSON.stringify(path)}, 'fixture'); }, 10);
    console.log('ready');
  `;
  const child = spawn(process.execPath, ['-e', `
    const {spawn} = require('node:child_process');
    const helper = spawn(process.execPath, ['-e', ${JSON.stringify(helper)}],
      {stdio: ['ignore', 'pipe', 'ignore']});
    helper.stdout.once('data', () => console.log('ready'));
    process.on('SIGTERM', () => process.exit(0));
    setInterval(() => {}, 1000);
  `], { detached: true, stdio: ['ignore', 'pipe', 'ignore'] });
  try {
    await ready(child);
    await stopChild(child, { group: true, graceMs: 100 });
    assert.equal(child.exitCode, 0);
    await rm(profile, { recursive: true, force: true, maxRetries: 4, retryDelay: 100 });
    await delay(150);
    await assert.rejects(access(profile), { code: 'ENOENT' });
  } finally {
    await stopChild(child, { group: true, graceMs: 100 });
    await rm(profile, { recursive: true, force: true, maxRetries: 4, retryDelay: 100 });
  }
});
