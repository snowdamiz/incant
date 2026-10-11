#!/usr/bin/env node
/**
 * Handoff 0040, check 4: one bounded headless-browser affordance check of the EXISTING
 * import form. It verifies that `.fbx`/`.FBX` paths are treated as models, that the
 * supported-format label advertises FBX, and that a stale image interpretation chosen
 * for an earlier path is not sent once the row holds an FBX path (import and reimport).
 *
 * Scope and boundaries:
 * - Builds this worktree's editor/ui sources with the already-installed, pinned vite
 *   (configFile: false, so no config bundle is written) into artifacts/ only; cacheDir
 *   is also under artifacts/. Nothing is written inside editor/.
 * - Serves the build on 127.0.0.1 and drives the cached Chrome Headless Shell through
 *   playwright-core. No native window, no system browser, no network beyond localhost.
 * - The bridge is a TEST DOUBLE injected before the app loads: it advertises only
 *   `asset.import`, records every dispatched command and imports nothing. Native import
 *   correctness comes from the engine CLI runs, not from this fixture.
 * Usage (repo root): node handoffs/0040-static-fbx-import/tools/ui_affordance.mjs
 */
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import react from '@vitejs/plugin-react';
import { build, preview } from 'vite';

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, '../../..');
const uiRoot = join(repo, 'editor/ui');
const out = join(repo, 'artifacts/0040-static-fbx-import/ui');
const shots = join(out, 'shots');
mkdirSync(shots, { recursive: true });
const deadline = setTimeout(() => {
  console.error('ui_affordance: 120 s bound exceeded');
  process.exit(2);
}, 120_000);

const DOUBLE = `(() => {
  const asset = (id, name, path, kind, extra) => ({ id: '01JA40UI0000000000000000' + id, name, path, kind, fingerprint: 'fixture-' + id, ...extra });
  const assets = [
    asset('01', 'crate', 'models/crate.glb', 'model'),
    asset('02', 'static', 'models/static.FBX', 'model'),
    asset('03', 'crate_albedo', 'textures/crate_albedo.png', 'texture', { textureUsage: 'color' }),
  ];
  const snapshot = {
    connection: { status: 'ready', project: { id: '01JA40UI000000000000000000', name: 'FBX affordance fixture' } },
    hierarchy: { status: 'ready', value: { roots: [], nodes: {} } },
    schemas: {}, entities: {}, diagnostics: [], history: { entries: [], applied: 0 }, console: [],
    provider: { status: 'not-connected', provider: 'openai' },
    agent: { status: 'unavailable', reason: 'Affordance test double.' },
    viewport: { status: 'not-attached', reason: 'Affordance test double.' },
    assets: { status: 'ready', value: assets },
    assetImport: { available: true },
  };
  window.__dispatched = [];
  window.__INCANT_BRIDGE__ = {
    protocolVersion: 1, capabilities: ['asset.import'], label: 'FBX affordance test double (not engine)', isFixture: false,
    getSnapshot: () => snapshot, subscribe: () => () => {},
    dispatch: (command) => { window.__dispatched.push(JSON.parse(JSON.stringify(command)));
      return Promise.resolve({ ok: false, error: { code: 'test-double', message: 'Test double: nothing was imported.' } }); },
    request: () => Promise.resolve({ ok: false, error: { code: 'test-double', message: 'Test double.' } }),
  };
})();`;

const report = { checks: [], tooling: {}, screenshots: [] };
const check = (name, pass, detail) => {
  report.checks.push({ name, pass: Boolean(pass), detail });
  console.log(`${pass ? 'PASS' : 'FAIL'} ${name}${detail ? ` -- ${JSON.stringify(detail)}` : ''}`);
};

await build({
  root: uiRoot, configFile: false, base: './', logLevel: 'warn', cacheDir: join(out, 'vite-cache'),
  plugins: [react()], build: { outDir: join(out, 'dist'), emptyOutDir: true, target: 'es2022', assetsInlineLimit: 0 },
});
const server = await preview({
  root: uiRoot, configFile: false, base: './', logLevel: 'warn', cacheDir: join(out, 'vite-cache'),
  build: { outDir: join(out, 'dist') }, preview: { port: 4196, strictPort: true, host: '127.0.0.1' },
});
const browser = await chromium.launch({ headless: true });
report.tooling = { browser: browser.version(), playwright: 'playwright-core 1.64.0', vite: '8.3.4' };
try {
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2, colorScheme: 'dark' });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  page.on('request', (r) => { if (!r.url().startsWith('http://127.0.0.1:4196')) errors.push(`remote request ${r.url()}`); });
  await page.addInitScript(DOUBLE);
  await page.goto('http://127.0.0.1:4196/');
  await page.getByRole('tab', { name: /^Assets/ }).click();
  await page.getByRole('button', { name: 'Import', exact: true }).click();
  const form = page.locator('form').filter({ has: page.getByRole('textbox', { name: 'Path 1' }) });
  const formats = (await page.locator('.import-form__formats').innerText()).trim();
  check('format label advertises FBX', /\bFBX\b/.test(formats), formats);

  const path1 = page.getByRole('textbox', { name: 'Path 1' });
  const row1 = page.locator('li.import-row').first();
  for (const path of ['models/static.fbx', 'models/static.FBX']) {
    await path1.fill(path);
    await path1.blur();
    const type = await row1.locator('.import-row__type').innerText().catch(() => null);
    const usage = await row1.getByRole('radio').count();
    const problem = await row1.locator('.import-row__problem').count();
    check(`${path} is typed as a model without interpretation choices`, type === 'Model' && usage === 0 && problem === 0, { type, usage, problem });
  }
  await row1.screenshot({ path: join(shots, 'ui-import-row-fbx-upper.png') });

  // Stale interpretation: choose "Normal map" for a PNG, then retype the same row as FBX.
  await path1.fill('textures/stale.png');
  await row1.getByRole('radio', { name: /Normal map/ }).check();
  check('png row offers interpretation and Normal map is selected', await row1.getByRole('radio', { name: /Normal map/ }).isChecked());
  await path1.fill('models/static.FBX');
  const usageAfter = await row1.getByRole('radio').count();
  check('retyped FBX row hides the stale interpretation', usageAfter === 0, { radios: usageAfter });
  const existing = await row1.locator('.import-row__existing').innerText().catch(() => null);
  check('FBX row recognises the registered FBX model', existing !== null && existing.includes('static'), existing);
  await form.screenshot({ path: join(shots, 'ui-import-form-fbx-after-stale-usage.png') });
  await page.getByRole('button', { name: /^Import 1 file$/ }).click();
  await page.waitForFunction(() => window.__dispatched.length >= 1, null, { timeout: 5000 });
  const sent = await page.evaluate(() => window.__dispatched[0]);
  check('import request omits the stale texture usage', JSON.stringify(sent) === JSON.stringify({ type: 'asset.import', sources: [{ source: 'models/static.FBX' }] }), sent);

  // Unsupported extension still names FBX among the accepted formats.
  await path1.fill('models/static.obj');
  await path1.blur();
  const problem = await row1.locator('.import-row__problem').innerText().catch(() => null);
  check('unsupported extension message lists FBX', problem !== null && /Unsupported file type/.test(problem) && /\bFBX\b/.test(problem), problem);

  // Reimport of the registered FBX model sends only its source.
  await page.getByRole('option', { name: /^static/ }).click();
  const reimport = page.getByRole('button', { name: 'Reimport' });
  await reimport.click();
  await page.waitForFunction(() => window.__dispatched.length >= 2, null, { timeout: 5000 });
  const resent = await page.evaluate(() => window.__dispatched[1]);
  check('FBX reimport request carries no texture usage', JSON.stringify(resent) === JSON.stringify({ type: 'asset.import', sources: [{ source: 'models/static.FBX' }] }), resent);
  check('no page errors or remote requests', errors.filter((e) => !/Test double/.test(e)).length === 0, errors);
  report.dispatched = await page.evaluate(() => window.__dispatched);
  report.screenshots = ['ui-import-row-fbx-upper.png', 'ui-import-form-fbx-after-stale-usage.png'];
  await context.close();
} finally {
  await browser.close();
  await new Promise((done) => server.httpServer.close(done));
  clearTimeout(deadline);
}
report.passed = report.checks.every((c) => c.pass);
writeFileSync(join(here, '../evidence/ui-affordance.json'), `${JSON.stringify(report, null, 1)}\n`);
console.log(report.passed ? 'ALL PASS' : 'FAILURES');
process.exit(report.passed ? 0 : 1);
