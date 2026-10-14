// Independent row oracle. Fresh borrowed-style views each tick; only immutable
// IDs and the backing allocation are reused. No host/loan revocation claim.
const instances = Object.freeze(Array.from({ length: 9999 }, (_, ordinal) =>
  Object.freeze({ entity: `row${ordinal}`, props: Object.freeze({ ordinal }) })));
let selectedInstances = instances;
let ids = Object.freeze(['controller', ...instances.map(value => value.entity)]);
let stride = 10;
let buffer = new ArrayBuffer(ids.length * stride * 8);
new Float64Array(buffer).fill(777);
globalThis.update = tick => exports.default.update({
  instances: selectedInstances, clock: { tick },
  chunks: [{ ids, transformStride: stride, transforms: new Float64Array(buffer) }],
});
globalThis.verify = tick => {
  const data = new Float64Array(buffer), phase = (tick % 64 - 32) / 64;
  const ordinals = new Map(selectedInstances.map(value => [value.entity, value.props.ordinal]));
  let sx = 0, sz = 0, sentinel = -1;
  for (let row = 0; row < ids.length; row++) {
    const ordinal = ordinals.get(ids[row]);
    if (ordinal === undefined) {
      for (let n = 0; n < stride; n++) if (data[row * stride + n] !== 777) throw Error('controller changed');
      continue;
    }
    const expected = [ordinal % 97 - 48 + phase, Math.floor(ordinal / 97) % 97 - 48,
      ordinal % 13 - 6 - phase];
    sx += expected[0]; sz += expected[2];
    if (ordinal === 0) sentinel = row;
    else for (let n = 0; n < 3; n++) if (data[row * stride + n] !== expected[n]) throw Error(`row ${row} column ${n}`);
    for (let n = 3; n < stride; n++) if (data[row * stride + n] !== 777) throw Error('padding changed');
  }
  if (sentinel < 0 || data[sentinel * stride] !== 9999 ||
      !Number.isFinite(data[sentinel * stride + 1]) || !Number.isFinite(data[sentinel * stride + 2]) ||
      Math.abs(data[sentinel * stride + 1] - sx / 9999) > 1e-12 ||
      Math.abs(data[sentinel * stride + 2] - sz / 9999) > 1e-12) throw Error('sentinel mismatch');
};
globalThis.transitions = () => {
  // Matching IDs with changed stride must use new offsets, never retained views.
  stride = 13; buffer = new ArrayBuffer(ids.length * stride * 8);
  new Float64Array(buffer).fill(777); update(501); verify(501);
  ids = Object.freeze([...ids].reverse());
  buffer = new ArrayBuffer(ids.length * stride * 8);
  new Float64Array(buffer).fill(777); update(502); verify(502);
  selectedInstances = Object.freeze(instances.map((value, ordinal) => Object.freeze({
    entity: ordinal === 1 ? 'row97' : ordinal === 97 ? 'row1' : value.entity,
    props: value.props,
  })));
  update(503); verify(503);
};
