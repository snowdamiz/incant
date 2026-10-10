// Weighted grid navigation look-dev behavior for handoff 0029.
//
// One actor crosses a 16 x 10 NavigationGrid using actual api.findGridPath routes.
// Every tick the behavior queries from its anchor cell (the cell it stands on, or
// the cell its current step is entering) to the goal. A new step always goes to
// the returned route's cells[1]; no path is handwritten and no diagonal is
// inferred. Between adjacent cell centres the actor interpolates linearly with
// ordinary Transform commands. There is no RigidBody or Collider: no physics.
//
// Course:
//   approach  Cheap route through door A, bending around the mud marsh.
//   close A   On reaching its 2nd cell the actor closes door A with an ordinary
//             set_component NavigationGrid command. The same tick still plans on
//             the start-of-tick grid; the next tick the route changes to gate B
//             through mud (more expensive).
//   close B   Entering a cell two cells (Chebyshev) from gate B while routed
//             through it, the actor closes gate B. Next tick: null route. The
//             actor finishes its current adjacent step, stops and waits. From
//             the first null tick the amber ring marks the stop cell and the
//             route markers are cleared.
//   reopen    After WAIT_TICKS stationary unreachable ticks it reopens door A.
//             Next tick a route exists again; the actor turns and arrives.
// Doors are never closed on the anchor or the cell being left (guarded).
//
// Independent probes (ticks 2..5): plus-shaped corner gauge (null pocket and
// orthogonal detour), declared metric checks, doorway corner, error cases.
//
// Logs: one debug row per tick and one info row per event, full double precision.
import type { Entity, GridPath, Json, NavigationGrid, ScriptApi } from '../../../sdk/ts/src/index';

type Cell = [number, number];

const W = 16, H = 10;
const START: Cell = [2, 4];
const GOAL: Cell = [13, 5];
const DOOR_A: Cell = [8, 2];
const GATE_B: Cell = [8, 7];
const GAUGE: Cell = [2, 8];
const SPEED = 3; // cells per second
const TURN_RATE = 3 * Math.PI; // rad/s
const TURN_IN_PLACE = (100 * Math.PI) / 180; // larger heading changes turn before moving
const WAIT_TICKS = 96; // stationary unreachable ticks before door A reopens
const MAX_EXPANSIONS = 4096;
const WALL_TOP = 0.55; // metres; matches grid_course.py WALL_HEIGHT
const PARK = [0, -40, 0];

type State = {
  tick: number;
  scene: string;
  grid: string;
  actor: string;
  phase: string; // boot | move | wait | arrived
  from: number[];
  to: number[];
  s: number; // cells travelled along the current step
  yaw: number; // radians about +Y; 0 faces +X (east)
  turning: boolean; // turning in place before a large heading change
  stopping: boolean; // route is null; finishing the current step, then waiting
  steps: number;
  doorA: boolean; // true = open
  gateB: boolean;
  drawn: string; // signature of the route currently drawn
  lastRoute: string; // signature of the last logged route
  waitTicks: number;
  nullTicks: number;
  trail: number;
  events: number;
  queries: number;
  probes: Record<string, Json>;
};

const ids: { pools: Record<string, string[]>; named: Record<string, string> } = { pools: {}, named: {} };

function resolveIds(api: ScriptApi): void {
  if (ids.pools.route_dot) return; // module globals are rebuilt after reopen; names are stable
  const names = new Map<string, Entity>();
  for (const e of api.query('Transform')) names.set(e.name, e);
  for (const key of ['route_dot', 'route_bar', 'trail', 'gauge_dot', 'gauge_bar']) {
    ids.pools[key] = [...names.keys()].filter((n) => n.startsWith(`Pool ${key} `)).sort().map((n) => names.get(n)!.id);
  }
  for (const n of ['Actor', 'Door A slab', 'Gate B slab', 'Wait ring', 'Gauge null start', 'Gauge null end',
    'Gauge detour start', 'Gauge detour end', 'Gauge null cross']) ids.named[n] = names.get(n)!.id;
}

function world(c: readonly number[], y = 0): number[] {
  return [c[0]! - 7.5, y, c[1]! - 4.5];
}

function emit(api: ScriptApi, s: State, kind: string, data: Record<string, Json>): void {
  s.events++;
  api.log(JSON.stringify({ t: s.tick, ev: kind, ...data }), 'info');
}

function place(api: ScriptApi, s: State, id: string, t: number[], yaw = 0, sx = 1): void {
  api.command({
    op: 'set_component', scene_id: s.scene, entity_id: id, component: 'Transform',
    value: { translation: [t[0]!, t[1]!, t[2]!], rotation: [0, Math.sin(yaw / 2), 0, Math.cos(yaw / 2)], scale: [sx, 1, 1] },
  });
}

function readGrid(api: ScriptApi, s: State): number[] {
  const g = api.query('NavigationGrid').find((e) => e.id === s.grid)!.components.NavigationGrid as unknown as NavigationGrid;
  if (g.dimensions[0] !== W || g.dimensions[1] !== H) throw Error(`unexpected grid ${JSON.stringify(g.dimensions)}`);
  return [...g.costs];
}

function find(api: ScriptApi, s: State, start: Cell, end: Cell, diagonal = true, maxExpansions = MAX_EXPANSIONS): GridPath | null {
  s.queries++;
  return api.findGridPath({ scene_id: s.scene, grid_entity: s.grid, path: { start, end, diagonal, max_expansions: maxExpansions } });
}

/** Independent check of a returned route against the start-of-tick costs.
 * Throws on any violation; returns the recomputed declared-metric cost. */
function verify(path: GridPath, costs: number[], start: Cell, end: Cell, diagonal: boolean): number {
  const cells = path.cells;
  const same = (a: readonly number[], b: readonly number[]) => a[0] === b[0] && a[1] === b[1];
  if (!same(cells[0]!, start) || !same(cells[cells.length - 1]!, end)) throw Error(`endpoints ${JSON.stringify(cells)}`);
  const cost = (c: readonly number[]) => costs[c[1]! * W + c[0]!]!;
  let total = 0; // start cell is not charged
  for (let i = 1; i < cells.length; i++) {
    const a = cells[i - 1]!, b = cells[i]!;
    const dx = b[0] - a[0], dy = b[1] - a[1];
    if (Math.max(Math.abs(dx), Math.abs(dy)) !== 1) throw Error(`non-adjacent ${JSON.stringify([a, b])}`);
    if (cost(b) === 0) throw Error(`blocked cell ${JSON.stringify(b)}`);
    const diag = dx !== 0 && dy !== 0;
    if (diag && (!diagonal || cost([b[0], a[1]]) === 0 || cost([a[0], b[1]]) === 0)) {
      throw Error(`illegal diagonal ${JSON.stringify([a, b])}`);
    }
    total += cost(b) * (diag ? 1414 : 1000);
  }
  if (cost(start) === 0) throw Error('blocked start');
  if (total !== path.cost) throw Error(`cost ${path.cost} != recomputed ${total}`);
  return total;
}

function signature(path: GridPath | null): string {
  return path === null ? 'null' : JSON.stringify(path.cells);
}

function drawRoute(api: ScriptApi, s: State, cells: readonly (readonly number[])[], dotPool: string, barPool: string, y: number): void {
  const dots = ids.pools[dotPool]!, bars = ids.pools[barPool]!;
  if (cells.length > dots.length) throw Error(`route of ${cells.length} cells exceeds marker pool`);
  dots.forEach((id, i) => place(api, s, id, i < cells.length ? world(cells[i]!, y) : PARK));
  bars.forEach((id, i) => {
    if (i + 1 >= cells.length) return place(api, s, id, PARK);
    const a = cells[i]!, b = cells[i + 1]!;
    const dx = b[0]! - a[0]!, dy = b[1]! - a[1]!;
    place(api, s, id, world(a, y), Math.atan2(-dy, dx), Math.hypot(dx, dy));
  });
}

function setDoor(api: ScriptApi, s: State, costs: number[], which: 'A' | 'B', open: boolean): void {
  const cell = which === 'A' ? DOOR_A : GATE_B;
  if (!open) {
    for (const c of [s.from, s.to]) {
      if (c[0] === cell[0] && c[1] === cell[1]) throw Error(`refusing to close door ${which} on the actor`);
    }
  }
  const next = [...costs];
  next[cell[1] * W + cell[0]] = open ? (which === 'A' ? 1 : 5) : 0;
  api.command({ op: 'set_component', scene_id: s.scene, entity_id: s.grid, component: 'NavigationGrid', value: { dimensions: [W, H], costs: next } });
  place(api, s, ids.named[which === 'A' ? 'Door A slab' : 'Gate B slab']!, open ? PARK : world(cell));
  if (which === 'A') s.doorA = open; else s.gateB = open;
  emit(api, s, open ? `reopen` : `close_${which.toLowerCase()}`, { cell, actor_from: s.from, actor_to: s.to, cost_written: next[cell[1] * W + cell[0]]! });
}

function probes(api: ScriptApi, s: State, costs: number[]): void {
  const record = (name: string, value: Json) => { s.probes[name] = value; emit(api, s, 'probe', { name, value }); };
  const expectPath = (name: string, a: Cell, b: Cell, expected: number, diagonal = true) => {
    const p = find(api, s, a, b, diagonal);
    if (p === null) throw Error(`${name}: unexpected null`);
    const recomputed = verify(p, costs, a, b, diagonal);
    if (p.cost !== expected) throw Error(`${name}: cost ${p.cost} != expected ${expected}`);
    record(name, { start: a, end: b, cells: p.cells as unknown as Json, cost: p.cost, recomputed, generation: p.generation });
    return p;
  };
  const expectThrow = (name: string, a: Cell, b: Cell, maxExpansions: number) => {
    try {
      const p = find(api, s, a, b, true, maxExpansions);
      throw Error(`${name}: expected an error, got ${signature(p)}`);
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      if (message.startsWith(`${name}:`)) throw e;
      record(name, { start: a, end: b, max_expansions: maxExpansions, error: message });
    }
  };
  if (s.tick === 2) {
    // Plus-shaped gauge: the pocket's four orthogonal neighbours are blocked, so
    // every diagonal out of it is illegal. A corner-cutting planner would return
    // [[2,8],[1,7]] at cost 1414.
    const sealed: Cell = [1, 7];
    const p = find(api, s, GAUGE, sealed);
    if (p !== null) throw Error(`gauge_null: expected null, got ${signature(p)}`);
    record('gauge_null', { start: GAUGE, end: sealed, result: null });
    place(api, s, ids.named['Gauge null start']!, world(GAUGE));
    place(api, s, ids.named['Gauge null end']!, world(sealed));
    // The X floats just above the wall tops, over the pinch corner the diagonal would cut.
    place(api, s, ids.named['Gauge null cross']!, world([(GAUGE[0] + sealed[0]) / 2, (GAUGE[1] + sealed[1]) / 2], WALL_TOP + 0.02));
    // Around the east arm: a corner-cutting planner would take [3,7]->[4,8]->[3,9]
    // at 2828; the legal route is four orthogonal steps at 4000.
    const d = expectPath('gauge_detour', [3, 7], [3, 9], 4000);
    drawRoute(api, s, d.cells, 'gauge_dot', 'gauge_bar', 0);
    place(api, s, ids.named['Gauge detour start']!, world([3, 7]));
    place(api, s, ids.named['Gauge detour end']!, world([3, 9]));
  } else if (s.tick === 3) {
    expectPath('metric_cardinal_into_mud', [9, 5], [9, 6], 5000);
    expectPath('metric_start_on_mud_not_charged', [9, 6], [9, 5], 1000);
    expectPath('metric_diagonal_into_floor', [3, 3], [4, 2], 1414);
  } else if (s.tick === 4) {
    // Doorway: [7,6]->[8,7] would cut the wall corner at [8,6]; legal is via [7,7].
    expectPath('doorway_corner', [7, 6], [8, 7], 10000);
    expectPath('four_way_start_goal', START, GOAL, 16000, false);
  } else if (s.tick === 5) {
    expectThrow('error_out_of_bounds', [W, 0], GOAL, MAX_EXPANSIONS);
    expectThrow('error_exhausted', START, GOAL, 1);
  }
}

function headingTo(from: readonly number[], to: readonly number[]): number {
  return Math.atan2(-(to[1]! - from[1]!), to[0]! - from[0]!);
}

function wrap(a: number): number {
  return Math.atan2(Math.sin(a), Math.cos(a));
}

export default defineBehavior<State>({
  initialState: {
    tick: 0, scene: '', grid: '', actor: '', phase: 'boot', from: [...START], to: [...START], s: 0, yaw: 0, turning: false, stopping: false,
    steps: 0, doorA: true, gateB: true, drawn: '', lastRoute: '', waitTicks: 0, nullTicks: 0, trail: 0,
    events: 0, queries: 0, probes: {},
  },
  update(api: ScriptApi, dt: number, s: State) {
    s.tick++;
    resolveIds(api);
    if (s.phase === 'boot') {
      const grid = api.query('NavigationGrid')[0]!;
      s.scene = grid.scene_id;
      s.grid = grid.id;
      s.actor = ids.named['Actor']!;
      s.phase = 'move';
      place(api, s, ids.pools.trail![s.trail++]!, world(START));
      emit(api, s, 'layout', { dt, w: W, h: H, start: START, goal: GOAL, door_a: DOOR_A, gate_b: GATE_B, gauge: GAUGE,
        speed_cells_per_s: SPEED, wait_ticks: WAIT_TICKS });
    }
    const costs = readGrid(api, s);
    probes(api, s, costs);

    let route: GridPath | null = null;
    if (s.phase !== 'arrived') {
      const anchor: Cell = [s.to[0]!, s.to[1]!];
      route = find(api, s, anchor, GOAL);
      const sig = signature(route);
      if (route !== null) verify(route, costs, anchor, GOAL, true);
      // Stop cue: as soon as the destination is unreachable the amber ring marks
      // the cell where the actor will stop (the end of its current step).
      if (route === null && s.phase === 'move' && !s.stopping) {
        s.stopping = true;
        place(api, s, ids.named['Wait ring']!, world(s.to));
        emit(api, s, 'stopping', { at: s.to, from: s.from });
      } else if (route !== null && s.stopping && s.phase === 'move') {
        s.stopping = false;
        place(api, s, ids.named['Wait ring']!, PARK);
        emit(api, s, 'stop_cancelled', { at: s.to });
      }
      if (sig !== s.lastRoute) {
        s.lastRoute = sig;
        emit(api, s, 'route', { anchor, cells: route ? route.cells as unknown as Json : null,
          cost: route ? route.cost : null, generation: route ? route.generation : null, door_a: s.doorA, gate_b: s.gateB });
      }
    }

    // Advance along adjacent cell centres; the route above starts at s.to.
    let budget = s.phase === 'arrived' ? 0 : SPEED * dt;
    let used = false;
    let turnBudget = TURN_RATE * dt;
    while (s.phase !== 'arrived') {
      const atCentre = s.from[0] === s.to[0] && s.from[1] === s.to[1];
      if (atCentre) {
        if (used) break; // one fresh route per step
        used = true;
        if (route === null) {
          if (s.phase !== 'wait') {
            s.phase = 'wait';
            s.waitTicks = 0;
            s.stopping = true;
            place(api, s, ids.named['Wait ring']!, world(s.to));
            emit(api, s, 'unreachable', { at: s.to, door_a: s.doorA, gate_b: s.gateB });
          }
          s.waitTicks++;
          s.nullTicks++;
          if (s.waitTicks === WAIT_TICKS) setDoor(api, s, costs, 'A', true);
          break;
        }
        if (route.cells.length === 1) {
          s.phase = 'arrived';
          emit(api, s, 'arrive', { at: s.to, steps: s.steps, null_ticks: s.nullTicks, queries: s.queries });
          break;
        }
        if (s.phase === 'wait') {
          s.phase = 'move';
          s.stopping = false;
          place(api, s, ids.named['Wait ring']!, PARK);
          emit(api, s, 'resume', { at: s.to, cost: route.cost, waited_ticks: s.waitTicks });
        }
        const next = route.cells[1]!;
        const diff = wrap(headingTo(s.to, next) - s.yaw);
        if (!s.turning && Math.abs(diff) > TURN_IN_PLACE) {
          s.turning = true;
          emit(api, s, 'turn_in_place', { at: s.to, toward: next, heading_change_deg: (diff * 180) / Math.PI });
        }
        if (s.turning) {
          const turn = Math.sign(diff) * Math.min(Math.abs(diff), turnBudget);
          s.yaw = wrap(s.yaw + turn);
          turnBudget -= Math.abs(turn);
          if (Math.abs(diff) > Math.abs(turn)) break; // still turning; the route is re-queried next tick
          s.turning = false;
        }
        s.to = [next[0], next[1]];
        s.s = 0;
        continue;
      }
      const length = Math.hypot(s.to[0]! - s.from[0]!, s.to[1]! - s.from[1]!);
      const remain = length - s.s;
      if (budget >= remain) {
        budget -= remain;
        s.from = [...s.to];
        s.s = 0;
        s.steps++;
        if (s.trail < ids.pools.trail!.length) place(api, s, ids.pools.trail![s.trail++]!, world(s.to));
        const cheb = Math.max(Math.abs(s.to[0]! - GATE_B[0]), Math.abs(s.to[1]! - GATE_B[1]));
        const viaB = route !== null && route.cells.some((c) => c[0] === GATE_B[0] && c[1] === GATE_B[1]);
        if (s.steps === 2 && s.doorA) setDoor(api, s, costs, 'A', false);
        else if (!s.doorA && s.gateB && viaB && cheb === 2) setDoor(api, s, costs, 'B', false);
        continue;
      }
      s.s += budget;
      budget = 0;
      break;
    }

    // Draw after moving. The line also shows the step in progress when the actor
    // is still heading to route.cells[0], so it stays attached to the actor.
    // Verification above uses the returned route alone.
    if (s.phase !== 'arrived' || s.drawn !== '[]') {
      const head = route?.cells[0];
      const onRoute = head !== undefined && head[0] === s.from[0] && head[1] === s.from[1];
      const shown = route === null || s.phase === 'arrived' ? [] : onRoute ? [...route.cells] : [s.from, ...route.cells];
      const drawn = JSON.stringify(shown);
      if (drawn !== s.drawn) {
        s.drawn = drawn;
        drawRoute(api, s, shown, 'route_dot', 'route_bar', 0);
      }
    }

    // Facing slews toward the current step; positions interpolate between centres.
    if (s.from[0] !== s.to[0] || s.from[1] !== s.to[1]) {
      const diff = wrap(headingTo(s.from, s.to) - s.yaw);
      s.yaw = wrap(s.yaw + Math.sign(diff) * Math.min(Math.abs(diff), turnBudget));
    }
    const length = Math.hypot(s.to[0]! - s.from[0]!, s.to[1]! - s.from[1]!);
    const f = length > 0 ? s.s / length : 0;
    const a = world(s.from), b = world(s.to);
    const pos = [a[0]! + (b[0]! - a[0]!) * f, 0, a[2]! + (b[2]! - a[2]!) * f];
    place(api, s, s.actor, pos, s.yaw);
    api.log(JSON.stringify({ t: s.tick, pos, yaw: s.yaw, from: s.from, to: s.to, s: s.s, phase: s.phase,
      cost: route ? route.cost : null, generation: route ? route.generation : null }), 'debug');
    return undefined;
  },
});
