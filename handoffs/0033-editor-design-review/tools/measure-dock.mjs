#!/usr/bin/env node
/**
 * BROWSER measurements of the output dock with large Problems, Console and History lists
 * (handoff 0033 correctness follow-up): mounted rows, and the time from an input to
 * React's commit (DOM updated, effects run) for opening a tab, changing page, filtering,
 * and messages arriving while the Console follows the newest page.
 *
 * Data: an EVIDENCE TEST DOUBLE built in the page from review-double.mjs (real schemas
 * through the real compiled bridge conversion), with generated output lists:
 *   - many groups: 20,000 problems on 10,000 entities (2 each);
 *   - one group: 20,000 problems on one entity;
 *   - 100,000 console messages (a long session), then 10 arrivals of 500 more;
 *   - 50,000 history entries with 25,000 applied.
 * The bridge itself caps none of these lists today.
 *
 * Headless Chrome through the pinned playwright-core, unthrottled and with CDP CPU
 * throttling (4x). These are browser numbers on the machine that runs this script, NOT
 * the PLAN.md M1 MacBook Air device gate, and must not be reported as such.
 *
 * Usage (repo root, after the UI build):
 *   node handoffs/0033-editor-design-review/tools/measure-dock.mjs <set>
 */
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:net';
import os from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import { preview } from 'vite';
import { reviewSnapshot } from './review-double.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const uiRoot = join(repoRoot, 'editor/ui');
const set = process.argv[2];
if (!set || !/^[a-z0-9-]+$/.test(set)) throw new Error('Usage: measure-dock.mjs <set-name>');
const outDir = join(repoRoot, 'artifacts/0033-editor-design-review', `dock-${set}`);
if (existsSync(outDir)) throw new Error(`${outDir} exists; choose a new set name.`);
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find((p) => p && existsSync(p));
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');
if (!existsSync(join(uiRoot, 'dist/index.html'))) throw new Error('Run the UI build first.');

/** The double: the review snapshot with generated output lists; `__dock.arrive(n)` appends messages. */
const double = (layout) => `
  (() => {
    const base = ${JSON.stringify(reviewSnapshot())};
    const entityIds = Object.keys(base.entities);
    const layout = ${JSON.stringify(layout)};
    const diagnostics = [];
    for (let n = 0; n < 20000; n += 1) {
      const entity = layout === 'one-group' ? entityIds[0] : entityIds[n % entityIds.length] + (n >= entityIds.length * 2 ? '-' + Math.floor(n / 2) : '');
      diagnostics.push({ id: 'd' + n, severity: n % 5 === 0 ? 'warning' : 'error', message: 'Problem ' + n + ': collider dimensions must be 0.001..10000 meters for part ' + (n % 64), entity, component: 'Collider', path: '/shape/parts/' + (n % 64) + '/shape/half_extents/' + (n % 3) });
    }
    const message = (n) => ({ id: 'm' + n, level: n % 50 === 0 ? 'warn' : 'info', source: 'native', message: 'Message ' + n + ': source watch reloaded scenes/harbor.scene.ron', at: new Date(Date.UTC(2026, 9, 10, 12, 0, 0) + n * 10).toISOString() });
    const console_ = []; for (let n = 0; n < 100000; n += 1) console_.push(message(n));
    const entries = []; for (let n = 0; n < 50000; n += 1) entries.push({ transaction: 'tx' + n, description: 'Edit ' + n, origin: { kind: n % 7 === 0 ? 'agent' : 'user', ...(n % 7 === 0 ? { model: 'review-model', conversation: 'c' } : {}) }, at: new Date(Date.UTC(2026, 9, 10, 12, 0, 0) + n * 1000).toISOString() });
    let snapshot = { ...base, diagnostics, console: console_, history: { entries, applied: 25000 } };
    const listeners = new Set();
    window.__dock = {
      arrive: (count) => {
        const start = snapshot.console.length;
        const added = []; for (let n = 0; n < count; n += 1) added.push(message(start + n));
        snapshot = { ...snapshot, console: [...snapshot.console, ...added] };
        listeners.forEach((fn) => fn());
        return snapshot.console.length;
      },
    };
    window.__INCANT_BRIDGE__ = {
      protocolVersion: 1, capabilities: [], label: 'Dock scale test double (not engine)', isFixture: false,
      getSnapshot: () => snapshot,
      subscribe: (fn) => { listeners.add(fn); return () => listeners.delete(fn); },
      dispatch: () => Promise.resolve({ ok: false, error: { code: 'test-double', message: 'Read-only.' } }),
      request: () => Promise.resolve({ ok: false, error: { code: 'test-double', message: 'Read-only.' } }),
    };
  })();
`;

const HELPERS = `
  window.__longTasks = [];
  try {
    new PerformanceObserver((list) => { for (const e of list.getEntries()) window.__longTasks.push(Math.round(e.duration)); })
      .observe({ type: 'longtask', buffered: true });
  } catch {}
  window.__untilTask = (test, limit = 30000) => new Promise((done) => {
    const start = performance.now();
    const channel = new MessageChannel();
    channel.port1.onmessage = () => {
      if (test()) { done(performance.now() - start); return; }
      if (performance.now() - start > limit) { done(-1); return; }
      channel.port2.postMessage(0);
    };
    if (test()) done(0); else channel.port2.postMessage(0);
  });
  window.__range = (noun) => document.querySelector('[aria-label="Pages of ' + noun + '"] .pager__range')?.textContent ?? null;
  window.__click = (selector) => { const el = document.querySelector(selector); if (!el) throw new Error('missing ' + selector); el.click(); };
`;

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

const round = (n) => Math.round(n * 10) / 10;

/** Runs `act` in the page and returns the ms until `until` holds, plus mounted rows and the range. */
async function step(page, label, act, until, rows, noun) {
  const result = await page.evaluate(
    async ({ act, until, rows, noun }) => {
      window.__longTasks.length = 0;
      const start = performance.now();
      new Function(act)();
      const wait = await window.__untilTask(new Function(`return (${until})`));
      return {
        ms: wait < 0 ? -1 : performance.now() - start,
        mounted: document.querySelectorAll(rows).length,
        range: window.__range(noun),
        longTasks: [...window.__longTasks],
      };
    },
    { act, until, rows, noun },
  );
  return { label, ...result, ms: round(result.ms) };
}

async function scenario(browser, origin, layout, throttle) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 874 }, deviceScaleFactor: 1, colorScheme: 'dark' });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  await page.addInitScript(HELPERS);
  await page.addInitScript(double(layout));
  const cdp = await context.newCDPSession(page);
  if (throttle > 1) await cdp.send('Emulation.setCPUThrottlingRate', { rate: throttle });
  await page.goto(`${origin}/`, { waitUntil: 'load' });
  const steps = [];
  const first = await page.evaluate(async () => {
    const wait = await window.__untilTask(() => document.querySelectorAll('.problems__item').length > 0, 60000);
    return { ms: performance.now(), wait, mounted: document.querySelectorAll('.problems__item').length, range: window.__range('problems') };
  });
  steps.push({ label: 'Load to first Problems page (from navigation start)', ms: round(first.ms), mounted: first.mounted, range: first.range });
  const P = ['.problems__item', 'problems'];
  steps.push(await step(page, 'Problems: next page', "window.__click('[aria-label=\"Pages of problems\"] [aria-label=\"Next page\"]')", "window.__range('problems')?.startsWith('101–')", ...P));
  steps.push(await step(page, 'Problems: last page', "window.__click('[aria-label=\"Pages of problems\"] [aria-label=\"Last page\"]')", "window.__range('problems')?.startsWith('19,901–')", ...P));
  steps.push(await step(page, 'Problems: hide errors', "window.__click('.filter-chip--error')", "document.querySelector('.filter-chip--error')?.getAttribute('aria-pressed') === 'false' && window.__range('problems')?.startsWith('1–')", ...P));
  steps.push(await step(page, 'Problems: show errors', "window.__click('.filter-chip--error')", "window.__range('problems') === '1–100 of 20,000 problems'", ...P));
  if (layout === 'many-groups') {
    const C = ['.console__row', 'messages'];
    steps.push(await step(page, 'Open Console (100,000 messages)', "document.getElementById('dock-tab-console').click()", "window.__range('messages') !== null", ...C));
    steps.push(await step(page, 'Console: first page', "window.__click('[aria-label=\"Pages of messages\"] [aria-label=\"First page\"]')", "window.__range('messages')?.startsWith('1–')", ...C));
    steps.push(await step(page, 'Console: Newest', "window.__click('.pager__anchor')", "window.__range('messages')?.startsWith('99,901–')", ...C));
    for (let n = 1; n <= 10; n += 1) {
      const total = 100000 + n * 500;
      steps.push(await step(page, `Console: 500 messages arrive (${total.toLocaleString('en-US')} total), following`, 'window.__dock.arrive(500)', `window.__range('messages')?.endsWith('of ${total.toLocaleString('en-US')} messages')`, ...C));
    }
    const H = ['.history__row', 'edits'];
    steps.push(await step(page, 'Open History (50,000 edits, 25,000 applied)', "document.getElementById('dock-tab-history').click()", "window.__range('edits') !== null", ...H));
    steps.push(await step(page, 'History: first page', "window.__click('[aria-label=\"Pages of edits\"] [aria-label=\"First page\"]')", "window.__range('edits')?.startsWith('1–')", ...H));
    steps.push(await step(page, 'History: Now', "window.__click('.pager__anchor')", "window.__range('edits')?.startsWith('25,001–')", ...H));
    const now = await page.evaluate(() => Boolean(document.querySelector('.timeline__now')));
    steps.push({ label: 'History: Now marker on the shown page', ms: null, mounted: now ? 1 : 0, range: null });
  }
  await context.close();
  return { layout, throttle, errors, steps };
}

mkdirSync(outDir, { recursive: true });
const port = await freePort(4233);
const server = await preview({ root: uiRoot, preview: { port, strictPort: true, host: '127.0.0.1' }, logLevel: 'error' });
const origin = `http://127.0.0.1:${port}`;
const browser = await chromium.launch({ executablePath, headless: true });
const report = {
  set,
  generated: new Date().toISOString(),
  browser: browser.version(),
  machine: { cpu: os.cpus()[0]?.model ?? 'unknown', cores: os.cpus().length, memoryGB: Math.round(os.totalmem() / 2 ** 30), platform: `${os.platform()} ${os.release()}` },
  note: 'Browser numbers on this machine with an in-page test double; not the M1 MacBook Air device gate and not phone evidence.',
  port,
  scenarios: [],
};
try {
  for (const throttle of [1, 4]) {
    for (const layout of ['many-groups', 'one-group']) {
      const result = await scenario(browser, origin, layout, throttle);
      report.scenarios.push(result);
      writeFileSync(join(outDir, 'report.json'), `${JSON.stringify(report, null, 2)}\n`);
      console.log(`${layout} @${throttle}x`);
      for (const s of result.steps) console.log(`  ${s.label}: ${s.ms ?? '—'} ms, ${s.mounted} mounted${s.range ? `, ${s.range}` : ''}${s.longTasks?.length ? `, long tasks ${s.longTasks.join('/')}` : ''}`);
      if (result.errors.length) console.log('  page errors:', result.errors);
    }
  }
} finally {
  await browser.close();
  await new Promise((done) => server.httpServer.close(done));
}
writeFileSync(join(outDir, 'report.json'), `${JSON.stringify(report, null, 2)}\n`);
console.log(`Wrote ${join(outDir, 'report.json')}`);
