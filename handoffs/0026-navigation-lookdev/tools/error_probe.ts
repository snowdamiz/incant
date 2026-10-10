// Exception repro for handoff 0026 (no rendering, no edit). One findPath per tick
// around the open-floor point (4.7, 1.7) where walkable_probe.ts saw a thrown
// "funnel did not cross its navigation corridor". Logs result or error per query.
import type { Json, ScriptApi } from '../../../sdk/ts/src/index';

type Vec3 = [number, number, number];
const QUERIES: [Vec3, Vec3][] = [
  [[4.7, 0.05, 1.7], [4.71, 0.05, 1.7]], [[4.7, 0, 1.7], [4.71, 0, 1.7]], [[4.7, 0, 1.7], [4.7, 0, 1.71]],
  [[4.7, 0, 1.7], [4.8, 0, 1.7]], [[4.6, 0, 1.7], [4.8, 0, 1.7]], [[4.7, 0, 1.7], [6.6, 0, 1.2]],
  [[3.0, 0, 1.7], [6.0, 0, 1.7]], [[4.71, 0, 1.7], [4.7, 0, 1.7]], [[4.7, 0, 1.6], [4.71, 0, 1.6]],
  [[4.7, 0, 1.8], [4.71, 0, 1.8]], [[4.65, 0, 1.7], [4.66, 0, 1.7]], [[4.75, 0, 1.7], [4.76, 0, 1.7]],
];

export default defineBehavior<{ tick: number } & Record<string, Json>>({
  initialState: { tick: 0 },
  update(api: ScriptApi, _dt: number, state) {
    state.tick += 1;
    const query = QUERIES[state.tick - 1];
    if (!query) return;
    const nav = api.query('NavigationMesh')[0]!;
    try {
      const path = api.findPath({ scene_id: nav.scene_id, mesh_entity: nav.id,
        path: { start: query[0], end: query[1], snap_distance: 1, max_visited: 4000 } });
      api.log(JSON.stringify({ event: 'probe', start: query[0], end: query[1], points: path ? path.points : null }));
    } catch (error) {
      api.log(JSON.stringify({ event: 'probe_error', start: query[0], end: query[1], message: String(error) }), 'error');
    }
  },
});
