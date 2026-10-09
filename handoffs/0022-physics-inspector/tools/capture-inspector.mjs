#!/usr/bin/env node
/**
 * BROWSER FIXTURE captures for handoff 0022: physics components in the Inspector.
 * Serves editor/ui/dist with `vite preview` on port 4192, drives local Chrome through
 * the pinned playwright-core, and writes PNGs plus report.json to
 * handoffs/0022-physics-inspector/screenshots/<set>/.
 *
 * Every image is the built UI rendered by headless Chrome over the shipped, read-only
 * `?fixture=sample` data (synthetic, no capabilities), whose physics schemas are
 * checked against the native bridge conversion of schemas/*.schema.json by
 * PhysicsFields.test.tsx. CSP is bypassed only to inject axe-core. None of these
 * images is native WebKit evidence and none shows engine-rendered pixels.
 *
 * Usage (repo root, after `npm run build --workspace editor/ui`):
 *   node handoffs/0022-physics-inspector/tools/capture-inspector.mjs <set>
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
// scroll: component region to bring into view before capture.
const STATES = [
  { key: 'crate-body', entity: 'Crate 01', scroll: 'RigidBody' },
  { key: 'crate-collider', entity: 'Crate 01', scroll: 'Collider' },
  { key: 'post-capsule-masks', entity: 'Mooring Post', scroll: 'Collider' },
  { key: 'trigger-sphere-sensor', entity: 'Wave Trigger', scroll: 'Collider' },
  { key: 'crate-axis-error', entity: 'Crate 03', scroll: 'Collider' },
  { key: 'unknown-shape-focus', entity: 'Pier Planks', scroll: 'Collider', focusNotice: true },
];

const server = await preview({ root: uiRoot, preview: { port: 4192, strictPort: true, host: '127.0.0.1' }, logLevel: 'error' });
const origin = 'http://127.0.0.1:4192';
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
      const region = page.locator('[data-region="inspector"]').getByRole('region', { name: state.scroll, exact: true });
      await region.waitFor();
      if (state.focusNotice) {
        // Keyboard path: Tab from the row before reaches the focusable mismatch notice.
        await page.locator('.control--notice').first().focus();
        await page.keyboard.press('Shift+Tab');
        await page.keyboard.press('Tab');
      }
      await region.evaluate((el) => el.scrollIntoView({ block: 'start' }));
      const measure = await page.evaluate((name) => {
        const inspector = document.querySelector('[data-region="inspector"]');
        const scroll = inspector?.querySelector('.panel__scroll');
        const section = [...document.querySelectorAll('section.component')].find((s) => s.getAttribute('aria-label') === name);
        const rect = (el) => {
          if (!el) return null;
          const r = el.getBoundingClientRect();
          return { x: Math.round(r.x), y: Math.round(r.y), width: Math.round(r.width), height: Math.round(r.height) };
        };
        const clipped = [...(section?.querySelectorAll('input') ?? [])]
          .filter((input) => input.scrollWidth > input.clientWidth + 1)
          .map((input) => input.getAttribute('aria-label') ?? input.closest('.field')?.querySelector('.field__label')?.textContent);
        return {
          rows: [...(section?.querySelectorAll('.field') ?? [])].map((row) => ({
            label: row.querySelector('.field__label')?.textContent,
            values: [...row.querySelectorAll('input')].map((i) => i.value),
            height: Math.round(row.getBoundingClientRect().height),
            value: rect(row.querySelector('.field__value')),
          })),
          sections: [...(section?.querySelectorAll('.component__section-title') ?? [])].map((t) => t.textContent),
          maskStrip: rect(section?.querySelector('.mask-strip')),
          maskCell: rect(section?.querySelector('.mask-strip__cell')),
          clippedInputs: clipped,
          active: document.activeElement?.className ?? null,
          activeOutline: document.activeElement ? getComputedStyle(document.activeElement).outlineStyle : null,
          inspector: rect(inspector),
          horizontalOverflow: scroll ? scroll.scrollWidth - scroll.clientWidth : null,
          outputTabs: [...document.querySelectorAll('[role="tab"]')].map((t) => t.textContent?.trim()),
        };
      }, state.scroll);
      await page.addScriptTag({ path: axePath });
      const axe = await page.evaluate(async () => {
        // eslint-disable-next-line no-undef
        const result = await axe.run(document, { resultTypes: ['violations'] });
        return result.violations.map((v) => ({ id: v.id, impact: v.impact, nodes: v.nodes.length, targets: v.nodes.slice(0, 3).map((n) => n.target.join(' ')) }));
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
