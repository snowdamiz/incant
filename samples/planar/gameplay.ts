import type { NativeBehavior, NativeChunk, NativeContext, NativeProp } from '@incant/runtime/native';

type Actor = { chunk: NativeChunk; row: number };
type Progress = {
  version: 1; phase: 'playing' | 'won' | 'lost'; collected: string[];
  exitInside: boolean; previousPlayer: [number, number];
};
type Config = {
  player: string; exit: string; pickups: string[]; hazards: string[];
  camera: string | null; speed: number;
  flipbooks: { player: string | null; boat: string | null } | null;
  hud: { table: string; status: string; prompt: string } | null;
  results: { won: string; lost: string; active: string; inactive: string } | null;
};
let cached: Config | undefined;
let displayed: { phase: Progress['phase']; collected: number; locale: string } | undefined;

function id(value: NativeProp | undefined, name: string): string {
  if (typeof value !== 'string' || !/^[0-7][0-9A-HJKMNP-TV-Z]{25}$/.test(value))
    throw Error(`Planar Sample: invalid ${name} identity`);
  return value;
}
function ids(value: NativeProp | undefined, name: string, minimum: number): string[] {
  if (!Array.isArray(value) || value.length < minimum || value.length > 16)
    throw Error(`Planar Sample: ${name} requires ${minimum}..16 entities`);
  return value.map(entry => id(entry, name));
}
function binding(value: NativeProp | undefined, name: string, keys: readonly string[]) {
  if (value === undefined || value === null) return null;
  if (typeof value !== 'object' || Array.isArray(value)) throw Error(`Planar Sample: invalid ${name} binding`);
  const p = value as { readonly [key: string]: NativeProp };
  if (Object.keys(p).some(key => !keys.includes(key))) throw Error(`Planar Sample: unknown ${name} binding`);
  return p;
}
function optionalId(value: NativeProp | undefined, name: string): string | null {
  return value === undefined || value === null ? null : id(value, name);
}
function config(context: NativeContext): Config {
  if (context.instances.length !== 1) throw Error('Planar Sample requires one controller');
  if (cached) return cached;
  const p = context.instances[0].props;
  const f = binding(p.flipbooks, 'flipbooks', ['player', 'boat']);
  const h = binding(p.hud, 'hud', ['table', 'status', 'prompt']);
  const r = binding(p.results, 'results', ['won', 'lost', 'active', 'inactive']);
  const result = {
    player: id(p.player, 'player'), exit: id(p.exit, 'exit'),
    pickups: ids(p.pickups, 'pickups', 1), hazards: ids(p.hazards ?? [], 'hazards', 0),
    camera: p.camera === null || p.camera === undefined ? null : id(p.camera, 'camera'),
    speed: p.speed === undefined ? 3 : p.speed,
    flipbooks: f && { player: optionalId(f.player, 'flipbooks.player'), boat: optionalId(f.boat, 'flipbooks.boat') },
    hud: h && { table: id(h.table, 'hud.table'), status: id(h.status, 'hud.status'), prompt: id(h.prompt, 'hud.prompt') },
    results: r && { won: id(r.won, 'results.won'), lost: id(r.lost, 'results.lost'),
      active: id(r.active, 'results.active'), inactive: id(r.inactive, 'results.inactive') },
  };
  if (typeof result.speed !== 'number' || !Number.isFinite(result.speed) || result.speed <= 0 || result.speed > 12)
    throw Error('Planar Sample speed must be finite in (0,12]');
  if (Object.keys(p).some(key => !Object.hasOwn(result, key))) throw Error('Planar Sample: unknown property');
  const roles = [result.player, result.exit, ...result.pickups, ...result.hazards];
  if (result.camera) roles.push(result.camera);
  if (new Set(roles).size !== roles.length) throw Error('Planar Sample roles must be distinct');
  const frames = result.flipbooks && Object.values(result.flipbooks).filter((v): v is string => v !== null);
  if (frames && new Set(frames).size !== frames.length) throw Error('Planar Sample flipbook roles must be distinct');
  const widgets = [...(result.hud ? [result.hud.status, result.hud.prompt] : []),
    ...(result.results ? Object.values(result.results) : [])];
  if (new Set(widgets).size !== widgets.length) throw Error('Planar Sample UI roles must be distinct');
  return cached = { ...result, speed: result.speed };
}
function actor(rows: Map<string, Actor>, name: string): Actor {
  const row = rows.get(name);
  if (!row) throw Error(`Planar Sample entity is missing: ${name}`);
  return row;
}
function position(row: Actor): [number, number] {
  const offset = row.row * row.chunk.transformStride;
  return [row.chunk.transforms[offset], row.chunk.transforms[offset + 1]];
}
function velocity(row: Actor, x: number, y: number): void {
  if (!row.chunk.velocities) throw Error('Planar Sample player requires Velocity');
  const offset = row.row * row.chunk.velocityStride;
  row.chunk.velocities[offset] = x;
  row.chunk.velocities[offset + 1] = y;
  row.chunk.velocities[offset + 2] = 0;
}
function progress(value: NativeProp | null, c: Config, at: [number, number]): Progress {
  if (value === null) return { version: 1, phase: 'playing', collected: [], exitInside: false, previousPlayer: at };
  if (typeof value !== 'object' || Array.isArray(value)) throw Error('Planar Sample: invalid saved state');
  const state = value as unknown as Progress;
  if (state.version !== 1 || !['playing', 'won', 'lost'].includes(state.phase) ||
      typeof state.exitInside !== 'boolean' || !Array.isArray(state.collected) ||
      state.collected.length > c.pickups.length || new Set(state.collected).size !== state.collected.length ||
      state.collected.some(item => !c.pickups.includes(item)) ||
      !Array.isArray(state.previousPlayer) || state.previousPlayer.length !== 2 ||
      state.previousPlayer.some(item => typeof item !== 'number' || !Number.isFinite(item)))
    throw Error('Planar Sample: incompatible saved state');
  return { ...state, collected: [...state.collected] };
}

/** Optional authored roles only. State frames are paused once, then sought on
 * transitions; HUD strings refresh on gameplay or locale changes. The cache is
 * ephemeral: a fresh VM rebinds accepted state, never substitutes a game reset.
 * No restart prompt is displayed until a real whole-session reset exists. */
function present(context: NativeContext, c: Config, state: Progress): void {
  const first = displayed === undefined, changedPhase = displayed?.phase !== state.phase;
  const ready = state.collected.length === c.pickups.length;
  if (c.flipbooks) {
    const seek = (entity: string | null, frame: number) => {
      if (!entity) return;
      if (first) context.commands.controlFlipbook(entity, 'pause');
      context.commands.controlFlipbook(entity, 'seek', frame);
    };
    if (changedPhase) seek(c.flipbooks.player, state.phase === 'won' ? 2 : state.phase === 'lost' ? 1 : 0);
    if (first || ready !== (displayed?.collected === c.pickups.length)) seek(c.flipbooks.boat, ready ? 1 : 0);
  }
  let localeKey = '';
  if (c.hud) {
    const locale = context.localization.locale();
    localeKey = [locale.locale, locale.pseudo ? '1' : '0', ...(locale.fallbacks ?? [])].join('|');
    if (first || changedPhase || displayed?.collected !== state.collected.length || displayed?.locale !== localeKey) {
      const key = state.phase === 'won' ? 'hud.won' : state.phase === 'lost' ? 'hud.lost'
        : ready ? 'hud.ready' : 'hud.collecting';
      const text = context.localization.localize({ table_id: c.hud.table, key,
        arguments: { collected: state.collected.length, total: c.pickups.length } });
      if (text.missing?.kind === 'key') throw Error(`Planar Sample: missing HUD key ${key}`);
      context.commands.setUiText(c.hud.status, text.text);
    }
    if (first) context.commands.setUiText(c.hud.prompt, '');
  }
  if (c.results && changedPhase) {
    context.commands.setParent(c.results.won, state.phase === 'won' ? c.results.active : c.results.inactive);
    context.commands.setParent(c.results.lost, state.phase === 'lost' ? c.results.active : c.results.inactive);
  }
  displayed = { phase: state.phase, collected: state.collected.length, locale: localeKey };
}

/** A top-down XY game driven by native 2D sensor events and bulk velocity writes.
 * Author a zero-gravity dynamic player and fixed sensors/solid tilemap. Configured
 * player/camera are unparented. Layout, sprites and input bindings belong to the
 * project; gameplay never changes the authoring document or moves through walls.
 */
export default {
  persistence: { version: 1 },
  update(context: NativeContext) {
    const c = config(context), needed = new Set([c.player, c.exit, ...c.pickups, ...c.hazards]);
    if (c.camera) needed.add(c.camera);
    const rows = new Map<string, Actor>();
    for (const chunk of context.chunks) for (let row = 0; row < chunk.ids.length; row++)
      if (needed.has(chunk.ids[row])) rows.set(chunk.ids[row], { chunk, row });
    const player = actor(rows, c.player), at = position(player), state = progress(context.state.get(), c, at);
    actor(rows, c.exit);
    for (const hazard of c.hazards) actor(rows, hazard);
    for (const pickup of c.pickups) if (!state.collected.includes(pickup)) actor(rows, pickup);
    if (c.camera) {
      const camera = actor(rows, c.camera), offset = camera.row * camera.chunk.transformStride;
      for (let axis = 0; axis < 2; axis++) camera.chunk.transforms[offset + axis] += at[axis] - state.previousPlayer[axis];
    }
    state.previousPlayer = at;
    if (state.phase === 'playing') {
      let hazard = false;
      for (const event of context.physics.triggerEvents2D()) {
        const other = event.first === c.player ? event.second : event.second === c.player ? event.first : null;
        if (other === c.exit) state.exitInside = event.entered;
        if (!event.entered || other === null) continue;
        if (c.hazards.includes(other)) hazard = true;
        if (c.pickups.includes(other) && !state.collected.includes(other)) {
          state.collected.push(other);
          context.commands.despawn(other);
          context.log(`planar.pickup ${other} ${state.collected.length}/${c.pickups.length}`);
        }
      }
      // A hazard takes priority when both terminal sensors are entered together.
      if (hazard) state.phase = 'lost';
      else if (state.exitInside && state.collected.length === c.pickups.length) state.phase = 'won';
      if (state.phase !== 'playing') context.log(`planar.${state.phase} tick=${context.clock.tick} pickups=${state.collected.length}`);
    }
    const move = state.phase === 'playing' ? context.input.actions.move?.value ?? [0, 0] : [0, 0];
    const factor = c.speed / Math.max(1, Math.hypot(move[0], move[1]));
    velocity(player, move[0] * factor, move[1] * factor);
    present(context, c, state);
    context.state.set(state);
    return undefined;
  },
} satisfies NativeBehavior;
