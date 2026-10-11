import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import ts from 'typescript';
import { transform } from '@swc/core';
import { decode, encode } from '@jridgewell/sourcemap-codec';
import { fail, LIMITS, portable, SDK_IMPORT, snapshotGraph, snapshotProjectGraph } from './graph.mjs';

const require = createRequire(import.meta.url);
const REPO = fileURLToPath(new URL('../../', import.meta.url));
const LIB = path.dirname(require.resolve('typescript'));
const SDK = path.join(REPO, 'sdk/ts/src');
const hash = data => createHash('sha256').update(data).digest('hex');

function typecheck(graph) {
  const native = path.join(SDK, 'native.d.ts');
  const generated = path.join(SDK, 'generated.d.ts');
  const virtual = path.join(REPO, '__incant_native_entry_check__.ts');
  const texts = new Map(graph.modules.map(record => [record.file, record.text]));
  if (texts.has(virtual)) fail('entry-check module path is reserved');
  for (const file of [native, generated]) texts.set(file, fs.readFileSync(file, 'utf8'));
  const entryImports = new Map(graph.entries.map((entry, i) => [`__incant_entry_${i}__`, entry.file]));
  texts.set(virtual, `import type { NativeBehavior } from '${SDK_IMPORT}';\n`
    + [...entryImports.keys()].map((name, i) => `import entry${i} from '${name}';\n`
      + `const checked${i}: NativeBehavior = entry${i};\nvoid checked${i};\n`).join(''));
  const records = new Map(graph.modules.map(record => [record.file, record]));
  const options = { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext,
    moduleResolution: ts.ModuleResolutionKind.Bundler, strict: true, noEmit: true,
    types: [], lib: ['lib.es2022.d.ts'], skipLibCheck: false, isolatedModules: true,
    verbatimModuleSyntax: true, allowImportingTsExtensions: true,
    moduleDetection: ts.ModuleDetectionKind.Force };
  const allowed = file => texts.has(file) || path.dirname(file) === LIB
    && /^lib\.[a-z0-9.]+\.d\.ts$/u.test(path.basename(file));
  const host = ts.createCompilerHost(options, true);
  host.fileExists = file => allowed(file) && (texts.has(file) || fs.existsSync(file));
  host.readFile = file => {
    if (!allowed(file)) return undefined;
    if (!texts.has(file)) texts.set(file, fs.readFileSync(file, 'utf8'));
    return texts.get(file);
  };
  host.getSourceFile = (file, languageVersion) => {
    const text = host.readFile(file);
    return text === undefined ? undefined : ts.createSourceFile(file, text, languageVersion, true);
  };
  host.resolveModuleNames = (names, from) => names.map(name => {
    const file = name === SDK_IMPORT ? native
      : from === virtual ? entryImports.get(name)
      : from === native && name === './generated' ? generated
      : records.get(from)?.dependencies.get(name);
    return file ? { resolvedFileName: file, extension: file.endsWith('.d.ts') ? ts.Extension.Dts : ts.Extension.Ts } : undefined;
  });
  const program = ts.createProgram([...records.keys(), virtual], options, host);
  const diagnostics = ts.getPreEmitDiagnostics(program);
  if (diagnostics.length) {
    const messages = diagnostics.slice(0, 20).map(diagnostic => {
      const location = diagnostic.file && diagnostic.start !== undefined
        ? diagnostic.file.getLineAndCharacterOfPosition(diagnostic.start) : null;
      const label = diagnostic.file?.fileName === virtual ? 'native entry contract'
        : diagnostic.file ? portable(path.relative(graph.root, diagnostic.file.fileName)) : 'typecheck';
      return `${label}${location ? `:${location.line + 1}:${location.character + 1}` : ''}: `
        + ts.flattenDiagnosticMessageText(diagnostic.messageText, '\n');
    });
    fail(`strict typecheck failed (${diagnostics.length} diagnostics)\n${messages.join('\n')}`);
  }
}

/** Pure authoring transform after a bounded file snapshot. No project JS runs. */
export async function cookNativeScript(root, entry) {
  const graph = await snapshotGraph(root, entry);
  return cookGraph(graph, false);
}

/** Source-keyed behaviors for the native authored attachment host. */
export async function cookNativeProjectScripts(root, catalog) {
  return cookGraph(await snapshotProjectGraph(root, catalog), true);
}

async function cookGraph(graph, project) {
  typecheck(graph);
  const indexes = new Map(graph.modules.map((record, i) => [record.file, i]));
  const sources = [], sourcesContent = [], names = [], mappings = [];
  let code = '';
  let line = 0;
  const append = text => { code += text; line += text.split('\n').length - 1; };
  append('"use strict";\nexports.default = (function () {\nconst factories = [\n');
  for (const record of graph.modules) {
    const transformed = await transform(record.text, {
      filename: record.label, sourceFileName: record.label,
      swcrc: false, configFile: false, sourceMaps: true, inlineSourcesContent: true,
      jsc: { parser: { syntax: 'typescript' }, target: 'es2022', externalHelpers: false },
      module: { type: 'commonjs', strict: true },
    });
    append('function (require, exports) {\n');
    const map = JSON.parse(transformed.map);
    const typeOnlyMap = map.sources.length === 0 && map.mappings === '';
    if (!typeOnlyMap && (map.sources.length !== 1 || map.sources[0] !== record.label))
      fail('unexpected compiler source map');
    const sourceIndex = sources.length, nameIndex = names.length;
    sources.push(record.label); sourcesContent.push(record.text); names.push(...map.names);
    const decoded = decode(map.mappings);
    for (let offset = 0; offset < decoded.length; offset++) {
      mappings[line + offset] = decoded[offset].map(segment => segment.length === 1 ? segment
        : segment.length === 4 ? [segment[0], sourceIndex, segment[2], segment[3]]
        : [segment[0], sourceIndex, segment[2], segment[3], nameIndex + segment[4]]);
    }
    append(transformed.code.endsWith('\n') ? transformed.code : `${transformed.code}\n`);
    append('},\n');
  }
  const dependencies = graph.modules.map(record => [...record.dependencies]
    .map(([name, file]) => [name, indexes.get(file)]));
  // Imports are static and local. Module factories run only inside the hardened
  // native VM; these functions do not have a Node loader, filesystem or network.
  append(`];\nconst dependencies = ${JSON.stringify(dependencies)};\n`
    + 'const cache = [];\nfunction load(id) {\n'
    + '  if (cache[id]) return cache[id];\n'
    + '  const values = Object.create(null); cache[id] = values;\n'
    + '  factories[id](name => {\n'
    + '    const edge = dependencies[id].find(item => item[0] === name);\n'
    + "    if (!edge) throw Error('Unresolved native module');\n"
    + '    return load(edge[1]);\n  }, values);\n  return values;\n}\n');
  // Source IDs are canonical validated data, never interpolated executable text.
  // A shared helper has one module identity across the complete source catalog.
  append(project ? `return { sources: [${graph.entries.map(entry =>
    `{id:${JSON.stringify(entry.id)},behavior:load(${indexes.get(entry.file)}).default}`).join(',')}] };\n})();\n`
    : `return load(${indexes.get(graph.entry)}).default;\n})();\n`);
  if (Buffer.byteLength(code) > LIMITS.bundleBytes) fail('compiled bundle exceeds the native byte limit');
  const map = JSON.stringify({ version: 3, file: 'behavior.cjs', sources, sourcesContent,
    names, mappings: encode(Array.from({ length: line }, (_, i) => mappings[i] ?? [])) });
  if (Buffer.byteLength(map) > LIMITS.mapBytes) fail('source map exceeds the cook byte limit');
  const manifest = { format: project ? 'incant-native-typescript-project' : 'incant-native-typescript', version: 1,
    ...(project ? { sources: graph.entries.map(({ id, path }) => ({ id, path })) }
      : { entry: portable(path.relative(graph.root, graph.entry)) }),
    compilers: { typescript: ts.version, swc: require('@swc/core/package.json').version },
    sourceBytes: graph.sourceBytes, bundleBytes: Buffer.byteLength(code), mapBytes: Buffer.byteLength(map),
    bundleSha256: hash(code), mapSha256: hash(map),
    modules: graph.modules.map(record => ({ path: record.label, bytes: record.bytes.length,
      sha256: hash(record.bytes) })),
    trust: 'fresh local source transform; no project code or configuration executed',
    limits: LIMITS };
  return { code, map, manifest };
}
