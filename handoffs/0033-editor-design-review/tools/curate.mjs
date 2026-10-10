#!/usr/bin/env node
/**
 * Copies a concise, unedited before/after subset of the browser captures into the
 * handoff and writes a measurement comparison and SHA-256 list. Sources stay in the
 * ignored artifacts/0033-editor-design-review/<set>/ directories.
 *
 * Usage (repo root): node handoffs/0033-editor-design-review/tools/curate.mjs <before-set> <after-set> <scale-set>
 */
import { createHash } from 'node:crypto';
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const handoff = resolve(here, '..');
const [beforeSet, afterSet, scaleSet] = process.argv.slice(2);
if (!beforeSet || !afterSet || !scaleSet) throw new Error('Usage: curate.mjs <before-set> <after-set> <scale-set>');
const artifacts = join(repoRoot, 'artifacts/0033-editor-design-review');

/** [state-size, crop]: crop is '' for the full window, or 'side' / 'inspector'. */
const PICKS = [
  ['transform-camera-normal-1440x874', ''],
  ['transform-camera-minimum-1000x650', ''],
  ['transform-camera-normal-1440x874', 'side'],
  ['transform-camera-narrow-1440x874', 'side'],
  ['camera-focus-normal-1440x874', 'side'],
  ['native-character-normal-1440x874', 'side'],
  ['native-character-minimum-1000x650', 'side'],
  ['native-character-narrow-1440x874', 'side'],
  ['native-mesh-normal-1440x874', 'side'],
  ['native-sun-normal-1440x874', 'side'],
  ['native-navmesh-normal-1440x874', 'side'],
  ['native-grid-normal-1440x874', 'side'],
  ['native-grid-large-normal-1440x874', 'side'],
  ['native-malformed-normal-1440x874', 'side'],
  ['railing-compound-normal-1440x874', 'side'],
  ['script-unsupported-normal-1440x874', 'side'],
  ['long-name-minimum-1000x650', 'side'],
  ['asset-texture-normal-1440x874', 'side'],
  ['agent-collapsed-normal-1440x874', 'side'],
  ['agent-signed-in-normal-1440x874', 'side'],
  ['stress-tree-normal-1440x874', ''],
];

const read = (set) => JSON.parse(readFileSync(join(artifacts, set, 'report.json'), 'utf8'));
const before = read(beforeSet);
const after = read(afterSet);
const scale = JSON.parse(readFileSync(join(artifacts, `scale-${scaleSet}`, 'report.json'), 'utf8'));
const shots = join(handoff, 'screenshots');
const evidence = join(handoff, 'evidence');
for (const dir of [join(shots, 'before'), join(shots, 'after'), evidence]) mkdirSync(dir, { recursive: true });

const sums = [];
const copied = { before: [], after: [] };
for (const [which, set] of [['before', beforeSet], ['after', afterSet]]) {
  for (const [state, crop] of PICKS) {
    const file = `${state}${crop ? `-${crop}` : ''}.png`;
    const source = join(artifacts, set, file);
    if (!existsSync(source)) continue;
    const target = join(shots, which, file);
    copyFileSync(source, target);
    sums.push(`${createHash('sha256').update(readFileSync(target)).digest('hex')}  screenshots/${which}/${file}`);
    copied[which].push(file);
  }
}
writeFileSync(join(evidence, 'screenshots.sha256'), `${sums.join('\n')}\n`);

/** The measurements that matter for the comparison, per capture. */
const summary = (capture) => {
  const m = capture?.measure ?? {};
  return capture
    ? {
        firstValueTop: m.firstValueTop ?? null,
        fixedChrome: m.fixedChrome ?? null,
        agentHeight: m.agent ? Math.round(m.agent.height) : null,
        clippedValues: (m.clippedValues ?? []).map((v) => `${v.name ?? '?'}: ${v.value} (${v.width}/${v.needed} px)`),
        clippedText: (m.clippedText ?? []).length,
        outOfBounds: (m.outOfBounds ?? []).length,
        horizontalOverflow: (m.horizontalOverflow ?? []).reduce((sum, o) => sum + o.overflow, 0),
        axe: capture.axe ? capture.axe.map((v) => v.id) : 'not run',
        contrast: m.contrast ?? null,
        focus: m.focus ?? null,
        timedOut: capture.timedOut ?? false,
        pageErrors: capture.errors?.length ?? 0,
        remoteRequests: capture.remoteRequests?.length ?? 0,
      }
    : null;
};
const byFile = (report) => new Map(report.captures.map((c) => [c.file.replace('.png', ''), c]));
const b = byFile(before);
const a = byFile(after);
const comparison = {
  generated: new Date().toISOString(),
  browser: after.browser,
  note: 'Headless Chrome captures of the built UI; not native WebKit. Sample fixture and evidence test double data.',
  before: beforeSet,
  after: afterSet,
  states: [...a.keys()].map((key) => ({ capture: key, before: summary(b.get(key)), after: summary(a.get(key)) })),
  totals: {
    captures: { before: before.captures.length, after: after.captures.length },
    axeViolations: {
      before: before.captures.reduce((n, c) => n + (c.axe?.length ?? 0), 0),
      after: after.captures.reduce((n, c) => n + (c.axe?.length ?? 0), 0),
    },
    clippedValues: {
      before: before.captures.reduce((n, c) => n + (c.measure?.clippedValues?.length ?? 0), 0),
      after: after.captures.reduce((n, c) => n + (c.measure?.clippedValues?.length ?? 0), 0),
    },
    outOfBoundsElements: {
      before: before.captures.reduce((n, c) => n + (c.measure?.outOfBounds?.length ?? 0), 0),
      after: after.captures.reduce((n, c) => n + (c.measure?.outOfBounds?.length ?? 0), 0),
    },
    pageErrors: {
      before: before.captures.reduce((n, c) => n + (c.errors?.length ?? 0), 0),
      after: after.captures.reduce((n, c) => n + (c.errors?.length ?? 0), 0),
    },
  },
};
writeFileSync(join(evidence, 'browser-comparison.json'), `${JSON.stringify(comparison, null, 2)}\n`);
writeFileSync(join(evidence, 'scale-report.json'), `${JSON.stringify(scale, null, 2)}\n`);
console.log(`Copied ${copied.before.length} before and ${copied.after.length} after screenshots; wrote evidence/.`);
