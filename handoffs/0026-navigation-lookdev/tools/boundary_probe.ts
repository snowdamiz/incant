// Dense short-query probe for handoff 0026 (no rendering, no edit). Around each
// former failure site, every point of an 11 x 11 grid (1 cm spacing) at three
// heights issues four 1 cm queries (+x, -x, +z, -z). Four queries per tick
// (64 of 256 native units each). Every query is wrapped so a thrown error is
// counted and logged with its inputs instead of ending the behavior.
import type { Json, ScriptApi } from '../../../sdk/ts/src/index';

type Vec3 = [number, number, number];
const SITES: [number, number][] = [[4.7, 1.7], [4.3, 2.7]];
const HEIGHTS = [0, 0.05, 0.12];
const DIRS: [number, number][] = [[0.01, 0], [-0.01, 0], [0, 0.01], [0, -0.01]];
const OFFSETS = Array.from({ length: 11 }, (_, i) => Math.round((i - 5) * 0.01 * 1000) / 1000);
const TOTAL = SITES.length * HEIGHTS.length * OFFSETS.length * OFFSETS.length * DIRS.length;

type State = { next: number; ok: number; nulls: number; errors: number };

function query(index: number): [Vec3, Vec3] {
  let i = index;
  const d = DIRS[i % DIRS.length]!; i = Math.floor(i / DIRS.length);
  const oz = OFFSETS[i % OFFSETS.length]!; i = Math.floor(i / OFFSETS.length);
  const ox = OFFSETS[i % OFFSETS.length]!; i = Math.floor(i / OFFSETS.length);
  const y = HEIGHTS[i % HEIGHTS.length]!; i = Math.floor(i / HEIGHTS.length);
  const [sx, sz] = SITES[i]!;
  const start: Vec3 = [Math.round((sx + ox) * 1000) / 1000, y, Math.round((sz + oz) * 1000) / 1000];
  return [start, [Math.round((start[0] + d[0]) * 1000) / 1000, y, Math.round((start[2] + d[1]) * 1000) / 1000]];
}

export default defineBehavior<State & Record<string, Json>>({
  initialState: { next: 0, ok: 0, nulls: 0, errors: 0 },
  update(api: ScriptApi, _dt: number, state) {
    const nav = api.query('NavigationMesh')[0]!;
    for (let k = 0; k < 4 && state.next < TOTAL; k += 1, state.next += 1) {
      const [start, end] = query(state.next);
      try {
        const path = api.findPath({ scene_id: nav.scene_id, mesh_entity: nav.id,
          path: { start, end, snap_distance: 1, max_visited: 1000 } });
        if (path) state.ok += 1; else state.nulls += 1;
      } catch (error) {
        state.errors += 1;
        api.log(JSON.stringify({ event: 'boundary_error', start, end, message: String(error) }), 'error');
      }
    }
    if (state.next === TOTAL) {
      api.log(JSON.stringify({ event: 'boundary_summary', total: TOTAL, ok: state.ok, nulls: state.nulls, errors: state.errors }));
      state.next += 1;
    }
  },
});
