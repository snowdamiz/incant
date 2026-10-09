#!/usr/bin/env node
/**
 * Handoff 0004 evidence. Serves editor/ui/dist with `vite preview` and captures the
 * BROWSER FIXTURE titlebar at 1x and 2x, plus the full fixture window, with a local
 * Chrome. These are browser-fixture captures, not native-window proof.
 *
 * Usage (from the worktree root):
 *   node handoffs/0004-wisp-mascot/tools/capture-titlebar.mjs <label>
 * Writes handoffs/0004-wisp-mascot/screenshots/<label>/.
 */
import { existsSync, mkdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const uiRoot = join(repoRoot, 'editor/ui');
const require = createRequire(join(uiRoot, 'package.json'));
const { chromium } = require('playwright-core');
const { preview } = await import(require.resolve('vite'));

const label = process.argv[2];
if (!label) throw new Error('Pass a label, e.g. before or after.');
const out = join(here, '..', 'screenshots', label);
mkdirSync(out, { recursive: true });

const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome']
  .filter(Boolean)
  .find((p) => existsSync(p));
if (!executablePath) throw new Error('No Chrome found. Set CHROME_PATH.');

const server = await preview({ root: uiRoot, preview: { port: 4194, strictPort: true, open: false }, logLevel: 'silent' });
const url = 'http://localhost:4194/';
const browser = await chromium.launch({ executablePath, headless: true });
try {
  for (const scale of [1, 2]) {
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: scale });
    const external = [];
    page.on('request', (r) => {
      if (!r.url().startsWith(url) && !r.url().startsWith('data:')) external.push(r.url());
    });
    await page.goto(url, { waitUntil: 'networkidle' });
    const logo = page.locator('.titlebar__logo');
    await logo.waitFor();
    const ok = await logo.evaluate((img) => img.complete && img.naturalWidth > 0);
    if (!ok) throw new Error('Titlebar logo failed to load.');
    const box = await logo.boundingBox();
    // Titlebar identity region, cropped so the logo is legible.
    await page.screenshot({ path: join(out, `titlebar-identity@${scale}x.png`), clip: { x: 0, y: 0, width: 360, height: 44 } });
    await logo.screenshot({ path: join(out, `titlebar-logo-only@${scale}x.png`) });
    if (scale === 1) await page.screenshot({ path: join(out, 'fixture-window@1x.png') });
    console.log(JSON.stringify({ scale, logoBox: box, externalRequests: external }));
    await page.close();
  }
} finally {
  await browser.close();
  await new Promise((r) => server.httpServer.close(r));
}
