#!/usr/bin/env node
/**
 * Rasterizes brand/icon-macos.svg into a macOS iconset and packs it into
 * brand/Incant.icns with the system `iconutil`. macOS only. Uses a locally
 * installed Chrome, like render-icon.mjs. Re-run after editing the SVG.
 */
import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';

if (process.platform !== 'darwin') throw new Error('iconutil is only available on macOS.');
const brandDir = resolve(dirname(fileURLToPath(import.meta.url)), '../brand');
const executablePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome']
  .filter(Boolean)
  .find((path) => existsSync(path));
if (!executablePath) throw new Error('No Chrome found. Set CHROME_PATH.');

// Apple's iconset members: 16, 32, 128, 256 and 512 pt, each at 1x and 2x.
const sizes = [16, 32, 128, 256, 512].flatMap((pt) => [
  { name: `icon_${pt}x${pt}.png`, px: pt },
  { name: `icon_${pt}x${pt}@2x.png`, px: pt * 2 },
]);

const svg = readFileSync(join(brandDir, 'icon-macos.svg'), 'utf8');
const work = mkdtempSync(join(tmpdir(), 'incant-iconset-'));
const iconset = join(work, 'Incant.iconset');
const browser = await chromium.launch({ executablePath, headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1024, height: 1024 }, deviceScaleFactor: 1 });
  for (const { name, px } of sizes) {
    await page.setViewportSize({ width: px, height: px });
    await page.setContent(
      `<html><body style="margin:0;background:transparent">${svg.replace('width="1024" height="1024"', `width="${px}" height="${px}"`)}</body></html>`,
    );
    await page.locator('svg').screenshot({ path: join(iconset, name), omitBackground: true });
  }
} finally {
  await browser.close();
}
execFileSync('iconutil', ['-c', 'icns', iconset, '-o', join(brandDir, 'Incant.icns')]);
rmSync(work, { recursive: true, force: true });
console.log('Wrote brand/Incant.icns');
