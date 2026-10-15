import { expect, test, type Page } from '@playwright/test'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import { dirname, resolve } from 'node:path'

const here = dirname(fileURLToPath(import.meta.url))
// Ordinary verification must not overwrite Claude's reviewed evidence.
const shots = process.env.INCANT_EVIDENCE_DIR ?? resolve(here, '../test-results/visual')
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

/** Scrolls each live scene (optionally within `scope`) into view and waits until it has drawn or shows its fallback. */
async function renderScenes(page: Page, scope = 'body'): Promise<void> {
  const canvases = page.locator(`${scope} canvas`)
  const count = await canvases.count()
  for (let i = 0; i < count; i++) {
    const canvas = canvases.nth(i)
    const scene = canvas.locator('..')
    await scene.scrollIntoViewIfNeeded()
    await expect
      .poll(() => canvas.evaluate((c: HTMLCanvasElement) => c.width !== 300 || c.style.display === 'none'), { timeout: 15_000 })
      .toBe(true)
    if (await canvas.evaluate((c: HTMLCanvasElement) => c.style.display === 'none')) {
      await expect(scene.locator('[data-gl-fallback]')).toBeVisible()
    }
  }
  await page.evaluate(() => window.scrollTo(0, 0))
}

test.describe('landing page', () => {
  // Emit failures when they happen, even if a later test reaches the job limit.
  test.afterEach(({}, info) => {
    for (const error of info.errors) console.error(`[failure] ${info.title}: ${(error.message ?? '').slice(0, 12_000)}`)
  })
  for (const vp of viewports) {
    test(`composes at ${vp.name} without horizontal overflow`, async ({ page }) => {
      test.slow() // Scroll every scene before the existing full-page capture.
      const external: string[] = []
      page.on('request', (req) => {
        const url = new URL(req.url())
        if (url.hostname !== '127.0.0.1') external.push(req.url())
      })
      await page.setViewportSize({ width: vp.width, height: vp.height })
      await settle(page)
      await renderScenes(page)
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)
      expect(overflow).toBeLessThanOrEqual(0)
      expect(external).toEqual([])
      await page.screenshot({ path: `${shots}/${vp.name}-fold.png` })
      await page.screenshot({ path: `${shots}/${vp.name}-full.png`, fullPage: true })
    })
  }

  test('live scenes draw into sized canvases or fall back to a still', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 })
    await settle(page)
    await renderScenes(page)
    const hero = page.locator('#top canvas')
    const drawn = await hero.evaluate((c: HTMLCanvasElement) => ({ w: c.width, h: c.height, shown: c.style.display !== 'none' }))
    if (drawn.shown) {
      expect(drawn.w).toBeGreaterThan(300)
      expect(drawn.h).toBeGreaterThan(150)
    }
    await expect(page.locator('#top [role="img"]')).toHaveAttribute('aria-label', /lighthouse/i)
  })

  test('refused slow WebGL contexts preserve fallback scenes and page content', async ({ page }) => {
    await page.addInitScript(() => {
      const original = HTMLCanvasElement.prototype.getContext
      HTMLCanvasElement.prototype.getContext = function (this: HTMLCanvasElement, id: string, options?: WebGLContextAttributes) {
        if (id === 'webgl2') {
          if (options?.failIfMajorPerformanceCaveat !== true) throw new Error('Missing performance admission')
          return null
        }
        return original.call(this, id, options)
      } as typeof original
    })
    const errors: string[] = []
    page.on('pageerror', (error) => errors.push(error.message))
    await settle(page)
    await renderScenes(page)
    expect(await page.locator('[data-gl-fallback]').count()).toBeGreaterThan(0)
    await expect(page.locator('#top').getByRole('link', { name: 'View on GitHub', exact: true })).toBeVisible()
    expect(errors).toEqual([])
  })

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
        // Credit links are legitimate destinations alongside the product plan.
        const destination = new URL(href)
        expect(destination.protocol).toBe('https:')
        expect(destination.username).toBe('')
        expect(destination.password).toBe('')
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

  test('walkthrough tabs drive the editor recreation from the keyboard', async ({ page }) => {
    test.slow() // captures five states of a live viewport
    await page.setViewportSize({ width: 1440, height: 900 })
    await settle(page)
    const ask = page.getByRole('tab', { name: /Ask/ })
    const panel = page.locator('#stage-panel')
    const picture = panel.locator(':scope > [role="img"]')
    await ask.focus()
    await expect(ask).toHaveAttribute('aria-selected', 'true')
    await expect(picture).toHaveAttribute('aria-label', /Recreation of the Incant editor/)

    await page.keyboard.press('ArrowRight')
    const review = page.getByRole('tab', { name: /Review/ })
    await expect(review).toBeFocused()
    await expect(review).toHaveAttribute('aria-selected', 'true')
    await expect(panel).toHaveAttribute('aria-labelledby', 'stage-tab-review')
    await expect(picture).toHaveAttribute('aria-label', /preview beam/)
    await expect(panel).toContainText('Apply 4 changes to the project?')

    await page.keyboard.press('End')
    await expect(page.getByRole('tab', { name: /Undo/ })).toHaveAttribute('aria-selected', 'true')
    await expect(panel).toContainText('revision 41')
    await page.keyboard.press('ArrowRight')
    await expect(ask).toHaveAttribute('aria-selected', 'true')

    await renderScenes(page, '#stage-panel')
    for (const id of ['ask', 'review', 'play', 'undo']) {
      await page.locator(`#stage-tab-${id}`).click()
      await page.waitForTimeout(900)
      await panel.screenshot({ path: `${shots}/stage-${id}-1440.png` })
    }
    await page.setViewportSize({ width: 390, height: 844 })
    await page.locator('#stage-tab-play').click()
    await page.waitForTimeout(900)
    await panel.screenshot({ path: `${shots}/stage-play-390.png` })
  })

  test('approving the agent’s change in the recreation advances to the play-test', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 })
    await settle(page)
    await page.locator('#stage-tab-review').click()
    await page.locator('#stage-panel').getByText('Allow', { exact: true }).click()
    await expect(page.getByRole('tab', { name: /Play-test/ })).toHaveAttribute('aria-selected', 'true')
    await expect(page.locator('#stage-panel')).toContainText('Play-test passed')
  })

  test('geometry graph nodes rebuild the model preview from the keyboard', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 })
    await settle(page)
    const stage = page.locator('#create')
    const preview = stage.getByRole('img', { name: /^Live preview of the Incant geometry graph/ })
    const nodes = stage.getByRole('button')
    await expect(nodes).toHaveCount(7)
    const before = await preview.getAttribute('aria-label')
    await expect(nodes.last()).toHaveAttribute('aria-pressed', 'true')

    await nodes.first().focus()
    await page.keyboard.press('Enter')
    await expect(nodes.first()).toHaveAttribute('aria-pressed', 'true')
    await expect(nodes.last()).toHaveAttribute('aria-pressed', 'false')
    await expect(stage.getByRole('button', { pressed: true })).toHaveCount(1)
    await expect(preview).not.toHaveAttribute('aria-label', before ?? '')
    await expect(preview).toHaveAttribute('aria-label', /displaying the Cylinder node/)

    await page.setViewportSize({ width: 390, height: 844 })
    await expect(nodes).toHaveCount(7)
    await nodes.nth(3).click()
    await expect(preview).toHaveAttribute('aria-label', /displaying the Array node/)
  })

  test('toolset and platforms list every capability and export target', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 })
    await settle(page)
    await expect(page.locator('#toolset li h3')).toHaveCount(12)
    const targets = page.locator('#ship li h3')
    await expect(targets).toHaveText(['Windows', 'macOS', 'Linux', 'iOS', 'Android', 'Web'])
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

  test('reduced motion disables animation and still draws each scene once', async ({ browser }) => {
    const context = await browser.newContext({ reducedMotion: 'reduce', viewport: { width: 1440, height: 900 } })
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
    await renderScenes(page)
    await context.close()
  })
})
