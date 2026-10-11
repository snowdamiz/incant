/**
 * NATIVE-BRIDGE DOUBLE. NOT ENGINE DATA AND NOT NATIVE PLAY.
 *
 * A deterministic stand-in for the Tauri commands behind editor/bridge/native.ts, so the
 * real NativeBridge class (and, in the browser review, the real native.generated.js) can
 * drive the UI: engine_read, play_read and play_command, plus the provider, window and
 * viewport calls the bridge makes at startup. It mirrors the rules and messages of the
 * editor worker (editor/app/src/play/mod.rs and worker.rs, 3910a85): generations, the
 * single startup gate, Stop during preparation, Pause, Resume, the deferred one-tick Step,
 * the 128-event packet and 1024-event pending limits, and one-entity inspection.
 *
 * Nothing here cooks, simulates physics or renders. Ticks advance only when the caller
 * runs `frame()`; runtime transforms are simple functions of the tick. Used by the UI
 * tests and, transpiled by handoffs/0044-native-editor-play/tools, by the browser review.
 * It has no runtime imports, so it can be injected into a page on its own.
 */
import type { EngineRead } from '../../../bridge/native';
import type { PlayEntity, PlayInput, PlayStatus, WindowChrome } from '../bridge/contract';

type Vec3 = [number, number, number];
type Quat = [number, number, number, number];
type Components = Record<string, Record<string, unknown>>;
interface Entity {
  id: string;
  name: string;
  parent: string | null;
  components: Components;
}

const CROCKFORD = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';
/** Deterministic canonical ULID `n` for the double's project. */
export function doubleId(n: number): string {
  let suffix = '';
  let rest = n;
  for (let index = 0; index < 16; index += 1) {
    suffix = CROCKFORD.charAt(rest % 32) + suffix;
    rest = Math.floor(rest / 32);
  }
  return `01K7Q5ZT4M${suffix}`;
}

export const IDS = {
  project: doubleId(1),
  harbor: doubleId(10),
  title: doubleId(11),
  rig: doubleId(20),
  playerCamera: doubleId(21),
  overviewCamera: doubleId(22),
  dockCamera: doubleId(23),
  player: doubleId(30),
  crate1: doubleId(31),
  crate2: doubleId(32),
  lantern: doubleId(33),
  spawn: doubleId(34),
  sign: doubleId(35),
  titleCamera: doubleId(40),
  logo: doubleId(41),
} as const;

const transform = (translation: Vec3, rotation: Quat = [0, 0, 0, 1], scale: Vec3 = [1, 1, 1]) => ({ translation, rotation, scale });
const body = (motion: string) => ({ motion, gravity_scale: 1, linear_damping: 0, angular_damping: 0.05, can_sleep: true, ccd: false });
const box = (half: Vec3) => ({ shape: { type: 'box', half_extents: half }, density: 420, friction: 0.7, restitution: 0, sensor: false, memberships: 4294967295, filter: 4294967295 });
const camera = (fov: number, projection?: Record<string, unknown>) => ({ fov_degrees: fov, near: 0.1, far: 400, ...(projection ? { projection } : {}) });

export type DoubleVariant = 'harbor' | 'single';

/**
 * A small native-shaped project. `harbor`: two scenes; Harbor has three cameras (one
 * must be chosen) and a sign that its script removes at runtime. `single`: Harbor only,
 * with one camera, so Play needs no choice.
 */
export function doubleRead(variant: DoubleVariant, schemas: EngineRead['schemas'], revision = 41): EngineRead {
  const harbor: Entity[] = [
    { id: IDS.rig, name: 'Camera Rig', parent: null, components: { Transform: transform([0, 0, 0]) } },
    { id: IDS.playerCamera, name: 'Player Camera', parent: IDS.rig, components: { Transform: transform([0, 3, -6], [0.0871557, 0, 0, 0.9961947]), Camera: camera(60) } },
    { id: IDS.player, name: 'Player', parent: null, components: { Transform: transform([0, 1, 0]), RigidBody: body('dynamic'), Velocity: { linear: [0, 0, 0] } } },
    { id: IDS.crate1, name: 'Crate 01', parent: null, components: { Transform: transform([2, 4, 1]), RigidBody: body('dynamic'), Collider: box([0.5, 0.5, 0.5]) } },
    { id: IDS.crate2, name: 'Crate 02', parent: null, components: { Transform: transform([3.25, 0.5, 1]), RigidBody: body('dynamic'), Collider: box([0.5, 0.5, 0.5]) } },
    { id: IDS.lantern, name: 'Dock Lantern', parent: null, components: { Transform: transform([-2, 2.5, 0]), PointLight: { color: [1, 0.78, 0.48], intensity: 180, range: 9 } } },
    { id: IDS.spawn, name: 'Spawn Point', parent: null, components: { Transform: transform([0, 0, 0]) } },
    { id: IDS.sign, name: 'Tutorial Sign', parent: null, components: { Transform: transform([1.5, 0, 3]) } },
  ];
  if (variant === 'harbor') {
    harbor.splice(
      2,
      0,
      { id: IDS.overviewCamera, name: 'Overview Camera', parent: IDS.rig, components: { Transform: transform([0, 18, 0], [-0.7071068, 0, 0, 0.7071068]), Camera: camera(60, { kind: 'orthographic', vertical_size: 24 }) } },
      { id: IDS.dockCamera, name: 'Dock Camera', parent: IDS.rig, components: { Transform: transform([6, 2, 4]), Camera: camera(45) } },
    );
  }
  const title: Entity[] = [
    { id: IDS.titleCamera, name: 'Title Camera', parent: null, components: { Transform: transform([0, 0, -10]), Camera: camera(60, { kind: 'pixel_perfect', pixels_per_unit: 16, reference_resolution: [320, 180] }) } },
    { id: IDS.logo, name: 'Logo', parent: null, components: { Transform: transform([0, 2, 0]) } },
  ];
  const scene = (id: string, name: string, entities: Entity[]) => ({ id, name, entities: Object.fromEntries(entities.map((entity) => [entity.id, entity])) });
  const scenes = variant === 'harbor'
    ? { [IDS.harbor]: scene(IDS.harbor, 'Harbor', harbor), [IDS.title]: scene(IDS.title, 'Title', title) }
    : { [IDS.harbor]: scene(IDS.harbor, 'Harbor', harbor) };
  return {
    project: { id: IDS.project, name: 'Driftwake', assets: {}, scenes },
    revision,
    can_redo: false,
    history: [],
    applied: 0,
    schemas,
    console: [],
    viewport_error: null,
    source_diagnostics: [],
    asset_import: { available: true },
  };
}

/** Runtime state as a function of the tick: what the double's "world" reports. */
function runtimeOf(entity: Entity, tick: number): { translation: Vec3; rotation: Quat; scale: Vec3; velocity: Vec3 | null } {
  const authored = entity.components.Transform as { translation: Vec3; rotation: Quat; scale: Vec3 };
  const seconds = tick / 60;
  const moving = 'Velocity' in entity.components || (entity.components.RigidBody as { motion?: string } | undefined)?.motion === 'dynamic';
  let translation: Vec3 = [...authored.translation];
  let rotation: Quat = [...authored.rotation];
  let velocity: Vec3 | null = moving ? [0, 0, 0] : null;
  if (entity.id === IDS.player) {
    // Walks along +X at 1.5 m/s, facing its direction of travel.
    translation = [authored.translation[0] + 1.5 * seconds, authored.translation[1], authored.translation[2]];
    rotation = [0, 0.7071068, 0, 0.7071068];
    velocity = [1.5, 0, 0];
  } else if (entity.id === IDS.crate1) {
    // Falls from 4 m and comes to rest on the dock at 0.5 m.
    const fall = 0.5 * 9.81 * seconds * seconds;
    const landed = authored.translation[1] - fall <= 0.5;
    translation = [authored.translation[0], landed ? 0.5 : authored.translation[1] - fall, authored.translation[2]];
    velocity = [0, landed ? 0 : -9.81 * seconds, 0];
  } else if (entity.id === IDS.lantern) {
    const half = (Math.sin(seconds * 2) * 8 * Math.PI) / 360;
    rotation = [0, 0, Math.sin(half), Math.cos(half)];
  } else if (entity.id === IDS.playerCamera) {
    translation = [authored.translation[0] + 1.5 * seconds, authored.translation[1], authored.translation[2]];
  }
  return { translation, rotation, scale: [...authored.scale], velocity };
}

interface Deferred<T> {
  resolve: (value: T) => void;
  reject: (error: unknown) => void;
}

export interface NativePlayDoubleOptions {
  readonly read: EngineRead;
  /** False refuses Play as an unsaved project does. */
  readonly saved?: boolean;
  /** Start waits in preparation until `finishStart()`; otherwise it completes at once. */
  readonly holdStart?: boolean;
  readonly window?: WindowChrome;
}

/** The double's control surface, for tests and the review script. */
export interface NativePlayDouble {
  readonly invoke: <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
  readonly listen: <T>(event: string, handler: (event: { payload: T }) => void) => Promise<() => void>;
  /** Every play_command, in order, as sent. */
  readonly commands: Record<string, unknown>[];
  /** Accepted input packets, with the generation each carried. */
  readonly inputs: { generation: number; events: PlayInput[] }[];
  /** Refused input packets and why. */
  readonly refusedInputs: { generation: number; events: PlayInput[]; error: string }[];
  readonly viewportBounds: unknown[];
  status(): PlayStatus;
  /** Completes a held start's preparation (as the worker's Load would). */
  finishStart(): void;
  /** Fails a held start's preparation with the engine's message. */
  failStart(message: string): void;
  /** One render-loop frame; `focused: false` is an unfocused window, which suspends. */
  frame(options?: { focused?: boolean }): void;
  frames(count: number): void;
  /** The surface became unavailable (minimized, hidden): Play is held. */
  unavailable(): void;
  /** A terminal runtime failure, as Worker::fail. */
  fail(message: string): void;
  /** Makes the next play_read fail, as a lost worker or transport would; restore with true. */
  setReadable(readable: boolean): void;
  setSaved(saved: boolean): void;
  /** Applies an authoring edit: the revision moves on (a start sent before it is refused). */
  bumpRevision(): void;
}

const MAX_PACKET = 128;
const MAX_PENDING = 1024;

export function createNativePlayDouble(options: NativePlayDoubleOptions): NativePlayDouble {
  let read = options.read;
  let saved = options.saved ?? true;
  let readable = true;
  const status: { -readonly [K in keyof PlayStatus]: PlayStatus[K] } = {
    generation: 0,
    sequence: 0,
    phase: 'stopped',
    tick: 0,
    discardedTicks: 0,
    presentedFrames: 0,
    scene: null,
    sourceRevision: null,
    spatial: false,
    suspended: false,
    preparing: false,
    error: null,
  };
  let loading = false;
  let game: { scene: string; running: boolean; held: boolean; pending: number } | null = null;
  let step: Deferred<PlayStatus> | null = null;
  let held: Deferred<void> | null = null;
  const commands: Record<string, unknown>[] = [];
  const inputs: NativePlayDouble['inputs'] = [];
  const refusedInputs: NativePlayDouble['refusedInputs'] = [];
  const viewportBounds: unknown[] = [];
  const listeners = new Map<string, Set<(event: { payload: unknown }) => void>>();

  const copy = (): PlayStatus => ({ ...status });
  const edit = (change: () => void) => {
    change();
    status.sequence += 1;
  };
  const failed = (generation: number, error: string) =>
    edit(() => {
      if (status.generation === generation) {
        status.phase = 'error';
        status.error = error;
        status.suspended = false;
      }
    });
  const publish = () =>
    edit(() => {
      if (!game) return;
      status.phase = game.running ? 'running' : 'paused';
      status.suspended = game.held;
    });
  const entities = (sceneId: string): Entity[] => {
    const scene = Object.hasOwn(read.project.scenes, sceneId) ? read.project.scenes[sceneId] : undefined;
    return scene ? (Object.values(scene.entities) as Entity[]) : [];
  };
  const canonical = (text: unknown) => typeof text === 'string' && /^[0-7][0-9A-HJKMNP-TV-Z]{25}$/.test(text);

  async function start(command: Record<string, unknown>): Promise<PlayStatus> {
    if (loading) throw 'A previous Play startup is still finishing';
    loading = true;
    let next = -1;
    try {
      edit(() => {
        if (status.generation !== command.generation) throw 'Play session changed; retry from current state';
        if (status.phase !== 'stopped' && status.phase !== 'error') throw 'Stop the current Play session before starting another';
        status.generation += 1;
        status.phase = 'preparing';
        status.error = null;
        status.preparing = true;
        status.tick = 0;
        status.discardedTicks = 0;
        status.presentedFrames = 0;
        status.scene = null;
        status.sourceRevision = null;
      });
      next = status.generation;
      const revision = read.revision;
      try {
        if (!saved) throw 'Open a saved project to run Play';
        if (revision !== command.expected_revision) throw 'Project changed; retry Play from the latest revision';
        const startOptions = (command.options ?? {}) as { scene?: string; camera?: string };
        const sceneIds = Object.keys(read.project.scenes);
        const scene = startOptions.scene ?? (sceneIds.length === 1 ? sceneIds[0]! : null);
        if (scene === null) throw 'native play: choose --scene when the project does not have exactly one scene';
        if (!canonical(scene)) throw 'native play: expected canonical uppercase ULID';
        if (startOptions.camera !== undefined && !canonical(startOptions.camera)) throw 'native play: expected canonical uppercase ULID';
        if (!Object.hasOwn(read.project.scenes, scene)) throw `native play startup failure: scene ${scene} does not exist`;
        if (startOptions.camera === undefined) throw 'native play: spatial content requires an explicit --camera and native render host';
        if (options.holdStart) {
          await new Promise<void>((resolve, reject) => {
            held = { resolve, reject };
          });
        }
        if (read.revision !== revision) throw 'Project changed during Play startup; retry';
        // The worker's Load: a session stopped meanwhile is not loaded.
        if (next !== status.generation) throw 'Play startup was cancelled';
        game = { scene, running: true, held: true, pending: 0 };
        edit(() => {
          status.phase = 'running';
          status.tick = 0;
          status.scene = scene;
          status.sourceRevision = revision;
          status.spatial = true;
          status.suspended = true;
          status.error = null;
        });
        return copy();
      } catch (error) {
        failed(next, String(error));
        throw error;
      }
    } finally {
      held = null;
      edit(() => {
        status.preparing = false;
      });
      loading = false;
    }
  }

  function inspect(command: Record<string, unknown>): PlayEntity {
    if (!game) throw 'Start Play first';
    const id = command.entity;
    if (typeof id !== 'string' || !/^[0-9A-HJKMNP-TV-Z]{26}$/i.test(id)) throw 'Invalid entity ID';
    if (!canonical(id)) throw 'Entity ID must be canonical';
    // The sign's script removes it on the first tick; nothing else is created or removed.
    const entity = entities(game.scene).find((item) => item.id === id && !(item.id === IDS.sign && status.tick > 0));
    if (!entity) throw 'Entity is absent from this runtime world';
    const state = runtimeOf(entity, status.tick);
    const [x, y, z] = state.translation;
    const [sx, sy, sz] = state.scale;
    return {
      generation: status.generation,
      tick: status.tick,
      entity: entity.id as PlayEntity['entity'],
      parent: entity.parent as PlayEntity['parent'],
      transform: { translation: state.translation, rotation: state.rotation, scale: state.scale },
      velocity: state.velocity ? { linear: state.velocity } : null,
      // Translation and scale only: the UI does not present the world matrix.
      worldTransform: [[sx, 0, 0, 0], [0, sy, 0, 0], [0, 0, sz, 0], [x, y, z, 1]],
    };
  }

  function validInput(events: unknown): events is PlayInput[] {
    if (!Array.isArray(events)) return false;
    const finite = (pair: unknown) => Array.isArray(pair) && pair.length === 2 && pair.every((value) => typeof value === 'number' && Number.isFinite(value) && Math.abs(value) <= 1_000_000);
    return events.every((event: Record<string, unknown>) => {
      switch (event?.type) {
        case 'focus':
          return typeof event.focused === 'boolean';
        case 'key':
          return typeof event.code === 'string' && typeof event.down === 'boolean';
        case 'mouse_button':
          return (event.button === 'left' || event.button === 'right' || event.button === 'middle') && typeof event.down === 'boolean';
        case 'pointer_position':
          return finite(event.position);
        case 'wheel':
          return finite(event.delta) && (event.unit === 'pixels' || event.unit === 'lines');
        default:
          return false;
      }
    });
  }

  async function command(raw: Record<string, unknown>): Promise<unknown> {
    commands.push(raw);
    const op = raw.op;
    if (op === 'start') return start(raw);
    if (op === 'input' && Array.isArray(raw.events) && raw.events.length > MAX_PACKET) throw 'Input packet exceeds 128 events';
    // Worker::drain: a pending step admits only Stop.
    if (step && op !== 'stop') throw 'Wait for the pending step to finish';
    if (op === 'step') {
      if (status.generation !== raw.generation || !game || game.running) throw 'Pause Play before stepping; only one step may be pending';
      return new Promise<PlayStatus>((resolve, reject) => {
        step = { resolve, reject };
      });
    }
    if (raw.generation !== status.generation) throw 'Play session changed; refresh and retry';
    if (op === 'stop') {
      game = null;
      const pending = step;
      step = null;
      pending?.reject('Play stopped before the requested step');
      edit(() => {
        status.generation += 1;
        status.phase = 'stopped';
        status.tick = 0;
        status.discardedTicks = 0;
        status.presentedFrames = 0;
        status.scene = null;
        status.sourceRevision = null;
        status.spatial = false;
        status.suspended = false;
        status.error = null;
      });
      return copy();
    }
    if (!game) throw 'Start Play first';
    switch (op) {
      case 'pause':
        game.running = false;
        game.held = true;
        game.pending = 0;
        break;
      case 'resume':
        game.running = true;
        break;
      case 'inspect':
        return inspect(raw);
      case 'input': {
        const generation = raw.generation as number;
        const events = raw.events as PlayInput[];
        const refuse = (error: string): never => {
          refusedInputs.push({ generation, events, error });
          throw error;
        };
        if (!game.running || game.held) refuse('Game input is inactive');
        if (!validInput(events)) refuse('Unsupported or invalid game input event');
        if (game.pending + events.length > MAX_PENDING) refuse('Game input queue is full');
        game.pending += events.length;
        inputs.push({ generation, events });
        break;
      }
      default:
        throw `unknown variant \`${String(op)}\``;
    }
    publish();
    return copy();
  }

  const chrome: WindowChrome = options.window ?? { platform: 'macos', controls: 'native-overlay', maximized: false, fullscreen: false, focused: true, leadingInset: 78, trailingInset: 0 };
  const invoke = async <T,>(name: string, args?: Record<string, unknown>): Promise<T> => {
    switch (name) {
      case 'engine_read':
        return read as T;
      case 'play_read':
        if (!readable) throw 'Native Play worker has stopped';
        return copy() as T;
      case 'play_command':
        return (await command((args?.command ?? {}) as Record<string, unknown>)) as T;
      case 'provider_read':
        return { status: 'not-connected', provider: 'openai' } as T;
      case 'window_read':
        return chrome as T;
      case 'viewport_bounds':
        viewportBounds.push(args);
        return null as T;
      default:
        throw `The native-bridge double does not implement ${name}.`;
    }
  };

  return {
    invoke,
    listen: async <T,>(event: string, handler: (event: { payload: T }) => void) => {
      const set = listeners.get(event) ?? new Set();
      set.add(handler as (event: { payload: unknown }) => void);
      listeners.set(event, set);
      return () => set.delete(handler as (event: { payload: unknown }) => void);
    },
    commands,
    inputs,
    refusedInputs,
    viewportBounds,
    status: copy,
    finishStart: () => held?.resolve(),
    failStart: (message) => held?.reject(message),
    frame: ({ focused = true } = {}) => {
      if (!game) return;
      const current = game;
      if (!focused && !step) {
        current.held = true;
        current.pending = 0;
      } else if (current.held) {
        current.held = false;
      }
      const advancing = current.running && !current.held;
      const stepping = step;
      step = null;
      edit(() => {
        if (advancing || stepping) status.tick += 1;
        status.presentedFrames += 1;
      });
      if (advancing || stepping) current.pending = 0;
      publish();
      stepping?.resolve(copy());
    },
    frames(count) {
      for (let index = 0; index < count; index += 1) this.frame();
    },
    unavailable: () => {
      const pending = step;
      step = null;
      pending?.reject('The viewport is unavailable; retry the step when visible');
      if (!game) return;
      game.held = true;
      game.pending = 0;
      publish();
    },
    fail: (message) => {
      game = null;
      const pending = step;
      step = null;
      pending?.reject(message);
      failed(status.generation, message);
    },
    setReadable: (value) => {
      readable = value;
    },
    setSaved: (value) => {
      saved = value;
    },
    bumpRevision: () => {
      read = { ...read, revision: read.revision + 1 };
      listeners.get('incant:engine-changed')?.forEach((handler) => handler({ payload: null }));
    },
  };
}
