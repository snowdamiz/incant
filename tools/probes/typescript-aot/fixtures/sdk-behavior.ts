import type { NativeBehavior } from '@incant/runtime/native';
const rows = new WeakMap<readonly string[], ReadonlyArray<number>>();
export default {
  persistence: { version: 1 },
  update(context) {
    for (const chunk of context.chunks) {
      let indexes = rows.get(chunk.ids);
      if (!indexes) {
        indexes = Object.freeze(chunk.ids.map((_, index) => index));
        rows.set(chunk.ids, indexes);
      }
      for (const row of indexes) chunk.transforms[row * chunk.transformStride] += context.dt;
    }
    for (const instance of context.instances) {
      const speed = instance.props.speed;
      if (typeof speed === 'number' && !Number.isFinite(speed)) throw Error('nonfinite');
    }
    context.log('aot-language-fixture');
    return undefined;
  },
} satisfies NativeBehavior;
