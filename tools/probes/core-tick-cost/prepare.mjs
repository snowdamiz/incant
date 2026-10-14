// Diagnostic source variants only; never writes an authored project or sample.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import ts from 'typescript';

const root = path.resolve(import.meta.dirname, '../../..');
const destination = path.resolve(process.argv[2]);
if (fs.existsSync(destination)) throw Error('diagnostic destination must be fresh');
const original = fs.readFileSync(path.join(root, 'samples/core/stress.ts'), 'utf8');
const fixture = fs.readFileSync(path.join(import.meta.dirname, 'fixture.js'), 'utf8');
function replace(text, from, to) {
  if (text.split(from).length !== 2) throw Error(`expected one source match: ${from}`);
  return text.replace(from, to);
}
const count = replace(original, '        count++;\n      }', '      }\n      count += values.length / 4;');
const length = replace(count, 'for (let index = 0; index < values.length; index += 4)',
  'for (let index = 0, length = values.length; index < length; index += 4)');
let cursor = replace(count, 'for (let index = 0; index < values.length; index += 4)',
  'for (let index = 0, length = values.length; index < length;)');
for (const value of ['values[index]', 'values[index + 1]', 'values[index + 2]', 'values[index + 3]'])
  cursor = replace(cursor, value, 'values[index++]');
let soa = replace(original, 'values: readonly number[]',
  'values: { row: number[]; x: number[]; y: number[]; z: number[] }');
soa = replace(soa, 'const values: number[] = [];', 'const values = { row: [] as number[], x: [] as number[], y: [] as number[], z: [] as number[] };');
soa = replace(soa, 'values.push(row, ordinal % 97 - 48, Math.floor(ordinal / 97) % 97 - 48, ordinal % 13 - 6);',
  'values.row.push(row); values.x.push(ordinal % 97 - 48); values.y.push(Math.floor(ordinal / 97) % 97 - 48); values.z.push(ordinal % 13 - 6);');
soa = replace(soa, 'for (let index = 0; index < values.length; index += 4)',
  'for (let index = 0, length = values.row.length; index < length; index++)');
for (const [old, next] of [['values[index]', 'values.row[index]'], ['values[index + 1]', 'values.x[index]'],
  ['values[index + 2]', 'values.y[index]'], ['values[index + 3]', 'values.z[index]']]) soa = replace(soa, old, next);
soa = replace(soa, '        count++;\n      }', '      }\n      count += values.row.length;');
let locals = replace(soa, 'const transforms = chunk.transforms, stride = chunk.transformStride;',
  'const transforms = chunk.transforms, stride = chunk.transformStride;\n      const { row, x, y, z } = values;');
const split = locals.indexOf('  update(context: NativeContext) {');
if (split < 0) throw Error('missing update boundary');
let update = locals.slice(split);
for (const member of ['row', 'x', 'y', 'z']) update = update.replaceAll(`values.${member}`, member);
locals = locals.slice(0, split) + update;

const sources = { baseline: original, count, length, cursor, soa, locals };
const omitted = replace(original, 'transforms[offset] = values[index + 1] + phase;', '/* deliberately omitted x write */');
const nanY = replace(original, 'sentinel.transforms[sentinelOffset + 1] = checksumX / COUNT;',
  'sentinel.transforms[sentinelOffset + 1] = NaN;');
const nanZ = replace(original, 'sentinel.transforms[sentinelOffset + 2] = checksumZ / COUNT;',
  'sentinel.transforms[sentinelOffset + 2] = NaN;');
fs.mkdirSync(path.join(destination, 'src'), { recursive: true });
const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const rows = [];
for (const [name, source] of Object.entries({ ...sources, omitted, nanY, nanZ })) {
  const compiled = ts.transpileModule(source, { compilerOptions: {
    module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022,
  }}).outputText;
  const js = `const exports = {};\n${compiled}\n${fixture}`;
  fs.writeFileSync(path.join(destination, `${name}.ts`), source);
  fs.writeFileSync(path.join(destination, `${name}.js`), js);
  rows.push({ name, source_sha256: hash(source), javascript_sha256: hash(js) });
}
fs.copyFileSync(path.join(import.meta.dirname, 'main.rs'), path.join(destination, 'src/main.rs'));
fs.writeFileSync(path.join(destination, 'Cargo.toml'), `[package]
name = "incant-core-tick-diagnostic"
version = "0.1.0"
edition = "2024"
[workspace]
[dependencies]
rquickjs = "=0.14.0"
bevy_ecs = { version = "=0.19.1", default-features = false, features = ["std"] }
serde_json = "=1.0.151"
[profile.ci]
inherits = "dev"
opt-level = 1
debug = 1
`);
fs.writeFileSync(path.join(destination, 'source.json'), JSON.stringify({ diagnostic_only: true,
  original_sha256: hash(original), fixture_sha256: hash(fixture), variants: rows,
  scope: 'Standalone QuickJS and Bevy cost diagnostics; no native host, shipping or gate measurement.',
}, null, 2) + '\n');
console.log(JSON.stringify({ destination, variants: rows.length, original_sha256: hash(original) }));
