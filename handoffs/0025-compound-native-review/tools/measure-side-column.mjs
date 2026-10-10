#!/usr/bin/env node
/**
 * BROWSER FIXTURE measurement for handoff 0025: Inspector/Agent space in the side
 * column at normal and minimum window sizes, across the provider fixtures that leave
 * the agent unavailable. Serves editor/ui/dist with `vite preview` on port 4196,
 * writes PNGs plus report.json to a NEW directory (an existing one is refused).
 * Not native WebKit evidence.
 *
 * Usage (repo root, after the UI build):
 *   node handoffs/0025-compound-native-review/tools/measure-side-column.mjs OUT_DIR
 */
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import { preview } from 'vite';

const here = dirname(fileURLToPath(import.meta.url));
const uiRoot = join(resolve(here, '../../..'), 'editor/ui');
if (!process.argv[2]) throw new Error('Usage: measure-side-column.mjs OUT_DIR');
const outDir = resolve(process.argv[2]);
if (existsSync(outDir)) throw new Error(`${outDir} already exists; choose a new directory.`);
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find(
  (p) => p && existsSync(p),
);
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');
mkdirSync(outDir, { recursive: true });

const SIZES = [
  { width: 1000, height: 650 },
  { width: 1440, height: 874 },
  { width: 1440, height: 900 },
  { width: 1920, height: 1080 },
];
const PROVIDERS = ['signed-out', 'signed-in', 'checking', 'error'];
const ENTITY = 'Rubble Pile';

const server = await preview({ root: uiRoot, preview: { port: 4196, strictPort: true, host: '127.0.0.1' }, logLevel: 'error' });
const origin = 'http://127.0.0.1:4196';
const browser = await chromium.launch({ executablePath, headless: true });
const report = { generated: new Date().toISOString(), browser: browser.version(), source: 'browser fixture', captures: [] };
try {
  for (const size of SIZES) {
    for (const provider of PROVIDERS) {
      const context = await browser.newContext({ viewport: size, deviceScaleFactor: 2, colorScheme: 'dark' });
      const page = await context.newPage();
      const errors = [];
      page.on('pageerror', (e) => errors.push(String(e)));
      await page.goto(`${origin}/?fixture=sample&provider=${provider}`, { waitUntil: 'networkidle' });
      await page.getByRole('treeitem', { name: new RegExp(`^${ENTITY}`) }).click();
      await page.locator('[data-region="inspector"] .object-list').waitFor();
      const measure = await page.evaluate(() => {
        const h = (sel) => {
          const el = document.querySelector(sel);
          return el ? Math.round(el.getBoundingClientRect().height) : null;
        };
        const transcript = document.querySelector('.agent__transcript');
        const empty = document.querySelector('.agent-empty');
        const scroller = document.querySelector('[data-region="inspector"] .panel__scroll');
        const composer = document.querySelector('.composer');
        const agent = document.querySelector('[data-region="agent"]');
        return {
          inspector: h('[data-region="inspector"]'),
          inspectorScroll: scroller ? Math.round(scroller.clientHeight) : null,
          agent: h('[data-region="agent"]'),
          transcript: transcript ? { client: transcript.clientHeight, scroll: transcript.scrollHeight, overflow: transcript.scrollHeight > transcript.clientHeight + 1, tabbable: transcript.tabIndex === 0 } : null,
          emptyText: empty?.innerText.replace(/\s+/g, ' ').trim(),
          composerInside: composer && agent ? composer.getBoundingClientRect().bottom <= agent.getBoundingClientRect().bottom + 0.5 : null,
          splitter: document.querySelector('[aria-label="Resize agent panel"]')?.getAttribute('aria-valuenow'),
        };
      });
      const file = `${provider}-${size.width}x${size.height}.png`;
      const side = await page.locator('.column--side').boundingBox();
      await page.screenshot({ path: join(outDir, file), clip: side });
      report.captures.push({ file, size, provider, measure, errors });
      await context.close();
    }
  }
} finally {
  await browser.close();
  await new Promise((done) => server.httpServer.close(done));
}
writeFileSync(join(outDir, 'report.json'), `${JSON.stringify(report, null, 2)}\n`);
for (const c of report.captures) console.log(c.file, JSON.stringify(c.measure), c.errors.length ? c.errors : '');
