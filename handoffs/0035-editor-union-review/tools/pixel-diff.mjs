#!/usr/bin/env node
/**
 * Pixel comparison of two PNG directories in headless Chrome (canvas getImageData), so
 * no image library is needed. For each file name present in both, it reports whether
 * the decoded pixels are identical, how many differ, and the differing bounding box in
 * image pixels; for a differing pair it writes a mask (changed pixels in red over a dim
 * copy of the candidate) into the output directory.
 *
 * Usage (repo root):
 *   node handoffs/0035-editor-union-review/tools/pixel-diff.mjs <baselineDir> <candidateDir> <outDir> [--json=file]
 */
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { chromium } from 'playwright-core';

const [baselineDir, candidateDir, outDir] = process.argv.slice(2).filter((a) => !a.startsWith('--')).map((p) => resolve(p));
const jsonOut = process.argv.find((a) => a.startsWith('--json='))?.slice('--json='.length);
if (!baselineDir || !candidateDir || !outDir) throw new Error('Usage: pixel-diff.mjs <baselineDir> <candidateDir> <outDir> [--json=file]');
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find((p) => p && existsSync(p));
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');
mkdirSync(outDir, { recursive: true });

const names = readdirSync(baselineDir).filter((n) => n.endsWith('.png') && existsSync(join(candidateDir, n))).sort();
const browser = await chromium.launch({ executablePath, headless: true });
const page = await browser.newPage();
const results = [];
try {
  for (const name of names) {
    const a = `data:image/png;base64,${readFileSync(join(baselineDir, name)).toString('base64')}`;
    const b = `data:image/png;base64,${readFileSync(join(candidateDir, name)).toString('base64')}`;
    const result = await page.evaluate(async ([srcA, srcB]) => {
      const load = async (src) => { const img = new Image(); img.src = src; await img.decode(); return img; };
      const [ia, ib] = await Promise.all([load(srcA), load(srcB)]);
      const pixels = (img) => {
        const c = document.createElement('canvas');
        c.width = img.naturalWidth;
        c.height = img.naturalHeight;
        const ctx = c.getContext('2d', { willReadFrequently: true });
        ctx.drawImage(img, 0, 0);
        return ctx.getImageData(0, 0, c.width, c.height);
      };
      const pa = pixels(ia);
      const pb = pixels(ib);
      if (pa.width !== pb.width || pa.height !== pb.height) return { sameSize: false, a: [pa.width, pa.height], b: [pb.width, pb.height] };
      let differing = 0;
      let box = null;
      // Differing pixels per 32 x 32 cell, so separate changes are located individually.
      const cells = new Map();
      const mask = new ImageData(pb.width, pb.height);
      for (let i = 0; i < pa.data.length; i += 4) {
        const same = pa.data[i] === pb.data[i] && pa.data[i + 1] === pb.data[i + 1] && pa.data[i + 2] === pb.data[i + 2] && pa.data[i + 3] === pb.data[i + 3];
        if (same) {
          mask.data[i] = pb.data[i] * 0.25; mask.data[i + 1] = pb.data[i + 1] * 0.25; mask.data[i + 2] = pb.data[i + 2] * 0.25; mask.data[i + 3] = 255;
          continue;
        }
        differing += 1;
        mask.data[i] = 255; mask.data[i + 1] = 40; mask.data[i + 2] = 40; mask.data[i + 3] = 255;
        const p = i / 4;
        const x = p % pa.width;
        const y = Math.floor(p / pa.width);
        box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
        const cell = `${Math.floor(x / 32) * 32},${Math.floor(y / 32) * 32}`;
        cells.set(cell, (cells.get(cell) ?? 0) + 1);
      }
      const clusters = [...cells].sort((p, q) => q[1] - p[1]).slice(0, 24).map(([at, count]) => ({ at, count }));
      let maskUrl = null;
      if (differing) {
        const c = document.createElement('canvas');
        c.width = pb.width;
        c.height = pb.height;
        c.getContext('2d').putImageData(mask, 0, 0);
        maskUrl = c.toDataURL('image/png');
      }
      return { sameSize: true, size: [pa.width, pa.height], differing, box, clusters, maskUrl };
    }, [a, b]);
    const { maskUrl, ...entry } = result;
    if (maskUrl) {
      entry.mask = name.replace('.png', '-diff.png');
      writeFileSync(join(outDir, entry.mask), Buffer.from(maskUrl.split(',')[1], 'base64'));
    }
    results.push({ name, ...entry });
    console.log(`${name}: ${entry.sameSize === false ? 'SIZE DIFFERS' : entry.differing ? `${entry.differing} px differ in [${entry.box.join(', ')}]` : 'identical'}`);
  }
} finally {
  await browser.close();
}
const summary = { generated: new Date().toISOString(), baselineDir, candidateDir, compared: results.length, identical: results.filter((r) => r.differing === 0).length, results };
if (jsonOut) writeFileSync(resolve(jsonOut), `${JSON.stringify(summary, null, 2)}\n`);
console.log(`${summary.identical} of ${summary.compared} identical`);
