/**
 * Decodes a PNG to RGBA with the local Chrome (no image libraries are installed).
 * Returns { width, height, data: Uint8ClampedArray } for an optional crop rectangle.
 */
import { existsSync, readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const require = createRequire(join(resolve(here, '../../../editor/ui'), 'package.json'));
const { chromium } = require('playwright-core');

export const chromePath = [process.env.CHROME_PATH, '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome']
  .filter(Boolean)
  .find((p) => existsSync(p));

export async function decodePng(path, crop) {
  const b64 = readFileSync(path).toString('base64');
  const browser = await chromium.launch({ executablePath: chromePath, headless: true });
  try {
    const page = await browser.newPage();
    const result = await page.evaluate(
      async ({ b64, crop }) => {
        const img = new Image();
        img.src = `data:image/png;base64,${b64}`;
        await img.decode();
        const c = crop ?? { x: 0, y: 0, width: img.naturalWidth, height: img.naturalHeight };
        const canvas = new OffscreenCanvas(c.width, c.height);
        const ctx = canvas.getContext('2d', { colorSpace: 'srgb' });
        ctx.drawImage(img, -c.x, -c.y);
        const d = ctx.getImageData(0, 0, c.width, c.height).data;
        let s = '';
        for (let i = 0; i < d.length; i += 32768) s += String.fromCharCode(...d.subarray(i, i + 32768));
        return { width: c.width, height: c.height, b64: btoa(s) };
      },
      { b64, crop },
    );
    return { width: result.width, height: result.height, data: new Uint8ClampedArray(Buffer.from(result.b64, 'base64')) };
  } finally {
    await browser.close();
  }
}
