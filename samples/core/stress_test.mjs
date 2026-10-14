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
function context(tick, reverse = false, previous, stride = 10) {
  const rows = ['core-controller', ...instances.map(i => i.entity)];
  if (reverse) rows.reverse();
  const chunks = [];
  for (let start = 0; start < rows.length; start += 137) {
    const ids = previous?.chunks[chunks.length].ids ?? Object.freeze(rows.slice(start, start + 137));
    const transforms = new Float64Array(ids.length * stride).fill(777);
    chunks.push({ ids, transforms, transformStride: stride });
  }
  return { instances, chunks, clock: { tick } };
}
function read(c, entity) {
  for (const chunk of c.chunks) {
    const row = chunk.ids.indexOf(entity);
    if (row >= 0) return Array.from(chunk.transforms.slice(row * chunk.transformStride, row * chunk.transformStride + 3));
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
let previous;
for (const [tick, reuse, reverse, stride] of [[1, false, false, 10], [63, true, false, 13],
    [64, false, true, 10], [65, true, true, 11]]) {
  const c = context(tick, reverse, reuse ? previous : undefined, stride);
  // Native loans are revoked after each tick. Matching immutable IDs do not
  // authorize retaining a previous chunk, view or stride.
  if (previous) for (const chunk of previous.chunks) structuredClone(chunk.transforms.buffer, { transfer: [chunk.transforms.buffer] });
  assert.equal(behavior.update(c), undefined);
  assert.deepEqual(read(c, 'core-controller'), [777, 777, 777]);
  for (let n = 1; n < 9999; n++) assert.deepEqual(read(c, id(n)), expected(tick, n));
  read(c, id(0)).forEach((v, i) => assert.ok(Math.abs(v - sentinel(tick)[i]) < 1e-12));
  for (const chunk of c.chunks) for (let n = 0; n < chunk.ids.length; n++)
    assert.ok(Array.from(chunk.transforms.slice(n * stride + 3, n * stride + stride)).every(v => v === 777));
  previous = c;
}
for (const invalid of ['few', 'ordinal', 'duplicate', 'extra-prop', 'missing-row']) {
  const c = context(1); c.instances = structuredClone(instances);
  if (invalid === 'few') c.instances.pop();
  if (invalid === 'ordinal') c.instances[9].props.ordinal = 8;
  if (invalid === 'duplicate') c.instances[9].entity = c.instances[8].entity;
  if (invalid === 'extra-prop') c.instances[9].props.extra = 1;
  if (invalid === 'missing-row') c.chunks[0].ids = Object.freeze(c.chunks[0].ids.map((v, i) => i === 10 ? 'not-owned' : v));
  assert.throws(() => load().update(c), /Core stress/);
}
// An omitted actual write must fail the checksum observable, even with count intact.
const broken = source.replace('transforms[offset] = values[index + 1] + phase;', '/* omitted x write */');
assert.notEqual(broken, source);
const c = context(2); load(broken).update(c);
assert.ok(Math.abs(read(c, id(0))[1] - sentinel(2)[1]) > 1);

// A new valid attachment snapshot may keep the same world rows but change the
// owned ordinal mapping. Cached metadata must not preserve the old assignments.
const changed = context(66, false, previous);
changed.instances = structuredClone(instances);
[changed.instances[1].entity, changed.instances[97].entity] = [changed.instances[97].entity, changed.instances[1].entity];
behavior.update(changed);
assert.deepEqual(read(changed, id(1)), expected(66, 97));
assert.deepEqual(read(changed, id(97)), expected(66, 1));
changed.instances = structuredClone(changed.instances);
changed.instances[1].props.ordinal = 97;
const beforeRejected = changed.chunks.map(chunk => Buffer.from(chunk.transforms.buffer).toString('hex'));
assert.throws(() => behavior.update(changed), /Core stress membership/);
assert.deepEqual(changed.chunks.map(chunk => Buffer.from(chunk.transforms.buffer).toString('hex')), beforeRejected);
console.log('Core stress controls passed: all 9999 rows/four phases, replaced row and attachment snapshots, detached old views, changed stride, five refusals and omitted-write mutation.');
