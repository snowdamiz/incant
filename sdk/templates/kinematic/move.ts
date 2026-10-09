/// <reference path="../../ts/src/index.d.ts" />
export default defineBehavior({
  initialState: { ticks: 0 },
  update(api, dt, state) {
    state.ticks++;
    for (const entity of api.query('Transform')) {
      const transform = entity.components.Transform as {
        translation: number[]; rotation: number[]; scale: number[];
      };
      api.command({
        op: 'set_component', scene_id: entity.scene_id, entity_id: entity.id,
        component: 'Transform',
        value: { ...transform, translation: [transform.translation[0] + dt, transform.translation[1], transform.translation[2]] },
      });
    }
  },
});
