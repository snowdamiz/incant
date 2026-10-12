#!/usr/bin/env node
/**
 * Copies the reviewed captures of one capture set (capture.mjs) and two regression sets
 * (regression.mjs: baseline and candidate) into handoffs/0044-native-editor-play/
 * screenshots/, and writes evidence/review.json (what each capture shows, from the set's
 * evidence.json) and evidence/screenshots.sha256. Existing curated files are replaced.
 *
 * Usage (repo root): node handoffs/0044-native-editor-play/tools/curate.mjs <captureSet> <baselineSet> <candidateSet>
 */
import { createHash } from 'node:crypto';
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { repoRoot } from './play-double.mjs';

const [captureSet, baseSet, candSet] = process.argv.slice(2);
if (!captureSet || !baseSet || !candSet) throw new Error('Usage: curate.mjs <captureSet> <baselineSet> <candidateSet>');
const artifacts = join(repoRoot, 'artifacts/0044-native-editor-play');
const handoff = join(repoRoot, 'handoffs/0044-native-editor-play');
const shots = join(handoff, 'screenshots');
const evidenceDir = join(handoff, 'evidence');
const capture = JSON.parse(readFileSync(join(artifacts, captureSet, 'evidence.json'), 'utf8'));

/** [scenario, step] per size folder, in review order. */
const PICK = {
  normal: [
    ['choose', 'stopped'], ['choose', 'picker'], ['choose', 'preparing-polled'],
    ['running', 'refreshed'], ['pause-step', 'paused'], ['pause-step', 'stepped'],
    ['stop-preparing', 'stopping'], ['stop-preparing', 'after-cancelled-startup'],
    ['unsaved', 'refused'], ['runtime-error', 'failed'], ['transport', 'unreadable'],
    ['missing', 'absent'], ['missing', 'other-scene'], ['suspended', 'suspended'],
    ['game-input', 'focused'], ['single', 'stopped'], ['fixture', 'explained'],
    ['keyboard', 'play-focused'], ['keyboard', 'choosing'], ['older-host', 'labelled'],
    ['shortcuts', 'filtered'],
  ],
  minimum: [
    ['choose', 'stopped'], ['choose', 'picker'], ['choose', 'preparing-polled'],
    ['running', 'refreshed'], ['pause-step', 'stepped'], ['unsaved', 'refused'],
    ['runtime-error', 'failed'], ['game-input', 'sent'], ['keyboard', 'play-focused'],
  ],
  narrow: [
    ['running', 'refreshed'], ['pause-step', 'stepped'], ['missing', 'absent'], ['unsaved', 'refused'],
  ],
};
const REGRESSION = ['account-signed-in-multi', 'account-signed-out', 'titlebar', 'hierarchy', 'inspector-crate-03', 'shortcuts-dialog', 'viewport-header'];

rmSync(shots, { recursive: true, force: true });
mkdirSync(evidenceDir, { recursive: true });
const sha = (path) => createHash('sha256').update(readFileSync(path)).digest('hex');
const review = { captureSet, baseSet, candSet, browser: capture.browser, double: capture.double, capture: [], regression: [], totals: {} };

for (const [size, picks] of Object.entries(PICK)) {
  mkdirSync(join(shots, size), { recursive: true });
  for (const [scenario, step] of picks) {
    const record = capture.steps.find((s) => s.file && s.scenario === scenario && s.step === step && s.size === size);
    if (!record) throw new Error(`No capture for ${scenario}/${step}/${size} in ${captureSet}`);
    const target = join(shots, size, `${scenario}--${step}.png`);
    copyFileSync(join(artifacts, captureSet, record.file), target);
    review.capture.push({
      file: relative(handoff, target),
      size: record.file.match(/(\d+x\d+)\.png$/)[1],
      header: record.shown.header.status,
      controls: record.shown.header.controls.filter((c) => !/^Play (scene|camera)/.test(c.name)).map((c) => `${c.name}${c.disabled ? ' (unavailable)' : ''}`),
      picker: record.shown.header.picker,
      notice: record.shown.notice,
      runtime: record.shown.runtime && { text: record.shown.runtime.text, rows: record.shown.runtime.rows },
      statusLine: record.shown.statusLine,
      source: record.shown.source,
      focus: record.shown.focus,
      inspectorWidth: record.shown.inspectorWidth,
      gameView: record.shown.gameView && { focusable: record.shown.gameView.focusable, focused: record.shown.gameView.focused },
      host: record.host && { phase: record.host.status.phase, generation: record.host.status.generation, tick: record.host.status.tick, preparing: record.host.status.preparing, boundsMatch: record.host.bounds.match },
      axeViolations: record.axe.length,
    });
  }
}

mkdirSync(join(shots, 'regression'), { recursive: true });
for (const name of REGRESSION) {
  const before = join(artifacts, baseSet, `${name}.png`);
  const after = join(artifacts, candSet, `${name}.png`);
  const same = sha(before) === sha(after);
  const target = join(shots, 'regression', `${name}--after.png`);
  copyFileSync(after, target);
  const entry = { name, identical: same, after: relative(handoff, target) };
  if (!same) {
    const old = join(shots, 'regression', `${name}--before.png`);
    copyFileSync(before, old);
    entry.before = relative(handoff, old);
  }
  review.regression.push(entry);
}

const steps = capture.steps.filter((s) => s.file);
review.totals = {
  steps: steps.length,
  axeViolations: steps.reduce((sum, s) => sum + s.axe.length, 0),
  pageErrors: capture.errors.length,
  remoteRequests: capture.remote.length,
  boundsMismatches: steps.filter((s) => s.host && !s.host.bounds.match).length,
  headerOrPageOverflow: steps.filter((s) => s.shown.header.overflowing.length || s.shown.pageOverflow.some((v) => v > 0)).length,
  titlebarInsets: [...new Set(steps.map((s) => s.shown.titlebarInset))],
  inputPackets: capture.steps.filter((s) => s.step === 'record').reduce((sum, s) => sum + s.inputs.length, 0),
  largestPacket: Math.max(0, ...capture.steps.filter((s) => s.step === 'record').flatMap((s) => s.inputs.map((p) => p.events.length))),
  refusedInputs: capture.steps.filter((s) => s.step === 'record').reduce((sum, s) => sum + s.refusedInputs.length, 0),
  inputGenerations: [...new Set(capture.steps.filter((s) => s.step === 'record').flatMap((s) => s.inputs.map((p) => p.generation)))],
};
writeFileSync(join(evidenceDir, 'review.json'), `${JSON.stringify(review, null, 2)}\n`);

const files = [...review.capture.map((c) => c.file), ...review.regression.flatMap((r) => [r.after, r.before].filter(Boolean))];
writeFileSync(join(evidenceDir, 'screenshots.sha256'), `${files.map((file) => `${sha(join(handoff, file))}  ${file}`).join('\n')}\n`);
console.log(`${files.length} screenshots; totals ${JSON.stringify(review.totals)}`);
for (const r of review.regression) console.log(`  regression ${r.name}: ${r.identical ? 'identical' : 'changed'}`);
if (!existsSync(join(evidenceDir, 'review.json'))) process.exitCode = 1;
