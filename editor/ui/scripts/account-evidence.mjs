#!/usr/bin/env node
/**
 * Real-browser evidence for handoff 0003 (ChatGPT account UI). Serves the
 * production build with `vite preview`, opens each simulated account fixture in
 * headless Chrome (playwright-core), opens the account dialog the way a user
 * would, and writes screenshots plus axe-core results (color contrast included)
 * to handoffs/0003-openai-login/screenshots/.
 *
 * These are browser fixture captures, NOT native evidence: no native host, no
 * system browser launch and no OpenAI sign-in happen here.
 *
 * Usage: npm run build --workspace editor/ui && node editor/ui/scripts/account-evidence.mjs
 * Env:   CHROME_PATH overrides the browser executable; EVIDENCE_PORT the preview port.
 */
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import { preview } from 'vite';

const here = dirname(fileURLToPath(import.meta.url));
const uiRoot = resolve(here, '..');
const repoRoot = resolve(uiRoot, '../..');
const outDir = join(repoRoot, 'handoffs/0003-openai-login/screenshots');
const require = createRequire(import.meta.url);
const axePath = require.resolve('axe-core/axe.min.js');
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome']
  .filter(Boolean)
  .find((path) => existsSync(path));
if (!executablePath) throw new Error('No Chrome found. Set CHROME_PATH.');
if (!existsSync(join(uiRoot, 'dist/index.html'))) throw new Error('Run the build first (dist/ is missing).');
mkdirSync(outDir, { recursive: true });

const STATES = ['checking', 'signed-out', 'signed-out-saved', 'browser', 'validating', 'signed-in', 'signed-in-multi', 'signed-in-cli-key', 'error', 'adding-browser', 'error-while-signed-in'];
const SIZE = { width: 1280, height: 800 };
const port = Number(process.env.EVIDENCE_PORT ?? 4183);
const server = await preview({ root: uiRoot, logLevel: 'warn', preview: { port, strictPort: true, host: '127.0.0.1' } });
const origin = `http://127.0.0.1:${port}`;
const browser = await chromium.launch({ executablePath, headless: true });
const rel = (path) => relative(repoRoot, path);
const evidence = { generatedAt: new Date().toISOString(), browser: `Chrome ${browser.version()} (headless)`, screenshots: [], axe: [] };

async function page(query, deviceScaleFactor = 2) {
  const context = await browser.newContext({ viewport: SIZE, deviceScaleFactor, bypassCSP: true, reducedMotion: 'reduce' });
  const p = await context.newPage();
  await p.goto(`${origin}/${query}`, { waitUntil: 'networkidle' });
  await p.evaluate(() => document.fonts.ready);
  return { context, p };
}

async function axe(p, label) {
  await p.addScriptTag({ path: axePath });
  const result = await p.evaluate(async () => {
    // eslint-disable-next-line no-undef
    const r = await axe.run(document, { resultTypes: ['violations'] });
    return r.violations.map((v) => ({ id: v.id, impact: v.impact, nodes: v.nodes.map((n) => n.target.join(' ')) }));
  });
  evidence.axe.push({ label, violations: result });
}

async function shoot(p, name, description, clip) {
  const path = join(outDir, `${name}.png`);
  if (clip) await p.locator(clip).screenshot({ path });
  else await p.screenshot({ path });
  evidence.screenshots.push({ file: rel(path), description });
}

for (const state of STATES) {
  const { context, p } = await page(`?fixture=sample&provider=${state}`);
  await shoot(p, `titlebar-${state}`, `Titlebar account chip, ${state}`, '.titlebar');
  await p.getByRole('button', { name: /^ChatGPT / }).click();
  await p.getByRole('dialog').waitFor();
  await shoot(p, `dialog-${state}`, `Account dialog opened from the titlebar, ${state}`, '.dialog');
  if (state === 'signed-out' || state === 'signed-in-multi') await shoot(p, `window-${state}`, `Full window with dialog, ${state}`);
  await axe(p, `dialog ${state}`);
  await context.close();
}

// Keyboard focus as rendered: Tab from the initial focus target.
{
  const { context, p } = await page('?fixture=sample&provider=signed-out');
  await p.keyboard.press('Escape');
  await p.getByRole('button', { name: /^ChatGPT / }).focus();
  await p.keyboard.press('Enter');
  await p.getByRole('dialog').waitFor();
  await shoot(p, 'focus-signed-out-initial', 'Initial focus on Continue with ChatGPT after opening with Enter', '.dialog');
  await context.close();
}
{
  const { context, p } = await page('?fixture=sample&provider=signed-in-multi');
  await p.getByRole('button', { name: /^ChatGPT / }).click();
  await p.getByRole('button', { name: 'Sign out' }).click();
  await shoot(p, 'dialog-signed-in-multi-confirm-signout', 'Sign-out confirmation with focus on the confirm button', '.dialog');
  await axe(p, 'dialog sign-out confirmation');
  await context.close();
}
// Agent panel entry points.
for (const state of ['signed-out', 'browser', 'error', 'signed-in']) {
  const { context, p } = await page(`?fixture=sample&provider=${state}`);
  await shoot(p, `agent-${state}`, `Agent panel, ${state}`, '[data-region="agent"]');
  if (state === 'signed-out') {
    await axe(p, 'shell signed-out');
    await p.locator('[data-region="agent"]').getByRole('button', { name: 'Continue with ChatGPT' }).click();
    await p.getByRole('dialog').waitFor();
    await shoot(p, 'flow-agent-continue', 'After clicking Continue with ChatGPT in the agent panel (simulated browser wait)', '.dialog');
  }
  await context.close();
}
// Narrow window: the dialog and chip must not clip.
{
  const context = await browser.newContext({ viewport: { width: 900, height: 600 }, deviceScaleFactor: 2, reducedMotion: 'reduce' });
  const p = await context.newPage();
  await p.goto(`${origin}/?fixture=sample&provider=signed-in-multi`, { waitUntil: 'networkidle' });
  await p.getByRole('button', { name: /^ChatGPT / }).click();
  await shoot(p, 'window-narrow-signed-in-multi', 'Narrow 900x600 window with dialog');
  await context.close();
}

await browser.close();
await server.close();
writeFileSync(join(outDir, 'evidence.json'), `${JSON.stringify(evidence, null, 2)}\n`);
const violations = evidence.axe.reduce((sum, run) => sum + run.violations.length, 0);
console.log(`Screenshots: ${evidence.screenshots.length} in ${rel(outDir)}`);
console.log(`axe runs: ${evidence.axe.length}, violations: ${violations}`);
for (const run of evidence.axe) for (const v of run.violations) console.log(`  ${run.label}: ${v.id} ${v.nodes.join(', ')}`);
