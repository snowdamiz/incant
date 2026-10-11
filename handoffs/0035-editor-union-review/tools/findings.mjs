#!/usr/bin/env node
/**
 * Browser reproductions of the 0035 findings, printed as plain text. Headless Chrome on
 * the built UI (`npm run build --workspace editor/ui`), first free port from 4233, closed
 * on exit. Synthetic data only: the sample/provider fixtures and union-double.mjs.
 *
 *   1. Account switch: keyboard focus after "Use <account>", and whether Escape closes.
 *   2. Sprite channels: accessible names of the Tint, Border and Pivot inputs.
 *   3. Inactive window: titlebar contrast with host chrome focused and unfocused.
 *   4. Dialog edge: whether the specks on the account dialog's top hairline are
 *      background text seen through its translucent ring (viewport text hidden or not).
 *   5. Import and reimport: where keyboard focus lands when the request settles (results
 *      come from the live double and are MOCKED).
 *
 * Usage (repo root): node handoffs/0035-editor-union-review/tools/findings.mjs [outDir]
 * With outDir, the two edge crops (finding 4) are written there.
 */
import { existsSync, mkdirSync } from 'node:fs';
import { createServer } from 'node:net';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import { preview } from 'vite';
import { unionInitScript, unionLiveInitScript } from './union-double.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const uiRoot = join(repoRoot, 'editor/ui');
const outDir = process.argv[2] ? resolve(process.argv[2]) : null;
const axePath = createRequire(join(uiRoot, 'package.json')).resolve('axe-core/axe.min.js');
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find((p) => p && existsSync(p));
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');
if (!existsSync(join(uiRoot, 'dist/index.html'))) throw new Error('Run the UI build first.');

async function freePort(start) {
  for (let port = start; port < start + 50; port += 1) {
    const free = await new Promise((done) => {
      const probe = createServer();
      probe.once('error', () => done(false));
      probe.listen(port, '127.0.0.1', () => probe.close(() => done(true)));
    });
    if (free) return port;
  }
  throw new Error('No free port found');
}

const MAC = { platform: 'macos', controls: 'native-overlay', maximized: false, fullscreen: false, focused: true, leadingInset: 78, trailingInset: 0 };
const SIGNED_IN = { status: 'connected', provider: 'openai', method: 'oauth', accountLabel: 'ada@example.com', accounts: [{ id: 'acct-fixture-1', label: 'ada@example.com' }], activeAccount: 'acct-fixture-1' };

const port = await freePort(4233);
const server = await preview({ root: uiRoot, preview: { port, strictPort: true, host: '127.0.0.1' }, logLevel: 'error' });
const origin = `http://127.0.0.1:${port}`;
const browser = await chromium.launch({ executablePath, headless: true });
const open = async (options, url = '/') => {
  const context = await browser.newContext({ viewport: { width: options.width ?? 1440, height: options.height ?? 874 }, deviceScaleFactor: 2, colorScheme: 'dark', bypassCSP: true });
  const page = await context.newPage();
  if (options.init) await page.addInitScript(options.init);
  await page.goto(`${origin}${url}`, { waitUntil: 'networkidle' });
  return { context, page };
};
const where = (page) => page.evaluate(() => {
  const el = document.activeElement;
  return el === document.body ? 'body' : `${el.tagName.toLowerCase()} "${el.getAttribute('aria-label') ?? el.textContent.trim().slice(0, 40)}"${el.closest('[role="dialog"]') ? ' (in dialog)' : ''}`;
});

try {
  console.log('1. Account switch (fixture signed-in-multi), keyboard only');
  {
    const { context, page } = await open({}, '/?fixture=sample&provider=signed-in-multi');
    await page.getByRole('button', { name: /^ChatGPT/ }).focus();
    await page.keyboard.press('Enter');
    console.log(`   open with Enter: ${await where(page)}`);
    await page.keyboard.press('Tab');
    console.log(`   Tab: ${await where(page)}`);
    await page.keyboard.press('Enter');
    await page.waitForTimeout(150);
    console.log(`   Enter on it: ${await where(page)}; account button: ${await page.getByRole('button', { name: /^ChatGPT/ }).getAttribute('aria-label')}`);
    await page.keyboard.press('Escape');
    await page.waitForTimeout(150);
    console.log(`   Escape straight after: dialogs still open = ${await page.getByRole('dialog').count()}`);
    await context.close();
  }

  console.log('2. Sprite channels (union double, Notice Board Panel)');
  {
    const { context, page } = await open({ init: unionInitScript() });
    await page.getByRole('treeitem', { name: /^Notice Board Panel/ }).click();
    const section = page.locator('[data-region="inspector"] section.component[aria-label="Sprite Renderer"]');
    for (const name of ['Tint', 'Border', 'Pivot']) {
      const names = await section.getByRole('group', { name, exact: true }).getByRole('textbox').evaluateAll((inputs) => inputs.map((input) => input.labels?.[0]?.textContent?.trim() ?? input.getAttribute('aria-label')));
      console.log(`   ${name}: textboxes named ${names.map((n) => `"${n}"`).join(', ')}`);
    }
    await context.close();
  }

  console.log('3. Titlebar contrast with macOS host chrome (axe color-contrast)');
  for (const focused of [true, false]) {
    const { context, page } = await open({ init: unionInitScript({ provider: SIGNED_IN, window: { ...MAC, focused } }) });
    await page.addScriptTag({ path: axePath });
    const rows = await page.evaluate(async () => {
      // eslint-disable-next-line no-undef
      const r = await axe.run(document.querySelector('.titlebar'), { runOnly: ['color-contrast'], resultTypes: ['violations', 'passes'] });
      return [...r.violations.map((v) => ['fails', v]), ...r.passes.map((v) => ['passes', v])].flatMap(([verdict, v]) => v.nodes
        .filter((n) => n.target.join(' ').includes('scene'))
        .map((n) => `${verdict} ${n.target.join(' ')} ${n.any[0]?.data?.fgColor} on ${n.any[0]?.data?.bgColor}: ${n.any[0]?.data?.contrastRatio}:1`));
    });
    console.log(`   window focused=${focused}: ${rows.join('; ') || 'no scene crumb result'}`);
    await context.close();
  }

  console.log('4. Account dialog top edge at 1000x650 (fixture browser step)');
  {
    const { context, page } = await open({ width: 1000, height: 650 }, '/?fixture=sample&provider=browser');
    await page.getByRole('button', { name: /^ChatGPT/ }).click();
    await page.waitForTimeout(200);
    const box = await page.getByRole('dialog').boundingBox();
    const ring = await page.getByRole('dialog').evaluate((d) => getComputedStyle(d).boxShadow);
    const text = await page.evaluate(() => [...document.querySelectorAll('[data-region="viewport"] p, [data-region="viewport"] span')].map((n) => ({ text: n.textContent.trim().slice(0, 40), top: Math.round(n.getBoundingClientRect().top), bottom: Math.round(n.getBoundingClientRect().bottom) })).filter((n) => n.text.startsWith('Sample data')));
    console.log(`   dialog top ${box.y}; ring ${ring}; viewport text ${JSON.stringify(text)}`);
    if (outDir) {
      mkdirSync(outDir, { recursive: true });
      const clip = { x: box.x, y: box.y - 6, width: 140, height: 24 };
      await page.screenshot({ path: join(outDir, 'edge-normal.png'), clip });
      await page.addStyleTag({ content: '[data-region="viewport"] * { visibility: hidden !important; }' });
      await page.screenshot({ path: join(outDir, 'edge-viewport-hidden.png'), clip });
      console.log(`   crops written to ${outDir}`);
    }
    await context.close();
  }
  console.log('5. Keyboard focus after an import or reimport finishes (live double; results MOCKED)');
  for (const ok of [true, false]) {
    const result = ok ? { ok: true } : { ok: false, error: { code: 'asset.import', message: 'Mocked failure.' } };
    const { context, page } = await open({ init: unionLiveInitScript({ capabilities: ['asset.import'], assetImport: { available: true }, results: { 'asset.import': result } }) });
    await page.getByRole('tab', { name: /^Assets/ }).click();
    await page.getByRole('button', { name: 'Import' }).click();
    const path = page.getByRole('textbox', { name: 'Path 1' });
    await path.fill('sprites/dock_props.atlas.json');
    await path.press('Enter');
    await page.waitForTimeout(200);
    console.log(`   import, Enter in the path, result ok=${ok}: ${await where(page)}`);
    await page.getByRole('option', { name: /^harbor_ui/ }).click();
    const button = page.locator('#asset-inspector .asset-section__actions button');
    await button.focus();
    await page.keyboard.press('Enter');
    await page.waitForTimeout(200);
    console.log(`   reimport, Enter on the button, result ok=${ok}: ${await where(page)}`);
    await context.close();
  }

} finally {
  await browser.close();
  await new Promise((done) => server.httpServer.close(done));
}
