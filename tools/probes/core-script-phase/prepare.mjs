// Generate only a bounded test fixture. Does not author/edit a project or run Rust.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { createRequire } from 'node:module';
const root = path.resolve(process.argv[2]);
const output = path.resolve(process.argv[3]);
const require = createRequire(path.join(root, 'package.json'));
const ts = require('typescript');
const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const source = fs.readFileSync(path.join(root, 'samples/core/stress.ts'), 'utf8');
if (hash(source) !== '7285e9009633052ba86ee0996c87f58097eb47fdaf0a73df10f42597b0119f1a')
  throw Error('Expected exact03bc stress source');
const result = ts.transpileModule(source, { fileName: 'stress.ts', reportDiagnostics: true,
  compilerOptions: { strict: true, target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS } });
if ((result.diagnostics ?? []).some(value => value.category === ts.DiagnosticCategory.Error))
  throw Error('TypeScript transpilation failed');
function replaceOnce(text, old, replacement) {
  if (text.split(old).length !== 2) throw Error('Mutation anchor differs');
  return text.replace(old, replacement);
}
const variants = { 'baseline.js': result.outputText };
for (const [axis, old] of Object.entries({
  x: 'transforms[offset] = values[index + 1] + phase;',
  y: 'transforms[offset + 1] = values[index + 2];',
  z: 'transforms[offset + 2] = values[index + 3] - phase;',
})) variants[`omit-${axis}.js`] = replaceOnce(result.outputText, old, '/* deliberately omitted write */');
fs.mkdirSync(output); // Fresh path only; no replacement of a previous generation.
fs.writeFileSync(path.join(output, 'stress.ts'), source);
for (const [name, value] of Object.entries(variants)) fs.writeFileSync(path.join(output, name), value);
const manifest = { format: 'incant-core-script-phase-input-v1', source_sha256: hash(source),
  node: process.version, node_path: process.execPath, node_sha256: hash(fs.readFileSync(process.execPath)),
  typescript: ts.version, typescript_sha256: hash(fs.readFileSync(require.resolve('typescript'))),
  compiler_options: { strict: true, target: 'ES2022', module: 'CommonJS' },
  files: Object.fromEntries(Object.entries(variants).map(([name, value]) => [name, hash(value)])) };
fs.writeFileSync(path.join(output, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
console.log(JSON.stringify(manifest));
