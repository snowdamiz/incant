// Prepare identical actual callback bytes; no authored project mutation or build.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import ts from 'typescript';
const root = path.resolve(import.meta.dirname, '../../..');
const out = path.resolve(process.argv[2]);
if (fs.existsSync(out)) throw Error('choose a fresh callback destination');
const original = fs.readFileSync(path.join(root, 'samples/core/stress.ts'), 'utf8');
const H = x => crypto.createHash('sha256').update(x).digest('hex');
if (H(original) !== '7285e9009633052ba86ee0996c87f58097eb47fdaf0a73df10f42597b0119f1a') throw Error('wrong actual stress source');
const fixture = fs.readFileSync(path.join(import.meta.dirname, 'fixture.js'), 'utf8');
function replace(from, to) {
  if (original.split(from).length !== 2) throw Error('mutation anchor changed');
  return original.replace(from, to);
}
const sources = { baseline: original,
  omitted: replace('transforms[offset] = values[index + 1] + phase;', '/* intentionally omitted x store */'),
  nanY: replace('sentinel.transforms[sentinelOffset + 1] = checksumX / COUNT;', 'sentinel.transforms[sentinelOffset + 1] = NaN;'),
  nanZ: replace('sentinel.transforms[sentinelOffset + 2] = checksumZ / COUNT;', 'sentinel.transforms[sentinelOffset + 2] = NaN;') };
fs.mkdirSync(out);
for (const [name, source] of Object.entries(sources)) {
  const result = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } });
  fs.writeFileSync(path.join(out, `${name}.js`), `const exports = {};\n${result.outputText}\n${fixture}`);
}
fs.writeFileSync(path.join(out, 'stress.ts'), original);
fs.copyFileSync(path.join(import.meta.dirname, 'semantics.js'), path.join(out, 'semantics.js'));
const files = Object.fromEntries(fs.readdirSync(out).sort().map(n => [n, H(fs.readFileSync(path.join(out, n)))]));
fs.writeFileSync(path.join(out, 'manifest.json'), JSON.stringify({ diagnostic_only: true, source_sha256: H(original), fixture_sha256: H(fixture), typescript: ts.version, files }, null, 2) + '\n');
console.log(JSON.stringify({ out, files }));
