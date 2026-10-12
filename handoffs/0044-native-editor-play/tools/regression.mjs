#!/usr/bin/env node
/**
 * Before/after element captures for handoff 0044, from one UI root's production build:
 * views this handoff must leave as they were (the ChatGPT account dialog in two states,
 * the titlebar, the hierarchy, an entity's authored Inspector) and two it changes on
 * purpose (the shortcut reference gains a Play section; the viewport header gains the
 * transport). Sample fixtures only; no bridge double, no native window. Compare two sets
 * with handoffs/0035-editor-union-review/tools/pixel-diff.mjs.
 *
 * Usage (repo root): node handoffs/0044-native-editor-play/tools/regression.mjs <uiRoot> <set>
 *   <uiRoot> is editor/ui, or an extracted baseline's editor/ui, each built first.
 */
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { chromium } from 'playwright-core';
import { preview } from 'vite';
import { repoRoot } from './play-double.mjs';

const [uiArg, set] = process.argv.slice(2);
if (!uiArg || !set || !/^[a-z0-9-]+$/.test(set)) throw new Error('Usage: regression.mjs <uiRoot> <set>');
const uiRoot = resolve(uiArg);
const outDir = join(repoRoot, 'artifacts/0044-native-editor-play', set);
if (existsSync(outDir)) throw new Error(`${outDir} exists; choose a new set name.`);
if (!existsSync(join(uiRoot, 'dist/index.html'))) throw new Error(`Build ${uiRoot} first.`);
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find((p) => p && existsSync(p));
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');
mkdirSync(outDir, { recursive: true });

const VIEWS = [
  { name: 'account-signed-in-multi', query: '?fixture=sample&provider=signed-in-multi', open: (p) => p.getByRole('button', { name: /^ChatGPT / }).click(), shot: '.dialog' },
  { name: 'account-signed-out', query: '?fixture=sample&provider=signed-out', open: (p) => p.getByRole('button', { name: /^ChatGPT / }).click(), shot: '.dialog' },
  { name: 'titlebar', query: '?fixture=sample', shot: '.titlebar' },
  { name: 'hierarchy', query: '?fixture=sample', shot: '[data-region="hierarchy"]' },
  { name: 'inspector-crate-03', query: '?fixture=sample', open: (p) => p.getByRole('treeitem', { name: /^Crate 03/ }).click(), shot: '[data-region="inspector"]' },
  { name: 'shortcuts-dialog', query: '?fixture=sample', open: (p) => p.getByRole('button', { name: 'Keyboard shortcuts' }).click(), shot: '.dialog', changes: 'Play section added' },
  { name: 'viewport-header', query: '?fixture=sample', shot: '.panel--viewport > .panel__header', changes: 'transport controls added' },
];

const port = 4290 + Math.floor(Math.random() * 40);
const server = await preview({ root: uiRoot, logLevel: 'warn', preview: { port, strictPort: true, host: '127.0.0.1' } });
const browser = await chromium.launch({ executablePath, headless: true });
const record = { uiRoot, browser: browser.version(), views: [] };
try {
  for (const view of VIEWS) {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2, reducedMotion: 'reduce', colorScheme: 'dark' });
    const page = await context.newPage();
    await page.goto(`http://127.0.0.1:${port}/${view.query}`, { waitUntil: 'networkidle' });
    await page.evaluate(() => document.fonts.ready);
    await page.waitForSelector('.app');
    if (view.open) await view.open(page);
    // Park the pointer where it hovers nothing, and let transitions settle.
    await page.mouse.move(1430, 890);
    await page.waitForTimeout(350);
    await page.locator(view.shot).first().screenshot({ path: join(outDir, `${view.name}.png`) });
    record.views.push({ name: view.name, query: view.query, element: view.shot, changes: view.changes ?? null });
    await context.close();
  }
} finally {
  await browser.close();
  await new Promise((done) => server.httpServer.close(done));
}
writeFileSync(join(outDir, 'views.json'), `${JSON.stringify(record, null, 2)}\n`);
console.log(`${record.views.length} views in ${outDir}`);
