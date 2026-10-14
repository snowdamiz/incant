// Typed bulk contract controls, run under Node; not QuickJS/native timing proof.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';

const source = fs.readFileSync(new URL('stress.ts', import.meta.url), 'utf8');
const load = (text = source) => {
  const js = ts.transpileModule(text, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText;
  const box = { exports: {} }; vm.runInNewContext(js, box); return box.exports.default;
};
const id = n => `row-${String(n).padStart(5, '0')}`;
const instances = Array.from({ length: 9999 }, (_, n) => ({ entity: id(n), props: { ordinal: n } }));
function context(tick, reverse = false) {
  const rows = ['core-controller', ...instances.map(i => i.entity)];
  if (reverse) rows.reverse();
  const chunks = [];
  for (let start = 0; start < rows.length; start += 137) {
    const ids = rows.slice(start, start + 137), transforms = new Float64Array(ids.length * 10).fill(777);
    chunks.push({ ids, transforms, transformStride: 10 });
  }
  return { instances, chunks, clock: { tick } };
}
function read(c, entity) {
  for (const chunk of c.chunks) {
    const row = chunk.ids.indexOf(entity);
    if (row >= 0) return Array.from(chunk.transforms.slice(row * 10, row * 10 + 3));
  }
  throw Error('absent row');
}
function expected(tick, n) {
  const phase = (tick % 64 - 32) / 64;
  return [n % 97 - 48 + phase, Math.floor(n / 97) % 97 - 48, n % 13 - 6 - phase];
}
function sentinel(tick) {
  const residue = (n, m) => Math.floor(n / m) * m * (m - 1) / 2 + (n % m) * (n % m - 1) / 2;
  const phase = (tick % 64 - 32) / 64;
  return [9999, residue(9999, 97) / 9999 - 48 + phase, residue(9999, 13) / 9999 - 6 - phase];
}
const behavior = load();
for (const tick of [1, 63, 64, 65]) {
  const c = context(tick, tick % 2 === 0);
  assert.equal(behavior.update(c), undefined);
  assert.deepEqual(read(c, 'core-controller'), [777, 777, 777]);
  for (let n = 1; n < 9999; n++) assert.deepEqual(read(c, id(n)), expected(tick, n));
  read(c, id(0)).forEach((v, i) => assert.ok(Math.abs(v - sentinel(tick)[i]) < 1e-12));
  for (const chunk of c.chunks) for (let n = 0; n < chunk.ids.length; n++)
    assert.ok(Array.from(chunk.transforms.slice(n * 10 + 3, n * 10 + 10)).every(v => v === 777));
}
for (const invalid of ['few', 'ordinal', 'duplicate', 'extra-prop', 'missing-row']) {
  const c = context(1); c.instances = structuredClone(instances);
  if (invalid === 'few') c.instances.pop();
  if (invalid === 'ordinal') c.instances[9].props.ordinal = 8;
  if (invalid === 'duplicate') c.instances[9].entity = c.instances[8].entity;
  if (invalid === 'extra-prop') c.instances[9].props.extra = 1;
  if (invalid === 'missing-row') c.chunks[0].ids[10] = 'not-owned';
  assert.throws(() => load().update(c), /Core stress/);
}
// An omitted actual write must fail the checksum observable, even with count intact.
const broken = source.replace('chunk.transforms[offset] = ordinal % 97 - 48 + phase;', '/* omitted x write */');
assert.notEqual(broken, source);
const c = context(2); load(broken).update(c);
assert.ok(Math.abs(read(c, id(0))[1] - sentinel(2)[1]) > 1);
console.log('Core stress typed bulk controls passed: all 9999 rows, four phases, five refusals, omitted-write mutation.');
