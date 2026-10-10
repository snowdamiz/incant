// Doorway-width diagnostic for handoff 0026 (no rendering). Each scene holds one
// NavigationMesh; on tick 1 the behavior asks for a route straight through the
// doorway and logs the result. Compile with tools/build_script.mjs.
import type { Json, ScriptApi } from '../../../sdk/ts/src/index';

export default defineBehavior<{ done: boolean } & Record<string, Json>>({
  initialState: { done: false },
  update(api: ScriptApi, _dt: number, state) {
    if (state.done) return;
    state.done = true;
    const nav = api.query('NavigationMesh')[0];
    if (!nav) throw Error('missing NavigationMesh');
    const path = api.findPath({
      scene_id: nav.scene_id, mesh_entity: nav.id,
      path: { start: [-3, 0, 0], end: [3, 0, 0], snap_distance: 1, max_visited: 4000 },
    });
    api.log(JSON.stringify({ event: 'doorway', path: path ? path.points : null }));
  },
});
