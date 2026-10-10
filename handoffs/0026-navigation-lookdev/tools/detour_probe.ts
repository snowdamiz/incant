// Detour diagnostic for handoff 0026 (no rendering). Applies the same barrier
// edit as navigation_course.ts on tick 1, then from tick 2 issues one query per
// tick (each costs 64 of 256 units) from the replanned start (-3.44, 0, 0) to the
// doorway-B jamb point and to intermediate points,
// logging every returned polyline. Used to test whether a shorter connected
// route than the returned one exists on the edited mesh.
import type { Json, ScriptApi } from '../../../sdk/ts/src/index';

type Vec3 = [number, number, number];
const FROM: Vec3 = [-3.44, 0, 0];
const QUERIES: [Vec3, Vec3][] = [
  [FROM, [-0.3, 0, -3.3]], [FROM, [-0.6, 0, -3.0]], [FROM, [-0.8, 0, -2.9]], [FROM, [-1.0, 0, -2.6]],
  [FROM, [-1.5, 0, -2.0]], [FROM, [-2.0, 0, -1.5]], [[-0.8, 0, -2.9], [-0.3, 0, -3.3]], [FROM, [6.6, 0, 1.2]],
  [[-0.8, 0, -2.9], [6.6, 0, 1.2]],
];

export default defineBehavior<{ tick: number } & Record<string, Json>>({
  initialState: { tick: 0 },
  update(api: ScriptApi, _dt: number, state) {
    state.tick += 1;
    const nav = api.query('NavigationMesh')[0]!;
    const barrier = api.query('Collider').find((e) => e.name === 'Barrier')!;
    if (state.tick === 1) {
      api.command({ op: 'set_component', scene_id: nav.scene_id, entity_id: barrier.id, component: 'Transform',
        value: { translation: [0, 0.5, 1.2], rotation: [0, 0, 0, 1], scale: [1, 1, 1] } });
      return;
    }
    const query = QUERIES[state.tick - 2];
    if (query) {
      const [start, end] = query;
      const path = api.findPath({ scene_id: nav.scene_id, mesh_entity: nav.id,
        path: { start, end, snap_distance: 1, max_visited: 4000 } });
      api.log(JSON.stringify({ event: 'probe', start, end, visited: path ? path.visited : null, points: path ? path.points : null, generation: path ? path.generation : null }));
    }
  },
});
