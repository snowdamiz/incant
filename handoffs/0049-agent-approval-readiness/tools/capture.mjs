#!/usr/bin/env node
/**
 * BROWSER review of the approval readiness hold, handoff 0049.
 *
 * Serves a built editor UI with `vite preview` on a private 127.0.0.1 port, drives the
 * Chromium that matches the pinned playwright-core (or CHROME_PATH) with real keyboard and
 * pointer input, and installs, before the page's modules run:
 *   1. handoff 0046's `approval` scenario: turn 3 awaits approval 4, and the session's
 *      allowance equals the project's (250,000);
 *   2. handoff 0046's tools/fake-host.js, unchanged: scripted host IPC;
 *   3. tools/readiness-host.js (this handoff): held Undo/Redo replies, held session reads
 *      and recorded decisions;
 *   4. the real compiled bridge, editor/bridge/native.generated.js, unchanged;
 *   5. a relabel of that bridge as a fixture, so nothing claims to be live.
 * DETERMINISTIC FIXTURE DATA: no provider, account, network, engine or native window is
 * used, and nothing here is native WebKit evidence.
 *
 * Usage (worktree root, after building the UI):
 *   node handoffs/0049-agent-approval-readiness/tools/capture.mjs <set> [--root=<dist dir>]
 * Writes PNGs and report.json to a NEW, git-ignored handoffs/0049-agent-approval-readiness/artifacts/<set>/.
 */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import { preview } from 'vite';
import { SCENARIOS } from '../../0046-project-agent-budget/tools/scenarios.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const uiRoot = join(repoRoot, 'editor/ui');
const args = process.argv.slice(2);
const set = args.find((a) => !a.startsWith('--'));
if (!set || !/^[a-z0-9-]+$/.test(set)) throw new Error('Usage: capture.mjs <set-name> [--root=<dist dir>]');
const rootFlag = args.find((a) => a.startsWith('--root='))?.slice('--root='.length);
const distRoot = resolve(rootFlag ?? join(uiRoot, 'dist'));
const outDir = join(here, '../artifacts', set);
if (existsSync(outDir)) throw new Error(`${outDir} exists; choose a new set name.`);
if (!existsSync(join(distRoot, 'index.html'))) throw new Error(`No UI build at ${distRoot}.`);
const axeSource = readFileSync(createRequire(join(uiRoot, 'package.json')).resolve('axe-core/axe.min.js'), 'utf8');
const hostScript = readFileSync(join(repoRoot, 'handoffs/0046-project-agent-budget/tools/fake-host.js'), 'utf8');
const readinessScript = readFileSync(join(here, 'readiness-host.js'), 'utf8');
const bridgeScript = readFileSync(join(repoRoot, 'editor/bridge/native.generated.js'), 'utf8');
const RELABEL = `(() => {
  const bridge = window.__INCANT_BRIDGE__;
  if (!bridge) throw new Error('Readiness fixture: the compiled bridge did not install.');
  bridge.label = 'Readiness fixture: scripted host IPC, not live';
  bridge.isFixture = true;
})();`;
const REFUSED = 'Connect an OpenAI account and refresh the native session before deciding an approval.';
const DRAFT = 'Then add fog over the water.';

/** Window size and Inspector column width. */
const SIZES = {
  'w1440-s380': { width: 1440, height: 900, side: 380 },
  'w1440-s280': { width: 1440, height: 900, side: 280 },
};

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

const agent = (page) => page.locator('[data-region="agent"]');
const decision = (page, name) => agent(page).locator('.agent-approval__actions').getByRole('button', { name });
const sideClip = async (page) => page.locator('.column--side').boundingBox();
const snapshotAgent = (page) => page.evaluate(() => window.__INCANT_BRIDGE__.getSnapshot().agent);
const waitReady = (page, ready) =>
  page.waitForFunction((value) => window.__INCANT_BRIDGE__.getSnapshot().agent.approvalReady === value, ready);
const decideCalls = (page) => page.evaluate(() => window.__ALLOWANCE_FIXTURE__.calls.filter((call) => call.name === 'engine_agent_decide'));

async function setSeparator(page, name, target) {
  const separator = page.getByRole('separator', { name });
  const current = Number(await separator.getAttribute('aria-valuenow'));
  if (current === target) return;
  const box = await separator.boundingBox();
  const vertical = (await separator.getAttribute('aria-orientation')) === 'vertical';
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  // The controlled pane sits after the separator.
  const delta = current - target;
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(vertical ? x + delta : x, vertical ? y : y + delta, { steps: 4 });
  await page.mouse.up();
  await page.mouse.move(page.viewportSize().width * 0.45, page.viewportSize().height * 0.3);
  const now = Number(await separator.getAttribute('aria-valuenow'));
  if (now !== target) throw new Error(`${name}: wanted ${target}, got ${now}`);
}

/** What the reader sees and what the bridge says, measured in the page. */
function measure(refused) {
  const round = (n) => Math.round(n * 10) / 10;
  const rect = (el) => {
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return { x: round(r.x), y: round(r.y), width: round(r.width), height: round(r.height) };
  };
  const pane = document.querySelector('[data-region="agent"]');
  const card = pane.querySelector('.agent-approval');
  const note = card?.querySelector('.agent-approval__note') ?? null;
  const state = (button) => (button ? button.getAttribute('aria-disabled') : 'missing');
  const actions = card ? [...card.querySelectorAll('.agent-approval__actions button')] : [];
  const named = (name) => actions.find((button) => button.textContent.trim() === name) ?? null;
  const paneRect = pane.getBoundingClientRect();
  const escaping = [...pane.querySelectorAll('*')]
    .filter((el) => {
      const r = el.getBoundingClientRect();
      if (r.width === 0 || r.height === 0) return false;
      for (let parent = el.parentElement; parent && parent !== pane; parent = parent.parentElement) {
        const style = getComputedStyle(parent);
        if (/(auto|scroll|hidden|clip)/.test(style.overflowX + style.overflowY)) return false;
      }
      return r.right > paneRect.right + 0.5 || r.left < paneRect.left - 0.5 || r.bottom > paneRect.bottom + 0.5;
    })
    .map((el) => String(el.className) || el.tagName);
  const horizontal = [...pane.querySelectorAll('*')]
    .filter((el) => /(auto|scroll)/.test(getComputedStyle(el).overflowX) && el.scrollWidth > el.clientWidth + 1)
    .map((el) => String(el.className) || el.tagName);
  const active = document.activeElement;
  const snapshot = window.__INCANT_BRIDGE__.getSnapshot();
  return {
    bridge: { agent: snapshot.agent, projectCap: snapshot.projectAgentBudget?.maxTokens ?? null, sessionCap: snapshot.agentSession?.budget.max_tokens ?? null, applied: snapshot.history?.applied ?? null },
    status: pane.querySelector('.agent__status')?.textContent?.replace(/^,\s*/, '').trim() ?? null,
    note: note?.textContent ?? null,
    noteIsError: note ? note.classList.contains('is-error') : null,
    deny: state(named('Deny')),
    allow: state(named('Allow')),
    stop: state(pane.querySelector('.send-button--stop')),
    refusedShown: document.body.innerText.includes(refused),
    draft: pane.querySelector('.composer__input')?.value ?? null,
    replyShown: pane.querySelector('.agent-turn__reply') !== null,
    geometry: { pane: rect(pane), transcript: rect(pane.querySelector('.agent__transcript')), card: rect(card), footer: rect(card?.querySelector('.agent-approval__decision')), note: rect(note) },
    escaping,
    horizontalScrollers: horizontal,
    active: active ? { tag: active.tagName, text: active.textContent?.trim().slice(0, 60) ?? '', focusVisible: active.matches(':focus-visible') || active.hasAttribute('data-focus-visible') } : null,
    decideCalls: window.__ALLOWANCE_FIXTURE__.calls.filter((call) => call.name === 'engine_agent_decide'),
  };
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

const server = await preview({ root: uiRoot, build: { outDir: distRoot }, preview: { port: await freePort(47490), strictPort: true, host: '127.0.0.1' }, logLevel: 'error' });
const origin = server.resolvedUrls.local[0].replace(/\/$/, '');
const executablePath = process.env.CHROME_PATH && existsSync(process.env.CHROME_PATH) ? process.env.CHROME_PATH : undefined;
const browser = await chromium.launch({ headless: true, ...(executablePath ? { executablePath } : {}) });
mkdirSync(outDir, { recursive: true });
const report = {
  set,
  generated: new Date().toISOString(),
  browser: browser.version(),
  dist: distRoot,
  data: 'deterministic fixture (0046 fake-host.js + 0049 readiness-host.js + real native.generated.js); not live, not native WebKit',
  captures: [],
  flows: [],
};

async function open(size, { deviceScaleFactor = 2 } = {}) {
  const context = await browser.newContext({ viewport: { width: size.width, height: size.height }, deviceScaleFactor, colorScheme: 'dark', reducedMotion: 'reduce' });
  const page = await context.newPage();
  const errors = [];
  const requests = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  page.on('request', (r) => {
    if (!r.url().startsWith(origin)) requests.push(r.url());
  });
  await page.addInitScript({ content: `window.__ALLOWANCE_FIXTURE_CONFIG__ = ${JSON.stringify(SCENARIOS.approval)};` });
  await page.addInitScript({ content: hostScript });
  await page.addInitScript({ content: readinessScript });
  await page.addInitScript({ content: bridgeScript });
  await page.addInitScript({ content: RELABEL });
  await page.goto(`${origin}/`, { waitUntil: 'networkidle' });
  await page.locator('.titlebar').waitFor();
  await setSeparator(page, 'Resize inspector and agent', size.side);
  await agent(page).locator('.agent__status', { hasText: 'Needs approval' }).waitFor();
  await waitReady(page, true);
  return { context, page, errors, requests };
}

const startHistory = async (page, name) => {
  await page.evaluate(() => {
    window.__READINESS__.holdHistory = true;
    window.__READINESS__.holdReads = true;
  });
  await page.getByRole('button', { name }).click();
  await waitReady(page, false);
};
const releaseHistory = async (page, applied) => {
  await page.evaluate(() => window.__READINESS__.release('history'));
  await page.waitForFunction((value) => window.__INCANT_BRIDGE__.getSnapshot().history.applied === value, applied);
  // The fresh session read is held; the bridge must still refuse decisions.
  await page.waitForFunction(() => window.__READINESS__.heldReads > 0);
  await page.waitForTimeout(300);
};
const releaseReads = async (page) => {
  await page.evaluate(() => window.__READINESS__.release('reads'));
  await waitReady(page, true);
  await page.waitForTimeout(200);
};

try {
  /* ---- Captures: pending, pressed while pending, the session read, recovered ---- */
  for (const [sizeKey, size] of Object.entries(SIZES)) {
    const { context, page, errors, requests } = await open(size);
    const capture = async (state, extra = {}) => {
      await page.mouse.move(size.width * 0.45, size.height * 0.3);
      await page.waitForTimeout(350);
      const file = `${state}-${sizeKey}`;
      await page.screenshot({ path: join(outDir, `${file}-side.png`), clip: await sideClip(page) });
      const measured = await page.evaluate(measure, REFUSED);
      const axe = extra.axe ? await axeOn(page) : null;
      report.captures.push({ file, state, size: sizeKey, ...size, measured, axe });
      console.log(`${file}: status=${measured.status} ready=${measured.bridge.agent.approvalReady} allow=${measured.allow} note=${JSON.stringify(measured.note)} refused=${measured.refusedShown} decide=${measured.decideCalls.length}${axe ? ` axe=${axe.length}` : ''}`);
    };
    let stepError = null;
    try {
      await page.getByLabel('Message to the agent').fill(DRAFT);
      await startHistory(page, /^Undo/);
      await capture('pending', { axe: true });
      // A pointer press on each decision while held. Playwright will not click an
      // aria-disabled control unless forced; a person can.
      await decision(page, 'Deny').click({ force: true });
      await decision(page, 'Allow').click({ force: true });
      await page.waitForTimeout(300);
      await capture('pressed');
      await releaseHistory(page, 1);
      await capture('reading');
      await releaseReads(page);
      await capture('recovered', { axe: true });
    } catch (error) {
      stepError = String(error);
      console.log(`${sizeKey}: STEP ERROR ${stepError}`);
    }
    report.captures.filter((c) => c.size === sizeKey).forEach((c) => Object.assign(c, { errors, requests, stepError }));
    await context.close();
  }

  /* ---- Flows: real keyboard input through the real bridge ---- */
  const run = async (name, sizeKey, body) => {
    const size = SIZES[sizeKey];
    const { context, page, errors, requests } = await open(size, { deviceScaleFactor: 2 });
    const result = { name, size: sizeKey, errors, requests, checks: {}, error: null };
    try {
      await body(page, result.checks, size);
    } catch (error) {
      result.error = String(error);
    }
    result.calls = await page.evaluate(() =>
      window.__ALLOWANCE_FIXTURE__.calls.filter((call) => !['engine_agent_read', 'provider_read', 'viewport_bounds', 'window_read', 'play_read', 'engine_read'].includes(call.name)),
    );
    report.flows.push(result);
    console.log(`flow ${name}: ${result.error ?? 'ok'}`);
    await context.close();
  };
  const active = (page) =>
    page.evaluate(() => {
      const el = document.activeElement;
      return { tag: el?.tagName, text: el?.textContent?.trim().slice(0, 80), focusVisible: !!el && (el.matches(':focus-visible') || el.hasAttribute('data-focus-visible')) };
    });
  const look = (page) => page.evaluate(measure, REFUSED);
  const shot = async (page, file, size) => {
    await page.mouse.move(size.width * 0.45, size.height * 0.3);
    await page.waitForTimeout(300);
    await page.screenshot({ path: join(outDir, `${file}.png`), clip: await sideClip(page) });
  };

  // Keyboard only on the decision: Enter and Space while held through the history reply and
  // the session read, then Enter after recovery sends the one real decision.
  await run('keyboard-undo', 'w1440-s380', async (page, checks, size) => {
    await page.getByLabel('Message to the agent').fill(DRAFT);
    await startHistory(page, /^Undo/);
    await page.getByLabel('Message to the agent').click();
    await page.keyboard.press('Shift+Tab');
    checks.focusedHeld = await active(page);
    await page.keyboard.press('Enter');
    await page.waitForTimeout(200);
    await page.keyboard.press('Space');
    await page.waitForTimeout(300);
    checks.afterKeysWhileHistoryHeld = await look(page);
    await shot(page, 'flow-keyboard-held', size);
    await releaseHistory(page, 1);
    await page.keyboard.press('Enter');
    await page.waitForTimeout(300);
    checks.afterEnterWhileReadHeld = await look(page);
    await releaseReads(page);
    checks.recovered = await look(page);
    await page.keyboard.press('Enter');
    await agent(page).locator('.agent-turn').last().locator('.agent-decision--allowed').waitFor();
    await page.waitForTimeout(300);
    checks.decided = await look(page);
    checks.decideCalls = await decideCalls(page);
    await shot(page, 'flow-keyboard-decided', size);
  });

  // Redo holds the same way at the narrow width; Stop stays usable through the hold.
  await run('redo-then-stop', 'w1440-s280', async (page, checks, size) => {
    await page.evaluate(() => window.__READINESS__.release('history'));
    await page.getByRole('button', { name: /^Undo/ }).click();
    await page.waitForFunction(() => window.__INCANT_BRIDGE__.getSnapshot().history.applied === 1);
    await waitReady(page, true);
    await startHistory(page, /^Redo/);
    await page.getByLabel('Message to the agent').click();
    await page.keyboard.press('Shift+Tab');
    checks.focusedHeld = await active(page);
    await page.keyboard.press('Enter');
    await page.waitForTimeout(300);
    checks.afterEnterWhileRedoHeld = await look(page);
    await shot(page, 'flow-redo-held', size);
    const stop = agent(page).getByRole('button', { name: 'Stop' });
    await stop.focus();
    checks.stopFocused = await active(page);
    await page.keyboard.press('Enter');
    await page.waitForFunction(() => window.__ALLOWANCE_FIXTURE__.calls.some((call) => call.name === 'engine_agent_cancel'));
    await agent(page).locator('.agent__status', { hasText: 'Stopping…' }).waitFor();
    checks.afterStop = await look(page);
    checks.agent = await snapshotAgent(page);
  });
} finally {
  await browser.close();
  await new Promise((done) => server.httpServer.close(done));
}
writeFileSync(join(outDir, 'report.json'), `${JSON.stringify(report, null, 2)}\n`);
console.log(`Wrote ${report.captures.length} captures and ${report.flows.length} flows to ${outDir}`);
