import { expect, test } from '@playwright/test'
import axe from 'axe-core'

test('production page loads local assets without errors or third-party requests', async ({ page, baseURL }) => {
  const failures: string[] = []
  const externalRequests: string[] = []
  page.on('pageerror', error => failures.push(error.message))
  page.on('requestfailed', request => failures.push(request.url()))
  page.on('response', response => {
    if (response.status() >= 400) failures.push(`${response.status()} ${response.url()}`)
  })
  page.on('request', request => {
    if (request.url().startsWith('http') && !request.url().startsWith(new URL(baseURL!).origin)) {
      externalRequests.push(request.url())
    }
  })
  await page.goto('./', { waitUntil: 'networkidle' })
  await expect(page.locator('h1')).toHaveCount(1)
  await expect(page.locator('main')).toBeVisible()
  await page.evaluate(async () => {
    await document.fonts.ready
    await Promise.all(Array.from(document.images, image => image.decode()))
  })
  expect(failures).toEqual([])
  expect(externalRequests).toEqual([])

  const invalidAssets = await page.evaluate((basePath) =>
    Array.from(document.querySelectorAll('script[src], link[rel="stylesheet"][href], img[src]'))
      .map(element => element.getAttribute('src') || element.getAttribute('href') || '')
      .filter(url => url.startsWith('/') && !url.startsWith(basePath)),
  new URL(baseURL!).pathname)
  expect(invalidAssets).toEqual([])
})

test('links have real destinations and every in-page anchor resolves', async ({ page }) => {
  await page.goto('./')
  const brokenLinks = await page.locator('a').evaluateAll(links => links.flatMap(link => {
    const href = link.getAttribute('href')
    if (!href || href === '#' || href.startsWith('javascript:')) return [href || '(empty)']
    if (href.startsWith('#') && !document.getElementById(decodeURIComponent(href.slice(1)))) return [href]
    return []
  }))
  expect(brokenLinks).toEqual([])
})

for (const width of [320, 390, 768, 1440]) {
  test(`content fits the ${width}px viewport`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 })
    await page.goto('./', { waitUntil: 'networkidle' })
    await expect(page.locator('h1')).toBeVisible()
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)
    expect(overflow).toBeLessThanOrEqual(1)
  })
}

for (const width of [390, 1440]) {
  test(`no WCAG A/AA accessibility violations at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 1000 })
    await page.goto('./', { waitUntil: 'networkidle' })
    await page.addScriptTag({ content: axe.source })
    const violations = await page.evaluate(async () => {
      const scanner = (window as unknown as { axe: typeof axe }).axe
      const result = await scanner.run(document, {
        runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa'] },
      })
      return result.violations.map(violation => ({
        id: violation.id,
        impact: violation.impact,
        nodes: violation.nodes.map(node => node.target),
      }))
    })
    expect(violations).toEqual([])
  })
}

test('keyboard users can skip navigation', async ({ page }) => {
  await page.goto('./')
  await page.keyboard.press('Tab')
  const skip = page.getByRole('link', { name: /skip/i })
  await expect(skip).toBeFocused()
  await skip.press('Enter')
  await expect(page.locator('main')).toBeFocused()
})
