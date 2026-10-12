import type { NativeBehavior } from '@incant/runtime/native';
const id = (n: number) => String(n).padStart(26, '0');
const states = [20, 21, 22, 23, 30].map(id);
const parameters = [10, 11, 12, 13].map(id);
export default {
  persistence: { version: 1 },
  update(t) {
    const a = t.animations, g = a.graphs, c = t.commands, tick = t.clock.tick;
    if (a.ids.length !== 2 || g.ids.length !== 2 || a.poseOffsets.length !== 3) throw Error('animation membership');
    const vectors = new Map<string, number[]>();
    for (const instance of t.instances) {
      const entity = instance.entity, i = a.ids.indexOf(entity), j = g.ids.indexOf(entity);
      if (i < 0 || j < 0 || a.transitionFlags[i] !== 0) throw Error('compound graph row');
      const poses = [];
      for (let k = a.poseOffsets[i]; k < a.poseOffsets[i + 1]; k++) poses.push([a.poseClips[k], a.poseSeconds[k], a.poseWeights[k]]);
      const row = {
        tick, entity, state: states.indexOf(g.states[j]), seconds: g.seconds[j], elapsed: g.elapsed[j],
        flags: a.flags[i], paused: g.paused[j], sample: [a.clips[i], a.seconds[i]],
        blend: [g.blending[j], g.sourceClips[j], g.sourceSeconds[j], g.weights[j]],
        kinds: [a.poseKinds[i * 2], a.poseKinds[i * 2 + 1]], split: a.poseSplits[i], outer: a.poseOuterWeights[i],
        fade: [g.fadeStates[j] === null ? -1 : states.indexOf(g.fadeStates[j]!), g.fadeSeconds[j],
          g.fadeSpeeds[j], g.fadeFlags[j], g.fadeElapsed[j], g.fadeDuration[j]],
        poses, parameters: [...g.parameterValues.slice(g.parameterOffsets[j], g.parameterOffsets[j + 1])],
      };
      const count = Number(t.state.get(entity) ?? 0) + 1;
      if (count !== tick) throw Error('persistent script count');
      t.state.set(count, entity);
      if (instance.props.rest === true) {
        vectors.set(id(230), [row.seconds, row.flags, row.paused]);
        if (tick === 5) c.controlAnimation(entity, 'playing', true);
        if (tick === 8) c.controlAnimation(entity, 'playing', false);
        if (tick === 11) c.controlAnimation(entity, 'playing', true);
      } else {
        vectors.set(id(220), [row.split, row.outer, poses.length]);
        vectors.set(id(221), [row.state, row.seconds, row.flags]);
        vectors.set(id(222), [row.fade[0], row.fade[4], row.paused]);
        if (tick === 4 || tick === 36) c.setAnimationParameter(entity, parameters[3], 1);
        if (tick === 7 || tick === 10) c.controlAnimation(entity, 'playing', tick === 10);
        if (tick === 9) {
          c.controlAnimation(entity, 'speed', -15);
          [0.5, 0.25, 0.5].forEach((value, k) => c.setAnimationParameter(entity, parameters[k], value));
        }
        if (tick === 12) c.setAnimationParameter(entity, parameters[3], 2);
        if (tick === 23 || tick === 43) c.setAnimationParameter(entity, parameters[3], 3);
        if (tick === 30 || tick === 39) {
          c.selectAnimationState(entity, states[tick === 30 ? 0 : 2]);
          c.setAnimationParameter(entity, parameters[3], 0);
        }
        if (tick === 36) c.controlAnimation(entity, 'speed', 15);
        if (tick === 50) c.setAnimationParameter(entity, parameters[3], 4);
      }
      t.log(JSON.stringify(row));
    }
    for (const chunk of t.chunks) for (let n = 0; n < chunk.ids.length; n++) {
      const v = vectors.get(chunk.ids[n]);
      if (v) { const k = n * chunk.transformStride; chunk.transforms[k] = v[0]; chunk.transforms[k + 1] = v[1]; chunk.transforms[k + 2] = v[2]; }
    }
    // Mutating owned columns must never mutate native state, including retained groups.
    a.poseWeights.fill(99); a.poseClips.fill(99); a.poseSeconds.fill(99);
    a.poseKinds.fill(99); a.poseSplits.fill(99); a.poseOuterWeights.fill(99);
    g.fadeSeconds.fill(99); g.fadeSpeeds.fill(99); g.fadeElapsed.fill(99); g.fadeDuration.fill(99);
  },
} satisfies NativeBehavior;
