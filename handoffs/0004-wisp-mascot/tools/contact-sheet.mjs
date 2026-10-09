#!/usr/bin/env node
/**
 * Contact sheet for the new brand assets (browser rendering, Chrome).
 *  - public/icon.svg rasterized by Chrome at 16, 18, 32, 128, 512 px at 1x, on the
 *    app's dark surface (#0b0c0f) and on white; small sizes also shown 8x enlarged
 *    with nearest-neighbour scaling so the actual pixels can be inspected.
 *  - public/icon.png (512) composited on dark and light, for halo inspection.
 *  - Every member of brand/Incant.icns, unpacked with `iconutil`, on a checkerboard.
 * Writes screenshots/after/contact-sheet.png and screenshots/after/small-sizes-8x.png.
 * Usage (from the worktree root): node handoffs/0004-wisp-mascot/tools/contact-sheet.mjs
 */
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync } from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromePath } from './decode.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const ui = join(repoRoot, 'editor/ui');
const require = createRequire(join(ui, 'package.json'));
const { chromium } = require('playwright-core');
const out = join(here, '..', 'screenshots', 'after');
mkdirSync(out, { recursive: true });

const svg = readFileSync(join(ui, 'public/icon.svg'), 'utf8');
const svgUrl = `data:image/svg+xml;base64,${Buffer.from(svg).toString('base64')}`;
const pngUrl = `data:image/png;base64,${readFileSync(join(ui, 'public/icon.png')).toString('base64')}`;
const work = mkdtempSync(join(tmpdir(), 'incant-icns-'));
execFileSync('iconutil', ['-c', 'iconset', join(ui, 'brand/Incant.icns'), '-o', join(work, 'x.iconset')]);
const members = readdirSync(join(work, 'x.iconset'))
  .filter((f) => f.endsWith('.png'))
  .map((f) => ({ name: f, url: `data:image/png;base64,${readFileSync(join(work, 'x.iconset', f)).toString('base64')}` }))
  .sort((a, b) => a.name.localeCompare(b.name, 'en', { numeric: true }));
rmSync(work, { recursive: true, force: true });

const SIZES = [16, 18, 32, 128, 512];
const SURFACES = [
  { name: 'dark #0b0c0f', bg: '#0b0c0f', fg: '#c9ccd3' },
  { name: 'panel #131418', bg: '#131418', fg: '#c9ccd3' },
  { name: 'light #ffffff', bg: '#ffffff', fg: '#333' },
];
const browser = await chromium.launch({ executablePath: chromePath, headless: true });
try {
  // Pass 1: capture the real 1x rasters of small sizes on each surface.
  const small = {};
  const p1 = await browser.newPage({ deviceScaleFactor: 1, viewport: { width: 200, height: 200 } });
  for (const s of SURFACES) {
    for (const px of [16, 18, 32]) {
      await p1.setContent(`<body style="margin:0;background:${s.bg}"><img id=i src="${svgUrl}" width=${px} height=${px} style="display:block"></body>`);
      await p1.locator('#i').evaluate((img) => img.decode());
      const buf = await p1.locator('#i').screenshot();
      small[`${s.bg}-${px}`] = `data:image/png;base64,${buf.toString('base64')}`;
    }
  }
  await p1.close();

  const cell = (s, px) => `
    <div class=cell style="background:${s.bg};color:${s.fg}">
      <img src="${svgUrl}" width=${px} height=${px}>
      <span>${px}px</span>
    </div>`;
  const zoom = (s, px) => `
    <div class=cell style="background:${s.bg};color:${s.fg}">
      <img src="${small[`${s.bg}-${px}`]}" width=${px * 8} height=${px * 8} style="image-rendering:pixelated">
      <span>${px}px actual raster, 8x</span>
    </div>`;
  const html = `<!doctype html><html><head><style>
    body{margin:0;padding:24px;background:#e9eaee;font:13px/1.3 -apple-system,system-ui,sans-serif;color:#222;width:1752px}
    h1{font-size:18px;margin:0 0 4px} h2{font-size:14px;margin:20px 0 8px} p{margin:0 0 8px;color:#555}
    .row{display:flex;gap:12px;align-items:stretch;flex-wrap:wrap}
    .cell{display:flex;flex-direction:column;align-items:center;justify-content:center;gap:8px;padding:16px;border-radius:8px;min-width:72px}
    .cell span{font-size:11px;opacity:.8}
    .checker{background-color:#fff;background-image:linear-gradient(45deg,#ccc 25%,transparent 25%),linear-gradient(-45deg,#ccc 25%,transparent 25%),linear-gradient(45deg,transparent 75%,#ccc 75%),linear-gradient(-45deg,transparent 75%,#ccc 75%);background-size:16px 16px;background-position:0 0,0 8px,8px -8px,-8px 0}
  </style></head><body>
    <h1>Incant wisp mascot: asset contact sheet</h1>
    <p>Browser rendering (headless Chrome, 1x). Not a native-window capture.</p>
    ${SURFACES.map((s) => `<h2>public/icon.svg on ${s.name}</h2><div class=row>${SIZES.map((px) => cell(s, px)).join('')}</div>`).join('')}
    <h2>public/icon.png (512 raster) on dark and light, for halo inspection</h2>
    <div class=row>
      <div class=cell style="background:#0b0c0f;color:#ccc"><img src="${pngUrl}" width=256 height=256><span>256 display</span></div>
      <div class=cell style="background:#fff"><img src="${pngUrl}" width=256 height=256><span>256 display</span></div>
      <div class="cell checker"><img src="${pngUrl}" width=256 height=256><span>alpha on checkerboard</span></div>
    </div>
    <h2>brand/Incant.icns members (unpacked with iconutil), on checkerboard, shown at native pixel size</h2>
    <div class=row style="align-items:flex-end">${members
      .filter((m) => !m.name.includes('512x512@2x'))
      .map((m) => `<div class="cell checker"><img src="${m.url}" style="image-rendering:auto"><span style="color:#222">${m.name}</span></div>`)
      .join('')}</div>
  </body></html>`;
  const p2 = await browser.newPage({ deviceScaleFactor: 1, viewport: { width: 1800, height: 1000 } });
  await p2.setContent(html);
  await p2.evaluate(() => Promise.all([...document.images].map((i) => i.decode())));
  await p2.screenshot({ path: join(out, 'contact-sheet.png'), fullPage: true });

  const zoomHtml = `<!doctype html><html><head><style>
    body{margin:0;padding:24px;background:#e9eaee;font:13px/1.3 -apple-system,system-ui,sans-serif;width:1100px}
    h1{font-size:18px;margin:0 0 4px} p{margin:0 0 12px;color:#555}
    .row{display:flex;gap:12px;align-items:flex-end;margin-bottom:12px}
    .cell{display:flex;flex-direction:column;align-items:center;gap:8px;padding:16px;border-radius:8px}
    .cell span{font-size:11px;opacity:.8}
  </style></head><body>
    <h1>Small sizes: actual 1x Chrome rasters of public/icon.svg, enlarged 8x (nearest neighbour)</h1>
    <p>Browser rendering, not native. Checks eye openings and silhouette at 16, 18 and 32 px.</p>
    ${SURFACES.map((s) => `<div class=row>${[16, 18, 32].map((px) => zoom(s, px)).join('')}</div>`).join('')}
  </body></html>`;
  await p2.setContent(zoomHtml);
  await p2.evaluate(() => Promise.all([...document.images].map((i) => i.decode())));
  await p2.screenshot({ path: join(out, 'small-sizes-8x.png'), fullPage: true });
  console.log(`icns members: ${members.map((m) => m.name).join(', ')}`);
} finally {
  await browser.close();
}
