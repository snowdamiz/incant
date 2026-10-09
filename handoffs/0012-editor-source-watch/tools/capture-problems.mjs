#!/usr/bin/env node
/**
 * BROWSER FIXTURE captures of the Problems dock for handoff 0012. Serves editor/ui/dist with
 * `vite preview` on port 4192, drives local Chrome through the pinned playwright-core, and
 * writes PNGs plus report.json to handoffs/0012-editor-source-watch/screenshots/<set>/.
 *
 * Every image is the built UI rendered by headless Chrome over SYNTHETIC data. The bridge
 * below is a capture-only evidence double: it holds hand-written diagnostics, refuses every
 * command and cooks nothing. The first diagnostic copies the text of the native source error
 * seen in w11; the others stress long nested paths, unbroken tokens and entity reveal.
 * None of these images is native WebKit evidence.
 *
 * Usage (repo root, after `npm run build --workspace editor/ui`):
 *   node handoffs/0012-editor-source-watch/tools/capture-problems.mjs <set>
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
const set = process.argv[2] ?? 'after';
const outDir = join(here, '..', 'screenshots', set);
mkdirSync(outDir, { recursive: true });
const axePath = createRequire(join(uiRoot, 'package.json')).resolve('axe-core/axe.min.js');
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find(
  (p) => p && existsSync(p),
);
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');

const ulid = (n) => `01J9ZF1XTR${String(n).padStart(16, '0')}`;
const LONG_PATH = 'models/environment/harbor/district_02/props/weathered_pier_lantern_cluster_variant_b.gltf';
const UNBROKEN = `textures/${'x'.repeat(96)}.png`;
const DIAGNOSTICS = {
  source: [
    {
      id: 'asset-source:1', severity: 'error', entity: null, component: null, path: 'triangle.gltf',
      message: 'could not cook triangle.gltf: invalid glTF: expected value at line 1 column 1',
    },
  ],
  stress: [
    {
      id: 'asset-source:1', severity: 'error', entity: null, component: null, path: 'triangle.gltf',
      message: 'could not cook triangle.gltf: invalid glTF: expected value at line 1 column 1',
    },
    {
      id: 'asset-source:2', severity: 'error', entity: null, component: null, path: LONG_PATH,
      message: `could not cook ${LONG_PATH}: invalid glTF: missing field \`asset\` at line 14 column 3`,
    },
    {
      id: 'asset-source:3', severity: 'error', entity: null, component: null, path: UNBROKEN,
      message: `could not cook ${UNBROKEN}: unsupported image data at byte 4096`,
    },
    {
      id: 'entity:1', severity: 'warning', entity: ulid(101), component: 'incant.light', path: 'intensity',
      message: 'Intensity is negative and is clamped to 0 at line 3 column 18',
    },
  ],
};

const double = (diagnostics) => `(() => {
  const snapshot = {
    connection: { status: 'ready', project: { id: '${ulid(0)}', name: 'Source Watch Fixture' } },
    hierarchy: { status: 'ready', value: { roots: ['${ulid(100)}'], nodes: {
      '${ulid(100)}': { id: '${ulid(100)}', name: 'Main', kind: 'scene', parent: null, children: ['${ulid(101)}'] },
      '${ulid(101)}': { id: '${ulid(101)}', name: 'Sun', kind: 'light', parent: '${ulid(100)}', children: [] },
    } } },
    schemas: {}, entities: {}, diagnostics: ${JSON.stringify(diagnostics)},
    history: { entries: [], applied: 0 },
    console: [],
    assets: { status: 'ready', value: [] },
    assetImport: { available: false, reason: 'Evidence double.' },
    provider: { status: 'not-connected', provider: 'openai' },
    agent: { status: 'unavailable', reason: 'Evidence double.' },
    viewport: { status: 'not-attached', reason: 'Evidence double: no native surface.' },
  };
  window.__INCANT_BRIDGE__ = {
    protocolVersion: 1,
    capabilities: [],
    label: 'Evidence double (not engine)',
    isFixture: false,
    getSnapshot: () => snapshot,
    subscribe: () => () => {},
    dispatch: () => Promise.resolve({ ok: false, error: { code: 'evidence.refused', message: 'Evidence double.' } }),
    request: () => Promise.resolve({ ok: true }),
  };
})();`;

const server = await preview({ root: uiRoot, logLevel: 'warn', preview: { port: 4192, strictPort: true, host: '127.0.0.1' } });
const origin = 'http://127.0.0.1:4192';
const browser = await chromium.launch({ executablePath, headless: true });
const report = {
  browser: `Chrome ${browser.version()} headless via playwright-core, deviceScaleFactor 1`,
  data: 'SYNTHETIC: capture-only evidence double; not engine state and not native WebKit',
  shots: [],
  rows: [],
  axe: [],
  keyboard: [],
  errors: [],
};

async function measure(p, label) {
  const result = await p.evaluate(() => {
    const list = document.querySelector('ul[aria-label="Problems"]');
    const rows = [...(list?.querySelectorAll('.list__row') ?? [])].map((row) => {
      const main = row.querySelector('.list__main');
      const meta = row.querySelector('.list__meta');
      const box = row.getBoundingClientRect();
      const mainBox = main.getBoundingClientRect();
      const metaBox = meta.getBoundingClientRect();
      return {
        message: main.textContent,
        meta: meta.textContent,
        rowSize: `${Math.round(box.width)}x${Math.round(box.height)}`,
        mainClipped: main.scrollWidth > main.clientWidth + 1 || main.scrollHeight > main.clientHeight + 1,
        metaClipped: meta.scrollWidth > meta.clientWidth + 1 || meta.scrollHeight > meta.clientHeight + 1,
        mainOutsideRow: mainBox.right > box.right + 0.5,
        metaOutsideRow: metaBox.right > box.right + 0.5,
        metaWidth: Math.round(metaBox.width),
      };
    });
    return {
      dock: `${Math.round(list.getBoundingClientRect().width)}px wide`,
      listHorizontalOverflow: list.scrollWidth > list.clientWidth + 1,
      rows,
    };
  });
  report.rows.push({ label, ...result });
}

try {
  for (const [w, h] of [[1000, 650], [1440, 900]]) {
    const size = `${w}x${h}`;
    for (const [name, diagnostics] of Object.entries(DIAGNOSTICS)) {
      const context = await browser.newContext({ viewport: { width: w, height: h }, deviceScaleFactor: 1, colorScheme: 'dark', bypassCSP: true });
      const p = await context.newPage();
      await p.addInitScript(double(diagnostics));
      p.on('pageerror', (e) => report.errors.push(String(e)));
      p.on('console', (m) => (m.type() === 'error' ? report.errors.push(m.text()) : null));
      await p.goto(`${origin}/`, { waitUntil: 'networkidle' });
      await p.evaluate(() => document.fonts.ready);
      await p.waitForSelector('ul[aria-label="Problems"]');
      await p.waitForTimeout(200);
      await p.screenshot({ path: join(outDir, `problems-${name}-${size}.png`) });
      const dock = await p.locator('[data-region="dock"]').boundingBox();
      await p.screenshot({ path: join(outDir, `problems-${name}-${size}-dock.png`), clip: dock });
      report.shots.push(`problems-${name}-${size}.png`, `problems-${name}-${size}-dock.png`);
      await measure(p, `${name} ${size}`);
      if (name === 'stress') {
        await p.evaluate(() => {
          const list = document.querySelector('ul[aria-label="Problems"]');
          list.scrollTop = list.scrollHeight;
        });
        await p.waitForTimeout(100);
        await p.screenshot({ path: join(outDir, `problems-stress-${size}-dock-scrolled.png`), clip: dock });
        report.shots.push(`problems-stress-${size}-dock-scrolled.png`);
        await p.addScriptTag({ path: axePath });
        const violations = await p.evaluate(async () => {
          // eslint-disable-next-line no-undef
          const out = await axe.run('[data-region="dock"]', { resultTypes: ['violations'] });
          return out.violations.map((v) => ({ id: v.id, nodes: v.nodes.map((n) => n.target.join(' ')).slice(0, 5) }));
        });
        report.axe.push({ label: `dock ${size}`, violations });
        // Keyboard: focus the entity row, check the focus ring, press Enter, check the reveal.
        const entityRow = p.locator('ul[aria-label="Problems"] button.list__row');
        await entityRow.focus();
        await p.keyboard.press('Shift+Tab');
        await p.keyboard.press('Tab');
        const focus = await p.evaluate(() => {
          const el = document.activeElement;
          const st = getComputedStyle(el);
          return {
            active: `${el.tagName.toLowerCase()} "${el.textContent.trim().slice(0, 60)}"`,
            focusVisible: el.matches(':focus-visible'),
            outline: `${st.outlineStyle} ${st.outlineWidth} ${st.outlineColor}`,
          };
        });
        await p.screenshot({ path: join(outDir, `problems-stress-${size}-focus.png`), clip: dock });
        report.shots.push(`problems-stress-${size}-focus.png`);
        await p.keyboard.press('Enter');
        await p.waitForTimeout(150);
        const selected = await p.evaluate(() => document.querySelector('[role="treeitem"][aria-selected="true"]')?.textContent?.trim() ?? null);
        report.keyboard.push({ label: size, ...focus, selectedAfterEnter: selected });
      }
      await context.close();
    }
  }
} finally {
  await browser.close();
  await new Promise((r) => server.httpServer.close(r));
}
writeFileSync(join(outDir, 'report.json'), `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify({ shots: report.shots.length, errors: report.errors, keyboard: report.keyboard, axe: report.axe }, null, 2));
