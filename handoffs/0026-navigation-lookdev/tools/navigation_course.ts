// Navigation look-dev behavior for handoff 0026.
//
// One kinematic capsule follows the polyline returned by api.findPath. It moves
// at constant speed along the returned points and resolves collisions through the
// read-only api.computeCharacterMotion sweep; the result is applied as an ordinary
// Velocity command. At EDIT_TICK the behavior moves the static "Barrier" source
// into doorway A with a Transform command; on the next tick it replans from its
// current position. Every tick it also logs one compact JSON diagnostic row.
//
// Path markers are ordinary pre-authored mesh entities ("Route ..." names) that
// the behavior moves onto the returned points. They are rendered scene geometry
// placed from query results, NOT a native navigation debug-draw feature.
//
// Compile with: node tools/build_script.mjs navigation_course.ts navigation_course.js
import type { CharacterQuery, Entity, Json, NavigationPath, ScriptApi } from '../../../sdk/ts/src/index';

type Vec3 = [number, number, number];

const SPEED = 2.0; // m/s along the returned polyline
const GRAVITY = 9.81; // m/s², behavior-owned
const EDIT_TICK = 100; // barrier moves into doorway A at the end of this tick
const START: Vec3 = [-6.5, 0, 1.2];
const GOAL: Vec3 = [6.6, 0, 1.2];
const POCKET: Vec3 = [6.5, 0, 4.2]; // behind a 0.5 m slit: must be null
const BARRIER_CLOSED = { translation: [0, 0.5, 1.2] as Vec3, rotation: [0, 0, 0, 1] };
const PARKED: Vec3 = [0, -30, 0]; // unused markers sit far below the floor
const MARK_LIFT = 0.012; // markers float 12 mm above the returned surface height

const OPTIONS: NonNullable<CharacterQuery['options']> = {
  offset: 0.02,
  slide: true,
  max_slope_climb_angle: Math.PI / 4,
  min_slope_slide_angle: Math.PI / 4,
  snap_to_ground: 0.3,
  autostep: { max_height: 0.25, min_width: 0.15, include_dynamic_bodies: false },
};

type Ids = {
  scene: string; nav: string; agent: string; barrier: string;
  activeDots: string[]; activeLinks: string[]; oldDots: string[]; oldLinks: string[];
};
type State = {
  tick: number; ids: Ids | null; route: number[][]; index: number; vy: number;
  routes: number; arrived: number; generation: number;
};

const r3 = (v: number) => Math.round(v * 1000) / 1000;

function translation(entity: Entity): Vec3 {
  return (entity.components['Transform'] as { translation: Vec3 }).translation;
}

function byName(entities: readonly Entity[], prefix: string): string[] {
  return entities
    .filter((e) => e.name.startsWith(prefix))
    .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
    .map((e) => e.id);
}

function discover(api: ScriptApi): Ids {
  const nav = api.query('NavigationMesh')[0];
  const agent = api.query('RigidBody').find((e) => e.name === 'Agent');
  const renderers = api.query('MeshRenderer');
  const barrier = api.query('Collider').find((e) => e.name === 'Barrier');
  if (!nav || !agent || !barrier) throw Error('scene is missing Navigation, Agent or Barrier');
  return {
    scene: nav.scene_id, nav: nav.id, agent: agent.id, barrier: barrier.id,
    activeDots: byName(renderers, 'Route active dot '), activeLinks: byName(renderers, 'Route active link '),
    oldDots: byName(renderers, 'Route old dot '), oldLinks: byName(renderers, 'Route old link '),
  };
}

function yawPitch(dx: number, dy: number, dz: number): number[] {
  // Rotate local +X onto (dx, dy, dz): yaw about +Y, then pitch about local +Z.
  const yaw = Math.atan2(-dz, dx);
  const pitch = Math.atan2(dy, Math.hypot(dx, dz));
  const [sy, cy, sp, cp] = [Math.sin(yaw / 2), Math.cos(yaw / 2), Math.sin(pitch / 2), Math.cos(pitch / 2)];
  // q = qYaw * qPitch, qYaw = (0, sy, 0, cy), qPitch = (0, 0, sp, cp)
  return [sy * sp, sy * cp, cy * sp, cy * cp];
}

function place(api: ScriptApi, ids: Ids, entity: string, at: Vec3, rotation: number[] = [0, 0, 0, 1], scale: Vec3 = [1, 1, 1]) {
  api.command({
    op: 'set_component', scene_id: ids.scene, entity_id: entity, component: 'Transform',
    value: { translation: at.map(r3), rotation, scale },
  });
}

function drawRoute(api: ScriptApi, ids: Ids, points: number[][], dots: string[], links: string[], pool: string, tick: number) {
  // Coverage record: every returned point needs a dot and every segment a link.
  const segments = Math.max(points.length - 1, 0);
  const covered = points.length <= dots.length && segments <= links.length;
  api.log(JSON.stringify({ event: 'markers', t: tick, pool, points: points.length, dots: dots.length,
    segments, links: links.length, covered }), covered ? 'info' : 'error');
  dots.forEach((id, i) => {
    const p = points[i];
    place(api, ids, id, p ? [p[0]!, p[1]! + MARK_LIFT, p[2]!] : PARKED);
  });
  links.forEach((id, i) => {
    const a = points[i];
    const b = points[i + 1];
    if (!a || !b) return place(api, ids, id, PARKED);
    const [dx, dy, dz] = [b[0]! - a[0]!, b[1]! - a[1]!, b[2]! - a[2]!];
    const length = Math.hypot(dx, dy, dz);
    const mid: Vec3 = [(a[0]! + b[0]!) / 2, (a[1]! + b[1]!) / 2 + MARK_LIFT, (a[2]! + b[2]!) / 2];
    place(api, ids, id, mid, yawPitch(dx, dy, dz), [Math.max(length, 0.001), 1, 1]);
  });
}

function describe(path: NavigationPath | null): Json {
  if (!path) return null;
  return { points: path.points.map((p) => p.map(r3)), corridor: path.corridor.length, visited: path.visited, generation: path.generation };
}

function query(api: ScriptApi, ids: Ids, start: Vec3, end: Vec3): NavigationPath | null {
  return api.findPath({ scene_id: ids.scene, mesh_entity: ids.nav, path: { start, end, snap_distance: 1, max_visited: 4000 } });
}

export default defineBehavior<State & Record<string, Json>>({
  initialState: { tick: 0, ids: null, route: [], index: 1, vy: 0, routes: 0, arrived: 0, generation: -1 },
  update(api: ScriptApi, dt: number, state) {
    state.tick += 1;
    const ids = state.ids ?? discover(api);
    state.ids = ids;
    const agent = api.query('RigidBody').find((e) => e.id === ids.agent)!;
    const p = translation(agent);

    if (state.tick === 1 || state.tick === EDIT_TICK + 1) {
      const from: Vec3 = state.tick === 1 ? START : [p[0], 0, p[2]];
      const route = query(api, ids, from, GOAL);
      const pocket = query(api, ids, START, POCKET);
      api.log(JSON.stringify({ event: 'route', t: state.tick, from: from.map(r3), to: GOAL, path: describe(route) }));
      api.log(JSON.stringify({ event: 'pocket', t: state.tick, from: START, to: POCKET, path: describe(pocket) }),
        pocket ? 'error' : 'info');
      if (!route) throw Error(`expected a route at tick ${state.tick}`);
      if (route.generation === state.generation) api.log(`generation unchanged at tick ${state.tick}`, 'warn');
      if (state.routes > 0) drawRoute(api, ids, state.route, ids.oldDots, ids.oldLinks, 'old', state.tick);
      drawRoute(api, ids, route.points, ids.activeDots, ids.activeLinks, 'active', state.tick);
      state.route = route.points;
      state.index = 1;
      state.routes += 1;
      state.generation = route.generation;
    }

    // Constant-speed walk along the returned polyline (XZ). Corners are consumed
    // within the tick, so the desired displacement never cuts a corner by more
    // than one tick of travel and never loses speed at a waypoint.
    let budget = SPEED * dt;
    let [x, z] = [p[0], p[2]];
    while (budget > 0 && state.index < state.route.length) {
      const target = state.route[state.index]!;
      const [dx, dz] = [target[0]! - x, target[2]! - z];
      const distance = Math.hypot(dx, dz);
      if (distance <= budget) {
        [x, z] = [target[0]!, target[2]!];
        budget -= distance;
        state.index += 1;
      } else {
        x += (dx / distance) * budget;
        z += (dz / distance) * budget;
        budget = 0;
      }
    }
    if (state.index >= state.route.length && !state.arrived) {
      state.arrived = state.tick;
      api.log(JSON.stringify({ event: 'arrived', t: state.tick, p: p.map(r3) }));
    }
    const desired: Vec3 = [x - p[0], 0, z - p[2]];
    state.vy -= GRAVITY * dt;
    const movement = api.computeCharacterMotion({
      scene_id: ids.scene, entity_id: ids.agent,
      translation: [desired[0], state.vy * dt, desired[2]], options: OPTIONS,
    });
    if (movement.grounded && state.vy < 0) state.vy = 0;
    api.command({
      op: 'set_component', scene_id: ids.scene, entity_id: ids.agent, component: 'Velocity',
      value: { linear: movement.translation.map((v) => v / dt) as Vec3 },
    });
    if (state.tick === EDIT_TICK) {
      api.command({
        op: 'set_component', scene_id: ids.scene, entity_id: ids.barrier, component: 'Transform',
        value: { ...BARRIER_CLOSED, scale: [1, 1, 1] },
      });
      api.log(JSON.stringify({ event: 'edit', t: state.tick, barrier: BARRIER_CLOSED }));
    }
    api.log(JSON.stringify({
      t: state.tick, p: p.map(r3), want: [r3(desired[0]), r3(desired[2])], move: movement.translation.map(r3),
      g: movement.grounded, slide: movement.sliding_down_slope, hit: movement.collisions, i: state.index, r: state.routes,
    }), 'debug');
  },
});
