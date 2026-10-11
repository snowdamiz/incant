#!/usr/bin/env node
/**
 * BROWSER review of the release union for handoff 0035. Nothing here is native WebKit,
 * native-app or device evidence.
 *
 * Adapted from handoffs/0033-editor-design-review/tools/capture.mjs: the same `vite
 * preview` server on the first free port from 4233 (an occupied port is never reused),
 * local Chrome through the pinned playwright-core, the same measure() and crops, so
 * numbers compare with 0033. Added here:
 *   - union states: the three projections of the real Camera schema, SpriteRenderer
 *     entities and cooked-atlas assets (union-double.mjs), and the import-path mirror;
 *   - states the 0033 set lacks: every account-dialog state, host window chrome,
 *     Windows/Linux key notation, the keyboard-opened menu;
 *   - probes that RECORD what happened (focus, reveal, paging, live announcements)
 *     in report.json as `observations`, with extra screenshots at named steps.
 *
 * Output: a NEW ignored directory artifacts/0035-editor-union-review/<set>/ (an existing
 * one is refused). The server and browser are closed on exit.
 *
 * Usage (repo root, after `npm run build --workspace editor/ui`):
 *   node handoffs/0035-editor-union-review/tools/capture.mjs <set> [--only=key,key] [--sizes=normal,minimum]
 */
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import { preview } from 'vite';
import { reviewInitScript } from '../../0033-editor-design-review/tools/review-double.mjs';
import { unionInitScript } from './union-double.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const uiRoot = join(repoRoot, 'editor/ui');
const args = process.argv.slice(2);
const set = args.find((a) => !a.startsWith('--'));
if (!set || !/^[a-z0-9-]+$/.test(set)) throw new Error('Usage: capture.mjs <set-name> [--only=a,b] [--sizes=normal,minimum]');
const flag = (name) => args.find((a) => a.startsWith(`--${name}=`))?.split('=')[1]?.split(',').filter(Boolean);
const only = flag('only');
const sizeFilter = flag('sizes');
const outDir = join(repoRoot, 'artifacts/0035-editor-union-review', set);
if (existsSync(outDir)) throw new Error(`${outDir} exists; choose a new set name.`);
const axePath = createRequire(join(uiRoot, 'package.json')).resolve('axe-core/axe.min.js');
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find((p) => p && existsSync(p));
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');
if (!existsSync(join(uiRoot, 'dist/index.html'))) throw new Error('Run the UI build first.');

// Synthetic, account-free provider states (the same invented labels as providerFixture.ts).
const ADA = { id: 'acct-fixture-1', label: 'ada@example.com' };
const SIGNED_IN = { status: 'connected', provider: 'openai', method: 'oauth', accountLabel: ADA.label, accounts: [ADA], activeAccount: ADA.id };
const MAC = { platform: 'macos', controls: 'native-overlay', maximized: false, fullscreen: false, focused: true, leadingInset: 78, trailingInset: 0 };
const WINDOWS = { platform: 'windows', controls: 'custom', maximized: false, fullscreen: false, focused: true, leadingInset: 0, trailingInset: 0 };

const union = (options) => ({ url: '/', init: unionInitScript(options) });
const U = union();
const fixture = (variant = 'sample', provider) => ({ url: `/?fixture=${variant}${provider ? `&provider=${provider}` : ''}` });

/** The 0033 double with long output lists (450 problems, 1,234 messages, 260 edits), as in 0033. */
const PAGED = {
  url: '/',
  init: `${reviewInitScript()}
  (() => {
    const bridge = window.__INCANT_BRIDGE__;
    const base = bridge.getSnapshot();
    const ids = Object.keys(base.entities);
    const diagnostics = [];
    for (let n = 0; n < 450; n += 1) diagnostics.push({ id: 'p' + n, severity: n % 4 === 0 ? 'warning' : 'error', message: 'Problem ' + n + ': compound part ' + (n % 64) + ' collider dimensions must be 0.001..10000 meters', entity: n < 300 ? ids[3] : ids[n % ids.length], component: 'Collider', path: '/shape/parts/' + (n % 64) + '/shape/half_extents/' + (n % 3) });
    const console_ = [];
    for (let n = 0; n < 1234; n += 1) console_.push({ id: 'c' + n, level: n % 9 === 0 ? 'warn' : n % 31 === 0 ? 'error' : 'info', source: 'native', message: 'Source watch reloaded scenes/harbor.scene.ron (' + n + ')', at: new Date(Date.UTC(2026, 9, 10, 12, 0, 0) + n * 1000).toISOString() });
    const entries = [];
    for (let n = 0; n < 260; n += 1) entries.push({ transaction: 'tx' + n, description: 'Move Crate ' + String(n % 4 + 1).padStart(2, '0') + ' to ' + n + ', 0, 2', origin: n % 5 === 0 ? { kind: 'agent', model: 'review-model', conversation: 'c' } : { kind: 'user' }, at: new Date(Date.UTC(2026, 9, 10, 12, 0, 0) + n * 60000).toISOString() });
    const snapshot = { ...base, diagnostics, console: console_, history: { entries, applied: 130 } };
    window.__INCANT_BRIDGE__ = { ...bridge, getSnapshot: () => snapshot };
  })();
`,
};

/** Window sizes; `inspector` moves the Inspector splitter with the keyboard (Home = minimum width). */
const SIZES = [
  { key: 'normal', width: 1440, height: 874 },
  { key: 'minimum', width: 1000, height: 650, inspector: 'Home' },
  { key: 'narrow', width: 1440, height: 874, inspector: 'Home' },
  { key: 'wide', width: 1920, height: 1080 },
];

const escape = (text) => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
/**
 * Selects a hierarchy row by clicking it. The tree mounts only the rows near its scroll
 * position, so a row further down (the Sprites group in a 650 px window) is first
 * scrolled into the mounted window, the way a user scrolls to it.
 */
const tree = (name) => async (page) => {
  const item = page.getByRole('treeitem', { name: new RegExp(`^${escape(name)}`) }).first();
  const list = page.locator('[data-region="hierarchy"] [role="tree"]');
  for (let step = 0; step < 60 && (await item.count()) === 0; step += 1) {
    await list.evaluate((el) => { el.scrollTop += Math.max(40, el.clientHeight * 0.75); });
    await page.waitForTimeout(120);
  }
  await item.click();
};
const component = (name) => async (page) => {
  await page.locator(`[data-region="inspector"] section.component[aria-label="${name}"]`).evaluate((el) => el.scrollIntoView({ block: 'start' }));
  await page.waitForTimeout(150);
};
/** Focuses a named Inspector value with a key press, so :focus-visible applies as when tabbing. */
const focusField = (name) => async (page) => {
  const target = page.locator('[data-region="inspector"]').getByRole('textbox', { name, exact: true }).first();
  await target.scrollIntoViewIfNeeded();
  await target.focus();
  await page.keyboard.press('Shift+Tab');
  await page.keyboard.press('Tab');
};
const openAssets = async (page) => page.getByRole('tab', { name: /^Assets/ }).click();
const asset = (name) => async (page) => {
  await openAssets(page);
  await page.getByRole('option', { name: new RegExp(`^${escape(name)}`) }).click();
};
const accountButton = (page) => page.getByRole('button', { name: /^ChatGPT/ });
const openAccount = async (page) => accountButton(page).click();
const openShortcuts = async (page) => page.getByRole('button', { name: 'Keyboard shortcuts' }).click();

/* ---------- page-side readers used by the probes ---------- */

/** What has focus: role, name as computed from label sources, region, ring. Runs in the page. */
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
    region: el.closest('[data-region]')?.getAttribute('data-region') ?? (el.closest('[role="dialog"]') ? 'dialog' : el.closest('.titlebar') ? 'titlebar' : null),
    outline: ringed ? `${getComputedStyle(ringed).outlineStyle} ${getComputedStyle(ringed).outlineWidth} ${getComputedStyle(ringed).outlineColor}` : null,
  };
}

/** The compound parts listbox: mounted, visible and selected rows, the detail caption, focus. */
function partsState() {
  const list = document.querySelector('[data-region="inspector"] [role="listbox"]');
  if (!list) return { list: null };
  const box = list.getBoundingClientRect();
  const options = [...list.querySelectorAll('[role="option"]')].map((option) => {
    const r = option.getBoundingClientRect();
    return { index: Number(option.getAttribute('data-index')), top: Math.round(r.top - box.top), bottom: Math.round(r.bottom - box.top), selected: option.getAttribute('aria-selected') === 'true' };
  });
  const visible = options.filter((o) => o.bottom > 0 && o.top < box.height).sort((a, b) => a.index - b.index);
  // Slots the scroll position should show, versus rows actually there.
  const ROW = 26;
  // Row i spans [i * ROW + 4, (i + 1) * ROW + 4) on the canvas; it shows when it overlaps the
  // scroll window, so a row starting exactly at the bottom edge does not count.
  const firstSlot = Math.max(0, Math.floor((list.scrollTop - 4) / ROW));
  const lastSlot = Math.min(Number(list.querySelector('[role="option"]')?.getAttribute('aria-setsize') ?? 0) - 1, Math.ceil((list.scrollTop + list.clientHeight - 4) / ROW) - 1);
  const shown = new Set(visible.map((o) => o.index));
  const missing = [];
  for (let slot = firstSlot; slot <= lastSlot; slot += 1) if (!shown.has(slot)) missing.push(slot + 1);
  const caption = document.querySelector('[data-region="inspector"] .object-list__caption')?.textContent?.replace(/\s+/g, ' ').trim();
  return {
    scrollTop: Math.round(list.scrollTop),
    clientHeight: list.clientHeight,
    scrollHeight: list.scrollHeight,
    mounted: options.length,
    mountedParts: options.map((o) => o.index + 1).sort((a, b) => a - b),
    visibleParts: visible.map((o) => o.index + 1),
    blankSlots: missing,
    selected: options.filter((o) => o.selected).map((o) => o.index + 1),
    caption,
  };
}

/** The grid value table: scroll position and what is mounted. */
function tableState() {
  const table = document.querySelector('[data-region="inspector"] [aria-roledescription="value table"]');
  if (!table) return { table: null };
  const rows = [...table.querySelectorAll('.number-table__row')];
  const indexes = rows.map((row) => Number(row.querySelector('.number-table__index')?.textContent)).sort((a, b) => a - b);
  const cells = table.querySelectorAll('.number-table__cell');
  const cellIds = [...cells].map((c) => Number(c.id.match(/_002f?(\d+)--value$/)?.[1] ?? c.id.match(/-(\d+)--value$/)?.[1] ?? NaN)).filter(Number.isFinite);
  const box = table.getBoundingClientRect();
  const visibleRows = rows.filter((row) => { const r = row.getBoundingClientRect(); return r.bottom > box.top && r.top < box.bottom; })
    .map((row) => Number(row.querySelector('.number-table__index')?.textContent)).sort((a, b) => a - b);
  return {
    scrollTop: Math.round(table.scrollTop), scrollLeft: Math.round(table.scrollLeft),
    clientWidth: table.clientWidth, clientHeight: table.clientHeight, scrollWidth: table.scrollWidth, scrollHeight: table.scrollHeight,
    mountedRows: rows.length, mountedCells: cells.length,
    firstRow: indexes[0] ?? null, lastRow: indexes.at(-1) ?? null,
    visibleRows: visibleRows.length ? [visibleRows[0], visibleRows.at(-1)] : null,
    cellRange: cellIds.length ? [Math.min(...cellIds), Math.max(...cellIds)] : null,
    focused: document.activeElement === table,
  };
}

/** Accessible names of the shortcut key cells, from their aria-labelledby, for chosen actions. */
function shortcutNames(actions) {
  const text = (id) => document.getElementById(id)?.textContent?.replace(/\s+/g, ' ').trim() ?? '';
  return [...document.querySelectorAll('.shortcuts__row')].filter((row) => actions.includes(row.querySelector('.shortcuts__action')?.firstChild?.textContent?.trim()))
    .map((row) => {
      const cell = row.querySelector('.shortcuts__keys');
      return {
        action: row.querySelector('.shortcuts__action')?.firstChild?.textContent?.trim(),
        visible: [...cell.querySelectorAll('kbd')].map((k) => k.textContent).join(' | '),
        name: (cell.getAttribute('aria-labelledby') ?? '').split(/\s+/).map(text).join(' '),
      };
    });
}

const obsFocus = async (page, obs, step, extra = {}) => obs.push({ step, focus: await page.evaluate(focusInfo), ...extra });

const STATES = [
  // ---- Camera: the three projections the real schema declares ----
  { key: 'camera-rig', ...fixture(), steps: [tree('Camera Rig')], sizes: ['normal', 'minimum', 'narrow'] },
  { key: 'map-camera', ...fixture(), steps: [tree('Map Camera')], sizes: ['normal', 'narrow'] },
  { key: 'closeup-camera', ...fixture(), steps: [tree('Close-up Camera')], sizes: ['normal', 'minimum', 'narrow'] },
  { key: 'u-pixel-camera', ...U, steps: [tree('Pixel Camera')], sizes: ['normal', 'minimum', 'narrow', 'wide'] },
  { key: 'u-pixel-focus', ...U, steps: [tree('Pixel Camera'), focusField('Pixel density')], sizes: ['normal', 'minimum'] },
  { key: 'u-pixel-bad', ...U, steps: [tree('Pixel Camera (out of range)')], sizes: ['normal', 'narrow'] },
  { key: 'u-broken-camera', ...U, steps: [tree('Broken Camera')], sizes: ['normal'] },
  { key: 'u-legacy-camera', ...U, steps: [tree('Legacy Camera')], sizes: ['normal'] },
  { key: 'u-map-camera', ...U, steps: [tree('Map Camera')], sizes: ['normal'] },
  { key: 'u-follow-camera', ...U, steps: [tree('Follow Camera')], sizes: ['normal'] },
  // ---- SpriteRenderer and cooked atlases ----
  { key: 'u-sprite-sign', ...U, steps: [tree('Dock Sign')], sizes: ['normal', 'minimum', 'narrow', 'wide'] },
  { key: 'u-sprite-coil', ...U, steps: [tree('Rope Coil')], sizes: ['normal', 'narrow'] },
  { key: 'u-sprite-board', ...U, steps: [tree('Notice Board Panel')], sizes: ['normal', 'minimum', 'narrow'] },
  { key: 'u-sprite-gull', ...U, steps: [tree('Gull')], sizes: ['normal'] },
  { key: 'u-sprite-lantern', ...U, steps: [tree('Lantern Glow')], sizes: ['normal', 'narrow'] },
  { key: 'u-sprite-broken', ...U, steps: [tree('Broken Sprite')], sizes: ['normal', 'minimum', 'narrow'] },
  { key: 'u-sprite-missing', ...U, steps: [tree('Missing Texture Sprite')], sizes: ['normal'] },
  { key: 'u-sprite-sky', ...U, steps: [tree('Sky Sprite')], sizes: ['normal'] },
  { key: 'u-sprite-lost', ...U, steps: [tree('Lost Frame Sprite')], sizes: ['normal'] },
  { key: 'u-sprite-long', ...U, steps: [tree('Harbor master’s weather-beaten')], sizes: ['normal', 'minimum', 'narrow'] },
  { key: 'u-sprite-tint-focus', ...U, steps: [tree('Notice Board Panel'), async (page) => {
    const tint = page.locator('[data-region="inspector"] [role="group"][aria-labelledby$="tint--value-label"] input').first();
    await tint.scrollIntoViewIfNeeded();
    await tint.focus();
    await page.keyboard.press('Shift+Tab');
    await page.keyboard.press('Tab');
  }], sizes: ['normal'] },
  { key: 'u-assets', ...U, steps: [openAssets], sizes: ['normal', 'minimum'] },
  { key: 'u-asset-atlas', ...U, steps: [asset('harbor_ui')], sizes: ['normal', 'minimum', 'narrow'] },
  { key: 'u-asset-atlas-long', ...U, steps: [asset('harbor_master_notice')], sizes: ['normal', 'narrow'] },
  { key: 'u-asset-sheet', ...U, steps: [asset('gull_sheet')], sizes: ['normal'] },
  // The UI's import-path rules against what the union's importer accepts.
  {
    key: 'u-import-mirror',
    ...union({ capabilities: ['asset.import'], assetImport: { available: true } }),
    steps: [async (page, obs) => {
      await openAssets(page);
      await page.getByRole('button', { name: 'Import' }).click();
      const first = page.getByRole('textbox', { name: 'Path 1' });
      await first.fill('sprites/dock_props.atlas.json');
      await first.press('Shift+Enter');
      await page.getByRole('textbox', { name: 'Path 2' }).fill('audio/gulls.ogg');
      await page.getByRole('textbox', { name: 'Path 2' }).press('Shift+Enter');
      await page.getByRole('textbox', { name: 'Path 3' }).fill('textures/dock_sign.png');
      await page.locator('form.import-form button[type="submit"]').click();
      await page.waitForTimeout(150);
      obs.push({
        step: 'submit',
        rows: await page.locator('.import-row').evaluateAll((rows) => rows.map((row) => ({
          path: row.querySelector('input')?.value,
          problem: row.querySelector('.import-row__problem')?.textContent ?? null,
          kind: row.querySelector('.import-row__type')?.textContent ?? (row.querySelector('fieldset, [role="radiogroup"]') ? 'image (usage choice)' : null),
        }))),
        formats: await page.locator('.import-form__formats').textContent(),
        live: await page.locator('[role="status"]').evaluateAll((nodes) => nodes.map((n) => n.textContent.trim()).filter(Boolean)),
        focus: await page.evaluate(focusInfo),
      });
    }],
    sizes: ['normal'],
  },
  // ---- Account dialog, every state (synthetic labels) ----
  { key: 'acct-signed-out', ...fixture('sample', 'signed-out'), steps: [openAccount], sizes: ['normal', 'minimum'] },
  { key: 'acct-checking', ...fixture('sample', 'checking'), steps: [openAccount], sizes: ['normal'] },
  { key: 'acct-browser', ...fixture('sample', 'browser'), steps: [openAccount], sizes: ['normal', 'minimum'] },
  { key: 'acct-validating', ...fixture('sample', 'validating'), steps: [openAccount], sizes: ['normal'] },
  { key: 'acct-adding', ...fixture('sample', 'adding-browser'), steps: [openAccount], sizes: ['normal'] },
  { key: 'acct-signed-in', ...fixture('sample', 'signed-in'), steps: [openAccount], sizes: ['normal', 'minimum'] },
  { key: 'acct-multi', ...fixture('sample', 'signed-in-multi'), steps: [openAccount], sizes: ['normal', 'minimum'] },
  { key: 'acct-saved', ...fixture('sample', 'signed-out-saved'), steps: [openAccount], sizes: ['normal', 'minimum'] },
  { key: 'acct-error', ...fixture('sample', 'error'), steps: [openAccount], sizes: ['normal'] },
  { key: 'acct-error-signed-in', ...fixture('sample', 'error-while-signed-in'), steps: [openAccount], sizes: ['normal'] },
  { key: 'acct-cli-key', ...fixture('sample', 'signed-in-cli-key'), steps: [openAccount], sizes: ['normal'] },
  // ---- Titlebar: avatar only, host chrome (synthetic provider), inactive window ----
  { key: 'title-signed-out', ...fixture('sample', 'signed-out'), sizes: ['normal', 'minimum'] },
  { key: 'title-checking', ...fixture('sample', 'checking'), sizes: ['normal'] },
  { key: 'title-browser', ...fixture('sample', 'browser'), sizes: ['normal'] },
  { key: 'title-signed-in', ...fixture('sample', 'signed-in'), sizes: ['normal', 'minimum'] },
  { key: 'title-error', ...fixture('sample', 'error'), sizes: ['normal'] },
  { key: 'title-mac', ...union({ provider: SIGNED_IN, window: MAC }), sizes: ['normal', 'minimum'] },
  { key: 'title-mac-inactive', ...union({ provider: SIGNED_IN, window: { ...MAC, focused: false } }), sizes: ['normal'] },
  { key: 'title-windows', ...union({ provider: SIGNED_IN, window: WINDOWS }), sizes: ['normal', 'minimum'] },
  // The UI draws minimize/maximize/close only for advertised window capabilities; the double still refuses them.
  { key: 'title-windows-controls', ...union({ provider: SIGNED_IN, window: WINDOWS, capabilities: ['window.drag', 'window.minimize', 'window.maximize', 'window.close'] }), sizes: ['normal', 'minimum'] },
  // ---- Shortcuts ----
  { key: 'keys-dialog', ...fixture(), steps: [openShortcuts], sizes: ['normal', 'minimum'] },
  { key: 'keys-windows', ...union({ window: WINDOWS }), steps: [openShortcuts], sizes: ['normal', 'minimum'] },
  { key: 'keys-nomatch', ...fixture(), steps: [openShortcuts, async (page) => page.keyboard.type('zzz')], sizes: ['normal'] },
  // ---- Agent pane on the union double (the native bridge's own unavailable reason) ----
  { key: 'u-agent-native', ...U, sizes: ['normal'] },
  { key: 'u-no-selection', ...U, sizes: ['normal', 'minimum'] },
  // ---- Probes: record behaviour, screenshot the final state ----
  {
    key: 'probe-end-tab',
    ...U,
    steps: [tree('Rubble Pile'), component('Collider'), async (page, obs, snap) => {
      // The list opens scrolled to its first problem (Part 42), so select Part 1 with Home.
      const selected = page.locator('[data-region="inspector"] [role="listbox"] [role="option"][aria-selected="true"]');
      obs.push({ step: 'opened', parts: await page.evaluate(partsState) });
      await selected.click();
      await page.keyboard.press('Home');
      obs.push({ step: 'Part 1 (click the selected row, Home)', focus: await page.evaluate(focusInfo), parts: await page.evaluate(partsState) });
      await page.keyboard.press('End');
      obs.push({ step: 'End', focus: await page.evaluate(focusInfo), parts: await page.evaluate(partsState) });
      await snap('end');
      await page.keyboard.press('Tab');
      await page.waitForTimeout(150);
      obs.push({ step: 'Tab', focus: await page.evaluate(focusInfo), parts: await page.evaluate(partsState),
        aria: await page.locator('[data-region="inspector"] [role="listbox"]').ariaSnapshot().catch((e) => String(e)) });
      await page.keyboard.press('Shift+Tab');
      await page.keyboard.press('Home');
      obs.push({ step: 'Shift+Tab, Home', focus: await page.evaluate(focusInfo), parts: await page.evaluate(partsState) });
      await page.keyboard.press('PageDown');
      await page.keyboard.press('PageDown');
      obs.push({ step: 'PageDown x2', focus: await page.evaluate(focusInfo), parts: await page.evaluate(partsState) });
      await page.keyboard.press('End');
      await page.keyboard.press('Tab');
      await page.waitForTimeout(150);
      obs.push({ step: 'End, Tab again', focus: await page.evaluate(focusInfo), parts: await page.evaluate(partsState) });
    }],
    sizes: ['normal', 'minimum'],
  },
  {
    key: 'probe-grid-paging',
    ...U,
    axe: false,
    steps: [tree('District Grid'), async (page, obs, snap) => {
      await page.getByRole('button', { name: 'Show values' }).click();
      const table = page.locator('[data-region="inspector"] [aria-roledescription="value table"]');
      await table.scrollIntoViewIfNeeded();
      await table.focus();
      await page.waitForTimeout(150);
      obs.push({ step: 'Show values, focus table', table: await page.evaluate(tableState), focus: await page.evaluate(focusInfo) });
      for (let i = 0; i < 3; i += 1) await page.keyboard.press('PageDown');
      await page.waitForTimeout(250);
      obs.push({ step: 'PageDown x3', table: await page.evaluate(tableState) });
      await page.keyboard.press('End');
      await page.waitForTimeout(250);
      obs.push({ step: 'End', table: await page.evaluate(tableState) });
      await snap('end');
      for (let i = 0; i < 60; i += 1) await page.keyboard.press('ArrowRight');
      await page.waitForTimeout(250);
      obs.push({ step: 'ArrowRight x60', table: await page.evaluate(tableState) });
      await table.evaluate((el) => { el.scrollLeft = el.scrollWidth; });
      await page.waitForTimeout(250);
      obs.push({ step: 'scrolled to the last column', table: await page.evaluate(tableState) });
      await snap('corner');
      await page.keyboard.press('Home');
      await page.waitForTimeout(250);
      obs.push({ step: 'Home', table: await page.evaluate(tableState), focus: await page.evaluate(focusInfo) });
    }],
    sizes: ['normal'],
  },
  {
    key: 'probe-pager',
    ...PAGED,
    steps: [async (page, obs, snap) => {
      const status = '[data-region="dock"] .pager [role="status"]';
      await page.evaluate((selector) => {
        window.__announce = [];
        const watch = () => {
          const el = document.querySelector(selector);
          if (!el || el.__watched) return;
          el.__watched = true;
          new MutationObserver(() => window.__announce.push({ text: el.textContent, at: Math.round(performance.now()) })).observe(el, { childList: true, characterData: true, subtree: true });
        };
        watch();
        new MutationObserver(watch).observe(document.body, { childList: true, subtree: true });
      }, status);
      const statusbar = async () => (await page.locator('.statusbar').textContent())?.replace(/\s+/g, ' ').trim();
      await page.getByRole('button', { name: 'Next page' }).click();
      await page.waitForTimeout(120);
      obs.push({ step: 'Next page', range: await page.locator('.pager__range').first().textContent(), statusbar: await statusbar(), focus: await page.evaluate(focusInfo) });
      await page.getByRole('button', { name: /^Warnings/ }).click();
      await page.waitForTimeout(120);
      obs.push({ step: 'Hide warnings (filter resets the page)', range: await page.locator('.pager__range').first().textContent() });
      await page.getByRole('button', { name: /^Warnings/ }).click();
      await page.waitForTimeout(120);
      obs.push({ step: 'Show warnings', range: await page.locator('.pager__range').first().textContent() });
      await page.getByRole('button', { name: 'Next page' }).click();
      await page.waitForTimeout(200);
      obs.push({ step: 'Next page again (same range)', range: await page.locator('.pager__range').first().textContent(), statusbar: await statusbar() });
      await page.getByRole('button', { name: 'Last page' }).click();
      await page.waitForTimeout(120);
      // At the end the button is aria-disabled but keeps focus; Enter on it must do nothing.
      await page.keyboard.press('Enter');
      await page.waitForTimeout(120);
      obs.push({ step: 'Last page, then Enter on it at the end', range: await page.locator('.pager__range').first().textContent(), focus: await page.evaluate(focusInfo),
        disabled: await page.getByRole('button', { name: 'Last page' }).getAttribute('aria-disabled'),
        rowsMounted: await page.locator('[data-region="dock"] li').count() });
      obs.push({ step: 'announcements', sequence: await page.evaluate(() => window.__announce) });
      await snap('last');
    }],
    sizes: ['normal', 'minimum'],
  },
  {
    key: 'probe-account',
    ...fixture('sample', 'signed-in-multi'),
    steps: [async (page, obs, snap) => {
      const name = async () => accountButton(page).getAttribute('aria-label');
      await openAccount(page);
      await obsFocus(page, obs, 'opened (signed in, three accounts)', { button: await name() });
      await page.getByRole('button', { name: 'Use ada@example.com' }).click();
      await page.waitForTimeout(100);
      await obsFocus(page, obs, 'Use ada@example.com', { button: await name(), rows: await page.locator('.account__list li').evaluateAll((li) => li.map((n) => n.textContent.replace(/\s+/g, ' ').trim())) });
      await page.getByRole('button', { name: 'Add another account' }).click();
      await page.waitForTimeout(100);
      await obsFocus(page, obs, 'Add another account (browser step)', { status: await page.locator('#account-status').textContent(), button: await name() });
      await snap('adding');
      await page.getByRole('button', { name: 'Cancel sign-in' }).click();
      await page.waitForTimeout(100);
      await obsFocus(page, obs, 'Cancel sign-in', { button: await name() });
      await page.getByRole('button', { name: 'Sign out', exact: true }).click();
      await page.waitForTimeout(100);
      await obsFocus(page, obs, 'Sign out (question)', { question: await page.locator('#signout-question').textContent() });
      await snap('question');
      await page.keyboard.press('Escape');
      await page.waitForTimeout(100);
      await obsFocus(page, obs, 'Escape (question dismissed, dialog stays)', { dialogOpen: await page.getByRole('dialog').count() });
      await page.getByRole('button', { name: 'Sign out', exact: true }).click();
      await page.locator('.account__confirm').getByRole('button', { name: 'Sign out' }).click();
      await page.waitForTimeout(100);
      await obsFocus(page, obs, 'Confirm sign out', { status: await page.locator('#account-status').textContent(), button: await name() });
      await snap('signed-out');
      await page.keyboard.press('Escape');
      await page.waitForTimeout(100);
      await obsFocus(page, obs, 'Escape (dialog closed, focus returns)', { dialogOpen: await page.getByRole('dialog').count() });
    }],
    sizes: ['normal', 'minimum'],
  },
  {
    key: 'probe-shortcuts',
    ...fixture(),
    steps: [async (page, obs, snap) => {
      await page.locator('[data-region="viewport"]').click({ position: { x: 40, y: 80 } });
      await page.keyboard.press('?');
      await page.waitForTimeout(100);
      await obsFocus(page, obs, '? pressed', { dialog: await page.getByRole('dialog').count() });
      await page.keyboard.type('hierarchy');
      await page.waitForTimeout(100);
      obs.push({ step: 'typed "hierarchy"', status: await page.getByRole('dialog').locator('.search-field__status').textContent(), rows: await page.locator('.shortcuts__row').count() });
      await snap('filtered');
      await page.keyboard.press('Escape');
      obs.push({ step: 'Escape (clears the filter)', value: await page.locator('#shortcuts-filter').inputValue(), dialog: await page.getByRole('dialog').count() });
      await page.keyboard.press('Escape');
      await page.waitForTimeout(100);
      await obsFocus(page, obs, 'Escape (closes)', { dialog: await page.getByRole('dialog').count() });
      await openShortcuts(page);
      obs.push({ step: 'names of key cells (macOS notation)', cells: await page.evaluate(shortcutNames, ['Undo', 'Redo', 'Delete', 'Show or hide the hierarchy', 'Keyboard shortcuts', 'Move selection']) });
      for (let i = 0; i < 3; i += 1) await page.keyboard.press('Tab');
      await obsFocus(page, obs, 'Tab x3 (focus stays in the dialog)');
      await page.keyboard.press('Escape');
      await page.waitForTimeout(100);
      await obsFocus(page, obs, 'Escape from the button-opened dialog');
    }],
    sizes: ['normal'],
  },
  {
    key: 'probe-shortcuts-windows',
    ...union({ window: WINDOWS }),
    steps: [openShortcuts, async (page, obs) => {
      obs.push({ step: 'names of key cells (Windows notation)', cells: await page.evaluate(shortcutNames, ['Undo', 'Redo', 'Delete', 'Show or hide the hierarchy', 'Keyboard shortcuts', 'Send the message']) });
      await page.keyboard.type('ctrl shift');
      obs.push({ step: 'typed "ctrl shift"', status: await page.getByRole('dialog').locator('.search-field__status').textContent(), actions: await page.locator('.shortcuts__action').evaluateAll((n) => n.map((e) => e.firstChild.textContent)) });
    }],
    sizes: ['normal'],
  },
  {
    key: 'probe-f6',
    ...fixture(),
    steps: [async (page, obs) => {
      await page.locator('[data-region="viewport"]').click({ position: { x: 40, y: 80 } });
      const cycle = [];
      for (let i = 0; i < 6; i += 1) {
        await page.keyboard.press('F6');
        cycle.push((await page.evaluate(focusInfo)).region);
      }
      const back = [];
      for (let i = 0; i < 3; i += 1) {
        await page.keyboard.press('Shift+F6');
        back.push((await page.evaluate(focusInfo)).region);
      }
      obs.push({ step: 'F6 x6', regions: cycle });
      obs.push({ step: 'Shift+F6 x3', regions: back });
    }],
    sizes: ['normal'],
  },
  {
    key: 'probe-menu',
    ...fixture(),
    steps: [async (page, obs, snap) => {
      await tree('Crate 02')(page);
      await page.keyboard.press('Shift+F10');
      await page.waitForTimeout(100);
      obs.push({ step: 'Shift+F10', focus: await page.evaluate(focusInfo),
        items: await page.getByRole('menuitem').evaluateAll((items) => items.map((i) => ({ text: i.textContent.replace(/\s+/g, ' ').trim(), disabled: i.getAttribute('aria-disabled') }))) });
      await snap('open');
      await page.keyboard.press('ArrowDown');
      await obsFocus(page, obs, 'ArrowDown');
      await page.keyboard.press('Escape');
      await page.waitForTimeout(100);
      await obsFocus(page, obs, 'Escape (focus returns)', { menu: await page.getByRole('menu').count() });
    }],
    sizes: ['normal', 'minimum'],
  },
  {
    key: 'probe-reveal',
    ...U,
    steps: [async (page, obs, snap) => {
      await page.getByRole('button', { name: /compound part 41 rotation must be a unit quaternion/ }).click();
      await page.waitForTimeout(200);
      obs.push({ step: 'Problems row: compound part 41 rotation', focus: await page.evaluate(focusInfo), parts: await page.evaluate(partsState),
        selected: await page.locator('[role="treeitem"][aria-selected="true"]').first().textContent() });
      await snap('part');
      await page.getByRole('button', { name: /raw pixel rectangles address the packed page/ }).click();
      await page.waitForTimeout(200);
      obs.push({ step: 'Problems row: raw atlas rectangle warning', focus: await page.evaluate(focusInfo),
        selected: await page.locator('[role="treeitem"][aria-selected="true"]').first().textContent() });
      await snap('region');
      await page.getByRole('button', { name: /pixel-perfect cameras require/ }).click();
      await page.waitForTimeout(200);
      obs.push({ step: 'Problems row: pixel-perfect camera range', focus: await page.evaluate(focusInfo),
        selected: await page.locator('[role="treeitem"][aria-selected="true"]').first().textContent() });
    }],
    sizes: ['normal'],
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

/** Generic measurements, identical to 0033's measure() plus `union` extras at the end. */
function measure() {
  const round = (n) => Math.round(n * 10) / 10;
  const rect = (el) => {
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return { x: round(r.x), y: round(r.y), width: round(r.width), height: round(r.height) };
  };
  const visible = (el) => {
    const r = el.getBoundingClientRect();
    if (r.width === 0 || r.height === 0) return false;
    const style = getComputedStyle(el);
    return style.visibility !== 'hidden' && style.display !== 'none' && !el.closest('.visually-hidden');
  };
  const parse = (c) => (c.match(/[\d.]+/g) ?? []).slice(0, 3).map(Number);
  const lum = ([r, g, b]) => [r, g, b].map((v) => v / 255).map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4)).reduce((s, v, i) => s + v * [0.2126, 0.7152, 0.0722][i], 0);
  const bgOf = (el) => {
    for (let node = el; node; node = node.parentElement) {
      const bg = getComputedStyle(node).backgroundColor;
      if (bg && bg !== 'transparent' && !/rgba\(\d+, \d+, \d+, 0\)/.test(bg)) return bg;
    }
    return 'rgb(11, 12, 15)';
  };
  const contrast = (el) => {
    const a = lum(parse(getComputedStyle(el).color));
    const b = lum(parse(bgOf(el)));
    return Math.round(((Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05)) * 100) / 100;
  };
  const path = (el) => {
    const parts = [];
    for (let node = el; node && node !== document.body && parts.length < 4; node = node.parentElement) {
      parts.unshift(`${node.tagName.toLowerCase()}${node.classList.length ? `.${[...node.classList].slice(0, 2).join('.')}` : ''}`);
    }
    return parts.join(' > ');
  };
  const inspector = document.querySelector('[data-region="inspector"]');
  const agent = document.querySelector('[data-region="agent"]');
  const out = { inspector: rect(inspector), agent: rect(agent), viewport: rect(document.querySelector('[data-viewport-host]')) };
  out.pageHorizontalOverflow = document.documentElement.scrollWidth - document.documentElement.clientWidth;
  out.pageVerticalOverflow = document.documentElement.scrollHeight - document.documentElement.clientHeight;
  out.dockRows = document.querySelectorAll('[data-region="dock"] li').length;
  const inView = (el) => {
    const r = el.getBoundingClientRect();
    if (r.bottom <= 0 || r.top >= innerHeight || r.right <= 0 || r.left >= innerWidth) return false;
    for (let a = el.parentElement; a && a !== document.body; a = a.parentElement) {
      const s = getComputedStyle(a);
      if (!/(auto|scroll|hidden|clip)/.test(`${s.overflowX} ${s.overflowY}`)) continue;
      const ar = a.getBoundingClientRect();
      if (r.bottom <= ar.top || r.top >= ar.bottom || r.right <= ar.left || r.left >= ar.right) return false;
    }
    return true;
  };
  const words = (root) => {
    if (!root) return null;
    let count = 0;
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      const text = node.textContent.trim();
      const el = node.parentElement;
      if (text && el && visible(el) && inView(el)) count += text.split(/\s+/).length;
    }
    return count;
  };
  out.visibleWords = Object.fromEntries(
    Object.entries({
      titlebar: '.titlebar', banner: '.notice-region', hierarchy: '[data-region="hierarchy"]', viewport: '[data-region="viewport"]',
      dock: '[data-region="dock"]', inspector: '[data-region="inspector"]', agent: '[data-region="agent"]', statusbar: '.statusbar',
      dialog: '[role="dialog"]', menu: '[role="menu"]', page: 'body',
    }).map(([area, selector]) => [area, words(document.querySelector(selector))]),
  );
  // ---- union extras (not in 0033) ----
  const button = [...document.querySelectorAll('.titlebar button')].find((b) => /^ChatGPT/.test(b.getAttribute('aria-label') ?? ''));
  out.account = button ? { rect: rect(button), name: button.getAttribute('aria-label'), title: button.getAttribute('title'), visibleText: button.innerText.trim() } : null;
  const dialog = document.querySelector('[role="dialog"]');
  out.dialog = dialog ? { rect: rect(dialog), fits: (() => { const r = dialog.getBoundingClientRect(); return r.top >= 0 && r.left >= 0 && r.bottom <= innerHeight && r.right <= innerWidth; })(),
    bodyOverflow: (() => { const b = dialog.querySelector('.dialog__body'); return b ? b.scrollHeight - b.clientHeight : null; })() } : null;
  out.statusbarText = document.querySelector('.statusbar')?.innerText.replace(/\s+/g, ' ').trim() ?? null;
  out.live = [...document.querySelectorAll('[role="status"], [aria-live]')].map((n) => n.textContent.replace(/\s+/g, ' ').trim()).filter(Boolean);
  out.unsupportedNotices = [...document.querySelectorAll('[data-region="inspector"] .control--notice')].filter(visible).map((n) => {
    const label = document.getElementById(n.getAttribute('aria-labelledby') ?? '')?.textContent?.trim() ?? null;
    return { label, text: n.querySelector('.control--notice__text')?.textContent?.trim(), raw: n.querySelector('.control--notice__raw')?.textContent };
  });
  out.rows = [...document.querySelectorAll('[data-region="inspector"] .field')].filter(visible).slice(0, 60).map((row) => {
    const label = row.querySelector('.field__label')?.textContent?.replace(/\s+/g, ' ').trim();
    const value = [...row.querySelectorAll(':scope > .field__value input, :scope > .field__value [role="checkbox"]')].map((i) => i.value ?? i.textContent).join(' ');
    const axes = [...row.querySelectorAll(':scope > .field__value .vector__axis-name')].map((a) => a.textContent).join('');
    const tag = row.querySelector(':scope > .field__value .control__tag')?.textContent ?? '';
    const ref = row.querySelector(':scope > .field__value .ref__name')?.textContent;
    return `${label}: ${ref ? `${ref} ` : ''}${value}${axes ? ` [${axes}]` : ''}${tag ? ` (${tag})` : ''}`;
  });
  if (!inspector) return out;
  const box = inspector.getBoundingClientRect();
  const controls = [...inspector.querySelectorAll('input, textarea, [role="checkbox"], [role="listbox"], [role="group"][tabindex="0"]')].filter(visible);
  const first = controls[0];
  out.firstValueTop = first ? round(first.getBoundingClientRect().top - box.top) : null;
  const scrollers = [...inspector.querySelectorAll('*')].filter((el) => {
    const s = getComputedStyle(el);
    return (s.overflowY === 'auto' || s.overflowY === 'scroll') && el.clientHeight > 40;
  });
  const main = scrollers[0];
  out.fixedChrome = main ? round(main.getBoundingClientRect().top - box.top) : null;
  out.horizontalOverflow = scrollers.map((el) => ({ el: path(el), overflow: el.scrollWidth - el.clientWidth })).filter((o) => o.overflow > 0);
  out.clippedValues = [...inspector.querySelectorAll('input')].filter(visible).filter((el) => el.scrollWidth > el.clientWidth + 1)
    .map((el) => ({ value: el.value, name: el.getAttribute('aria-label') ?? document.getElementById(el.getAttribute('aria-labelledby') ?? '')?.textContent ?? null, width: el.clientWidth, needed: el.scrollWidth }));
  out.clippedText = [...inspector.querySelectorAll('*')].filter((el) => visible(el) && el.children.length === 0 && el.textContent.trim() && !(el instanceof HTMLInputElement))
    .filter((el) => {
      const s = getComputedStyle(el);
      const ellipsis = s.textOverflow === 'ellipsis' && el.scrollWidth > el.clientWidth + 1;
      const clamped = s.webkitLineClamp !== 'none' && s.webkitLineClamp !== '' && el.scrollHeight > el.clientHeight + 1;
      return ellipsis || clamped;
    })
    .map((el) => ({ text: el.textContent.trim().slice(0, 80), el: path(el) }));
  out.outOfBounds = [...inspector.querySelectorAll('*')].filter(visible).filter((el) => {
    const r = el.getBoundingClientRect();
    return r.right > box.right + 0.5 || r.left < box.left - 0.5;
  }).slice(0, 20).map((el) => ({ el: path(el), rect: rect(el) }));
  const headers = [...inspector.querySelectorAll('section h3, section [aria-expanded]')].filter(visible);
  out.componentHeaders = headers.slice(0, 12).map((el) => ({ text: el.textContent.trim().slice(0, 40), height: round(el.getBoundingClientRect().height) }));
  const rows = [...inspector.querySelectorAll('.field')].filter(visible);
  out.rowHeights = rows.slice(0, 40).map((el) => round(el.getBoundingClientRect().height));
  const label = inspector.querySelector('.field__label');
  const value = inspector.querySelector('.field input');
  out.contrast = { label: label ? contrast(label) : null, value: value ? contrast(value) : null };
  const active = document.activeElement;
  if (active && inspector.contains(active)) {
    const ringed = [active, active.parentElement, active.parentElement?.parentElement].find((el) => el && getComputedStyle(el).outlineStyle !== 'none');
    out.focus = { name: active.getAttribute('aria-label') ?? document.getElementById(active.getAttribute('aria-labelledby') ?? '')?.textContent ?? null,
      value: active.value ?? null, outline: ringed ? `${getComputedStyle(ringed).outlineStyle} ${getComputedStyle(ringed).outlineWidth} ${getComputedStyle(ringed).outlineColor}` : null, ring: rect(ringed) };
  }
  return out;
}

/** One capture. Returns a report entry; never throws for page-side failures. */
async function captureOne(browser, origin, state, size) {
  const context = await browser.newContext({ viewport: { width: size.width, height: size.height }, deviceScaleFactor: 2, colorScheme: 'dark', bypassCSP: true });
  const file = `${state.key}-${size.key}-${size.width}x${size.height}.png`;
  const entry = { file, crops: {}, steps: {}, state: state.key, size, stepError: null, timedOut: false, errors: [], remoteRequests: [], observations: [] };
  const page = await context.newPage();
  page.on('pageerror', (e) => entry.errors.push(String(e)));
  page.on('console', (m) => m.type() === 'error' && entry.errors.push(m.text()));
  page.on('request', (r) => !r.url().startsWith(origin) && !r.url().startsWith('data:') && entry.remoteRequests.push(r.url()));
  const snap = async (name) => {
    entry.steps[name] = file.replace('.png', `--${name}.png`);
    await page.screenshot({ path: join(outDir, entry.steps[name]) });
  };
  const work = (async () => {
    if (state.init) await page.addInitScript(state.init);
    await page.goto(`${origin}${state.url}`, { waitUntil: 'networkidle' });
    if (size.inspector) {
      const splitter = page.getByRole('separator', { name: /^Resize inspector/ });
      if (await splitter.count()) {
        await splitter.focus();
        await page.keyboard.press(size.inspector);
        await page.locator('body').click({ position: { x: 1, y: size.height - 2 } }).catch(() => undefined);
      }
    }
    try {
      for (const step of state.steps ?? []) await step(page, entry.observations, snap);
    } catch (error) {
      entry.stepError = String(error).slice(0, 300);
    }
    await page.waitForTimeout(150);
    entry.measure = await page.evaluate(measure);
    if (state.axe !== false) {
      await page.addScriptTag({ path: axePath });
      entry.axe = await page.evaluate(async () => {
        // eslint-disable-next-line no-undef
        const result = await axe.run(document, { resultTypes: ['violations'] });
        return result.violations.map((v) => ({ id: v.id, impact: v.impact, nodes: v.nodes.length, targets: v.nodes.slice(0, 3).map((n) => n.target.join(' ')) }));
      });
    }
    await page.screenshot({ path: join(outDir, file) });
    const crop = async (name, locator) => {
      if ((await locator.count()) === 0) return;
      const box = await locator.first().boundingBox().catch(() => null);
      if (!box || box.width < 2 || box.height < 2) return;
      entry.crops[name] = file.replace('.png', `-${name}.png`);
      await page.screenshot({ path: join(outDir, entry.crops[name]), clip: box });
    };
    await crop('inspector', page.locator('[data-region="inspector"]'));
    await crop('agent', page.locator('[data-region="agent"]'));
    await crop('side', page.locator('.column--side'));
    await crop('titlebar', page.locator('.titlebar'));
    await crop('dialog', page.locator('[role="dialog"]'));
    await crop('dock', page.locator('[data-region="dock"]'));
  })();
  const limit = new Promise((done) => setTimeout(() => done('timeout'), 90_000));
  if ((await Promise.race([work.then(() => 'done', (error) => { entry.stepError ??= String(error).slice(0, 300); return 'done'; }), limit])) === 'timeout') {
    entry.timedOut = true;
    work.catch(() => undefined);
  }
  await Promise.race([context.close().catch(() => undefined), new Promise((done) => setTimeout(done, 10_000))]);
  return entry;
}

mkdirSync(outDir, { recursive: true });
const port = await freePort(4233);
const server = await preview({ root: uiRoot, preview: { port, strictPort: true, host: '127.0.0.1' }, logLevel: 'error' });
const origin = `http://127.0.0.1:${port}`;
const browser = await chromium.launch({ executablePath, headless: true });
const report = { set, generated: new Date().toISOString(), browser: browser.version(), port, uiRoot, captures: [] };
const save = () => writeFileSync(join(outDir, 'report.json'), `${JSON.stringify(report, null, 2)}\n`);
try {
  for (const state of STATES) {
    if (only && !only.includes(state.key)) continue;
    for (const size of SIZES) {
      if (!state.sizes.includes(size.key) || (sizeFilter && !sizeFilter.includes(size.key))) continue;
      const entry = await captureOne(browser, origin, state, size);
      report.captures.push(entry);
      save();
      const axeIds = (entry.axe ?? []).map((v) => v.id);
      console.log(`${entry.file}${entry.timedOut ? '  TIMED OUT' : ''}${entry.stepError ? `  STEP ERROR ${entry.stepError}` : ''}${axeIds.length ? `  axe:${axeIds.join(',')}` : ''}${entry.errors.length ? `  page errors:${entry.errors.length}` : ''}`);
    }
  }
} finally {
  await browser.close();
  await new Promise((done) => server.httpServer.close(done));
}
save();
console.log(`Wrote ${report.captures.length} captures to ${outDir}`);
