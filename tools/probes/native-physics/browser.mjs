/** Executes the serial WASM solver in fresh Chromium processes; no visual checks. */
import { chromium } from 'playwright-core';
import { createServer } from 'node:http';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const [directory, trialsText] = process.argv.slice(2);
const trials = Number(trialsText);
if (!directory || !Number.isInteger(trials) || trials < 2) throw new Error('directory and >=2 trials required');
const root = resolve(directory);
const uploads = new Set();
const server = createServer(async (request, response) => {
  try {
    const path = new URL(request.url, 'http://localhost').pathname;
    if (request.method === 'POST' && /^\/browser-(3d|2d)-\d+\.bin$/.test(path)) {
      const chunks = [];
      let size = 0;
      for await (const chunk of request) {
        size += chunk.length;
        if (size > 32 * 1024 * 1024) throw new Error('unexpected stream size');
        chunks.push(chunk);
      }
      await writeFile(resolve(root, path.slice(1)), Buffer.concat(chunks));
      uploads.add(path);
      response.end('ok');
    } else if (path === '/') {
      response.setHeader('Content-Type', 'text/html');
      response.end('<!doctype html><title>Native ECS serial WASM physics parity</title>');
    } else if (['/pkg/incant_native_physics_parity.js', '/pkg/incant_native_physics_parity_bg.wasm'].includes(path)) {
      response.setHeader('Content-Type', path.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
      response.end(await readFile(resolve(root, path.slice(1))));
    } else { response.writeHead(404).end(); }
  } catch (error) { response.writeHead(500).end(String(error)); }
});
await new Promise((done) => server.listen(0, '127.0.0.1', done));
const records = [];
try {
  const origin = `http://127.0.0.1:${server.address().port}`;
  for (let trial = 0; trial < trials; trial++) {
    const browser = await chromium.launch({ headless: true });
    try {
      const page = await browser.newPage();
      await page.goto(origin);
      const streams = await page.evaluate(async ({ trial }) => {
        const module = await import('/pkg/incant_native_physics_parity.js');
        await module.default();
        const results = [];
        for (const scenario of ['3d', '2d']) {
          // state_stream constructs and actually steps the shared Rust world 640 times.
          const bytes = scenario === '2d' ? module.state_stream_2d() : module.state_stream();
          const file = `/browser-${scenario}-${trial}.bin`;
          const response = await fetch(file, { method: 'POST', body: bytes });
          if (!response.ok) throw new Error(`stream upload: ${response.status}`);
          results.push({ scenario, file: file.slice(1), bytes: bytes.length });
        }
        return results;
      }, { trial });
      records.push({ trial, browser: browser.version(), streams });
    } finally { await browser.close(); }
  }
  if (uploads.size !== trials * 2) throw new Error('missing browser execution stream');
  await writeFile(resolve(root, 'browser.json'), JSON.stringify(records, null, 2) + '\n');
  console.log(JSON.stringify({ browserProcesses: trials, streams: uploads.size }));
} finally { await new Promise((done) => server.close(done)); }
