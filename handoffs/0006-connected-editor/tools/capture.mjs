#!/usr/bin/env node
/**
 * BROWSER FIXTURE captures for handoff 0006. Serves editor/ui/dist with `vite preview`,
 * drives local Chrome through playwright-core, and writes PNGs to
 * handoffs/0006-connected-editor/screenshots/<set>/browser-fixture/.
 * Every image is the built UI rendered by Chrome over SAMPLE FIXTURE data. None of them
 * is native evidence; native captures are taken separately from this worktree's app.
 *
 * Usage (repo root): node handoffs/0006-connected-editor/tools/capture.mjs <set>
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
const outDir = join(here, '..', 'screenshots', set, 'browser-fixture');
mkdirSync(outDir, { recursive: true });
const axePath = createRequire(join(uiRoot, 'package.json')).resolve('axe-core/axe.min.js');
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find(
  (p) => p && existsSync(p),
);
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');

const MAC_DOUBLE = `(() => {
  const snapshot = {
    connection: { status: 'ready', project: { id: '01J9ZF1XTR0000000000000000', name: 'Dock Prototype' } },
    hierarchy: { status: 'ready', value: { roots: [], nodes: {} } }, schemas: {}, entities: {}, diagnostics: [],
    history: { entries: [], applied: 0 }, console: [], provider: { status: 'not-connected', provider: 'openai' },
    agent: { status: 'unavailable', reason: 'Evidence test double.' },
    viewport: { status: 'not-attached', reason: 'Evidence test double (browser).' },
    window: { platform: 'macos', controls: 'native-overlay', maximized: false, fullscreen: false, focused: true, leadingInset: 78, trailingInset: 0 },
  };
  const refuse = () => Promise.resolve({ ok: false, error: { code: 'test-double', message: 'Evidence test double.' } });
  window.__INCANT_BRIDGE__ = { protocolVersion: 1, capabilities: ['window.drag', 'window.maximize'], label: 'Evidence test double (not engine)',
    isFixture: false, getSnapshot: () => snapshot, subscribe: () => () => {}, dispatch: refuse, request: () => Promise.resolve({ ok: true }) };
})();`;
const WIN_DOUBLE = MAC_DOUBLE.replace(`platform: 'macos', controls: 'native-overlay'`, `platform: 'windows', controls: 'custom'`)
  .replace('leadingInset: 78', 'leadingInset: 0')
  .replace(`['window.drag', 'window.maximize']`, `['window.drag', 'window.minimize', 'window.maximize', 'window.close']`);

const server = await preview({ root: uiRoot, logLevel: 'warn', preview: { port: 4186, strictPort: true, host: '127.0.0.1' } });
const origin = 'http://127.0.0.1:4186';
const browser = await chromium.launch({ executablePath, headless: true });
const report = { browser: `Chrome ${browser.version()} headless`, data: 'SAMPLE FIXTURE / evidence test doubles, not engine state', shots: [], axe: [], layout: [], errors: [] };

async function page(size, { init, scale = 2 } = {}) {
  const context = await browser.newContext({ viewport: size, deviceScaleFactor: scale, colorScheme: 'dark', bypassCSP: true });
  const p = await context.newPage();
  if (init) await p.addInitScript(init);
  p.on('pageerror', (e) => report.errors.push(String(e)));
  p.on('console', (m) => (m.type() === 'error' ? report.errors.push(m.text()) : null));
  return { context, p };
}
async function open(p, query) {
  await p.goto(`${origin}/${query}`, { waitUntil: 'networkidle' });
  await p.evaluate(() => document.fonts.ready);
  await p.waitForSelector('.app, .fatal');
  await p.waitForTimeout(250);
}
async function shot(p, name, clip) {
  await p.waitForTimeout(250);
  await p.screenshot({ path: join(outDir, `${name}.png`), clip });
  report.shots.push(`${name}.png`);
}
async function layout(p, label) {
  report.layout.push({
    label,
    ...(await p.evaluate(() => {
      const box = (s) => {
        const r = document.querySelector(s)?.getBoundingClientRect();
        return r ? [Math.round(r.x), Math.round(r.y), Math.round(r.width), Math.round(r.height)] : null;
      };
      const sizes = [...document.querySelectorAll('body *')]
        .filter((e) => [...e.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim()) && e.getClientRects().length)
        .map((e) => parseFloat(getComputedStyle(e).fontSize));
      return {
        viewportHost: box('[data-viewport-host]'),
        viewportRadii: (() => {
          const st = getComputedStyle(document.querySelector('[data-viewport-host]'));
          return [st.borderTopLeftRadius, st.borderTopRightRadius, st.borderBottomRightRadius, st.borderBottomLeftRadius];
        })(),
        hierarchy: box('[data-region="hierarchy"]'),
        inspector: box('[data-region="inspector"]'),
        dock: box('[data-region="dock"]'),
        agent: box('[data-region="agent"]'),
        minTextPx: Math.min(...sizes),
        pageOverflow: document.documentElement.scrollWidth > innerWidth || document.documentElement.scrollHeight > innerHeight,
      };
    })),
  });
}
async function axe(p, label) {
  await p.addScriptTag({ path: axePath });
  const r = await p.evaluate(async () => {
    // eslint-disable-next-line no-undef
    const out = await axe.run(document, { resultTypes: ['violations'] });
    return out.violations.map((v) => ({ id: v.id, nodes: v.nodes.map((n) => n.target.join(' ')).slice(0, 5) }));
  });
  report.axe.push({ label, violations: r });
}
const selectCrate = async (p) => {
  await p.getByRole('button', { name: /Half extents must be positive/ }).click();
  await p.keyboard.press('Shift+F6');
  await p.keyboard.press('Shift+F6');
};

try {
  for (const [w, h] of [[1440, 900], [1280, 800], [1000, 650]]) {
    const { context, p } = await page({ width: w, height: h });
    await open(p, '?fixture=sample');
    await shot(p, `01-sample-${w}x${h}`);
    await layout(p, `sample ${w}x${h}`);
    await selectCrate(p);
    await shot(p, `02-entity-selected-keyboard-focus-${w}x${h}`);
    await layout(p, `entity selected ${w}x${h}`);
    if (w === 1440) {
      await shot(p, `40-detail-top-left-1440`, { x: 0, y: 0, width: 720, height: 150 });
      await shot(p, `41-detail-top-right-1440`, { x: 860, y: 0, width: 580, height: 150 });
      await shot(p, `42-detail-agent-1440`, { x: 1040, y: 520, width: 400, height: 380 });
      await shot(p, `43-detail-dock-1440`, { x: 180, y: 580, width: 720, height: 320 });
      await axe(p, 'sample, entity selected, 1440x900');
      await p.getByRole('tab', { name: /Console/ }).click();
      await shot(p, `03-console-${w}x${h}`);
      await p.getByRole('tab', { name: /History/ }).click();
      await shot(p, `04-history-${w}x${h}`);
      await axe(p, 'history tab, 1440x900');
      await p.locator('[role="treeitem"][aria-selected="true"]').focus();
      await p.keyboard.press('F2');
      await shot(p, `05-rename-attempt-fixture-${w}x${h}`);
      await p.keyboard.press('Shift+Slash');
      await shot(p, `06-shortcuts-dialog-${w}x${h}`);
      await p.keyboard.press('Escape');
      await p.locator('.tree').evaluate((e) => (e.scrollTop = 99999));
      await shot(p, `07-hierarchy-scrolled-${w}x${h}`);
      await p.keyboard.press('Meta+Alt+Digit1');
      await p.keyboard.press('Meta+Alt+Digit1');
      // Pointer drag on the hierarchy separator, then keyboard focus on it.
      const sep = await p.getByRole('separator', { name: 'Resize hierarchy' }).boundingBox();
      await p.mouse.move(sep.x + 2, 400);
      await p.mouse.down();
      await p.mouse.move(sep.x + 82, 400, { steps: 8 });
      await p.mouse.up();
      await layout(p, 'after dragging the hierarchy separator 80 px right');
      await shot(p, `09-resized-by-drag-${w}x${h}`);
      await p.getByRole('separator', { name: 'Resize output panel' }).focus();
      await p.keyboard.press('ArrowUp');
      await shot(p, `09b-separator-keyboard-focus-${w}x${h}`);
      await p.locator('.titlebar .tool-button[aria-label="Undo"]').focus();
      await p.keyboard.press('Tab');
      await shot(p, `44-detail-titlebar-focus-1440`, { x: 300, y: 0, width: 840, height: 60 });
      await p.getByRole('tab', { name: /Problems/ }).focus();
      await shot(p, `45-detail-tab-focus-1440`, { x: 200, y: 560, width: 700, height: 120 });
      await p.keyboard.press('Meta+Alt+Digit1');
      await p.keyboard.press('Meta+Alt+Digit2');
      await shot(p, `08-panels-hidden-${w}x${h}`);
    }
    await context.close();
  }
  for (const provider of ['signed-out', 'signed-in-multi', 'browser', 'error-while-signed-in']) {
    const { context, p } = await page({ width: 1440, height: 900 });
    await open(p, `?fixture=sample&provider=${provider}`);
    if (provider === 'signed-out') await shot(p, `10-agent-signed-out-1440x900`);
    await p.locator('.provider-chip').click();
    await shot(p, `11-account-${provider}-1440x900`);
    if (provider === 'signed-in-multi') await axe(p, 'account dialog signed-in-multi');
    await context.close();
  }
  for (const variant of ['large', 'empty', 'loading', 'hierarchy-error', 'connection-error']) {
    const { context, p } = await page({ width: 1280, height: 800 });
    await open(p, `?fixture=${variant}`);
    await shot(p, `20-state-${variant}-1280x800`);
    await context.close();
  }
  {
    const { context, p } = await page({ width: 1280, height: 800 });
    await open(p, '');
    await shot(p, '21-no-engine-1280x800');
    await context.close();
  }
  for (const [name, init] of [['mac', MAC_DOUBLE], ['windows', WIN_DOUBLE]]) {
    const { context, p } = await page({ width: 1440, height: 900 }, { init });
    await open(p, '');
    await shot(p, `30-titlebar-${name}-double-1440`, { x: 0, y: 0, width: 1440, height: 48 });
    await shot(p, `31-window-${name}-double-1440x900`);
    await context.close();
  }
} finally {
  writeFileSync(join(outDir, 'report.json'), JSON.stringify(report, null, 2));
  await browser.close();
  await new Promise((r) => server.httpServer.close(r));
}
console.log(`wrote ${report.shots.length} captures to ${outDir}; errors: ${report.errors.length}`);
