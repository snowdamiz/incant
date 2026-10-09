#!/usr/bin/env node
/**
 * Fidelity check: rasterizes brand/wisp-mascot.svg at reference scale in Chrome and
 * compares its body and eye coverage with reference.png inside the mascot crop.
 * Writes screenshots/trace/overlay.png (reference, trace outline in red) and
 * screenshots/trace/diff.png (mismatch > 50% coverage in magenta), and prints IoU.
 * Usage (from the worktree root): node handoffs/0004-wisp-mascot/tools/compare-trace.mjs
 */
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromePath } from './decode.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const require = createRequire(join(repoRoot, 'editor/ui/package.json'));
const { chromium } = require('playwright-core');
const out = join(here, '..', 'screenshots', 'trace');
mkdirSync(out, { recursive: true });

const trace = JSON.parse(readFileSync(join(here, 'trace.json'), 'utf8'));
const ref = readFileSync(join(here, '..', 'reference.png')).toString('base64');
const C = { x: 400, y: 130, w: 460, h: 600 };
const browser = await chromium.launch({ executablePath: chromePath, headless: true });
try {
  const page = await browser.newPage();
  const res = await page.evaluate(
    async ({ ref, trace, C }) => {
      const img = new Image();
      img.src = `data:image/png;base64,${ref}`;
      await img.decode();
      const canvas = (w, h) => { const c = document.createElement('canvas'); c.width = w; c.height = h; return c; };
      const refC = canvas(C.w, C.h).getContext('2d');
      refC.drawImage(img, -C.x, -C.y);
      const r = refC.getImageData(0, 0, C.w, C.h).data;
      // Trace raster: body purple, eyes white, on white.
      const t = canvas(C.w, C.h).getContext('2d');
      t.fillStyle = '#fff'; t.fillRect(0, 0, C.w, C.h);
      t.translate(-C.x, -C.y);
      t.fillStyle = trace.color; t.fill(new Path2D(trace.paths.body));
      t.fillStyle = '#fff'; for (const e of trace.paths.eyes) t.fill(new Path2D(e));
      const d = t.getImageData(0, 0, C.w, C.h).data;
      const cov = (a, i) => Math.min(1, Math.max(0, (a[i + 2] - a[i] + 2) / 125));
      let inter = 0, union = 0, mism = 0, maxDev = 0, bi = 0, bu = 0;
      const diff = canvas(C.w, C.h); const dc = diff.getContext('2d');
      dc.drawImage(img, -C.x, -C.y); dc.fillStyle = 'rgba(255,255,255,0.75)'; dc.fillRect(0, 0, C.w, C.h);
      const dd = dc.getImageData(0, 0, C.w, C.h);
      for (let i = 0; i < r.length; i += 4) {
        const a = cov(r, i), b = cov(d, i);
        inter += Math.min(a, b); union += Math.max(a, b);
        if (a >= 0.5 && b >= 0.5) bi++; if (a >= 0.5 || b >= 0.5) bu++;
        maxDev = Math.max(maxDev, Math.abs(a - b));
        if (Math.abs(a - b) > 0.5) { mism++; dd.data[i] = 255; dd.data[i + 1] = 0; dd.data[i + 2] = 255; dd.data[i + 3] = 255; }
      }
      dc.putImageData(dd, 0, 0);
      // Overlay: reference with red trace outline.
      const ov = canvas(C.w * 2, C.h * 2); const oc = ov.getContext('2d');
      oc.imageSmoothingEnabled = false; oc.scale(2, 2); oc.drawImage(img, -C.x, -C.y);
      oc.translate(-C.x, -C.y); oc.strokeStyle = '#ff2020'; oc.lineWidth = 0.6;
      oc.stroke(new Path2D(trace.paths.body)); for (const e of trace.paths.eyes) oc.stroke(new Path2D(e));
      return { iou: inter / union, binaryIou: bi / bu, mismatchedPixels: mism, overlay: ov.toDataURL('image/png'), diff: diff.toDataURL('image/png') };
    },
    { ref, trace, C },
  );
  writeFileSync(join(out, 'overlay.png'), Buffer.from(res.overlay.split(',')[1], 'base64'));
  writeFileSync(join(out, 'diff.png'), Buffer.from(res.diff.split(',')[1], 'base64'));
  console.log(JSON.stringify({ softIou: res.iou, binaryIou: res.binaryIou, mismatchedPixels: res.mismatchedPixels }));
} finally {
  await browser.close();
}
