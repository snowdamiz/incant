// Walkability map for handoff 0026 (no rendering, no edit). Tests the room's 0.2 m grid of
// floor points at the quantized surface height (y = 0.05) with a 6 cm snap
// distance: a point counts as walkable when a 1 cm query starting there returns
// a path. Four queries per tick
// (64 of 256 native units each). Logs one row of results per grid row; a query
// that throws is logged with its point and marked '!'.
import type { Json, ScriptApi } from '../../../sdk/ts/src/index';

const XS = Array.from({ length: 80 }, (_, i) => Math.round((-7.9 + 0.2 * i) * 100) / 100); // -7.9 .. 7.9
const ZS = Array.from({ length: 50 }, (_, i) => Math.round((-4.9 + 0.2 * i) * 100) / 100); // -4.9 .. 4.9

// PROBE_Y/PROBE_SNAP: 0.05/0.06 tests the flat quantized floor tightly; 0.12/0.1
// also accepts surfaces raised by height smearing (0.02-0.22 m) with ~7 cm blur.
const PROBE_Y = 0.05;
const PROBE_SNAP = 0.06;

type State = { tick: number; next: number; row: string };

export default defineBehavior<State & Record<string, Json>>({
  initialState: { tick: 0, next: 0, row: '' },
  update(api: ScriptApi, _dt: number, state) {
    state.tick += 1;
    const nav = api.query('NavigationMesh')[0]!;
    for (let k = 0; k < 4 && state.next < XS.length * ZS.length; k += 1, state.next += 1) {
      const x = XS[state.next % XS.length]!;
      const z = ZS[Math.floor(state.next / XS.length)]!;
      let mark = '.';
      try {
        const path = api.findPath({ scene_id: nav.scene_id, mesh_entity: nav.id,
          path: { start: [x, PROBE_Y, z], end: [x + 0.01, PROBE_Y, z], snap_distance: PROBE_SNAP, max_visited: 1000 } });
        if (path !== null) mark = '#';
      } catch (error) {
        // A thrown query is a finding in itself: record the point and the message ('!').
        mark = '!';
        api.log(JSON.stringify({ event: 'query_error', x, z, message: String(error) }), 'error');
      }
      state.row += mark;
      if (state.row.length === XS.length) {
        api.log(JSON.stringify({ event: 'walkable', z, xs: [XS[0], XS[XS.length - 1]], row: state.row }));
        state.row = '';
      }
    }
  },
});
