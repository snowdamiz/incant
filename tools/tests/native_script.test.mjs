import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import vm from 'node:vm';
import { test } from 'node:test';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { decode } from '@jridgewell/sourcemap-codec';
import { cookNativeScript } from '../native_script/cook.mjs';
import { snapshotGraph, LIMITS } from '../native_script/graph.mjs';
import { publishArtifact } from '../native_script/publish.mjs';

const REPO = fileURLToPath(new URL('../../', import.meta.url));
const ENTRY = `import type { NativeBehavior } from '@incant/runtime/native';
export default { update() { return undefined; } } satisfies NativeBehavior;\n`;
async function fixture(files, action) {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'incant-native-ts-test-'));
  try {
    for (const [name, text] of Object.entries(files)) {
      const file = path.join(root, name);
      await fs.mkdir(path.dirname(file), { recursive: true });
      await fs.writeFile(file, text);
    }
    await action(root);
  } finally { await fs.rm(root, { recursive: true, force: true }); }
}

test('strict local graph has deterministic bundle/maps and executes each imported module once', async () => {
  await fixture({
    'entry.ts': `import type { NativeBehavior } from '@incant/runtime/native';
import { move, count } from './logic';
import { count as sameCount } from './logic.ts';
export default { update({ chunks }) {
  if (count() !== sameCount()) throw Error('duplicate module');
  for (const chunk of chunks) move(chunk.transforms);
  return undefined;
} } satisfies NativeBehavior;`,
    'logic.ts': 'let calls = 0; export function count() { return calls; }\nexport function move(values: Float64Array) { values[0] += ++calls; }\n',
  }, async root => {
    const first = await cookNativeScript(root, 'entry.ts');
    assert.deepEqual(first, await cookNativeScript(root, 'entry.ts'));
    assert.equal(first.manifest.modules.length, 2);
    assert.ok(!first.map.includes(root));
    // Trusted fixture only: this checks bundle semantics, not the native sandbox.
    const exports = Object.create(null);
    vm.runInNewContext(first.code, { exports }, { timeout: 1000 });
    const transforms = new Float64Array(10);
    exports.default.update({ chunks: [{ transforms }] });
    exports.default.update({ chunks: [{ transforms }] });
    assert.equal(transforms[0], 3);
    const map = JSON.parse(first.map);
    const lines = decode(map.mappings);
    const generated = first.code.split('\n').findIndex(line => line.includes('values[0] +='));
    const source = map.sources.indexOf('logic.ts');
    assert.ok(lines[generated].some(segment => segment[1] === source && segment[2] === 1));
  });
});

test('type-only modules and reexports work without running type-only side effects', async () => {
  await fixture({ 'entry.ts': "export { default } from './behavior';",
    'behavior.ts': ENTRY.replace('export default', "import type { Count } from './types';\nconst value: Count = 1; void value;\nexport default"),
    'types.ts': "throw Error('type-only dependency executed'); export type Count = number;" }, async root => {
    const result = await cookNativeScript(root, 'entry.ts');
    const exports = {};
    vm.runInNewContext(result.code, { exports }, { timeout: 1000 });
    assert.equal(exports.default.update(), undefined);
  });
});

test('pure type-only modules preserve valid flat maps without generated source segments', async () => {
  await fixture({ 'entry.ts': ENTRY.replace('export default', "import type { Count } from './types';\nconst count: Count = 1; void count;\nexport default"),
    'types.ts': 'export type Count = number;' }, async root => {
    const result = await cookNativeScript(root, 'entry.ts');
    assert.ok(JSON.parse(result.map).sources.includes('types.ts'));
    const exports = {};
    vm.runInNewContext(result.code, { exports }, { timeout: 1000 });
    assert.equal(exports.default.update(), undefined);
  });
});

test('cyclic local imports use CommonJS live exports without duplicate factories', async () => {
  await fixture({ 'entry.ts': ENTRY.replace('export default', "import { sum } from './a';\nexport default").replace('return undefined;', "if (sum() !== 3) throw Error('cycle'); return undefined;"),
    'a.ts': "import { b } from './b'; export const a = 1; export function sum() { return a + b; }",
    'b.ts': "import { a } from './a'; export const b = 2; export function unused() { return a; }" }, async root => {
    const exports = {};
    vm.runInNewContext((await cookNativeScript(root, 'entry.ts')).code, { exports }, { timeout: 1000 });
    exports.default.update();
  });
});

test('compile never executes top-level code or project configuration/plugins', async () => {
  await fixture({ 'entry.ts': `throw Error('MUST_NOT_RUN');\n${ENTRY}`,
    '.swcrc': '{"jsc":{"experimental":{"plugins":[["./plugin.js",{}]]}}}',
    'tsconfig.json': '{"compilerOptions":{"strict":false,"types":["bad"],"plugins":[{"name":"./plugin.js"}]}}',
    'package.json': '{"scripts":{"prepare":"touch MUST_NOT_RUN"}}',
    'plugin.js': "throw Error('PLUGIN_MUST_NOT_RUN')" }, async root => {
    const result = await cookNativeScript(root, 'entry.ts');
    assert.ok(result.code.includes('MUST_NOT_RUN'));
    assert.deepEqual(result.manifest.modules.map(module => module.path), ['entry.ts']);
  });
});

test('strict types, no host globals, and native entry contract are enforced', async () => {
  for (const source of [ENTRY.replace('return undefined;', 'const value: number = "bad"; void value; return undefined;'),
    ENTRY.replace('return undefined;', 'process.exit(0); return undefined;'),
    ENTRY.replace('return undefined;', 'fetch("https://example.invalid"); return undefined;'),
    'export default { initialState: {}, tick() {} };',
    'export const value = 1;']) {
    await fixture({ 'entry.ts': source }, async root => {
      await assert.rejects(cookNativeScript(root, 'entry.ts'), /strict typecheck failed/u);
    });
  }
});

test('external, dynamic, commonjs and unsupported syntax fail explicitly', async () => {
  for (const statement of ["import fs from 'node:fs';", "import value from 'a-package';",
    "import { NativeBehavior } from '@incant/runtime/native';", "import('./logic');",
    "const require = () => 1;", "const value = import.meta;", "import value = require('./logic');",
    "/// <reference path='../outside.ts' />", "/// <reference lib='dom' />",
    "declare module 'evil' {}", "@decorator class Class {}", "import './logic.js';",
    "import './logic?raw';", "import './logic' with { type: 'json' };"]) {
    await fixture({ 'entry.ts': `${statement}\n${ENTRY}`, 'logic.ts': 'export const value = 1;' }, async root => {
      await assert.rejects(cookNativeScript(root, 'entry.ts'), /Native TypeScript cook:/u);
    });
  }
});

test('containment, ambiguous paths, declarations and symlinks are refused', async () => {
  await fixture({ 'entry.ts': ENTRY, 'outside.ts': 'export const value = 1;',
    'nested/escape.ts': "import '../outside';", 'logic.ts': '', 'logic/index.ts': '' }, async root => {
    await assert.rejects(snapshotGraph(path.join(root, 'nested'), 'escape.ts'), /escapes/u);
    await fs.writeFile(path.join(root, 'entry.ts'), "import './logic';");
    await assert.rejects(snapshotGraph(root, 'entry.ts'), /ambiguous/u);
    await fs.writeFile(path.join(root, 'entry.ts'), "import './missing';");
    await assert.rejects(snapshotGraph(root, 'entry.ts'), /missing/u);
    await fs.writeFile(path.join(root, 'declaration.d.ts'), 'export const value: number;');
    await assert.rejects(snapshotGraph(root, 'declaration.d.ts'), /not declarations/u);
    await fs.symlink(path.join(root, 'outside.ts'), path.join(root, 'link.ts'));
    await assert.rejects(snapshotGraph(root, 'link.ts'), /symbolic links/u);
    await fs.symlink(path.join(root, 'logic'), path.join(root, 'linked'), 'junction');
    await assert.rejects(snapshotGraph(root, 'linked/index.ts'), /symbolic links/u);
  });
});

test('UTF-8, per-file, aggregate and graph-count limits are enforced before transform', async () => {
  await fixture({ 'entry.ts': Buffer.from([255]) }, async root => {
    await assert.rejects(snapshotGraph(root, 'entry.ts'), /encoded data/u);
    await fs.writeFile(path.join(root, 'entry.ts'), ' '.repeat(LIMITS.fileBytes + 1));
    await assert.rejects(snapshotGraph(root, 'entry.ts'), /file byte limit/u);
    for (let i = 0; i < 5; i++) await fs.writeFile(path.join(root, `${i}.ts`),
      (i < 4 ? `import './${i + 1}';\n` : '') + '//'+ 'x'.repeat(LIMITS.fileBytes - 50));
    await assert.rejects(snapshotGraph(root, '0.ts'), /aggregate source/u);
    for (let i = 0; i <= LIMITS.modules; i++) await fs.writeFile(path.join(root, `${i}.ts`),
      i < LIMITS.modules ? `import './${i + 1}';` : '');
    await assert.rejects(snapshotGraph(root, '0.ts'), /module count/u);
  });
});

test('CLI publishes only after validation and refuses existing output', async () => {
  await fixture({ 'entry.ts': ENTRY }, async root => {
    const run = output => spawnSync(process.execPath, ['--max-old-space-size=512',
      path.join(REPO, 'tools/build_native_script.mjs'), root, 'entry.ts', output],
    { encoding: 'utf8', timeout: 60_000 });
    const output = path.join(root, 'cooked');
    assert.equal(run(output).status, 0);
    const before = await fs.readFile(path.join(output, 'behavior.cjs'));
    assert.notEqual(run(output).status, 0);
    assert.deepEqual(await fs.readFile(path.join(output, 'behavior.cjs')), before);
    await fs.writeFile(path.join(root, 'entry.ts'), 'export default false;');
    assert.notEqual(run(path.join(root, 'invalid')).status, 0);
    await assert.rejects(fs.stat(path.join(root, 'invalid')), { code: 'ENOENT' });
  });
});

test('mid-write failures roll back only newly created artifacts and allow retry', async () => {
  for (const failAt of [2, 3]) await fixture({ 'unrelated.txt': 'preserve' }, async root => {
    const artifact = { code: 'code', map: 'map', manifest: { format: 'test' } };
    const destination = path.join(root, 'cooked');
    let opens = 0;
    const io = { ...fs, open: async (...args) => {
      const handle = await fs.open(...args), number = ++opens;
      return { stat: () => handle.stat(), close: () => handle.close(), writeFile: async data => {
        await handle.writeFile(data.slice(0, 1));
        if (number === failAt) throw Error('injected partial write');
        await handle.writeFile(data.slice(1));
      } };
    } };
    await assert.rejects(publishArtifact(destination, artifact, io), /injected partial write/u);
    await assert.rejects(fs.stat(destination), { code: 'ENOENT' });
    assert.equal(await fs.readFile(path.join(root, 'unrelated.txt'), 'utf8'), 'preserve');
    await publishArtifact(destination, artifact);
    assert.equal(await fs.readFile(path.join(destination, 'behavior.cjs'), 'utf8'), 'code');
    await assert.rejects(publishArtifact(destination, artifact, io), { code: 'EEXIST' });
    assert.equal(await fs.readFile(path.join(destination, 'behavior.cjs'), 'utf8'), 'code');
  });
});

test('rollback preserves unrelated files added inside a newly created output', async () => {
  await fixture({}, async root => {
    const destination = path.join(root, 'cooked');
    const io = { ...fs, open: async () => {
      await fs.writeFile(path.join(destination, 'keep.txt'), 'preserve');
      throw Error('injected open failure');
    } };
    await assert.rejects(publishArtifact(destination, { code: 'code' }, io), /incomplete cleanup/u);
    assert.equal(await fs.readFile(path.join(destination, 'keep.txt'), 'utf8'), 'preserve');
  });
});
