#!/usr/bin/env node
/**
 * BROWSER FIXTURE captures for handoff 0024: compound collider parts in the Inspector.
 * Serves editor/ui/dist with `vite preview` on port 4194, drives local Chrome through
 * the pinned playwright-core, and writes PNGs plus report.json to a NEW directory
 * (an existing one is refused, so earlier evidence is never overwritten).
 *
 * Every image is the built UI rendered by headless Chrome over the shipped, read-only
 * `?fixture=sample` data (synthetic, no capabilities). Its Collider schema is checked
 * against the native bridge conversion of schemas/Collider.schema.json by
 * PhysicsFields.test.tsx. CSP is bypassed only to inject axe-core. None of these
 * images is native WebKit evidence and none shows engine-rendered pixels.
 *
 * Usage (repo root, after `npm run build --workspace editor/ui`):
 *   node handoffs/0024-compound-inspector/tools/capture-inspector.mjs OUT_DIR
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
if (!process.argv[2]) throw new Error('Usage: capture-inspector.mjs OUT_DIR');
const outDir = resolve(process.argv[2]);
if (existsSync(outDir)) throw new Error(`${outDir} already exists; choose a new directory.`);
const axePath = createRequire(join(uiRoot, 'package.json')).resolve('axe-core/axe.min.js');
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find(
  (p) => p && existsSync(p),
);
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');
if (!existsSync(join(uiRoot, 'dist/index.html'))) throw new Error('Run the UI build first.');
mkdirSync(outDir, { recursive: true });

const SIZES = [
  { width: 1440, height: 900 },
  { width: 1000, height: 650 },
];
/**
 * keys: keyboard presses after Tab focus reaches the parts list (selection follows focus).
 * anchor: what to scroll to the top of the Inspector before capture.
 */
const STATES = [
  { key: 'arch-valid', entity: 'Stone Arch', anchor: 'parts' },
  { key: 'arch-rotated-keystone', entity: 'Stone Arch', anchor: 'detail', keys: ['End'] },
  { key: 'handcart-mixed', entity: 'Handcart', anchor: 'parts', keys: ['End'] },
  { key: 'handcart-collision', entity: 'Handcart', anchor: 'collision' },
  { key: 'railing-axis-error', entity: 'Broken Railing', anchor: 'parts' },
  { key: 'railing-rotation-error', entity: 'Broken Railing', anchor: 'detail', keys: ['ArrowDown'] },
  { key: 'railing-missing-tag', entity: 'Broken Railing', anchor: 'detail', keys: ['ArrowDown', 'ArrowDown'] },
  { key: 'railing-unknown-tag', entity: 'Broken Railing', anchor: 'detail', keys: ['ArrowDown', 'ArrowDown', 'ArrowDown'] },
  { key: 'railing-not-object', entity: 'Broken Railing', anchor: 'detail', keys: ['End'] },
  { key: 'rubble-64-start', entity: 'Rubble Pile', anchor: 'component' },
  { key: 'rubble-64-scrolled', entity: 'Rubble Pile', anchor: 'parts', keys: ['PageDown', 'PageDown', 'PageDown', 'PageDown', 'PageDown', 'PageDown', 'PageDown', 'ArrowDown', 'ArrowDown'] },
  { key: 'crate-primitive', entity: 'Crate 01', anchor: 'component' },
  { key: 'post-capsule-masks', entity: 'Mooring Post', anchor: 'component' },
  { key: 'saved-accounts-shell', entity: 'Handcart', anchor: 'component', provider: 'signed-in-multi' },
];

const server = await preview({ root: uiRoot, preview: { port: 4194, strictPort: true, host: '127.0.0.1' }, logLevel: 'error' });
const origin = 'http://127.0.0.1:4194';
const browser = await chromium.launch({ executablePath, headless: true });
const report = { generated: new Date().toISOString(), browser: browser.version(), source: 'browser fixture (?fixture=sample)', captures: [] };
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
      const query = `fixture=sample${state.provider ? `&provider=${state.provider}` : ''}`;
      await page.goto(`${origin}/?${query}`, { waitUntil: 'networkidle' });
      await page.getByRole('treeitem', { name: new RegExp(`^${state.entity}`) }).click();
      const inspector = page.locator('[data-region="inspector"]');
      const collider = inspector.getByRole('region', { name: 'Collider', exact: true });
      await collider.waitFor();
      // Keyboard path: Tab from the Shape row reaches the parts list in one stop.
      const tabStops = [];
      const list = collider.getByRole('listbox');
      if (await list.count()) {
        await collider.getByRole('textbox', { name: 'Shape', exact: true }).focus();
        await page.keyboard.press('Tab');
        tabStops.push(await page.evaluate(() => document.activeElement?.getAttribute('aria-label')));
        for (const key of state.keys ?? []) await page.keyboard.press(key);
      }
      const anchor = {
        component: collider,
        parts: collider.locator('.object-list'),
        detail: collider.locator('.object-list__detail'),
        collision: collider.getByRole('group', { name: 'Collision' }),
      }[state.anchor];
      // Scroll only the Inspector's own scroller: scrollIntoView would also scroll the
      // document and crop the titlebar (seen in the 0022 captures).
      await anchor.evaluate((el) => {
        const scroller = el.closest('.panel__scroll');
        scroller.scrollTop += el.getBoundingClientRect().top - scroller.getBoundingClientRect().top;
      });
      await page.evaluate(() => window.scrollTo(0, 0));
      const measure = await page.evaluate(() => {
        const inspectorEl = document.querySelector('[data-region="inspector"]');
        const scroll = inspectorEl?.querySelector('.panel__scroll');
        const section = [...document.querySelectorAll('section.component')].find((s) => s.getAttribute('aria-label') === 'Collider');
        const listEl = section?.querySelector('[role="listbox"]');
        const rect = (el) => {
          if (!el) return null;
          const r = el.getBoundingClientRect();
          return { x: Math.round(r.x), y: Math.round(r.y), width: Math.round(r.width), height: Math.round(r.height) };
        };
        const active = document.activeElement;
        return {
          summary: section?.querySelector('.object-list__summary')?.textContent ?? null,
          options: listEl ? listEl.querySelectorAll('[role="option"]').length : 0,
          tabbableOptions: listEl ? listEl.querySelectorAll('[role="option"][tabindex="0"]').length : 0,
          list: rect(listEl),
          listScroll: listEl ? { top: listEl.scrollTop, height: listEl.scrollHeight, client: listEl.clientHeight } : null,
          detailCaption: section?.querySelector('.object-list__caption')?.textContent ?? null,
          detail: rect(section?.querySelector('.object-list__detail')),
          detailRows: [...(section?.querySelectorAll('.object-list__detail .field') ?? [])].map((row) => ({
            label: row.querySelector('.field__label')?.firstChild?.textContent ?? null,
            values: [...row.querySelectorAll('input')].map((i) => i.value),
          })),
          notices: [...(section?.querySelectorAll('.control--notice__text, .component__notice') ?? [])].map((n) => n.textContent),
          problems: [...(section?.querySelectorAll('.field__problem') ?? [])].map((n) => n.textContent),
          // Chrome inputs with text-overflow: ellipsis do not report scrollWidth overflow,
          // so measure the value's rendered width against the input's content box.
          clippedInputs: [...(section?.querySelectorAll('input') ?? [])]
            .filter((input) => {
              const style = getComputedStyle(input);
              const ctx = document.createElement('canvas').getContext('2d');
              ctx.font = `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`;
              const box = input.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
              return ctx.measureText(input.value).width > box + 0.5;
            })
            .map((input) => `${input.id}=${input.value}`),
          activeName: active?.getAttribute('aria-label') ?? active?.id ?? null,
          activeOutline: active ? getComputedStyle(active).outlineStyle : null,
          activeVisible: active ? (() => {
            const r = active.getBoundingClientRect();
            const s = scroll?.getBoundingClientRect();
            return !!s && r.top >= s.top && r.bottom <= s.bottom;
          })() : null,
          inspector: rect(inspectorEl),
          horizontalOverflow: scroll ? scroll.scrollWidth - scroll.clientWidth : null,
          dockTabs: [...document.querySelectorAll('[role="tab"]')].map((t) => t.textContent?.trim()),
        };
      });
      // Chrome's own accessibility tree for the list (names as assistive tech receives them).
      const listAria = (await list.count()) ? (await list.ariaSnapshot()).split('\n').slice(0, 8) : [];
      await page.addScriptTag({ path: axePath });
      const axe = await page.evaluate(async () => {
        // eslint-disable-next-line no-undef
        const result = await axe.run(document, { resultTypes: ['violations'] });
        return result.violations.map((v) => ({ id: v.id, impact: v.impact, nodes: v.nodes.length, targets: v.nodes.slice(0, 3).map((n) => n.target.join(' ')) }));
      });
      const file = `${state.key}-${size.width}x${size.height}.png`;
      await page.screenshot({ path: join(outDir, file) });
      const box = await inspector.boundingBox();
      const crop = `${state.key}-${size.width}x${size.height}-inspector.png`;
      await page.screenshot({ path: join(outDir, crop), clip: box });
      report.captures.push({ file, crop, size, state: state.key, entity: state.entity, keys: state.keys ?? [], tabStops, listAria, measure, axe, errors, remoteRequests: remote });
      await context.close();
    }
  }
} finally {
  await browser.close();
  await new Promise((done) => server.httpServer.close(done));
}
writeFileSync(join(outDir, 'report.json'), `${JSON.stringify(report, null, 2)}\n`);
console.log(`Wrote ${report.captures.length} captures to ${outDir}`);
