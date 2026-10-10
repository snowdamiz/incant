// Off-mesh link look-dev behavior for handoff 0028.
//
// One walker crosses three separate walkable platforms using actual
// api.findPath routes. Route metadata alone decides how each segment moves:
// a segment i -> i+1 is a gap traversal only when `traversals` contains
// {from_index: i, to_index: i + 1}. Point spacing is never used to infer a link.
//
// Course (all motion is ordinary Transform commands; no RigidBody, no physics):
//   out     West -> East. Expect the directed jump-up and the bridge (forward).
//           At tick 1 also query East -> West and expect null (directed link,
//           return link disabled).
//   pause   Stand on the goal for PAUSE_TICKS.
//   unlock  Query East -> West (expect null), then enable the return link with an
//           ordinary set_component NavigationMesh command. Commands are visible
//           next tick.
//   return  Replan East -> West. Expect the bridge reversed, then the return drop.
//   lock    On arrival disable the return link. Next tick query East -> West
//           (expect null again) and West -> East (still valid, new generation).
//
// Jumps are a logical parabola between the route's actual snapped endpoints
// (points[from_index] -> points[to_index]); the engine does not execute or
// validate them. Marker dots are pre-authored mesh entities moved by commands.
// They are rendered scene geometry, NOT native debug draw.
//
// Logs: one debug row per tick with unrounded walker feet position, one info row
// per event. All numbers are emitted at full double precision.
import type { Entity, Json, NavigationPath, NavigationQuery, OffMeshLink, ScriptApi } from '../../../sdk/ts/src/index';

type V3 = [number, number, number];

const WALK_SPEED = 1.4; // m/s along walking legs
const JUMP_HSPEED = 2.4; // m/s horizontal while airborne
const JUMP_MIN = 0.5; // s
const APEX = 0.35; // m above the straight chord, plus half the height change
const TURN_RATE = 3 * Math.PI; // rad/s yaw slew limit
const PAUSE_TICKS = 30;
const WALK_DOT_EVERY = 10;
const JUMP_DOT_EVERY = 3;
const DOT_LIFT = 0.012; // floor trail dots sit this far above the walker's feet height
const PARK: V3 = [0, -40, 0];

const START: V3 = [-6.2, 0, 0];
const GOAL: V3 = [6.2, 0, 0];
const LINKS = {
  up: '01JA2NAV000000000000000001', // directed West -> Centre jump-up (north lane)
  bridge: '01JA2NAV000000000000000002', // bidirectional Centre <-> East
  ret: '01JA2NAV000000000000000003', // directed Centre -> West drop (south lane), starts disabled
} as const;
const RETURN_MARKERS = ['Return pad launch', 'Return pad landing', 'Return arrow'];
const LINK_KEY: Record<string, string> = { [LINKS.up]: 'up', [LINKS.bridge]: 'bridge', [LINKS.ret]: 'ret' };

type Jump = { i: number; link: string; reversed: boolean };
type State = {
  tick: number;
  scene: string;
  nav: string;
  walker: string;
  phase: string; // boot | out | pause | unlock | return | lock | done
  points: number[][];
  jumps: Jump[];
  seg: number;
  s: number; // metres along a walk segment, or fraction of a jump
  pos: number[];
  yaw: number;
  pause: number;
  walkDots: number;
  jumpDots: Record<string, number>;
  generations: number[];
  events: number;
  launches: number;
  landings: number;
};

const ids: { pools: Record<string, string[]>; pads: Record<string, string> } = { pools: {}, pads: {} };

function byName(api: ScriptApi): Map<string, Entity> {
  const map = new Map<string, Entity>();
  for (const e of api.query('Transform')) map.set(e.name, e);
  return map;
}

function resolveIds(api: ScriptApi): void {
  if (ids.pools.walk) return; // module globals are rebuilt after reopen; names are stable
  const names = byName(api);
  const pool = (prefix: string) =>
    [...names.keys()].filter((n) => n.startsWith(prefix)).sort().map((n) => names.get(n)!.id);
  ids.pools = {
    walk: pool('Trail walk '),
    up: pool('Trail jump up '),
    bridge: pool('Trail jump bridge '),
    ret: pool('Trail jump ret '),
  };
  for (const n of RETURN_MARKERS) {
    for (const state of [' on', ' off']) ids.pads[n + state] = names.get(n + state)!.id;
  }
}

function emit(api: ScriptApi, s: State, kind: string, data: Record<string, Json>): void {
  s.events++;
  api.log(JSON.stringify({ t: s.tick, ev: kind, ...data }), 'info');
}

function setTransform(api: ScriptApi, s: State, id: string, t: number[], yaw = 0): void {
  api.command({
    op: 'set_component', scene_id: s.scene, entity_id: id, component: 'Transform',
    value: { translation: [t[0]!, t[1]!, t[2]!], rotation: [0, Math.sin(yaw / 2), 0, Math.cos(yaw / 2)], scale: [1, 1, 1] },
  });
}

function query(api: ScriptApi, s: State, from: V3, to: V3): NavigationPath | null {
  const q: NavigationQuery = { scene_id: s.scene, mesh_entity: s.nav, path: { start: from, end: to, snap_distance: 0.3, max_visited: 4000 } };
  return api.findPath(q);
}

function describe(path: NavigationPath | null): Json {
  if (!path) return null;
  return {
    generation: path.generation, visited: path.visited, corridor: path.corridor.length,
    points: path.points as unknown as Json, traversals: path.traversals as unknown as Json,
  };
}

/** Adopt a route; reject any traversal that is not a consecutive forward pair. */
function adopt(s: State, path: NavigationPath, expected: { link: string; reversed: boolean }[]): void {
  const got = path.traversals.map((t) => ({ link: t.link_id, reversed: t.reversed }));
  if (JSON.stringify(got) !== JSON.stringify(expected)) throw Error(`unexpected traversals ${JSON.stringify(path.traversals)}`);
  for (const t of path.traversals) {
    if (t.to_index !== t.from_index + 1 || t.to_index >= path.points.length) throw Error(`non-consecutive traversal ${JSON.stringify(t)}`);
  }
  s.points = path.points.map((p) => [p[0], p[1], p[2]]);
  s.jumps = path.traversals.map((t) => ({ i: t.from_index, link: t.link_id, reversed: t.reversed }));
  s.seg = 0;
  s.s = 0;
  s.generations.push(path.generation);
}

function jumpAt(s: State, seg: number): Jump | undefined {
  return s.jumps.find((j) => j.i === seg);
}

function horizontal(a: number[], b: number[]): number {
  return Math.hypot(b[0]! - a[0]!, b[2]! - a[2]!);
}

function jumpDuration(a: number[], b: number[]): number {
  return Math.max(JUMP_MIN, horizontal(a, b) / JUMP_HSPEED);
}

function jumpPoint(a: number[], b: number[], t: number): number[] {
  const h = APEX + Math.abs(b[1]! - a[1]!) / 2;
  return [a[0]! + (b[0]! - a[0]!) * t, a[1]! + (b[1]! - a[1]!) * t + 4 * h * t * (1 - t), a[2]! + (b[2]! - a[2]!) * t];
}

function evaluate(s: State): number[] {
  const last = s.points.length - 1;
  if (s.seg >= last) return s.points[last]!.slice();
  const a = s.points[s.seg]!, b = s.points[s.seg + 1]!;
  if (jumpAt(s, s.seg)) return jumpPoint(a, b, s.s);
  const length = Math.hypot(b[0]! - a[0]!, b[1]! - a[1]!, b[2]! - a[2]!);
  const f = length > 0 ? s.s / length : 1;
  return a.map((v, i) => v + (b[i]! - v) * f);
}

/** Advance along the route by dt seconds, carrying leftover time across segments
 * and across walk/jump boundaries so no tick loses motion at a corner. */
function advance(api: ScriptApi, s: State, dt: number): void {
  let budget = dt;
  const last = s.points.length - 1;
  while (budget > 0 && s.seg < last) {
    const a = s.points[s.seg]!, b = s.points[s.seg + 1]!;
    const jump = jumpAt(s, s.seg);
    if (jump) {
      const T = jumpDuration(a, b);
      if (s.s === 0) {
        s.launches++;
        emit(api, s, 'launch', { link: jump.link, key: LINK_KEY[jump.link]!, reversed: jump.reversed, from_index: jump.i, to_index: jump.i + 1, from: a, to: b, duration_s: T, budget_s: budget });
      }
      const remain = (1 - s.s) * T;
      if (budget >= remain) {
        budget -= remain;
        s.seg++;
        s.s = 0;
        s.landings++;
        emit(api, s, 'land', { link: jump.link, key: LINK_KEY[jump.link]!, at: b, leftover_s: budget });
        continue;
      }
      s.s += budget / T;
      budget = 0;
    } else {
      const length = Math.hypot(b[0]! - a[0]!, b[1]! - a[1]!, b[2]! - a[2]!);
      const remain = length - s.s;
      if (budget * WALK_SPEED >= remain) {
        budget -= remain / WALK_SPEED;
        s.seg++;
        s.s = 0;
        continue;
      }
      s.s += budget * WALK_SPEED;
      budget = 0;
    }
  }
}

function slewYaw(s: State, from: number[], to: number[], dt: number): void {
  const dx = to[0]! - from[0]!, dz = to[2]! - from[2]!;
  if (Math.hypot(dx, dz) < 1e-6) return;
  const target = Math.atan2(-dx, -dz); // model forward is local -Z
  let d = target - s.yaw;
  while (d > Math.PI) d -= 2 * Math.PI;
  while (d < -Math.PI) d += 2 * Math.PI;
  const max = TURN_RATE * dt;
  s.yaw += Math.max(-max, Math.min(max, d));
}

function setReturnLink(api: ScriptApi, s: State, enabled: boolean): void {
  const nav = api.query('NavigationMesh').find((e) => e.id === s.nav)!;
  const value = JSON.parse(JSON.stringify(nav.components.NavigationMesh)) as { links: OffMeshLink[] } & Record<string, Json>;
  const link = value.links.find((l) => l.id === LINKS.ret)!;
  link.enabled = enabled;
  api.command({ op: 'set_component', scene_id: s.scene, entity_id: s.nav, component: 'NavigationMesh', value: value as unknown as Json });
  // Swap the rendered marker colour with the authored state: the visible marker's
  // transform is copied to its counterpart, which takes its place.
  const on = enabled ? ' on' : ' off', off = enabled ? ' off' : ' on';
  for (const marker of RETURN_MARKERS) {
    const shown = api.query('Transform').find((e) => e.id === ids.pads[marker + off])!;
    const t = shown.components.Transform as { translation: number[]; rotation: number[] };
    api.command({
      op: 'set_component', scene_id: s.scene, entity_id: ids.pads[marker + on]!, component: 'Transform',
      value: { translation: [t.translation[0]!, t.translation[1]!, t.translation[2]!], rotation: [t.rotation[0]!, t.rotation[1]!, t.rotation[2]!, t.rotation[3]!], scale: [1, 1, 1] },
    });
    setTransform(api, s, ids.pads[marker + off]!, PARK);
  }
  emit(api, s, enabled ? 'enable' : 'disable', { link: LINKS.ret, command: 'set_component NavigationMesh' });
}

function authoredLinks(api: ScriptApi, s: State): Json {
  const nav = api.query('NavigationMesh').find((e) => e.id === s.nav)!;
  return (nav.components.NavigationMesh as { links: Json }).links;
}

export default defineBehavior<State>({
  initialState: {
    tick: 0, scene: '', nav: '', walker: '', phase: 'boot', points: [], jumps: [], seg: 0, s: 0,
    pos: [...START], yaw: -Math.PI / 2, pause: 0, walkDots: 0, jumpDots: { up: 0, bridge: 0, ret: 0 },
    generations: [], events: 0, launches: 0, landings: 0,
  },
  update(api: ScriptApi, dt: number, s: State) {
    s.tick++;
    resolveIds(api);
    if (s.phase === 'boot') {
      const names = byName(api);
      const walker = names.get('Walker')!;
      s.scene = walker.scene_id;
      s.walker = walker.id;
      s.nav = api.query('NavigationMesh')[0]!.id;
      const out = query(api, s, START, GOAL);
      const back = query(api, s, GOAL, START);
      emit(api, s, 'plan', { name: 'out', from: START, to: GOAL, path: describe(out), authored_links: authoredLinks(api, s) });
      emit(api, s, 'plan', { name: 'back-before-unlock', from: GOAL, to: START, path: describe(back), expect: 'null' });
      if (!out || back !== null) throw Error('unexpected initial connectivity');
      adopt(s, out, [{ link: LINKS.up, reversed: false }, { link: LINKS.bridge, reversed: false }]);
      s.pos = s.points[0]!.slice();
      s.phase = 'out';
    } else if (s.phase === 'unlock') {
      // The enable command from the previous tick is now visible: replan.
      const back = query(api, s, GOAL, START);
      emit(api, s, 'plan', { name: 'back-after-unlock', from: GOAL, to: START, path: describe(back) });
      if (!back) throw Error('return still unreachable after enabling the return link');
      adopt(s, back, [{ link: LINKS.bridge, reversed: true }, { link: LINKS.ret, reversed: false }]);
      s.phase = 'return';
    } else if (s.phase === 'lock') {
      const back = query(api, s, GOAL, START);
      const out = query(api, s, START, GOAL);
      emit(api, s, 'plan', { name: 'back-after-lock', from: GOAL, to: START, path: describe(back), expect: 'null' });
      emit(api, s, 'plan', { name: 'out-after-lock', from: START, to: GOAL, path: describe(out) });
      if (back !== null || !out) throw Error('unexpected connectivity after disabling the return link');
      s.generations.push(out.generation);
      s.phase = 'done';
    }

    const before = s.pos.slice();
    const segBefore = s.seg;
    if (s.phase === 'out' || s.phase === 'return') advance(api, s, dt);
    const pos = s.phase === 'out' || s.phase === 'return' ? evaluate(s) : s.pos;
    const airborne = (s.phase === 'out' || s.phase === 'return') && s.seg < s.points.length - 1 && !!jumpAt(s, s.seg);
    slewYaw(s, before, pos, dt);
    s.pos = pos;
    setTransform(api, s, s.walker, pos, s.yaw);

    if (airborne) {
      const key = LINK_KEY[jumpAt(s, s.seg)!.link]!;
      const pool = ids.pools[key]!;
      if (s.tick % JUMP_DOT_EVERY === 0 && s.jumpDots[key]! < pool.length) {
        setTransform(api, s, pool[s.jumpDots[key]!]!, pos);
        s.jumpDots[key]!++;
      }
    } else if ((s.phase === 'out' || s.phase === 'return') && s.tick % WALK_DOT_EVERY === 0) {
      const pool = ids.pools.walk!;
      setTransform(api, s, pool[s.walkDots % pool.length]!, [pos[0]!, pos[1]! + DOT_LIFT, pos[2]!]);
      s.walkDots++;
    }
    api.log(JSON.stringify({ t: s.tick, ph: s.phase, m: airborne ? 'jump' : 'walk', seg: s.seg, segBefore, s: s.s, p: pos, yaw: s.yaw }), 'debug');

    if ((s.phase === 'out' || s.phase === 'return') && s.seg >= s.points.length - 1) {
      emit(api, s, 'arrive', { leg: s.phase, at: pos });
      if (s.phase === 'out') {
        s.phase = 'pause';
        s.pause = PAUSE_TICKS;
      } else {
        setReturnLink(api, s, false);
        s.phase = 'lock';
      }
    } else if (s.phase === 'pause' && --s.pause === 0) {
      const back = query(api, s, GOAL, START);
      emit(api, s, 'plan', { name: 'back-before-enable', from: GOAL, to: START, path: describe(back), expect: 'null' });
      if (back !== null) throw Error('return reachable before enabling the return link');
      setReturnLink(api, s, true);
      s.phase = 'unlock';
    }
    return undefined;
  },
});
