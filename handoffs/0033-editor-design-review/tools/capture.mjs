#!/usr/bin/env node
/**
 * BROWSER captures of the editor UI for handoff 0033 (full design pass).
 *
 * Serves editor/ui/dist with `vite preview` on the first free port from 4233 (an
 * occupied port is never reused), drives local Chrome through the pinned
 * playwright-core, and writes PNGs plus report.json to a NEW ignored directory
 * artifacts/0033-editor-design-review/<set>/ (an existing directory is refused).
 * The server and browser are closed on exit.
 *
 * Data: the sample fixture (?fixture=…) and the evidence test double in
 * review-double.mjs (real schemas converted by the real compiled bridge function;
 * no capabilities; every command refused). Nothing here is native WebKit evidence.
 *
 * Measurements are DOM-generic so the same run works before and after the redesign.
 *
 * Usage (repo root, after `npm run build --workspace editor/ui`):
 *   node handoffs/0033-editor-design-review/tools/capture.mjs <set> [--only=key,key] [--sizes=normal,minimum]
 */
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import { preview } from 'vite';
import { reviewInitScript } from './review-double.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const uiRoot = join(repoRoot, 'editor/ui');
const args = process.argv.slice(2);
const set = args.find((a) => !a.startsWith('--'));
if (!set || !/^[a-z0-9-]+$/.test(set)) throw new Error('Usage: capture.mjs <set-name> [--only=a,b] [--sizes=normal,minimum]');
const flag = (name) => args.find((a) => a.startsWith(`--${name}=`))?.split('=')[1]?.split(',').filter(Boolean);
const only = flag('only');
const sizeFilter = flag('sizes');
const outDir = join(repoRoot, 'artifacts/0033-editor-design-review', set);
if (existsSync(outDir)) throw new Error(`${outDir} exists; choose a new set name.`);
const axePath = createRequire(join(uiRoot, 'package.json')).resolve('axe-core/axe.min.js');
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find((p) => p && existsSync(p));
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');
if (!existsSync(join(uiRoot, 'dist/index.html'))) throw new Error('Run the UI build first.');

const DOUBLE = reviewInitScript();

/** Window sizes; `inspector` moves the Inspector splitter with the keyboard. */
const SIZES = [
  { key: 'normal', width: 1440, height: 874 },
  { key: 'minimum', width: 1000, height: 650, inspector: 'Home' },
  { key: 'narrow', width: 1440, height: 874, inspector: 'Home' },
  { key: 'wide', width: 1920, height: 1080 },
];

const tree = (name) => async (page) => {
  await page.getByRole('treeitem', { name: new RegExp(`^${name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`) }).first().click();
};
const fixture = (variant = 'sample') => ({ url: `/?fixture=${variant}` });
const double = { url: '/', init: DOUBLE };

/**
 * Tab from the first focusable control in the Inspector body onto the first numeric
 * value whose accessible name matches `name`. Records the path of names.
 */
const focusField = (name) => async (page) => {
  const target = page.locator('[data-region="inspector"]').getByRole('textbox', { name, exact: true }).first();
  await target.scrollIntoViewIfNeeded();
  await target.focus();
  // A key press, so keyboard modality (and :focus-visible) applies, as when tabbing.
  await page.keyboard.press('Shift+Tab');
  await page.keyboard.press('Tab');
};

const STATES = [
  { key: 'no-selection', ...fixture(), sizes: ['normal', 'minimum', 'wide'] },
  { key: 'transform-camera', ...fixture(), steps: [tree('Pixel Camera')], sizes: ['normal', 'minimum', 'narrow', 'wide'] },
  { key: 'camera-focus', ...fixture(), steps: [tree('Pixel Camera'), focusField('Near clip')], sizes: ['normal', 'minimum'] },
  { key: 'light-fixture', ...fixture(), steps: [tree('Sun')], sizes: ['normal'] },
  { key: 'crate-error', ...fixture(), steps: [tree('Crate 03')], sizes: ['normal', 'minimum'] },
  { key: 'railing-compound', ...fixture(), steps: [tree('Broken Railing')], sizes: ['normal', 'minimum'] },
  { key: 'handcart-compound', ...fixture(), steps: [tree('Handcart')], sizes: ['normal', 'narrow'] },
  { key: 'script-unsupported', ...fixture(), steps: [tree('Grapple Controller')], sizes: ['normal', 'minimum'] },
  { key: 'long-name', ...fixture(), steps: [tree('Mooring Post')], sizes: ['normal', 'minimum'] },
  { key: 'asset-texture', ...fixture(), steps: [async (page) => { await page.getByRole('tab', { name: /^Assets/ }).click(); await page.getByRole('option', { name: /^Crate albedo/ }).click(); }], sizes: ['normal', 'minimum'] },
  { key: 'asset-long', ...fixture(), steps: [async (page) => { await page.getByRole('tab', { name: /^Assets/ }).click(); await page.getByRole('option', { name: /^weathered_pier/ }).click(); }], sizes: ['normal'] },
  { key: 'native-character', ...double, steps: [tree('Player Character')], sizes: ['normal', 'minimum', 'narrow', 'wide'] },
  { key: 'native-navmesh', ...double, steps: [tree('Harbor NavMesh')], sizes: ['normal', 'minimum'] },
  { key: 'native-grid', ...double, steps: [tree('Courtyard Grid')], sizes: ['normal', 'minimum'] },
  // 65,536 grid costs. axe is skipped: it is the page's responsiveness that is measured here.
  { key: 'native-grid-large', ...double, steps: [tree('District Grid')], sizes: ['normal'], axe: false },
  { key: 'native-sun', ...double, steps: [tree('Sun')], sizes: ['normal', 'minimum'] },
  { key: 'native-spot', ...double, steps: [tree('Gate Spot')], sizes: ['normal'] },
  { key: 'native-env', ...double, steps: [tree('Harbor Sky')], sizes: ['normal'] },
  { key: 'native-mesh', ...double, steps: [tree('Crate Stack')], sizes: ['normal', 'minimum', 'narrow'] },
  { key: 'native-rubble', ...double, steps: [tree('Rubble Pile')], sizes: ['normal'] },
  { key: 'native-audio', ...double, steps: [tree('Audio Emitter')], sizes: ['normal'] },
  { key: 'native-ortho', ...double, steps: [tree('Map Camera')], sizes: ['normal'] },
  { key: 'native-broken-camera', ...double, steps: [tree('Broken Camera')], sizes: ['normal', 'minimum'] },
  { key: 'native-malformed', ...double, steps: [tree('Malformed Props')], sizes: ['normal', 'minimum'] },
  { key: 'native-long-name', ...double, steps: [tree('Mooring Post')], sizes: ['normal', 'minimum'] },
  { key: 'agent-signed-in', url: '/?fixture=sample&provider=signed-in', sizes: ['normal'] },
  { key: 'agent-signing-in', url: '/?fixture=sample&provider=browser', sizes: ['normal'] },
  { key: 'agent-collapsed', ...fixture(), steps: [tree('Crate 03'), async (page) => page.locator('[data-region="agent"]').getByRole('button', { name: /^Agent/ }).click()], sizes: ['normal'] },
  { key: 'import-form', ...fixture(), steps: [async (page) => { await page.getByRole('tab', { name: /^Assets/ }).click(); await page.getByRole('button', { name: 'Import' }).click({ force: true }); }], sizes: ['normal'] },
  { key: 'tree-focus', ...fixture(), steps: [async (page) => { await page.keyboard.press('F6'); await page.keyboard.press('ArrowDown'); await page.keyboard.press('ArrowDown'); }], sizes: ['normal'] },
  { key: 'stress-tree', ...fixture('stress'), steps: [async (page) => { await page.getByLabel('Filter hierarchy by name').fill('Entity 0'); await page.waitForTimeout(400); }], sizes: ['normal'] },
  { key: 'stress-assets', ...fixture('stress'), steps: [async (page) => page.getByRole('tab', { name: /^Assets/ }).click()], sizes: ['normal'] },
  { key: 'dock-history', ...fixture(), steps: [async (page) => page.getByRole('tab', { name: /^History/ }).click()], sizes: ['normal'] },
  { key: 'dock-console', ...fixture(), steps: [async (page) => page.getByRole('tab', { name: /^Console/ }).click()], sizes: ['normal'] },
  { key: 'account-dialog', ...fixture(), steps: [async (page) => page.getByRole('button', { name: /^ChatGPT/ }).click()], sizes: ['normal'] },
  { key: 'shortcuts-dialog', ...fixture(), steps: [async (page) => page.getByRole('button', { name: 'Keyboard shortcuts' }).click()], sizes: ['normal'] },
  { key: 'project-loading', ...fixture('project-loading'), sizes: ['normal'] },
  { key: 'project-error', ...fixture('project-error'), sizes: ['normal'] },
  { key: 'empty-project', ...fixture('empty'), sizes: ['normal'] },
  { key: 'no-engine', url: '/', sizes: ['normal'] },
];

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

/** Generic, design-independent measurements of the Inspector column and the page. */
function measure() {
  const round = (n) => Math.round(n * 10) / 10;
  const rect = (el) => {
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return { x: round(r.x), y: round(r.y), width: round(r.width), height: round(r.height) };
  };
  const visible = (el) => {
    const r = el.getBoundingClientRect();
    if (r.width === 0 || r.height === 0) return false;
    const style = getComputedStyle(el);
    return style.visibility !== 'hidden' && style.display !== 'none' && !el.closest('.visually-hidden');
  };
  const parse = (c) => (c.match(/[\d.]+/g) ?? []).slice(0, 3).map(Number);
  const lum = ([r, g, b]) => [r, g, b].map((v) => v / 255).map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4)).reduce((s, v, i) => s + v * [0.2126, 0.7152, 0.0722][i], 0);
  const bgOf = (el) => {
    for (let node = el; node; node = node.parentElement) {
      const bg = getComputedStyle(node).backgroundColor;
      if (bg && bg !== 'transparent' && !/rgba\(\d+, \d+, \d+, 0\)/.test(bg)) return bg;
    }
    return 'rgb(11, 12, 15)';
  };
  const contrast = (el) => {
    const a = lum(parse(getComputedStyle(el).color));
    const b = lum(parse(bgOf(el)));
    return Math.round(((Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05)) * 100) / 100;
  };
  const path = (el) => {
    const parts = [];
    for (let node = el; node && node !== document.body && parts.length < 4; node = node.parentElement) {
      parts.unshift(`${node.tagName.toLowerCase()}${node.classList.length ? `.${[...node.classList].slice(0, 2).join('.')}` : ''}`);
    }
    return parts.join(' > ');
  };
  const inspector = document.querySelector('[data-region="inspector"]');
  const agent = document.querySelector('[data-region="agent"]');
  const out = { inspector: rect(inspector), agent: rect(agent), viewport: rect(document.querySelector('[data-viewport-host]')) };
  out.pageHorizontalOverflow = document.documentElement.scrollWidth - document.documentElement.clientWidth;
  if (!inspector) return out;
  const box = inspector.getBoundingClientRect();
  const controls = [...inspector.querySelectorAll('input, textarea, [role="checkbox"], [role="listbox"], [role="group"][tabindex="0"]')].filter(visible);
  const first = controls[0];
  out.firstValueTop = first ? round(first.getBoundingClientRect().top - box.top) : null;
  // Height of everything fixed above the scrolling content (panel header, any pinned identity).
  const scrollers = [...inspector.querySelectorAll('*')].filter((el) => {
    const s = getComputedStyle(el);
    return (s.overflowY === 'auto' || s.overflowY === 'scroll') && el.clientHeight > 40;
  });
  const main = scrollers[0];
  out.fixedChrome = main ? round(main.getBoundingClientRect().top - box.top) : null;
  out.horizontalOverflow = scrollers.map((el) => ({ el: path(el), overflow: el.scrollWidth - el.clientWidth })).filter((o) => o.overflow > 0);
  out.clippedValues = [...inspector.querySelectorAll('input')].filter(visible).filter((el) => el.scrollWidth > el.clientWidth + 1)
    .map((el) => ({ value: el.value, name: el.getAttribute('aria-label') ?? document.getElementById(el.getAttribute('aria-labelledby') ?? '')?.textContent ?? null, width: el.clientWidth, needed: el.scrollWidth }));
  out.clippedText = [...inspector.querySelectorAll('*')].filter((el) => visible(el) && el.children.length === 0 && el.textContent.trim() && !(el instanceof HTMLInputElement))
    .filter((el) => {
      const s = getComputedStyle(el);
      const ellipsis = s.textOverflow === 'ellipsis' && el.scrollWidth > el.clientWidth + 1;
      const clamped = s.webkitLineClamp !== 'none' && s.webkitLineClamp !== '' && el.scrollHeight > el.clientHeight + 1;
      return ellipsis || clamped;
    })
    .map((el) => ({ text: el.textContent.trim().slice(0, 80), el: path(el) }));
  out.outOfBounds = [...inspector.querySelectorAll('*')].filter(visible).filter((el) => {
    const r = el.getBoundingClientRect();
    return r.right > box.right + 0.5 || r.left < box.left - 0.5;
  }).slice(0, 20).map((el) => ({ el: path(el), rect: rect(el) }));
  const headers = [...inspector.querySelectorAll('section h3, section [aria-expanded]')].filter(visible);
  out.componentHeaders = headers.slice(0, 12).map((el) => ({ text: el.textContent.trim().slice(0, 40), height: round(el.getBoundingClientRect().height) }));
  const rows = [...inspector.querySelectorAll('.field')].filter(visible);
  out.rowHeights = rows.slice(0, 40).map((el) => round(el.getBoundingClientRect().height));
  const label = inspector.querySelector('.field__label');
  const value = inspector.querySelector('.field input');
  out.contrast = {
    label: label ? contrast(label) : null,
    value: value ? contrast(value) : null,
  };
  const active = document.activeElement;
  if (active && inspector.contains(active)) {
    const ringed = [active, active.parentElement, active.parentElement?.parentElement].find((el) => el && getComputedStyle(el).outlineStyle !== 'none');
    out.focus = { name: active.getAttribute('aria-label') ?? document.getElementById(active.getAttribute('aria-labelledby') ?? '')?.textContent ?? null,
      value: active.value ?? null, outline: ringed ? `${getComputedStyle(ringed).outlineStyle} ${getComputedStyle(ringed).outlineWidth} ${getComputedStyle(ringed).outlineColor}` : null, ring: rect(ringed) };
  }
  return out;
}

/** One capture. Returns a report entry; never throws for page-side failures. */
async function captureOne(browser, origin, state, size) {
  const context = await browser.newContext({ viewport: { width: size.width, height: size.height }, deviceScaleFactor: 2, colorScheme: 'dark', bypassCSP: true });
  const file = `${state.key}-${size.key}-${size.width}x${size.height}.png`;
  const entry = { file, crops: {}, state: state.key, size, stepError: null, timedOut: false, errors: [], remoteRequests: [] };
  const page = await context.newPage();
  page.on('pageerror', (e) => entry.errors.push(String(e)));
  page.on('console', (m) => m.type() === 'error' && entry.errors.push(m.text()));
  page.on('request', (r) => !r.url().startsWith(origin) && !r.url().startsWith('data:') && entry.remoteRequests.push(r.url()));
  const work = (async () => {
    if (state.init) await page.addInitScript(state.init);
    await page.goto(`${origin}${state.url}`, { waitUntil: 'networkidle' });
    if (size.inspector) {
      const splitter = page.getByRole('separator', { name: /^Resize inspector/ });
      if (await splitter.count()) {
        await splitter.focus();
        await page.keyboard.press(size.inspector);
        await page.locator('body').click({ position: { x: 1, y: size.height - 2 } }).catch(() => undefined);
      }
    }
    try {
      for (const step of state.steps ?? []) await step(page);
    } catch (error) {
      entry.stepError = String(error).slice(0, 300);
    }
    await page.waitForTimeout(150);
    const started = Date.now();
    entry.measure = await page.evaluate(measure);
    entry.measureMs = Date.now() - started;
    if (state.axe !== false) {
      await page.addScriptTag({ path: axePath });
      entry.axe = await page.evaluate(async () => {
        // eslint-disable-next-line no-undef
        const result = await axe.run(document, { resultTypes: ['violations'] });
        return result.violations.map((v) => ({ id: v.id, impact: v.impact, nodes: v.nodes.length, targets: v.nodes.slice(0, 3).map((n) => n.target.join(' ')) }));
      });
    }
    await page.screenshot({ path: join(outDir, file) });
    for (const region of ['inspector', 'agent']) {
      const box = await page.locator(`[data-region="${region}"]`).boundingBox().catch(() => null);
      if (!box) continue;
      entry.crops[region] = file.replace('.png', `-${region}.png`);
      await page.screenshot({ path: join(outDir, entry.crops[region]), clip: box });
    }
    // The whole right column, Inspector plus Agent, as one crop.
    const side = await page.locator('.column--side').boundingBox().catch(() => null);
    if (side) {
      entry.crops.side = file.replace('.png', '-side.png');
      await page.screenshot({ path: join(outDir, entry.crops.side), clip: side });
    }
  })();
  const limit = new Promise((done) => setTimeout(() => done('timeout'), 60_000));
  if ((await Promise.race([work.then(() => 'done', (error) => { entry.stepError ??= String(error).slice(0, 300); return 'done'; }), limit])) === 'timeout') {
    entry.timedOut = true;
    work.catch(() => undefined);
  }
  // A page that never finishes rendering must not hold the run: close with a bound too.
  await Promise.race([context.close().catch(() => undefined), new Promise((done) => setTimeout(done, 10_000))]);
  return entry;
}

mkdirSync(outDir, { recursive: true });
const port = await freePort(4233);
const server = await preview({ root: uiRoot, preview: { port, strictPort: true, host: '127.0.0.1' }, logLevel: 'error' });
const origin = `http://127.0.0.1:${port}`;
const browser = await chromium.launch({ executablePath, headless: true });
const report = { set, generated: new Date().toISOString(), browser: browser.version(), port, captures: [] };
const save = () => writeFileSync(join(outDir, 'report.json'), `${JSON.stringify(report, null, 2)}\n`);
try {
  for (const state of STATES) {
    if (only && !only.includes(state.key)) continue;
    for (const size of SIZES) {
      if (!state.sizes.includes(size.key) || (sizeFilter && !sizeFilter.includes(size.key))) continue;
      const entry = await captureOne(browser, origin, state, size);
      report.captures.push(entry);
      save();
      const axeIds = (entry.axe ?? []).map((v) => v.id);
      console.log(`${entry.file}${entry.timedOut ? '  TIMED OUT after 60 s' : ''}${entry.stepError ? `  STEP ERROR ${entry.stepError}` : ''}${axeIds.length ? `  axe:${axeIds.join(',')}` : ''}`);
    }
  }
} finally {
  await browser.close();
  await new Promise((done) => server.httpServer.close(done));
}
save();
console.log(`Wrote ${report.captures.length} captures to ${outDir}`);
