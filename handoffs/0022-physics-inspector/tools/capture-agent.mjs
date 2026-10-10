#!/usr/bin/env node
/**
 * BROWSER FIXTURE captures for the compact Agent empty state (handoff 0022 follow-up).
 * Serves editor/ui/dist with `vite preview` on port 4193 and drives local Chrome via the
 * pinned playwright-core. `?fixture=sample&provider=signed-in` reproduces the native
 * "Agent not ready" state (account connected, agent unavailable); `signed-out` adds the
 * ChatGPT button to the composer bar. The Agent panel is reduced to its 180 px minimum
 * with the splitter's keyboard Home control, as in the native captures. Synthetic data,
 * not native WebKit evidence. Writes PNGs + report.json to screenshots/<set>/.
 *
 * Usage (repo root, after `npm run build --workspace editor/ui`):
 *   node handoffs/0022-physics-inspector/tools/capture-agent.mjs <set>
 */
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import { preview } from 'vite';

const here = dirname(fileURLToPath(import.meta.url));
const uiRoot = join(resolve(here, '../../..'), 'editor/ui');
const set = process.argv[2] ?? 'agent';
const outDir = join(here, '..', 'screenshots', set);
mkdirSync(outDir, { recursive: true });
const axePath = createRequire(join(uiRoot, 'package.json')).resolve('axe-core/axe.min.js');
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'].find(
  (p) => p && existsSync(p),
);
if (!executablePath) throw new Error('No Chrome found; set CHROME_PATH.');

const STATES = [
  { key: 'agent-180-signed-in', size: { width: 1000, height: 650 }, provider: 'signed-in', minimum: true },
  { key: 'agent-180-signed-out', size: { width: 1000, height: 650 }, provider: 'signed-out', minimum: true },
  // Layout probe only: the native build's longer unavailable reason, substituted into the
  // rendered body text to check the overflow/scroll fallback. Not a different app state.
  {
    key: 'agent-180-native-reason-probe',
    size: { width: 1000, height: 650 },
    provider: 'signed-in',
    minimum: true,
    probeText: 'Use the headless agent command for the Phase 0 provider spike. Everything else works offline, no account needed.',
  },
  { key: 'agent-default-signed-in-1000', size: { width: 1000, height: 650 }, provider: 'signed-in' },
  { key: 'agent-default-signed-in-1440', size: { width: 1440, height: 900 }, provider: 'signed-in' },
  { key: 'agent-default-signed-out-1440', size: { width: 1440, height: 900 }, provider: 'signed-out' },
];

const server = await preview({ root: uiRoot, preview: { port: 4193, strictPort: true, host: '127.0.0.1' }, logLevel: 'error' });
const origin = 'http://127.0.0.1:4193';
const browser = await chromium.launch({ executablePath, headless: true });
const report = { set, generated: new Date().toISOString(), browser: browser.version(), captures: [] };
try {
  for (const state of STATES) {
    const context = await browser.newContext({ viewport: state.size, deviceScaleFactor: 2, colorScheme: 'dark', bypassCSP: true });
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', (e) => errors.push(String(e)));
    page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
    await page.goto(`${origin}/?fixture=sample&provider=${state.provider}`, { waitUntil: 'networkidle' });
    if (state.minimum) {
      await page.getByRole('separator', { name: 'Resize agent panel' }).focus();
      await page.keyboard.press('Home');
    }
    if (state.probeText) {
      await page.locator('.agent-empty__body').evaluate((el, text) => { el.textContent = text; }, state.probeText);
      await page.waitForTimeout(50); // let ResizeObserver report the new overflow
    }
    // Keyboard path into the transcript: Tab from the Agent heading area.
    await page.locator('[data-region="agent"]').focus();
    await page.keyboard.press('Tab');
    const measure = await page.evaluate(() => {
      const agent = document.querySelector('[data-region="agent"]');
      const transcript = agent.querySelector('.agent__transcript');
      const rect = (el) => {
        if (!el) return null;
        const r = el.getBoundingClientRect();
        return { x: Math.round(r.x), y: Math.round(r.y), width: Math.round(r.width), height: Math.round(r.height), bottom: Math.round(r.bottom) };
      };
      const visible = (el) => {
        if (!el) return null;
        const r = el.getBoundingClientRect();
        const t = transcript.getBoundingClientRect();
        return getComputedStyle(el).display !== 'none' && r.top >= t.top - 0.5 && r.bottom <= t.bottom + 0.5;
      };
      return {
        panel: rect(agent),
        transcript: { ...rect(transcript), scrollHeight: transcript.scrollHeight, clientHeight: transcript.clientHeight },
        composer: rect(agent.querySelector('.composer')),
        mark: { rect: rect(agent.querySelector('.agent-empty__mark')), visible: visible(agent.querySelector('.agent-empty__mark')) },
        title: { text: agent.querySelector('.agent-empty__title')?.textContent, rect: rect(agent.querySelector('.agent-empty__title')), visible: visible(agent.querySelector('.agent-empty__title')) },
        body: { rect: rect(agent.querySelector('.agent-empty__body')), visible: visible(agent.querySelector('.agent-empty__body')) },
        separator: document.querySelector('[role="separator"][aria-label="Resize agent panel"]')?.getAttribute('aria-valuenow'),
        transcriptTabIndex: transcript.getAttribute('tabindex'),
        activeElement: document.activeElement?.className || document.activeElement?.tagName,
        activeOutline: document.activeElement ? getComputedStyle(document.activeElement).outlineStyle : null,
        outputTabs: [...document.querySelectorAll('[role="tab"]')].map((t) => t.textContent?.trim()),
      };
    });
    await page.addScriptTag({ path: axePath });
    const axe = await page.evaluate(async () => {
      // eslint-disable-next-line no-undef
      const result = await axe.run(document, { resultTypes: ['violations'] });
      return result.violations.map((v) => ({ id: v.id, nodes: v.nodes.length, targets: v.nodes.slice(0, 3).map((n) => n.target.join(' ')) }));
    });
    const file = `${state.key}.png`;
    await page.screenshot({ path: join(outDir, file) });
    const side = await page.locator('[data-region="agent"]').boundingBox();
    const crop = `${state.key}-agent.png`;
    await page.screenshot({ path: join(outDir, crop), clip: side });
    let scrolled = null;
    if (measure.transcript.scrollHeight > measure.transcript.clientHeight) {
      // Keyboard scrolling of the focused transcript.
      await page.locator('.agent__transcript').focus();
      await page.keyboard.press('End');
      await page.waitForTimeout(100);
      scrolled = await page.locator('.agent__transcript').evaluate((el) => el.scrollTop);
      await page.screenshot({ path: join(outDir, `${state.key}-scrolled-agent.png`), clip: side });
    }
    report.captures.push({ file, crop, ...state, measure, axe, errors, keyboardScrollTop: scrolled });
    await context.close();
  }
} finally {
  await browser.close();
  await new Promise((done) => server.httpServer.close(done));
}
writeFileSync(join(outDir, 'report.json'), `${JSON.stringify(report, null, 2)}\n`);
console.log(`Wrote ${report.captures.length} captures to ${outDir}`);
