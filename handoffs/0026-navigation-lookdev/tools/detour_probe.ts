// Detour diagnostic for handoff 0026 (no rendering). Applies the same barrier
// edit as navigation_course.ts on tick 1, then from tick 2 issues one query per
// tick (each costs 64 of 256 units): the v6 fixed-start diagnostic from
// (-3.44, 0, 0), then legs from the v7 replan start (-3.291, 0, 2) to the
// doorway-B jamb via intermediate points, logging every returned polyline. Used to test whether a shorter connected
// route than the returned one exists on the edited mesh.
import type { Json, ScriptApi } from '../../../sdk/ts/src/index';

type Vec3 = [number, number, number];
const FROM: Vec3 = [-3.44, 0, 0];
const GOAL: Vec3 = [6.6, 0, 1.2];
const JAMB: Vec3 = [-0.7, 0, -2.9];
const V7_START: Vec3 = [-3.291, 0, 2];
// v6 fixed-start diagnostic, v6 detour legs, then the v7 replan leg around the pillar.
const QUERIES: [Vec3, Vec3][] = [
  [FROM, GOAL], [FROM, [-0.3, 0, -3.3]], [FROM, [-0.8, 0, -2.9]], [[-0.8, 0, -2.9], [-0.3, 0, -3.3]],
  [V7_START, GOAL], [V7_START, JAMB],
  [V7_START, [-2.6, 0, 1.0]], [[-2.6, 0, 1.0], JAMB],
  [V7_START, [-2.5, 0, 1.3]], [[-2.5, 0, 1.3], JAMB],
  [V7_START, [-2.4, 0, 1.5]], [[-2.4, 0, 1.5], JAMB],
  [V7_START, [-2.0, 0, 1.4]], [[-2.0, 0, 1.4], JAMB],
  // Halves of the straight line (-2, 1.4) -> JAMB, to test whether it is connected.
  [[-2.0, 0, 1.4], [-1.35, 0, -0.75]], [[-1.35, 0, -0.75], JAMB], [V7_START, [-1.35, 0, -0.75]],
  // Straight crossings of the z = 1.4 tile-row border away from the (-1.6, 1.4) corner.
  [[-2.0, 0, 1.8], [-2.0, 0, 0.8]], [[-2.2, 0, 1.8], [-2.2, 0, 0.8]], [[-1.0, 0, 1.8], [-1.0, 0, 0.8]],
  [[-2.0, 0, 1.0], [-1.35, 0, -0.75]], [[-2.4, 0, 1.8], [-1.8, 0, 0.6]],
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
