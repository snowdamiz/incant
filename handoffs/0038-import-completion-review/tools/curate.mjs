#!/usr/bin/env node
/**
 * Copies the committed screenshots for handoff 0038 out of the ignored capture sets,
 * writes evidence/screenshots.sha256, and writes evidence/completion-review.json: source
 * and build identity, the observed completion states, the pixel comparisons and log
 * hashes. Run from the repo root after the capture sets and pixel comparisons below.
 *
 *   b1  frozen source a62d35e (before the copy change)
 *   g1  final source; g2 the same build again, to measure rendering noise
 *
 * All three sets come from this version of capture.mjs.
 *
 * Usage: node handoffs/0038-import-completion-review/tools/curate.mjs
 */
import { createHash } from 'node:crypto';
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const packet = resolve(here, '..');
const artifacts = join(repoRoot, 'artifacts/0038-import-completion-review');
const sha = (file) => createHash('sha256').update(readFileSync(file)).digest('hex');
const json = (file) => JSON.parse(readFileSync(join(artifacts, file), 'utf8'));

/** Committed screenshots: destination folder → [set, file]. */
const PICKS = {
  normal: [
    'noop-reimport-normal-1440x874--done.png',
    'changed-reimport-normal-1440x874--changed.png',
    'changed-reimport-normal-1440x874--undone.png',
    'changed-reimport-normal-1440x874--newer.png',
    'changed-import-normal-1440x874--changed.png',
    'unknown-reimport-normal-1440x874--done.png',
    'failure-import-normal-1440x874--done.png',
    'away-noop-reimport-normal-1440x874--done.png',
    'noop-concurrent-normal-1440x874--done.png',
  ].map((file) => ['g1', file]),
  minimum: [
    'noop-reimport-minimum-1000x650--done.png',
    'noop-import-minimum-1000x650--done.png',
    'changed-reimport-minimum-1000x650--changed.png',
    'changed-reimport-minimum-1000x650--undone.png',
    'unknown-reimport-minimum-1000x650--done.png',
    'failure-import-minimum-1000x650--done.png',
  ].map((file) => ['g1', file]),
  'inspector-280': [
    'noop-reimport-narrow-1440x874--done-side.png',
    'noop-import-narrow-1440x874--done-side.png',
    'changed-reimport-narrow-1440x874--changed-side.png',
    'changed-reimport-narrow-1440x874--undone-side.png',
    'changed-reimport-narrow-1440x874--redone-side.png',
    'changed-reimport-narrow-1440x874--newer-side.png',
    'changed-import-narrow-1440x874--changed-side.png',
    'changed-import-narrow-1440x874--undone-side.png',
    'unknown-reimport-narrow-1440x874--done-side.png',
    'failure-import-narrow-1440x874--done-side.png',
  ].map((file) => ['g1', file]),
  'status-line': [
    'away-noop-reimport-normal-1440x874--done-statusbar.png',
    'away-noop-import-normal-1440x874--done-statusbar.png',
    'long-noop-reimport-minimum-1000x650--done-statusbar.png',
  ].map((file) => ['g1', file]),
  // The defect on the frozen source, for comparison with status-line/ and normal/.
  before: [
    'away-noop-reimport-normal-1440x874--done.png',
    'away-noop-reimport-normal-1440x874--done-statusbar.png',
  ].map((file) => ['b1', file]),
  // Existing failure copy on a reimport, reported for a follow-up (not changed here).
  probe: [['g1', 'failure-reimport-normal-1440x874--done-side.png']],
};

const manifest = [];
for (const [folder, picks] of Object.entries(PICKS)) {
  mkdirSync(join(packet, 'screenshots', folder), { recursive: true });
  for (const [set, file] of picks) {
    const from = join(artifacts, set, file);
    if (!existsSync(from)) throw new Error(`Missing ${set}/${file}`);
    const to = join(packet, 'screenshots', folder, file);
    copyFileSync(from, to);
    manifest.push(`${sha(to)}  screenshots/${folder}/${file}`);
  }
}
mkdirSync(join(packet, 'evidence'), { recursive: true });
writeFileSync(join(packet, 'evidence/screenshots.sha256'), `${manifest.join('\n')}\n`);

/* ---------- observations ---------- */

const frozenSet = json('b1/report.json');
const finalSet = json('g1/report.json');
const repeatSet = json('g2/report.json');
const observe = (capture) => ({
  state: capture.state,
  size: `${capture.size.key} ${capture.size.width}x${capture.size.height}`,
  note: capture.note,
  commands: capture.commands.map((c) => (c.type === 'asset.import' ? { type: c.type, sources: c.sources } : c)),
  mockedResults: capture.results,
  steps: capture.steps.map((s) => ({
    step: s.name,
    inspectorWidth: s.observed.inspectorWidth,
    notice: s.observed.notice && { text: s.observed.notice.text, role: s.observed.notice.role, lines: s.observed.notice.paragraphs.map((p) => p.lines), contrast: s.observed.notice.paragraphs.map((p) => p.contrast), inView: s.observed.notice.inView },
    statusLine: s.observed.statusLine,
    statusLineClipped: s.observed.statusLineFit?.clipped ?? null,
    action: s.observed.action,
    header: s.observed.header,
    justImported: s.observed.justImported,
    historyShown: s.observed.historyShown && { meta: s.observed.historyShown.meta, nowAfter: s.observed.historyShown.nowAfter },
    host: s.observed.host,
    focus: s.focus,
    axeViolations: s.axe.length,
  })),
  liveRegionChanges: capture.live.filter((l) => l.step !== 'load' && !/search-field__status/.test(l.region)),
  pageErrors: capture.errors.length,
  remoteRequests: capture.remoteRequests.length,
  stepError: capture.stepError,
});
const totals = (report) => {
  const steps = report.captures.flatMap((c) => c.steps);
  return {
    captures: report.captures.length,
    steps: steps.length,
    axeViolations: steps.reduce((n, s) => n + s.axe.length, 0),
    pageErrors: report.captures.reduce((n, c) => n + c.errors.length, 0),
    remoteRequests: report.captures.reduce((n, c) => n + c.remoteRequests.length, 0),
    stepErrors: report.captures.filter((c) => c.stepError).length,
    pageOverflow: steps.filter((s) => s.observed.pageOverflow.some((n) => n > 0)).length,
    statusLineClipped: steps.filter((s) => s.observed.statusLineFit?.clipped).length,
  };
};

/* ---------- pixel comparisons ---------- */

const change = json('pixel-b1-g1.json');
const noise = json('pixel-g1-g2.json');
const noisy = new Set(noise.results.filter((r) => r.differing > 0).map((r) => r.name));
const STATUS_BAND = 28 * 2; // the 28 px status line at deviceScaleFactor 2
/** The text a screenshot shows, from the step that wrote it, in a capture set. */
const shownText = (report, name) => {
  const step = report.captures.flatMap((c) => c.steps).find((s) => name.startsWith(s.file.replace('.png', '')));
  return step ? JSON.stringify([step.observed.notice?.text ?? null, step.observed.statusLine]) : null;
};
const classify = (r) => {
  const statusLine = r.name.endsWith('-statusbar.png') || (!/-(side|dock)\.png$/.test(r.name) && r.box && r.box[1] >= r.size[1] - STATUS_BAND);
  if (statusLine) return 'status line only';
  if (noisy.has(r.name)) return 'noise: also differs between two runs of the final build';
  if (/--(pending|rows)/.test(r.name)) return 'noise: spinner, caret or hover transition before completion';
  const text = shownText(finalSet, r.name);
  if (r.differing < 100 && text !== null && text === shownText(frozenSet, r.name)) return 'noise: same notice and status text, sparse transition pixels';
  return 'unexplained';
};
const changed = change.results.filter((r) => r.differing > 0).map((r) => ({ name: r.name, differing: r.differing, box: r.box, class: classify(r) }));
if (changed.some((r) => r.class === 'unexplained')) throw new Error(`Unexplained differences: ${changed.filter((r) => r.class === 'unexplained').map((r) => r.name).join(', ')}`);

/* ---------- identity and logs ---------- */

const git = (...args) => execFileSync('git', args, { cwd: repoRoot, encoding: 'utf8' }).trim();
const base = 'a62d35ea2f7d6f9bdbf487b7d84d0555689d553f';
const blobSha = (path) => createHash('sha256').update(execFileSync('git', ['show', `${base}:${path}`], { cwd: repoRoot })).digest('hex');
const SOURCES = ['editor/ui/src/assets/AssetsContext.tsx', 'editor/ui/src/components/assets/AssetInspector.tsx', 'editor/ui/src/components/assets/Assets.test.tsx'];
const LOGS = [
  'vitest-baseline.log', 'tsc-baseline.log', 'build-baseline.log',
  'vitest-final-on-old-copy.log', 'vitest-commit.log', 'tsc-commit.log', 'build-commit.log',
  'build-frozen-rebuild.log', 'build-final-rebuild.log',
  'b1.log', 'g1.log', 'g2.log', 'pixel-b1-g1.log', 'pixel-g1-g2.log',
];

const evidence = {
  handoff: '0038-import-completion-review',
  label: 'Headless browser presentation review on a MOCKED live double. Not native command execution, native WebKit, screen-reader or device evidence.',
  model: { id: 'claude-opus-5-5', name: 'Claude Opus 5.5', thinking: 'max (as reported to the session; ACP selection is verified by the runner)' },
  source: {
    base,
    baseMatchesAstraEvidence: 'all 8 source_sha256 entries in docs/spikes/evidence/editor-import-receipts-2026-10-11.json matched before any change',
    files: Object.fromEntries(SOURCES.map((path) => [path, { before: blobSha(path), after: sha(join(repoRoot, path)) }])),
    // Product paths that differ from the base: only the two UI files this packet may change.
    changedFromBase: git('diff', '--name-status', base, '--', 'editor', 'crates', 'schemas', 'docs', 'Cargo.toml', 'Cargo.lock', '.github').split('\n').filter(Boolean),
  },
  build: { frozen: frozenSet.build, final: finalSet.build },
  browser: { chrome: finalSet.browser, headless: true, playwrightCore: '1.64.0', node: process.version, deviceScaleFactor: 2, timezone: 'UTC' },
  tests: {
    baseline: 'frozen source: 30 files, 529 tests passed; strict tsc clean; production build passed',
    updatedAssertionOnPreviousCopy: 'Assets.test.tsx with the previous AssetsContext.tsx: 2 failed (the two no-op announcement cases), 37 passed',
    final: 'commit tree: 30 files, 529 tests passed; strict tsc clean; production build passed, reproducing the bundles g1 and g2 captured',
  },
  sets: {
    b1: { source: 'frozen a62d35e', generated: frozenSet.generated, totals: totals(frozenSet) },
    g1: { source: 'final (this commit)', generated: finalSet.generated, totals: totals(finalSet) },
    g2: { source: 'final (this commit), repeat', generated: repeatSet.generated, totals: totals(repeatSet) },
  },
  observations: finalSet.captures.map(observe),
  before: frozenSet.captures.filter((c) => c.state.startsWith('away-') || c.state.startsWith('noop-')).map((c) => ({
    state: c.state, size: `${c.size.key} ${c.size.width}x${c.size.height}`,
    statusLine: c.steps.at(-1)?.observed.statusLine, notice: c.steps.at(-1)?.observed.notice?.text ?? null, header: c.steps.at(-1)?.observed.header,
  })),
  pixel: {
    frozenToFinal: { compared: change.compared, identical: change.identical, differing: changed },
    finalRepeat: { compared: noise.compared, identical: noise.identical, differing: noise.results.filter((r) => r.differing > 0).map((r) => ({ name: r.name, differing: r.differing, box: r.box })) },
  },
  logs: Object.fromEntries(LOGS.map((file) => [`artifacts/0038-import-completion-review/${file}`, { sha256: sha(join(artifacts, file)), bytes: readFileSync(join(artifacts, file)).length }])),
};
writeFileSync(join(packet, 'evidence/completion-review.json'), `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`${manifest.length} screenshots; ${changed.length} frozen-to-final differences (${changed.filter((r) => r.class === 'status line only').length} status line only)`);
