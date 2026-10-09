// Browser review captures of the landing page sections (Chrome via Playwright).
// Usage: node capture-site.mjs <baseURL> <outDir>
import { createRequire } from 'node:module'
import { mkdirSync } from 'node:fs'
import path from 'node:path'
const require = createRequire(path.resolve('website/package.json'))
const { chromium } = require('@playwright/test')

const [base, out] = process.argv.slice(2)
mkdirSync(out, { recursive: true })
const widths = [320, 390, 768, 1024, 1280, 1440]
const sections = ['workflow', 'create']
const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH })
for (const w of widths) {
  const page = await browser.newPage({ viewport: { width: w, height: 900 }, deviceScaleFactor: 1, reducedMotion: 'reduce' })
  await page.goto(base)
  await page.evaluate(() => document.fonts.ready)
  for (const id of sections) {
    const el = page.locator(`#${id}`)
    const box = await el.evaluate((n) => { const r = n.getBoundingClientRect(); return { x: r.left + scrollX, y: r.top + scrollY, width: r.width, height: r.height } })
    // Full-page clip keeps the sticky header out of the section image.
    await page.screenshot({ path: path.join(out, `${id}-${w}.png`), fullPage: true, clip: box })
  }
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)
  console.log(w, 'overflow', overflow)
  await page.close()
}
await browser.close()
