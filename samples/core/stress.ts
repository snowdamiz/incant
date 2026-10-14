import type { NativeBehavior, NativeChunk, NativeContext } from '@incant/runtime/native';

// One callback for 9,999 authored attachments, beside the unchanged Core controller.
// No borrowed chunk/view survives an update, and no render/physics component is used.
const COUNT = 9999;
let membership: Map<string, number> | undefined;
function members(context: NativeContext): Map<string, number> {
  if (context.instances.length !== COUNT) throw Error('Core stress requires 9999 attachments');
  if (!membership) {
    const ids = new Map<string, number>();
    for (let index = 0; index < COUNT; index++) {
      const instance = context.instances[index], ordinal = instance.props.ordinal;
      if (typeof ordinal !== 'number' || ordinal !== index ||
          Object.keys(instance.props).length !== 1 || ids.has(instance.entity))
        throw Error('Core stress membership is not the exact ordered ordinal set');
      ids.set(instance.entity, ordinal);
    }
    membership = ids;
  }
  return membership;
}
export default {
  persistence: { version: 1 },
  update(context: NativeContext) {
    const owned = members(context), tick = context.clock.tick;
    if (!Number.isSafeInteger(tick) || tick < 1) throw Error('Core stress invalid tick');
    const phase = (tick % 64 - 32) / 64;
    let count = 0, checksumX = 0, checksumZ = 0;
    let sentinel: NativeChunk | undefined, sentinelOffset = 0;
    for (const chunk of context.chunks) for (let row = 0; row < chunk.ids.length; row++) {
      const ordinal = owned.get(chunk.ids[row]);
      if (ordinal === undefined) continue;
      const offset = row * chunk.transformStride;
      chunk.transforms[offset] = ordinal % 97 - 48 + phase;
      chunk.transforms[offset + 1] = Math.floor(ordinal / 97) % 97 - 48;
      chunk.transforms[offset + 2] = ordinal % 13 - 6 - phase;
      // Read back each actual written column; an omitted-write mutation changes
      // these independent aggregate observables, not just a private loop counter.
      checksumX += chunk.transforms[offset];
      checksumZ += chunk.transforms[offset + 2];
      count++;
      if (ordinal === 0) { sentinel = chunk; sentinelOffset = offset; }
    }
    if (count !== COUNT || !sentinel) throw Error('Core stress bulk coverage is incomplete');
    sentinel.transforms[sentinelOffset] = count;
    sentinel.transforms[sentinelOffset + 1] = checksumX / COUNT;
    sentinel.transforms[sentinelOffset + 2] = checksumZ / COUNT;
    return undefined;
  },
} satisfies NativeBehavior;
