#!/usr/bin/env node
/**
 * Compact, committable summary of one or more 0035 capture sets (report.json under the
 * ignored artifacts/0035-editor-union-review/<set>/): page-level checks for every
 * capture, the Inspector rows and notices the union states show, the titlebar account
 * control, dialogs, and every probe's recorded observations. Browser evidence only.
 *
 * Usage (repo root): node handoffs/0035-editor-union-review/tools/summarize.mjs <set>[,<set>...] [--out=file]
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const sets = process.argv[2]?.split(',').filter(Boolean);
if (!sets?.length || sets.some((s) => !/^[a-z0-9-]+$/.test(s))) throw new Error('Usage: summarize.mjs <set>[,<set>...] [--out=file]');
const outFlag = process.argv.find((a) => a.startsWith('--out='))?.slice('--out='.length);
const reports = sets.map((set) => ({ set, ...JSON.parse(readFileSync(join(repoRoot, 'artifacts/0035-editor-union-review', set, 'report.json'), 'utf8')) }));

// A capture repeated in a later set (a supplement re-run) replaces the earlier one.
const byFile = new Map();
for (const r of reports) for (const c of r.captures) byFile.set(c.file, { set: r.set, ...c });
const captures = [...byFile.values()];
const totals = {
  captures: captures.length,
  axeViolations: captures.flatMap((c) => (c.axe ?? []).map((v) => ({ capture: c.file, id: v.id, targets: v.targets }))),
  axeNotRun: captures.filter((c) => !c.axe).map((c) => c.file),
  stepErrors: captures.filter((c) => c.stepError).map((c) => ({ capture: c.file, error: c.stepError })),
  timeouts: captures.filter((c) => c.timedOut).map((c) => c.file),
  pageErrors: captures.reduce((n, c) => n + c.errors.length, 0),
  remoteRequests: captures.reduce((n, c) => n + c.remoteRequests.length, 0),
  windowScroll: captures.filter((c) => c.measure && (c.measure.pageHorizontalOverflow > 0 || c.measure.pageVerticalOverflow > 0)).map((c) => c.file),
  inspectorHorizontalOverflow: captures.filter((c) => c.measure?.horizontalOverflow?.length).map((c) => ({ capture: c.file, overflow: c.measure.horizontalOverflow })),
  outOfBounds: captures.filter((c) => c.measure?.outOfBounds?.length).map((c) => ({ capture: c.file, count: c.measure.outOfBounds.length })),
  clippedValues: captures.filter((c) => c.measure?.clippedValues?.length).map((c) => ({ capture: c.file, values: c.measure.clippedValues.map((v) => `${v.name}: ${v.value}`) })),
  mostDockRows: Math.max(...captures.map((c) => c.measure?.dockRows ?? 0)),
  dialogsNotFitting: captures.filter((c) => c.measure?.dialog && !c.measure.dialog.fits).map((c) => c.file),
};

const states = captures.map((c) => {
  const m = c.measure ?? {};
  return {
    capture: c.file.replace('.png', ''),
    set: c.set,
    rows: m.rows?.length ? m.rows : undefined,
    notices: m.unsupportedNotices?.length ? m.unsupportedNotices : undefined,
    account: m.account ? { size: m.account.rect && [m.account.rect.width, m.account.rect.height], name: m.account.name, title: m.account.title, visibleText: m.account.visibleText } : undefined,
    dialog: m.dialog ? { size: [m.dialog.rect.width, m.dialog.rect.height], fits: m.dialog.fits, bodyOverflow: m.dialog.bodyOverflow } : undefined,
    statusbar: m.statusbarText,
    live: m.live?.length ? m.live : undefined,
    visibleWords: m.visibleWords,
    observations: c.observations?.length ? c.observations : undefined,
    steps: Object.keys(c.steps ?? {}).length ? Object.values(c.steps) : undefined,
  };
});

const summary = {
  generated: new Date().toISOString(),
  note: 'Headless Chrome on the built union UI (a20d258 editor/ui) at device scale 2; sample fixture, synthetic provider fixtures and the 0035 evidence double (real schemas through the real compiled bridge). Not native WebKit, not the native app and not device evidence.',
  sets: reports.map((r) => ({ set: r.set, generated: r.generated, browser: r.browser, captures: r.captures.length })),
  totals,
  states,
};
const text = `${JSON.stringify(summary, null, 2)}\n`;
if (outFlag) writeFileSync(resolve(outFlag), text);
console.log(JSON.stringify(totals, null, 2));
