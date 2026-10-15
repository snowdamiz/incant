#!/usr/bin/env node
/**
 * BROWSER captures and keyboard flows for the hierarchy's Duplicate, handoff 0078.
 *
 * Serves editor/ui/dist with `vite preview` on the first free port from 4378, drives local
 * Chrome (headless, its own temporary profile) through the pinned playwright-core, and
 * installs, before the page's modules run:
 *   1. the scenario's configuration and tools/fake-host.js, a scripted stand-in for the
 *      native host's IPC (window.__TAURI__) — or, for the Play scenario, handoff 0044's
 *      transpiled native-bridge Play double with duplication advertised,
 *   2. the real compiled bridge, editor/bridge/native.generated.js, unchanged,
 *   3. a relabel of that bridge as a fixture, so every capture says it is not live.
 * DETERMINISTIC FIXTURE DATA: no account, network, engine or project file is used, and
 * nothing here is native WebKit evidence or proof of a command-bus copy. PNGs and
 * report.json go to a NEW ignored directory artifacts/0078-editor-subtree-duplicate/<set>/
 * (an existing one is refused). The browser and the server are closed on every exit.
 *
 * Usage (worktree root, after the UI build):
 *   node handoffs/0078-editor-subtree-duplicate/tools/capture.mjs <set> [--only=key,key] [--no-flows] [--flows-only]
 */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import { preview } from 'vite';
import { SCENARIOS } from './scenarios.mjs';
import { bridgeSource, doubleSource, schemas } from '../../0044-native-editor-play/tools/play-double.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const uiRoot = join(repoRoot, 'editor/ui');
const args = process.argv.slice(2);
const set = args.find((a) => !a.startsWith('--'));
if (!set || !/^[a-z0-9-]+$/.test(set)) throw new Error('Usage: capture.mjs <set-name> [--only=a,b] [--no-flows] [--flows-only]');
const only = args.find((a) => a.startsWith('--only='))?.slice('--only='.length).split(',').filter(Boolean);
const noFlows = args.includes('--no-flows');
const flowsOnly = args.includes('--flows-only');
const outDir = join(repoRoot, 'artifacts/0078-editor-subtree-duplicate', set);
if (existsSync(outDir)) throw new Error(`${outDir} exists; choose a new set name.`);
if (!existsSync(join(uiRoot, 'dist/index.html'))) throw new Error('Run the UI build first.');
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find((p) => p && existsSync(p));
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');
const axeSource = readFileSync(createRequire(join(uiRoot, 'package.json')).resolve('axe-core/axe.min.js'), 'utf8');
const hostScript = readFileSync(join(here, 'fake-host.js'), 'utf8');
const LABEL = 'Duplicate fixture: scripted host IPC, not live';
const RELABEL = `(() => {
  const bridge = window.__INCANT_BRIDGE__;
  if (!bridge) throw new Error('Duplicate fixture: the compiled bridge did not install.');
  bridge.label = ${JSON.stringify(LABEL)};
  bridge.isFixture = true;
})();`;
/** FIXTURE: handoff 0044's Play double (simulated Play, never renders), advertising duplication. */
const PLAY_HOST = `${doubleSource()}
(() => {
  const module = window.__nativePlayDoubleModule;
  const double = module.createNativePlayDouble({ read: module.doubleRead('single', ${JSON.stringify(schemas())}), holdStart: true });
  const fixture = { calls: [], receipts: [] };
  window.__playDouble = double;
  window.__DUPLICATE_FIXTURE__ = fixture;
  const invoke = async (name, args) => {
    fixture.calls.push({ name, args: args === undefined ? null : JSON.parse(JSON.stringify(args)) });
    if (name === 'engine_duplicate_subtree') throw 'The Play fixture does not duplicate.';
    const value = await double.invoke(name, args);
    return name === 'engine_read' ? { ...value, entity_duplicate: { available: true } } : value;
  };
  window.__TAURI__ = { core: { invoke }, event: { listen: double.listen } };
})();`;

/** The normal desktop window and a narrower one (the shell's minimum width is 960). */
const SIZES = {
  desktop: { width: 1440, height: 900 },
  narrow: { width: 1024, height: 680 },
};

const escape = (text) => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
/** The first row with this name; a copy is last among its siblings, so this is the original. */
const row = (page, name) => page.getByRole('treeitem', { name: new RegExp(`^${escape(name)}`) }).first();
const copyRow = async (page) => page.locator(`#tree-row-${await page.evaluate(() => window.__DUPLICATE_FIXTURE__.receipts.at(-1)?.entity)}`);
const statusLine = (text) => async (page) => page.locator('.statusbar__message', { hasText: text }).waitFor();
const menu = (page) => page.getByRole('menu');
const select = (name) => async (page) => {
  await row(page, name).click();
};
const duplicateKeys = (page) => page.keyboard.press(page.__mac === false ? 'Control+d' : 'Meta+d');
const contextMenu = (name) => async (page) => {
  await row(page, name).click({ button: 'right' });
  await menu(page).waitFor();
};
const keyboardMenu = (name) => async (page) => {
  await row(page, name).click();
  await page.keyboard.press('Shift+F10');
  await menu(page).waitFor();
  await page.keyboard.press('ArrowDown');
};
const duplicated = (name) => async (page) => {
  await select(name)(page);
  await duplicateKeys(page);
  await statusLine(`Duplicated ${name}`)(page);
};
const playing = async (page) => {
  const header = page.locator('.panel--viewport > .panel__header');
  await header.getByRole('button', { name: 'Play', exact: true }).click();
  await page.waitForFunction(() => window.__playDouble.status().phase === 'preparing');
  await page.evaluate(() => window.__playDouble.finishStart());
  await page.waitForFunction(() => window.__playDouble.status().phase === 'running');
  await page.evaluate(() => window.__playDouble.frame());
  await header.locator('.status-pill', { hasText: 'Playing' }).waitFor();
};

/** Static states: scenario, steps before capture, sizes, and what to crop. */
const STATES = [
  { key: 'menu', scenario: 'harbor', steps: [contextMenu('Player Character')], sizes: ['desktop', 'narrow'], crop: 'hierarchy' },
  { key: 'menu-keyboard', scenario: 'harbor', steps: [keyboardMenu('Map Camera')], sizes: ['desktop'], crop: 'hierarchy' },
  { key: 'menu-windows', scenario: 'windows', mac: false, steps: [contextMenu('Crate Stack')], sizes: ['desktop'], crop: 'hierarchy' },
  { key: 'copied', scenario: 'harbor', steps: [duplicated('Player Character')], sizes: ['desktop', 'narrow'], crop: 'window' },
  { key: 'copied-nested', scenario: 'harbor', steps: [duplicated('Map Camera')], sizes: ['desktop'], crop: 'hierarchy' },
  { key: 'copied-long', scenario: 'long', steps: [async (page) => {
    await row(page, 'Bollards').click();
    await page.keyboard.press('ArrowRight');
    await page.keyboard.press('ArrowRight');
    await page.evaluate(() => { window.__before = document.getElementById('hierarchy-tree')?.scrollTop ?? null; });
    await duplicateKeys(page);
    await statusLine('Duplicated Bollard 001')(page);
  }], sizes: ['desktop'], crop: 'window' },
  { key: 'refused-singleton', scenario: 'harbor', steps: [select('Environment'), duplicateKeys, statusLine('Duplicate failed')], sizes: ['desktop'], crop: 'window' },
  { key: 'refused-linked', scenario: 'harbor', steps: [select('Lamp Post (prefab instance)'), duplicateKeys, statusLine('Duplicate failed')], sizes: ['desktop'], crop: 'statusbar' },
  { key: 'refused-references', scenario: 'harbor', steps: [select('Malformed Props'), duplicateKeys, statusLine('Duplicate failed')], sizes: ['desktop'], crop: 'statusbar' },
  { key: 'unknown', scenario: 'unverified', steps: [select('Crate Stack'), duplicateKeys, statusLine('Duplicate result unknown')], sizes: ['desktop'], crop: 'window' },
  { key: 'pending', scenario: 'slow', steps: [select('Crate Stack'), duplicateKeys, async (page) => {
    await page.keyboard.press('Shift+F10');
    await menu(page).waitFor();
    await page.keyboard.press('ArrowDown');
  }], sizes: ['desktop'], crop: 'hierarchy' },
  { key: 'older-host', scenario: 'older-host', steps: [contextMenu('Crate Stack'), async (page) => page.keyboard.press('ArrowDown')], sizes: ['desktop'], crop: 'hierarchy' },
  { key: 'scene', scenario: 'harbor', steps: [contextMenu('Harbor'), async (page) => page.keyboard.press('ArrowDown')], sizes: ['desktop'], crop: 'hierarchy' },
  { key: 'playing', scenario: 'play', steps: [playing, select('Crate 01'), duplicateKeys, statusLine('Stop Play to duplicate.'), contextMenu('Crate 01'), async (page) => page.keyboard.press('ArrowDown')], sizes: ['desktop'], crop: 'window' },
  { key: 'history', scenario: 'harbor', steps: [duplicated('Crate Stack'), async (page) => {
    await page.getByRole('tab', { name: /^History/ }).click();
    await page.locator('[data-region="dock"]').getByText('Duplicate subtree').first().waitFor();
  }], sizes: ['desktop'], crop: 'window' },
  { key: 'menu-1x', scenario: 'harbor', scale: 1, steps: [contextMenu('Player Character')], sizes: ['desktop'], crop: 'hierarchy' },
  { key: 'refused-singleton-status', scenario: 'harbor', steps: [select('Environment'), duplicateKeys, statusLine('Duplicate failed')], sizes: ['narrow'], crop: 'statusbar' },
  { key: 'shortcuts', scenario: 'harbor', steps: [async (page) => {
    await page.keyboard.press('?');
    const dialog = page.getByRole('dialog', { name: 'Keyboard shortcuts' });
    await dialog.waitFor();
    await dialog.locator('.shortcuts__row', { hasText: 'Duplicate' }).evaluate((el) => el.scrollIntoView({ block: 'center' }));
  }], sizes: ['desktop'], crop: 'dialog' },
  { key: 'shortcuts-windows', scenario: 'windows', mac: false, steps: [async (page) => {
    await page.keyboard.press('?');
    const dialog = page.getByRole('dialog', { name: 'Keyboard shortcuts' });
    await dialog.waitFor();
    await dialog.getByRole('searchbox').fill('hierarchy');
  }], sizes: ['desktop'], crop: 'dialog' },
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

/** Geometry, focus, selection and words of what the state shows. */
function measure() {
  const round = (n) => Math.round(n * 10) / 10;
  const rect = (el) => {
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return { x: round(r.x), y: round(r.y), width: round(r.width), height: round(r.height) };
  };
  const inside = (r) => !r || (r.x >= 0 && r.y >= 0 && r.x + r.width <= innerWidth + 0.5 && r.y + r.height <= innerHeight + 0.5);
  const tree = document.getElementById('hierarchy-tree');
  const treeRect = rect(tree);
  const active = document.activeElement;
  const menuEl = document.querySelector('[role="menu"]');
  const copyId = window.__DUPLICATE_FIXTURE__.receipts.at(-1)?.entity ?? null;
  const copy = copyId ? document.getElementById(`tree-row-${copyId}`) : null;
  const copyRect = rect(copy);
  return {
    window: { width: innerWidth, height: innerHeight, scrollWidth: document.documentElement.scrollWidth, scrollHeight: document.documentElement.scrollHeight },
    tree: tree
      ? {
          ...treeRect,
          busy: tree.getAttribute('aria-busy'),
          scrollTop: tree.scrollTop,
          scrollBefore: window.__before ?? null,
          mountedRows: tree.querySelectorAll('[role="treeitem"]').length,
          selected: [...tree.querySelectorAll('[role="treeitem"][aria-selected="true"]')].map((el) => ({ id: el.id.replace('tree-row-', ''), name: el.querySelector('.tree-row__name')?.textContent, level: el.getAttribute('aria-level'), posinset: el.getAttribute('aria-posinset'), setsize: el.getAttribute('aria-setsize'), expanded: el.getAttribute('aria-expanded') })),
        }
      : null,
    copy: copyId
      ? { id: copyId, mounted: copy !== null, rect: copyRect, withinTree: !!(copyRect && treeRect && copyRect.y >= treeRect.y && copyRect.y + copyRect.height <= treeRect.y + treeRect.height + 0.5) }
      : null,
    menu: menuEl
      ? { ...rect(menuEl), inside: inside(rect(menuEl)), label: menuEl.getAttribute('aria-label'), items: [...menuEl.querySelectorAll('[role="menuitem"]')].map((item) => ({ label: item.querySelector('.menu__label')?.textContent, shortcut: item.querySelector('.menu__shortcut')?.textContent ?? null, disabled: item.getAttribute('aria-disabled'), title: item.getAttribute('title'), focused: item === document.activeElement })) }
      : null,
    status: document.querySelector('.statusbar__message')?.textContent ?? '',
    source: document.querySelector('.statusbar__conn')?.textContent ?? '',
    inspector: document.querySelector('.inspector__name')?.textContent ?? null,
    active: active ? { tag: active.tagName, id: active.id || null, role: active.getAttribute('role'), name: active.getAttribute('aria-label') ?? active.textContent?.trim().slice(0, 60), focusVisible: active.matches(':focus-visible') || active.hasAttribute('data-focus-visible') } : null,
    dialogRows: [...document.querySelectorAll('[role="dialog"] .shortcuts__row')].filter((el) => /Duplicate/.test(el.textContent)).map((el) => el.textContent),
  };
}

const server = await preview({ root: uiRoot, cacheDir: join(uiRoot, 'node_modules/.vite'), preview: { port: await freePort(4378), strictPort: true, host: '127.0.0.1' }, logLevel: 'error' });
const origin = server.resolvedUrls.local[0].replace(/\/$/, '');
const browser = await chromium.launch({ executablePath, headless: true });
mkdirSync(outDir, { recursive: true });
const report = {
  set,
  generated: new Date().toISOString(),
  browser: browser.version(),
  data: 'deterministic duplicate fixture (tools/fake-host.js or the 0044 Play double, plus the real native.generated.js); not live, not native WebKit, no command bus',
  captures: [],
  flows: [],
};

async function open(scenarioKey, size, { deviceScaleFactor = 2, mac = true } = {}) {
  const context = await browser.newContext({ viewport: size, deviceScaleFactor, colorScheme: 'dark', reducedMotion: 'reduce' });
  const page = await context.newPage();
  page.__mac = mac;
  const errors = [];
  const requests = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  page.on('request', (r) => {
    if (!r.url().startsWith(origin)) requests.push(r.url());
  });
  if (scenarioKey === 'play') {
    await page.addInitScript({ content: PLAY_HOST });
  } else {
    await page.addInitScript({ content: `window.__DUPLICATE_FIXTURE_CONFIG__ = ${JSON.stringify(SCENARIOS[scenarioKey])};` });
    await page.addInitScript({ content: hostScript });
  }
  await page.addInitScript({ content: bridgeSource() });
  await page.addInitScript({ content: RELABEL });
  await page.goto(`${origin}/`, { waitUntil: 'networkidle' });
  await page.getByRole('tree', { name: 'Hierarchy' }).waitFor();
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

/** Every document command the bridge sent: duplicates, history and other edits. */
const calls = (page) => page.evaluate(() => window.__DUPLICATE_FIXTURE__.calls.filter((call) => ['engine_duplicate_subtree', 'engine_history', 'engine_execute'].includes(call.name)).map((call) => ({ name: call.name, args: call.args })));

async function crop(page, kind, size, path) {
  const pad = 12;
  const clip = (box) => {
    const x = Math.max(0, Math.floor(box.x - pad));
    const y = Math.max(0, Math.floor(box.y - pad));
    return { x, y, width: Math.min(size.width - x, Math.ceil(box.width + 2 * pad)), height: Math.min(size.height - y, Math.ceil(box.height + 2 * pad)) };
  };
  if (kind === 'window') return page.screenshot({ path, scale: 'device' });
  if (kind === 'statusbar') {
    const bar = await page.locator('.statusbar').boundingBox();
    return page.screenshot({ path, clip: { x: 0, y: bar.y - 1, width: size.width, height: bar.height + 1 } });
  }
  if (kind === 'dialog') return page.screenshot({ path, clip: clip(await page.getByRole('dialog').boundingBox()) });
  // The hierarchy column, widened to include an open menu.
  const column = await page.locator('[data-region="hierarchy"]').boundingBox();
  const open = (await menu(page).count()) > 0 ? await menu(page).boundingBox() : null;
  const right = Math.max(column.x + column.width, open ? open.x + open.width + pad : 0);
  return page.screenshot({ path, clip: { x: 0, y: 0, width: Math.min(size.width, Math.ceil(right)), height: size.height } });
}

try {
  if (!flowsOnly) {
    for (const state of STATES) {
      if (only && !only.includes(state.key)) continue;
      for (const sizeKey of state.sizes) {
        const size = SIZES[sizeKey];
        const { context, page, errors, requests } = await open(state.scenario, size, { mac: state.mac !== false, deviceScaleFactor: state.scale ?? 2 });
        let stepError = null;
        try {
          for (const step of state.steps) await step(page);
          // Park the pointer over the empty viewport so no row shows a hover state.
          await page.mouse.move(Math.round(size.width * 0.6), Math.round(size.height * 0.3));
          await page.waitForTimeout(300);
        } catch (error) {
          stepError = String(error);
        }
        const file = `${state.key}-${sizeKey}-${state.crop}.png`;
        await crop(page, state.crop, size, join(outDir, file));
        const measured = await page.evaluate(measure);
        const axe = await axeOn(page);
        report.captures.push({ file, state: state.key, scenario: state.scenario, size: sizeKey, ...size, measured, axe, calls: await calls(page), errors, requests, stepError });
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

/* ---- Keyboard flows through the real bridge ---- */

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
    result.calls = await calls(page);
    results.push(result);
    console.log(`flow ${name}: ${result.error ?? 'ok'}`);
    await context.close();
  };
  const active = (page) =>
    page.evaluate(() => {
      const el = document.activeElement;
      return { tag: el?.tagName, id: el?.id || null, role: el?.getAttribute('role') ?? null, text: el?.textContent?.trim().slice(0, 60), focusVisible: !!el && (el.matches(':focus-visible') || el.hasAttribute('data-focus-visible')) };
    });
  const status = (page) => page.locator('.statusbar__message').innerText();
  const tabToTree = async (page) => {
    for (let n = 0; n < 40 && !(await page.evaluate(() => document.activeElement?.getAttribute('role') === 'treeitem')); n += 1) await page.keyboard.press('Tab');
  };

  // Keyboard only: reach the tree, duplicate a nested entity, then Undo and Redo through History.
  await run('keyboard-duplicate-undo-redo', 'harbor', async (page, checks) => {
    await tabToTree(page);
    checks.start = await active(page);
    await page.keyboard.type('Map');
    checks.source = await active(page);
    await page.keyboard.press('Meta+d');
    await statusLine('Duplicated Map Camera')(page);
    checks.afterDuplicate = { focus: await active(page), measured: await page.evaluate(measure) };
    await page.keyboard.press('Meta+z');
    await page.waitForFunction(() => window.__DUPLICATE_FIXTURE__.engine.applied === 0);
    await page.waitForTimeout(200);
    checks.afterUndo = { focus: await active(page), measured: await page.evaluate(measure), status: await status(page) };
    await page.keyboard.press('Shift+Meta+z');
    await page.waitForFunction(() => window.__DUPLICATE_FIXTURE__.engine.applied === 1);
    await page.waitForTimeout(200);
    checks.afterRedo = { focus: await active(page), measured: await page.evaluate(measure) };
    checks.history = await page.evaluate(() => window.__DUPLICATE_FIXTURE__.engine.history.map((entry) => `${entry.description} (${entry.actor.origin})`));
  });

  // For comparison with Undo above: where focus goes when Delete removes the focused row.
  await run('delete-focus-comparison', 'harbor', async (page, checks) => {
    await tabToTree(page);
    await page.keyboard.type('Rubble');
    checks.source = await active(page);
    await page.keyboard.press('Meta+Backspace');
    await page.waitForFunction(() => window.__DUPLICATE_FIXTURE__.engine.applied === 1);
    await page.waitForTimeout(200);
    checks.afterDelete = { focus: await active(page), history: await page.evaluate(() => window.__DUPLICATE_FIXTURE__.engine.history.map((entry) => entry.description)) };
  });

  // Held keys and a second press while the first copy is pending: one request.
  await run('held-keys-and-second-press', 'slow', async (page, checks) => {
    await row(page, 'Crate Stack').click();
    await page.keyboard.down('Meta');
    await page.keyboard.down('d');
    await page.keyboard.down('d');
    await page.keyboard.down('d');
    await page.keyboard.up('d');
    await page.waitForTimeout(100);
    checks.afterHeld = { status: await status(page), busy: await page.locator('#hierarchy-tree').getAttribute('aria-busy') };
    await page.keyboard.press('d');
    await page.keyboard.up('Meta');
    await statusLine('Wait for the current duplicate to finish.')(page);
    checks.afterSecond = { status: await status(page), rows: await page.locator('#hierarchy-tree [role="treeitem"]').count(), selected: await page.locator('#hierarchy-tree [aria-selected="true"]').allInnerTexts() };
  });

  // Text fields, dialogs and other panels keep ⌘D; a scene says why it cannot be copied.
  await run('guards', 'harbor', async (page, checks) => {
    const filter = page.getByRole('searchbox', { name: 'Filter hierarchy by name' });
    await filter.click();
    await page.keyboard.type('cra');
    await page.keyboard.press('Meta+d');
    checks.filter = { value: await filter.inputValue(), focus: await active(page) };
    await filter.fill('');
    await row(page, 'Crate Stack').click();
    await page.keyboard.press('F2');
    await page.keyboard.press('Meta+d');
    checks.rename = { focus: await active(page), input: await page.getByLabel('New name').count() };
    await page.keyboard.press('Escape');
    await page.keyboard.press('F6');
    checks.otherPanel = { focus: await active(page) };
    await page.keyboard.press('Meta+d');
    await page.keyboard.press('?');
    await page.getByRole('dialog', { name: 'Keyboard shortcuts' }).waitFor();
    await page.keyboard.press('Meta+d');
    checks.dialog = { focus: await active(page) };
    await page.keyboard.press('Escape');
    await row(page, 'Harbor').click();
    await page.keyboard.press('Meta+d');
    await statusLine('Scenes cannot be duplicated.')(page);
    checks.scene = { status: await status(page), focus: await active(page) };
  });

  // While Play has a world, ⌘D and the menu leave the inspected selection alone.
  await run('live-play', 'play', async (page, checks) => {
    await playing(page);
    await row(page, 'Crate 01').click();
    await page.locator('.runtime').waitFor();
    await page.keyboard.press('Meta+d');
    await statusLine('Stop Play to duplicate.')(page);
    checks.playing = { status: await status(page), selected: await page.locator('#hierarchy-tree [aria-selected="true"]').allInnerTexts(), runtime: await page.locator('.runtime__title').innerText(), focus: await active(page) };
    await page.locator('.panel--viewport > .panel__header').getByRole('button', { name: 'Stop', exact: true }).click();
    await page.locator('.panel--viewport > .panel__header .status-pill', { hasText: 'Playing' }).waitFor({ state: 'detached' });
    await row(page, 'Crate 01').click({ button: 'right' });
    checks.afterStop = await page.getByRole('menuitem', { name: /^Duplicate/ }).evaluate((el) => ({ disabled: el.getAttribute('aria-disabled'), title: el.getAttribute('title') }));
  });

  return results;
}
