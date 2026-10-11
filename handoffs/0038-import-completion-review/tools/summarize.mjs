#!/usr/bin/env node
/**
 * Condenses capture.mjs reports into the observations result.md cites. Reads ignored
 * artifacts; writes a JSON summary (and prints a table) without changing the report.
 *
 * Usage: node handoffs/0038-import-completion-review/tools/summarize.mjs <set>[,<set>] [--out=path.json]
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const args = process.argv.slice(2);
const sets = args.find((a) => !a.startsWith('--'))?.split(',') ?? [];
if (sets.length === 0) throw new Error('Usage: summarize.mjs <set>[,<set>] [--out=path.json]');
const out = args.find((a) => a.startsWith('--out='))?.slice('--out='.length);

const summary = { sets: {}, totals: { captures: 0, steps: 0, axeViolations: 0, pageErrors: 0, remoteRequests: 0, stepErrors: 0, pageOverflow: 0, clippedInInspector: 0 } };
for (const set of sets) {
  const report = JSON.parse(readFileSync(join(repoRoot, 'artifacts/0038-import-completion-review', set, 'report.json'), 'utf8'));
  const captures = report.captures.map((capture) => {
    summary.totals.captures += 1;
    summary.totals.pageErrors += capture.errors.length;
    summary.totals.remoteRequests += capture.remoteRequests.length;
    summary.totals.stepErrors += capture.stepError ? 1 : 0;
    return {
      state: capture.state,
      size: `${capture.size.key} ${capture.size.width}x${capture.size.height}`,
      note: capture.note,
      stepError: capture.stepError,
      commands: capture.commands,
      results: capture.results,
      steps: capture.steps.map((s) => {
        const o = s.observed;
        summary.totals.steps += 1;
        summary.totals.axeViolations += s.axe.length;
        summary.totals.pageOverflow += o.pageOverflow.some((n) => n > 0) ? 1 : 0;
        summary.totals.clippedInInspector += o.clippedInInspector.length;
        return {
          step: s.name,
          file: s.file,
          inspectorWidth: o.inspectorWidth,
          notice: o.notice && { text: o.notice.text, role: o.notice.role, tone: o.notice.tone, lines: o.notice.paragraphs.map((p) => p.lines), contrast: o.notice.paragraphs.map((p) => p.contrast), inView: o.notice.inView, dismiss: o.notice.dismiss },
          statusLine: o.statusLine,
          pending: o.pending,
          action: o.action,
          header: o.header,
          paths: o.paths,
          justImported: o.justImported,
          historyShown: o.historyShown && { meta: o.historyShown.meta, nowAfter: o.historyShown.nowAfter },
          host: { applied: o.host.applied, of: o.host.entries.length, head: o.host.entries[o.host.applied - 1] ?? null },
          focus: s.focus.element === 'body' ? 'body' : `${s.focus.tag}${s.focus.role ? `[${s.focus.role}]` : ''} "${s.focus.name}" in ${s.focus.region}${s.focus.outline && !s.focus.outline.startsWith('none') ? ', ring' : ''}`,
          axe: s.axe.map((v) => v.id),
          clipped: o.clippedInInspector,
        };
      }),
      live: capture.live.filter((l) => !/search-field__status/.test(l.region) && l.step !== 'load')
        .map((l) => `${l.step} | ${l.region} | ${l.change}${l.added?.length ? ` +${JSON.stringify(l.added.join(''))}` : ''}${l.removed?.length ? ` -${JSON.stringify(l.removed.join(''))}` : ''}${l.change.startsWith('region') ? ` ${JSON.stringify(l.text)}` : ''}`),
    };
  });
  summary.sets[set] = { generated: report.generated, browser: report.browser, label: report.label, build: report.build, captures };
}

for (const [set, data] of Object.entries(summary.sets)) {
  console.log(`# ${set}  Chrome ${data.browser}`);
  for (const capture of data.captures) {
    console.log(`\n## ${capture.state} @ ${capture.size}${capture.stepError ? `  STEP ERROR ${capture.stepError}` : ''}`);
    for (const s of capture.steps) {
      console.log(`  [${s.step}] inspector ${s.inspectorWidth}px  notice: ${s.notice ? `${JSON.stringify(s.notice.text)} (${s.notice.role}, lines ${s.notice.lines.join('/')}, contrast ${s.notice.contrast.join('/')}, ${s.notice.inView ? 'in view' : 'NOT IN VIEW'})` : '—'}`);
      console.log(`      status line: ${JSON.stringify(s.statusLine)}  action: ${JSON.stringify(s.action)}  header: ${JSON.stringify(s.header)}${s.pending ? `  pending: ${JSON.stringify(s.pending)}` : ''}`);
      console.log(`      History shown: ${s.historyShown ? `${s.historyShown.meta}, Now after ${JSON.stringify(s.historyShown.nowAfter)}` : '—'}  host: ${s.host.applied}/${s.host.of} head ${JSON.stringify(s.host.head)}`);
      console.log(`      just imported: ${JSON.stringify(s.justImported)}  paths: ${JSON.stringify(s.paths)}`);
      console.log(`      focus: ${s.focus}  axe: ${s.axe.length}  clipped: ${s.clipped.length}`);
    }
    console.log(`  commands: ${JSON.stringify(capture.commands.map((c) => (c.type === 'asset.import' ? `import ${c.sources.map((x) => x.source).join(' + ')}` : c.type === 'entity.rename' ? `rename → ${c.name}` : c.type)))}`);
    console.log(`  results: ${JSON.stringify(capture.results)}`);
    for (const line of capture.live) console.log(`  live: ${line}`);
  }
}
console.log(`\nTotals: ${JSON.stringify(summary.totals)}`);
if (out) writeFileSync(resolve(repoRoot, out), `${JSON.stringify(summary, null, 2)}\n`);
