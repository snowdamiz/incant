/**
 * EVIDENCE TEST DOUBLE for handoff 0033. Not engine data and not a bridge.
 *
 * Builds one BridgeSnapshot the way the native editor does: an engine "read" made of
 * an invented project plus the real Rust-generated component schemas in schemas/,
 * converted by the real compiled bridge function `snapshotFromEngine`
 * (editor/bridge/native.generated.js, loaded in a sandbox with no host). The browser
 * then receives it through a bridge object that advertises no capabilities and
 * rejects every command, so nothing can be edited.
 *
 * Only these fields are replaced after conversion, each for a stated reason:
 *   - diagnostics: engine-style messages at exact paths (the read has no validator);
 *   - viewport: "not attached" (a browser page has no native surface);
 *   - connection/label: the project is named "(test double)".
 */
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import vm from 'node:vm';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');

/** The registry the native app sends (crates/incant_doc schema_registry), minus Project. */
const COMPONENT_SCHEMAS = [
  'Transform', 'Velocity', 'Script', 'MeshRenderer', 'Camera', 'NavigationGrid', 'NavigationMesh',
  'AudioBus', 'AudioSource', 'AudioListener', 'RigidBody', 'Collider', 'AngularVelocity',
  'DirectionalLight', 'PointLight', 'SpotLight', 'EnvironmentLight',
];

function loadSnapshotFromEngine() {
  const source = readFileSync(join(repoRoot, 'editor/bridge/native.generated.js'), 'utf8');
  // The generated file is an IIFE over `{}` that ends by calling installNativeBridge(),
  // which returns early without window.__TAURI__. Capture its exports instead.
  const wrapped = source.replace(/\}\)\(\{\}\);\s*$/, '})(globalThis.__exports = {});');
  if (wrapped === source) throw new Error('native.generated.js has an unexpected shape');
  const sandbox = { window: {}, console, globalThis: undefined };
  sandbox.globalThis = sandbox;
  vm.runInNewContext(wrapped, sandbox, { filename: 'native.generated.js' });
  const fn = sandbox.__exports?.snapshotFromEngine;
  if (typeof fn !== 'function') throw new Error('snapshotFromEngine was not exported');
  return fn;
}

const CROCKFORD = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';
/** Deterministic, well-formed, invented ULID. */
export function ulid(n) {
  let suffix = '';
  let rest = n;
  for (let i = 0; i < 14; i += 1) {
    suffix = CROCKFORD.charAt(rest % 32) + suffix;
    rest = Math.floor(rest / 32);
  }
  return `01JA33RV${'0'.repeat(4)}${suffix}`.slice(0, 26);
}

const turn = (axis, degrees) => {
  const half = (degrees * Math.PI) / 360;
  return [...axis.map((v) => Number((v * Math.sin(half)).toFixed(6))), Number(Math.cos(half).toFixed(6))];
};
const transform = (translation, rotation = [0, 0, 0, 1], scale = [1, 1, 1]) => ({ translation, rotation, scale });
const ALL = 4294967295;

export function reviewRead() {
  let serial = 100;
  const next = () => ulid((serial += 1));
  const asset = (name, path, kind, extra = {}) => ({ id: next(), name, path, kind, sha256: `review-${name}`, ...extra });
  const assets = [
    asset('crate_stack', 'models/props/crate_stack.glb', 'model'),
    asset('mooring_post', 'models/dock/mooring_post.glb', 'model'),
    asset('harbor_sky', 'textures/sky/harbor_sky_4k.exr', 'texture', { import_settings: { type: 'texture', usage: 'linear' } }),
    asset('crate_albedo', 'textures/props/crate_albedo.png', 'texture', { import_settings: { type: 'texture', usage: 'color' } }),
    asset('harbor_loop', 'audio/harbor_loop.ogg', 'audio'),
  ];
  const [crateModel, postModel, skyTexture, , harborLoop] = assets;
  const scriptId = next();
  const sceneId = next();
  const entities = {};
  const add = (name, components, parent = null) => {
    const id = next();
    entities[id] = { id, name, parent, components };
    return id;
  };

  const environment = add('Environment', { Transform: transform([0, 0, 0]) });
  add('Sun', {
    Transform: transform([0, 40, 0], turn([1, 0, 0], -52)),
    DirectionalLight: { color: [1, 0.92, 0.8], intensity: 98000, shadows: { distance: 120 } },
  }, environment);
  add('Lantern', {
    Transform: transform([3.25, 2.4, -1.5]),
    PointLight: { color: [1, 0.62, 0.3], intensity: 800, range: 12 },
  }, environment);
  add('Gate Spot', {
    Transform: transform([-6, 5.5, 2], turn([1, 0, 0], -35)),
    SpotLight: { color: [0.8, 0.9, 1], intensity: 2400, range: 25, inner_degrees: 18, outer_degrees: 32 },
  }, environment);
  add('Harbor Sky', { EnvironmentLight: { texture: skyTexture.id, intensity: 1.2, rotation_degrees: -35 } }, environment);

  const player = add('Player Character', {
    Transform: transform([2.5, 1.02, -4.75], turn([0, 1, 0], 30)),
    RigidBody: { motion: 'kinematic', gravity_scale: 0, linear_damping: 0, angular_damping: 0, can_sleep: false, ccd: true },
    Collider: {
      shape: { type: 'capsule', half_height: 0.5, radius: 0.32 },
      density: 985, friction: 0, restitution: 0, sensor: false, memberships: 2, filter: 0xfffffffd,
    },
    Velocity: { linear: [1.35, -0.21, 0] },
    Script: {
      source: scriptId,
      props: { walk_speed: 4.5, jump_height: 1.2, coyote_ms: 120, label: 'hero', abilities: ['dash', 'grapple'], tuning: { accel: 30, air_control: 0.4 } },
    },
  });
  add('Follow Camera', {
    Transform: transform([0, 1.6, -3.2], turn([1, 0, 0], -8)),
    Camera: { projection: { kind: 'perspective' }, fov_degrees: 55, near: 0.05, far: 500 },
  }, player);

  const cameras = add('Cameras', { Transform: transform([0, 0, 0]) });
  add('Legacy Camera', { Transform: transform([0, 1.7, -3.5]), Camera: { fov_degrees: 70, near: 0.1, far: 800 } }, cameras);
  add('Map Camera', {
    Transform: transform([0, 24, 0], [-0.707107, 0, 0, 0.707107]),
    Camera: { fov_degrees: 60, near: 0.1, far: 100, projection: { kind: 'orthographic', vertical_size: 24 } },
  }, cameras);
  // Only projections main's schemas/Camera.schema.json declares (perspective, orthographic);
  // Broken Camera below covers a kind the schema lacks.
  add('Close-up Camera', {
    Transform: transform([0, 2, 10]),
    Camera: { fov_degrees: 35, near: 0.05, far: 60, projection: { kind: 'perspective' } },
  }, cameras);
  const broken = add('Broken Camera', {
    Transform: transform([0, 3, 12]),
    Camera: { fov_degrees: 60, near: 0.000001, far: 1e9, projection: { kind: 'fisheye', strength: 2 } },
  }, cameras);

  const navigation = add('Navigation', { Transform: transform([0, 0, 0]) });
  const groundA = next();
  const groundB = next();
  const navmesh = add('Harbor NavMesh', {
    NavigationMesh: {
      settings: {
        min: [-40, -2, -40], max: [40, 8, 40], cell_size: 0, cell_height: 0.1, tile_cells: 64,
        agent_radius: 0.35, agent_height: 1.8, max_climb: 0.4, max_slope_degrees: 40,
      },
      sources: [
        { entity: groundA, geometry: 'collider' },
        { entity: groundB, geometry: 'collider' },
        { entity: player, geometry: 'mesh' },
        { entity: environment, geometry: 'mesh' },
      ],
      links: [
        { id: next(), start: [4, 0, 2], end: [4, 1.2, 4.5], snap_distance: 0.5, bidirectional: true, enabled: true, extra_cost: 0 },
        { id: next(), start: [-2.5, 0, -6], end: [-2.5, -1.5, -9], snap_distance: 0.5, bidirectional: false, enabled: true, extra_cost: 2.5 },
        { id: next(), start: [10, 0.4, 1], end: [12, 0.4, 1], snap_distance: 0.25, bidirectional: true, enabled: false, extra_cost: 0 },
      ],
    },
  }, navigation);
  const grid = (w, h, seed) => Array.from({ length: w * h }, (_, i) => {
    const x = i % w;
    const y = Math.floor(i / w);
    if ((x === Math.floor(w / 3) && y > 2 && y < h - 3) || (y === Math.floor(h / 2) && x > w / 2 && x < w - 2)) return 0;
    return 1 + ((x * 7 + y * 13 + seed) % 5 === 0 ? 4 : (x + y) % 3 === 0 ? 1 : 0);
  });
  add('Courtyard Grid', { NavigationGrid: { dimensions: [24, 16], costs: grid(24, 16, 3) } }, navigation);
  add('District Grid', { NavigationGrid: { dimensions: [256, 256], costs: grid(256, 256, 7) } }, navigation);

  const props = add('Props', { Transform: transform([0, 0, 6]) });
  let partSerial = 9000;
  const part = (translation, shape, rotation = [0, 0, 0, 1]) => ({ id: ulid((partSerial += 1)), translation, rotation, shape });
  add('Crate Stack', {
    Transform: transform([-1.5, 0.5, 2], turn([0, 1, 0], 12.5)),
    MeshRenderer: { mesh: crateModel.id, materials: [next(), next(), next()], cast_shadows: true },
    RigidBody: { motion: 'dynamic', gravity_scale: 1, linear_damping: 0.05, angular_damping: 0.05, can_sleep: true, ccd: false },
    Collider: {
      shape: {
        type: 'compound',
        parts: [
          part([0, 0.5, 0], { type: 'box', half_extents: [0.5, 0.5, 0.5] }),
          part([0.1, 1.5, -0.05], { type: 'box', half_extents: [0.45, 0.45, 0.45] }, turn([0, 1, 0], 15)),
          part([-0.9, 0.35, 0.2], { type: 'sphere', radius: 0.35 }),
          part([0.95, 0.4, 0.1], { type: 'capsule', half_height: 0.25, radius: 0.15 }, turn([0, 0, 1], 90)),
        ],
      },
      density: 420, friction: 0.7, restitution: 0.05, sensor: false, memberships: ALL, filter: ALL,
    },
  }, props);
  const rubble = add('Rubble Pile', {
    Transform: transform([6.5, 0, 3]),
    RigidBody: { motion: 'fixed', gravity_scale: 1, linear_damping: 0, angular_damping: 0, can_sleep: true, ccd: false },
    Collider: {
      shape: {
        type: 'compound',
        parts: Array.from({ length: 64 }, (_, i) => {
          const ring = Math.floor(i / 16);
          const angle = (i % 16) * 22.5;
          const radius = 1.6 - ring * 0.35;
          const at = [Number((Math.cos((angle * Math.PI) / 180) * radius).toFixed(3)), Number((0.15 + ring * 0.28).toFixed(3)), Number((Math.sin((angle * Math.PI) / 180) * radius).toFixed(3))];
          if (i % 5 === 3) return part(at, { type: 'sphere', radius: 0.12 + (i % 3) * 0.04 });
          if (i % 5 === 4) return part(at, { type: 'capsule', half_height: 0.18, radius: 0.08 }, turn([0, 0, 1], 90));
          return part(at, { type: 'box', half_extents: [0.14 + (i % 4) * 0.03, 0.1, 0.12 + (i % 3) * 0.02] }, i === 41 ? [0, 0.4, 0, 0.4] : turn([0, 1, 0], angle));
        }),
      },
      density: 2200, friction: 0.9, restitution: 0, sensor: false, memberships: ALL, filter: ALL,
    },
  }, props);
  add('Audio Emitter', {
    Transform: transform([1, 2, 0]),
    AudioSource: {
      clip: harborLoop.id, gain_db: -6, pan: 0, rate: 1, start_seconds: 0, playing: true, looping: true, streaming: false,
      bus: null, spatial: { min_distance: 1, max_distance: 30 },
    },
  }, props);
  add('Mooring Post with an intentionally long name that keeps going to test truncation', {
    Transform: transform([3, 0, 0]),
    MeshRenderer: { mesh: postModel.id, materials: [next()], cast_shadows: false },
    Collider: { shape: { type: 'capsule', half_height: 0.6, radius: 0.15 }, density: 600, friction: 0.5, restitution: 0, sensor: false, memberships: 1, filter: 6 },
  }, props);
  const malformed = add('Malformed Props', {
    Transform: { translation: ['1', 2], rotation: [0, 0, 0, 1], scale: [1, 1, 1], skew: 3 },
    MeshRenderer: { mesh: crateModel.id, cast_shadows: 'yes' },
    Wobble: { amplitude: 0.2, frequency: 3 },
  }, props);

  const read = {
    status: 'ready',
    project: {
      id: ulid(1),
      name: 'Harbor review (test double)',
      assets: Object.fromEntries(assets.map((a) => [a.id, a])),
      scenes: { [sceneId]: { id: sceneId, name: 'Harbor', entities } },
    },
    revision: 1,
    can_redo: false,
    history: [],
    applied: 0,
    schemas: Object.fromEntries(COMPONENT_SCHEMAS.map((name) => [name, JSON.parse(readFileSync(join(repoRoot, 'schemas', `${name}.schema.json`), 'utf8'))])),
    console: [],
    viewport_error: null,
  };
  const diagnostics = [
    { id: 'rv-1', severity: 'error', message: 'navigation cell_size must be 0.01..10 meters', entity: navmesh, component: 'NavigationMesh', path: '/settings/cell_size' },
    { id: 'rv-2', severity: 'error', message: 'compound part 41 rotation must be a unit quaternion', entity: rubble, component: 'Collider', path: '/shape/parts/41/rotation' },
    { id: 'rv-3', severity: 'error', message: 'unknown camera projection kind `fisheye`', entity: broken, component: 'Camera', path: '/projection/kind' },
    { id: 'rv-4', severity: 'error', message: 'translation must contain 3 finite numbers', entity: malformed, component: 'Transform', path: '/translation' },
    { id: 'rv-5', severity: 'error', message: 'missing field `materials`', entity: malformed, component: 'MeshRenderer', path: '/materials' },
    { id: 'rv-6', severity: 'warning', message: 'Component `Wobble` has no registered schema and is kept as authored.', entity: malformed, component: 'Wobble', path: null },
  ];
  return { read, diagnostics };
}

export function reviewSnapshot() {
  const snapshotFromEngine = loadSnapshotFromEngine();
  const { read, diagnostics } = reviewRead();
  const converted = snapshotFromEngine(read);
  return {
    ...converted,
    diagnostics: [...converted.diagnostics, ...diagnostics],
    viewport: { status: 'not-attached', reason: 'Evidence test double: a browser page has no engine surface.' },
  };
}

/** Init script that installs the double as window.__INCANT_BRIDGE__. */
export function reviewInitScript() {
  const snapshot = reviewSnapshot();
  return `
    (() => {
      const snapshot = ${JSON.stringify(snapshot)};
      const refused = Promise.resolve({ ok: false, error: { code: 'test-double', message: 'Evidence test double: read-only.' } });
      window.__INCANT_BRIDGE__ = { protocolVersion: 1, capabilities: [], label: 'Evidence test double (not engine)', isFixture: false,
        getSnapshot: () => snapshot, subscribe: () => () => {}, dispatch: () => refused, request: () => refused };
    })();
  `;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const snapshot = reviewSnapshot();
  const names = Object.values(snapshot.hierarchy.value.nodes).map((n) => `${n.kind}\t${n.name}`);
  console.log(names.join('\n'));
  console.log(`schemas: ${Object.keys(snapshot.schemas).join(', ')}`);
  console.log(`agent: ${JSON.stringify(snapshot.agent)}`);
}
