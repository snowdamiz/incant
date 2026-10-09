#!/usr/bin/env node
/**
 * Rasterizes public/icon.svg to public/icon.png (512×512, transparent background)
 * and updates the Tauri host's copy. Re-run after editing the SVG.
 */
import { copyFileSync, existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';

const publicDir = resolve(dirname(fileURLToPath(import.meta.url)), '../public');
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome']
  .filter(Boolean)
  .find((path) => existsSync(path));
if (!executablePath) throw new Error('No Chrome found. Set CHROME_PATH.');

const svg = readFileSync(join(publicDir, 'icon.svg'), 'utf8');
const browser = await chromium.launch({ executablePath, headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 512, height: 512 }, deviceScaleFactor: 1 });
  await page.setContent(`<html><body style="margin:0;background:transparent">${svg}</body></html>`);
  await page.locator('svg').screenshot({ path: join(publicDir, 'icon.png'), omitBackground: true });
} finally {
  await browser.close();
}
copyFileSync(join(publicDir, 'icon.png'), resolve(publicDir, '../../app/icons/icon.png'));
console.log('Wrote public/icon.png and app/icons/icon.png');
