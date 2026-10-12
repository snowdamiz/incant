#!/usr/bin/env node
/**
 * BROWSER probe for handoff 0047, headless Chromium from the pinned playwright-core; not
 * native WebKit. After the keyboard focuses a text field, a mouse press on a focusable
 * element focuses it three ways: by the press's default action, by script focus, and by
 * script focus with `focusVisible: false`. Prints whether each matches :focus-visible.
 *
 * Usage, from the worktree root: node handoffs/0047-splitter-input/tools/probe-focus-visible.mjs
 */
import { chromium } from 'playwright-core';

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();
const results = {};
for (const mode of ['default-focus', 'script-focus', 'script-focus-visible-false']) {
  await page.setContent('<input id="field" value="field"><div id="target" tabindex="0" style="width:40px;height:40px;margin:20px;background:#888"></div>');
  await page.evaluate((how) => {
    const target = document.getElementById('target');
    target.addEventListener('mousedown', (event) => {
      if (how === 'default-focus') return;
      event.preventDefault();
      target.focus(how === 'script-focus' ? { preventScroll: true } : { preventScroll: true, focusVisible: false });
    });
  }, mode);
  await page.keyboard.press('Tab');
  const box = await page.locator('#target').boundingBox();
  await page.mouse.click(box.x + 20, box.y + 20);
  results[mode] = await page.evaluate(() => ({
    focused: document.activeElement?.id ?? null,
    focusVisible: document.getElementById('target').matches(':focus-visible'),
  }));
}
console.log(JSON.stringify({ browser: `chromium ${browser.version()} (headless)`, afterKeyboardFocusedField: results }, null, 2));
await browser.close();
