#!/usr/bin/env node
/**
 * BROWSER captures and keyboard flows for the project file menu and engine pin review,
 * handoff 0077.
 *
 * Serves editor/ui/dist with `vite preview` on the first free port from 4377, drives local
 * Chrome (headless, its own profile) through the pinned playwright-core, and installs,
 * before the page's modules run:
 *   1. the scenario's configuration,
 *   2. tools/fake-host.js, a scripted stand-in for the native host's IPC (window.__TAURI__),
 *   3. the real compiled bridge, editor/bridge/native.generated.js, unchanged,
 *   4. a relabel of that bridge as a fixture, so every capture says it is not live,
 *   5. for `rejectRequests` scenarios only, a request that rejects instead of resolving.
 * DETERMINISTIC FIXTURE DATA: no account, network, engine or file is used, and nothing here
 * is native WebKit evidence or proof that a project file was written. PNGs and report.json
 * go to a NEW ignored directory artifacts/0077-project-file-ui/<set>/ (an existing one is
 * refused).
 *
 * Usage (worktree root, after the UI build):
 *   node handoffs/0077-project-file-ui/tools/capture.mjs <set> [--only=key,key] [--no-flows] [--flows-only]
 */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import { preview } from 'vite';
import { SCENARIOS } from './scenarios.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const uiRoot = join(repoRoot, 'editor/ui');
const args = process.argv.slice(2);
const set = args.find((a) => !a.startsWith('--'));
if (!set || !/^[a-z0-9-]+$/.test(set)) throw new Error('Usage: capture.mjs <set-name> [--only=a,b] [--no-flows] [--flows-only]');
const only = args.find((a) => a.startsWith('--only='))?.slice('--only='.length).split(',').filter(Boolean);
const noFlows = args.includes('--no-flows');
const flowsOnly = args.includes('--flows-only');
const outDir = join(repoRoot, 'artifacts/0077-project-file-ui', set);
if (existsSync(outDir)) throw new Error(`${outDir} exists; choose a new set name.`);
if (!existsSync(join(uiRoot, 'dist/index.html'))) throw new Error('Run the UI build first.');
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find((p) => p && existsSync(p));
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');
const axeSource = readFileSync(createRequire(join(uiRoot, 'package.json')).resolve('axe-core/axe.min.js'), 'utf8');
const hostScript = readFileSync(join(here, 'fake-host.js'), 'utf8');
const bridgeScript = readFileSync(join(repoRoot, 'editor/bridge/native.generated.js'), 'utf8');
const RELABEL = `(() => {
  const bridge = window.__INCANT_BRIDGE__;
  if (!bridge) throw new Error('Project file fixture: the compiled bridge did not install.');
  bridge.label = 'Project file fixture: scripted host IPC, not live';
  bridge.isFixture = true;
})();`;
/** FIXTURE: for scenarios with `rejectRequests`, the bridge's request promise rejects with a structured error instead of resolving. */
const REJECT = `(() => {
  const reject = window.__FILE_FIXTURE_CONFIG__.rejectRequests;
  if (!reject) return;
  const bridge = window.__INCANT_BRIDGE__;
  const request = bridge.request.bind(bridge);
  bridge.request = (hostRequest) => (reject[hostRequest.type] ? Promise.reject(reject[hostRequest.type]) : request(hostRequest));
})();`;

/** The normal desktop window and a narrower one (the shell's minimum width is 960). */
const SIZES = {
  desktop: { width: 1440, height: 900 },
  narrow: { width: 1024, height: 680 },
};
const BOTH = ['desktop', 'narrow'];

const titleButton = (page) => page.locator('.titlebar__project-button');
const dialog = (page) => page.getByRole('dialog', { name: 'Pin engine version' });
const openMenu = async (page) => {
  await titleButton(page).click();
  await page.getByRole('menu', { name: 'Project file' }).waitFor();
};
const chooseItem = (name) => async (page) => {
  await openMenu(page);
  await page.getByRole('menuitem', { name }).click();
};
const openPin = async (page) => {
  await chooseItem(/^Pin engine version/)(page);
  await dialog(page).waitFor();
};
const reviewed = async (page) => {
  await openPin(page);
  await dialog(page).getByRole('button', { name: 'Pin', exact: true }).and(page.locator(':not([aria-disabled])')).waitFor();
};
const statusLine = (text) => async (page) => page.locator('.statusbar__message', { hasText: text }).waitFor();
const press = (keys) => async (page) => page.keyboard.press(keys);

/** Static states: scenario, steps before capture, sizes, and what to crop. */
const STATES = [
  { key: 'menu', scenario: 'saved', steps: [openMenu], sizes: BOTH, crop: 'titlebar' },
  {
    key: 'menu-keyboard',
    scenario: 'saved',
    steps: [
      async (page) => {
        // Keyboard only: Tab from the page start to the project button, then Enter.
        for (let n = 0; n < 6 && !(await titleButton(page).evaluate((el) => el === document.activeElement)); n += 1) await page.keyboard.press('Tab');
        await page.keyboard.press('Enter');
        await page.getByRole('menu', { name: 'Project file' }).waitFor();
      },
    ],
    sizes: ['desktop'],
    crop: 'titlebar',
  },
  { key: 'menu-unavailable', scenario: 'older-host', steps: [openMenu], sizes: ['desktop'], crop: 'titlebar' },
  { key: 'menu-long-name', scenario: 'long-name', steps: [openMenu], sizes: ['narrow'], crop: 'titlebar' },
  { key: 'saving', scenario: 'slow-save', steps: [press('Meta+S'), async (page) => titleButton(page).locator('.spinner').waitFor()], sizes: BOTH, crop: 'titlebar' },
  { key: 'saved', scenario: 'unsaved', steps: [press('Meta+S'), statusLine('Saved revision')], sizes: BOTH, crop: 'window' },
  { key: 'nothing-to-save', scenario: 'saved', steps: [press('Meta+S'), statusLine('Nothing to save')], sizes: ['desktop'], crop: 'statusbar' },
  { key: 'save-failed', scenario: 'save-fails', steps: [press('Meta+S'), async (page) => page.getByRole('alert').filter({ hasText: 'Save result unknown.' }).waitFor()], sizes: BOTH, crop: 'window' },
  { key: 'review-requesting', scenario: 'slow-review', steps: [openPin], sizes: ['desktop'], crop: 'dialog' },
  { key: 'review', scenario: 'saved', steps: [reviewed], sizes: BOTH, crop: 'dialog', window: true },
  {
    key: 'review-details',
    scenario: 'saved',
    steps: [reviewed, async (page) => dialog(page).getByRole('button', { name: 'Review details' }).click()],
    sizes: BOTH,
    crop: 'dialog',
  },
  { key: 'review-refused-unsaved', scenario: 'unsaved', steps: [openPin, async (page) => dialog(page).getByRole('alert').waitFor()], sizes: BOTH, crop: 'dialog' },
  { key: 'review-refused-pinned', scenario: 'pinned', steps: [openPin, async (page) => dialog(page).getByRole('alert').waitFor()], sizes: ['desktop'], crop: 'dialog' },
  {
    key: 'applying',
    scenario: 'slow-apply',
    steps: [reviewed, async (page) => dialog(page).getByRole('button', { name: 'Pin', exact: true }).click(), async (page) => dialog(page).getByRole('button', { name: 'Pinning…' }).waitFor()],
    sizes: ['desktop'],
    crop: 'dialog',
  },
  {
    key: 'applied',
    scenario: 'saved',
    steps: [reviewed, async (page) => dialog(page).getByRole('button', { name: 'Pin', exact: true }).click(), async (page) => dialog(page).getByText('Pinned.').waitFor()],
    sizes: BOTH,
    crop: 'dialog',
  },
  {
    key: 'apply-failed',
    scenario: 'apply-fails',
    steps: [reviewed, async (page) => dialog(page).getByRole('button', { name: 'Pin', exact: true }).click(), async (page) => dialog(page).getByRole('alert').waitFor()],
    sizes: ['desktop'],
    crop: 'dialog',
  },
  {
    key: 'stale',
    scenario: 'saved',
    steps: [reviewed, async (page) => page.evaluate(() => window.__FILE_FIXTURE__.change()), async (page) => dialog(page).getByRole('alert').waitFor()],
    sizes: BOTH,
    crop: 'dialog',
  },
  {
    key: 'retired',
    scenario: 'slow-apply',
    steps: [
      reviewed,
      async (page) => dialog(page).getByRole('button', { name: 'Pin', exact: true }).click(),
      async (page) => page.evaluate(() => window.__FILE_FIXTURE__.openOther()),
      async (page) => dialog(page).getByRole('alert').waitFor(),
    ],
    sizes: ['desktop'],
    crop: 'dialog',
    window: true,
  },
  { key: 'save-rejected', scenario: 'save-rejected', steps: [press('Meta+S'), async (page) => page.getByRole('alert').filter({ hasText: 'Save result unknown.' }).waitFor()], sizes: ['desktop'], crop: 'window' },
  {
    key: 'apply-rejected',
    scenario: 'apply-rejected',
    steps: [reviewed, async (page) => dialog(page).getByRole('button', { name: 'Pin', exact: true }).click(), async (page) => dialog(page).getByRole('alert').waitFor()],
    sizes: ['desktop'],
    crop: 'dialog',
  },
  {
    key: 'dialog-keyboard-focus',
    scenario: 'saved',
    steps: [
      async (page) => {
        for (let n = 0; n < 6 && !(await titleButton(page).evaluate((el) => el === document.activeElement)); n += 1) await page.keyboard.press('Tab');
        await page.keyboard.press('Enter');
        await page.keyboard.press('ArrowDown');
        await page.keyboard.press('Enter');
        await dialog(page).getByRole('button', { name: 'Pin', exact: true }).and(page.locator(':not([aria-disabled])')).waitFor();
        await page.keyboard.press('Tab');
      },
    ],
    sizes: ['desktop'],
    crop: 'dialog',
  },
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

/** Geometry, overflow, focus and words of what the state shows. */
function measure() {
  const round = (n) => Math.round(n * 10) / 10;
  const rect = (el) => {
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return { x: round(r.x), y: round(r.y), width: round(r.width), height: round(r.height) };
  };
  const inside = (r) => !r || (r.x >= 0 && r.y >= 0 && r.x + r.width <= innerWidth + 0.5 && r.y + r.height <= innerHeight + 0.5);
  const active = document.activeElement;
  const dialogEl = document.querySelector('[role="dialog"]');
  const menuEl = document.querySelector('[role="menu"]');
  const button = document.querySelector('.titlebar__project-button');
  const name = button?.querySelector('.titlebar__project-name');
  const panels = [...document.querySelectorAll('[data-region]')].map((el) => ({ region: el.getAttribute('data-region'), ...rect(el) }));
  return {
    window: { width: innerWidth, height: innerHeight, scrollWidth: document.documentElement.scrollWidth, scrollHeight: document.documentElement.scrollHeight, scrollX, scrollY },
    titlebar: rect(document.querySelector('.titlebar')),
    projectButton: rect(button),
    projectName: name ? { ...rect(name), truncated: name.scrollWidth > name.clientWidth + 0.5, text: name.textContent } : null,
    projectButtonLabel: button?.getAttribute('aria-label') ?? null,
    menu: menuEl ? { ...rect(menuEl), inside: inside(rect(menuEl)), items: [...menuEl.querySelectorAll('[role="menuitem"]')].map((item) => ({ text: item.textContent, disabled: item.getAttribute('aria-disabled') })) } : null,
    dialog: dialogEl
      ? {
          ...rect(dialogEl),
          inside: inside(rect(dialogEl)),
          scrolls: dialogEl.querySelector('.dialog__body').scrollHeight > dialogEl.querySelector('.dialog__body').clientHeight + 1,
          described: (dialogEl.getAttribute('aria-describedby') ?? '').split(' ').map((id) => document.getElementById(id)?.textContent ?? '').join(' '),
          buttons: [...dialogEl.querySelectorAll('button')].map((b) => ({ text: b.textContent.trim(), disabled: b.getAttribute('aria-disabled') })),
        }
      : null,
    notice: [...document.querySelectorAll('.notice-region .notice')].map((el) => el.textContent),
    status: document.querySelector('.statusbar__message')?.textContent ?? '',
    panels,
    active: active ? { tag: active.tagName, className: String(active.className), name: active.getAttribute('aria-label') ?? active.textContent?.trim().slice(0, 60), focusVisible: active.matches(':focus-visible') || active.hasAttribute('data-focus-visible') } : null,
    imgInDialog: dialogEl ? dialogEl.querySelectorAll('img').length : 0,
  };
}

const server = await preview({ root: uiRoot, cacheDir: join(uiRoot, 'node_modules/.vite'), preview: { port: await freePort(4377), strictPort: true, host: '127.0.0.1' }, logLevel: 'error' });
const origin = server.resolvedUrls.local[0].replace(/\/$/, '');
const browser = await chromium.launch({ executablePath, headless: true });
mkdirSync(outDir, { recursive: true });
const report = {
  set,
  generated: new Date().toISOString(),
  browser: browser.version(),
  data: 'deterministic project file fixture (tools/fake-host.js + real native.generated.js); not live, not native WebKit, no file written',
  captures: [],
  flows: [],
};

async function open(scenarioKey, size, { deviceScaleFactor = 2 } = {}) {
  const context = await browser.newContext({ viewport: size, deviceScaleFactor, colorScheme: 'dark', reducedMotion: 'reduce' });
  const page = await context.newPage();
  const errors = [];
  const requests = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  page.on('request', (r) => {
    if (!r.url().startsWith(origin)) requests.push(r.url());
  });
  await page.addInitScript({ content: `window.__FILE_FIXTURE_CONFIG__ = ${JSON.stringify(SCENARIOS[scenarioKey])};` });
  await page.addInitScript({ content: hostScript });
  await page.addInitScript({ content: bridgeScript });
  await page.addInitScript({ content: RELABEL });
  await page.addInitScript({ content: REJECT });
  await page.goto(`${origin}/`, { waitUntil: 'networkidle' });
  await titleButton(page).waitFor();
  return { context, page, errors, requests };
}

async function axeOn(page) {
  // Evaluated through the DevTools protocol: the page keeps its real CSP (script-src 'self').
  await page.evaluate(axeSource);
  return page.evaluate(async () => {
    // eslint-disable-next-line no-undef
    const result = await axe.run(document, { resultTypes: ['violations'] });
    return result.violations.map((v) => ({ id: v.id, impact: v.impact, nodes: v.nodes.length, targets: v.nodes.slice(0, 4).map((n) => n.target.join(' ')) }));
  });
}

const fileCalls = (page) => page.evaluate(() => window.__FILE_FIXTURE__.calls.filter((call) => call.name === 'engine_project_file' || call.name === 'window_action').map((call) => ({ name: call.name, ...(call.args?.request ? { request: call.args.request } : { args: call.args }) })));

try {
  if (!flowsOnly) {
    for (const state of STATES) {
      if (only && !only.includes(state.key)) continue;
      for (const sizeKey of state.sizes) {
        const size = SIZES[sizeKey];
        const { context, page, errors, requests } = await open(state.scenario, size);
        let stepError = null;
        try {
          for (const step of state.steps) await step(page);
          await page.waitForTimeout(300);
        } catch (error) {
          stepError = String(error);
        }
        const file = `${state.key}-${sizeKey}`;
        if (state.crop === 'titlebar') {
          await page.screenshot({ path: join(outDir, `${file}-titlebar.png`), clip: { x: 0, y: 0, width: size.width, height: 150 } });
        } else if (state.crop === 'statusbar') {
          const bar = await page.locator('.statusbar').boundingBox();
          await page.screenshot({ path: join(outDir, `${file}-statusbar.png`), clip: { x: 0, y: bar.y - 1, width: size.width, height: bar.height + 1 } });
        } else if (state.crop === 'dialog') {
          const box = await dialog(page).boundingBox();
          if (box) {
            const pad = 24;
            const x = Math.max(0, box.x - pad);
            const y = Math.max(0, box.y - pad);
            await page.screenshot({ path: join(outDir, `${file}-dialog.png`), clip: { x, y, width: Math.min(size.width - x, box.width + 2 * pad), height: Math.min(size.height - y, box.height + 2 * pad) } });
          }
        }
        if (state.crop === 'window' || state.window) await page.screenshot({ path: join(outDir, `${file}-window.png`), scale: 'css' });
        const measured = await page.evaluate(measure);
        const axe = await axeOn(page);
        const calls = await fileCalls(page);
        report.captures.push({ file, state: state.key, scenario: state.scenario, size: sizeKey, ...size, measured, axe, calls, errors, requests, stepError });
        console.log(`${file}: ${stepError ? `STEP ERROR ${stepError}` : 'ok'} axe=${axe.length} errors=${errors.length}`);
        await context.close();
      }
    }
  }
  if (!noFlows) report.flows = await flows();
} finally {
  await browser.close();
  await new Promise((done) => server.httpServer.close(done));
}
writeFileSync(join(outDir, 'report.json'), `${JSON.stringify(report, null, 2)}\n`);
console.log(`Wrote ${report.captures.length} captures and ${report.flows.length} flows to ${outDir}`);

/* ---- Keyboard and pointer flows through the real bridge ---- */

async function flows() {
  const results = [];
  const run = async (name, scenario, body) => {
    const { context, page, errors, requests } = await open(scenario, SIZES.desktop, { deviceScaleFactor: 1 });
    const result = { name, scenario, errors, requests, checks: {}, error: null };
    try {
      await body(page, result.checks);
    } catch (error) {
      result.error = String(error);
    }
    result.calls = await fileCalls(page);
    results.push(result);
    console.log(`flow ${name}: ${result.error ?? 'ok'}`);
    await context.close();
  };
  const active = (page) =>
    page.evaluate(() => {
      const el = document.activeElement;
      return { tag: el?.tagName, className: String(el?.className ?? ''), text: el?.textContent?.trim().slice(0, 60), label: el?.getAttribute('aria-label') ?? null, focusVisible: !!el && (el.matches(':focus-visible') || el.hasAttribute('data-focus-visible')) };
    });
  const status = (page) => page.locator('.statusbar__message').innerText();

  // ⌘S from a text field: the field keeps its text and focus; one save goes to the host.
  await run('save-from-text-field', 'unsaved', async (page, checks) => {
    const filter = page.getByRole('searchbox', { name: 'Filter hierarchy by name' });
    await filter.click();
    await page.keyboard.type('lan');
    await page.keyboard.press('Meta+S');
    await statusLine('Saved revision')(page);
    checks.status = await status(page);
    checks.filterValue = await filter.inputValue();
    checks.focus = await active(page);
  });

  // Keyboard only, from the titlebar to a pinned project and back.
  await run('keyboard-pin', 'saved', async (page, checks) => {
    for (let n = 0; n < 6 && !(await titleButton(page).evaluate((el) => el === document.activeElement)); n += 1) await page.keyboard.press('Tab');
    checks.start = await active(page);
    await page.keyboard.press('Enter');
    checks.menuFocus = await active(page);
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Enter');
    await dialog(page).waitFor();
    checks.dialogFocus = await active(page);
    await dialog(page).getByRole('button', { name: 'Pin', exact: true }).and(page.locator(':not([aria-disabled])')).waitFor();
    checks.afterReviewFocus = await active(page);
    await page.keyboard.press('Tab');
    checks.tabbedTo = await active(page);
    await page.keyboard.press('Enter');
    await dialog(page).getByText('Pinned.').waitFor();
    checks.afterPinFocus = await active(page);
    checks.described = await dialog(page).evaluate((el) => document.getElementById(el.getAttribute('aria-describedby'))?.textContent);
    await page.keyboard.press('Escape');
    checks.dialogGone = (await dialog(page).count()) === 0;
    checks.returnedFocus = await active(page);
    checks.history = await page.evaluate(() => window.__FILE_FIXTURE__.engine.history.map((entry) => entry.description));
  });

  // Holding the keys and choosing Save again sends one request while one runs.
  await run('duplicate-save', 'slow-save', async (page, checks) => {
    await page.keyboard.press('Meta+S');
    await page.keyboard.down('Meta');
    await page.keyboard.press('s');
    await page.keyboard.press('s');
    await page.keyboard.up('Meta');
    await openMenu(page);
    checks.saveItem = await page.getByRole('menuitem', { name: /^Save/ }).evaluate((el) => ({ text: el.textContent, disabled: el.getAttribute('aria-disabled') }));
    await page.getByRole('menuitem', { name: /^Save/ }).click({ force: true });
    await page.waitForTimeout(200);
    checks.titleLabel = await titleButton(page).getAttribute('aria-label');
  });

  // The project name and the menu are not drag areas; the empty titlebar is.
  await run('drag-areas', 'saved', async (page, checks) => {
    const drags = () => page.evaluate(() => window.__FILE_FIXTURE__.calls.filter((call) => call.name === 'window_action').length);
    await titleButton(page).click();
    await page.getByRole('menuitem', { name: /^Save/ }).hover();
    await page.mouse.down();
    await page.mouse.up();
    checks.afterButtonAndMenu = await drags();
    const spacer = await page.locator('.titlebar__start .titlebar__spacer').boundingBox();
    await page.mouse.click(spacer.x + spacer.width / 2, spacer.y + spacer.height / 2);
    checks.afterSpacer = await drags();
  });

  // Escape from a pointer-opened dialog returns focus to the project button.
  await run('escape-returns-focus', 'saved', async (page, checks) => {
    await reviewed(page);
    await page.keyboard.press('Escape');
    checks.dialogGone = (await dialog(page).count()) === 0;
    checks.focus = await active(page);
  });

  return results;
}
