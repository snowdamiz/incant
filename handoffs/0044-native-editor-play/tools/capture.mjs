#!/usr/bin/env node
/**
 * BROWSER review of native Play controls and runtime inspection, handoff 0044.
 *
 * Headless Chrome on the real production UI build (editor/ui/dist). The UI talks to the
 * REAL compiled native bridge (editor/bridge/native.generated.js), which talks to the
 * NATIVE-BRIDGE DOUBLE in play-double.mjs instead of the Tauri host. So the requests and
 * their wiring are real, every Play result is SIMULATED, and nothing here is native
 * WebKit, a native window, the GPU surface, a screen reader or a device. Native visual
 * acceptance waits for Astra's binary and graphics-slot grant.
 *
 * Usage (repo root, after `npm run build --workspace editor/ui`):
 *   node handoffs/0044-native-editor-play/tools/capture.mjs <set> [--only=a,b] [--sizes=normal,minimum,narrow]
 * Output: a NEW ignored directory artifacts/0044-native-editor-play/<set>/ (an existing
 * one is refused): PNG captures and evidence.json.
 */
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { createRequire } from 'node:module';
import { join } from 'node:path';
import { chromium } from 'playwright-core';
import { preview } from 'vite';
import { DOUBLE_LABEL, initScript, repoRoot } from './play-double.mjs';

const uiRoot = join(repoRoot, 'editor/ui');
const args = process.argv.slice(2);
const set = args.find((a) => !a.startsWith('--'));
if (!set || !/^[a-z0-9-]+$/.test(set)) throw new Error('Usage: capture.mjs <set-name> [--only=a,b] [--sizes=normal,minimum,narrow]');
const flag = (name) => args.find((a) => a.startsWith(`--${name}=`))?.split('=')[1]?.split(',').filter(Boolean);
const only = flag('only');
const sizeFilter = flag('sizes');
const outDir = join(repoRoot, 'artifacts/0044-native-editor-play', set);
if (existsSync(outDir)) throw new Error(`${outDir} exists; choose a new set name.`);
const axePath = createRequire(join(uiRoot, 'package.json')).resolve('axe-core/axe.min.js');
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find((p) => p && existsSync(p));
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');
if (!existsSync(join(uiRoot, 'dist/index.html'))) throw new Error('Run the UI build first.');

/** Window sizes. `inspector: 'Home'` moves the Inspector splitter to its 280 px minimum by keyboard. */
const SIZES = [
  { key: 'normal', width: 1440, height: 900 },
  { key: 'minimum', width: 1000, height: 650 },
  { key: 'narrow', width: 1440, height: 900, inspector: 'Home' },
];

/* ---------- page helpers ---------- */

const settle = (page, ms = 250) => page.waitForTimeout(ms);
/** One bridge status read is 250 ms; wait for two. */
const poll = (page) => page.waitForTimeout(600);
const double = (page, method, ...values) => page.evaluate(([name, list]) => window.__playDouble[name](...list), [method, values]);
const select = (page, name) => page.getByRole('treeitem', { name, exact: true }).click();
const button = (page, name) => page.locator('.play-controls').getByRole('button', { name, exact: true });
const statusText = (page) => page.locator('.panel--viewport .panel__header .status-pill').textContent();
async function waitStatus(page, text) {
  await page.waitForFunction((want) => document.querySelector('.panel--viewport .panel__header .status-pill')?.textContent?.includes(want), text, { timeout: 4000 });
}
/** Chooses a scene and camera in the header picker, without starting Play. */
async function choose(page, option) {
  await page.locator('.play-target').click();
  await page.getByRole('menu').waitFor();
  await page.getByRole('menuitemradio', { name: option }).click();
}
/** A running, unsuspended session of Harbor with the Player Camera, at tick `ticks`. */
async function playHarbor(page, ticks = 90, camera = /Harbor · Player Camera/) {
  await choose(page, camera);
  await button(page, 'Play').click();
  await settle(page, 100);
  await double(page, 'finishStart');
  await double(page, 'frame');
  await waitStatus(page, 'Playing');
  if (ticks > 1) await double(page, 'frames', ticks - 1);
  await poll(page);
}

/* ---------- page-side readers ---------- */

function shown() {
  const clean = (text) => (text ?? '').replace(/\s+/g, ' ').trim() || null;
  const round = (n) => Math.round(n * 10) / 10;
  const rect = (el) => {
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return { x: round(r.x), y: round(r.y), width: round(r.width), height: round(r.height) };
  };
  const header = document.querySelector('.panel--viewport > .panel__header');
  const controls = [...document.querySelectorAll('.play-controls button')].map((b) => ({
    name: b.getAttribute('aria-label'),
    disabled: b.getAttribute('aria-disabled') === 'true',
    title: b.getAttribute('title'),
    rect: rect(b),
  }));
  const el = document.activeElement;
  const runtime = document.querySelector('.runtime');
  const clipped = [...document.querySelectorAll('.panel--viewport .panel__header *, .runtime *')].filter((node) => {
    if (node.children.length > 0 || !node.textContent.trim()) return false;
    return getComputedStyle(node).textOverflow === 'ellipsis' && node.scrollWidth > node.clientWidth + 1;
  }).map((node) => clean(node.textContent));
  const headerBox = header?.getBoundingClientRect();
  const overflowing = header ? [...header.querySelectorAll('*')].filter((node) => {
    const r = node.getBoundingClientRect();
    return r.width > 0 && (r.right > headerBox.right + 0.5 || r.left < headerBox.left - 0.5);
  }).map((node) => node.className || node.tagName) : [];
  return {
    header: {
      status: clean(header?.querySelector('.status-pill')?.textContent),
      picker: clean(document.querySelector('.play-target')?.getAttribute('aria-label')),
      controls,
      overflowing,
      box: rect(header),
    },
    notice: clean(document.querySelector('.play-notice')?.textContent),
    runtime: runtime ? {
      text: clean(runtime.textContent),
      rows: [...runtime.querySelectorAll('.field')].map((row) => ({
        label: clean(row.querySelector('.field__label')?.textContent),
        values: [...row.querySelectorAll('input')].map((input) => input.value),
      })),
      busy: runtime.getAttribute('aria-busy') === 'true',
    } : null,
    menu: [...document.querySelectorAll('[role="menu"] [role^="menuitem"]')].map((item) => ({ name: clean(item.textContent), checked: item.getAttribute('aria-checked') })),
    gameView: (() => {
      const host = document.querySelector('[data-viewport-host]');
      return host ? { rect: rect(host), focusable: host.getAttribute('tabindex') === '0', focused: document.activeElement === host, ring: getComputedStyle(host).boxShadow, role: host.getAttribute('role') } : null;
    })(),
    focus: !el || el === document.body ? 'body' : `${el.tagName.toLowerCase()}${el.getAttribute('role') ? `[${el.getAttribute('role')}]` : ''} ${clean(el.getAttribute('aria-label') ?? el.textContent)?.slice(0, 60) ?? ''}`.trim(),
    statusLine: clean(document.querySelector('.statusbar__message')?.textContent),
    source: clean(document.querySelector('.statusbar__conn')?.textContent),
    inspectorWidth: round(document.querySelector('[data-region="inspector"]')?.getBoundingClientRect().width ?? 0),
    titlebarInset: round(document.querySelector('.titlebar__inset')?.getBoundingClientRect().width ?? 0),
    pageOverflow: [document.documentElement.scrollWidth - document.documentElement.clientWidth, document.documentElement.scrollHeight - document.documentElement.clientHeight],
    clipped,
  };
}

function hostState() {
  const d = window.__playDouble;
  if (!d) return null;
  // The last native placement the bridge sent (physical pixels), against the game view now.
  const host = document.querySelector('[data-viewport-host]').getBoundingClientRect();
  const dpr = window.devicePixelRatio;
  const bounds = d.viewportBounds.at(-1)?.rect ?? null;
  const expected = [host.x, host.y, host.width, host.height].map((v) => Math.round(v * dpr * 10) / 10);
  return {
    status: d.status(),
    commands: d.commands.length,
    inputs: d.inputs.length,
    refusedInputs: d.refusedInputs.map((r) => r.error),
    lastCommand: d.commands.at(-1) ?? null,
    bounds: { sent: bounds && bounds.map((v) => Math.round(v * 10) / 10), gameView: expected, match: !!bounds && bounds.every((v, i) => Math.abs(v - expected[i]) < 0.6) },
  };
}

/* ---------- scenarios ---------- */

const SCENARIOS = [
  {
    key: 'choose',
    note: 'Two scenes, Harbor with three cameras: the picker asks for a scene; Play opens it; choosing starts Play. Held in preparation; Stop is available.',
    sizes: ['normal', 'minimum', 'narrow'],
    steps: async (page, snap) => {
      await snap('stopped');
      await button(page, 'Play').click();
      await page.getByRole('menu').waitFor();
      await snap('picker');
      await page.getByRole('menuitemradio', { name: /Harbor · Player Camera/ }).click();
      await settle(page, 150);
      await snap('preparing');
      await poll(page);
      await snap('preparing-polled');
    },
  },
  {
    key: 'running',
    note: 'Running Harbor; Crate 01 selected: runtime Transform and Velocity at the inspected tick above the authored components.',
    sizes: ['normal', 'minimum', 'narrow'],
    steps: async (page, snap) => {
      await select(page, 'Crate 01');
      await playHarbor(page, 30);
      await snap('playing');
      await double(page, 'frames', 60);
      await poll(page);
      await snap('playing-later');
      await page.getByRole('button', { name: 'Read runtime state again' }).click();
      await settle(page, 300);
      await snap('refreshed');
    },
  },
  {
    key: 'pause-step',
    note: 'Pause (button), Step twice (shortcut), Resume: inspection after the pause and each step, labelled with its tick.',
    sizes: ['normal', 'minimum', 'narrow'],
    steps: async (page, snap) => {
      await select(page, 'Player');
      await playHarbor(page, 120);
      await button(page, 'Pause').click();
      await waitStatus(page, 'Paused');
      await settle(page, 300);
      await snap('paused');
      for (const n of [1, 2]) {
        await page.keyboard.press('Meta+Alt+KeyP');
        await settle(page, 60);
        await double(page, 'frame');
        await settle(page, 300);
        if (n === 2) await snap('stepped');
      }
      await page.keyboard.press('Meta+Shift+KeyP');
      await double(page, 'frame');
      await waitStatus(page, 'Playing');
      await settle(page, 300);
      await snap('resumed');
    },
  },
  {
    key: 'stop-preparing',
    note: 'Stop while the start is preparing; the cancelled startup finishes afterwards and must not show an error or replace the stopped state.',
    sizes: ['normal'],
    steps: async (page, snap) => {
      await button(page, 'Play').click();
      await page.getByRole('menuitemradio', { name: /Harbor · Dock Camera/ }).click();
      await poll(page);
      await button(page, 'Stop').click();
      await settle(page, 200);
      await snap('stopping');
      await double(page, 'finishStart');
      await poll(page);
      await snap('after-cancelled-startup');
    },
  },
  {
    key: 'stop-immediately',
    note: 'Stop pressed before any status shows the new session: queued, then sent once the generation is known.',
    sizes: ['normal'],
    steps: async (page, snap) => {
      // Choose first, so Play starts with one press.
      await choose(page, /Harbor · Player Camera/);
      await button(page, 'Play').click();
      await button(page, 'Stop').click();
      await settle(page, 60);
      await snap('stop-queued');
      await poll(page);
      await double(page, 'finishStart');
      await poll(page);
      await snap('stopped');
    },
  },
  {
    key: 'unsaved',
    double: { saved: false },
    note: 'An unsaved project: the engine refuses Play; the notice says so, once, and Dismiss clears it.',
    sizes: ['normal', 'minimum', 'narrow'],
    steps: async (page, snap) => {
      await choose(page, /Harbor · Player Camera/);
      await button(page, 'Play').click();
      await waitStatus(page, 'Play failed');
      await settle(page, 300);
      await snap('refused');
      await page.getByRole('button', { name: 'Dismiss' }).click();
      await poll(page);
      await snap('dismissed');
    },
  },
  {
    key: 'runtime-error',
    note: 'A terminal runtime failure: the session ends, its message stays until Dismiss or the next Play.',
    sizes: ['normal', 'minimum'],
    steps: async (page, snap) => {
      await select(page, 'Crate 01');
      await playHarbor(page, 240);
      await double(page, 'fail', 'Native surface was lost or rejected; reopen the editor to recreate it');
      await waitStatus(page, 'Play failed');
      await settle(page, 300);
      await snap('failed');
    },
  },
  {
    key: 'missing',
    note: 'Runtime inspection of an entity the game removed, and of an entity in the scene that is not playing.',
    sizes: ['normal', 'narrow'],
    steps: async (page, snap) => {
      await playHarbor(page, 30);
      await select(page, 'Tutorial Sign');
      await settle(page, 300);
      await snap('absent');
      await select(page, 'Title Camera');
      await settle(page, 300);
      await snap('other-scene');
    },
  },
  {
    key: 'suspended',
    note: 'The window lost focus: the worker holds the session; the header says Suspended once it lasts.',
    sizes: ['normal'],
    steps: async (page, snap) => {
      await playHarbor(page, 30);
      await double(page, 'frame', { focused: false });
      await poll(page);
      await snap('suspended');
    },
  },
  {
    key: 'game-input',
    note: 'Click to focus the game view (that press is not sent), then keys, buttons, motion and wheel; F6 leaves and releases.',
    sizes: ['normal', 'minimum'],
    steps: async (page, snap) => {
      await playHarbor(page, 30);
      const host = page.locator('[data-viewport-host]');
      const box = await host.boundingBox();
      await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
      await poll(page);
      await snap('focused');
      await page.keyboard.down('KeyW');
      await page.keyboard.down('ShiftLeft');
      await page.mouse.move(box.x + 40, box.y + 40);
      await page.mouse.move(box.x + 80, box.y + 60, { steps: 12 });
      await page.mouse.down({ button: 'left' });
      await page.mouse.wheel(0, 120);
      await page.mouse.up({ button: 'left' });
      await page.keyboard.up('ShiftLeft');
      await page.keyboard.up('KeyW');
      await page.keyboard.press('Slash');
      await settle(page, 400);
      await snap('sent');
      await page.keyboard.press('F6');
      await settle(page, 400);
      await snap('left');
    },
  },
  {
    key: 'transport',
    note: 'The Play status cannot be read (worker gone): distinct from a failed session; Stop stays offered for the session last seen.',
    sizes: ['normal'],
    steps: async (page, snap) => {
      await playHarbor(page, 30);
      await double(page, 'setReadable', false);
      await waitStatus(page, 'Play unavailable');
      await settle(page, 300);
      await snap('unreadable');
    },
  },
  {
    key: 'single',
    double: { variant: 'single' },
    note: 'One scene with one camera: no picker; Play starts it directly.',
    sizes: ['normal', 'minimum'],
    steps: async (page, snap) => {
      await snap('stopped');
      await select(page, 'Player');
      await button(page, 'Play').click();
      await settle(page, 100);
      await double(page, 'finishStart');
      await double(page, 'frame');
      await double(page, 'frames', 45);
      await poll(page);
      await snap('playing');
    },
  },
  {
    key: 'fixture',
    url: '?fixture=sample',
    noDouble: true,
    note: 'The read-only sample fixture advertises no Play: the controls are marked unavailable and say why.',
    sizes: ['normal'],
    steps: async (page, snap) => {
      await snap('unsupported');
      // aria-disabled stays clickable for people; Playwright needs force to click it.
      await button(page, 'Play').click({ force: true });
      await settle(page, 200);
      await snap('explained');
    },
  },
  {
    key: 'keyboard',
    note: 'Keyboard only: F6 to the viewport, Tab through the picker and transport (rings), Enter on Play opens the choice, arrows and Enter start; F6 later lands in the game view.',
    sizes: ['normal', 'minimum'],
    steps: async (page, snap) => {
      for (let n = 0; n < 6 && !(await page.evaluate(() => document.activeElement?.closest('[data-region="viewport"]'))); n += 1) await page.keyboard.press('F6');
      await page.keyboard.press('Tab');
      await snap('picker-focused');
      await page.keyboard.press('Tab');
      await snap('play-focused');
      await page.keyboard.press('Enter');
      await page.getByRole('menu').waitFor();
      await page.keyboard.press('ArrowDown');
      await snap('choosing');
      await page.keyboard.press('Enter');
      await settle(page, 100);
      await double(page, 'finishStart');
      await double(page, 'frame');
      await poll(page);
      await snap('started');
      for (let n = 0; n < 6 && !(await page.evaluate(() => document.activeElement?.matches('[data-viewport-host]'))); n += 1) await page.keyboard.press('F6');
      await poll(page);
      await snap('game-view');
    },
  },
  {
    key: 'older-host',
    double: { drop: ['play.inspect', 'play.input'] },
    note: 'A host without play.inspect and play.input: Play works, runtime inspection and game input are labelled unavailable.',
    sizes: ['normal'],
    steps: async (page, snap) => {
      await select(page, 'Player');
      await playHarbor(page, 30);
      await snap('labelled');
      const box = await page.locator('[data-viewport-host]').boundingBox();
      await page.mouse.click(box.x + 60, box.y + 60);
      await settle(page, 200);
      await snap('input-explained');
    },
  },
  {
    key: 'shortcuts',
    note: 'The shortcut reference with the Play section.',
    sizes: ['normal'],
    steps: async (page, snap) => {
      await page.getByRole('button', { name: 'Keyboard shortcuts' }).click();
      await page.getByRole('dialog').waitFor();
      await page.getByRole('searchbox', { name: 'Filter shortcuts' }).fill('play');
      await snap('filtered');
    },
  },
];

/* ---------- run ---------- */

async function freePort(from) {
  for (let port = from; port < from + 50; port += 1) {
    const ok = await new Promise((resolve) => {
      const server = createServer().once('error', () => resolve(false)).once('listening', () => server.close(() => resolve(true)));
      server.listen(port, '127.0.0.1');
    });
    if (ok) return port;
  }
  throw new Error('No free port.');
}

mkdirSync(outDir, { recursive: true });
const port = await freePort(4244);
const server = await preview({ root: uiRoot, logLevel: 'warn', preview: { port, strictPort: true, host: '127.0.0.1' } });
const origin = `http://127.0.0.1:${port}`;
const browser = await chromium.launch({ executablePath, headless: true });
const sha = (path) => createHash('sha256').update(readFileSync(path)).digest('hex');
const evidence = {
  set,
  generatedAt: new Date().toISOString(),
  browser: `Chrome ${browser.version()} (headless, playwright-core)`,
  double: DOUBLE_LABEL,
  build: { js: sha(join(uiRoot, 'dist/index.html')) },
  steps: [],
  errors: [],
  remote: [],
};

try {
  for (const size of SIZES.filter((s) => !sizeFilter || sizeFilter.includes(s.key))) {
    for (const scenario of SCENARIOS.filter((s) => (!only || only.includes(s.key)) && s.sizes.includes(size.key))) {
      const context = await browser.newContext({ viewport: { width: size.width, height: size.height }, deviceScaleFactor: 2, reducedMotion: 'reduce', colorScheme: 'dark', bypassCSP: true });
      const page = await context.newPage();
      page.on('pageerror', (error) => evidence.errors.push({ scenario: scenario.key, size: size.key, error: String(error) }));
      page.on('console', (message) => {
        if (message.type() === 'error' || message.type() === 'warning') evidence.errors.push({ scenario: scenario.key, size: size.key, console: message.text() });
      });
      page.on('request', (request) => {
        if (!request.url().startsWith(origin) && !request.url().startsWith('data:')) evidence.remote.push(request.url());
      });
      if (!scenario.noDouble) await page.addInitScript(initScript(scenario.double ?? {}));
      await page.goto(`${origin}/${scenario.url ?? ''}`, { waitUntil: 'networkidle' });
      await page.evaluate(() => document.fonts.ready);
      await page.waitForSelector('.app');
      await page.waitForFunction(() => document.querySelector('.tree [role="treeitem"]'));
      await poll(page);
      if (size.inspector) {
        await page.getByRole('separator', { name: 'Resize inspector and agent' }).focus();
        await page.keyboard.press(size.inspector);
        await page.locator('.tree [role="treeitem"]').first().click();
      }
      const snap = async (name) => {
        await settle(page, 120);
        const file = `${scenario.key}--${name}--${size.key}-${size.width}x${size.height}.png`;
        await page.screenshot({ path: join(outDir, file) });
        await page.addScriptTag({ path: axePath });
        const axe = await page.evaluate(async () => {
          // eslint-disable-next-line no-undef
          const result = await axe.run(document, { resultTypes: ['violations'] });
          return result.violations.map((v) => ({ id: v.id, impact: v.impact, nodes: v.nodes.map((n) => n.target.join(' ')) }));
        });
        evidence.steps.push({ scenario: scenario.key, step: name, size: size.key, file, sha256: sha(join(outDir, file)), shown: await page.evaluate(shown), host: await page.evaluate(hostState), axe });
        process.stdout.write(`${file}  ${axe.length ? `axe:${axe.map((v) => v.id).join(',')}` : 'axe:0'}\n`);
      };
      try {
        await scenario.steps(page, snap);
        if (!scenario.noDouble) evidence.steps.push({ scenario: scenario.key, size: size.key, step: 'record', commands: await page.evaluate(() => window.__playDouble.commands), inputs: await page.evaluate(() => window.__playDouble.inputs), refusedInputs: await page.evaluate(() => window.__playDouble.refusedInputs) });
      } catch (error) {
        evidence.errors.push({ scenario: scenario.key, size: size.key, step: 'scenario', error: String(error?.stack ?? error) });
        await page.screenshot({ path: join(outDir, `${scenario.key}--FAILED--${size.key}.png`) });
        process.stdout.write(`FAILED ${scenario.key} ${size.key}: ${error}\n`);
      }
      await context.close();
    }
  }
} finally {
  await browser.close();
  await new Promise((done) => server.httpServer.close(done));
}

writeFileSync(join(outDir, 'evidence.json'), `${JSON.stringify(evidence, null, 2)}\n`);
const violations = evidence.steps.reduce((sum, step) => sum + (step.axe?.length ?? 0), 0);
console.log(`steps: ${evidence.steps.filter((s) => s.file).length}, axe violations: ${violations}, errors: ${evidence.errors.length}, remote requests: ${evidence.remote.length}`);
if (evidence.errors.length) for (const error of evidence.errors) console.log('  ', JSON.stringify(error).slice(0, 400));
