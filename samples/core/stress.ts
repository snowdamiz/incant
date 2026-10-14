import type { NativeBehavior, NativeChunk, NativeContext } from '@incant/runtime/native';

// One callback for 9,999 authored attachments, beside the unchanged Core controller.
// No borrowed chunk/view survives an update, and no render/physics component is used.
const COUNT = 9999;
let membership: Map<string, number> | undefined;
let sourceInstances: NativeContext['instances'] | undefined;
type Rows = { values: readonly number[]; sentinelRow: number | undefined };
let rowCache = new WeakMap<NativeChunk['ids'], Rows>();
function members(context: NativeContext): Map<string, number> {
  if (context.instances.length !== COUNT) throw Error('Core stress requires 9999 attachments');
  if (!membership || sourceInstances !== context.instances) {
    const ids = new Map<string, number>();
    for (let index = 0; index < COUNT; index++) {
      const instance = context.instances[index], ordinal = instance.props.ordinal;
      if (typeof ordinal !== 'number' || ordinal !== index ||
          Object.keys(instance.props).length !== 1 || ids.has(instance.entity))
        throw Error('Core stress membership is not the exact ordered ordinal set');
      ids.set(instance.entity, ordinal);
    }
    membership = ids;
    sourceInstances = context.instances;
    rowCache = new WeakMap();
  }
  return membership;
}
// Store row indices and invariant coordinates only. All writes and checksum
// reads still use this tick's borrowed numeric columns and current stride.
function rowsFor(ids: NativeChunk['ids'], owned: Map<string, number>): Rows {
  let rows = rowCache.get(ids);
  if (!rows) {
    const values: number[] = [];
    let sentinelRow: number | undefined;
    for (let row = 0; row < ids.length; row++) {
      const ordinal = owned.get(ids[row]);
      if (ordinal === undefined) continue;
      values.push(row, ordinal % 97 - 48, Math.floor(ordinal / 97) % 97 - 48, ordinal % 13 - 6);
      if (ordinal === 0) sentinelRow = row;
    }
    rows = { values, sentinelRow };
    rowCache.set(ids, rows);
  }
  return rows;
}
export default {
  persistence: { version: 1 },
  update(context: NativeContext) {
    const owned = members(context), tick = context.clock.tick;
    if (!Number.isSafeInteger(tick) || tick < 1) throw Error('Core stress invalid tick');
    const phase = (tick % 64 - 32) / 64;
    let count = 0, checksumX = 0, checksumZ = 0;
    let sentinel: NativeChunk | undefined, sentinelOffset = 0;
    for (const chunk of context.chunks) {
      const { values, sentinelRow } = rowsFor(chunk.ids, owned);
      const transforms = chunk.transforms, stride = chunk.transformStride;
      for (let index = 0; index < values.length; index += 4) {
        const offset = values[index] * stride;
        transforms[offset] = values[index + 1] + phase;
        transforms[offset + 1] = values[index + 2];
        transforms[offset + 2] = values[index + 3] - phase;
        // Read back every actual write; the checksum must detect an omitted write.
        checksumX += transforms[offset];
        checksumZ += transforms[offset + 2];
        count++;
      }
      if (sentinelRow !== undefined) { sentinel = chunk; sentinelOffset = sentinelRow * stride; }
    }
    if (count !== COUNT || !sentinel) throw Error('Core stress bulk coverage is incomplete');
    sentinel.transforms[sentinelOffset] = count;
    sentinel.transforms[sentinelOffset + 1] = checksumX / COUNT;
    sentinel.transforms[sentinelOffset + 2] = checksumZ / COUNT;
    return undefined;
  },
} satisfies NativeBehavior;
