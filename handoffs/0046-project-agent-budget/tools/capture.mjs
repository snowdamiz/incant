#!/usr/bin/env node
/**
 * BROWSER captures and flows for the Agent pane's token allowance, handoff 0046.
 *
 * Serves editor/ui/dist with `vite preview` on the first free port from 4346, drives local
 * Chrome through the pinned playwright-core, and installs, before the page's modules run:
 *   1. the scenario's configuration,
 *   2. tools/fake-host.js, a scripted stand-in for the native host's IPC (window.__TAURI__),
 *   3. the real compiled bridge, editor/bridge/native.generated.js, unchanged,
 *   4. a relabel of that bridge as a fixture, so every capture says it is not live.
 * DETERMINISTIC FIXTURE DATA: no provider, account, network or engine is used, and nothing
 * here is native WebKit evidence. PNGs and report.json go to a NEW ignored directory
 * artifacts/0046-project-agent-budget/<set>/ (an existing one is refused).
 *
 * Usage (repo root, after the UI build):
 *   node handoffs/0046-project-agent-budget/tools/capture.mjs <set> [--only=key,key] [--sizes=a,b] [--flows-only] [--no-flows]
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
if (!set || !/^[a-z0-9-]+$/.test(set)) throw new Error('Usage: capture.mjs <set-name> [--only=a,b] [--sizes=a,b] [--flows-only] [--no-flows]');
const flag = (name) => args.find((a) => a.startsWith(`--${name}=`))?.split('=')[1]?.split(',').filter(Boolean);
const only = flag('only');
const sizeFilter = flag('sizes');
const flowsOnly = args.includes('--flows-only');
const noFlows = args.includes('--no-flows');
const outDir = join(repoRoot, 'artifacts/0046-project-agent-budget', set);
if (existsSync(outDir)) throw new Error(`${outDir} exists; choose a new set name.`);
if (!existsSync(join(uiRoot, 'dist/index.html'))) throw new Error('Run the UI build first.');
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find((p) => p && existsSync(p));
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');
const axeSource = readFileSync(createRequire(join(uiRoot, 'package.json')).resolve('axe-core/axe.min.js'), 'utf8');
const hostScript = readFileSync(join(here, 'fake-host.js'), 'utf8');
const bridgeScript = readFileSync(join(repoRoot, 'editor/bridge/native.generated.js'), 'utf8');
const RELABEL = `(() => {
  const bridge = window.__INCANT_BRIDGE__;
  if (!bridge) throw new Error('Allowance fixture: the compiled bridge did not install.');
  bridge.label = 'Allowance fixture: scripted host IPC, not live';
  bridge.isFixture = true;
})();`;
/** Records, in the page, every status the header shows and whether a cannot-run note ever appears. */
const WATCH = `(() => {
  const seen = { statuses: [], notes: [] };
  window.__ALLOWANCE_WATCH__ = seen;
  const check = () => {
    const pane = document.querySelector('[data-region="agent"]');
    if (!pane) return;
    const status = pane.querySelector('.agent__status')?.textContent?.replace(/^,\\s*/, '').trim();
    if (status && seen.statuses.at(-1) !== status) seen.statuses.push(status);
    const note = pane.querySelector('.agent-note__body')?.textContent;
    if (note && seen.notes.at(-1) !== note) seen.notes.push(note);
  };
  new MutationObserver(check).observe(document, { subtree: true, childList: true, characterData: true });
})();`;

/** Window size and Inspector column width. 1000 x 650 starts with the Agent pane collapsed. */
const SIZES = {
  'w1440-s380': { width: 1440, height: 900, side: 380 },
  'w1440-s280': { width: 1440, height: 900, side: 280 },
  'w1000-s280': { width: 1000, height: 650, side: 280 },
  'w1000-s380': { width: 1000, height: 650, side: 380 },
};
const ALL = Object.keys(SIZES);

const agent = (page) => page.locator('[data-region="agent"]');
const status = (text) => async (page) => {
  await agent(page).locator('.agent__status', { hasText: text }).waitFor();
};
const openEditor = async (page) => {
  await agent(page).getByRole('button', { name: 'Change allowance' }).click();
  await agent(page).getByRole('textbox', { name: 'Allowance in tokens for this project' }).waitFor();
};
const typeAllowance = (text) => async (page) => {
  await openEditor(page);
  await agent(page).getByRole('textbox', { name: 'Allowance in tokens for this project' }).fill(text);
};
const submitAllowance = async (page) => {
  await agent(page).getByRole('textbox', { name: 'Allowance in tokens for this project' }).press('Enter');
};
const field = (page) => agent(page).getByRole('textbox', { name: 'Allowance in tokens for this project' });

/** Static states: the scenario the fixture serves, what to wait for, and steps before capture. */
const STATES = [
  { key: 'ready-default', scenario: 'ready-default', ready: status('Ready'), sizes: ALL },
  { key: 'ready', scenario: 'ready', ready: status('Ready'), sizes: ALL, window: ['w1440-s380', 'w1000-s280'] },
  {
    key: 'change-focused',
    scenario: 'ready',
    ready: status('Ready'),
    steps: [
      async (page) => {
        await page.getByLabel('Message to the agent').click();
        for (let n = 0; n < 4; n += 1) await page.keyboard.press('Tab');
      },
    ],
    sizes: ['w1440-s280', 'w1000-s380'],
  },
  { key: 'editing', scenario: 'ready', ready: status('Ready'), steps: [openEditor], sizes: ALL, window: ['w1440-s380'] },
  { key: 'editing-invalid', scenario: 'ready', ready: status('Ready'), steps: [typeAllowance('1.5')], sizes: ['w1440-s380', 'w1440-s280', 'w1000-s280'] },
  { key: 'editing-below', scenario: 'ready', ready: status('Ready'), steps: [typeAllowance('5000')], sizes: ['w1440-s280', 'w1000-s380'] },
  { key: 'editing-zero', scenario: 'ready', ready: status('Ready'), steps: [typeAllowance('0')], sizes: ['w1440-s380', 'w1000-s280'] },
  { key: 'editing-running', scenario: 'approval', ready: status('Needs approval'), steps: [openEditor], sizes: ['w1440-s380', 'w1440-s280', 'w1000-s280'], agentHeight: 520 },
  {
    key: 'saving',
    scenario: 'flow-slow-save',
    ready: status('Ready'),
    steps: [typeAllowance('500000'), submitAllowance, status('Updating…')],
    sizes: ['w1440-s380', 'w1000-s280'],
  },
  {
    key: 'save-failed',
    scenario: 'flow-save-fails',
    ready: status('Ready'),
    steps: [typeAllowance('300000'), submitAllowance, async (page) => agent(page).getByText('Project revision changed.').waitFor()],
    sizes: ['w1440-s280', 'w1000-s280'],
  },
  { key: 'exhausted', scenario: 'exhausted', ready: status('Allowance used up'), sizes: ALL },
  { key: 'exhausted-editing', scenario: 'exhausted', ready: status('Allowance used up'), steps: [typeAllowance('500000')], sizes: ['w1440-s280', 'w1000-s280'] },
  { key: 'over-cap', scenario: 'over-cap', ready: status('Allowance used up'), sizes: ['w1440-s380', 'w1000-s280'] },
  { key: 'zero', scenario: 'zero', ready: status('Off'), sizes: ['w1440-s380', 'w1440-s280', 'w1000-s280'] },
  { key: 'reserved', scenario: 'reserved', ready: status('Allowance used up'), sizes: ['w1440-s380', 'w1440-s280'] },
  { key: 'signed-out', scenario: 'signed-out', ready: status('Not signed in'), sizes: ['w1440-s380', 'w1000-s280'] },
  { key: 'signed-out-editing', scenario: 'signed-out', ready: status('Not signed in'), steps: [openEditor], sizes: ['w1440-s280', 'w1000-s280'] },
  { key: 'older-host', scenario: 'older-host', ready: status('Ready'), sizes: ['w1440-s380', 'w1000-s280'] },
  { key: 'older-exhausted', scenario: 'older-exhausted', ready: status('Allowance used up'), sizes: ['w1440-s380', 'w1000-s280'] },
  { key: 'ready-max', scenario: 'ready-max', ready: status('Ready'), sizes: ['w1440-s280', 'w1000-s280'] },
  { key: 'editing-max', scenario: 'ready-max', ready: status('Ready'), steps: [openEditor], sizes: ['w1440-s280', 'w1000-s380'] },
  {
    key: 'collapsed-exhausted',
    scenario: 'exhausted',
    ready: status('Allowance used up'),
    steps: [async (page) => agent(page).getByRole('button', { name: /^Agent/ }).click()],
    sizes: ['w1440-s280'],
    keepCollapsed: true,
  },
  {
    key: 'stalled',
    scenario: 'flow-lag',
    ready: status('Ready'),
    steps: [
      async (page) => page.evaluate(() => { window.__ALLOWANCE_FIXTURE__.sessionLag = 600_000; }),
      typeAllowance('400000'),
      submitAllowance,
      async (page) => agent(page).getByRole('button', { name: 'Refresh' }).waitFor({ timeout: 8000 }),
    ],
    sizes: ['w1440-s380', 'w1000-s280'],
  },
  {
    key: 'refresh-failed',
    scenario: 'ready',
    ready: status('Ready'),
    steps: [
      typeAllowance('400000'),
      async (page) => page.evaluate(() => {
        window.__ALLOWANCE_FIXTURE__.readError = 'fixture transport failure (raw text the UI must never show)';
      }),
      submitAllowance,
      async (page) => agent(page).getByRole('button', { name: 'Retry' }).waitFor(),
    ],
    sizes: ['w1440-s380', 'w1000-s280'],
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

async function setSeparator(page, name, target) {
  const separator = page.getByRole('separator', { name });
  const current = Number(await separator.getAttribute('aria-valuenow'));
  if (current === target) return;
  const box = await separator.boundingBox();
  const vertical = (await separator.getAttribute('aria-orientation')) === 'vertical';
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  // Both separators here are inverted: the controlled pane sits after them.
  const delta = current - target;
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(vertical ? x + delta : x, vertical ? y : y + delta, { steps: 4 });
  await page.mouse.up();
  // Off the separator, so its hover highlight is not in the capture.
  await page.mouse.move(page.viewportSize().width * 0.45, page.viewportSize().height * 0.3);
  const now = Number(await separator.getAttribute('aria-valuenow'));
  if (now !== target) throw new Error(`${name}: wanted ${target}, got ${now}`);
}

/** Measurements of the pane as rendered: geometry, overflow, focus and words. */
function measure() {
  const round = (n) => Math.round(n * 10) / 10;
  const rect = (el) => {
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return { x: round(r.x), y: round(r.y), width: round(r.width), height: round(r.height) };
  };
  const pane = document.querySelector('[data-region="agent"]');
  const side = document.querySelector('.column--side');
  const transcript = pane?.querySelector('.agent__transcript');
  const allowance = pane?.querySelector('.agent-allowance');
  const paneRect = pane.getBoundingClientRect();
  // Anything drawn past the pane's edges that is not inside a scroller of its own.
  const escaping = [...pane.querySelectorAll('*')]
    .filter((el) => {
      const r = el.getBoundingClientRect();
      if (r.width === 0 || r.height === 0) return false;
      let parent = el.parentElement;
      while (parent && parent !== pane) {
        const style = getComputedStyle(parent);
        if (/(auto|scroll|hidden|clip)/.test(style.overflowX + style.overflowY)) return false;
        parent = parent.parentElement;
      }
      return r.right > paneRect.right + 0.5 || r.left < paneRect.left - 0.5 || r.bottom > paneRect.bottom + 0.5;
    })
    .map((el) => el.className || el.tagName);
  const horizontal = [...pane.querySelectorAll('*')].filter((el) => {
    const style = getComputedStyle(el);
    return /(auto|scroll)/.test(style.overflowX) && el.scrollWidth > el.clientWidth + 1;
  }).map((el) => el.className || el.tagName);
  const active = document.activeElement;
  const input = allowance?.querySelector('input');
  return {
    window: { width: innerWidth, height: innerHeight, scrollX, scrollY, scrollWidth: document.documentElement.scrollWidth, scrollHeight: document.documentElement.scrollHeight },
    side: rect(side),
    pane: rect(pane),
    status: pane.querySelector('.agent__status')?.textContent?.replace(/^,\s*/, '').trim() ?? null,
    transcript: transcript ? { ...rect(transcript), scrollHeight: transcript.scrollHeight, clientHeight: transcript.clientHeight } : null,
    composer: rect(pane.querySelector('.composer')),
    note: pane.querySelector('.agent-note__body')?.textContent ?? null,
    allowance: allowance
      ? {
          box: rect(allowance),
          line: allowance.querySelector('.agent-usage')?.textContent ?? null,
          level: [...(allowance.querySelector('.agent-usage')?.classList ?? [])].find((name) => name.startsWith('agent-usage--')) ?? null,
          editing: !!input,
          value: input?.value ?? null,
          invalid: input?.getAttribute('aria-invalid') ?? null,
          feedback: allowance.querySelector('.agent-allowance__note')?.textContent ?? null,
          inputWidth: input ? round(input.getBoundingClientRect().width) : null,
          valueFits: input ? input.scrollWidth <= input.clientWidth + 1 : null,
          rows: allowance.querySelector('form') ? new Set([...allowance.querySelectorAll('form > *')].map((el) => Math.round(el.getBoundingClientRect().top))).size : 1,
        }
      : null,
    bottomGap: allowance ? round(paneRect.bottom - allowance.getBoundingClientRect().bottom) : null,
    escaping,
    horizontalScrollers: horizontal,
    active: active ? { tag: active.tagName, className: String(active.className), name: active.getAttribute('aria-label') ?? active.textContent?.trim().slice(0, 60), focusVisible: active.matches(':focus-visible') || active.hasAttribute('data-focus-visible') } : null,
    words: (pane.innerText.match(/\S+/g) ?? []).length,
    rawLeak: document.body.innerText.includes('raw text the UI must never show'),
  };
}

const server = await preview({ root: uiRoot, preview: { port: await freePort(4346), strictPort: true, host: '127.0.0.1' }, logLevel: 'error' });
const origin = server.resolvedUrls.local[0].replace(/\/$/, '');
const browser = await chromium.launch({ executablePath, headless: true });
mkdirSync(outDir, { recursive: true });
const report = { set, generated: new Date().toISOString(), browser: browser.version(), data: 'deterministic allowance fixture (tools/fake-host.js + real native.generated.js); not live, not native WebKit', captures: [], flows: [] };

async function open(scenarioKey, size, { deviceScaleFactor = 2 } = {}) {
  const context = await browser.newContext({ viewport: { width: size.width, height: size.height }, deviceScaleFactor, colorScheme: 'dark', reducedMotion: 'reduce' });
  const page = await context.newPage();
  const errors = [];
  const requests = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  page.on('request', (r) => { if (!r.url().startsWith(origin)) requests.push(r.url()); });
  await page.addInitScript({ content: `window.__ALLOWANCE_FIXTURE_CONFIG__ = ${JSON.stringify(SCENARIOS[scenarioKey])};` });
  await page.addInitScript({ content: hostScript });
  await page.addInitScript({ content: bridgeScript });
  await page.addInitScript({ content: RELABEL });
  await page.addInitScript({ content: WATCH });
  await page.goto(`${origin}/`, { waitUntil: 'networkidle' });
  await page.locator('.titlebar').waitFor();
  return { context, page, errors, requests };
}

async function layout(page, size, state) {
  await setSeparator(page, 'Resize inspector and agent', size.side);
  const toggle = agent(page).getByRole('button', { name: /^Agent/ });
  if ((await toggle.getAttribute('aria-expanded')) === 'false' && !state.keepCollapsed) await toggle.click();
  if (state.agentHeight && (await page.getByRole('separator', { name: 'Resize agent panel' }).count())) {
    const max = Number(await page.getByRole('separator', { name: 'Resize agent panel' }).getAttribute('aria-valuemax'));
    await setSeparator(page, 'Resize agent panel', Math.min(state.agentHeight, max, size.height - 300));
  }
}

async function axeOn(page) {
  // Evaluated through the DevTools protocol: the page keeps its real CSP (script-src 'self').
  await page.evaluate(axeSource);
  return page.evaluate(async () => {
    // eslint-disable-next-line no-undef
    const result = await axe.run(document.querySelector('[data-region="agent"]'), { resultTypes: ['violations'] });
    return result.violations.map((v) => ({ id: v.id, impact: v.impact, nodes: v.nodes.length, targets: v.nodes.slice(0, 3).map((n) => n.target.join(' ')) }));
  });
}

try {
  if (!flowsOnly) {
    for (const state of STATES) {
      if (only && !only.includes(state.key)) continue;
      for (const sizeKey of state.sizes) {
        if (sizeFilter && !sizeFilter.includes(sizeKey)) continue;
        const size = SIZES[sizeKey];
        const { context, page, errors, requests } = await open(state.scenario, size);
        let stepError = null;
        try {
          await layout(page, size, state);
          await state.ready(page);
          for (const step of state.steps ?? []) await step(page);
          await page.waitForTimeout(350);
        } catch (error) {
          stepError = String(error);
        }
        const file = `${state.key}-${sizeKey}`;
        const side = await page.locator('.column--side').boundingBox();
        await page.screenshot({ path: join(outDir, `${file}-side.png`), clip: side });
        if (state.window?.includes(sizeKey)) await page.screenshot({ path: join(outDir, `${file}-window.png`) });
        const measured = await page.evaluate(measure);
        const axe = await axeOn(page);
        report.captures.push({ file, state: state.key, size: sizeKey, ...size, measured, axe, errors, requests, stepError });
        console.log(`${file}: ${stepError ? `STEP ERROR ${stepError}` : 'ok'} axe=${axe.length} errors=${errors.length} escaping=${measured.escaping.length} h=${measured.horizontalScrollers.length}`);
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

/* ---- Interactive flows: real keyboard and pointer input through the real bridge ---- */

async function flows() {
  const results = [];
  const run = async (name, scenario, size, body) => {
    const { context, page, errors, requests } = await open(scenario, size, { deviceScaleFactor: 1 });
    const result = { name, scenario, size, errors, requests, checks: {}, error: null };
    try {
      await layout(page, size, {});
      await body(page, result.checks);
    } catch (error) {
      result.error = String(error);
    }
    result.calls = await page.evaluate(() => window.__ALLOWANCE_FIXTURE__.calls.filter((call) => !['engine_agent_read', 'provider_read', 'viewport_bounds', 'window_read', 'play_read'].includes(call.name)));
    result.watched = await page.evaluate(() => window.__ALLOWANCE_WATCH__);
    results.push(result);
    console.log(`flow ${name}: ${result.error ?? 'ok'}`);
    await context.close();
  };
  const active = (page) => page.evaluate(() => {
    const el = document.activeElement;
    return { tag: el?.tagName, className: String(el?.className ?? ''), text: el?.textContent?.trim().slice(0, 80), label: el?.getAttribute('aria-label') ?? null, focusVisible: !!el && (el.matches(':focus-visible') || el.hasAttribute('data-focus-visible')) };
  });
  const line = (page) => agent(page).locator('.agent-usage').innerText();
  const statusLine = (page) => page.locator('.statusbar__message').innerText();
  const size = SIZES['w1440-s380'];
  const narrow = SIZES['w1000-s280'];

  // Keyboard only, from a used-up allowance: Tab to Change, raise it, and the agent can run again.
  await run('raise-from-exhausted', 'flow-save', narrow, async (page, checks) => {
    await status('Allowance used up')(page);
    checks.noteBefore = await agent(page).locator('.agent-note__body').innerText();
    const change = agent(page).getByRole('button', { name: 'Change allowance' });
    await agent(page).getByRole('button', { name: /^Agent/ }).focus();
    for (let n = 0; n < 12 && !(await change.evaluate((el) => el === document.activeElement)); n += 1) await page.keyboard.press('Tab');
    checks.changeFocused = await active(page);
    await page.keyboard.press('Enter');
    checks.fieldFocused = await active(page);
    checks.selected = await field(page).evaluate((el) => [el.selectionStart, el.selectionEnd, el.value]);
    await page.keyboard.type('500000');
    await page.keyboard.press('Enter');
    await status('Ready')(page);
    await page.waitForTimeout(200);
    checks.focusAfter = await active(page);
    checks.line = await line(page);
    checks.composer = await page.getByLabel('Message to the agent').count();
    checks.statusLine = await statusLine(page);
    await page.screenshot({ path: join(outDir, 'flow-raise-from-exhausted-side.png'), clip: await page.locator('.column--side').boundingBox() });
  });

  // Undo and Redo through the titlebar: the cap changes back and forth, usage never does, and
  // the pane keeps its composer instead of flashing a note.
  await run('undo-redo', 'flow-undo', size, async (page, checks) => {
    await status('Ready')(page);
    await typeAllowance('400000')(page);
    await submitAllowance(page);
    await agent(page).locator('.agent-usage', { hasText: 'of 400,000' }).waitFor();
    await status('Ready')(page);
    await page.evaluate(() => { window.__ALLOWANCE_WATCH__.statuses = []; window.__ALLOWANCE_WATCH__.notes = []; });
    await page.getByRole('button', { name: /^Undo/ }).click();
    await agent(page).locator('.agent-usage', { hasText: 'of 250,000' }).waitFor();
    await status('Ready')(page);
    checks.afterUndo = await line(page);
    await page.getByRole('button', { name: /^Redo/ }).click();
    await agent(page).locator('.agent-usage', { hasText: 'of 400,000' }).waitFor();
    await status('Ready')(page);
    checks.afterRedo = await line(page);
    checks.watchedDuringHistory = await page.evaluate(() => structuredClone(window.__ALLOWANCE_WATCH__));
    checks.history = await page.evaluate(() => window.__ALLOWANCE_FIXTURE__.engine.history.map((entry) => entry.description));
  });

  // Lowering the allowance while the agent waits for approval: the warning says it stops the
  // request, decisions are held while the change applies, Stop stays usable.
  await run('lower-while-running', 'flow-running', size, async (page, checks) => {
    await status('Needs approval')(page);
    await openEditor(page);
    checks.warning = await agent(page).locator('.agent-allowance__note').innerText();
    await field(page).fill('100000');
    await submitAllowance(page);
    await page.waitForTimeout(100);
    checks.whileSaving = {
      decisionNote: await agent(page).locator('.agent-approval__note').innerText().catch(() => null),
      allowDisabled: await agent(page).getByRole('button', { name: 'Allow' }).getAttribute('aria-disabled').catch(() => null),
      stopDisabled: await agent(page).getByRole('button', { name: 'Stop' }).getAttribute('aria-disabled').catch(() => 'missing'),
    };
    await agent(page).locator('.agent-failure').waitFor();
    checks.failure = await agent(page).locator('.agent-failure').innerText();
    await status('Ready')(page);
    checks.line = await line(page);
    await page.screenshot({ path: join(outDir, 'flow-lower-while-running-side.png'), clip: await page.locator('.column--side').boundingBox() });
  });

  // Escape cancels without a command and returns focus to Change.
  await run('escape-cancel', 'ready', size, async (page, checks) => {
    await status('Ready')(page);
    await typeAllowance('123')(page);
    await page.keyboard.press('Escape');
    checks.focusAfter = await active(page);
    checks.editorGone = (await field(page).count()) === 0;
  });

  // Three quick submissions send one command. A save slower than the refresh grace period
  // keeps Updating… (it is in progress, not stalled).
  await run('duplicate-submit', 'flow-slow-save', size, async (page, checks) => {
    await status('Ready')(page);
    await typeAllowance('500000')(page);
    await page.keyboard.press('Enter');
    await page.keyboard.press('Enter');
    // Playwright will not click an aria-disabled control unless forced; a person can.
    await agent(page).getByRole('button', { name: /^Sav/ }).click({ force: true });
    await page.waitForTimeout(200);
    checks.saveLabel = await agent(page).getByRole('button', { name: /^Sav/ }).innerText();
    checks.saveDisabled = await agent(page).getByRole('button', { name: /^Sav/ }).getAttribute('aria-disabled');
    checks.header = await agent(page).locator('.agent__status').innerText();
    await page.waitForTimeout(5000);
    checks.headerAfterGrace = (await agent(page).locator('.agent__status').innerText()).replace(/^,\s*/, '');
    checks.composerAfterGrace = await page.getByLabel('Message to the agent').count();
    checks.refreshOffered = await agent(page).getByRole('button', { name: 'Refresh' }).count();
  });

  // A refused save keeps what was typed and says why; the next save succeeds.
  await run('save-failed', 'flow-save-fails', narrow, async (page, checks) => {
    await status('Ready')(page);
    await typeAllowance('300000')(page);
    await submitAllowance(page);
    await agent(page).getByText('Project revision changed.').waitFor();
    checks.value = await field(page).inputValue();
    checks.focus = await active(page);
    await page.waitForTimeout(150);
    // The shell announces command failures with the command's name (follow-up patch 1).
    checks.statusLine = await statusLine(page);
    await submitAllowance(page);
    await agent(page).locator('.agent-usage', { hasText: 'of 300,000' }).waitFor();
    checks.after = await line(page);
  });

  // The session never catches up by itself: Updating… gives way to the host's reason and
  // Refresh above the same composer.
  await run('stalled', 'flow-lag', size, async (page, checks) => {
    await status('Ready')(page);
    await page.evaluate(() => { window.__ALLOWANCE_FIXTURE__.sessionLag = 1500; });
    await page.getByLabel('Message to the agent').fill('Add fog over the water.');
    await typeAllowance('400000')(page);
    await submitAllowance(page);
    await status('Updating…')(page);
    checks.composerWhileUpdating = await page.getByLabel('Message to the agent').count();
    await agent(page).getByRole('button', { name: 'Refresh' }).waitFor({ timeout: 8000 });
    checks.notice = await agent(page).locator('.agent-notice__text').innerText();
    checks.header = (await agent(page).locator('.agent__status').innerText()).replace(/^,\s*/, '');
    checks.draftWhileStalled = await page.getByLabel('Message to the agent').inputValue();
    await agent(page).getByRole('button', { name: 'Refresh' }).click();
    await status('Ready')(page);
    checks.line = await line(page);
    await page.waitForTimeout(150);
    checks.statusLine = await statusLine(page);
  });

  // Without an account: the allowance is still a project setting that can be changed.
  await run('offline', 'flow-offline', narrow, async (page, checks) => {
    await status('Not signed in')(page);
    await typeAllowance('120000')(page);
    await submitAllowance(page);
    await agent(page).locator('.agent-usage', { hasText: 'of 120,000' }).waitFor();
    checks.line = await line(page);
    checks.header = await agent(page).locator('.agent__status').innerText();
  });

  // Another edit (Undo elsewhere, another window): the line follows, without flashing a note.
  await run('external-change', 'ready', size, async (page, checks) => {
    await status('Ready')(page);
    await page.evaluate(() => { window.__ALLOWANCE_WATCH__.statuses = []; window.__ALLOWANCE_FIXTURE__.setCap(400000); });
    await agent(page).locator('.agent-usage', { hasText: 'of 400,000' }).waitFor();
    await status('Ready')(page);
    checks.line = await line(page);
    checks.watched = await page.evaluate(() => structuredClone(window.__ALLOWANCE_WATCH__));
  });

  return results;
}
