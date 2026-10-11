#!/usr/bin/env node
/**
 * Compares a re-run of the unchanged 0033 capture tool on the union build with 0033's
 * committed per-state "after" metrics (handoffs/0033-editor-design-review/evidence/
 * plain-comparison.json, the plain revision). Same 96 states, same data and sizes, so
 * any difference is a layout or text change in the union. Browser evidence only.
 *
 * Usage (repo root): node handoffs/0035-editor-union-review/tools/compare-sweep.mjs <0033-set> [--out=file]
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const set = process.argv[2];
if (!set || !/^[a-z0-9-]+$/.test(set)) throw new Error('Usage: compare-sweep.mjs <0033-set> [--out=file]');
const outFlag = process.argv.find((a) => a.startsWith('--out='))?.slice('--out='.length);
const baseline = JSON.parse(readFileSync(join(repoRoot, 'handoffs/0033-editor-design-review/evidence/plain-comparison.json'), 'utf8'));
const report = JSON.parse(readFileSync(join(repoRoot, 'artifacts/0033-editor-design-review', set, 'report.json'), 'utf8'));

/** The same summary fields 0033 recorded per state. */
const summary = (c) => ({
  visibleWords: c.measure.visibleWords,
  firstValueTop: c.measure.firstValueTop ?? null,
  fixedChrome: c.measure.fixedChrome ?? null,
  clippedValues: (c.measure.clippedValues ?? []).length,
  outOfBounds: (c.measure.outOfBounds ?? []).length,
  pageHorizontalOverflow: c.measure.pageHorizontalOverflow,
  pageVerticalOverflow: c.measure.pageVerticalOverflow,
  dockRowsMounted: c.measure.dockRows,
  // 0033 records a state whose axe run is skipped (axe: false) as "not run".
  axe: c.axe ? c.axe.map((v) => v.id) : 'not run',
  stepError: c.stepError,
  timedOut: c.timedOut,
  pageErrors: c.errors.length,
  remoteRequests: c.remoteRequests.length,
});

const diffs = [];
const identical = [];
for (const state of baseline.states) {
  const capture = report.captures.find((c) => c.file === `${state.capture}.png`);
  if (!capture) {
    diffs.push({ capture: state.capture, missing: true });
    continue;
  }
  const now = summary(capture);
  const before = state.after;
  const changed = {};
  for (const [key, value] of Object.entries(before)) {
    if (key === 'visibleWords') {
      for (const [area, words] of Object.entries(value)) {
        if (words !== now.visibleWords[area]) changed[`visibleWords.${area}`] = [words, now.visibleWords[area]];
      }
    } else if (JSON.stringify(value) !== JSON.stringify(now[key])) {
      changed[key] = [value, now[key]];
    }
  }
  if (Object.keys(changed).length) diffs.push({ capture: state.capture, changed });
  else identical.push(state.capture);
}
const result = {
  generated: new Date().toISOString(),
  note: 'Unchanged 0033 capture tool and evidence double, re-run on the union build (a20d258 UI). Compared with 0033 plain-comparison.json "after" (plain revision, before the cdf0fab status-dot change and Astra\'s 0d14f4f accessibility fixes). Headless Chrome; not native WebKit.',
  browser: { baseline: baseline.browser, union: report.browser },
  states: baseline.states.length,
  identical: identical.length,
  changed: diffs,
};
const text = `${JSON.stringify(result, null, 2)}\n`;
if (outFlag) writeFileSync(resolve(outFlag), text);
console.log(`${identical.length} of ${baseline.states.length} states identical; ${diffs.length} differ`);
for (const d of diffs) console.log(d.capture, JSON.stringify(d.changed ?? d));
