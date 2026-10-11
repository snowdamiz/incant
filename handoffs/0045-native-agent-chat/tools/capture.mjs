#!/usr/bin/env node
/**
 * BROWSER captures and flows for the Agent pane, handoff 0045.
 *
 * Serves editor/ui/dist with `vite preview` on the first free port from 4245, drives local
 * Chrome through the pinned playwright-core, and installs, before the page's modules run:
 *   1. the scenario's configuration,
 *   2. tools/fake-host.js, a scripted stand-in for the native host's IPC (window.__TAURI__),
 *   3. the real compiled bridge, editor/bridge/native.generated.js, unchanged,
 *   4. a relabel of that bridge as a fixture, so every capture says it is not live.
 * DETERMINISTIC FIXTURE DATA: no provider, account, network or engine is used, and nothing
 * here is native WebKit evidence. PNGs and report.json go to a NEW ignored directory
 * artifacts/0045-native-agent-chat/<set>/ (an existing one is refused).
 *
 * Usage (repo root, after the UI build):
 *   node handoffs/0045-native-agent-chat/tools/capture.mjs <set> [--only=key,key] [--sizes=a,b] [--flows-only] [--no-flows]
 */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import { preview } from 'vite';
import { SCENARIOS, SIGNED_OUT } from './scenarios.mjs';

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
const outDir = join(repoRoot, 'artifacts/0045-native-agent-chat', set);
if (existsSync(outDir)) throw new Error(`${outDir} exists; choose a new set name.`);
if (!existsSync(join(uiRoot, 'dist/index.html'))) throw new Error('Run the UI build first.');
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find((p) => p && existsSync(p));
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');
const axeSource = readFileSync(createRequire(join(uiRoot, 'package.json')).resolve('axe-core/axe.min.js'), 'utf8');
const hostScript = readFileSync(join(here, 'fake-host.js'), 'utf8');
const bridgeScript = readFileSync(join(repoRoot, 'editor/bridge/native.generated.js'), 'utf8');
const RELABEL = `(() => {
  const bridge = window.__INCANT_BRIDGE__;
  if (!bridge) throw new Error('Agent fixture: the compiled bridge did not install.');
  bridge.label = 'Agent fixture: scripted host IPC, not live';
  bridge.isFixture = true;
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
const typeDraft = (text) => async (page) => {
  await page.getByLabel('Message to the agent').fill(text);
};

/** Static states: the session the fixture serves, what to wait for, and steps before capture. */
const STATES = [
  { key: 'ready-empty', scenario: 'ready-empty', ready: status('Ready'), sizes: ALL },
  { key: 'ready-draft', scenario: 'ready-empty', ready: status('Ready'), steps: [typeDraft('Hang a second lantern on the mooring post and point the close-up camera at it.')], sizes: ['w1440-s280', 'w1000-s280'] },
  { select: 'Lantern', key: 'streaming', scenario: 'streaming', ready: status('Working…'), sizes: ALL, window: ['w1440-s380', 'w1000-s280'] },
  { select: 'Lantern', key: 'approval', scenario: 'approval', ready: status('Needs approval'), sizes: ALL, window: ['w1440-s380', 'w1000-s280'] },
  {
    key: 'approval-arguments',
    scenario: 'approval',
    ready: status('Needs approval'),
    steps: [
      async (page) => {
        const disclosure = agent(page).getByRole('button', { name: /Exact arguments/ });
        await disclosure.focus();
        await page.keyboard.press('Enter');
        await page.keyboard.press('Tab');
      },
    ],
    sizes: ['w1440-s380', 'w1440-s280', 'w1000-s280'],
    agentHeight: 520,
  },
  { key: 'approval-always', scenario: 'approval-always', ready: status('Needs approval'), sizes: ['w1440-s380', 'w1000-s280'] },
  { key: 'approval-signed-out', scenario: 'approval-signed-out', ready: status('Needs approval'), sizes: ['w1440-s380', 'w1000-s280'], agentHeight: 520 },
  {
    key: 'approval-deciding',
    scenario: 'approval-slow',
    ready: status('Needs approval'),
    steps: [
      async (page) => {
        const allow = agent(page).getByRole('button', { name: 'Allow' });
        await allow.focus();
        await page.keyboard.press('Enter');
        await agent(page).getByRole('button', { name: 'Allowing…' }).waitFor();
      },
    ],
    sizes: ['w1440-s380', 'w1000-s280'],
    agentHeight: 520,
  },
  { key: 'history', scenario: 'history', ready: status('Ready'), sizes: ALL, agentHeight: 520 },
  { select: 'Lantern', key: 'history-default', scenario: 'history', ready: status('Ready'), sizes: ['w1440-s380', 'w1000-s280'] },
  { key: 'stopping', scenario: 'stopping', ready: status('Stopping…'), sizes: ['w1440-s380', 'w1000-s280'] },
  { key: 'finishing', scenario: 'finishing', ready: status('Finishing…'), sizes: ['w1440-s380', 'w1000-s280'] },
  { key: 'exhausted', scenario: 'exhausted', ready: status('Unavailable'), sizes: ['w1440-s380', 'w1440-s280', 'w1000-s280'] },
  { key: 'signed-out-history', scenario: 'signed-out', ready: status('Not signed in'), sizes: ['w1440-s380', 'w1440-s280', 'w1000-s280'] },
  {
    key: 'transport-error',
    scenario: 'history',
    ready: status('Ready'),
    steps: [
      async (page) => {
        await page.evaluate(() => {
          const fixture = window.__AGENT_FIXTURE__;
          fixture.readError = 'fixture transport failure (raw text the UI must never show)';
          fixture.setProvider(fixture.provider);
        });
        await status('Unavailable')(page);
      },
    ],
    sizes: ['w1440-s380', 'w1440-s280', 'w1000-s280'],
  },
  {
    key: 'transport-error-running',
    scenario: 'streaming',
    ready: status('Working…'),
    steps: [
      async (page) => {
        await page.evaluate(() => {
          window.__AGENT_FIXTURE__.readError = 'fixture transport failure (raw text the UI must never show)';
        });
        await agent(page).locator('.agent-notice').waitFor();
      },
    ],
    sizes: ['w1440-s380', 'w1000-s280'],
  },
  {
    key: 'long',
    scenario: 'long',
    ready: status('Needs approval'),
    steps: [
      async (page) => {
        await agent(page).getByRole('button', { name: /Exact arguments/ }).click();
      },
    ],
    sizes: ['w1440-s380', 'w1440-s280', 'w1000-s280'],
    agentHeight: 560,
  },
  {
    key: 'long-top',
    scenario: 'long',
    ready: status('Needs approval'),
    steps: [async (page) => agent(page).locator('.agent__transcript').evaluate((el) => { el.scrollTop = 0; })],
    sizes: ['w1440-s280'],
    agentHeight: 560,
  },
  {
    key: 'model-limit',
    scenario: 'ready-empty',
    ready: status('Ready'),
    steps: [
      typeDraft('Hang a second lantern on the mooring post.'),
      // 44 three-byte characters: within the field's 128-character cap, over the bridge's 128-byte limit.
      async (page) => agent(page).getByRole('textbox', { name: 'Model' }).fill('模型'.repeat(22)),
    ],
    sizes: ['w1440-s380', 'w1000-s280'],
  },
  {
    key: 'send-failed',
    scenario: 'send-fails',
    ready: status('Ready'),
    steps: [
      typeDraft('Hang a second lantern on the mooring post.'),
      async (page) => {
        await page.getByLabel('Message to the agent').press('Meta+Enter');
        await status('Unavailable')(page);
      },
    ],
    sizes: ['w1440-s380', 'w1000-s280'],
  },
  {
    key: 'collapsed-approval',
    scenario: 'approval',
    ready: status('Needs approval'),
    steps: [async (page) => agent(page).getByRole('button', { name: /^Agent/ }).click()],
    sizes: ['w1440-s380', 'w1440-s280'],
    keepCollapsed: true,
  },
  {
    key: 'many-scrolled',
    scenario: 'many',
    ready: status('Working…'),
    steps: [
      async (page) => {
        await agent(page).locator('.agent__transcript').evaluate((el) => { el.scrollTop = 0; });
        await page.evaluate(() => window.__AGENT_FIXTURE__.change((s) => { s.turns.at(-1).text += ' its near plane, so the lantern should be in view.'; }));
        await agent(page).locator('.agent-jump').waitFor();
      },
    ],
    sizes: ['w1440-s380', 'w1000-s280'],
  },
  {
    key: 'auto-mode',
    scenario: 'history',
    ready: status('Ready'),
    steps: [async (page) => agent(page).getByRole('combobox', { name: 'Approval' }).selectOption('auto')],
    sizes: ['w1440-s380', 'w1440-s280'],
  },
  {
    key: 'focus-model',
    scenario: 'ready-empty',
    ready: status('Ready'),
    steps: [
      async (page) => {
        await page.getByLabel('Message to the agent').focus();
        await page.keyboard.press('Tab');
      },
    ],
    sizes: ['w1440-s280'],
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
      return r.right > paneRect.right + 0.5 || r.left < paneRect.left - 0.5;
    })
    .map((el) => el.className || el.tagName);
  const horizontal = [...pane.querySelectorAll('*')].filter((el) => {
    const style = getComputedStyle(el);
    return /(auto|scroll)/.test(style.overflowX) && el.scrollWidth > el.clientWidth + 1;
  }).map((el) => el.className || el.tagName);
  const active = document.activeElement;
  return {
    window: { width: innerWidth, height: innerHeight, scrollX, scrollY, scrollWidth: document.documentElement.scrollWidth, scrollHeight: document.documentElement.scrollHeight },
    side: rect(side),
    pane: rect(pane),
    status: pane.querySelector('.agent__status')?.textContent?.replace(/^,\s*/, '').trim() ?? null,
    transcript: transcript ? { ...rect(transcript), scrollHeight: transcript.scrollHeight, clientHeight: transcript.clientHeight, scrollTop: transcript.scrollTop, tabIndex: transcript.getAttribute('tabindex') } : null,
    composer: rect(pane.querySelector('.composer')),
    note: pane.querySelector('.agent-note__body')?.textContent ?? null,
    escaping,
    horizontalScrollers: horizontal,
    active: active ? { tag: active.tagName, className: active.className, name: active.getAttribute('aria-label') ?? active.textContent?.trim().slice(0, 60), focusVisible: active.matches(':focus-visible') || active.hasAttribute('data-focus-visible') } : null,
    words: (pane.innerText.match(/\S+/g) ?? []).length,
    rawLeak: document.body.innerText.includes('raw text the UI must never show'),
  };
}

const server = await preview({ root: uiRoot, preview: { port: await freePort(4245), strictPort: true, host: '127.0.0.1' }, logLevel: 'error' });
const origin = server.resolvedUrls.local[0].replace(/\/$/, '');
const browser = await chromium.launch({ executablePath, headless: true });
mkdirSync(outDir, { recursive: true });
const report = { set, generated: new Date().toISOString(), browser: browser.version(), data: 'deterministic agent fixture (tools/fake-host.js + real native.generated.js); not live, not native WebKit', captures: [], flows: [] };

async function open(scenarioKey, size, { deviceScaleFactor = 2 } = {}) {
  const context = await browser.newContext({ viewport: { width: size.width, height: size.height }, deviceScaleFactor, colorScheme: 'dark', reducedMotion: 'reduce' });
  const page = await context.newPage();
  const errors = [];
  const requests = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  page.on('request', (r) => { if (!r.url().startsWith(origin)) requests.push(r.url()); });
  await page.addInitScript({ content: `window.__AGENT_FIXTURE_CONFIG__ = ${JSON.stringify(SCENARIOS[scenarioKey])};` });
  await page.addInitScript({ content: hostScript });
  await page.addInitScript({ content: bridgeScript });
  await page.addInitScript({ content: RELABEL });
  await page.goto(`${origin}/`, { waitUntil: 'networkidle' });
  await page.locator('.titlebar').waitFor();
  return { context, page, errors, requests };
}

async function layout(page, size, state) {
  if (state.select) await page.getByRole('treeitem', { name: new RegExp(`^${state.select}`) }).first().click();
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
        console.log(`${file}: ${stepError ? `STEP ERROR ${stepError}` : 'ok'} axe=${axe.length} errors=${errors.length} escaping=${measured.escaping.length}`);
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
    result.calls = await page.evaluate(() => window.__AGENT_FIXTURE__.calls.filter((call) => call.name.startsWith('engine_agent_') && call.name !== 'engine_agent_read'));
    results.push(result);
    console.log(`flow ${name}: ${result.error ?? 'ok'}`);
    await context.close();
  };
  const active = (page) => page.evaluate(() => {
    const el = document.activeElement;
    return { tag: el?.tagName, className: el?.className, text: el?.textContent?.trim().slice(0, 80), label: el?.getAttribute('aria-label') ?? null };
  });
  const size = SIZES['w1440-s380'];
  const SIGNED_SHORT = SIZES['w1000-s280'];

  // Keyboard only: type, Mod+Enter (twice, quickly), review, Allow; focus lands on the decision.
  await run('send-approve', 'flow-send', size, async (page, checks) => {
    await status('Ready')(page);
    const input = page.getByLabel('Message to the agent');
    await input.focus();
    await page.keyboard.type('Make the lantern visible from the close-up camera.');
    await page.keyboard.press('Meta+Enter');
    await page.keyboard.press('Meta+Enter');
    checks.draftAfterSend = await input.inputValue();
    await status('Needs approval')(page);
    checks.focusWhileWaiting = await active(page);
    checks.headerWhileWaiting = await agent(page).locator('.agent__status').innerText();
    const allow = agent(page).getByRole('button', { name: 'Allow' });
    await allow.focus();
    await page.keyboard.press('Enter');
    await agent(page).locator('.agent-decision--allowed').waitFor();
    await page.waitForTimeout(100);
    checks.focusAfterAllow = await active(page);
    await agent(page).locator('.agent-outcome', { hasText: '1 edit' }).waitFor();
    checks.outcome = await agent(page).locator('.agent-turn').last().locator('.agent-outcome').innerText();
    checks.statusLine = await page.locator('.statusbar__message').innerText();
    await agent(page).getByRole('button', { name: 'Show in History' }).click();
    await page.waitForTimeout(150);
    checks.afterHistory = await active(page);
    checks.historyTabSelected = await page.getByRole('tab', { name: /^History/ }).getAttribute('aria-selected');
    await page.screenshot({ path: join(outDir, 'flow-send-approve-window.png') });
  });

  // Stop from the keyboard; the same button keeps focus through Stopping… and back to Send.
  await run('stop', 'flow-stop', size, async (page, checks) => {
    await status('Ready')(page);
    const input = page.getByLabel('Message to the agent');
    await input.fill('Check every prop group.');
    await input.press('Meta+Enter');
    await status('Working…')(page);
    await page.waitForTimeout(600);
    const stop = agent(page).getByRole('button', { name: 'Stop' });
    await stop.focus();
    await page.keyboard.press('Enter');
    await status('Stopping…')(page);
    checks.focusWhileStopping = await active(page);
    await agent(page).locator('.agent-outcome__status', { hasText: 'Stopped' }).waitFor();
    checks.headerAfterCancelled = await agent(page).locator('.agent__status').innerText();
    await status('Ready')(page);
    checks.focusWhenIdle = await active(page);
    checks.outcome = await agent(page).locator('.agent-turn').last().locator('.agent-outcome').innerText();
  });

  // The provider disconnects while a worker runs: Stop stays usable and is sent with the turn's ID.
  await run('stop-signed-out', 'streaming', size, async (page, checks) => {
    await status('Working…')(page);
    await page.evaluate((provider) => window.__AGENT_FIXTURE__.setProvider(provider), SIGNED_OUT);
    await page.waitForTimeout(300);
    checks.headerSignedOut = await agent(page).locator('.agent__status').innerText();
    const stop = agent(page).getByRole('button', { name: 'Stop' });
    checks.stopDisabled = await stop.getAttribute('aria-disabled');
    await stop.click();
    await status('Stopping…')(page);
    await page.screenshot({ path: join(outDir, 'flow-stop-signed-out-side.png'), clip: await page.locator('.column--side').boundingBox() });
  });

  // A transport failure keeps the history; Retry recovers it.
  await run('retry', 'history', size, async (page, checks) => {
    await status('Ready')(page);
    await page.evaluate(() => {
      const fixture = window.__AGENT_FIXTURE__;
      fixture.readError = 'fixture transport failure (raw text the UI must never show)';
      fixture.setProvider(fixture.provider);
    });
    await status('Unavailable')(page);
    checks.turnsDuringError = await agent(page).locator('.agent-turn').count();
    checks.note = await agent(page).locator('.agent-note__body').innerText();
    checks.rawLeak = await page.evaluate(() => document.body.innerText.includes('raw text the UI must never show'));
    await page.evaluate(() => { window.__AGENT_FIXTURE__.readError = null; });
    const retry = agent(page).getByRole('button', { name: 'Retry' });
    await retry.focus();
    await page.keyboard.press('Enter');
    await status('Ready')(page);
    checks.turnsAfterRetry = await agent(page).locator('.agent-turn').count();
    checks.focusAfterRetry = await active(page);
    await page.waitForTimeout(150);
    checks.statusLine = await page.locator('.statusbar__message').innerText();
  });

  // Reading earlier turns: new output does not move the reader; Latest returns to the end.
  await run('scroll', 'flow-many', size, async (page, checks) => {
    await status('Working…')(page);
    const log = agent(page).locator('.agent__transcript');
    checks.initial = await log.evaluate((el) => ({ top: el.scrollTop, end: el.scrollHeight - el.clientHeight }));
    await log.evaluate((el) => { el.scrollTop = 120; });
    await page.waitForTimeout(100);
    for (let n = 0; n < 3; n += 1) {
      await page.evaluate((i) => window.__AGENT_FIXTURE__.change((s) => { s.turns.at(-1).text += ` More output ${i}.`; }), n);
      await page.waitForTimeout(400);
    }
    checks.afterOutput = await log.evaluate((el) => ({ top: el.scrollTop, end: el.scrollHeight - el.clientHeight }));
    checks.jumpShown = await agent(page).locator('.agent-jump').count();
    await agent(page).locator('.agent-jump').click();
    await page.waitForTimeout(100);
    checks.afterJump = await log.evaluate((el) => ({ top: el.scrollTop, end: el.scrollHeight - el.clientHeight }));
    checks.focusAfterJump = await active(page);
    await page.evaluate(() => window.__AGENT_FIXTURE__.change((s) => { s.turns.at(-1).text += ' Following again.'; }));
    await page.waitForTimeout(400);
    checks.followingAgain = await log.evaluate((el) => ({ top: el.scrollTop, end: el.scrollHeight - el.clientHeight }));
  });

  // A send the host refuses keeps the draft; Retry brings the composer back with it.
  await run('send-failed', 'send-fails', size, async (page, checks) => {
    await status('Ready')(page);
    await page.getByLabel('Message to the agent').fill('Hang a second lantern on the mooring post.');
    await page.getByLabel('Message to the agent').press('Meta+Enter');
    await status('Unavailable')(page);
    checks.note = await agent(page).locator('.agent-note__body').innerText();
    await page.evaluate(() => { window.__AGENT_FIXTURE__.startError = null; });
    await agent(page).getByRole('button', { name: 'Retry' }).click();
    await status('Ready')(page);
    checks.draftAfterRetry = await page.getByLabel('Message to the agent').inputValue();
  });

  // Reading a review in the short 1000 x 650 pane: keyboard focus is never hidden under the
  // sticky decision row, expanding holds the view, and following resumes after the decision.
  await run('review-reading', 'flow-send', SIGNED_SHORT, async (page, checks) => {
    await status('Ready')(page);
    await page.getByLabel('Message to the agent').fill('Make the lantern visible from the close-up camera.');
    await page.getByLabel('Message to the agent').press('Meta+Enter');
    await status('Needs approval')(page);
    await page.waitForTimeout(300);
    const log = agent(page).locator('.agent__transcript');
    const geometry = () => page.evaluate(() => {
      const pane = document.querySelector('[data-region="agent"]');
      const scroller = pane.querySelector('.agent__transcript');
      const sticky = pane.querySelector('.agent-approval__decision');
      const active = document.activeElement;
      const box = (el) => (el ? (({ top, bottom }) => ({ top: Math.round(top), bottom: Math.round(bottom) }))(el.getBoundingClientRect()) : null);
      return { scrollTop: scroller.scrollTop, view: box(scroller), sticky: box(sticky), active: box(active), activeClass: active?.className ?? null, title: box(pane.querySelector('.agent-approval__title')) };
    });
    checks.onArrival = await geometry();
    const disclosure = agent(page).getByRole('button', { name: /Exact arguments/ });
    await disclosure.focus();
    await page.waitForTimeout(150);
    checks.disclosureFocused = await geometry();
    await page.keyboard.press('Enter');
    await page.waitForTimeout(400);
    checks.afterExpand = await geometry();
    await page.keyboard.press('Tab');
    await page.waitForTimeout(150);
    checks.argumentsFocused = await geometry();
    await page.waitForTimeout(400);
    checks.argumentsStill = await geometry();
    const allow = agent(page).getByRole('button', { name: 'Allow' });
    await allow.focus();
    await page.keyboard.press('Enter');
    await agent(page).locator('.agent-outcome', { hasText: '1 edit' }).waitFor();
    await page.waitForTimeout(400);
    checks.afterDecision = await log.evaluate((el) => ({ top: el.scrollTop, end: el.scrollHeight - el.clientHeight }));
    await page.screenshot({ path: join(outDir, 'flow-review-reading-side.png'), clip: await page.locator('.column--side').boundingBox() });
  });

  // Deny from the keyboard: the turn ends, and says the denial ended it.
  await run('deny', 'flow-send', size, async (page, checks) => {
    await status('Ready')(page);
    await page.getByLabel('Message to the agent').fill('Make the lantern visible from the close-up camera.');
    await page.getByLabel('Message to the agent').press('Meta+Enter');
    await status('Needs approval')(page);
    const deny = agent(page).getByRole('button', { name: 'Deny' });
    await deny.focus();
    await page.keyboard.press('Enter');
    await agent(page).locator('.agent-decision--denied').waitFor();
    await page.waitForTimeout(100);
    checks.focusAfterDeny = await active(page);
    await agent(page).locator('.agent-failure').waitFor();
    checks.failure = await agent(page).locator('.agent-failure').innerText();
  });

  return results;
}
