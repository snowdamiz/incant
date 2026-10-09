#!/usr/bin/env node
/**
 * BROWSER FIXTURE captures for handoff 0009 (adapted from 0008). Serves editor/ui/dist with
 * `vite preview` on port 4189, drives local Chrome through playwright-core, and writes PNGs to
 * handoffs/0009-loading-and-focus/screenshots/<set>/.
 *
 * Every image is the built UI rendered by headless Chrome over SYNTHETIC data. The project
 * loading and failed states use evidence test doubles that copy the shape of
 * editor/bridge/native.ts snapshotFromEngine() for `loading` and `error`; they are not engine
 * output. None of these images is native WebKit evidence.
 *
 * "webkit-sim" captures rewrite every `:focus-visible` selector in the loaded stylesheets to a
 * selector that never matches. That reproduces the observed native defect (WebKit draws no
 * :focus-visible ring after script focus) inside Chrome. It is a simulation, not WebKit.
 *
 * Usage (repo root, after `npm run build --workspace editor/ui`):
 *   node handoffs/0009-loading-and-focus/tools/capture-editor.mjs <set>
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

/** Evidence test double: the native adapter's loading/error snapshot plus independent account metadata. */
const projectDouble = (state) => `(() => {
  const error = ${state === 'failed' ? `{ code: 'project.timeout', message: "Project loading timed out. Check the file's availability and folder permissions, then reopen the project." }` : 'undefined'};
  const snapshot = {
    connection: error ? { status: 'error', error } : { status: 'connecting' },
    hierarchy: error ? { status: 'error', error } : { status: 'loading' },
    schemas: {}, entities: {}, history: { entries: [], applied: 0 }, console: [],
    diagnostics: error ? [{ id: 'project-load', severity: 'error', message: error.message, entity: null, component: null, path: null }] : [],
    provider: { status: 'connected', provider: 'openai', method: 'oauth', accountLabel: 'Sample Account',
      accounts: [{ id: 'a1', label: 'Sample Account' }, { id: 'a2', label: 'Second Sample' }], activeAccount: 'a1' },
    agent: { status: 'unavailable', reason: error ? error.message : 'Loading project.' },
    viewport: error ? { status: 'error', error } : { status: 'not-attached', reason: 'Loading project.' },
  };
  const refuse = () => Promise.resolve({ ok: false, error: { code: 'engine.busy', message: 'Wait for the current engine operation.' } });
  window.__INCANT_BRIDGE__ = { protocolVersion: 1, capabilities: ['entity.rename', 'entity.delete', 'history.undo', 'history.redo',
    'provider.connect', 'provider.cancel', 'provider.disconnect', 'provider.switch'], label: 'Evidence test double (not engine)',
    isFixture: false, getSnapshot: () => snapshot, subscribe: () => () => {}, dispatch: refuse, request: () => Promise.resolve({ ok: true }) };
})();`;

const server = await preview({ root: uiRoot, logLevel: 'warn', preview: { port: 4189, strictPort: true, host: '127.0.0.1' } });
const origin = 'http://127.0.0.1:4189';
const browser = await chromium.launch({ executablePath, headless: true });
const report = {
  browser: `Chrome ${browser.version()} headless`,
  data: 'SYNTHETIC: sample fixture and evidence test doubles, not engine state',
  shots: [],
  axe: [],
  focus: [],
  text: [],
  errors: [],
};

async function page(size, { init } = {}) {
  const context = await browser.newContext({ viewport: size, deviceScaleFactor: 2, colorScheme: 'dark', bypassCSP: true });
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
  await p.waitForTimeout(200);
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
/** Visible panel text, so the report records exactly what each state claims. */
async function panelText(p, label) {
  report.text.push({
    label,
    ...(await p.evaluate(() => {
      const t = (s) => document.querySelector(s)?.innerText.replace(/\s+/g, ' ').trim() ?? null;
      return {
        titlebar: t('.titlebar__identity'),
        banner: t('.notice-region'),
        hierarchy: t('[data-region="hierarchy"]'),
        viewport: t('[data-region="viewport"]'),
        dock: t('[data-region="dock"]'),
        inspector: t('[data-region="inspector"]'),
        agent: t('[data-region="agent"] .agent__transcript'),
        statusbar: t('.statusbar'),
        alerts: [...document.querySelectorAll('[role="alert"]')].length,
      };
    })),
  });
}
/** Chrome stand-in for WebKit's missing :focus-visible after script focus. */
const simulateWebKitFocus = (p) =>
  p.evaluate(() => {
    let rewritten = 0;
    for (const sheet of document.styleSheets) {
      const walk = (rules) => {
        for (const rule of rules) {
          if (rule.selectorText?.includes(':focus-visible')) {
            rule.selectorText = rule.selectorText.replaceAll(':focus-visible', ':not(*)');
            rewritten += 1;
          }
          if (rule.cssRules) walk(rule.cssRules);
        }
      };
      walk(sheet.cssRules);
    }
    return rewritten;
  });
async function focusReport(p, label) {
  report.focus.push({
    label,
    ...(await p.evaluate(() => {
      const el = document.activeElement;
      const st = el ? getComputedStyle(el) : null;
      return {
        active: el?.getAttribute('aria-label') ?? el?.textContent?.trim() ?? null,
        dataFocusVisible: el?.hasAttribute('data-focus-visible') ?? false,
        matchesFocusVisible: el?.matches(':focus-visible') ?? false,
        outline: st ? `${st.outlineStyle} ${st.outlineWidth} ${st.outlineColor}` : null,
      };
    })),
  });
}

try {
  for (const [w, h] of [[1000, 650], [1440, 900]]) {
    const size = `${w}x${h}`;
    for (const state of ['loading', 'failed']) {
      const { context, p } = await page({ width: w, height: h }, { init: projectDouble(state) });
      await open(p, '');
      await shot(p, `0${state === 'loading' ? 1 : 2}-project-${state}-${size}`);
      await panelText(p, `project ${state} ${size}, Problems tab`);
      await p.getByRole('tab', { name: /History/ }).click();
      await shot(p, `0${state === 'loading' ? 1 : 2}b-project-${state}-history-${size}`);
      await panelText(p, `project ${state} ${size}, History tab`);
      if (w === 1440) await axe(p, `project ${state} ${size}`);
      await context.close();
    }
    {
      const { context, p } = await page({ width: w, height: h });
      await open(p, '?fixture=sample&provider=signed-in-multi');
      await shot(p, `03-project-ready-${size}`);
      await panelText(p, `project ready ${size}`);
      // Pointer-opened account dialog.
      await p.locator('.provider-chip').click();
      await focusReport(p, `pointer-open ${size}`);
      await shot(p, `04-account-pointer-open-${size}`);
      await p.keyboard.press('Escape');
      // Keyboard-opened account dialog: focus the chip by Tab order, then Enter.
      await p.mouse.click(w / 2, h - 200);
      await p.locator('.provider-chip').focus();
      await p.keyboard.press('Enter');
      await focusReport(p, `keyboard-open ${size}`);
      await shot(p, `05-account-keyboard-open-${size}`);
      if (w === 1440) {
        await axe(p, `account keyboard-open ${size}`);
        await p.keyboard.press('Tab');
        await focusReport(p, `keyboard Tab inside dialog ${size}`);
        await shot(p, `05b-account-keyboard-tab-${size}`);
      }
      await context.close();
    }
    // Same two paths with :focus-visible disabled, standing in for WebKit.
    {
      const { context, p } = await page({ width: w, height: h });
      await open(p, '?fixture=sample&provider=signed-in-multi');
      report.focus.push({ label: `webkit-sim ${size}`, rewrittenRules: await simulateWebKitFocus(p) });
      await p.locator('.provider-chip').click();
      await focusReport(p, `webkit-sim pointer-open ${size}`);
      await shot(p, `06-webkit-sim-account-pointer-open-${size}`);
      await p.keyboard.press('Escape');
      await p.mouse.click(w / 2, h - 200);
      await p.locator('.provider-chip').focus();
      await p.keyboard.press('Enter');
      await focusReport(p, `webkit-sim keyboard-open ${size}`);
      await shot(p, `07-webkit-sim-account-keyboard-open-${size}`);
      await shot(p, `07c-webkit-sim-account-keyboard-open-detail-${size}`, await p.locator('.dialog__header').boundingBox().then((b) => ({ x: b.x - 8, y: b.y - 8, width: b.width + 16, height: b.height + 16 })));
      // Keyboard sign-out question: focus must land, visibly, on the safe answer.
      await p.getByRole('button', { name: 'Sign out' }).focus();
      await p.keyboard.press('Enter');
      await focusReport(p, `webkit-sim keyboard sign-out question ${size}`);
      await shot(p, `08-webkit-sim-signout-question-keyboard-${size}`);
      await context.close();
    }
  }
} finally {
  writeFileSync(join(outDir, 'report.json'), JSON.stringify(report, null, 2));
  await browser.close();
  await new Promise((r) => server.httpServer.close(r));
}
console.log(`wrote ${report.shots.length} captures to ${outDir}; errors: ${report.errors.length}`);
