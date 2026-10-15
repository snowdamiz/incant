import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import fsSync from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import vm from 'node:vm';
import { test } from 'node:test';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import ts from 'typescript';
import { decode } from '@jridgewell/sourcemap-codec';
import { cookNativeScript, cookNativeProjectScripts } from '../native_script/cook.mjs';
import { snapshotGraph, snapshotProjectGraph, sourceCatalog, LIMITS } from '../native_script/graph.mjs';
import { publishArtifact } from '../native_script/publish.mjs';
import { snapshotCompilerHost } from '../native_script/typecheck_host.mjs';
import { PHASES, phaseRecorder } from '../native_script/phases.mjs';

const REPO = fileURLToPath(new URL('../../', import.meta.url));
const ENTRY = `import type { NativeBehavior } from '@incant/runtime/native';
export default { update() { return undefined; } } satisfies NativeBehavior;\n`;
const A = '00000000000000000000000001', B = '00000000000000000000000002';

for (const engine of ['D:\\engine Ω space', '\\\\server\\share\\engine Ω space']) {
  test(`restricted compiler host typechecks normalized Windows paths: ${engine}`, () => {
    const paths = path.win32, project = paths.join(engine, 'project Ω');
    const lib = paths.join(engine, 'node_modules/typescript/lib');
    const actualLib = path.dirname(createRequire(import.meta.url).resolve('typescript'));
    const native = paths.join(engine, 'sdk/native.d.ts');
    const generated = paths.join(engine, 'sdk/generated.d.ts');
    const virtual = paths.join(engine, '__incant_native_entry_check__.ts');
    const entry = paths.join(project, 'main.ts'), shared = paths.join(project, 'shared.ts');
    const texts = new Map([
      [native, "import type { Tick } from './generated'; export interface NativeBehavior { update(dt: Tick): void; }"],
      [generated, 'export type Tick = number;'],
      [entry, "import type { NativeBehavior } from '@incant/runtime/native'; import { move } from './shared'; export default { update(dt) { move(dt); } } satisfies NativeBehavior;"],
      [shared, 'export function move(dt: number): void { const a = new Float64Array(2); a[0] = Math.abs(dt); }'],
      [virtual, "import type { NativeBehavior } from '@incant/runtime/native'; import entry from '__entry__'; const checked: NativeBehavior = entry; void checked;"],
    ]);
    const snapshot = { texts, native, generated, virtual, lib,
      entryImports: new Map([['__entry__', entry]]), records: [
        { file: entry, dependencies: new Map([['./shared', shared]]) },
        { file: shared, dependencies: new Map() },
      ] };
    const reads = [];
    const localLibrary = file => {
      assert.equal(paths.dirname(file), lib, 'only pinned library text may touch disk');
      return path.join(actualLib, paths.basename(file));
    };
    const io = { existsSync: file => fsSync.existsSync(localLibrary(file)),
      readFileSync: (file, encoding) => { reads.push(file); return fsSync.readFileSync(localLibrary(file), encoding); } };
    const options = { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext,
      moduleResolution: ts.ModuleResolutionKind.Bundler, strict: true, noEmit: true,
      types: [], lib: ['lib.es2022.d.ts'], skipLibCheck: false };
    const host = snapshotCompilerHost(options, snapshot, io, paths);
    const slash = file => file.replaceAll('\\', '/');
    assert.equal(host.fileExists(slash(entry)), true);
    assert.equal(host.readFile(slash(entry)), texts.get(entry));
    assert.equal(host.resolveModuleNames(['./shared'], slash(entry))[0].resolvedFileName, shared);
    assert.equal(host.resolveModuleNames(['./generated'], slash(native))[0].resolvedFileName, generated);
    const diagnostics = ts.getPreEmitDiagnostics(ts.createProgram([entry, shared, virtual], options, host));
    assert.deepEqual(diagnostics.map(d => ts.flattenDiagnosticMessageText(d.messageText, '\n')), []);
    assert.ok(reads.length > 0, 'the real pinned ES libraries must be checked');
    const before = reads.length;
    for (const file of [paths.join(project, 'not-snapshotted.ts'),
      paths.join(lib, 'untrusted.d.ts'), paths.join(lib, '../outside/lib.es2022.d.ts'),
      paths.join(lib, 'nested/lib.es2022.d.ts')]) {
      assert.equal(host.fileExists(slash(file)), false);
      assert.equal(host.readFile(slash(file)), undefined);
    }
    assert.equal(reads.length, before, 'refused paths never reach the filesystem');
    texts.set(shared, 'export function move(dt: number): void { const wrong: number = "bad"; void wrong; process.exit(0); }');
    const bad = ts.getPreEmitDiagnostics(ts.createProgram([entry, shared, virtual], options,
      snapshotCompilerHost(options, snapshot, io, paths)));
    assert.ok(bad.some(d => d.code === 2322), 'strict type mismatch remains refused');
    assert.ok(bad.some(d => d.code === 2591 || d.code === 2580 || d.code === 2304), 'host globals remain unavailable');
    texts.set(slash(entry), texts.get(entry));
    assert.throws(() => snapshotCompilerHost(options, snapshot, io, paths), /ambiguous compiler snapshot path/u);
  });
}
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

test('project entries share one graph, deterministic source order and module identity', async () => {
  const entry = `import { next } from './shared';
import type { NativeBehavior } from '@incant/runtime/native';
const initialized = next();
export default { update({ chunks }) {
  chunks[0].transforms[0] = initialized * 10 + next(); return undefined;
} } satisfies NativeBehavior;`;
  await fixture({ 'z.ts': entry, 'a.ts': entry, 'shared.ts':
    'let count = 0; export function next() { return ++count; }' }, async root => {
    // IDs, not paths or caller ordering, define entry initialization order.
    const catalog = [{ id: B, path: 'a.ts' }, { id: A, path: 'z.ts' }];
    const first = await cookNativeProjectScripts(root, catalog);
    assert.deepEqual(first, await cookNativeProjectScripts(root, catalog.toReversed()));
    assert.equal(first.manifest.format, 'incant-native-typescript-project');
    assert.deepEqual(first.manifest.sources, catalog.toReversed());
    assert.deepEqual(first.manifest.modules.map(m => m.path), ['a.ts', 'shared.ts', 'z.ts']);
    assert.equal(first.manifest.sourceBytes, Buffer.byteLength(entry) * 2
      + Buffer.byteLength('let count = 0; export function next() { return ++count; }'));
    const exports = Object.create(null), transforms = new Float64Array(10);
    vm.runInNewContext(first.code, { exports }, { timeout: 1000 });
    assert.deepEqual(Array.from(exports.default.sources, source => source.id), [A, B]);
    exports.default.sources[0].behavior.update({ chunks: [{ transforms }] });
    assert.equal(transforms[0], 13);
    exports.default.sources[1].behavior.update({ chunks: [{ transforms }] });
    assert.equal(transforms[0], 24);
    const map = JSON.parse(first.map);
    assert.deepEqual(map.sources, ['a.ts', 'shared.ts', 'z.ts']);
    assert.ok(!first.map.includes(root));
  });
});

test('each project entry satisfies the contract without executing initialization', async () => {
  await fixture({ 'a.ts': ENTRY, 'b.ts': `throw Error('NOT_RUN');\n${ENTRY}` }, async root => {
    const catalog = [{ id: A, path: 'a.ts' }, { id: B, path: 'b.ts' }];
    assert.ok((await cookNativeProjectScripts(root, catalog)).code.includes('NOT_RUN'));
    for (const text of ['export default false;', ENTRY.replace('return undefined;',
      'const invalid: number = "wrong"; void invalid; return undefined;')]) {
      await fs.writeFile(path.join(root, 'b.ts'), text);
      await assert.rejects(cookNativeProjectScripts(root, catalog), /strict typecheck failed/u);
    }
  });
});

test('source catalog rejects ambiguous identities and nonportable entry paths', () => {
  const entry = { id: A, path: 'a.ts' };
  for (const value of [null, [], {}, [null], [entry, entry], [entry, { id: B, path: 'a.ts' }],
    [{ ...entry, id: '80000000000000000000000001' }], [{ ...entry, id: A.toLowerCase().replace('1', 'i') }],
    [{ ...entry, compilerOptions: {} }], Array(65).fill(entry),
    ...['/a.ts', '../a.ts', './a.ts', 'x//a.ts', 'x/../a.ts', 'C:/a.ts', 'x\\a.ts',
      'a.d.ts', 'a.js', 'x\0a.ts', 'x\na.ts', `${'x'.repeat(1024)}.ts`].map(file => [{ ...entry, path: file }])]) {
    assert.throws(() => sourceCatalog(value), /Native TypeScript cook:/u);
  }
  assert.deepEqual(sourceCatalog([{ id: B, path: 'nested/é.ts' }, entry]),
    [entry, { id: B, path: 'nested/é.ts' }]);
});

test('project graphs share aggregate byte and module limits across separate entry trees', async () => {
  await fixture({}, async root => {
    const catalog = [];
    for (let i = 0; i < 5; i++) {
      const file = `${i}.ts`;
      await fs.writeFile(path.join(root, file), '//'+ 'x'.repeat(LIMITS.fileBytes - 2));
      catalog.push({ id: String(i).padStart(26, '0'), path: file });
    }
    await assert.rejects(snapshotProjectGraph(root, catalog), /aggregate source byte limit/u);
    for (let i = 0; i < 64; i++) {
      await fs.writeFile(path.join(root, `${i}.ts`), i === 63 ? 'import "./helper";' : '');
    }
    await fs.writeFile(path.join(root, 'helper.ts'), '');
    const entries = Array.from({ length: 64 }, (_, i) =>
      ({ id: String(i).padStart(26, '0'), path: `${i}.ts` }));
    await assert.rejects(snapshotProjectGraph(root, entries), /module count limit/u);
  });
});

test('project CLI accepts only bounded catalog data and publishes no failed artifact', async () => {
  await fixture({ 'a.ts': ENTRY, 'b.ts': ENTRY }, async root => {
    const run = (input, name) => spawnSync(process.execPath, ['--max-old-space-size=512',
      path.join(REPO, 'tools/build_native_script.mjs'), '--project', root, path.join(root, name)],
    { input, encoding: 'utf8', timeout: 60_000 });
    const valid = JSON.stringify([{ id: B, path: 'b.ts' }, { id: A, path: 'a.ts' }]);
    const result = run(valid, 'cooked');
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(JSON.parse(result.stdout).sources, JSON.parse(valid).toReversed());
    for (const [input, name] of [['{', 'bad-json'], [' '.repeat(96 * 1024 + 1), 'large'],
      [JSON.stringify([{ id: A, path: '../a.ts' }]), 'escape']]) {
      assert.notEqual(run(input, name).status, 0);
      await assert.rejects(fs.stat(path.join(root, name)), { code: 'ENOENT' });
    }
  });
});

test('compiler phase observer preserves artifacts and emits only a bounded fixed sequence', async () => {
  await fixture({ 'entry.ts': ENTRY }, async root => {
    const phaseFile = path.join(root, 'phases.jsonl');
    const run = (name, observed) => spawnSync(process.execPath, ['--max-old-space-size=512',
      path.join(REPO, 'tools/build_native_script.mjs'), '--project', root, path.join(root, name)],
    { input: JSON.stringify([{ id: A, path: 'entry.ts' }]), encoding: 'utf8', timeout: 60_000,
      env: { ...process.env, INCANT_NATIVE_COOK_PHASE_FILE: observed ? phaseFile : '' } });
    for (const [name, observed] of [['plain', false], ['observed', true]]) {
      const result = run(name, observed);
      assert.equal(result.status, 0, result.stderr);
    }
    for (const name of ['behavior.cjs', 'behavior.cjs.map', 'manifest.json']) {
      assert.deepEqual(await fs.readFile(path.join(root, 'plain', name)),
        await fs.readFile(path.join(root, 'observed', name)));
    }
    const text = await fs.readFile(phaseFile, 'utf8');
    const rows = text.trimEnd().split('\n').map(JSON.parse);
    assert.deepEqual(rows.map(row => row.phase), PHASES);
    assert.ok(Buffer.byteLength(text) < 4096);
    let previous = 0;
    for (const row of rows) {
      assert.deepEqual(Object.keys(row).sort(), ['elapsed_us', 'phase']);
      assert.ok(Number.isSafeInteger(row.elapsed_us) && row.elapsed_us >= previous);
      previous = row.elapsed_us;
    }
    const refused = phaseRecorder(path.join(root, 'refused.jsonl'));
    try {
      assert.throws(() => refused.mark('private-source'), /phase order/u);
      refused.mark('driver_entry');
      assert.throws(() => refused.mark('driver_entry'), /phase order/u);
    } finally { refused.close(); }
    assert.equal((await fs.readFile(path.join(root, 'refused.jsonl'), 'utf8')).includes('private'), false);
  });
});

test('actual strict animation guest cook stays identical to the QuickJS test input', async () => {
  const cooked = await cookNativeScript(path.join(REPO, 'crates/incant_script/tests/fixtures'), 'animation.ts');
  assert.equal(cooked.code, await fs.readFile(path.join(REPO, 'crates/incant_script/tests/fixtures/animation.cjs'), 'utf8'));
});

test('animation SDK refuses wrong overload types and readonly ID mutation', async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'incant-animation-types-'));
  try {
    const entry = body => `import type { NativeBehavior } from '@incant/runtime/native';
export default {update({animations,commands}){const id=animations.ids[0];${body};return undefined;}} satisfies NativeBehavior;`;
    for (const bad of [
      "commands.controlAnimation(id,'crossfade',0,0)",
      "commands.controlAnimation(id,'crossfade',0,0,'1')",
      "commands.controlAnimation(id,'select',0)",
      "commands.controlAnimation(id,'select','0',0)",
      "commands.controlAnimation(id,'seek',false)",
      "commands.controlAnimation(id,'playing',1)",
      "commands.controlAnimation(id,'looping',true,0)",
      "commands.controlAnimation(id,'speed','1')",
      "commands.controlAnimation(id,'unknown')",
      "animations.ids[0]='changed'",
    ]) {
      await fs.writeFile(path.join(root, 'entry.ts'), entry(bad));
      await assert.rejects(cookNativeScript(root, 'entry.ts'), /strict typecheck failed/u, bad);
    }
    await fs.writeFile(path.join(root, 'entry.ts'), entry(`
commands.controlAnimation(id,'select',null,0);
commands.controlAnimation(id,'select',255,0);
commands.controlAnimation(id,'seek',0.5);
commands.controlAnimation(id,'playing',false);
commands.controlAnimation(id,'looping',true);
commands.controlAnimation(id,'speed',-16)`));
    assert.ok((await cookNativeScript(root, 'entry.ts')).code.includes('controlAnimation'));
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
});

test('strict crossfade guest cook matches the actual multi-instance QuickJS input', async () => {
  const root = path.join(REPO, 'crates/incant_script/tests/fixtures');
  const cooked = await cookNativeScript(root, 'crossfade.ts');
  assert.equal(cooked.code, await fs.readFile(path.join(root, 'crossfade.cjs'), 'utf8'));
});

test('strict coroutine cook binds the native fixture and explicit continuation contract', async () => {
  const root = path.join(REPO, 'crates/incant_script/tests/fixtures');
  const cooked = await cookNativeScript(root, 'coroutine.ts');
  assert.equal(cooked.code, await fs.readFile(path.join(root, 'coroutine.cjs'), 'utf8'));
  const template = await fs.readFile(path.join(REPO, 'sdk/templates/native-coroutines/pulse.ts'), 'utf8');
  assert.equal(template.replace("'../../ts/src/native'", "'@incant/runtime/native'"),
    await fs.readFile(path.join(root, 'coroutine.ts'), 'utf8'));
});

test('coroutine types reject deferred results, invalid continuation types and state mutation', async () => {
  for (const resume of [
    "async resume(){return {kind:'done'}}", "*resume(){yield 1}",
    "resume(){return undefined}", "resume(){return {kind:'wait',delayTicks:'1',state:null}}",
    "resume(){return {kind:'wait',delayTicks:1,state:()=>0}}",
    "resume(c,t){t.id='changed';return {kind:'done'}}",
    "resume(c,t){t.state={};return {kind:'done'}}",
  ]) await fixture({'entry.ts': `import type {NativeBehavior} from '@incant/runtime/native';
export default {coroutines:{step:{${resume}}},update(){return undefined}} satisfies NativeBehavior;`},
    async root => assert.rejects(cookNativeScript(root, 'entry.ts'), /strict typecheck failed/u));
  for (const call of ["coroutines.start('a','step',()=>0)","coroutines.start('a','step',null,4)","coroutines.cancel(4)"])
    await fixture({'entry.ts': `import type {NativeBehavior} from '@incant/runtime/native';
export default {update({coroutines}){${call};return undefined}} satisfies NativeBehavior;`},
      async root => assert.rejects(cookNativeScript(root, 'entry.ts'), /strict typecheck failed/u));
});
