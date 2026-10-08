/** CI-only browser execution probe, with no pixel/design assertions. */
import { chromium } from 'playwright-core';
import { writeFile } from 'node:fs/promises';
const browser=await chromium.launch({headless:true});
try {
  const page=await browser.newPage();
  await page.goto('http://127.0.0.1:4188/');
  await page.waitForFunction(()=>window.__INCANT_SMOKE__ !== undefined);
  const report=await page.evaluate(()=>window.__INCANT_SMOKE__);
  await writeFile('artifacts/web/result.json',JSON.stringify(report,null,2)+'\n');
  if(report.ok!==true || report.ticks!==120 || Math.abs(report.position_x-6)>1e-9) throw new Error('Web smoke failed');
  console.log(report);
} finally { await browser.close(); }
