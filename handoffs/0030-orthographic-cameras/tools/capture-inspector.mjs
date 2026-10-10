#!/usr/bin/env node
/**
 * BROWSER captures of the read-only Camera Inspector for handoff 0030.
 *
 * Serves editor/ui/dist with `vite preview` on port 4193, drives local Chrome
 * through the pinned playwright-core, and writes PNGs plus report.json to a NEW
 * ignored directory artifacts/0030-orthographic-cameras/inspector/<set>/ (an
 * existing directory is refused). Selected crops are copied into the handoff.
 *
 * States:
 *   legacy        sample fixture "Camera Rig": no projection field (legacy JSON)
 *   orthographic  sample fixture "Map Camera": { kind: orthographic, vertical_size: 8 }
 *   ortho-focus   the same, keyboard focus moved by Tab onto Vertical size
 *   malformed     EVIDENCE TEST DOUBLE bridge holding one camera with projection
 *                 { kind: "isometric" }; it rejects every command and holds no document
 *
 * Widths: 1440x900 with the default Inspector width, and 1000x650 with the Inspector
 * splitter moved to its minimum by the keyboard (Home). Every image is the built UI
 * in headless Chrome; none is native WebKit evidence or engine-rendered pixels.
 *
 * Usage (repo root, after `npm run build --workspace editor/ui`):
 *   node handoffs/0030-orthographic-cameras/tools/capture-inspector.mjs <set>
 */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import { preview } from 'vite';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const uiRoot = join(repoRoot, 'editor/ui');
const set = process.argv[2];
if (!set || !/^[a-z0-9-]+$/.test(set)) throw new Error('Usage: capture-inspector.mjs <set-name>');
const outDir = join(repoRoot, 'artifacts/0030-orthographic-cameras/inspector', set);
if (existsSync(outDir)) throw new Error(`${outDir} exists; choose a new set name.`);
const axePath = createRequire(join(uiRoot, 'package.json')).resolve('axe-core/axe.min.js');
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find(
  (p) => p && existsSync(p),
);
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');
if (!existsSync(join(uiRoot, 'dist/index.html'))) throw new Error('Run the UI build first.');

// Same resolved schema as the fixture's Camera mirror (CameraFields.test.tsx proves the
// fixture equals snapshotFromEngine over schemas/Camera.schema.json).
const nativeSchema = JSON.parse(readFileSync(join(repoRoot, 'schemas/Camera.schema.json'), 'utf8'));
const MALFORMED_DOUBLE = `
  (() => {
    const id = '01JA30CA000000000000000001';
    const variant = (kind, extra = {}) => ({ type: 'object', additionalProperties: false,
      properties: { kind: { type: 'string', const: kind, enum: [kind], optional: false }, ...extra }, required: ['kind', ...Object.keys(extra)] });
    const camera = { type: 'Camera', version: 1, title: 'Camera', order: ['fov_degrees', 'near', 'far'], properties: {
      far: { type: 'number', format: 'double', optional: false },
      fov_degrees: { type: 'number', format: 'double', description: ${JSON.stringify(nativeSchema.properties.fov_degrees.description)}, optional: false },
      near: { type: 'number', format: 'double', optional: false },
      projection: { type: 'tagged-union', description: ${JSON.stringify(nativeSchema.properties.projection.description)}, discriminator: 'kind',
        variants: { perspective: variant('perspective'), orthographic: variant('orthographic', { vertical_size: { type: 'number', format: 'double',
          description: ${JSON.stringify(nativeSchema.$defs.CameraProjection.oneOf[1].properties.vertical_size.description)}, optional: false } }) },
        optional: true },
    } };
    const snapshot = {
      connection: { status: 'ready', project: { id: '01JA30CA000000000000000000', name: 'Malformed camera (test double)' } },
      hierarchy: { status: 'ready', value: { roots: [id], nodes: { [id]: { id, name: 'Isometric Shot', kind: 'camera', parent: null, children: [] } } } },
      schemas: { Camera: camera },
      entities: { [id]: { id, name: 'Isometric Shot', kind: 'camera', components: [
        { type: 'Camera', schemaVersion: 1, value: { fov_degrees: 60, near: 0.1, far: 100, projection: { kind: 'isometric' } } }] } },
      diagnostics: [], history: { entries: [], applied: 0 }, console: [],
      provider: { status: 'not-connected', provider: 'openai' },
      agent: { status: 'unavailable', reason: 'Evidence test double.' },
      viewport: { status: 'not-attached', reason: 'Evidence test double.' },
    };
    const refused = Promise.resolve({ ok: false, error: { code: 'test-double', message: 'Evidence test double.' } });
    window.__INCANT_BRIDGE__ = { protocolVersion: 1, capabilities: [], label: 'Evidence test double (not engine)', isFixture: false,
      getSnapshot: () => snapshot, subscribe: () => () => {}, dispatch: () => refused, request: () => refused };
  })();
`;

const SIZES = [
  { key: 'normal', width: 1440, height: 900, minimum: false },
  { key: 'minimum', width: 1000, height: 650, minimum: true },
];
const STATES = [
  { key: 'legacy', entity: 'Camera Rig', url: '/?fixture=sample' },
  { key: 'orthographic', entity: 'Map Camera', url: '/?fixture=sample' },
  { key: 'ortho-focus', entity: 'Map Camera', url: '/?fixture=sample', focus: 'Vertical size' },
  { key: 'malformed', entity: 'Isometric Shot', url: '/', init: MALFORMED_DOUBLE },
];

mkdirSync(outDir, { recursive: true });
const server = await preview({ root: uiRoot, preview: { port: 4193, strictPort: true, host: '127.0.0.1' }, logLevel: 'error' });
const origin = 'http://127.0.0.1:4193';
const browser = await chromium.launch({ executablePath, headless: true });
const report = { set, generated: new Date().toISOString(), browser: browser.version(), captures: [] };
try {
  for (const size of SIZES) {
    for (const state of STATES) {
      const context = await browser.newContext({
        viewport: { width: size.width, height: size.height },
        deviceScaleFactor: 2,
        colorScheme: 'dark',
        bypassCSP: true,
      });
      const page = await context.newPage();
      const errors = [];
      page.on('pageerror', (e) => errors.push(String(e)));
      page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
      const remote = [];
      page.on('request', (r) => !r.url().startsWith(origin) && !r.url().startsWith('data:') && remote.push(r.url()));
      if (state.init) await page.addInitScript(state.init);
      await page.goto(`${origin}${state.url}`, { waitUntil: 'networkidle' });
      if (size.minimum) {
        await page.getByRole('separator', { name: 'Resize inspector and agent' }).focus();
        await page.keyboard.press('Home');
      }
      await page.getByRole('treeitem', { name: new RegExp(`^${state.entity}`) }).click();
      const camera = page.locator('[data-region="inspector"]').getByRole('region', { name: 'Camera' });
      await camera.waitFor();
      const keys = [];
      if (state.focus) {
        // Keyboard path: Projection, then Tab to the variant field beneath it.
        await page.getByRole('textbox', { name: 'Projection' }).focus();
        await page.keyboard.press('Shift+Tab');
        await page.keyboard.press('Tab');
        keys.push(await page.evaluate(() => document.activeElement?.getAttribute('aria-labelledby')));
        await page.keyboard.press('Tab');
        keys.push(await page.evaluate(() => document.activeElement?.getAttribute('aria-labelledby')));
      }
      await camera.scrollIntoViewIfNeeded();
      const measure = await page.evaluate(() => {
        const parse = (c) => (c.match(/[\d.]+/g) ?? []).slice(0, 3).map(Number);
        const lum = ([r, g, b]) =>
          [r, g, b]
            .map((v) => v / 255)
            .map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4))
            .reduce((sum, v, i) => sum + v * [0.2126, 0.7152, 0.0722][i], 0);
        const bgOf = (el) => {
          for (let node = el; node; node = node.parentElement) {
            const bg = getComputedStyle(node).backgroundColor;
            if (bg && !bg.includes('rgba(0, 0, 0, 0)') && bg !== 'transparent') return bg;
          }
          return 'rgb(0, 0, 0)';
        };
        const contrast = (el) => {
          const a = lum(parse(getComputedStyle(el).color));
          const b = lum(parse(bgOf(el)));
          return Math.round(((Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05)) * 100) / 100;
        };
        const rect = (el) => {
          if (!el) return null;
          const r = el.getBoundingClientRect();
          return { x: Math.round(r.x), y: Math.round(r.y), width: Math.round(r.width), height: Math.round(r.height) };
        };
        const inspector = document.querySelector('[data-region="inspector"]');
        const scroll = inspector?.querySelector('.panel__scroll');
        const section = [...inspector.querySelectorAll('section.component')].find((s) => s.getAttribute('aria-label') === 'Camera');
        const rows = [...section.querySelectorAll('.field')].map((row) => {
          const label = row.querySelector('.field__label');
          const tag = row.querySelector('.control__tag');
          const unit = row.querySelector('.control__unit');
          const input = row.querySelector('input');
          return {
            label: label?.textContent,
            labelClipped: label ? label.scrollHeight > label.clientHeight + 1 : null,
            value: input?.value ?? null,
            valueClipped: input ? input.scrollWidth > input.clientWidth + 1 : null,
            unit: unit?.textContent ?? null,
            tag: tag?.textContent ?? null,
            notice: row.querySelector('.control--notice')?.textContent ?? null,
            box: rect(row.querySelector('.field__value > *')),
            contrast: {
              label: label ? contrast(label) : null,
              value: input ? contrast(input) : null,
              unit: unit ? contrast(unit) : null,
              tag: tag ? contrast(tag) : null,
            },
          };
        });
        const active = document.activeElement;
        const focusStyle = active && active.closest('.control') ? getComputedStyle(active.closest('.control--number, .control--choice, .control, input') ?? active) : null;
        return {
          rows,
          inspector: rect(inspector),
          horizontalOverflow: scroll ? scroll.scrollWidth - scroll.clientWidth : null,
          variantGroups: section.querySelectorAll('.field-group--variant').length,
          readOnlyChip: !!inspector.querySelector('.quiet-chip'),
          editableInputs: [...section.querySelectorAll('input')].filter((i) => !i.readOnly).length,
          focused: active?.getAttribute('aria-labelledby') ?? null,
          focusOutline: focusStyle ? `${focusStyle.outlineStyle} ${focusStyle.outlineWidth}` : null,
          outputTabs: [...document.querySelectorAll('[role="tab"]')].map((t) => t.textContent?.trim()),
        };
      });
      await page.addScriptTag({ path: axePath });
      const axeResult = await page.evaluate(async () => {
        // eslint-disable-next-line no-undef
        const result = await axe.run(document, { resultTypes: ['violations'] });
        return result.violations.map((v) => ({ id: v.id, impact: v.impact, nodes: v.nodes.length }));
      });
      const file = `${state.key}-${size.key}-${size.width}x${size.height}.png`;
      await page.screenshot({ path: join(outDir, file) });
      const inspectorBox = await page.locator('[data-region="inspector"]').boundingBox();
      const crop = file.replace('.png', '-inspector.png');
      await page.screenshot({ path: join(outDir, crop), clip: inspectorBox });
      report.captures.push({ file, crop, size, state: state.key, entity: state.entity, keys, measure, axe: axeResult, errors, remoteRequests: remote });
      await context.close();
    }
  }
} finally {
  await browser.close();
  await new Promise((done) => server.httpServer.close(done));
}
writeFileSync(join(outDir, 'report.json'), `${JSON.stringify(report, null, 2)}\n`);
console.log(`Wrote ${report.captures.length} captures to ${outDir}`);
