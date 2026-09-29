#!/usr/bin/env node
// Offline documentation consistency, not proof of implementation or PR approval.
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

function sections(markdown) {
  const headings = [...markdown.matchAll(/^## (P\d{2,}) — .+$/gm)];
  return headings.map((heading, i) => ({
    id: heading[1],
    body: markdown.slice(heading.index, headings[i + 1]?.index ?? markdown.length),
  }));
}

export function validatePlan(plan, tracker) {
  const errors = [];
  const phases = sections(plan);
  const logs = sections(tracker);
  const rows = [...tracker.matchAll(/^\| \[(P\d{2,})[^\n]*?\]\(PLAN\.md#(p\d{2,})\) \| ([^|]+) \| ([^|]*) \|[^|\n]*\|$/gm)];
  if (!tracker.includes('| Phase | Progress | Invariant / deliverable | PR / merge |')) errors.push('tracker: missing invariant/deliverable column');
  const unique = (ids, name) => {
    if (!ids.length) errors.push(`${name}: no phases found`);
    if (new Set(ids).size !== ids.length) errors.push(`${name}: duplicate phase`);
    return new Set(ids);
  };
  const ids = unique(phases.map(p => p.id), 'plan');
  for (const [name, values] of [['tracker table', rows.map(r => r[1])], ['handoffs', logs.map(l => l.id)]]) {
    const actual = unique(values, name);
    if (actual.size !== ids.size || [...ids].some(id => !actual.has(id))) errors.push(`${name}: phase set differs from plan`);
  }
  const status = new Map(rows.map(r => [r[1], r[3].trim()]));
  for (const row of rows) {
    if (row[2] !== row[1].toLowerCase()) errors.push(`${row[1]}: wrong plan anchor`);
    if (!['open', 'in progress', 'closed'].includes(row[3].trim())) errors.push(`${row[1]}: invalid progress`);
    if (!row[4].trim() || /^(?:—|-|TBD)$/i.test(row[4].trim())) errors.push(`${row[1]}: missing invariant/deliverable`);
  }
  const dependencies = new Map();
  for (const phase of phases) {
    const field = phase.body.match(/^Depends on: (.+)\.$/m)?.[1];
    const deps = field === 'none' ? [] : (field ?? '').split(', ');
    if (!field || (field !== 'none' && !/^P\d{2,}(, P\d{2,})*$/.test(field))) errors.push(`${phase.id}: malformed dependencies`);
    if (new Set(deps).size !== deps.length) errors.push(`${phase.id}: duplicate dependency`);
    dependencies.set(phase.id, deps);
    for (const dep of deps) if (!ids.has(dep)) errors.push(`${phase.id}: unknown dependency ${dep}`);
    const checks = [...phase.body.matchAll(/^- \[([ x])\] .+/gm)];
    if (checks.length < 3) errors.push(`${phase.id}: fewer than three acceptance criteria`);
    for (const fieldName of ['Deliver:', 'Evidence:']) {
      if (!phase.body.includes(fieldName)) errors.push(`${phase.id}: missing ${fieldName}`);
    }
    if (!plan.includes(`<a id="${phase.id.toLowerCase()}"></a>`)) errors.push(`${phase.id}: missing explicit anchor`);
    if (status.get(phase.id) === 'closed') {
      if (checks.some(c => c[1] !== 'x')) errors.push(`${phase.id}: closed with incomplete acceptance`);
      if (deps.some(dep => status.get(dep) !== 'closed')) errors.push(`${phase.id}: closed before dependency`);
      const log = logs.find(l => l.id === phase.id)?.body ?? '';
      if (!/^Merge: [a-f0-9]{40}\.$/m.test(log)) errors.push(`${phase.id}: closed without full merge SHA`);
      if (!/^PR: https:\/\/github\.com\/[^/\s]+\/[^/\s]+\/pull\/\d+\.?$/m.test(log)) errors.push(`${phase.id}: closed without PR URL`);
    }
  }
  const active = new Set();
  const done = new Set();
  function visit(id) {
    if (active.has(id)) { errors.push(`dependency cycle at ${id}`); return; }
    if (done.has(id) || !ids.has(id)) return;
    active.add(id);
    for (const dep of dependencies.get(id) ?? []) visit(dep);
    active.delete(id);
    done.add(id);
  }
  for (const id of ids) visit(id);
  for (const log of logs) {
    for (const field of ['Work', 'Verification', 'Unexpected', 'Next', 'PR', 'Merge']) {
      if (!new RegExp(`^${field}: \\S`, 'm').test(log.body)) errors.push(`${log.id}: missing ${field} handoff`);
    }
  }
  return errors;
}

export function validateLinks(documents, root, fileExists = existsSync) {
  const errors = [];
  for (const [file, body] of documents) {
    for (const match of body.matchAll(/\[[^\]\n]+\]\(([^)\s]+)\)/g)) {
      const target = match[1];
      if (/^(https?:|mailto:)/.test(target)) continue;
      const [path, anchor] = target.split('#');
      const resolved = path ? resolve(dirname(file), path) : file;
      const local = relative(root, resolved);
      if (local === '..' || local.startsWith(`..${sep}`) || !fileExists(resolved)) {
        errors.push(`${relative(root, file)}: missing/outside local link ${target}`);
      } else if (anchor) {
        // Linked phase anchors are deliberately explicit to remain stable as titles change.
        const contents = documents.get(resolved);
        if (contents === undefined || !contents.includes(`<a id="${anchor}"></a>`)) errors.push(`${relative(root, file)}: missing explicit anchor ${target}`);
      }
    }
  }
  return errors;
}

function markdownFiles(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const path = resolve(directory, entry.name);
    return entry.isDirectory() ? markdownFiles(path) : entry.name.endsWith('.md') ? [path] : [];
  });
}

const script = fileURLToPath(import.meta.url);
if (process.argv[1] && resolve(process.argv[1]) === script) {
  const root = resolve(dirname(script), '..');
  const files = [resolve(root, 'README.md'), resolve(root, 'AGENTS.md'), ...markdownFiles(resolve(root, 'docs'))];
  const documents = new Map(files.map(file => [file, readFileSync(file, 'utf8')]));
  const plan = documents.get(resolve(root, 'docs/implementation/PLAN.md'));
  const tracker = documents.get(resolve(root, 'docs/implementation/TRACKER.md'));
  const errors = [...validatePlan(plan, tracker), ...validateLinks(documents, root)];
  if (errors.length) {
    for (const error of errors) process.stderr.write(`${error}\n`);
    process.exitCode = 1;
  } else {
    process.stdout.write(`Implementation foundation OK: ${sections(plan).length} phases; ${documents.size} documents.\n`);
  }
}
