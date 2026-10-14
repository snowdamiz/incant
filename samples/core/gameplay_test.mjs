// Observable Core row selection with fresh borrowed views; no engine/timing claim.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';

const source = fs.readFileSync(new URL('gameplay.ts', import.meta.url), 'utf8');
const js = ts.transpileModule(source, { compilerOptions: {
  module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022,
} }).outputText;
const box = { exports: {} }; vm.runInNewContext(js, box);
const behavior = box.exports.default;
const id = n => String(n).padStart(26, '0');
const props = { player: id(1), navigation: id(2), exit: id(3),
  enemies: [id(4)], pickups: [id(5)], camera: id(6) };
let state = { version: 1, health: 3, collected: [], phase: 'won',
  invulnerableUntil: 0, verticalSpeed: 0, grounded: true,
  routes: [{ points: [], cursor: 0, lastPlan: -1 }], previousPlayer: [0, 0, 0],
  jumps: 0, searches: 0 };
let previous;
function context(rows, stride, at) {
  const chunks = rows.map(ids => {
    const transforms = new Float64Array(ids.length * stride).fill(777);
    const velocities = new Float64Array(ids.length * 4).fill(999);
    for (let row = 0; row < ids.length; row++) {
      const p = ids[row] === props.player ? at : ids[row] === props.camera ? [20, 30, 40] : [10, 10, 10];
      transforms.set(p, row * stride);
    }
    return { ids, transforms, velocities, transformStride: stride, velocityStride: 4 };
  });
  if (previous) for (const chunk of previous.chunks) for (const v of [chunk.transforms, chunk.velocities])
    structuredClone(v.buffer, { transfer: [v.buffer] });
  const c = { chunks, instances: [{ entity: id(7), props }], clock: { tick: 1 },
    input: { actions: {} }, state: { get: () => state, set: value => { state = structuredClone(value); } } };
  previous = c;
  return c;
}
const one = Object.freeze(Array.from({ length: 10000 }, (_, n) => id(n + 1)));
const reordered = Object.freeze([...one].reverse());
const layouts = [[one], [one], [reordered],
  [Object.freeze(reordered.slice(0, 5000)), Object.freeze(reordered.slice(5000))]];
for (let tick = 0; tick < layouts.length; tick++) {
  const oldPlayer = [...state.previousPlayer], at = [tick + 2, tick + 3, tick + 4];
  const stride = tick % 2 ? 13 : 10;
  const c = context(layouts[tick], stride, at);
  assert.equal(behavior.update(c), undefined);
  assert.deepEqual(state.previousPlayer, at);
  for (const chunk of c.chunks) for (let row = 0; row < chunk.ids.length; row++) {
    const entity = chunk.ids[row];
    const expectedPosition = entity === props.player ? at : entity === props.camera
      ? [20, 30, 40].map((v, axis) => v + at[axis] - oldPlayer[axis]) : [10, 10, 10];
    assert.deepEqual(Array.from(chunk.transforms.slice(row * stride, row * stride + 3)), expectedPosition);
    assert.ok(Array.from(chunk.transforms.slice(row * stride + 3, (row + 1) * stride)).every(v => v === 777));
    assert.deepEqual(Array.from(chunk.velocities.slice(row * 4, row * 4 + 4)),
      [props.player, props.enemies[0]].includes(entity) ? [0, 0, 0, 999] : [999, 999, 999, 999]);
  }
}
// A row removed after a successful lookup must not resolve through old metadata.
const removed = Object.freeze(one.filter(entity => entity !== props.enemies[0]));
assert.throws(() => behavior.update(context([removed], 10, [2, 3, 4])), /Core Sample entity is missing/);
console.log('Core gameplay row controls passed: 10000 rows, immutable snapshot reuse/reorder/split/removal, detached prior loans, current strides, camera and velocity outputs.');
