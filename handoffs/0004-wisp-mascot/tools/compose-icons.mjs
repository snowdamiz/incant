#!/usr/bin/env node
/**
 * Bakes the traced mascot (tools/trace.json) into the production SVGs:
 *   editor/ui/public/icon.svg      512 canvas, mascot height 488 (12 px margin)
 *   editor/ui/brand/icon-macos.svg 1024 canvas, mascot inside the 824 px macOS body
 *                                  (100 px inset) with the system-style drop shadow
 * Coordinates are baked (no transforms) and rounded to 0.1 unit.
 * Usage (from the worktree root): node handoffs/0004-wisp-mascot/tools/compose-icons.mjs
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const trace = JSON.parse(readFileSync(join(here, 'trace.json'), 'utf8'));
const { tight, color } = trace;
const cx = (tight.minX + tight.maxX) / 2;
const cy = (tight.minY + tight.maxY) / 2;
const h = tight.maxY - tight.minY;

function bake(d, size, height) {
  const s = height / h;
  let axis = 0;
  return d.replace(/-?\d+(?:\.\d+)?/g, (n) => {
    const v = axis++ % 2 === 0 ? (+n - cx) * s + size / 2 : (+n - cy) * s + size / 2;
    return (Math.round(v * 10) / 10).toString();
  });
}
const eyes = trace.paths.eyes.join('');

const icon = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <!--
    Incant application icon: the wisp mascot alone (no wordmark, no tile), on a
    transparent background. Traced from the director-supplied reference in
    handoffs/0004-wisp-mascot/ (see brand/README.md). Source of public/icon.png,
    rendered by scripts/render-icon.mjs. Used by the titlebar (18 px) and the favicon.
  -->
  <path fill="${color}" d="${bake(trace.paths.body, 512, 488)}"/>
  <path fill="#FFFFFF" d="${bake(eyes, 512, 488)}"/>
</svg>
`;

const macos = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
  <!--
    Incant application icon, macOS variant. The same wisp mascot as public/icon.svg,
    placed on the macOS icon grid: the mascot fills the 824 px body height inset
    100 px on a 1024 canvas, with no tile, and the system-style soft drop shadow
    stays inside the transparent margin. Rendered to brand/Incant.icns by
    scripts/render-macos-icon.mjs. See brand/README.md for provenance.
  -->
  <defs>
    <filter id="shadow" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="10" stdDeviation="10" flood-color="#000" flood-opacity="0.3" />
    </filter>
  </defs>
  <g filter="url(#shadow)">
    <path fill="${color}" d="${bake(trace.paths.body, 1024, 824)}"/>
    <path fill="#FFFFFF" d="${bake(eyes, 1024, 824)}"/>
  </g>
</svg>
`;

writeFileSync(join(repoRoot, 'editor/ui/public/icon.svg'), icon);
writeFileSync(join(repoRoot, 'editor/ui/brand/icon-macos.svg'), macos);
console.log(`Wrote public/icon.svg (${icon.length} B) and brand/icon-macos.svg (${macos.length} B)`);
