// Steering look-dev behavior for handoff 0027.
//
// One hundred walkers (four 5x5 blocks of 25) swap sides of a plaza through a
// central octagonal plinth. Every tick the behavior:
//   1. reads each walker's Transform/Velocity (start-of-tick snapshot),
//   2. computes a goal-seeking preferred velocity (behavior-owned; the only
//      "routing" is a keep-the-plinth-on-the-left tangent toward the goal),
//   3. calls api.steerAgents ONCE for all 100 walkers with the plinth, two
//      gantry posts and the overhead gantry beam as height-filtered obstacles,
//   4. applies the returned proposals as ordinary Velocity commands.
// Walkers have no RigidBody or Collider: no physics collision is claimed. What
// the frames show is the steering proposals integrated by the engine.
//
// Trail markers are ordinary pre-authored mesh entities ("Trail ...") that the
// behavior moves onto tracked walkers' positions every TRAIL_EVERY ticks. They
// are rendered scene geometry, NOT native debug draw.
//
// Every tick logs one diagnostic row (info) and one position row (debug, mm).
//
// Compile with: node tools/build_script.mjs steering_plaza.ts steering_plaza.js
import type { Entity, Json, ScriptApi, SteeringAgent, SteeringObstacle } from '../../../sdk/ts/src/index';

type V2 = [number, number];

const RADIUS = 0.25; // physical and visual radius (m)
const HEIGHT = 1.8; // physical and visual height (m)
const MARGIN = 0.05;
const MAX_SPEED = 1.4; // m/s
const TIME_HORIZON = 2;
const OBSTACLE_TIME_HORIZON = 1;
const ARRIVE = 0.05; // m
const STALL_SPEED = 0.1; // m/s, applied proposal
const TRAIL_EVERY = 20; // ticks
const TRAIL_LIFT = 0.010; // above the 5 mm zone pads

// Plinth: regular octagon, circumradius 1.0 m, flat edges facing the axes.
const PLINTH_R = 1.0;
const PLINTH: V2[] = Array.from({ length: 8 }, (_, i) => {
  const a = Math.PI / 8 + (i * Math.PI) / 4;
  return [PLINTH_R * Math.cos(a), PLINTH_R * Math.sin(a)] as V2;
});
const ROUTE_R = PLINTH_R + RADIUS + MARGIN + 0.25; // tangent circle used for preferred velocity only
const POST_HALF = 0.15;
const GANTRY_X = -5.5;
const POST_Z = 4.5;
const BEAM = { min_y: 2.1, max_y: 2.35, half_x: 0.15, half_z: 4.65 };

// Travel vector per group, from the start block to the opposite block.
const TRAVEL: Record<string, V2> = { W: [20, 0], E: [-20, 0], N: [0, 20], S: [0, -20] };

function square(cx: number, cz: number, hx: number, hz: number): V2[] {
  // Counterclockwise in (x, z) (positive signed area): solid interior.
  return [[cx - hx, cz - hz], [cx + hx, cz - hz], [cx + hx, cz + hz], [cx - hx, cz + hz]];
}

const OBSTACLES: SteeringObstacle[] = [
  { id: '01JA2STR0000000000000000P1' /* plinth */, closed: true, min_y: 0, max_y: 0.9, vertices: PLINTH },
  { id: '01JA2STR0000000000000000P2' /* north post */, closed: true, min_y: 0, max_y: BEAM.max_y, vertices: square(GANTRY_X, -POST_Z, POST_HALF, POST_HALF) },
  { id: '01JA2STR0000000000000000P3' /* south post */, closed: true, min_y: 0, max_y: BEAM.max_y, vertices: square(GANTRY_X, POST_Z, POST_HALF, POST_HALF) },
  // Overhead beam: supplied on purpose; its 2.1-2.35 m interval does not overlap a
  // 0-1.8 m walker, so the solver must ignore it (height filter).
  { id: '01JA2STR0000000000000000P4' /* overhead beam */, closed: true, min_y: BEAM.min_y, max_y: BEAM.max_y, vertices: square(GANTRY_X, 0, BEAM.half_x, BEAM.half_z) },
];

type Walker = { id: string; name: string; group: string; goal: V2; tracked: number; stall: number };
type State = {
  tick: number; scene: string; walkers: Walker[]; trails: string[][]; drops: number[];
  minSeparation: number; minPlinthGap: number; minPostGap: number; firstAllArrived: number;
  overlapTicks: number; maxStall: number; maxNeighbors: number; maxApplied: number;
};

const r3 = (v: number) => Math.round(v * 1000) / 1000;

function cross(a: V2, b: V2): number {
  return a[0] * b[1] - a[1] * b[0];
}

function pointSegment(p: V2, a: V2, b: V2): number {
  const ab: V2 = [b[0] - a[0], b[1] - a[1]];
  const len2 = ab[0] * ab[0] + ab[1] * ab[1];
  const t = len2 === 0 ? 0 : Math.max(0, Math.min(1, ((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]) / len2));
  return Math.hypot(p[0] - a[0] - t * ab[0], p[1] - a[1] - t * ab[1]);
}

/** Signed distance from p to a convex CCW polygon (negative inside). */
function polygonDistance(p: V2, poly: V2[]): number {
  let inside = true;
  let best = Infinity;
  for (let i = 0; i < poly.length; i++) {
    const a = poly[i]!;
    const b = poly[(i + 1) % poly.length]!;
    if (cross([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]) < 0) inside = false;
    best = Math.min(best, pointSegment(p, a, b));
  }
  return inside ? -best : best;
}

function preferred(p: V2, goal: V2, dt: number): V2 {
  const to: V2 = [goal[0] - p[0], goal[1] - p[1]];
  const distance = Math.hypot(to[0], to[1]);
  if (distance < 1e-6) return [0, 0];
  const speed = Math.min(MAX_SPEED, distance / dt);
  let dir: V2 = [to[0] / distance, to[1] / distance];
  const c: V2 = [-p[0], -p[1]]; // plinth centre (origin) relative to p
  const d = Math.hypot(c[0], c[1]);
  // Only route around the plinth while the goal lies beyond it: once the goal
  // direction points away from the plinth centre, walk straight to the goal.
  const ahead = to[0] * c[0] + to[1] * c[1] > 0;
  if (ahead && pointSegment([0, 0], p, goal) < ROUTE_R && d > 1e-6) {
    if (d > ROUTE_R) {
      // Tangent to the routing circle that keeps the plinth on the left.
      const beta = Math.asin(ROUTE_R / d);
      const base = Math.atan2(c[1], c[0]);
      const candidates: V2[] = [base + beta, base - beta].map((a) => [Math.cos(a), Math.sin(a)] as V2);
      dir = candidates.find((u) => cross(u, c) > 0) ?? candidates[0]!;
    } else {
      // Inside the routing circle: walk around it in the same sense.
      const t: V2 = [c[1] / d, -c[0] / d];
      dir = cross(t, c) > 0 ? t : [-t[0], -t[1]];
    }
  }
  return [dir[0] * speed, dir[1] * speed];
}

function translation(e: Entity): number[] {
  return (e.components['Transform'] as { translation: number[] }).translation;
}

function discover(api: ScriptApi): Pick<State, 'scene' | 'walkers' | 'trails' | 'drops'> {
  const movers = api.query('Velocity').slice().sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
  const renderers = api.query('MeshRenderer');
  if (movers.length !== 100) throw Error(`expected 100 walkers, found ${movers.length}`);
  const trails: string[][] = [];
  const walkers = movers.map((e) => {
    // Names: "Walker <group> <row>-<col>[ tracked]"
    const [, group = '', cell = ''] = e.name.split(' ');
    const travel = TRAVEL[group];
    if (!travel) throw Error(`unknown walker group in ${e.name}`);
    const p = translation(e);
    let tracked = -1;
    if (e.name.endsWith(' tracked')) {
      tracked = trails.length;
      const prefix = `Trail ${group} ${cell} `;
      trails.push(renderers.filter((r) => r.name.startsWith(prefix))
        .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0)).map((r) => r.id));
    }
    return { id: e.id, name: e.name, group, goal: [p[0]! + travel[0], p[2]! + travel[1]] as V2, tracked, stall: 0 };
  });
  return { scene: movers[0]!.scene_id, walkers, trails, drops: trails.map(() => 0) };
}

export default defineBehavior<State & Record<string, Json>>({
  initialState: {
    tick: 0, scene: '', walkers: [], trails: [], drops: [], minSeparation: 1000, minPlinthGap: 1000,
    minPostGap: 1000, firstAllArrived: 0, overlapTicks: 0, maxStall: 0, maxNeighbors: 0, maxApplied: 0,
  },
  update(api: ScriptApi, dt: number, s) {
    s.tick += 1;
    if (s.walkers.length === 0) Object.assign(s, discover(api));
    const byId = new Map(api.query('Velocity').map((e) => [e.id, e]));
    const positions: V2[] = [];
    const agents: SteeringAgent[] = s.walkers.map((w) => {
      const e = byId.get(w.id)!;
      const p = translation(e);
      const v = (e.components['Velocity'] as { linear: number[] }).linear;
      positions.push([p[0]!, p[2]!]);
      return {
        id: w.id, position: [p[0]!, p[1]!, p[2]!], velocity: [v[0]!, v[2]!],
        preferred_velocity: preferred([p[0]!, p[2]!], w.goal, dt),
        radius: RADIUS, height: HEIGHT, max_speed: MAX_SPEED, responsibility: 1,
      };
    });

    // Diagnostics on the start-of-tick snapshot.
    let separation = Infinity;
    let pair: number[] = [];
    let overlaps = 0;
    let withinMargin = 0;
    for (let i = 0; i < positions.length; i++) {
      for (let j = i + 1; j < positions.length; j++) {
        const d = Math.hypot(positions[i]![0] - positions[j]![0], positions[i]![1] - positions[j]![1]);
        if (d < separation) { separation = d; pair = [i, j]; }
        if (d < 2 * RADIUS) overlaps++;
        if (d < 2 * (RADIUS + MARGIN)) withinMargin++;
      }
    }
    const plinthGap = Math.min(...positions.map((p) => polygonDistance(p, OBSTACLES[0]!.vertices) - RADIUS));
    const postGap = Math.min(...positions.flatMap((p) => [1, 2].map((k) => polygonDistance(p, OBSTACLES[k]!.vertices) - RADIUS)));
    const remaining = s.walkers.map((w, i) => Math.hypot(w.goal[0] - positions[i]![0], w.goal[1] - positions[i]![1]));
    const arrived = remaining.filter((d) => d < ARRIVE).length;

    const proposals = api.steerAgents({
      agents, obstacles: OBSTACLES, time_horizon: TIME_HORIZON,
      obstacle_time_horizon: OBSTACLE_TIME_HORIZON, margin: MARGIN,
    });
    const proposal = new Map(proposals.map((v) => [v.id, v]));
    let applied = 0;
    let sum = 0;
    let stalled = 0;
    let longest = 0;
    let neighbors = 0;
    s.walkers.forEach((w, i) => {
      const v = proposal.get(w.id)!;
      const speed = Math.hypot(v.velocity[0], v.velocity[1]);
      applied = Math.max(applied, speed);
      sum += speed;
      neighbors = Math.max(neighbors, v.neighbors);
      w.stall = remaining[i]! >= ARRIVE && speed < STALL_SPEED ? w.stall + 1 : 0;
      if (w.stall > 0) stalled++;
      longest = Math.max(longest, w.stall);
      api.command({ op: 'set_component', scene_id: s.scene, entity_id: w.id, component: 'Velocity',
        value: { linear: [v.velocity[0], 0, v.velocity[1]] } });
      if (w.tracked >= 0 && (s.tick - 1) % TRAIL_EVERY === 0 && remaining[i]! >= ARRIVE) {
        const pool = s.trails[w.tracked]!;
        const n = s.drops[w.tracked]!;
        if (n < pool.length) {
          api.command({ op: 'set_component', scene_id: s.scene, entity_id: pool[n]!, component: 'Transform',
            value: { translation: [r3(positions[i]![0]), TRAIL_LIFT, r3(positions[i]![1])], rotation: [0, 0, 0, 1], scale: [1, 1, 1] } });
          s.drops[w.tracked] = n + 1;
        } else if (n === pool.length) {
          api.log(JSON.stringify({ event: 'trail-pool-exhausted', t: s.tick, walker: w.name }), 'warn');
          s.drops[w.tracked] = n + 1;
        }
      }
    });
    s.minSeparation = Math.min(s.minSeparation, separation);
    s.minPlinthGap = Math.min(s.minPlinthGap, plinthGap);
    s.minPostGap = Math.min(s.minPostGap, postGap);
    s.maxStall = Math.max(s.maxStall, longest);
    s.maxNeighbors = Math.max(s.maxNeighbors, neighbors);
    s.maxApplied = Math.max(s.maxApplied, applied);
    if (overlaps > 0) s.overlapTicks++;
    if (arrived === 100 && s.firstAllArrived === 0) {
      s.firstAllArrived = s.tick;
      api.log(JSON.stringify({ event: 'all-arrived', t: s.tick }));
    }
    if (overlaps > 0) {
      api.log(JSON.stringify({ event: 'overlap', t: s.tick, pairs: overlaps, sep: r3(separation),
        a: s.walkers[pair[0]!]!.name, b: s.walkers[pair[1]!]!.name }), 'warn');
    }
    api.log(JSON.stringify({
      t: s.tick, sep: r3(separation), pair: pair.map((k) => s.walkers[k]!.name), overlaps, in_margin: withinMargin,
      plinth_gap: r3(plinthGap), post_gap: r3(postGap), v_max: r3(applied), v_mean: r3(sum / 100),
      arrived, stalled, longest_stall: longest, neighbors, remaining_max: r3(Math.max(...remaining)),
    }), 'info');
    api.log(JSON.stringify({ t: s.tick, p: positions.flatMap((p) => [Math.round(p[0] * 1000), Math.round(p[1] * 1000)]) }), 'debug');
  },
});
