import test from 'node:test';
import assert from 'node:assert/strict';
import { validateLinks, validatePlan } from './check-implementation-plan.mjs';

function fixture(closed = false) {
  const plan = ['P00', 'P01'].map((id, i) => `<a id="${id.toLowerCase()}"></a>
## ${id} — Test
Depends on: ${i ? 'P00' : 'none'}.
Deliver: something.
Evidence: test result.
${[1, 2, 3].map(n => `- [${closed ? 'x' : ' '}] criterion ${n}`).join('\n')}
`).join('\n');
  const tracker = ['P00', 'P01'].map(id => `| [${id} — Test](PLAN.md#${id.toLowerCase()}) | ${closed ? 'closed' : 'open'} | — |`).join('\n') + '\n' + ['P00', 'P01'].map(id => `## ${id} — Test
Work: test.
Verification: command/result recorded.
Unexpected: none.
Next: next task.
PR: https://github.com/example/repo/pull/1
Merge: ${closed ? 'a'.repeat(40) : 'pending'}.
`).join('\n');
  return { plan, tracker };
}

test('open foundation validates', () => {
  const { plan, tracker } = fixture();
  assert.deepEqual(validatePlan(plan, tracker), []);
});
test('complete closed foundation validates structurally', () => {
  const { plan, tracker } = fixture(true);
  assert.deepEqual(validatePlan(plan, tracker), []);
});
test('unsupported progress cannot pass', () => {
  const { plan, tracker } = fixture();
  assert.match(validatePlan(plan, tracker.replace('| open |', '| ready |')).join('\n'), /invalid progress/);
});
test('phase sets and duplicate handoffs are checked', () => {
  const { plan, tracker } = fixture();
  assert.match(validatePlan(plan, tracker.replace('## P01', '## P00')).join('\n'), /duplicate phase/);
  assert.match(validatePlan(plan, tracker.replace('## P01', '## P02')).join('\n'), /phase set differs/);
});
test('cycles and unknown dependencies reject', () => {
  const { plan, tracker } = fixture();
  assert.match(validatePlan(plan.replace('Depends on: none.', 'Depends on: P01.'), tracker).join('\n'), /cycle/);
  assert.match(validatePlan(plan.replace('Depends on: P00.', 'Depends on: P99.'), tracker).join('\n'), /unknown dependency/);
});
test('missing evidence and criteria reject', () => {
  const { plan, tracker } = fixture();
  assert.match(validatePlan(plan.replace('Evidence:', 'Notes:').replace('- [ ] criterion 1', ''), tracker).join('\n'), /acceptance/);
  assert.match(validatePlan(plan.replace('Evidence:', 'Notes:'), tracker).join('\n'), /missing Evidence/);
});
test('closed requires all criteria and a real-looking full merge SHA', () => {
  const { plan, tracker } = fixture(true);
  assert.match(validatePlan(plan.replace('[x]', '[ ]'), tracker).join('\n'), /incomplete acceptance/);
  assert.match(validatePlan(plan, tracker.replace('a'.repeat(40), 'pending')).join('\n'), /merge SHA/);
});
test('closed cannot precede a dependency', () => {
  const { plan, tracker } = fixture(true);
  assert.match(validatePlan(plan, tracker.replace('| closed |', '| in progress |')).join('\n'), /closed before dependency/);
});
test('next-agent handoff is mandatory', () => {
  const { plan, tracker } = fixture();
  assert.match(validatePlan(plan, tracker.replace('Next:', 'Later:')).join('\n'), /missing Next handoff/);
});
test('explicit plan links resolve without work or network', () => {
  const docs = new Map([
    ['/repo/README.md', '[plan](docs/PLAN.md#p00) [source](https://example.com)'],
    ['/repo/docs/PLAN.md', '<a id="p00"></a>'],
  ]);
  assert.deepEqual(validateLinks(docs, '/repo', path => docs.has(path)), []);
});
test('missing files, escaped paths and bad anchors reject', () => {
  const docs = new Map([
    ['/repo/README.md', '[missing](work/hidden.md) [escape](../secret) [bad](docs/PLAN.md#p99)'],
    ['/repo/docs/PLAN.md', '<a id="p00"></a>'],
  ]);
  assert.equal(validateLinks(docs, '/repo', path => docs.has(path)).length, 3);
});
