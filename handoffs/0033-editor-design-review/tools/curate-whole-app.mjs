#!/usr/bin/env node
/**
 * Whole-app pass (director feedback 2026-10-10): copies an unedited before/after subset
 * of the browser captures into the handoff, one pair per surface of the coverage matrix,
 * and writes the measurement comparison and a SHA-256 list. Sources stay in the ignored
 * artifacts/0033-editor-design-review/<set>/ directories.
 *
 * "Before" is revision 3 as committed at e8ee0d6, built from an extracted copy with
 * `capture.mjs <set> --root=…`; "after" is revision 4. Both use the same states, data,
 * browser and sizes. Native captures (which show the signed-in account) are never read.
 *
 * Usage (repo root): node handoffs/0033-editor-design-review/tools/curate-whole-app.mjs <before-set[,before-set]> <after-set>
 * Several before sets may be given (comma-separated); each capture comes from the first that has it.
 */
import { createHash } from 'node:crypto';
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const handoff = resolve(here, '..');
const [beforeSet, afterSet] = process.argv.slice(2);
if (!beforeSet || !afterSet) throw new Error('Usage: curate-whole-app.mjs <before-set> <after-set>');
const artifacts = join(repoRoot, 'artifacts/0033-editor-design-review');

/** [capture, crop, pair]: crop '' is the full window; pair false copies only the after image. */
const PICKS = [
  ['no-selection-normal-1440x874', '', true],
  ['transform-camera-normal-1440x874', '', true],
  ['transform-camera-minimum-1000x650', '', true],
  ['transform-camera-narrow-1440x874', 'side', true],
  ['native-rubble-parts-normal-1440x874', 'inspector', true],
  ['native-rubble-parts-narrow-1440x874', 'inspector', true],
  ['native-rubble-parts-minimum-1000x650', 'inspector', true],
  ['railing-parts-normal-1440x874', 'inspector', true],
  ['railing-compound-normal-1440x874', 'side', true],
  ['native-character-normal-1440x874', 'side', true],
  ['native-malformed-normal-1440x874', 'side', true],
  ['native-navmesh-normal-1440x874', 'side', true],
  ['dock-history-normal-1440x874', '', true],
  ['dock-console-normal-1440x874', '', true],
  ['context-menu-normal-1440x874', '', false],
  ['shortcuts-dialog-normal-1440x874', '', true],
  ['shortcuts-filter-minimum-1000x650', '', false],
  ['account-signed-in-normal-1440x874', '', true],
  ['asset-texture-normal-1440x874', '', true],
  ['project-loading-normal-1440x874', '', true],
  ['project-error-normal-1440x874', '', true],
  ['empty-project-normal-1440x874', '', true],
  ['no-engine-normal-1440x874', '', true],
  ['bad-fixture-normal-1440x874', '', true],
  ['incompatible-normal-1440x874', '', true],
  // Long output lists (correctness follow-up): every row mounted before, one page after.
  ['dock-paged-problems-normal-1440x874', '', true],
  ['dock-paged-console-minimum-1000x650', '', true],
  ['dock-paged-history-normal-1440x874', '', true],
];

const read = (set) => JSON.parse(readFileSync(join(artifacts, set, 'report.json'), 'utf8'));
const beforeSets = beforeSet.split(',');
const befores = beforeSets.map(read);
// One report of every before capture; a later set only adds captures the earlier ones lack.
const seen = new Set();
const before = { ...befores[0], captures: befores.flatMap((report) => report.captures.filter((c) => !seen.has(c.file) && seen.add(c.file))) };
const after = read(afterSet);
const beforeSource = (file) => beforeSets.find((set) => existsSync(join(artifacts, set, file)));
const shots = join(handoff, 'screenshots/whole-app');
const evidence = join(handoff, 'evidence');
for (const dir of [join(shots, 'before'), join(shots, 'after'), evidence]) mkdirSync(dir, { recursive: true });

const sums = [];
let copied = 0;
for (const [capture, crop, pair] of PICKS) {
  const file = `${capture}${crop ? `-${crop}` : ''}.png`;
  for (const [which, set] of pair ? [['before', beforeSource(file)], ['after', afterSet]] : [['after', afterSet]]) {
    if (!set) throw new Error(`No before set has ${file}`);
    const source = join(artifacts, set, file);
    if (!existsSync(source)) throw new Error(`Missing ${source}`);
    const target = join(shots, which, file);
    copyFileSync(source, target);
    sums.push(`${createHash('sha256').update(readFileSync(target)).digest('hex')}  screenshots/whole-app/${which}/${file}`);
    copied += 1;
  }
}
writeFileSync(join(evidence, 'whole-app.sha256'), `${sums.join('\n')}\n`);

const summary = (capture) => {
  const m = capture?.measure ?? {};
  return capture
    ? {
        inspectorWidth: m.inspector?.width ?? null,
        agentHeight: m.agent ? Math.round(m.agent.height) : null,
        firstValueTop: m.firstValueTop ?? null,
        rowHeights: [...new Set(m.rowHeights ?? [])],
        componentHeaderHeights: [...new Set((m.componentHeaders ?? []).map((h) => h.height))],
        clippedValues: (m.clippedValues ?? []).map((v) => `${v.name ?? '?'}: ${v.value} (${v.width}/${v.needed} px)`),
        outOfBounds: (m.outOfBounds ?? []).length,
        pageHorizontalOverflow: m.pageHorizontalOverflow ?? null,
        pageVerticalOverflow: m.pageVerticalOverflow ?? null,
        dockRowsMounted: m.dockRows ?? null,
        axe: capture.axe ? capture.axe.map((v) => v.id) : 'not run',
        contrast: m.contrast ?? null,
        focus: m.focus ?? null,
        stepError: capture.stepError ?? null,
        timedOut: capture.timedOut ?? false,
        pageErrors: capture.errors?.length ?? 0,
        remoteRequests: capture.remoteRequests?.length ?? 0,
      }
    : null;
};
const byFile = (report) => new Map(report.captures.map((c) => [c.file.replace('.png', ''), c]));
const b = byFile(before);
const a = byFile(after);
const total = (report, count) => report.captures.reduce((n, c) => n + count(c), 0);
const totals = (count) => ({ before: total(before, count), after: total(after, count) });
const comparison = {
  generated: new Date().toISOString(),
  browser: after.browser,
  note: 'Headless Chrome captures of the built UI at device scale 2; not native WebKit. Sample fixture, provider fixtures and the evidence test double; no account data.',
  // Repository-relative, so no local path is committed.
  before: { sets: beforeSets, ui: 'revision 3, e8ee0d6', uiRoot: before.uiRoot ? relative(repoRoot, before.uiRoot) : null },
  after: { set: afterSet, ui: 'revision 4, this commit', uiRoot: after.uiRoot ? relative(repoRoot, after.uiRoot) : null },
  totals: {
    captures: totals(() => 1),
    axeViolations: totals((c) => c.axe?.length ?? 0),
    clippedValues: totals((c) => c.measure?.clippedValues?.length ?? 0),
    outOfBoundsElements: totals((c) => c.measure?.outOfBounds?.length ?? 0),
    statesWithScrollableWindow: totals((c) => ((c.measure?.pageVerticalOverflow ?? 0) > 0 ? 1 : 0)),
    statesWithHorizontalPageOverflow: totals((c) => ((c.measure?.pageHorizontalOverflow ?? 0) > 0 ? 1 : 0)),
    mostDockRowsMounted: {
      before: Math.max(0, ...before.captures.map((c) => c.measure?.dockRows ?? 0)),
      after: Math.max(0, ...after.captures.map((c) => c.measure?.dockRows ?? 0)),
    },
    stepErrors: totals((c) => (c.stepError ? 1 : 0)),
    pageErrors: totals((c) => c.errors?.length ?? 0),
    remoteRequests: totals((c) => c.remoteRequests?.length ?? 0),
  },
  states: [...a.keys()].map((key) => ({ capture: key, before: summary(b.get(key)), after: summary(a.get(key)) })),
};
writeFileSync(join(evidence, 'whole-app-comparison.json'), `${JSON.stringify(comparison, null, 2)}\n`);
console.log(`Copied ${copied} screenshots to screenshots/whole-app; wrote evidence/whole-app-comparison.json and whole-app.sha256.`);
console.log(JSON.stringify(comparison.totals));
