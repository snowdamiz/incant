#!/usr/bin/env node
/**
 * Copies the concise, account-free evidence for handoff 0035 from the ignored artifacts
 * into the handoff (screenshots/<group>/ and evidence/), rewrites absolute paths in the
 * evidence JSON as repository-relative, and writes evidence/screenshots.sha256.
 * Screenshots are unedited captures, or crops/4x enlargements made with macOS `sips`
 * (marked "crop" below). Every account shown is an invented example address.
 *
 * Usage (repo root): node handoffs/0035-editor-union-review/tools/curate.mjs
 */
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const handoff = resolve(here, '..');
const artifacts = join(repoRoot, 'artifacts/0035-editor-union-review');

/** [group, source path under artifacts/0035-editor-union-review] */
const PICKS = [
  ['camera', 'r2/u-pixel-camera-normal-1440x874.png'],
  ['camera', 'r2/u-pixel-camera-minimum-1000x650.png'],
  ['camera', 'r2/u-pixel-bad-normal-1440x874-inspector.png'],
  ['camera', 'r2/u-broken-camera-normal-1440x874-inspector.png'],
  ['camera', 'r2/camera-rig-narrow-1440x874-inspector.png'],
  ['camera', 'r2/map-camera-narrow-1440x874-inspector.png'],
  ['camera', 'r2/closeup-camera-minimum-1000x650-inspector.png'],
  ['sprite', 'r2/u-sprite-sign-normal-1440x874-inspector.png'],
  ['sprite', 'r2/u-sprite-sign-minimum-1000x650.png'],
  ['sprite', 'r2/u-sprite-board-narrow-1440x874-inspector.png'],
  ['sprite', 'r2/u-sprite-coil-narrow-1440x874-inspector.png'],
  ['sprite', 'r2/u-sprite-lantern-normal-1440x874-inspector.png'],
  ['sprite', 'r2/u-sprite-broken-normal-1440x874-inspector.png'],
  ['sprite', 'r2/u-sprite-long-narrow-1440x874-inspector.png'],
  ['assets', 'r2/u-asset-atlas-normal-1440x874.png'],
  ['assets', 'r2/u-import-mirror-normal-1440x874-inspector.png'],
  ['account', 'r2/acct-multi-normal-1440x874-dialog.png'],
  ['account', 'r2/acct-saved-minimum-1000x650-dialog.png'],
  ['account', 'r2/acct-browser-minimum-1000x650-dialog.png'],
  ['account', 'r2/acct-error-signed-in-normal-1440x874-dialog.png'],
  ['account', 'r2/probe-account-normal-1440x874--question.png'],
  ['titlebar', 's1/title-windows-controls-minimum-1000x650-titlebar.png'],
  ['titlebar', 'r2/title-mac-minimum-1000x650-titlebar.png'],
  ['titlebar', 'r2/title-mac-inactive-normal-1440x874-titlebar.png'],
  ['titlebar', 'scratch/title-signed-in-end.png'], // crop of r2 title-signed-in titlebar
  ['titlebar', 'scratch/title-error-end.png'], // crop of r2 title-error titlebar
  ['titlebar', 'scratch/title-browser-end.png'], // crop of r2 title-browser titlebar
  ['shortcuts', 'r2/keys-dialog-minimum-1000x650.png'],
  ['shortcuts', 'r2/keys-windows-normal-1440x874-dialog.png'],
  ['shortcuts', 'r2/probe-shortcuts-normal-1440x874--filtered.png'],
  ['interaction', 's2/probe-end-tab-minimum-1000x650--end.png'],
  ['interaction', 'r2/probe-grid-paging-normal-1440x874-inspector.png'],
  ['interaction', 'scratch/grid-corner.png'], // crop of r2 probe-grid-paging --corner
  ['interaction', 'r2/probe-pager-minimum-1000x650--last.png'],
  ['interaction', 'r2/probe-menu-minimum-1000x650--open.png'],
  ['interaction', 'r2/probe-reveal-normal-1440x874--region.png'],
  ['regression', 'pixel-plain/dock-paged-problems-minimum-1000x650-diff.png'],
  ['regression', 'findings-edge/edge-normal-4x.png'], // findings.mjs crop, enlarged 4x with sips
  ['regression', 'findings-edge/edge-viewport-hidden-4x.png'], // the same with the viewport text hidden
];

/** Evidence JSON: [source under artifacts, destination name under evidence/]. */
const EVIDENCE = [
  ['summary-final.json', 'union-review.json'],
  ['sweep-compare.json', 'sweep-compare.json'],
  ['pixel-plain.json', 'pixel-plain.json'],
  ['pixel-repeat.json', 'pixel-repeat.json'],
];

const shots = join(handoff, 'screenshots');
const evidence = join(handoff, 'evidence');
if (existsSync(shots)) rmSync(shots, { recursive: true });
mkdirSync(evidence, { recursive: true });
const lines = [];
for (const [group, source] of PICKS) {
  const from = join(artifacts, source);
  if (!existsSync(from)) throw new Error(`Missing ${from}`);
  const to = join(shots, group, source.split('/').pop());
  mkdirSync(dirname(to), { recursive: true });
  copyFileSync(from, to);
  lines.push(`${createHash('sha256').update(readFileSync(to)).digest('hex')}  ${relative(handoff, to)}`);
}
const relativize = (text) => text.split(repoRoot).join('.');
for (const [source, name] of EVIDENCE) {
  const text = relativize(readFileSync(join(artifacts, source), 'utf8'));
  writeFileSync(join(evidence, name), text);
  lines.push(`${createHash('sha256').update(text).digest('hex')}  ${relative(handoff, join(evidence, name))}`);
}
writeFileSync(join(evidence, 'screenshots.sha256'), `${lines.join('\n')}\n`);
console.log(`Copied ${PICKS.length} screenshots and ${EVIDENCE.length} evidence files; ${lines.length} hashes in evidence/screenshots.sha256`);
