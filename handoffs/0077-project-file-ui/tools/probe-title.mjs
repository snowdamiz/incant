#!/usr/bin/env node
/**
 * Layout probe for the titlebar project button (handoff 0077): the same fixture page as
 * capture.mjs, reporting each identity box's width against its content's natural width.
 * Prints JSON; writes nothing.
 *
 * Usage (worktree root, after the UI build): node handoffs/0077-project-file-ui/tools/probe-title.mjs [scenario] [width]
 */
import { existsSync, readFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import { preview } from 'vite';
import { SCENARIOS } from './scenarios.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const uiRoot = join(repoRoot, 'editor/ui');
const [scenario = 'saved', width = '1440'] = process.argv.slice(2);
const executablePath = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
if (!existsSync(executablePath)) throw new Error('No Chrome found.');

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

const server = await preview({ root: uiRoot, cacheDir: join(uiRoot, 'node_modules/.vite'), preview: { port: await freePort(4377), strictPort: true, host: '127.0.0.1' }, logLevel: 'error' });
const origin = server.resolvedUrls.local[0].replace(/\/$/, '');
const browser = await chromium.launch({ executablePath, headless: true });
try {
  const context = await browser.newContext({ viewport: { width: Number(width), height: 900 }, colorScheme: 'dark' });
  const page = await context.newPage();
  await page.addInitScript({ content: `window.__FILE_FIXTURE_CONFIG__ = ${JSON.stringify(SCENARIOS[scenario])};` });
  await page.addInitScript({ content: readFileSync(join(here, 'fake-host.js'), 'utf8') });
  await page.addInitScript({ content: readFileSync(join(repoRoot, 'editor/bridge/native.generated.js'), 'utf8') });
  await page.addInitScript({ content: 'window.__INCANT_BRIDGE__.isFixture = true;' });
  await page.goto(`${origin}/`, { waitUntil: 'networkidle' });
  await page.locator('.titlebar__project-button').waitFor();
  await page.evaluate(() => document.fonts.ready);
  const result = await page.evaluate(() => {
    const box = (selector) => {
      const el = document.querySelector(selector);
      if (!el) return null;
      const style = getComputedStyle(el);
      return {
        selector,
        width: Math.round(el.getBoundingClientRect().width * 10) / 10,
        scrollWidth: el.scrollWidth,
        clientWidth: el.clientWidth,
        display: style.display,
        flex: style.flex,
        minWidth: style.minWidth,
        maxWidth: style.maxWidth,
        overflow: style.overflow,
        font: `${style.fontWeight} ${style.fontSize} ${style.fontFamily.split(',')[0]}`,
      };
    };
    return {
      fonts: [...document.fonts].filter((font) => font.status === 'loaded').map((font) => `${font.family} ${font.weight}`),
      boxes: ['.titlebar__start', '.titlebar__identity', '.titlebar__project', '.titlebar__project-button', '.titlebar__project-name', '.fixture-tag', '.titlebar__start > .titlebar__spacer'].map(box),
    };
  });
  console.log(JSON.stringify(result, null, 2));
  await context.close();
} finally {
  await browser.close();
  await new Promise((done) => server.httpServer.close(done));
}
