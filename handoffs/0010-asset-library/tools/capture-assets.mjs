#!/usr/bin/env node
/**
 * BROWSER FIXTURE captures for handoff 0010 (adapted from 0009). Revision 4: assets are a
 * left-column view beside the Hierarchy; details and import open in the Inspector. Serves editor/ui/dist with
 * `vite preview` on port 4190 (never the user's 4176), drives local Chrome through the pinned
 * playwright-core, and writes PNGs plus report.json to handoffs/0010-asset-library/screenshots/<set>/.
 *
 * Every image is the built UI rendered by headless Chrome over SYNTHETIC data:
 *  - `?fixture=...` is the shipped, read-only sample fixture (no capabilities).
 *  - "evidence doubles" below are injected only by this script. They advertise asset.import so
 *    the busy, failed and successful states can be reviewed. Their dispatch either never
 *    resolves (busy), returns a fixed error (failed), or swaps to a second, hand-written
 *    snapshot (scripted success). They cook nothing and are not engine output.
 * None of these images is native WebKit evidence.
 *
 * Usage (repo root, after `npm run build --workspace editor/ui`):
 *   node handoffs/0010-asset-library/tools/capture-assets.mjs <set>
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
const ASSETS = [
  { id: ulid(1), name: 'dock_kit', path: 'models/dock_kit.glb', kind: 'model', fingerprint: 'evidence-1' },
  { id: ulid(2), name: 'crate', path: 'models/props/crate.gltf', kind: 'model', fingerprint: 'evidence-2' },
  { id: ulid(3), name: 'Crate albedo', path: 'textures/crate_albedo.png', kind: 'texture', fingerprint: 'evidence-3', textureUsage: 'color' },
  { id: ulid(4), name: 'crate_normal', path: 'textures/crate_normal.png', kind: 'texture', fingerprint: 'evidence-4', textureUsage: 'normal' },
  { id: ulid(5), name: 'pier_roughness', path: 'textures/pier_roughness.jpg', kind: 'texture', fingerprint: 'evidence-5', textureUsage: 'linear' },
  {
    id: ulid(6),
    name: 'weathered_pier_planks_with_moss_and_salt_stains_variant_b',
    path: 'textures/environment/harbor/district_02/surfaces/wood/weathered_pier_planks_with_moss_and_salt_stains_variant_b_albedo.png',
    kind: 'texture',
    fingerprint: 'evidence-6',
    textureUsage: 'color',
  },
];
const IMPORTED = [
  { id: ulid(7), name: 'lantern', path: 'models/props/lantern.glb', kind: 'model', fingerprint: 'evidence-7' },
  { id: ulid(8), name: 'lantern_normal', path: 'textures/lantern_normal.png', kind: 'texture', fingerprint: 'evidence-8', textureUsage: 'normal' },
];

/**
 * Evidence double. `mode`: idle (dispatch refused), hold (never resolves), fail, conflict,
 * success (swaps to a hand-written post-import snapshot). `assets`/`assetImport` override.
 */
const double = ({ mode = 'idle', assets = ASSETS, assetImport = { available: true }, omitAssets = false } = {}) => `(() => {
  const base = (assets, applied) => ({
    connection: { status: 'ready', project: { id: '${ulid(0)}', name: 'Dock Prototype' } },
    hierarchy: { status: 'ready', value: { roots: ['${ulid(100)}'], nodes: {
      '${ulid(100)}': { id: '${ulid(100)}', name: 'Dock Prototype', kind: 'scene', parent: null, children: ['${ulid(101)}', '${ulid(102)}'] },
      '${ulid(101)}': { id: '${ulid(101)}', name: 'Sun', kind: 'light', parent: '${ulid(100)}', children: [] },
      '${ulid(102)}': { id: '${ulid(102)}', name: 'Main Camera', kind: 'camera', parent: '${ulid(100)}', children: [] },
    } } },
    schemas: {}, entities: {}, diagnostics: [],
    history: { entries: applied ? [{ transaction: '${ulid(900)}', description: 'Import assets', origin: { kind: 'user' }, at: '2026-10-09T10:00:00Z' }] : [], applied: applied ? 1 : 0 },
    console: [],
    ${omitAssets ? '' : `assets: { status: 'ready', value: assets },`}
    assetImport: ${JSON.stringify(assetImport)},
    provider: { status: 'not-connected', provider: 'openai' },
    agent: { status: 'unavailable', reason: 'Sign in with ChatGPT to use the agent.' },
    viewport: { status: 'not-attached', reason: 'Evidence double: no native surface.' },
  });
  let snapshot = base(${JSON.stringify(assets)}, false);
  const listeners = new Set();
  const mode = ${JSON.stringify(mode)};
  const dispatched = (window.__EVIDENCE_DISPATCHED__ = []);
  window.__INCANT_BRIDGE__ = {
    protocolVersion: 1,
    capabilities: ['entity.rename', 'entity.delete', 'asset.import', 'history.undo', 'history.redo'],
    label: 'Evidence double (not engine)',
    isFixture: false,
    getSnapshot: () => snapshot,
    subscribe: (l) => (listeners.add(l), () => listeners.delete(l)),
    dispatch: (command) => {
      dispatched.push(command);
      if (command.type !== 'asset.import') return Promise.resolve({ ok: false, error: { code: 'evidence.refused', message: 'Evidence double: only imports are scripted.' } });
      if (mode === 'hold') return new Promise(() => {});
      if (mode === 'fail') return Promise.resolve({ ok: false, error: { code: 'engine.request', message: 'could not cook models/props/lantern.glb: buffer 0 points outside the file (lantern.bin is missing)' } });
      if (mode === 'conflict') return Promise.resolve({ ok: false, error: { code: 'engine.request', message: 'document revision conflict: expected 41, current 42' } });
      if (mode === 'success') {
        snapshot = base([...${JSON.stringify(assets)}, ...${JSON.stringify(IMPORTED)}], true);
        listeners.forEach((l) => l());
        return Promise.resolve({ ok: true });
      }
      return Promise.resolve({ ok: false, error: { code: 'evidence.refused', message: 'Evidence double: imports are not scripted in this mode.' } });
    },
    request: () => Promise.resolve({ ok: true }),
  };
})();`;

const server = await preview({ root: uiRoot, logLevel: 'warn', preview: { port: 4190, strictPort: true, host: '127.0.0.1' } });
const origin = 'http://127.0.0.1:4190';
const browser = await chromium.launch({ executablePath, headless: true });
const report = {
  browser: `Chrome ${browser.version()} headless via playwright-core`,
  data: 'SYNTHETIC: shipped read-only sample fixture and capture-only evidence doubles; not engine state',
  shots: [],
  axe: [],
  focus: [],
  dispatched: [],
  layout: [],
  errors: [],
};

async function page(size, init) {
  const context = await browser.newContext({ viewport: size, deviceScaleFactor: 2, colorScheme: 'dark', bypassCSP: true });
  const p = await context.newPage();
  if (init) await p.addInitScript(init);
  p.on('pageerror', (e) => report.errors.push(String(e)));
  p.on('console', (m) => (m.type() === 'error' ? report.errors.push(m.text()) : null));
  return { context, p };
}
async function open(p, query = '') {
  await p.goto(`${origin}/${query}`, { waitUntil: 'networkidle' });
  await p.evaluate(() => document.fonts.ready);
  await p.waitForSelector('.app, .fatal');
  await p.getByRole('tab', { name: /^Assets/ }).click();
  await p.waitForTimeout(150);
}
async function shot(p, name, clipSelector) {
  await p.waitForTimeout(200);
  let clip;
  if (clipSelector) {
    const b = await p.locator(clipSelector).boundingBox();
    clip = { x: b.x, y: b.y, width: b.width, height: b.height };
  }
  await p.screenshot({ path: join(outDir, `${name}.png`), clip });
  report.shots.push(`${name}.png`);
}
async function axe(p, label) {
  await p.addScriptTag({ path: axePath });
  const violations = await p.evaluate(async () => {
    // eslint-disable-next-line no-undef
    const out = await axe.run(document, { resultTypes: ['violations'] });
    return out.violations.map((v) => ({ id: v.id, nodes: v.nodes.map((n) => n.target.join(' ')).slice(0, 5) }));
  });
  report.axe.push({ label, violations });
}
async function focusReport(p, label) {
  await p.waitForTimeout(120); // focus moves after render (setTimeout 0)
  report.focus.push({
    label,
    ...(await p.evaluate(() => {
      const el = document.activeElement;
      const st = el ? getComputedStyle(el) : null;
      return {
        active: `${el?.tagName.toLowerCase()} ${el?.getAttribute('role') ?? ''} "${(el?.getAttribute('aria-label') ?? el?.textContent ?? '').trim().slice(0, 60)}"`,
        matchesFocusVisible: el?.matches(':focus-visible') ?? false,
        outline: st ? `${st.outlineStyle} ${st.outlineWidth} ${st.outlineColor}` : null,
      };
    })),
  });
}
async function layoutReport(p, label) {
  report.layout.push({
    label,
    ...(await p.evaluate(() => {
      const r = (s) => {
        const b = document.querySelector(s)?.getBoundingClientRect();
        return b ? `${Math.round(b.width)}x${Math.round(b.height)}` : null;
      };
      const overflow = [...document.querySelectorAll('.asset-browser *, .asset-inspector *')].filter((e) => e.scrollWidth > e.clientWidth + 1 && getComputedStyle(e).overflow === 'visible').length;
      return { left: r('[data-region="hierarchy"]'), list: r('.asset-list'), inspector: r('[data-region="inspector"]'), visibleOverflowingElements: overflow };
    })),
  });
}
const typePaths = async (p, paths) => {
  for (const [index, path] of paths.entries()) {
    if (index > 0) await p.getByRole('button', { name: 'Add path' }).click();
    await p.getByRole('textbox', { name: `Path ${index + 1}` }).fill(path);
  }
};

try {
  for (const [w, h] of [[1000, 650], [1440, 900]]) {
    const size = `${w}x${h}`;
    const vp = { width: w, height: h };
    // 01 Populated (read-only shipped fixture).
    {
      const { context, p } = await page(vp);
      await open(p, '?fixture=sample');
      await shot(p, `01-populated-fixture-${size}`);
      await layoutReport(p, `populated ${size}`);
      // 02 Long path: open details on the long asset by keyboard.
      await p.getByRole('option', { name: /weathered_pier/ }).click();
      await shot(p, `02-details-long-path-fixture-${size}`);
      await layoutReport(p, `details ${size}`);
      if (w === 1440) await axe(p, `details fixture ${size}`);
      // 03 Read-only fixture: import pane explains why.
      await p.keyboard.press('Escape');
      // aria-disabled keeps it focusable; activating it explains why (Playwright needs force).
      await p.getByRole('button', { name: 'Import', exact: true }).click({ force: true });
      await shot(p, `03-import-unavailable-fixture-${size}`);
      await context.close();
    }
    // 04 Empty, 05 loading, 06 failed project, 07 large list, 08 absent (older host).
    for (const [name, query] of [['04-empty', 'empty'], ['05-project-loading', 'project-loading'], ['06-project-failed', 'project-error'], ['07-large', 'large']]) {
      const { context, p } = await page(vp);
      await open(p, `?fixture=${query}`);
      await shot(p, `${name}-fixture-${size}`);
      await context.close();
    }
    {
      const { context, p } = await page(vp, double({ omitAssets: true }));
      await open(p);
      await shot(p, `08-assets-absent-older-host-double-${size}`);
      await context.close();
    }
    // 09 Unsaved project: empty, import unavailable with the host's reason.
    {
      const { context, p } = await page(vp, double({ assets: [], assetImport: { available: false, reason: 'Open a saved project to import assets.' } }));
      await open(p);
      await shot(p, `09-unsaved-unavailable-double-${size}`);
      await context.close();
    }
    // 10 Import form with typed paths and validation, 11 busy, 12 failed, 13 conflict, 14 success.
    for (const mode of ['hold', 'fail', 'conflict', 'success']) {
      const { context, p } = await page(vp, double({ mode }));
      await open(p);
      await p.getByRole('button', { name: 'Import', exact: true }).click();
      await focusReport(p, `import opened by pointer ${mode} ${size}`);
      if (mode === 'hold') {
        await typePaths(p, ['models/props/lantern.glb', 'textures\\lantern_normal.png', 'textures/crate_normal.png']);
        await p.getByRole('button', { name: /^Import 3 files/ }).click();
        await shot(p, `10-import-validation-double-${size}`);
        await focusReport(p, `after invalid submit ${size}`);
        await p.getByRole('textbox', { name: 'Path 2' }).fill('textures/lantern_normal.png');
        await p.getByRole('radio', { name: /Normal map/ }).first().check();
        await shot(p, `10b-import-ready-double-${size}`);
        if (w === 1440) await axe(p, `import form ${size}`);
        await p.getByRole('button', { name: /^Import 3 files/ }).click();
        await shot(p, `11-import-busy-double-${size}`);
        // Navigation stays responsive while cooking.
        await p.getByRole('tab', { name: /^History/ }).click();
        await shot(p, `11b-busy-other-tab-double-${size}`);
        await p.getByRole('tab', { name: /^Assets/ }).click();
        if (w === 1440) await axe(p, `busy ${size}`);
      } else {
        await typePaths(p, ['models/props/lantern.glb', 'textures/lantern_normal.png']);
        await p.getByRole('radio', { name: /Normal map/ }).first().check();
        await p.getByRole('button', { name: /^Import 2 files/ }).click();
        await p.waitForTimeout(150);
        const n = { fail: '12-import-failed', conflict: '13-import-conflict', success: '14-import-success' }[mode];
        await shot(p, `${n}-double-${size}`);
        if (w === 1440) await axe(p, `${mode} ${size}`);
      }
      report.dispatched.push({ mode, size, commands: await p.evaluate(() => window.__EVIDENCE_DISPATCHED__) });
      await context.close();
    }
    // 15 Reimport a texture with a changed interpretation; 16 reimport busy.
    {
      const { context, p } = await page(vp, double({ mode: 'hold' }));
      await open(p);
      await p.getByRole('option', { name: /^Crate albedo/ }).click();
      await p.getByRole('radio', { name: /Normal map/ }).check();
      await shot(p, `15-reimport-changed-usage-double-${size}`);
      await p.getByRole('button', { name: /^Reimport as Normal map/ }).click();
      await shot(p, `16-reimport-busy-double-${size}`);
      report.dispatched.push({ mode: 'reimport-hold', size, commands: await p.evaluate(() => window.__EVIDENCE_DISPATCHED__) });
      await context.close();
    }
    // 17 Keyboard path: F6 to the left column (Assets view), Down, Enter opens details in the Inspector.
    {
      const { context, p } = await page(vp, double());
      await open(p);
      await p.mouse.click(w / 2, 120);
      for (let i = 0; i < 6; i += 1) {
        await p.keyboard.press('F6');
        if (await p.evaluate(() => document.activeElement?.closest('[data-region]')?.getAttribute('data-region') === 'hierarchy')) break;
      }
      await focusReport(p, `F6 to the asset list ${size}`);
      await p.keyboard.press('ArrowDown');
      await shot(p, `17-keyboard-list-focus-double-${size}`);
      await focusReport(p, `ArrowDown in list ${size}`);
      await p.keyboard.press('Enter');
      await focusReport(p, `Enter opens details ${size}`);
      await shot(p, `17b-keyboard-details-focus-double-${size}`);
      await p.keyboard.press('Escape');
      await focusReport(p, `Escape closes details ${size}`);
      await context.close();
    }
  }
} finally {
  writeFileSync(join(outDir, 'report.json'), JSON.stringify(report, null, 2));
  await browser.close();
  await new Promise((r) => server.httpServer.close(r));
}
console.log(`wrote ${report.shots.length} captures to ${outDir}; errors: ${report.errors.length}`);
