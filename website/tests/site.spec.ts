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

test('mobile navigation supports keyboard dismissal and working section links', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 })
  await page.goto('./')
  const toggle = page.getByRole('button', { name: /^(Open|Close) menu$/ })
  await toggle.click()
  await expect(toggle).toHaveAttribute('aria-expanded', 'true')
  const firstSection = page.locator('#mobile-menu a[href^="#"]').first()
  const destination = await firstSection.getAttribute('href')
  expect(destination).toBeTruthy()
  await expect(firstSection).toBeFocused()
  await page.keyboard.press('Escape')
  await expect(toggle).toHaveAttribute('aria-expanded', 'false')
  await expect(toggle).toBeFocused()

  await toggle.press('Enter')
  await firstSection.press('Enter')
  expect(new URL(page.url()).hash).toBe(destination)
  await expect(toggle).toHaveAttribute('aria-expanded', 'false')
  await expect(page.locator('#mobile-menu')).not.toBeVisible()
})

test('product walkthrough changes its content and supports arrow, Home, and End keys', async ({ page }) => {
  await page.goto('./')
  const tabs = page.getByRole('tab')
  const first = tabs.first()
  const second = tabs.nth(1)
  const last = tabs.last()
  const initialContent = await page.getByRole('tabpanel').innerText()
  await first.focus()
  await first.press('ArrowRight')
  await expect(second).toBeFocused()
  await expect(second).toHaveAttribute('aria-selected', 'true')
  await expect(page.getByRole('tabpanel')).toHaveCount(1)
  await expect(page.getByRole('tabpanel')).not.toHaveText(initialContent)
  await expect(page.getByRole('tabpanel')).toHaveAttribute('aria-labelledby', (await second.getAttribute('id'))!)
  const secondContent = await page.getByRole('tabpanel').innerText()
  await second.press('End')
  await expect(last).toBeFocused()
  await expect(last).toHaveAttribute('aria-selected', 'true')
  await expect(page.getByRole('tabpanel')).not.toHaveText(secondContent)
  await last.press('ArrowRight')
  await expect(first).toBeFocused()
  await first.press('End')
  await last.press('Home')
  await expect(first).toBeFocused()
  await expect(first).toHaveAttribute('aria-selected', 'true')
})

test('FAQ answers can be opened and closed with the keyboard', async ({ page }) => {
  await page.goto('./')
  const item = page.locator('details').filter({ has: page.locator('summary').filter({ hasText: /offline/i }) })
  const question = item.locator('summary')
  await expect(item).not.toHaveAttribute('open')
  await question.focus()
  await question.press('Enter')
  await expect(item).toHaveAttribute('open')
  await expect(item.locator('p')).toBeVisible()
  await question.press('Space')
  await expect(item).not.toHaveAttribute('open')
  await expect(item.locator('p')).not.toBeVisible()
})
