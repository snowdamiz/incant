#!/usr/bin/env node
/**
 * Plain pass (director feedback "rejects ornamental UI", 2026-10-10): copies an unedited
 * before/after subset of the browser captures into the handoff, one pair per surface of
 * the coverage matrix, and writes the measurement comparison (including the words a
 * sighted user can read per area) and a SHA-256 list. Sources stay in the ignored
 * artifacts/0033-editor-design-review/<set>/ directories.
 *
 * "Before" is 5cfa59d (revision 4 with the correctness follow-ups), built from an
 * extracted copy with `capture.mjs <set> --root=…`; "after" is revision 5. Both use the
 * same states, data, browser and sizes. Account data is the synthetic provider fixtures;
 * native captures (which show the signed-in account) are never read.
 *
 * Usage (repo root): node handoffs/0033-editor-design-review/tools/curate-plain.mjs <before-set> <after-set>
 */
import { createHash } from 'node:crypto';
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const handoff = resolve(here, '..');
const [beforeSet, afterSet] = process.argv.slice(2);
if (!beforeSet || !afterSet) throw new Error('Usage: curate-plain.mjs <before-set> <after-set>');
const artifacts = join(repoRoot, 'artifacts/0033-editor-design-review');

/** [capture, crop]: crop '' is the full window. Every pick is a before/after pair. */
const PICKS = [
  // Titlebar (avatar alone, no repeated scene), status bar, empty Inspector.
  ['no-selection-normal-1440x874', ''],
  ['no-selection-minimum-1000x650', ''],
  // Inspector: no metadata row or component tiles; plain "Default".
  ['camera-rig-normal-1440x874', 'inspector'],
  ['camera-rig-minimum-1000x650', ''],
  ['native-character-normal-1440x874', 'inspector'],
  // Compound collider: no explanation paragraph; counts in the list header.
  ['native-rubble-parts-normal-1440x874', 'inspector'],
  ['native-rubble-parts-minimum-1000x650', 'inspector'],
  ['railing-parts-normal-1440x874', 'inspector'],
  // Diagnostics stay: raw values and messages, without dashed boxes.
  ['native-malformed-normal-1440x874', 'inspector'],
  // Account surfaces (synthetic accounts).
  ['account-dialog-normal-1440x874', ''],
  ['account-saved-normal-1440x874', ''],
  ['account-signed-in-normal-1440x874', ''],
  ['agent-error-normal-1440x874', ''],
  ['agent-checking-normal-1440x874', 'agent'],
  // Keyboard shortcuts: one table instead of a card grid.
  ['shortcuts-dialog-normal-1440x874', ''],
  ['shortcuts-filter-minimum-1000x650', ''],
  ['context-menu-normal-1440x874', ''],
  // Output dock: plain tabs, filters, history and console; the pager says its range once.
  ['dock-history-normal-1440x874', ''],
  ['dock-console-normal-1440x874', ''],
  ['dock-paged-problems-minimum-1000x650', ''],
  // Assets.
  ['asset-texture-normal-1440x874', ''],
  ['import-form-normal-1440x874', 'inspector'],
  // Project and engine states: one statement each, no illustration or badge.
  ['project-loading-normal-1440x874', ''],
  ['project-error-normal-1440x874', ''],
  ['hierarchy-error-normal-1440x874', ''],
  ['empty-project-normal-1440x874', ''],
  ['no-engine-normal-1440x874', ''],
  ['incompatible-normal-1440x874', ''],
  ['bad-fixture-normal-1440x874', ''],
];

const read = (set) => JSON.parse(readFileSync(join(artifacts, set, 'report.json'), 'utf8'));
const before = read(beforeSet);
const after = read(afterSet);
const shots = join(handoff, 'screenshots/plain');
const evidence = join(handoff, 'evidence');
for (const dir of [join(shots, 'before'), join(shots, 'after'), evidence]) mkdirSync(dir, { recursive: true });

const sums = [];
let copied = 0;
for (const [capture, crop] of PICKS) {
  const file = `${capture}${crop ? `-${crop}` : ''}.png`;
  for (const [which, set] of [['before', beforeSet], ['after', afterSet]]) {
    const source = join(artifacts, set, file);
    if (!existsSync(source)) throw new Error(`Missing ${source}`);
    const target = join(shots, which, file);
    copyFileSync(source, target);
    sums.push(`${createHash('sha256').update(readFileSync(target)).digest('hex')}  screenshots/plain/${which}/${file}`);
    copied += 1;
  }
}
writeFileSync(join(evidence, 'plain.sha256'), `${sums.join('\n')}\n`);

const summary = (capture) => {
  const m = capture?.measure ?? {};
  return capture
    ? {
        visibleWords: m.visibleWords ?? null,
        firstValueTop: m.firstValueTop ?? null,
        fixedChrome: m.fixedChrome ?? null,
        clippedValues: (m.clippedValues ?? []).length,
        outOfBounds: (m.outOfBounds ?? []).length,
        pageHorizontalOverflow: m.pageHorizontalOverflow ?? null,
        pageVerticalOverflow: m.pageVerticalOverflow ?? null,
        dockRowsMounted: m.dockRows ?? null,
        axe: capture.axe ? capture.axe.map((v) => v.id) : 'not run',
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
const words = (c, area) => c.measure?.visibleWords?.[area] ?? 0;
const AREAS = ['titlebar', 'banner', 'hierarchy', 'viewport', 'dock', 'inspector', 'agent', 'statusbar', 'dialog', 'menu', 'page'];
const comparison = {
  generated: new Date().toISOString(),
  browser: after.browser,
  note: 'Headless Chrome captures of the built UI at device scale 2; not native WebKit. Sample fixture, synthetic provider fixtures and the evidence test double; no real account data. visibleWords counts words of visible text inside the window and every clipping ancestor; visually hidden (spoken-only) text is excluded, so it also falls when less of a scrolling list fits on screen.',
  // Repository-relative, so no local path is committed.
  before: { set: beforeSet, ui: 'revision 4 with correctness follow-ups, 5cfa59d', uiRoot: before.uiRoot ? relative(repoRoot, before.uiRoot) : null },
  after: { set: afterSet, ui: 'revision 5 (plain), this commit', uiRoot: after.uiRoot ? relative(repoRoot, after.uiRoot) : null },
  totals: {
    captures: totals(() => 1),
    visibleWords: Object.fromEntries(AREAS.map((area) => [area, totals((c) => words(c, area))])),
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
writeFileSync(join(evidence, 'plain-comparison.json'), `${JSON.stringify(comparison, null, 2)}\n`);
console.log(`Copied ${copied} screenshots to screenshots/plain; wrote evidence/plain-comparison.json and plain.sha256.`);
console.log(JSON.stringify(comparison.totals));
