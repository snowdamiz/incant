#!/usr/bin/env node
import { transformFile } from '@swc/core';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
const [input, output] = process.argv.slice(2);
if (!input || !output) throw new Error('Usage: node tools/build_script.mjs input.ts output.js');
const result = await transformFile(resolve(input), {
  jsc: { parser: { syntax: 'typescript' }, target: 'es2022' },
  module: { type: 'commonjs', strict: true },
  sourceMaps: true,
});
await mkdir(dirname(resolve(output)), { recursive: true });
await writeFile(output, result.code);
await writeFile(`${output}.map`, result.map);
