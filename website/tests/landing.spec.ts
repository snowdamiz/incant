import { expect, test, type Page } from '@playwright/test'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import { dirname, resolve } from 'node:path'

const here = dirname(fileURLToPath(import.meta.url))
const shots = resolve(here, '../../handoffs/0005-landing-page/screenshots')
const axeSource = readFileSync(createRequire(import.meta.url).resolve('axe-core/axe.min.js'), 'utf8')

const viewports = [
  { name: 'desktop-1440', width: 1440, height: 900 },
  { name: 'tablet-768', width: 768, height: 1024 },
  { name: 'mobile-390', width: 390, height: 844 },
  { name: 'narrow-320', width: 320, height: 640 },
] as const

async function settle(page: Page): Promise<void> {
  await page.goto('./')
  await expect(page.getByRole('heading', { level: 1 })).toBeVisible()
  await page.evaluate(() => document.fonts.ready)
  await page.waitForLoadState('networkidle')
}

test.describe('landing page', () => {
  for (const vp of viewports) {
    test(`composes at ${vp.name} without horizontal overflow`, async ({ page }) => {
      const external: string[] = []
      page.on('request', (req) => {
        const url = new URL(req.url())
        if (url.hostname !== '127.0.0.1') external.push(req.url())
      })
      await page.setViewportSize({ width: vp.width, height: vp.height })
      await settle(page)
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)
      expect(overflow).toBeLessThanOrEqual(0)
      expect(external).toEqual([])
      await page.screenshot({ path: `${shots}/${vp.name}-full.png`, fullPage: true })
      await page.screenshot({ path: `${shots}/${vp.name}-fold.png` })
    })
  }

  test('axe finds no serious or critical violations', async ({ page }) => {
    for (const vp of [viewports[0], viewports[2]]) {
      await page.setViewportSize({ width: vp.width, height: vp.height })
      await settle(page)
      await page.addScriptTag({ content: axeSource })
      const violations = await page.evaluate(async () => {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const axe = (window as any).axe
        const result = await axe.run(document, { resultTypes: ['violations'] })
        return result.violations.map((v: { id: string; impact: string; nodes: { target: string[] }[] }) => ({
          id: v.id,
          impact: v.impact,
          targets: v.nodes.map((n) => n.target.join(' ')),
        }))
      })
      console.log(vp.name, JSON.stringify(violations))
      const severe = violations.filter((v: { impact: string }) => v.impact === 'serious' || v.impact === 'critical')
      expect(severe).toEqual([])
    }
  })

  test('skip link, heading order and outbound links', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 })
    await settle(page)
    await page.keyboard.press('Tab')
    const skip = page.getByRole('link', { name: 'Skip to content' })
    await expect(skip).toBeFocused()
    await expect(skip).toBeInViewport()
    await page.keyboard.press('Enter')
    await expect(page.locator('main')).toBeFocused()

    expect(await page.locator('h1').count()).toBe(1)
    const levels = await page.$$eval('h1,h2,h3,h4', (els) => els.map((e) => Number(e.tagName[1])))
    for (let i = 1; i < levels.length; i++) expect(levels[i]! - levels[i - 1]!).toBeLessThanOrEqual(1)

    const hrefs = await page.$$eval('a[href]', (as) => as.map((a) => a.getAttribute('href') ?? ''))
    for (const href of hrefs) {
      if (href.startsWith('#')) {
        expect(await page.locator(href).count(), href).toBe(1)
      } else {
        expect(href).toMatch(/^https:\/\/github\.com\/snowdamiz\/incant(\/blob\/main\/PLAN\.md)?$/)
      }
    }
  })

  test('mobile menu opens, traps nothing and closes with Escape', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await settle(page)
    const toggle = page.getByRole('button', { name: 'Open menu' })
    await toggle.focus()
    await page.keyboard.press('Enter')
    await expect(page.locator('#mobile-menu')).toBeVisible()
    await expect(page.getByRole('button', { name: 'Close menu' })).toHaveAttribute('aria-expanded', 'true')
    await expect(page.locator('#mobile-menu a').first()).toBeFocused()
    await page.waitForTimeout(400) // let the header background transition finish
    await page.screenshot({ path: `${shots}/mobile-390-menu-open.png` })
    await page.keyboard.press('Escape')
    await expect(page.locator('#mobile-menu')).toBeHidden()
    await expect(page.getByRole('button', { name: 'Open menu' })).toBeFocused()

    await page.getByRole('button', { name: 'Open menu' }).click()
    await page.locator('#mobile-menu').getByRole('link', { name: 'FAQ' }).click()
    await expect(page.locator('#mobile-menu')).toBeHidden()
    await expect(page).toHaveURL(/#faq$/)
  })

  test('workflow tabs change the illustration via keyboard', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 })
    await settle(page)
    const describe = page.getByRole('tab', { name: /Describe/ })
    await describe.focus()
    await expect(describe).toHaveAttribute('aria-selected', 'true')
    await expect(page.locator('#panel-describe')).toBeVisible()

    await page.keyboard.press('ArrowDown')
    const inspect = page.getByRole('tab', { name: /Inspect/ })
    await expect(inspect).toBeFocused()
    await expect(inspect).toHaveAttribute('aria-selected', 'true')
    await expect(page.locator('#panel-inspect')).toBeVisible()
    await expect(page.locator('#panel-describe')).toBeHidden()

    await page.keyboard.press('End')
    await expect(page.getByRole('tab', { name: /Rewind/ })).toHaveAttribute('aria-selected', 'true')
    await page.keyboard.press('ArrowRight')
    await expect(describe).toHaveAttribute('aria-selected', 'true')

    for (const id of ['describe', 'inspect', 'playtest', 'rewind']) {
      await page.locator(`#tab-${id}`).click()
      await page.locator('#workflow').screenshot({ path: `${shots}/workflow-${id}-1440.png`, animations: 'disabled' })
    }
  })

  test('FAQ disclosures toggle', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 })
    await settle(page)
    const second = page.locator('#faq details').nth(1)
    await expect(second).not.toHaveAttribute('open', '')
    await second.locator('summary').focus()
    await page.keyboard.press('Enter')
    await expect(second).toHaveAttribute('open', '')
  })

  test('reduced motion disables animation', async ({ browser }) => {
    const context = await browser.newContext({ reducedMotion: 'reduce' })
    const page = await context.newPage()
    await page.goto('./')
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible()
    const iterations = await page.evaluate(() =>
      [...document.querySelectorAll('*')]
        .map((el) => getComputedStyle(el))
        .filter((s) => s.animationName !== 'none')
        .map((s) => s.animationIterationCount),
    )
    expect(iterations.every((n) => n === '1')).toBe(true)
    await context.close()
  })
})
