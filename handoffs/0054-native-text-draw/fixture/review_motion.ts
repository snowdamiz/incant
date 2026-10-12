import type { NativeBehavior } from '@incant/runtime/native';
// Review motion only: props give each instance a base position, a horizontal
// sway amplitude and a yaw amplitude; poses derive from simulation time.
export default {
  update({ instances, chunks, clock }) {
    const t = clock.elapsed_seconds;
    for (const instance of instances) {
      const base = instance.props.base as readonly number[];
      const sway = Number(instance.props.sway);
      const yaw = Number(instance.props.spin) * Math.sin(0.7 * t);
      for (const chunk of chunks) {
        const i = chunk.ids.indexOf(instance.entity);
        if (i < 0) continue;
        const o = i * chunk.transformStride;
        chunk.transforms[o] = base[0] + sway * Math.sin(0.8 * t);
        chunk.transforms[o + 1] = base[1];
        chunk.transforms[o + 2] = base[2];
        chunk.transforms[o + 3] = 0;
        chunk.transforms[o + 4] = Math.sin(yaw / 2);
        chunk.transforms[o + 5] = 0;
        chunk.transforms[o + 6] = Math.cos(yaw / 2);
      }
    }
    return undefined;
  },
} satisfies NativeBehavior;
