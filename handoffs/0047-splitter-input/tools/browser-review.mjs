#!/usr/bin/env node
/**
 * BROWSER review for handoff 0047: splitter drags must not select text. This is headless
 * Chromium from the pinned playwright-core with real mouse and keyboard input; it is not
 * the native WebKit window, which the root reviews with its own binary.
 *
 * Builds four pages into the ignored handoffs/0047-splitter-input/artifacts/:
 *   harness-{before,after}  tools/harness: the real Splitter between selectable panes,
 *   editor-{before,after}   the editor UI, opened on ?fixture=sample.
 * "before" aliases Splitter.tsx to its copy from the base commit, the source the native
 * review found the defect in; "after" uses this worktree's. Each page is served with
 * `vite preview` on a free private port from 47470. Writes screenshots/browser/*.png and
 * evidence/browser-review.json, and exits 1 if any check on the "after" pages fails.
 *
 * Usage, from the worktree root with node_modules linked:
 *   node handoffs/0047-splitter-input/tools/browser-review.mjs
 */
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { createServer } from 'node:net';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import react from '@vitejs/plugin-react';
import { chromium } from 'playwright-core';
import { build, preview } from 'vite';

const BASE = 'dc5e620';
const here = dirname(fileURLToPath(import.meta.url));
const packet = resolve(here, '..');
const repoRoot = resolve(packet, '../..');
const uiRoot = join(repoRoot, 'editor/ui');
const splitterSource = join(uiRoot, 'src/components/Splitter.tsx');
const work = join(packet, 'artifacts');
const shots = join(packet, 'screenshots/browser');
const VIEWPORT = { width: 1280, height: 800 };
const EDITOR_VIEWPORT = { width: 1440, height: 900 };
// Only the fixed Splitter handles lost capture; React DOM builds its handler names at
// run time, so this prop key tells the two builds apart.
const AFTER_MARKER = /onLostPointerCapture:/;

const git = (...args) => execFileSync('git', args, { cwd: repoRoot, encoding: 'utf8' });
const sha256 = (text) => createHash('sha256').update(text).digest('hex');

rmSync(work, { recursive: true, force: true });
mkdirSync(join(work, 'before'), { recursive: true });
mkdirSync(shots, { recursive: true });
for (const name of readdirSync(shots)) if (name.endsWith('.png')) rmSync(join(shots, name));
const beforeSource = git('show', `${BASE}:editor/ui/src/components/Splitter.tsx`);
const beforePath = join(work, 'before/Splitter.tsx');
writeFileSync(beforePath, beforeSource);
const afterSource = readFileSync(splitterSource, 'utf8');

async function buildPage(kind, variant) {
  const outDir = join(work, `${kind}-${variant}`);
  await build({
    configFile: false,
    root: kind === 'editor' ? uiRoot : join(here, 'harness'),
    base: './',
    logLevel: 'warn',
    cacheDir: join(work, '.vite', `${kind}-${variant}`),
    plugins: [react()],
    resolve: variant === 'before' ? { alias: [{ find: /^.*\/components\/Splitter$/, replacement: beforePath }] } : {},
    build: { outDir, emptyOutDir: true, target: 'es2022', assetsInlineLimit: 0, sourcemap: false },
  });
  const scripts = readdirSync(join(outDir, 'assets')).filter((name) => name.endsWith('.js'));
  const marked = scripts.some((name) => AFTER_MARKER.test(readFileSync(join(outDir, 'assets', name), 'utf8')));
  if (marked !== (variant === 'after')) throw new Error(`${kind}-${variant} was built with the wrong Splitter.`);
  return outDir;
}

function freePort(from) {
  return new Promise((done, fail) => {
    const probe = createServer();
    probe.once('error', () => (from < 47570 ? freePort(from + 1).then(done, fail) : fail(new Error('No free port.'))));
    probe.listen(from, '127.0.0.1', () => probe.close(() => done(from)));
  });
}

async function serve(dir, from) {
  const port = await freePort(from);
  const server = await preview({
    configFile: false,
    root: work,
    logLevel: 'warn',
    cacheDir: join(work, '.vite', 'preview'),
    build: { outDir: dir },
    preview: { host: '127.0.0.1', port, strictPort: true },
  });
  return { server, url: `http://127.0.0.1:${port}/` };
}

/**
 * Records pointer capture, context menu and selectstart events, before the page's modules
 * run. selectstart is the browser starting a text selection: the press's default action.
 */
const RECORD_EVENTS = () => {
  window.__events = [];
  for (const type of ['selectstart', 'gotpointercapture', 'lostpointercapture', 'pointercancel', 'contextmenu']) {
    document.addEventListener(type, (event) => window.__events.push({ type, pointerId: event.pointerId ?? null }), true);
  }
};

function readState(page, label) {
  return page.evaluate((name) => {
    const separator = [...document.querySelectorAll('[role="separator"]')].find((element) => element.getAttribute('aria-label') === name);
    const selection = window.getSelection();
    const active = document.activeElement;
    return {
      value: Number(separator?.getAttribute('aria-valuenow')),
      selectedText: selection?.toString() ?? '',
      collapsed: selection?.isCollapsed ?? true,
      focus: active === separator ? 'separator' : (active?.getAttribute('aria-label') ?? active?.tagName.toLowerCase() ?? null),
      focusVisible: separator?.matches(':focus-visible') ?? false,
      keyboardRing: separator?.hasAttribute('data-focus-visible') ?? false,
      events: window.__events.splice(0),
    };
  }, label);
}

const clearSelection = (page) => page.evaluate(() => window.getSelection()?.removeAllRanges());
// Finishes CSS transitions first: the separator's focus colour fades in.
const capture = (page, name) => page.screenshot({ path: join(shots, name), animations: 'disabled' });

/** The separator's centre, or a point on it level with `across` (a line of text). */
async function center(page, label, across = {}) {
  const box = await page.getByRole('separator', { name: label }).boundingBox();
  if (!box) throw new Error(`${label} is not visible.`);
  return { x: across.x ?? box.x + box.width / 2, y: across.y ?? box.y + box.height / 2 };
}

/**
 * Presses at `from`, moves through `path` sampling the page after each leg, optionally
 * captures the page with the button still held, then releases at the last point.
 */
async function drag(page, label, from, path, { button = 'left', modifier, shot } = {}) {
  await page.mouse.move(from.x, from.y);
  await page.evaluate(() => window.__events.splice(0));
  if (modifier) await page.keyboard.down(modifier);
  await page.mouse.down({ button });
  const samples = [];
  for (const point of path) {
    await page.mouse.move(point.x, point.y, { steps: 8 });
    samples.push(await readState(page, label));
  }
  if (shot) await capture(page, shot);
  await page.mouse.up({ button });
  if (modifier) await page.keyboard.up(modifier);
  const after = await readState(page, label);
  return { samples, after, events: [...samples, after].flatMap(types) };
}

const noSelection = (states) => states.every((state) => state.selectedText === '' && state.collapsed);
function types(state) {
  return state.events.map((event) => event.type);
}

async function reviewHarness(context, url, variant, layout) {
  const page = await context.newPage();
  await page.addInitScript(RECORD_EVENTS);
  await page.goto(`${url}?layout=${layout}&variant=${variant}`);
  await page.evaluate(() => document.fonts.ready);
  const vertical = layout === 'vertical';
  const label = vertical ? 'Resize left pane' : 'Resize bottom pane';
  const max = vertical ? 560 : 420;
  // The screen direction that grows the controlled pane: right, or up for the inverted rows.
  const grow = vertical ? 1 : -1;
  const along = (point, distance) => (vertical ? { x: point.x + distance, y: point.y } : { x: point.x, y: point.y + distance });
  // Presses on the column separator happen level with a paragraph, so drags cross text.
  const prose = await page.locator('[data-prose="first"]').boundingBox();
  const level = vertical ? { y: prose.y + prose.height / 2 } : {};
  const result = { variant, layout };

  // 0. A pointer drag after the keyboard focused the text field: the separator takes the
  //    focus a press gives, without :focus-visible or the shell's keyboard ring. Script
  //    focus without focusVisible: false would match :focus-visible here in Chromium.
  await page.keyboard.press('Tab');
  const tabbed = await readState(page, label);
  let from = await center(page, label, level);
  result.keyboardThenPointer = { tabbed, ...(await drag(page, label, from, [along(from, grow * 30)])) };

  // 1. A primary drag from a text field focused by pointer: within bounds, then past the
  //    clamp, so the pointer crosses the other pane's text. Then hover back without a button.
  await page.getByRole('textbox').click();
  const start = await readState(page, label);
  from = await center(page, label, level);
  const path = [along(from, grow * 120), along(from, grow * (vertical ? 600 : 400))];
  const primary = await drag(page, label, from, path, { shot: `harness-${layout}-${variant}-drag.png` });
  await page.mouse.move(from.x - grow * (vertical ? 120 : 0), from.y - grow * (vertical ? 0 : 120), { steps: 8 });
  const hover = await readState(page, label);
  result.primaryDrag = { start, ...primary, hoverAfterRelease: hover };

  // 2. The keyboard continues from the separator the press focused.
  await page.keyboard.press(vertical ? 'ArrowLeft' : 'ArrowDown');
  result.keyboardAfterDrag = await readState(page, label);
  await capture(page, `harness-${layout}-${variant}-keyboard.png`);

  // 3. A drag that starts on text still selects it.
  await clearSelection(page);
  const paragraph = await page.locator('[data-prose="second"]').boundingBox();
  const textStart = { x: paragraph.x + 1, y: paragraph.y + 6 };
  const textEnd = { x: paragraph.x + paragraph.width * 0.6, y: paragraph.y + paragraph.height - 6 };
  result.textSelection = await drag(page, label, textStart, [textEnd], { shot: `harness-${layout}-${variant}-text-selection.png` });
  await clearSelection(page);

  // 4. A secondary-button press and move, and 5. a Control-click, do not resize.
  from = await center(page, label, level);
  result.secondaryButton = await drag(page, label, from, [along(from, grow * -80)], { button: 'right' });
  await page.keyboard.press('Escape');
  from = await center(page, label, level);
  result.controlClick = await drag(page, label, from, [along(from, grow * -80)], { modifier: 'Control' });
  await page.keyboard.press('Escape');

  // 6. Losing pointer capture mid-drag ends the drag, even with the button still held: a
  //    2 px nudge that stays on the separator's drag target must not resize, and moving
  //    on across the text must not select it.
  await clearSelection(page);
  from = await center(page, label, level);
  await page.mouse.move(from.x, from.y);
  await page.evaluate(() => window.__events.splice(0));
  await page.mouse.down();
  const moved = along(from, grow * -40);
  await page.mouse.move(moved.x, moved.y, { steps: 4 });
  const held = await readState(page, label);
  const pointerId = held.events.find((event) => event.type === 'gotpointercapture')?.pointerId ?? null;
  await page.evaluate(([name, id]) => {
    const separator = [...document.querySelectorAll('[role="separator"]')].find((element) => element.getAttribute('aria-label') === name);
    if (id !== null) separator?.releasePointerCapture(id);
  }, [label, pointerId]);
  const released = await readState(page, label);
  const nudge = along(moved, grow * -2);
  await page.mouse.move(nudge.x, nudge.y);
  const nudged = await readState(page, label);
  const across = along(moved, grow * -160);
  await page.mouse.move(across.x, across.y, { steps: 8 });
  const afterRelease = await readState(page, label);
  await capture(page, `harness-${layout}-${variant}-lost-capture.png`);
  await page.mouse.up();
  result.lostCapture = { pointerId, held, released, nudged, afterRelease, afterUp: await readState(page, label) };
  await page.close();
  return result;
}

async function reviewEditor(context, url, variant) {
  const page = await context.newPage();
  await page.setViewportSize(EDITOR_VIEWPORT);
  await page.addInitScript(RECORD_EVENTS);
  await page.goto(`${url}?fixture=sample`);
  await page.evaluate(() => document.fonts.ready);
  await page.getByRole('treeitem', { name: 'Player', exact: true }).click();
  await page.locator('[data-region="inspector"]').getByText('Translation', { exact: true }).first().waitFor({ timeout: 5000 });
  const result = { variant };

  // The Inspector's separator: grow the Inspector, then push past its minimum so the
  // pointer crosses the Inspector's fields, where the native review saw the selection.
  const inspector = 'Resize inspector and agent';
  let from = await center(page, inspector);
  from = { x: from.x, y: 260 };
  result.inspectorDrag = await drag(page, inspector, from, [{ x: from.x - 100, y: from.y }, { x: from.x + 300, y: from.y + 40 }], {
    shot: `editor-${variant}-inspector-drag.png`,
  });

  // The output dock's separator: grow the dock, then push past its minimum across its text.
  const dock = 'Resize output panel';
  from = await center(page, dock);
  result.dockDrag = await drag(page, dock, from, [{ x: from.x, y: from.y - 100 }, { x: from.x + 40, y: from.y + 200 }], {
    shot: `editor-${variant}-dock-drag.png`,
  });
  await capture(page, `editor-${variant}-resized.png`);

  // A press on a separator closes an open menu, like any press outside it.
  await clearSelection(page);
  await page.getByRole('treeitem', { name: 'Player', exact: true }).click({ button: 'right' });
  const menuOpened = (await page.getByRole('menu').count()) === 1;
  from = await center(page, inspector);
  await page.mouse.move(from.x, 300);
  await page.mouse.down();
  const menuAfterPress = await page.getByRole('menu').count();
  await page.mouse.up();
  result.menu = { menuOpened, menuOpenAfterPress: menuAfterPress > 0, state: await readState(page, inspector) };

  // A drag that starts on Inspector text still selects it.
  await clearSelection(page);
  const labels = page.locator('[data-region="inspector"]').getByText('Translation', { exact: true });
  const scale = page.locator('[data-region="inspector"]').getByText('Scale', { exact: true });
  const first = await labels.first().boundingBox();
  const last = await scale.first().boundingBox();
  if (first && last) {
    result.textSelection = await drag(page, inspector, { x: first.x + 1, y: first.y + first.height / 2 }, [{ x: last.x + last.width - 1, y: last.y + last.height / 2 }], {
      shot: `editor-${variant}-text-selection.png`,
    });
  } else {
    result.textSelection = { error: 'Inspector labels not found.' };
  }
  await page.close();
  return result;
}

function checks(harness, editor) {
  const list = [];
  const check = (name, pass, detail) => list.push({ name, pass: Boolean(pass), detail });
  for (const run of harness) {
    const max = run.layout === 'vertical' ? 560 : 420;
    const key = `harness ${run.layout}`;
    const { primaryDrag: drag, keyboardAfterDrag: keyed } = run;
    check(`${key}: a primary press starts no text selection (no selectstart)`, !drag.events.includes('selectstart'), drag.events);
    check(`${key}: no selection during or after a primary drag`, noSelection([...drag.samples, drag.after]), drag.samples.map((s) => s.selectedText.length));
    check(`${key}: the drag resizes and clamps at ${max}`, drag.samples[0].value !== drag.start.value && drag.after.value === max, [drag.start.value, ...drag.samples.map((s) => s.value), drag.after.value]);
    check(`${key}: the press focuses the separator without a focus ring`, drag.after.focus === 'separator' && !drag.after.focusVisible && !drag.after.keyboardRing, drag.after);
    const typed = run.keyboardThenPointer;
    check(`${key}: after a keyboard-focused field, the press still focuses the separator without a ring`, typed.tabbed.focus === 'input' && typed.after.focus === 'separator' && !typed.after.focusVisible && !typed.after.keyboardRing, typed.after);
    check(`${key}: the pointer is captured and released`, drag.events.includes('gotpointercapture') && drag.events.includes('lostpointercapture'), drag.events);
    check(`${key}: hovering back without a button does not resize`, drag.hoverAfterRelease.value === max, drag.hoverAfterRelease.value);
    check(`${key}: the keyboard continues from the separator, with a ring`, keyed.value === max - 16 && keyed.focus === 'separator' && keyed.keyboardRing, keyed);
    const text = run.textSelection;
    check(`${key}: a drag on text still selects it`, text.events.includes('selectstart') && text.after.selectedText.length > 20 && text.after.value === keyed.value, text.after.selectedText);
    const { secondaryButton: secondary, controlClick: control } = run;
    check(`${key}: a secondary-button drag does not resize or capture`, secondary.after.value === keyed.value && !secondary.events.includes('gotpointercapture'), secondary.events);
    check(`${key}: a Control-click drag does not resize or capture, nor start a selection`, control.after.value === keyed.value && !control.events.includes('gotpointercapture') && !control.events.includes('selectstart') && noSelection([...control.samples, control.after]), control.events);
    const lost = run.lostCapture;
    const lostEvents = [lost.released, lost.nudged].flatMap(types);
    check(`${key}: losing capture ends the drag`, lost.pointerId !== null && lostEvents.includes('lostpointercapture') && lost.nudged.value === lost.released.value && lost.afterRelease.value === lost.released.value, [lost.held.value, lost.released.value, lost.nudged.value, lost.afterRelease.value, lostEvents]);
    check(`${key}: no selection after capture is lost mid-drag`, noSelection([lost.nudged, lost.afterRelease, lost.afterUp]), lost.afterUp.selectedText);
  }
  for (const [name, run] of [['Inspector', editor.inspectorDrag], ['output dock', editor.dockDrag]]) {
    check(`editor: pressing the ${name} separator starts no text selection (no selectstart)`, !run.events.includes('selectstart'), run.events);
    check(`editor: no selection while dragging the ${name} separator`, noSelection([...run.samples, run.after]), run.samples.map((s) => s.selectedText.slice(0, 80)));
    check(`editor: the ${name} separator resizes and takes focus without a ring`, run.samples[0].value !== run.samples[1].value && run.after.focus === 'separator' && !run.after.focusVisible && !run.after.keyboardRing, [...run.samples.map((s) => s.value), run.after.value]);
  }
  check('editor: a separator press closes an open menu', editor.menu.menuOpened && !editor.menu.menuOpenAfterPress && editor.menu.state.focus === 'separator', editor.menu);
  const text = editor.textSelection;
  check('editor: a drag on Inspector text still selects it', Boolean(text.events?.includes('selectstart') && text.after.selectedText.length > 10), text.after?.selectedText ?? text.error);
  return list;
}

const require = createRequire(join(uiRoot, 'package.json'));
const version = (name) => {
  try {
    return require(`${name}/package.json`).version;
  } catch {
    return 'unknown';
  }
};
const versions = { node: process.version, vite: version('vite'), playwrightCore: version('playwright-core'), react: version('react') };

const pages = {};
for (const kind of ['harness', 'editor']) for (const variant of ['before', 'after']) pages[`${kind}-${variant}`] = await buildPage(kind, variant);
const servers = [];
let port = 47470;
const urls = {};
for (const [name, dir] of Object.entries(pages)) {
  const served = await serve(dir, port);
  servers.push(served.server);
  urls[name] = served.url;
  port = Number(new URL(served.url).port) + 1;
}

const browser = await chromium.launch({ headless: true });
let report;
try {
  const context = await browser.newContext({ viewport: VIEWPORT, deviceScaleFactor: 1, colorScheme: 'dark' });
  const harness = {};
  const editor = {};
  for (const variant of ['before', 'after']) {
    harness[variant] = [];
    for (const layout of ['vertical', 'horizontal']) harness[variant].push(await reviewHarness(context, urls[`harness-${variant}`], variant, layout));
    editor[variant] = await reviewEditor(context, urls[`editor-${variant}`], variant);
  }
  const after = checks(harness.after, editor.after);
  const before = checks(harness.before, editor.before);
  report = {
    label: 'BROWSER EVIDENCE: headless Chromium driven by playwright-core with real mouse and keyboard input. Not the native WebKit window.',
    browser: `${browser.browserType().name()} ${browser.version()} (headless)`,
    versions,
    viewport: { harness: VIEWPORT, editor: EDITOR_VIEWPORT, deviceScaleFactor: 1 },
    source: {
      base: BASE,
      head: git('rev-parse', '--short', 'HEAD').trim(),
      splitterBeforeSha256: sha256(beforeSource),
      splitterAfterSha256: sha256(afterSource),
      afterIsWorkingTree: git('status', '--porcelain', '--', 'editor/ui/src/components/Splitter.tsx').trim() !== '' ? 'uncommitted changes in Splitter.tsx' : 'committed',
    },
    summary: {
      afterPassed: after.filter((c) => c.pass).length,
      afterFailed: after.filter((c) => !c.pass).map((c) => c.name),
      beforeFailed: before.filter((c) => !c.pass).map((c) => c.name),
    },
    checks: { after, before },
    harness,
    editor,
  };
} finally {
  await browser.close();
  await Promise.all(servers.map((server) => server.close()));
}

writeFileSync(join(packet, 'evidence/browser-review.json'), `${JSON.stringify(report, null, 2)}\n`);
console.log(report.browser);
for (const check of report.checks.after) console.log(`${check.pass ? 'PASS' : 'FAIL'} after: ${check.name}`);
for (const name of report.summary.beforeFailed) console.log(`before fails: ${name}`);
process.exit(report.summary.afterFailed.length === 0 ? 0 : 1);
