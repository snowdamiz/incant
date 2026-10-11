#!/usr/bin/env node
/**
 * BROWSER presentation review of import completion receipts for handoff 0038. Headless
 * Chrome on the real production UI build (editor/ui/dist). Nothing here is native WebKit,
 * native-app, screen-reader or device evidence, and no host result is real: every import,
 * Undo, Redo and rename result comes from the MOCKED live double in receipt-double.mjs.
 * What is real is the UI: the requests it builds, its completion text, focus, live
 * regions and layout.
 *
 * Adapted from handoffs/0035-editor-union-review/tools/capture.mjs: `vite preview` on the
 * first free port from 4238 (an occupied port is never reused), local Chrome through the
 * pinned playwright-core, deviceScaleFactor 2, and the same three window sizes. Each step
 * records the visible notice, the status line, the History the UI shows and the host
 * holds, the just-imported marks, focus, axe violations and live-region DOM changes.
 *
 * Output: a NEW ignored directory artifacts/0038-import-completion-review/<set>/ (an
 * existing one is refused). The server and browser are closed on exit.
 *
 * Usage (repo root, after `npm run build --workspace editor/ui`):
 *   node handoffs/0038-import-completion-review/tools/capture.mjs <set> [--only=key,key] [--sizes=normal,minimum,narrow]
 */
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import { preview } from 'vite';
import { entry, receiptInitScript } from './receipt-double.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const uiRoot = join(repoRoot, 'editor/ui');
const args = process.argv.slice(2);
const set = args.find((a) => !a.startsWith('--'));
if (!set || !/^[a-z0-9-]+$/.test(set)) throw new Error('Usage: capture.mjs <set-name> [--only=a,b] [--sizes=normal,minimum,narrow]');
const flag = (name) => args.find((a) => a.startsWith(`--${name}=`))?.split('=')[1]?.split(',').filter(Boolean);
const only = flag('only');
const sizeFilter = flag('sizes');
const outDir = join(repoRoot, 'artifacts/0038-import-completion-review', set);
if (existsSync(outDir)) throw new Error(`${outDir} exists; choose a new set name.`);
const axePath = createRequire(join(uiRoot, 'package.json')).resolve('axe-core/axe.min.js');
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find((p) => p && existsSync(p));
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');
if (!existsSync(join(uiRoot, 'dist/index.html'))) throw new Error('Run the UI build first.');

/** Window sizes; `inspector: 'Home'` moves the Inspector splitter to its 280 px minimum with the keyboard. */
const SIZES = [
  { key: 'normal', width: 1440, height: 874 },
  { key: 'minimum', width: 1000, height: 650, inspector: 'Home' },
  { key: 'narrow', width: 1440, height: 874, inspector: 'Home' },
];

/* ---------- MOCKED host outcomes (see receipt-double.mjs) ---------- */

const IMPORT_TX = entry('2026-10-10T14:31:20.000Z', 7200, 'Import assets');
const SOURCE_WATCH = entry('2026-10-10T14:31:21.000Z', 7300, 'Reimport changed sources', { kind: 'import', source: 'source-watch' });
const NEW_ASSETS = [
  { id: '01M4K3KQ0000000000000007AA', name: 'bollard', path: 'models/dock/bollard.glb', kind: 'model', fingerprint: 'mocked-bollard' },
  { id: '01M4K3KQ0000000000000007AB', name: 'rope_coil', path: 'textures/dock/rope_coil.png', kind: 'texture', fingerprint: 'mocked-rope_coil', textureUsage: 'color' },
  { id: '01M4K3KQ0000000000000007AC', name: 'gull_cry', path: 'audio/gull_cry.wav', kind: 'audio', fingerprint: 'mocked-gull_cry' },
];
const OUTCOME = {
  unchanged: { kind: 'unchanged' },
  unchangedConcurrent: { kind: 'unchanged', concurrent: SOURCE_WATCH },
  changedReimport: { kind: 'changed', entry: IMPORT_TX },
  changedImport: { kind: 'changed', entry: IMPORT_TX, assets: NEW_ASSETS },
  // A host without receipts that did commit: History gains "Import assets", the result says nothing.
  missing: { kind: 'missing', entry: IMPORT_TX },
  // incant_import ImportError::Cook wrapping AssetError::Io, as the native host formats it (code asset.import).
  cookFailure: { kind: 'failure', error: { code: 'asset.import', message: 'could not cook textures/dock/rope_coil.png: asset IO: No such file or directory (os error 2)' } },
};

/* ---------- page helpers ---------- */

const settle = (page, ms = 250) => page.waitForTimeout(ms);
const openAssets = (page) => page.getByRole('tab', { name: /^Assets/ }).click();
const openHierarchy = (page) => page.getByRole('tab', { name: /^Hierarchy/ }).click();
const openHistory = (page) => page.getByRole('tab', { name: /^History/ }).click();
const selectAsset = (page, name) => page.getByRole('option', { name: new RegExp(`^${name}`) }).click();
const release = (page, outcome) => page.evaluate((value) => window.__incantReview.release(value), outcome);
const step = (page, name) => page.evaluate((value) => { window.__live.step = value; }, name);

/** Keyboard reimport: focus the details' action button and press Enter, as a keyboard user does. */
async function reimportWithKeyboard(page) {
  await page.locator('#asset-inspector .asset-section__actions button').focus();
  await page.keyboard.press('Enter');
}

/** Opens the import form and types the paths, adding each next row with Shift+Enter. */
async function typePaths(page, paths) {
  await page.locator('.asset-browser__import').click();
  for (const [i, path] of paths.entries()) {
    const input = page.getByRole('textbox', { name: `Path ${i + 1}` });
    await input.fill(path);
    if (i < paths.length - 1) await input.press('Shift+Enter');
  }
}

/* ---------- page-side readers ---------- */

/** Records every DOM change inside a live region, labelled with the current review step. */
function installLiveLog() {
  const live = (window.__live = { step: 'load', log: [] });
  const clean = (text) => (text ?? '').replace(/\s+/g, ' ').trim();
  const SELECTOR = '[role="status"], [role="alert"], [aria-live]';
  const label = (region) => region.classList.contains('statusbar__message') ? 'status line'
    : region.classList.contains('asset-note') ? 'notice'
      : region.classList.contains('asset-pending') ? 'pending'
        : `${region.tagName.toLowerCase()}.${[...region.classList].join('.')}`;
  const start = () => new MutationObserver((records) => {
    for (const record of records) {
      for (const node of record.addedNodes) {
        if (node.nodeType !== 1) continue;
        const regions = [...(node.matches(SELECTOR) ? [node] : []), ...node.querySelectorAll(SELECTOR)];
        for (const region of regions) live.log.push({ step: live.step, region: label(region), change: 'region inserted', text: clean(region.textContent) });
      }
      for (const node of record.removedNodes) {
        if (node.nodeType !== 1) continue;
        const regions = [...(node.matches(SELECTOR) ? [node] : []), ...node.querySelectorAll(SELECTOR)];
        for (const region of regions) live.log.push({ step: live.step, region: label(region), change: 'region removed', text: clean(region.textContent) });
      }
      const target = record.target.nodeType === 1 ? record.target : record.target.parentElement;
      const region = target?.closest(SELECTOR);
      if (!region) continue;
      if (record.type === 'characterData') {
        live.log.push({ step: live.step, region: label(region), change: 'text changed', from: record.oldValue, to: record.target.textContent, text: clean(region.textContent) });
        continue;
      }
      const added = [...record.addedNodes].map((n) => n.textContent).filter(Boolean);
      const removed = [...record.removedNodes].map((n) => n.textContent).filter(Boolean);
      if (added.length === 0 && removed.length === 0) continue;
      live.log.push({ step: live.step, region: label(region), change: added.length && removed.length ? 'replaced' : added.length ? 'added' : 'removed', added, removed, text: clean(region.textContent) });
    }
  }).observe(document.documentElement, { subtree: true, childList: true, characterData: true, characterDataOldValue: true });
  if (document.documentElement) start();
  else document.addEventListener('DOMContentLoaded', start, { once: true });
}

/** What has focus: role, name, region and ring. Same reader as 0035. */
function focusInfo() {
  const el = document.activeElement;
  if (!el || el === document.body) return { element: 'body' };
  const text = (id) => document.getElementById(id)?.textContent?.replace(/\s+/g, ' ').trim() ?? '';
  const labelled = el.getAttribute('aria-labelledby');
  const name = el.getAttribute('aria-label')
    ?? (labelled ? labelled.split(/\s+/).map(text).join(' ') : null)
    ?? (el.labels?.[0]?.textContent?.trim() || null)
    ?? el.textContent?.replace(/\s+/g, ' ').trim().slice(0, 80);
  const ringed = [el, el.parentElement, el.parentElement?.parentElement].find((n) => n && getComputedStyle(n).outlineStyle !== 'none');
  return {
    tag: el.tagName.toLowerCase(),
    role: el.getAttribute('role'),
    name,
    region: el.closest('[data-region]')?.getAttribute('data-region') ?? (el.closest('.titlebar') ? 'titlebar' : null),
    outline: ringed ? `${getComputedStyle(ringed).outlineStyle} ${getComputedStyle(ringed).outlineWidth} ${getComputedStyle(ringed).outlineColor}` : null,
  };
}

/** The completion state as shown, and the host state it should agree with. */
function completion() {
  const clean = (text) => (text ?? '').replace(/\s+/g, ' ').trim() || null;
  const round = (n) => Math.round(n * 10) / 10;
  const rgb = (value) => {
    const srgb = value.match(/color\(srgb ([\d.]+) ([\d.]+) ([\d.]+)/);
    if (srgb) return srgb.slice(1, 4).map((v) => Number(v) * 255);
    return (value.match(/[\d.]+/g) ?? []).slice(0, 3).map(Number);
  };
  const lum = (c) => rgb(c).map((v) => v / 255).map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4)).reduce((s, v, i) => s + v * [0.2126, 0.7152, 0.0722][i], 0);
  const background = (el) => {
    for (let node = el; node; node = node.parentElement) {
      const bg = getComputedStyle(node).backgroundColor;
      if (bg && bg !== 'transparent' && !/rgba\(\d+, \d+, \d+, 0\)/.test(bg)) return bg;
    }
    return 'rgb(11, 12, 15)';
  };
  const contrast = (el) => {
    const a = lum(getComputedStyle(el).color);
    const b = lum(background(el));
    return round((Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05));
  };
  const inspector = document.querySelector('[data-region="inspector"]');
  const note = document.querySelector('#asset-inspector .asset-note');
  const scroller = note?.closest('.panel__scroll');
  let notice = null;
  if (note) {
    const box = note.getBoundingClientRect();
    const view = scroller?.getBoundingClientRect() ?? box;
    const paragraphs = [...note.querySelectorAll('p')].map((p) => ({
      text: clean(p.textContent),
      lines: Math.round(p.getBoundingClientRect().height / parseFloat(getComputedStyle(p).lineHeight)),
      contrast: contrast(p),
    }));
    notice = {
      text: clean(note.textContent),
      role: note.getAttribute('role'),
      tone: note.classList.contains('asset-note--error') ? 'error' : 'success',
      icon: note.querySelector(':scope > .icon') ? getComputedStyle(note.querySelector(':scope > .icon')).color : null,
      width: round(box.width),
      height: round(box.height),
      inView: box.top >= view.top - 0.5 && box.bottom <= view.bottom + 0.5,
      paragraphs,
      dismiss: note.querySelector('button')?.getAttribute('aria-label') ?? null,
    };
  }
  const host = window.__incantReview.snapshot().history;
  const historyList = document.querySelector('[data-region="dock"] .timeline');
  const message = document.querySelector('.statusbar__message');
  return {
    inspectorWidth: inspector ? round(inspector.getBoundingClientRect().width) : null,
    notice,
    statusLine: clean(message?.textContent),
    // The status line is one line with an ellipsis: does the whole message fit?
    statusLineFit: message ? { width: message.clientWidth, needed: message.scrollWidth, clipped: message.scrollWidth > message.clientWidth + 1 } : null,
    pending: clean(document.querySelector('#asset-inspector .asset-pending')?.textContent),
    action: clean(document.querySelector('#asset-inspector .asset-section__actions button, #asset-inspector .import-form__footer button')?.textContent),
    paths: [...document.querySelectorAll('#asset-inspector .import-row__input')].map((input) => input.value),
    header: clean(document.querySelector('#asset-inspector-title')?.textContent),
    justImported: [...document.querySelectorAll('#asset-list [role="option"]')].filter((row) => row.querySelector('.asset-row__recent')).map((row) => clean(row.textContent)),
    historyShown: historyList ? {
      meta: clean(document.querySelector('[data-region="dock"] .history__toolbar .panel__meta')?.textContent),
      rows: [...historyList.querySelectorAll('.history__row')].map((row) => clean(row.querySelector('.list__main')?.textContent)),
      nowAfter: (() => {
        const marker = historyList.querySelector('.timeline__now, [class*="now"]');
        const previous = marker?.previousElementSibling;
        return previous ? clean(previous.querySelector('.list__main')?.textContent) : null;
      })(),
    } : null,
    host: { applied: host.applied, entries: host.entries.map((item) => item.description), head: host.entries[host.applied - 1]?.transaction ?? null },
    pageOverflow: [document.documentElement.scrollWidth - document.documentElement.clientWidth, document.documentElement.scrollHeight - document.documentElement.clientHeight],
    clippedInInspector: inspector ? [...inspector.querySelectorAll('*')].filter((el) => {
      if (el.children.length > 0 || !el.textContent.trim()) return false;
      const s = getComputedStyle(el);
      return (s.textOverflow === 'ellipsis' && el.scrollWidth > el.clientWidth + 1) || (s.webkitLineClamp !== 'none' && s.webkitLineClamp !== '' && el.scrollHeight > el.clientHeight + 1);
    }).map((el) => clean(el.textContent).slice(0, 80)) : [],
  };
}

/* ---------- scenarios ---------- */

/** Common start: History visible in the dock, the Assets view open. */
async function begin(page) {
  await openHistory(page);
  await openAssets(page);
}

const STATES = [
  {
    key: 'noop-reimport',
    note: 'Reimport crate_albedo; MOCKED unchanged receipt (transaction null), history unchanged with a redo tail',
    steps: async (page, snap) => {
      await begin(page);
      await selectAsset(page, 'crate_albedo');
      await snap('before');
      await step(page, 'reimport');
      await reimportWithKeyboard(page);
      await settle(page, 150);
      await snap('pending');
      await step(page, 'release unchanged');
      await release(page, OUTCOME.unchanged);
      await settle(page);
      await snap('done');
    },
  },
  {
    key: 'noop-import',
    note: 'Import form with two paths of existing assets; MOCKED unchanged receipt',
    steps: async (page, snap) => {
      await begin(page);
      await typePaths(page, ['models/props/crate_stack.glb', 'textures/props/crate_albedo.png']);
      await snap('rows');
      await step(page, 'import');
      await page.keyboard.press('Enter');
      await settle(page, 150);
      await snap('pending');
      await step(page, 'release unchanged');
      await release(page, OUTCOME.unchanged);
      await settle(page);
      await snap('done');
    },
  },
  {
    key: 'changed-reimport',
    note: 'Reimport crate_albedo; MOCKED changed receipt; then Undo (Cmd+Z), Redo (Shift+Cmd+Z) and a newer rename through the hierarchy',
    steps: async (page, snap) => {
      await begin(page);
      await selectAsset(page, 'crate_albedo');
      await step(page, 'reimport');
      await reimportWithKeyboard(page);
      await settle(page, 150);
      await step(page, 'release changed');
      await release(page, OUTCOME.changedReimport);
      await settle(page);
      await snap('changed');
      await step(page, 'undo');
      await page.keyboard.press('Meta+Z');
      await settle(page);
      await snap('undone');
      await step(page, 'redo');
      await page.keyboard.press('Meta+Shift+Z');
      await settle(page);
      await snap('redone');
      await step(page, 'newer edit');
      await openHierarchy(page);
      await page.getByRole('treeitem', { name: /^Sun\b/ }).click();
      await page.keyboard.press('F2');
      await page.keyboard.type('Harbor Sun');
      await page.keyboard.press('Enter');
      await settle(page);
      await openAssets(page);
      await settle(page);
      await snap('newer');
    },
  },
  {
    key: 'changed-import',
    note: 'Import three new files; MOCKED changed receipt and asset list; then Cmd+Z from the reset path, then the titlebar Undo',
    steps: async (page, snap) => {
      await begin(page);
      await typePaths(page, NEW_ASSETS.map((asset) => asset.path));
      await step(page, 'import');
      await page.keyboard.press('Enter');
      await settle(page, 150);
      await step(page, 'release changed');
      await release(page, OUTCOME.changedImport);
      await settle(page);
      await snap('changed');
      // Focus is back in the reset path field, where Cmd+Z belongs to the text field.
      await step(page, 'Cmd+Z in the path field');
      await page.keyboard.press('Meta+Z');
      await settle(page);
      await snap('text-undo');
      await step(page, 'titlebar Undo');
      await page.locator('.titlebar').getByRole('button', { name: /^Undo/ }).click();
      await settle(page);
      await snap('undone');
    },
  },
  {
    key: 'unknown-reimport',
    note: 'Reimport crate_albedo; MOCKED host without receipts that appended "Import assets"',
    steps: async (page, snap) => {
      await begin(page);
      await selectAsset(page, 'crate_albedo');
      await step(page, 'reimport');
      await reimportWithKeyboard(page);
      await settle(page, 150);
      await step(page, 'release without receipt');
      await release(page, OUTCOME.missing);
      await settle(page);
      await snap('done');
    },
  },
  {
    key: 'unknown-import',
    note: 'Import form; MOCKED host without receipts that appended "Import assets"',
    sizes: ['normal'],
    steps: async (page, snap) => {
      await begin(page);
      await typePaths(page, ['models/dock/bollard.glb']);
      await step(page, 'import');
      await page.keyboard.press('Enter');
      await settle(page, 150);
      await step(page, 'release without receipt');
      await release(page, OUTCOME.missing);
      await settle(page);
      await snap('done');
    },
  },
  {
    key: 'failure-import',
    note: 'Import form with two new paths; MOCKED native cook failure (asset.import)',
    steps: async (page, snap) => {
      await begin(page);
      await typePaths(page, ['models/dock/bollard.glb', 'textures/dock/rope_coil.png']);
      await step(page, 'import');
      await page.keyboard.press('Enter');
      await settle(page, 150);
      await step(page, 'release failure');
      await release(page, OUTCOME.cookFailure);
      await settle(page);
      await snap('done');
    },
  },
  // Probe only (existing failure copy, unchanged by the receipts work): a reimport failure.
  {
    key: 'failure-reimport',
    note: 'Reimport crate_albedo; MOCKED native cook failure (asset.import) for a moved source',
    sizes: ['normal'],
    steps: async (page, snap) => {
      await begin(page);
      await selectAsset(page, 'crate_albedo');
      await step(page, 'reimport');
      await reimportWithKeyboard(page);
      await settle(page, 150);
      await step(page, 'release failure');
      await release(page, { kind: 'failure', error: { code: 'asset.import', message: 'could not cook textures/props/crate_albedo.png: asset IO: No such file or directory (os error 2)' } });
      await settle(page);
      await snap('done');
    },
  },
  // The pending note says "You can keep working": the status line is then the only feedback.
  {
    key: 'away-noop-reimport',
    note: 'Reimport crate_albedo, select gull_sheet while it runs; MOCKED unchanged receipt',
    sizes: ['normal', 'narrow'],
    steps: async (page, snap) => {
      await begin(page);
      await selectAsset(page, 'crate_albedo');
      await step(page, 'reimport');
      await reimportWithKeyboard(page);
      await settle(page, 150);
      await step(page, 'select another asset');
      await selectAsset(page, 'gull_sheet');
      await settle(page, 150);
      await step(page, 'release unchanged');
      await release(page, OUTCOME.unchanged);
      await settle(page);
      await snap('done');
    },
  },
  {
    key: 'away-changed-reimport',
    note: 'Reimport crate_albedo, select gull_sheet while it runs; MOCKED changed receipt (for comparison)',
    sizes: ['normal'],
    steps: async (page, snap) => {
      await begin(page);
      await selectAsset(page, 'crate_albedo');
      await step(page, 'reimport');
      await reimportWithKeyboard(page);
      await settle(page, 150);
      await step(page, 'select another asset');
      await selectAsset(page, 'gull_sheet');
      await settle(page, 150);
      await step(page, 'release changed');
      await release(page, OUTCOME.changedReimport);
      await settle(page);
      await snap('done');
    },
  },
  {
    key: 'away-noop-import',
    note: 'Import two existing paths, select harbor_loop while it runs; MOCKED unchanged receipt',
    sizes: ['normal'],
    steps: async (page, snap) => {
      await begin(page);
      await typePaths(page, ['models/props/crate_stack.glb', 'textures/props/crate_albedo.png']);
      await step(page, 'import');
      await page.keyboard.press('Enter');
      await settle(page, 150);
      await step(page, 'select an asset');
      await selectAsset(page, 'harbor_loop');
      await settle(page, 150);
      await step(page, 'release unchanged');
      await release(page, OUTCOME.unchanged);
      await settle(page);
      await snap('done');
    },
  },
  // The longest name in the double, to see what a one-line status line keeps.
  {
    key: 'long-noop-reimport',
    note: 'Reimport the long-named atlas; MOCKED unchanged receipt',
    sizes: ['normal', 'minimum'],
    steps: async (page, snap) => {
      await begin(page);
      await selectAsset(page, 'harbor_master_notice');
      await step(page, 'reimport');
      await reimportWithKeyboard(page);
      await settle(page, 150);
      await step(page, 'release unchanged');
      await release(page, OUTCOME.unchanged);
      await settle(page);
      await snap('done');
    },
  },
  {
    key: 'long-changed-reimport',
    note: 'Reimport the long-named atlas; MOCKED changed receipt (for comparison)',
    sizes: ['minimum'],
    steps: async (page, snap) => {
      await begin(page);
      await selectAsset(page, 'harbor_master_notice');
      await step(page, 'reimport');
      await reimportWithKeyboard(page);
      await settle(page, 150);
      await step(page, 'release changed');
      await release(page, OUTCOME.changedReimport);
      await settle(page);
      await snap('done');
    },
  },
  {
    key: 'noop-concurrent',
    note: 'Reimport crate_albedo; MOCKED unchanged receipt while the published read carries a later source-watch transaction',
    sizes: ['normal'],
    steps: async (page, snap) => {
      await begin(page);
      await selectAsset(page, 'crate_albedo');
      await step(page, 'reimport');
      await reimportWithKeyboard(page);
      await settle(page, 150);
      await step(page, 'release unchanged + concurrent');
      await release(page, OUTCOME.unchangedConcurrent);
      await settle(page);
      await snap('done');
    },
  },
];

/* ---------- runner ---------- */

function freePort(start) {
  return new Promise((done, fail) => {
    const attempt = (port) => {
      if (port > start + 40) return fail(new Error('No free port'));
      const probe = createServer();
      probe.once('error', () => attempt(port + 1));
      probe.once('listening', () => probe.close(() => done(port)));
      probe.listen(port, '127.0.0.1');
    };
    attempt(start);
  });
}

/** The exact build being reviewed: SHA-256 of every file in dist. */
function buildHashes() {
  const dist = join(uiRoot, 'dist');
  const walk = (dir) => readdirSync(dir, { withFileTypes: true }).flatMap((d) => (d.isDirectory() ? walk(join(dir, d.name)) : [join(dir, d.name)]));
  return Object.fromEntries(walk(dist).filter((file) => /\.(html|js|css)$/.test(file)).sort()
    .map((file) => [file.slice(uiRoot.length + 1), createHash('sha256').update(readFileSync(file)).digest('hex')]));
}

async function captureOne(browser, origin, state, size, init) {
  const context = await browser.newContext({
    viewport: { width: size.width, height: size.height }, deviceScaleFactor: 2, colorScheme: 'dark', bypassCSP: true,
    timezoneId: 'UTC', locale: 'en-US',
  });
  const base = `${state.key}-${size.key}-${size.width}x${size.height}`;
  const record = { state: state.key, note: state.note, size, steps: [], stepError: null, errors: [], remoteRequests: [], live: [] };
  const page = await context.newPage();
  page.on('pageerror', (e) => record.errors.push(String(e)));
  page.on('console', (m) => m.type() === 'error' && record.errors.push(m.text()));
  page.on('request', (r) => !r.url().startsWith(origin) && !r.url().startsWith('data:') && record.remoteRequests.push(r.url()));
  const snap = async (name) => {
    const file = `${base}--${name}.png`;
    const observed = await page.evaluate(completion);
    const focus = await page.evaluate(focusInfo);
    const axe = await page.evaluate(async () => {
      // eslint-disable-next-line no-undef
      const result = await axe.run(document, { resultTypes: ['violations'] });
      return result.violations.map((v) => ({ id: v.id, impact: v.impact, nodes: v.nodes.length, targets: v.nodes.slice(0, 3).map((n) => n.target.join(' ')) }));
    });
    await page.screenshot({ path: join(outDir, file) });
    const crops = {};
    for (const [name_, selector] of [['side', '.column--side'], ['dock', '[data-region="dock"]'], ['statusbar', '.statusbar']]) {
      const box = await page.locator(selector).first().boundingBox().catch(() => null);
      if (!box || box.width < 2 || box.height < 2) continue;
      crops[name_] = file.replace('.png', `-${name_}.png`);
      await page.screenshot({ path: join(outDir, crops[name_]), clip: box });
    }
    record.steps.push({ name, file, crops, observed, focus, axe });
  };
  const work = (async () => {
    await page.addInitScript(installLiveLog);
    await page.addInitScript(init);
    await page.goto(`${origin}/`, { waitUntil: 'networkidle' });
    await page.addScriptTag({ path: axePath });
    if (size.inspector) {
      const splitter = page.getByRole('separator', { name: /^Resize inspector/ });
      await splitter.focus();
      await page.keyboard.press(size.inspector);
      await page.locator('body').click({ position: { x: 1, y: size.height - 2 } }).catch(() => undefined);
    }
    try {
      await state.steps(page, snap);
    } catch (error) {
      record.stepError = String(error).slice(0, 400);
    }
    record.commands = await page.evaluate(() => window.__incantReview.commands);
    record.results = await page.evaluate(() => window.__incantReview.results);
    record.live = await page.evaluate(() => window.__live.log);
  })();
  const limit = new Promise((done) => setTimeout(() => done('timeout'), 90_000));
  if ((await Promise.race([work.then(() => 'done', (error) => { record.stepError ??= String(error).slice(0, 400); return 'done'; }), limit])) === 'timeout') {
    record.stepError ??= 'timed out';
    work.catch(() => undefined);
  }
  await Promise.race([context.close().catch(() => undefined), new Promise((done) => setTimeout(done, 10_000))]);
  return record;
}

mkdirSync(outDir, { recursive: true });
const init = receiptInitScript();
const port = await freePort(4238);
const server = await preview({ root: uiRoot, preview: { port, strictPort: true, host: '127.0.0.1' }, logLevel: 'error' });
const origin = `http://127.0.0.1:${port}`;
const browser = await chromium.launch({ executablePath, headless: true });
const report = {
  set, generated: new Date().toISOString(), browser: browser.version(), headless: true, port,
  label: 'Browser presentation review on a MOCKED live double; not native command execution',
  build: buildHashes(), captures: [],
};
const save = () => writeFileSync(join(outDir, 'report.json'), `${JSON.stringify(report, null, 2)}\n`);
try {
  for (const state of STATES) {
    if (only && !only.includes(state.key)) continue;
    for (const size of SIZES) {
      if ((state.sizes && !state.sizes.includes(size.key)) || (sizeFilter && !sizeFilter.includes(size.key))) continue;
      const record = await captureOne(browser, origin, state, size, init);
      report.captures.push(record);
      save();
      const axeIds = [...new Set(record.steps.flatMap((s) => s.axe.map((v) => v.id)))];
      console.log(`${state.key} ${size.key}: ${record.steps.map((s) => s.name).join(', ')}${record.stepError ? `  STEP ERROR ${record.stepError}` : ''}${axeIds.length ? `  axe:${axeIds.join(',')}` : ''}${record.errors.length ? `  page errors:${record.errors.length}` : ''}`);
    }
  }
} finally {
  await browser.close();
  await new Promise((done) => server.httpServer.close(done));
}
save();
console.log(`Wrote ${report.captures.length} captures to ${outDir}`);
