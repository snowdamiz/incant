#!/usr/bin/env node
/**
 * BROWSER scale measurements for handoff 0033: mounted rows/controls and interaction
 * timings of the built UI with 100,000 entities, 20,000 assets and a 65,536-value grid.
 *
 * Interaction times run from the input to React's commit (DOM updated and effects run),
 * polled on macrotasks; the next paint follows within one frame and is not included.
 * Scroll times are frame-driven and run to the first frame with the new rows. Long
 * tasks (50 ms or more) after the helpers load are listed per scenario.
 *
 * Headless Chrome through the pinned playwright-core, unthrottled and with CDP CPU
 * throttling (4x) as a rough stand-in for slower hardware. These are browser numbers on
 * the machine that runs this script; they are NOT the PLAN.md M1 MacBook Air device gate
 * (edit under 100 ms, viewport at 60 fps) and must not be reported as such.
 *
 * Data:
 *   - ?fixture=stress: the sample fixture's 100,000-entity scene (read-only);
 *   - a MUTABLE TEST DOUBLE built in the page (100,000 entities) that accepts
 *     entity.rename and publishes a new snapshot, to time the UI side of an edit;
 *   - review-double.mjs: native-shaped schemas through the real bridge conversion.
 *
 * Usage (repo root, after the UI build):
 *   node handoffs/0033-editor-design-review/tools/measure-scale.mjs <set>
 */
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:net';
import os from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import { preview } from 'vite';
import { reviewInitScript } from './review-double.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const uiRoot = join(repoRoot, 'editor/ui');
const set = process.argv[2];
if (!set || !/^[a-z0-9-]+$/.test(set)) throw new Error('Usage: measure-scale.mjs <set-name>');
const outDir = join(repoRoot, 'artifacts/0033-editor-design-review', `scale-${set}`);
if (existsSync(outDir)) throw new Error(`${outDir} exists; choose a new set name.`);
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find((p) => p && existsSync(p));
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');
if (!existsSync(join(uiRoot, 'dist/index.html'))) throw new Error('Run the UI build first.');

/** A mutable test double: 100,000 entities in 100 groups; rename publishes a new snapshot. */
const MUTABLE_DOUBLE = `
  (() => {
    const crock = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';
    const id = (n) => { let s = ''; for (let i = 0; i < 16; i += 1) { s = crock[n % 32] + s; n = Math.floor(n / 32); } return '01JA34SC00' + s; };
    const nodes = {}; const entities = {}; let next = 1;
    const root = id(next++);
    const groups = [];
    for (let g = 0; g < 100; g += 1) {
      const gid = id(next++); const children = [];
      for (let i = 0; i < 1000; i += 1) {
        const eid = id(next++);
        const name = 'Entity ' + String(g + 1).padStart(3, '0') + '.' + String(i + 1).padStart(4, '0');
        nodes[eid] = { id: eid, name, kind: 'mesh', parent: gid, children: [] };
        entities[eid] = { id: eid, name, kind: 'mesh', components: [{ type: 'Transform', schemaVersion: 1, value: { translation: [i, 0, g], rotation: [0, 0, 0, 1], scale: [1, 1, 1] } }] };
        children.push(eid);
      }
      const name = 'Group ' + String(g + 1).padStart(3, '0');
      nodes[gid] = { id: gid, name, kind: 'group', parent: root, children };
      entities[gid] = { id: gid, name, kind: 'group', components: [] };
      groups.push(gid);
    }
    nodes[root] = { id: root, name: 'Scale Scene (test double)', kind: 'scene', parent: null, children: groups };
    entities[root] = { id: root, name: 'Scale Scene (test double)', kind: 'scene', components: [] };
    const schemas = { Transform: { type: 'Transform', version: 1, title: 'Transform', order: ['translation', 'rotation', 'scale'], properties: {
      translation: { type: 'array', items: { type: 'number' }, minItems: 3, maxItems: 3, optional: false },
      rotation: { type: 'array', items: { type: 'number' }, minItems: 4, maxItems: 4, optional: false },
      scale: { type: 'array', items: { type: 'number' }, minItems: 3, maxItems: 3, optional: false } } } };
    let snapshot = {
      connection: { status: 'ready', project: { id: id(0), name: 'Scale double (100,000 entities)' } },
      hierarchy: { status: 'ready', value: { roots: [root], nodes } }, schemas, entities, diagnostics: [],
      history: { entries: [], applied: 0 }, console: [], provider: { status: 'not-connected', provider: 'openai' },
      agent: { status: 'unavailable', reason: 'Evidence test double.' }, viewport: { status: 'not-attached', reason: 'Evidence test double.' },
      assets: { status: 'ready', value: [] }, assetImport: { available: false, reason: 'Evidence test double.' },
    };
    const listeners = new Set();
    window.__scale = { root, firstGroup: groups[0], times: [] };
    window.__INCANT_BRIDGE__ = {
      protocolVersion: 1, capabilities: ['entity.rename'], label: 'Scale test double (not engine)', isFixture: false,
      getSnapshot: () => snapshot,
      subscribe: (fn) => { listeners.add(fn); return () => listeners.delete(fn); },
      dispatch: (command) => {
        if (command.type !== 'entity.rename') return Promise.resolve({ ok: false, error: { code: 'test-double', message: 'Only rename is simulated.' } });
        const start = performance.now();
        const h = snapshot.hierarchy.value;
        const node = h.nodes[command.entity];
        const nextNodes = { ...h.nodes, [command.entity]: { ...node, name: command.name } };
        const nextEntities = { ...snapshot.entities, [command.entity]: { ...snapshot.entities[command.entity], name: command.name } };
        const entries = [...snapshot.history.entries, { transaction: id(900000 + snapshot.history.entries.length), description: 'Rename to ' + command.name, origin: { kind: 'user' }, at: new Date().toISOString() }];
        snapshot = { ...snapshot, hierarchy: { status: 'ready', value: { roots: h.roots, nodes: nextNodes } }, entities: nextEntities, history: { entries, applied: entries.length } };
        const built = performance.now();
        listeners.forEach((fn) => fn());
        window.__scale.times.push({ build: built - start, published: performance.now() });
        return Promise.resolve({ ok: true });
      },
      request: () => Promise.resolve({ ok: false, error: { code: 'test-double', message: 'Evidence test double.' } }),
    };
  })();
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

/**
 * In-page helpers. `__untilTask` polls on macrotasks (MessageChannel), so it measures
 * the time until React has committed the change (DOM updated, effects run), without
 * waiting for a frame. `__untilFrame` waits for the next frame after the condition
 * holds, for work that is itself frame-driven (scrolling). Long tasks (50 ms or more)
 * are collected for the whole page.
 */
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
  window.__untilFrame = (test, limit = 10000) => new Promise((done) => {
    const start = performance.now();
    const step = () => {
      if (test()) { requestAnimationFrame(() => done(performance.now() - start)); return; }
      if (performance.now() - start > limit) { done(-1); return; }
      requestAnimationFrame(step);
    };
    step();
  });
  window.__key = (key) => document.activeElement.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }));
  window.__counts = () => ({
    treeitems: document.querySelectorAll('[role="treeitem"]').length,
    options: document.querySelectorAll('[role="option"]').length,
    inputs: document.querySelectorAll('input, textarea').length,
    tableCells: document.querySelectorAll('.number-table__cell').length,
    domNodes: document.getElementsByTagName('*').length,
  });
`;

const median = (values) => {
  const sorted = [...values].filter((v) => v >= 0).sort((a, b) => a - b);
  return sorted.length ? Math.round(sorted[Math.floor(sorted.length / 2)] * 10) / 10 : null;
};
const max = (values) => (values.length ? Math.round(Math.max(...values) * 10) / 10 : null);
const round = (v) => (v === null || v === undefined ? null : Math.round(v * 10) / 10);

async function stressScenario(page, origin) {
  const result = {};
  const started = Date.now();
  await page.goto(`${origin}/?fixture=stress`, { waitUntil: 'load' });
  await page.getByRole('tree', { name: 'Hierarchy' }).waitFor({ timeout: 60000 });
  result.loadToTreeMs = Date.now() - started;
  await page.addScriptTag({ content: HELPERS });
  result.initialCounts = await page.evaluate(() => window.__counts());

  // Filter to every entity (100,000 matches), then to one group (1,000 matches).
  for (const [label, text, expect] of [['filterAll', 'Entity', '100,000 matches'], ['filterGroup', 'Entity 050.', '1,000 matches']]) {
    const ms = await page.evaluate(async ([value, wanted]) => {
      const input = document.getElementById('hierarchy-filter');
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
      const start = performance.now();
      setter.call(input, value);
      input.dispatchEvent(new Event('input', { bubbles: true }));
      await window.__untilTask(() => document.getElementById('hierarchy-filter-status')?.textContent?.includes(wanted), 30000);
      return performance.now() - start;
    }, [text, expect]);
    result[label] = { ms: round(ms), counts: await page.evaluate(() => window.__counts()) };
  }

  // Keyboard: 30 Arrow Down presses, each until the next row has focus; then End and Home.
  await page.getByRole('treeitem').first().click();
  const arrows = [];
  for (let i = 0; i < 30; i += 1) {
    arrows.push(await page.evaluate(async () => {
      const before = document.activeElement;
      const start = performance.now();
      window.__key('ArrowDown');
      await window.__untilTask(() => document.activeElement !== before);
      return performance.now() - start;
    }));
  }
  result.arrowDown = { medianMs: median(arrows), maxMs: max(arrows), samples: arrows.length };
  result.end = await page.evaluate(async () => {
    const start = performance.now();
    window.__key('End');
    const ms = await window.__untilTask(() => document.activeElement?.textContent?.includes('Entity 050.1000'));
    return { ms: ms < 0 ? -1 : performance.now() - start, focused: document.activeElement?.textContent?.trim().slice(0, 40), counts: window.__counts() };
  });
  result.end.ms = round(result.end.ms);

  // Clear the filter, expand group 1 (1,000 rows) and scroll to its middle.
  await page.getByLabel('Filter hierarchy by name').fill('');
  await page.waitForTimeout(300);
  result.scroll = await page.evaluate(async () => {
    const tree = document.getElementById('hierarchy-tree');
    const start = performance.now();
    tree.scrollTop = 28 * 60;
    const ms = await window.__untilFrame(() => [...document.querySelectorAll('[role="treeitem"]')].some((row) => row.style.top === `${2 + 28 * 62}px`));
    return { ms: ms < 0 ? -1 : performance.now() - start, counts: window.__counts() };
  });
  result.scroll.ms = round(result.scroll.ms);

  // Assets: 20,000 rows in folder groups.
  result.assets = await page.evaluate(async () => {
    const start = performance.now();
    document.getElementById('navigator-tab-assets').click();
    await window.__untilTask(() => document.querySelector('[role="listbox"][aria-label="Assets"] [role="option"]'));
    const open = performance.now() - start;
    const first = document.querySelector('[role="listbox"][aria-label="Assets"] [role="option"]');
    first.click();
    first.focus();
    const t = performance.now();
    window.__key('End');
    await window.__untilTask(() => document.activeElement?.textContent?.startsWith('surface_9999'));
    return { openMs: open, endMs: performance.now() - t, counts: window.__counts() };
  });
  result.assets.openMs = round(result.assets.openMs);
  result.assets.endMs = round(result.assets.endMs);
  return result;
}

async function renameScenario(page, origin) {
  await page.addInitScript(MUTABLE_DOUBLE);
  const started = Date.now();
  await page.goto(`${origin}/`, { waitUntil: 'load' });
  await page.getByRole('tree', { name: 'Hierarchy' }).waitFor({ timeout: 60000 });
  const result = { loadToTreeMs: Date.now() - started };
  await page.addScriptTag({ content: HELPERS });
  const samples = [];
  for (let i = 0; i < 5; i += 1) {
    const row = page.getByRole('treeitem', { name: /^(Group 001|District)/ }).first();
    await row.click();
    await row.press('F2');
    const input = page.getByLabel('New name');
    await input.fill(`District ${i + 1}`);
    samples.push(await page.evaluate(async (name) => {
      const field = document.activeElement;
      const start = performance.now();
      field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));
      await window.__untilTask(() => [...document.querySelectorAll('[role="treeitem"]')].some((row) => row.textContent?.startsWith(name)));
      const total = performance.now() - start;
      const last = window.__scale.times.at(-1);
      return { totalMs: total, snapshotBuildMs: last?.build ?? null, uiAfterPublishMs: last ? performance.now() - last.published : null };
    }, `District ${i + 1}`));
  }
  result.rename = {
    totalMedianMs: median(samples.map((s) => s.totalMs)),
    totalMaxMs: max(samples.map((s) => s.totalMs)),
    doubleSnapshotBuildMedianMs: median(samples.map((s) => s.snapshotBuildMs ?? -1)),
    samples: samples.map((s) => ({ totalMs: round(s.totalMs), snapshotBuildMs: round(s.snapshotBuildMs) })),
  };
  result.counts = await page.evaluate(() => window.__counts());
  return result;
}

async function gridScenario(page, origin) {
  await page.addInitScript(reviewInitScript());
  await page.goto(`${origin}/`, { waitUntil: 'load' });
  await page.getByRole('tree', { name: 'Hierarchy' }).waitFor({ timeout: 60000 });
  await page.addScriptTag({ content: HELPERS });
  const row = page.getByRole('treeitem', { name: /^District Grid/ });
  const select = await page.evaluate(async () => {
    const target = [...document.querySelectorAll('[role="treeitem"]')].find((r) => r.textContent?.startsWith('District Grid'));
    const start = performance.now();
    target.click();
    await window.__untilTask(() => document.querySelector('[data-region="inspector"]')?.textContent?.includes('256 × 256'));
    return performance.now() - start;
  });
  void row;
  const inspectorInputs = await page.evaluate(() => document.querySelectorAll('[data-region="inspector"] input').length);
  const show = await page.evaluate(async () => {
    const button = [...document.querySelectorAll('[data-region="inspector"] button')].find((b) => b.textContent?.includes('Show values'));
    const start = performance.now();
    button.click();
    await window.__untilTask(() => document.querySelector('.number-table__cell'));
    return { ms: performance.now() - start, counts: window.__counts() };
  });
  const scroll = await page.evaluate(async () => {
    const table = document.querySelector('.number-table');
    const start = performance.now();
    table.scrollTop = 22 * 180;
    table.scrollLeft = 34 * 120;
    await window.__untilFrame(() => [...document.querySelectorAll('.number-table__index')].some((cell) => cell.textContent === '180'));
    return { ms: performance.now() - start, counts: window.__counts() };
  });
  return { selectMs: round(select), inspectorInputs, showValues: { ms: round(show.ms), counts: show.counts }, scrollTable: { ms: round(scroll.ms), counts: scroll.counts } };
}

mkdirSync(outDir, { recursive: true });
const port = await freePort(4233);
const server = await preview({ root: uiRoot, preview: { port, strictPort: true, host: '127.0.0.1' }, logLevel: 'error' });
const origin = `http://127.0.0.1:${port}`;
const browser = await chromium.launch({ executablePath, headless: true });
const report = {
  set,
  generated: new Date().toISOString(),
  browser: `Chrome ${browser.version()} (headless, playwright-core)`,
  machine: { cpu: os.cpus()[0]?.model, cores: os.cpus().length, memoryGb: Math.round(os.totalmem() / 2 ** 30), platform: `${os.platform()} ${os.release()}` },
  note: 'Browser measurements on this machine only. Not the M1 MacBook Air device gate (PLAN.md: edit under 100 ms, viewport 60 fps on a 100,000-entity scene).',
  runs: [],
};
try {
  for (const throttle of [1, 4]) {
    const run = { cpuThrottle: throttle };
    for (const [name, scenario] of [['stressFixture', stressScenario], ['renameDouble', renameScenario], ['navigationGrid', gridScenario]]) {
      const context = await browser.newContext({ viewport: { width: 1440, height: 874 }, deviceScaleFactor: 2, colorScheme: 'dark', bypassCSP: true });
      const page = await context.newPage();
      if (throttle > 1) {
        const cdp = await context.newCDPSession(page);
        await cdp.send('Emulation.setCPUThrottlingRate', { rate: throttle });
      }
      const errors = [];
      page.on('pageerror', (e) => errors.push(String(e)));
      try {
        run[name] = await scenario(page, origin);
        run[name].longTasksMs = await page.evaluate(() => window.__longTasks ?? []);
      } catch (error) {
        run[name] = { error: String(error).slice(0, 400) };
      }
      run[name].pageErrors = errors;
      console.log(`throttle ${throttle}x ${name}: ${JSON.stringify(run[name]).slice(0, 300)}`);
      await context.close();
    }
    report.runs.push(run);
  }
} finally {
  await browser.close();
  await new Promise((done) => server.httpServer.close(done));
}
writeFileSync(join(outDir, 'report.json'), `${JSON.stringify(report, null, 2)}\n`);
console.log(`Wrote ${join(outDir, 'report.json')}`);
