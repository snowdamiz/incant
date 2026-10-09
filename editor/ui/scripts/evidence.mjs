#!/usr/bin/env node
/**
 * Real-browser evidence for handoff 0001. Serves the production build (dist/)
 * with `vite preview`, drives a locally installed Chrome through playwright-core,
 * and writes screenshots plus measurements to handoffs/0001-editor-foundation/.
 *
 * Nothing here is synthesized: every PNG is a capture of the built UI as Chrome
 * rendered it. Native viewport pixels are NOT covered (the native window is not
 * part of this run); see NATIVE_VIEWPORT.md.
 *
 * Usage: npm run build --workspace editor/ui && npm run evidence --workspace editor/ui
 * Env:   CHROME_PATH overrides the browser executable.
 */
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import { preview } from 'vite';

const here = dirname(fileURLToPath(import.meta.url));
const uiRoot = resolve(here, '..');
const repoRoot = resolve(uiRoot, '../..');
const outRoot = join(repoRoot, 'handoffs/0001-editor-foundation');
// Revision output goes to its own directory; the first design's captures in screenshots/
// are kept untouched as the "before" evidence. Override with EVIDENCE_REVISION.
const revision = process.env.EVIDENCE_REVISION ?? 'revision-1';
const shotsDir = join(outRoot, 'screenshots', revision);
const require = createRequire(import.meta.url);
const axePath = require.resolve('axe-core/axe.min.js');

const CHROME_CANDIDATES = [
  process.env.CHROME_PATH,
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  '/usr/bin/google-chrome',
  '/usr/bin/chromium',
].filter(Boolean);
const executablePath = CHROME_CANDIDATES.find((path) => existsSync(path));
if (!executablePath) throw new Error('No Chrome found. Set CHROME_PATH.');
if (!existsSync(join(uiRoot, 'dist/index.html'))) throw new Error('Run the build first (dist/ is missing).');

mkdirSync(shotsDir, { recursive: true });

const SIZES = {
  small: { width: 1280, height: 800 },
  large: { width: 1920, height: 1080 },
};

/**
 * EVIDENCE TEST DOUBLE, injected only for the transparency measurement. It
 * reports an attached viewport so the shell switches to its transparent mode;
 * it holds no document and rejects every command.
 */
const ATTACHED_DOUBLE = `
  (() => {
    const empty = { roots: [], nodes: {} };
    const snapshot = {
      connection: { status: 'ready', project: { id: '01J9ZF1XTR0000000000000000', name: 'Transparency check' } },
      hierarchy: { status: 'ready', value: empty },
      schemas: {}, entities: {}, diagnostics: [],
      history: { entries: [], applied: 0 }, console: [],
      provider: { status: 'not-connected', provider: 'openai' },
      agent: { status: 'unavailable', reason: 'Evidence test double.' },
      viewport: { status: 'attached', surface: 'evidence-test-double' },
    };
    const refused = Promise.resolve({ ok: false, error: { code: 'test-double', message: 'Evidence test double.' } });
    window.__INCANT_BRIDGE__ = {
      protocolVersion: 1, capabilities: [], label: 'Evidence test double (not engine)', isFixture: false,
      getSnapshot: () => snapshot, subscribe: () => () => {}, dispatch: () => refused, request: () => refused,
    };
  })();
`;

/**
 * EVIDENCE TEST DOUBLES for titlebar chrome. They describe a host window (macOS overlay
 * traffic lights, or Windows custom caption buttons) so the titlebar layout can be
 * captured; they record requests and change nothing. The OS-drawn traffic lights are not
 * in a browser capture: only the reserved inset is.
 */
const chromeDouble = (window, capabilities) => `
  (() => {
    const snapshot = {
      connection: { status: 'ready', project: { id: '01J9ZF1XTR0000000000000000', name: 'Dock Prototype' } },
      hierarchy: { status: 'ready', value: { roots: [], nodes: {} } },
      schemas: {}, entities: {}, diagnostics: [],
      history: { entries: [], applied: 0 }, console: [],
      provider: { status: 'not-connected', provider: 'openai' },
      agent: { status: 'unavailable', reason: 'Evidence test double.' },
      viewport: { status: 'not-attached', reason: 'Evidence test double.' },
      window: ${JSON.stringify(window)},
    };
    window.__evidenceRequests = [];
    window.__INCANT_BRIDGE__ = {
      protocolVersion: 1, capabilities: ${JSON.stringify(capabilities)}, label: 'Evidence test double (not engine)', isFixture: false,
      getSnapshot: () => snapshot, subscribe: () => () => {},
      dispatch: () => Promise.resolve({ ok: false, error: { code: 'test-double', message: 'Evidence test double.' } }),
      request: (r) => { window.__evidenceRequests.push(r.type); return Promise.resolve({ ok: true }); },
    };
  })();
`;
const MAC_DOUBLE = chromeDouble(
  { platform: 'macos', controls: 'native-overlay', maximized: false, fullscreen: false, focused: true, leadingInset: 78, trailingInset: 0 },
  ['window.drag', 'window.maximize'],
);
const WINDOWS_DOUBLE = chromeDouble(
  { platform: 'windows', controls: 'custom', maximized: false, fullscreen: false, focused: true, leadingInset: 0, trailingInset: 0 },
  ['window.drag', 'window.minimize', 'window.maximize', 'window.close'],
);

const server = await preview({
  root: uiRoot,
  logLevel: 'warn',
  preview: { port: 4174, strictPort: true, host: '127.0.0.1' },
});
const origin = 'http://127.0.0.1:4174';
const browser = await chromium.launch({ executablePath, headless: true });
const version = browser.version();

const evidence = {
  generatedAt: new Date().toISOString(),
  browser: `Chrome ${version} (headless, playwright-core)`,
  executablePath,
  build: 'vite build output served by vite preview',
  screenshots: [],
  axe: [],
  layout: [],
  network: { offlineOnly: true, external: [] },
  console: [],
  transparency: null,
};

const rel = (path) => relative(repoRoot, path);

async function newPage(size, { bypassCSP = false, init } = {}) {
  const context = await browser.newContext({ viewport: size, deviceScaleFactor: 1, bypassCSP, colorScheme: 'dark' });
  const page = await context.newPage();
  if (init) await page.addInitScript(init);
  page.on('request', (request) => {
    const url = request.url();
    if (!url.startsWith(origin) && !url.startsWith('data:') && url !== 'about:blank') {
      evidence.network.offlineOnly = false;
      evidence.network.external.push(url);
    }
  });
  page.on('console', (message) => {
    if (message.type() === 'error' || message.type() === 'warning') {
      evidence.console.push({ url: page.url(), type: message.type(), text: message.text() });
    }
  });
  page.on('pageerror', (error) => evidence.console.push({ url: page.url(), type: 'pageerror', text: String(error) }));
  return { context, page };
}

async function open(page, query) {
  await page.goto(`${origin}/${query}`, { waitUntil: 'networkidle' });
  await page.evaluate(() => document.fonts.ready);
  await page.waitForSelector('.app, .fatal');
}

// Let 90–160 ms hover/selection transitions settle so captures show resting states.
const settle = (page) => page.waitForTimeout(300);

async function shot(page, name, description) {
  await settle(page);
  const path = join(shotsDir, `${name}.png`);
  await page.screenshot({ path });
  const size = page.viewportSize();
  evidence.screenshots.push({ file: rel(path), size: `${size.width}x${size.height}`, description });
}

async function measureLayout(page, label) {
  const result = await page.evaluate(() => {
    const box = (selector) => {
      const element = document.querySelector(selector);
      if (!element) return null;
      const r = element.getBoundingClientRect();
      return { x: Math.round(r.x), y: Math.round(r.y), width: Math.round(r.width), height: Math.round(r.height) };
    };
    const regions = Object.fromEntries(
      ['hierarchy', 'viewport', 'dock', 'inspector', 'agent'].map((name) => [name, box(`[data-region="${name}"]`)]),
    );
    const rows = [...document.querySelectorAll('.tree-row')];
    const fontSizes = [...document.querySelectorAll('body *')]
      .filter((element) => element.childNodes.length && [...element.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim()))
      .filter((element) => element.getClientRects().length > 0)
      .map((element) => parseFloat(getComputedStyle(element).fontSize));
    const truncatedRows = rows.filter((row) => {
      const name = row.querySelector('.tree-row__name');
      return name && name.scrollWidth > name.clientWidth;
    }).length;
    const pageOverflow = document.documentElement.scrollWidth > window.innerWidth || document.documentElement.scrollHeight > window.innerHeight;
    return {
      regions,
      treeRowHeight: rows[0] ? Math.round(rows[0].getBoundingClientRect().height) : null,
      visibleTreeRows: rows.filter((row) => {
        const r = row.getBoundingClientRect();
        const tree = document.querySelector('.tree')?.getBoundingClientRect();
        return tree && r.top >= tree.top && r.bottom <= tree.bottom;
      }).length,
      truncatedRowsWithTitleTooltip: truncatedRows,
      minVisibleTextPx: fontSizes.length ? Math.min(...fontSizes) : null,
      pageOverflow,
      fonts: [...document.fonts].filter((f) => f.status === 'loaded').map((f) => f.family),
    };
  });
  evidence.layout.push({ label, size: page.viewportSize(), ...result });
}

async function runAxe(query, size, label, prepare) {
  const { context, page } = await newPage(size, { bypassCSP: true });
  await open(page, query);
  if (prepare) await prepare(page);
  await page.addScriptTag({ path: axePath });
  const result = await page.evaluate(async () => {
    // eslint-disable-next-line no-undef
    const r = await axe.run(document, { resultTypes: ['violations', 'incomplete'] });
    return {
      violations: r.violations.map((v) => ({ id: v.id, impact: v.impact, nodes: v.nodes.map((n) => n.target.join(' ')) })),
      incomplete: r.incomplete.map((v) => ({ id: v.id, nodes: v.nodes.length })),
    };
  });
  evidence.axe.push({ label, size: `${size.width}x${size.height}`, ...result });
  await context.close();
}

async function titlebarShot(page, name, description) {
  await settle(page);
  const path = join(shotsDir, `${name}.png`);
  await page.screenshot({ path, clip: { x: 0, y: 0, width: page.viewportSize().width, height: 40 } });
  evidence.screenshots.push({ file: rel(path), size: `${page.viewportSize().width}x40`, description });
}

const selectCrate = async (page) => {
  await page.getByRole('button', { name: /Half extents must be positive/ }).click();
  // Return to the tree by keyboard (dock -> viewport -> hierarchy) so the ring is a real :focus-visible state.
  await page.keyboard.press('Shift+F6');
  await page.keyboard.press('Shift+F6');
};

try {
  // Before: there was no editor UI in the repository; the webview would be blank.
  {
    const { context, page } = await newPage(SIZES.small);
    await page.goto('about:blank');
    await shot(page, '00-before-blank-1280x800', 'Before: no editor UI existed in the repository (blank page).');
    await context.close();
  }

  for (const [key, size] of Object.entries(SIZES)) {
    const tag = `${size.width}x${size.height}`;
    const { context, page } = await newPage(size);

    await open(page, '');
    await shot(page, `01-no-engine-${tag}`, 'After: no bridge injected. Explicit no-engine state; no fabricated data.');
    await measureLayout(page, `no-engine ${tag}`);

    await open(page, '?fixture=sample');
    await shot(page, `02-sample-initial-${tag}`, 'After: labeled sample fixture, nothing selected.');
    await selectCrate(page);
    await shot(page, `03-sample-selected-error-${tag}`, 'After: Crate 03 selected from Problems; field error in inspector; keyboard focus ring on tree row.');
    await measureLayout(page, `sample (Crate 03 selected) ${tag}`);

    if (key === 'small') {
      await page.getByRole('button', { name: /Script asset not found/ }).click();
      await shot(page, `04-inspector-unsupported-${tag}`, 'Inspector: type mismatch, unsupported field type and unregistered component shown explicitly.');

      await page.getByRole('tab', { name: /Console/ }).click();
      await shot(page, `05-console-${tag}`, 'Console tab with level filters.');
      await page.getByRole('tab', { name: /History/ }).click();
      await shot(page, `06-history-${tag}`, 'History tab with provenance (user, agent, script, import) and an undone entry.');

      await page.locator('.tree-row').first().focus();
      await page.keyboard.press('F2');
      await page.waitForTimeout(100);
      await shot(page, `07-readonly-explained-${tag}`, 'F2 on the read-only fixture: status bar explains why rename is unavailable.');

      await page.keyboard.press('Shift+Slash');
      await page.waitForSelector('[role="dialog"]');
      await shot(page, `08-shortcuts-dialog-${tag}`, 'Keyboard shortcuts dialog (focus on Close).');
      await page.keyboard.press('Escape');

      for (const variant of ['empty', 'loading', 'hierarchy-error', 'connection-error', 'large']) {
        await open(page, `?fixture=${variant}`);
        await page.waitForTimeout(50);
        await shot(page, `10-state-${variant}-${tag}`, `State: ${variant}.`);
        if (variant === 'large') await measureLayout(page, `large (2,041 entities) ${tag}`);
      }
      await open(page, '?fixture=does-not-exist');
      await shot(page, `11-state-bad-fixture-${tag}`, 'Unknown fixture name is reported, not guessed.');
    }
    await context.close();
  }

  // Titlebar chrome variants.
  {
    const { context, page } = await newPage(SIZES.small);
    await open(page, '?fixture=sample');
    await titlebarShot(page, '20-titlebar-browser', 'Titlebar in a plain browser (no host): no insets, no window controls.');
    await page.getByRole('button', { name: 'Hierarchy panel' }).click();
    await page.getByRole('button', { name: 'Inspector and agent panel' }).click();
    await shot(page, '21-panels-collapsed-1280x800', 'Hierarchy and inspector hidden from the titlebar layout toggles; viewport and output fill the window.');
    await context.close();
  }
  for (const [label, init] of [['macos', MAC_DOUBLE], ['windows', WINDOWS_DOUBLE]]) {
    const { context, page } = await newPage(SIZES.small, { init });
    await open(page, '');
    await titlebarShot(page, `22-titlebar-${label}`, `Titlebar with an evidence test double reporting ${label} chrome.${label === 'macos' ? ' The 78 px leading inset is where the OS draws traffic lights (not visible in a browser capture).' : ' Custom caption buttons are shown because the double advertises window.* capabilities.'}`);
    if (label === 'windows') {
      await page.getByRole('button', { name: 'Close' }).hover();
      await titlebarShot(page, '23-titlebar-windows-close-hover', 'Windows caption buttons, Close hovered.');
      await page.getByRole('button', { name: 'Minimize' }).click();
      await page.mouse.move(640, 300);
      await page.mouse.dblclick(400, 20);
      evidence.windowRequests = await page.evaluate(() => window.__evidenceRequests);
    }
    await context.close();
  }

  // Native transparency: with an attached viewport, the viewport hole must be transparent and chrome opaque.
  {
    const { context, page } = await newPage(SIZES.small, { init: ATTACHED_DOUBLE });
    await open(page, '');
    const path = join(shotsDir, '12-native-viewport-transparent-1280x800.png');
    await page.screenshot({ path, omitBackground: true });
    evidence.screenshots.push({
      file: rel(path),
      size: '1280x800',
      description: 'Evidence test double reports an attached viewport; PNG captured with omitBackground, so the viewport hole is transparent (alpha 0) where the native surface will show. Not a native render.',
    });
    evidence.transparency = await page.evaluate(() => {
      const host = document.querySelector('[data-viewport-host]').getBoundingClientRect();
      const bg = (selector) => getComputedStyle(document.querySelector(selector)).backgroundColor;
      return {
        viewportHost: { x: Math.round(host.x), y: Math.round(host.y), width: Math.round(host.width), height: Math.round(host.height) },
        htmlBackground: getComputedStyle(document.documentElement).backgroundColor,
        bodyBackground: getComputedStyle(document.body).backgroundColor,
        viewportHostBackground: bg('[data-viewport-host]'),
        hierarchyBackground: bg('[data-region="hierarchy"]'),
        viewportHeaderBackground: bg('[data-region="viewport"] .panel__header'),
      };
    });
    // Sample actual PNG alpha via the browser's own decoder.
    const { readFileSync } = await import('node:fs');
    const png = readFileSync(path).toString('base64');
    const t = evidence.transparency.viewportHost;
    evidence.transparency.pixelAlpha = await page.evaluate(
      async ({ png, points }) => {
        const image = new Image();
        image.src = `data:image/png;base64,${png}`;
        await image.decode();
        const canvas = document.createElement('canvas');
        canvas.width = image.width;
        canvas.height = image.height;
        const ctx = canvas.getContext('2d', { willReadFrequently: true });
        ctx.drawImage(image, 0, 0);
        return Object.fromEntries(points.map(([name, x, y]) => [name, ctx.getImageData(x, y, 1, 1).data[3]]));
      },
      {
        png,
        points: [
          ['viewportCenter', t.x + Math.round(t.width / 2), t.y + Math.round(t.height / 2)],
          ['hierarchyPanel', 40, 300],
          ['topBar', 400, 20],
          ['statusBar', 400, 790],
        ],
      },
    );
    await context.close();
  }

  // axe-core in the real browser, color-contrast included.
  await runAxe('', SIZES.small, 'no-engine');
  await runAxe('?fixture=sample', SIZES.small, 'sample, Crate 03 selected', selectCrate);
  await runAxe('?fixture=sample', SIZES.large, 'sample, Crate 03 selected', selectCrate);
  await runAxe('?fixture=sample', SIZES.small, 'sample, History tab', (page) => page.getByRole('tab', { name: /History/ }).click());
  await runAxe('?fixture=sample', SIZES.small, 'sample, Console tab', (page) => page.getByRole('tab', { name: /Console/ }).click());
  await runAxe('?fixture=sample', SIZES.small, 'sample, Grapple Controller', (page) =>
    page.getByRole('button', { name: /Script asset not found/ }).click(),
  );
  for (const variant of ['empty', 'loading', 'hierarchy-error', 'connection-error']) {
    await runAxe(`?fixture=${variant}`, SIZES.small, variant);
  }
} finally {
  await browser.close();
  await new Promise((done) => server.httpServer.close(done));
}

const violations = evidence.axe.reduce((sum, run) => sum + run.violations.length, 0);
writeFileSync(join(shotsDir, 'evidence.json'), `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`Screenshots: ${evidence.screenshots.length} in ${rel(shotsDir)}`);
console.log(`axe runs: ${evidence.axe.length}, violations: ${violations}`);
console.log(`Offline only: ${evidence.network.offlineOnly}; console errors/warnings: ${evidence.console.length}`);
console.log(`Transparency: ${JSON.stringify(evidence.transparency?.pixelAlpha)}`);
if (violations > 0 || !evidence.network.offlineOnly) process.exitCode = 1;
