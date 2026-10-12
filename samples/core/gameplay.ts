import type { NativeBehavior, NativeChunk, NativeContext, NativeId, NativeProp } from '@incant/runtime/native';

type Vec3 = [number, number, number];
type Actor = { chunk: NativeChunk; row: number };
type Route = { points: readonly Vec3[]; cursor: number; lastPlan: number };
type Progress = {
  version: 1; health: number; collected: readonly string[];
  phase: 'playing' | 'won' | 'lost'; invulnerableUntil: number;
  verticalSpeed: number; grounded: boolean; routes: readonly Route[];
  previousPlayer: Vec3; jumps: number; searches: number;
};
type Config = {
  player: string; navigation: string; exit: string;
  enemies: readonly string[]; pickups: readonly string[]; camera: string | null;
  speed: number; enemySpeed: number; jumpSpeed: number;
  hud: HudBinding | null; audio: AudioBinding | null;
};
type HudBinding = {
  table: string; health: string; collected: string; objective: string;
  won: string; lost: string; result: string; inactive: string;
};
type AudioBinding = Partial<Record<'music' | 'pickup' | 'damage' | 'won' | 'lost', string>>;
let cached: Config | undefined;
let displayed: { health: number; collected: number; phase: Progress['phase']; locale: string } | undefined;

function properties(value: NativeProp, field: string): { readonly [key: string]: NativeProp } {
  if (value === null || typeof value !== 'object' || Array.isArray(value))
    throw Error(`Core Sample: ${field} must be an object`);
  return value as { readonly [key: string]: NativeProp };
}
function hudBinding(value: NativeProp | undefined): HudBinding | null {
  if (value === undefined || value === null) return null;
  const p = properties(value, 'hud');
  const result = {
    table: id(p.table, 'hud.table'), health: id(p.health, 'hud.health'),
    collected: id(p.collected, 'hud.collected'), objective: id(p.objective, 'hud.objective'),
    won: id(p.won, 'hud.won'), lost: id(p.lost, 'hud.lost'),
    result: id(p.result, 'hud.result'), inactive: id(p.inactive, 'hud.inactive'),
  };
  const widgets = [result.health, result.collected, result.objective, result.won,
    result.lost, result.result, result.inactive];
  if (new Set(widgets).size !== widgets.length) throw Error('Core Sample: HUD roles must be distinct');
  if (Object.keys(p).some(key => !Object.hasOwn(result, key))) throw Error('Core Sample: unknown HUD binding');
  return result;
}
function audioBinding(value: NativeProp | undefined): AudioBinding | null {
  if (value === undefined || value === null) return null;
  const p = properties(value, 'audio'), result: AudioBinding = {};
  for (const key of Object.keys(p)) {
    if (!['music', 'pickup', 'damage', 'won', 'lost'].includes(key))
      throw Error('Core Sample: unknown audio binding');
    result[key as keyof AudioBinding] = id(p[key], 'audio.' + key);
  }
  const sources = Object.values(result);
  if (new Set(sources).size !== sources.length) throw Error('Core Sample: audio roles must be distinct');
  return result;
}

function id(value: NativeProp | undefined, field: string): string {
  if (typeof value !== 'string' || !/^[0-7][0-9A-HJKMNP-TV-Z]{25}$/.test(value))
    throw Error(`Core Sample: invalid ${field} entity`);
  return value;
}
function ids(value: NativeProp | undefined, field: string): string[] {
  if (!Array.isArray(value) || value.length === 0 || value.length > 16)
    throw Error(`Core Sample: ${field} requires 1..16 entities`);
  const result = value.map((entry) => id(entry, field));
  if (new Set(result).size !== result.length) throw Error(`Core Sample: duplicate ${field}`);
  return result;
}
function speed(value: NativeProp | undefined, fallback: number): number {
  if (value === undefined) return fallback;
  if (typeof value !== 'number' || !Number.isFinite(value) || value <= 0 || value > 12)
    throw Error('Core Sample: speed must be finite and in (0,12]');
  return value;
}
function config(context: NativeContext): Config {
  if (context.instances.length !== 1) throw Error('Core Sample needs one gameplay controller');
  if (cached) return cached;
  const p = context.instances[0].props;
  const result = {
    player: id(p.player, 'player'), navigation: id(p.navigation, 'navigation'),
    exit: id(p.exit, 'exit'), enemies: ids(p.enemies, 'enemies'), pickups: ids(p.pickups, 'pickups'),
    camera: p.camera === undefined || p.camera === null ? null : id(p.camera, 'camera'),
    speed: speed(p.speed, 3), enemySpeed: speed(p.enemySpeed, 1.5), jumpSpeed: speed(p.jumpSpeed, 6),
    hud: hudBinding(p.hud), audio: audioBinding(p.audio),
  };
  const owned = [result.player, result.navigation, result.exit, ...result.enemies, ...result.pickups];
  if (result.camera) owned.push(result.camera);
  if (new Set(owned).size !== owned.length) throw Error('Core Sample roles must use distinct entities');
  return cached = result;
}
function position(actor: Actor): Vec3 {
  const offset = actor.row * actor.chunk.transformStride;
  return [actor.chunk.transforms[offset], actor.chunk.transforms[offset + 1], actor.chunk.transforms[offset + 2]];
}
function velocity(actor: Actor, value: Vec3): void {
  const values = actor.chunk.velocities;
  if (!values) throw Error('Core Sample mover requires an authored Velocity component');
  const offset = actor.row * actor.chunk.velocityStride;
  values[offset] = value[0]; values[offset + 1] = value[1]; values[offset + 2] = value[2];
}
function actor(actors: Map<NativeId, Actor>, entity: string): Actor {
  const result = actors.get(entity);
  if (!result) throw Error(`Core Sample entity is missing: ${entity}`);
  return result;
}
function distance(a: Vec3, b: Vec3): number {
  return Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);
}
function advance(context: NativeContext, body: Actor, entity: string, displacement: Vec3) {
  const movement = context.physics.computeCharacterMotion({ entity_id: entity, translation: displacement });
  velocity(body, movement.translation.map((v) => v / context.dt) as Vec3);
  return movement;
}
function initial(player: Vec3, count: number): Progress {
  return { version: 1, health: 3, collected: [], phase: 'playing', invulnerableUntil: 0,
    verticalSpeed: 0, grounded: false, routes: Array.from({ length: count }, () => ({ points: [], cursor: 0, lastPlan: -1 })),
    previousPlayer: player, jumps: 0, searches: 0 };
}

/** Bind gameplay to authored widgets and sources without changing their styling,
 * fonts, geometry, audio routing or the project document. Only changed displays
 * queue commands. A fresh VM refreshes text/parents but does not replay cues. */
function present(context: NativeContext, c: Config, state: Progress, previous: Progress | null): void {
  if (c.hud) {
    const h = c.hud, locale = context.localization.locale();
    const localeKey = [locale.locale, locale.pseudo ? '1' : '0', ...(locale.fallbacks ?? [])].join('|');
    const refresh = displayed === undefined || displayed.locale !== localeKey;
    const text = (key: string, args: Record<string, number> = {}) => {
      const value = context.localization.localize({ table_id: h.table, key, arguments: args });
      if (value.missing?.kind === 'key') throw Error(`Core Sample: missing HUD key ${key}`);
      return value.text;
    };
    if (refresh || displayed!.health !== state.health)
      context.commands.setUiText(h.health, text('hud.health', { health: state.health }));
    if (refresh || displayed!.collected !== state.collected.length) {
      context.commands.setUiText(h.collected, text('hud.samples', {
        collected: state.collected.length, total: c.pickups.length,
      }));
      context.commands.setUiText(h.objective, text(
        state.collected.length === c.pickups.length ? 'hud.objective.exit' : 'hud.objective.collect'));
    }
    if (refresh) {
      context.commands.setUiText(h.won, text('hud.won'));
      context.commands.setUiText(h.lost, text('hud.lost'));
    }
    if (displayed === undefined || displayed.phase !== state.phase) {
      context.commands.setParent(h.won, state.phase === 'won' ? h.result : h.inactive);
      context.commands.setParent(h.lost, state.phase === 'lost' ? h.result : h.inactive);
    }
    displayed = { health: state.health, collected: state.collected.length, phase: state.phase, locale: localeKey };
  }
  const a = c.audio;
  if (!a) return;
  if (a.music && previous === null && state.phase === 'playing') context.commands.restartAudio(a.music);
  if (a.pickup && state.collected.length > (previous?.collected.length ?? 0)) context.commands.restartAudio(a.pickup);
  if (a.damage && state.health < (previous?.health ?? 3)) context.commands.restartAudio(a.damage);
  if (state.phase !== 'playing' && previous?.phase !== state.phase) {
    if (a.music) context.commands.stopAudio(a.music);
    const cue = a[state.phase];
    if (cue) context.commands.restartAudio(cue);
  }
}

/** One source callback updates the whole game through native column views.
 * All gameplay state lives in the explicit source state. No project queries,
 * authoring commands, per-entity script callbacks or world serialization.
 * Positions are world-space: all configured bodies/camera must be unparented.
 * Level construction rejects off-mesh links: this game follows walkable paths.
 */
export default {
  persistence: { version: 1 },
  update(context: NativeContext) {
    const c = config(context);
    const needed = new Set([c.player, c.exit, ...c.enemies, ...c.pickups]);
    if (c.camera) needed.add(c.camera);
    const actors = new Map<string, Actor>();
    for (const chunk of context.chunks) for (let row = 0; row < chunk.ids.length; row++) {
      const entity = chunk.ids[row];
      if (needed.has(entity)) actors.set(entity, { chunk, row });
    }
    const player = actor(actors, c.player), at = position(player);
    const previous = context.state.get();
    const stored = previous as Progress | null;
    if (stored && (stored.version !== 1 || stored.routes.length !== c.enemies.length ||
        !['playing', 'won', 'lost'].includes(stored.phase) || stored.health < 0 || stored.health > 3))
      throw Error('Core Sample: incompatible saved gameplay state');
    const state: Progress = stored ? { ...stored } : initial(at, c.enemies.length);
    if (c.camera) {
      const camera = actor(actors, c.camera), offset = camera.row * camera.chunk.transformStride;
      for (let axis = 0; axis < 3; axis++) camera.chunk.transforms[offset + axis] += at[axis] - state.previousPlayer[axis];
    }
    state.previousPlayer = at;
    if (state.phase !== 'playing') {
      velocity(player, [0, 0, 0]);
      for (const entity of c.enemies) velocity(actor(actors, entity), [0, 0, 0]);
      present(context, c, state, stored);
      context.state.set(state);
      return undefined;
    }

    const collected = new Set(state.collected);
    for (const event of context.physics.triggerEvents()) {
      if (!event.entered) continue;
      const other = event.first === c.player ? event.second : event.second === c.player ? event.first : null;
      if (other !== null && c.pickups.includes(other) && !collected.has(other)) {
        collected.add(other);
        context.commands.despawn(other);
        context.log(`core.pickup ${other} ${collected.size}/${c.pickups.length}`);
      }
    }
    state.collected = [...collected];
    if (state.collected.length === c.pickups.length && distance(at, position(actor(actors, c.exit))) <= 1) {
      state.phase = 'won';
      context.log(`core.won tick=${context.clock.tick} health=${state.health} pickups=${state.collected.length}`);
    }

    if (state.phase === 'playing' && context.clock.tick >= state.invulnerableUntil &&
        c.enemies.some((entity) => distance(at, position(actor(actors, entity))) <= 1.2)) {
      state.health -= 1;
      state.invulnerableUntil = context.clock.tick + Math.ceil(0.75 / context.dt);
      context.log(`core.damage health=${state.health}`);
      if (state.health === 0) {
        state.phase = 'lost';
        context.log(`core.lost tick=${context.clock.tick}`);
      }
    }
    if (state.phase !== 'playing') {
      velocity(player, [0, 0, 0]);
      for (const entity of c.enemies) velocity(actor(actors, entity), [0, 0, 0]);
      present(context, c, state, stored);
      context.state.set(state);
      return undefined;
    }

    const move = context.input.actions.move?.value ?? [0, 0];
    const magnitude = Math.hypot(move[0], move[1]);
    const factor = c.speed * context.dt / Math.max(1, magnitude);
    if (context.input.actions.jump?.pressed && state.grounded) {
      state.verticalSpeed = c.jumpSpeed;
      state.jumps += 1;
      context.log(`core.jump ${state.jumps}`);
    }
    state.verticalSpeed = Math.max(-25, state.verticalSpeed - 18 * context.dt);
    const moved = advance(context, player, c.player, [move[0] * factor, state.verticalSpeed * context.dt, move[1] * factor]);
    state.grounded = moved.grounded;
    if ((moved.grounded && state.verticalSpeed < 0) ||
        (state.verticalSpeed > 0 && moved.translation[1] < state.verticalSpeed * context.dt - 1e-6)) state.verticalSpeed = 0;

    // Stagger refreshes, so at most one bounded navmesh search occurs per tick.
    // A route miss stops that enemy; it never falls back to walking through walls.
    const routes = state.routes.map((route) => ({ ...route }));
    const refresh = (context.clock.tick - 1) % c.enemies.length;
    for (let index = 0; index < c.enemies.length; index++) {
      const entity = c.enemies[index], body = actor(actors, entity), from = position(body), route = routes[index];
      if (index === refresh && (route.lastPlan < 0 || context.clock.tick - route.lastPlan >= 15)) {
        const path = context.navigation.findPath(c.navigation, {
          start: from, end: at, snap_distance: 2, max_visited: 2048,
        });
        if (path && path.traversals.length) throw Error('Core Sample level requires an explicit off-mesh traversal controller');
        if (path && path.points.length > 128) throw Error('Core Sample navigation route exceeds 128 corners');
        route.points = path?.points ?? []; route.cursor = 1; route.lastPlan = context.clock.tick;
        state.searches += 1;
        if (state.searches <= c.enemies.length)
          context.log(`core.route ${entity} corners=${route.points.length}`);
      }
      while (route.cursor < route.points.length && Math.hypot(
        route.points[route.cursor][0] - from[0], route.points[route.cursor][2] - from[2]) < 0.15) route.cursor += 1;
      let dx = 0, dz = 0;
      if (route.cursor < route.points.length) {
        const target = route.points[route.cursor];
        dx = target[0] - from[0]; dz = target[2] - from[2];
        const amount = Math.min(1, c.enemySpeed * context.dt / Math.max(1e-9, Math.hypot(dx, dz)));
        dx *= amount; dz *= amount;
      }
      advance(context, body, entity, [dx, -0.1, dz]);
    }
    state.routes = routes;
    present(context, c, state, stored);
    context.state.set(state);
    return undefined;
  },
} satisfies NativeBehavior;
