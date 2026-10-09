#!/usr/bin/env node
/**
 * BROWSER FIXTURE captures for handoff 0021: the Inspector's directional-light shadow
 * rows. Serves editor/ui/dist with `vite preview` on port 4191, drives local Chrome
 * through the pinned playwright-core, and writes PNGs plus report.json to
 * handoffs/0021-directional-shadows/screenshots/<set>/.
 *
 * Every image is the built UI rendered by headless Chrome over the shipped, read-only
 * `?fixture=sample` data (synthetic, no capabilities). "Sun" carries shadows
 * { distance: 120 } and "Sky Fill" omits shadows (CSP is bypassed only to inject axe-core); both use the fixture's mirror of
 * the native DirectionalLight.shadows schema. None of these images is native WebKit
 * evidence and none shows engine-rendered pixels.
 *
 * Usage (repo root, after `npm run build --workspace editor/ui`):
 *   node handoffs/0021-directional-shadows/tools/capture-inspector.mjs <set>
 */
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import { preview } from 'vite';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const uiRoot = join(repoRoot, 'editor/ui');
const set = process.argv[2] ?? 'initial';
const outDir = join(here, '..', 'screenshots', set);
mkdirSync(outDir, { recursive: true });
const axePath = createRequire(join(uiRoot, 'package.json')).resolve('axe-core/axe.min.js');
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find(
  (p) => p && existsSync(p),
);
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');
if (!existsSync(join(uiRoot, 'dist/index.html'))) throw new Error('Run the UI build first.');

const SIZES = [
  { width: 1440, height: 900 },
  { width: 1000, height: 650 },
];
const STATES = [
  { key: 'shadows-enabled', entity: 'Sun' },
  { key: 'shadows-disabled', entity: 'Sky Fill' },
  { key: 'shadows-disabled-focus', entity: 'Sky Fill', focus: true },
];

const server = await preview({ root: uiRoot, preview: { port: 4191, strictPort: true, host: '127.0.0.1' }, logLevel: 'error' });
const origin = 'http://127.0.0.1:4191';
const browser = await chromium.launch({ executablePath, headless: true });
const report = { set, generated: new Date().toISOString(), browser: browser.version(), captures: [] };
try {
  for (const size of SIZES) {
    for (const state of STATES) {
      const context = await browser.newContext({ viewport: size, deviceScaleFactor: 2, colorScheme: 'dark', bypassCSP: true });
      const page = await context.newPage();
      const errors = [];
      page.on('pageerror', (e) => errors.push(String(e)));
      page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
      const remote = [];
      page.on('request', (r) => !r.url().startsWith(origin) && !r.url().startsWith('data:') && remote.push(r.url()));
      await page.goto(`${origin}/?fixture=sample`, { waitUntil: 'networkidle' });
      await page.getByRole('treeitem', { name: new RegExp(`^${state.entity}`) }).click();
      const light = page.getByRole('region', { name: 'Light' });
      await light.waitFor();
      if (state.focus) {
        await page.getByRole('textbox', { name: 'Shadows' }).focus();
        await page.keyboard.press('Shift+Tab');
        await page.keyboard.press('Tab');
      }
      await light.scrollIntoViewIfNeeded();
      const measure = await page.evaluate(() => {
        const inspector = document.querySelector('[data-region="inspector"]');
        const scroll = inspector?.querySelector('.panel__scroll');
        const section = [...document.querySelectorAll('section.component')].find((s) => s.getAttribute('aria-label') === 'Light');
        const rect = (el) => {
          if (!el) return null;
          const r = el.getBoundingClientRect();
          return { x: Math.round(r.x), y: Math.round(r.y), width: Math.round(r.width), height: Math.round(r.height) };
        };
        const rows = [...(section?.querySelectorAll('.field') ?? [])].map((row) => ({
          label: row.querySelector('.field__label')?.textContent,
          value: row.querySelector('input')?.value ?? null,
          height: Math.round(row.getBoundingClientRect().height),
          valueBox: rect(row.querySelector('.field__value > *')),
        }));
        const unset = section?.querySelector('.control--unset input');
        const style = unset ? getComputedStyle(unset) : null;
        const box = section?.querySelector('.control--unset');
        const boxStyle = box ? getComputedStyle(box) : null;
        const sideBg = inspector ? getComputedStyle(inspector).backgroundColor : null;
        return {
          rows,
          group: section?.querySelector('fieldset legend')?.textContent ?? null,
          unset: unset
            ? { color: style.color, fontSize: style.fontSize, border: boxStyle.borderColor, outline: boxStyle.outlineStyle, describedBy: unset.getAttribute('aria-describedby') }
            : null,
          activeElement: document.activeElement?.getAttribute('aria-labelledby') ?? document.activeElement?.className ?? null,
          inspectorBackground: sideBg,
          inspector: rect(inspector),
          horizontalOverflow: scroll ? scroll.scrollWidth - scroll.clientWidth : null,
          regions: [...document.querySelectorAll('[data-region]')].map((r) => r.getAttribute('data-region')),
          titlebar: rect(document.querySelector('.titlebar')),
          assetsTab: !!document.querySelector('[role="tab"][aria-controls], [aria-label*="Assets"]'),
          outputTabs: [...document.querySelectorAll('[role="tab"]')].map((t) => t.textContent?.trim()),
        };
      });
      await page.addScriptTag({ path: axePath });
      const axe = await page.evaluate(async () => {
        // eslint-disable-next-line no-undef
        const result = await axe.run(document, { resultTypes: ['violations'] });
        return result.violations.map((v) => ({ id: v.id, impact: v.impact, nodes: v.nodes.length }));
      });
      const file = `${state.key}-${size.width}x${size.height}.png`;
      await page.screenshot({ path: join(outDir, file) });
      const inspectorBox = await page.locator('[data-region="inspector"]').boundingBox();
      const crop = `${state.key}-${size.width}x${size.height}-inspector.png`;
      await page.screenshot({ path: join(outDir, crop), clip: inspectorBox });
      report.captures.push({ file, crop, size, state: state.key, entity: state.entity, measure, axe, errors, remoteRequests: remote });
      await context.close();
    }
  }
} finally {
  await browser.close();
  await new Promise((done) => server.httpServer.close(done));
}
writeFileSync(join(outDir, 'report.json'), `${JSON.stringify(report, null, 2)}\n`);
console.log(`Wrote ${report.captures.length} captures to ${outDir}`);
