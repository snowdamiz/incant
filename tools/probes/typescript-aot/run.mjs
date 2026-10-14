#!/usr/bin/env node
/** Fixed trusted probe only. Never runs project inputs or compiles native code. */
import fs from 'node:fs';
import path from 'node:path';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import ts from 'typescript';
import { transformSync } from '@swc/core';
import { cookNativeScript } from '../../native_script/cook.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, '../../..');
const FIXTURES = path.join(HERE, 'fixtures');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const read = file => fs.readFileSync(file);
function inventory(root) {
  const result = {};
  function visit(relative) {
    for (const item of fs.readdirSync(path.join(root, relative), { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
      const name = path.join(relative, item.name);
      if (item.isDirectory()) visit(name);
      else if (item.isFile()) result[name] = hash(read(path.join(root, name)));
      else if (item.isSymbolicLink()) result[name] = { link: fs.readlinkSync(path.join(root, name)) };
      else throw Error('Unexpected toolchain file: ' + name);
    }
  }
  visit('');
  return result;
}
const require = createRequire(import.meta.url);
const [outArg, toolchainArg] = process.argv.slice(2);
if (!outArg || !toolchainArg || process.argv.length !== 4)
  throw Error('Usage: node tools/probes/typescript-aot/run.mjs NEW_OUTPUT INSTALLED_PINNED_TOOLCHAIN');
const out = path.resolve(outArg), toolchain = path.resolve(toolchainArg);
fs.mkdirSync(out); // Exclusive run: existing results cannot be overwritten.
const receipt = { version: 1, kind: 'typescript-aot-language-spike', completed: false,
  node: { version: process.version, executable: process.execPath, sha256: hash(read(process.execPath)) },
  typescript: ts.version, swc: require('@swc/core/package.json').version,
  scope: 'Trusted language/compiler controls in Node; no engine host, native compiler, benchmark, iOS or complete SDK conformance.',
  sources: {}, commands: [], cases: [], decision18: {
    sdk100percent: 'unmeasured', nativeMac2x: 'unmeasured', iosNoJit: 'unmeasured',
    mobileAddedBytesUnder5MB: 'unmeasured', maintainedSecurityRelease: 'unresolved', adopted: false },
};
function save() { fs.writeFileSync(path.join(out, 'result.json'), JSON.stringify(receipt, null, 2) + '\n'); }
try {
  const lock = JSON.parse(read(path.join(HERE, 'toolchain/package-lock.json')));
  assert.equal(hash(read(path.join(toolchain, 'package-lock.json'))), hash(read(path.join(HERE, 'toolchain/package-lock.json'))));
  for (const [name, expected] of Object.entries(lock.packages)) {
    if (!name) continue;
    const installed = JSON.parse(read(path.join(toolchain, name, 'package.json')));
    assert.equal(installed.version, expected.version, name);
  }
  const asc = path.join(toolchain, 'node_modules/assemblyscript/bin/asc.js');
  const toolchainFiles = inventory(path.join(toolchain, 'node_modules'));
  fs.writeFileSync(path.join(out, 'toolchain-files.json'), JSON.stringify(toolchainFiles, null, 2) + '\n');
  receipt.assemblyscript = { version: '0.28.20', lock_sha256: hash(read(path.join(HERE, 'toolchain/package-lock.json'))), compiler_sha256: hash(read(asc)) };
  receipt.assemblyscript.installed_files_sha256 = hash(read(path.join(out, 'toolchain-files.json')));
  for (const name of fs.readdirSync(FIXTURES).sort()) receipt.sources['fixtures/' + name] = hash(read(path.join(FIXTURES, name)));
  for (const name of ['sdk/ts/src/native.d.ts', 'sdk/ts/src/generated.d.ts', 'tools/native_script/cook.mjs', 'tools/native_script/graph.mjs', 'tools/native_script/typecheck_host.mjs'])
    receipt.sources[name] = hash(read(path.join(ROOT, name)));
  receipt.sources['run.mjs'] = hash(read(fileURLToPath(import.meta.url)));
  const cooked = await cookNativeScript(FIXTURES, 'sdk-behavior.ts');
  fs.writeFileSync(path.join(out, 'sdk-behavior.cjs'), cooked.code);
  fs.writeFileSync(path.join(out, 'sdk-cook.json'), JSON.stringify(cooked.manifest, null, 2) + '\n');
  receipt.sdk_frontend = { passed: true, bundle_sha256: cooked.manifest.bundleSha256,
    scope: 'Actual current native strict TypeScript/SWC cook only; no host execution.' };
  const expected = { 'recursive-props': 3, 'weak-metadata': 9, closure: 3, 'caught-error': 7, 'for-of': 6 };
  for (const name of [...Object.keys(expected), 'bulk-kernel']) {
    const source = path.join(FIXTURES, name + '.ts');
    const program = ts.createProgram([source], { strict: true, noEmit: true, target: ts.ScriptTarget.ES2022,
      module: ts.ModuleKind.ESNext, types: [], lib: ['lib.es2022.d.ts'] });
    const diagnostics = ts.getPreEmitDiagnostics(program);
    assert.equal(diagnostics.length, 0, diagnostics.map(d => ts.flattenDiagnosticMessageText(d.messageText, '\n')).join('\n'));
    const js = transformSync(read(source).toString(), { swcrc: false, configFile: false,
      jsc: { parser: { syntax: 'typescript' }, target: 'es2022' }, module: { type: 'commonjs' } }).code;
    // Prepared input only: a future pinned shermes build can consume these without
    // a CommonJS loader. Retain TypeScript separately; erased JS is not typed mode.
    fs.writeFileSync(path.join(out, name + '.hermes.js'), 'var exports = {};\n' + js
      + (name === 'bulk-kernel'
        ? '\n[1,32,64,120].forEach(function(t) { print(exports.update(t), exports.first(), exports.last()); });\n'
        : '\nprint(exports.probe());\n'));
    const context = vm.createContext({ exports: {} });
    new vm.Script(js, { filename: name + '.js' }).runInContext(context, { timeout: 1000 });
    const jsValues = name === 'bulk-kernel' ? [1, 32, 64, 120].map(tick => {
      context.tick = tick;
      return new vm.Script('[exports.update(tick), exports.first(), exports.last()]').runInContext(context, { timeout: 1000 });
    }) : [new vm.Script('exports.probe()').runInContext(context, { timeout: 1000 })];
    if (name !== 'bulk-kernel') assert.equal(jsValues[0], expected[name]);
    const args = [asc, source, '--outFile', path.join(out, name + '.wasm'), '--textFile', path.join(out, name + '.wat'),
      '--optimizeLevel', '3', '--shrinkLevel', '0', '--initialMemory', '8', '--maximumMemory', '16'];
    const started = Date.now();
    const done = spawnSync(process.execPath, args, { cwd: out, timeout: 60000, maxBuffer: 2 * 1024 * 1024,
      env: { PATH: path.dirname(process.execPath), HOME: out }, encoding: 'utf8' });
    for (const stream of ['stdout', 'stderr']) fs.writeFileSync(path.join(out, name + '.' + stream), done[stream] ?? '');
    receipt.commands.push({ name, argv: [process.execPath, ...args], started, ended: Date.now(), exit: done.status,
      signal: done.signal, error: done.error?.message ?? null,
      stdout_sha256: hash(Buffer.from(done.stdout ?? '')), stderr_sha256: hash(Buffer.from(done.stderr ?? '')) });
    if (done.error || done.signal || done.status === null) throw Error('Compiler did not finish: ' + name);
    const row = { name, strict_ts_passed: true, node_values: jsValues,
      assembly_exit: done.status, assembly_compiled: done.status === 0, wasm_execution: 'not_run' };
    if (done.status === 0) {
      const wasm = read(path.join(out, name + '.wasm'));
      const { instance } = await WebAssembly.instantiate(wasm, { env: { abort() { throw Error('AssemblyScript abort'); } } });
      const actual = name === 'bulk-kernel' ? [1, 32, 64, 120].map(tick =>
        [instance.exports.update(tick), instance.exports.first(), instance.exports.last()]) : [instance.exports.probe()];
      assert.equal(JSON.stringify(actual), JSON.stringify(jsValues), name + ' differs');
      row.wasm_execution = 'node_wasm_values_match'; row.wasm_sha256 = hash(wasm); row.wasm_bytes = wasm.length;
      // These values compare language arithmetic only. No Node elapsed time is an adoption benchmark.
    }
    receipt.cases.push(row); save();
  }
  assert.equal(receipt.cases.find(c => c.name === 'bulk-kernel').wasm_execution, 'node_wasm_values_match');
  assert.deepEqual(inventory(path.join(toolchain, 'node_modules')), toolchainFiles, 'Compiler toolchain changed during execution');
  receipt.completed = true;
} catch (error) {
  receipt.failure = String(error); throw error;
} finally { save(); }
console.log(JSON.stringify({ completed: receipt.completed, cases: receipt.cases.map(c => ({ name: c.name, assembly_exit: c.assembly_exit, wasm_execution: c.wasm_execution })), decision18: receipt.decision18 }));
