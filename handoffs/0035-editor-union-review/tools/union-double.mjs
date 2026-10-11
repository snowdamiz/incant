/**
 * EVIDENCE TEST DOUBLE for handoff 0035 (release-union review). Not engine data, not a bridge.
 *
 * It extends the 0033 double (../../0033-editor-design-review/tools/review-double.mjs)
 * without changing it: the same invented project, converted by the same real compiled
 * bridge function `snapshotFromEngine` (editor/bridge/native.generated.js). Only the
 * union's additions are new:
 *   - schemas: the union's whole native registry (crates/incant_doc schema_registry),
 *     which adds SpriteRenderer, SpriteAtlas and Project to the 0033 set. The registry
 *     list is checked against the Rust source, so a registry change fails loudly;
 *   - entities: two pixel-perfect cameras (the real Camera schema now declares
 *     pixel_perfect) and a "Sprites" group of SpriteRenderer entities;
 *   - assets: two cooked atlas textures (kind "texture", source *.atlas.json, colour
 *     usage, as incant_import registers them) and one ordinary sprite sheet.
 *
 * Diagnostics are SYNTHETIC: the native bridge sends no per-entity validation yet.
 * Each message is copied verbatim from the Rust validators (incant_2d SpriteError,
 * incant_doc validate_component and camera::validate, incant_import sprites.rs), placed
 * at an exact path so the Inspector's presentation of them can be reviewed.
 *
 * Every command is refused; the bridge advertises no capabilities unless a review
 * state asks for one (the import-path reproduction), and even then dispatch refuses.
 */
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import vm from 'node:vm';
import { reviewRead, ulid } from '../../0033-editor-design-review/tools/review-double.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');

/** crates/incant_doc schema_registry() at the union (a20d258), in its declaration order. */
export const UNION_REGISTRY = [
  'Project', 'SpriteAtlas', 'Transform', 'Velocity', 'Script', 'MeshRenderer', 'Camera', 'SpriteRenderer',
  'NavigationGrid', 'NavigationMesh', 'AudioBus', 'AudioSource', 'AudioListener', 'RigidBody', 'Collider',
  'AngularVelocity', 'DirectionalLight', 'PointLight', 'SpotLight', 'EnvironmentLight',
];

/** Fails when the Rust registry no longer lists exactly UNION_REGISTRY. */
function checkRegistry() {
  const source = readFileSync(join(repoRoot, 'crates/incant_doc/src/lib.rs'), 'utf8');
  const start = source.indexOf('pub fn schema_registry()');
  const end = source.indexOf('// Shared inspector annotations', start);
  if (start < 0 || end < 0) throw new Error('schema_registry() not found in crates/incant_doc/src/lib.rs');
  const names = [...source.slice(start, end).matchAll(/\(\s*"([A-Za-z]+)"\.into\(\)/g)].map((m) => m[1]);
  if (names.join(',') !== UNION_REGISTRY.join(',')) {
    throw new Error(`Native registry changed: ${names.join(', ')}. Update UNION_REGISTRY deliberately.`);
  }
}

function loadSnapshotFromEngine() {
  const source = readFileSync(join(repoRoot, 'editor/bridge/native.generated.js'), 'utf8');
  // Same capture as the 0033 double: the IIFE's exports, without installing a bridge.
  const wrapped = source.replace(/\}\)\(\{\}\);\s*$/, '})(globalThis.__exports = {});');
  if (wrapped === source) throw new Error('native.generated.js has an unexpected shape');
  const sandbox = { window: {}, console, globalThis: undefined };
  sandbox.globalThis = sandbox;
  vm.runInNewContext(wrapped, sandbox, { filename: 'native.generated.js' });
  const fn = sandbox.__exports?.snapshotFromEngine;
  if (typeof fn !== 'function') throw new Error('snapshotFromEngine was not exported');
  return fn;
}

const transform = (translation, rotation = [0, 0, 0, 1], scale = [1, 1, 1]) => ({ translation, rotation, scale });

/** Message text copied from the Rust sources named in the header. */
export const ENGINE = {
  frameUlid: 'sprite atlas frame must be a canonical ULID',
  size: 'sprite size must be finite, positive and at most 1000000 metres per axis',
  normalized: 'sprite pivot and tint must be finite normalized values',
  textureMissing: 'sprite texture does not exist',
  textureColor: 'sprite requires a color texture asset',
  rawAtlasRegion: 'raw pixel rectangles address the packed page and can show unrelated content after repacking; use a stable atlas frame ID',
  frameMissing: 'referenced frame does not exist in the cooked atlas',
  pixelCamera: 'pixel-perfect cameras require 1..4096 pixels per metre and a 16..4096 logical resolution of at most 4194304 pixels',
  // serde's own wording for an unknown unit variant of SpriteSampling.
  sampling: 'unknown variant `bilinear`, expected `nearest` or `linear`',
};

export function unionRead() {
  checkRegistry();
  const { read, diagnostics } = reviewRead();
  const scene = Object.values(read.project.scenes)[0];
  const entities = scene.entities;
  const byName = (name) => Object.values(entities).find((entity) => entity.name === name)?.id;
  const assets = read.project.assets;
  // IDs beyond the 0033 double's ranges (its serial starts at 101, its parts at 9001).
  let serial = 20_000;
  const next = () => ulid((serial += 1));
  const asset = (name, path, usage) => {
    const id = next();
    assets[id] = { id, name, path, kind: 'texture', sha256: `review-${name}`, import_settings: { type: 'texture', usage } };
    return id;
  };
  const atlas = asset('harbor_ui', 'sprites/harbor_ui.atlas.json', 'color');
  const gulls = asset('gull_sheet', 'sprites/birds/gull_sheet.png', 'color');
  const longAtlas = asset(
    'harbor_master_notice_board_weathered_signage_atlas_variant_c',
    'sprites/signage/harbor_master/notice_board_weathered_signage_atlas_variant_c.atlas.json',
    'color',
  );
  const linearSky = Object.values(assets).find((a) => a.name === 'harbor_sky')?.id;
  const add = (name, components, parent) => {
    const id = next();
    entities[id] = { id, name, parent, components };
    return id;
  };

  // Cameras: the third projection the real schema declares, well-formed and out of range.
  const cameras = byName('Cameras');
  const pixel = add('Pixel Camera', {
    Transform: transform([0, 4, 16]),
    Camera: { projection: { kind: 'pixel_perfect', pixels_per_unit: 32, reference_resolution: [320, 180] }, fov_degrees: 60, near: 0.1, far: 100 },
  }, cameras);
  const pixelBad = add('Pixel Camera (out of range)', {
    Transform: transform([0, 4, 18]),
    Camera: { projection: { kind: 'pixel_perfect', pixels_per_unit: 0.5, reference_resolution: [320] }, fov_degrees: 60, near: 0.1, far: 100 },
  }, cameras);

  // Sprites: stable atlas frames, a raw rectangle on an atlas, nine-slice, flips, a sprite
  // authored with only its required fields, malformed values and broken references.
  const group = add('Sprites', { Transform: transform([0, 0, -8]) }, null);
  const frame = (n) => ulid(30_000 + n);
  const full = (texture, region, extra = {}) => ({
    texture, region, size: [1, 1], pivot: [0.5, 0.5], tint: [1, 1, 1, 1],
    flip_x: false, flip_y: false, layer: 0, order: 0, sampling: 'nearest', mode: { kind: 'simple' }, ...extra,
  });
  const sign = add('Dock Sign', {
    Transform: transform([-2, 1.2, 0]),
    SpriteRenderer: full(atlas, { frame: frame(1) }, { size: [1.5, 0.75], pivot: [0.5, 0], layer: 2 }),
  }, group);
  const coil = add('Rope Coil', {
    Transform: transform([-0.5, 0.25, 0]),
    SpriteRenderer: full(atlas, [64, 0, 32, 32], { size: [0.5, 0.5], layer: 2, order: 1 }),
  }, group);
  const board = add('Notice Board Panel', {
    Transform: transform([1.5, 1.5, 0]),
    SpriteRenderer: full(atlas, { frame: frame(2) }, {
      size: [3, 2], tint: [0.92, 0.86, 0.74, 1], layer: 1, order: 3, sampling: 'linear',
      mode: { kind: 'nine_slice', border: [12, 12, 10, 14], pixels_per_unit: 32 },
    }),
  }, group);
  add('Gull', {
    Transform: transform([3.5, 4, 0]),
    SpriteRenderer: full(gulls, [0, 0, 48, 32], { size: [0.75, 0.5], pivot: [0.5, 0], tint: [1, 1, 1, 0.85], flip_x: true, layer: 5, order: -2 }),
  }, group);
  // Only the five required fields: serde fills flips, layer, order, sampling and mode.
  add('Lantern Glow', {
    Transform: transform([0.5, 2.2, 0]),
    SpriteRenderer: { texture: atlas, region: { frame: frame(3) }, size: [0.4, 0.4], pivot: [0.5, 0.5], tint: [1, 0.78, 0.45, 0.9] },
  }, group);
  const broken = add('Broken Sprite', {
    Transform: transform([5, 1, 0]),
    SpriteRenderer: {
      texture: atlas, region: { frame: 'lantern-glow' }, size: [-1, 0.5], pivot: [0.5], tint: [1, 0.5, 0.2],
      flip_x: 'yes', flip_y: false, layer: 40000, order: 0, sampling: 'bilinear', mode: { kind: 'tiled', tile: 4 }, opacity: 0.5,
    },
  }, group);
  const missing = add('Missing Texture Sprite', {
    Transform: transform([6.5, 1, 0]),
    SpriteRenderer: full(ulid(99_999), [0, 0, 16, 16], { size: [0.25, 0.25] }),
  }, group);
  const linear = add('Sky Sprite', {
    Transform: transform([8, 1, 0]),
    SpriteRenderer: full(linearSky, [0, 0, 256, 128], { size: [4, 2] }),
  }, group);
  const lost = add('Lost Frame Sprite', {
    Transform: transform([9.5, 1, 0]),
    SpriteRenderer: full(atlas, { frame: frame(99) }, { size: [0.5, 0.5] }),
  }, group);
  add('Harbor master’s weather-beaten notice board sprite with an intentionally long name', {
    Transform: transform([11, 1.5, 0]),
    SpriteRenderer: full(longAtlas, { frame: frame(4) }, {
      size: [2.5, 1.25], tint: [0.8, 0.84, 0.9, 1], layer: -12, order: 2147483647, sampling: 'linear',
      mode: { kind: 'nine_slice', border: [24, 24, 18, 30], pixels_per_unit: 64 },
    }),
  }, group);

  const issue = (id, severity, message, entity, component, path) => ({ id, severity, message, entity, component, path });
  const union = [
    issue('u-1', 'error', ENGINE.pixelCamera, pixelBad, 'Camera', '/projection'),
    issue('u-2', 'warning', ENGINE.rawAtlasRegion, coil, 'SpriteRenderer', '/region'),
    issue('u-3', 'error', ENGINE.frameUlid, broken, 'SpriteRenderer', '/region/frame'),
    issue('u-4', 'error', ENGINE.size, broken, 'SpriteRenderer', '/size/0'),
    issue('u-5', 'error', ENGINE.normalized, broken, 'SpriteRenderer', '/tint'),
    issue('u-6', 'error', ENGINE.sampling, broken, 'SpriteRenderer', '/sampling'),
    issue('u-7', 'error', ENGINE.textureMissing, missing, 'SpriteRenderer', '/texture'),
    issue('u-8', 'error', ENGINE.textureColor, linear, 'SpriteRenderer', '/texture'),
    issue('u-9', 'error', ENGINE.frameMissing, lost, 'SpriteRenderer', '/region'),
  ];

  read.schemas = Object.fromEntries(
    UNION_REGISTRY.map((name) => [name, JSON.parse(readFileSync(join(repoRoot, 'schemas', `${name}.schema.json`), 'utf8'))]),
  );
  return { read, diagnostics: [...diagnostics, ...union], ids: { pixel, pixelBad, sign, coil, board, broken, missing, linear, lost, atlas, gulls, longAtlas } };
}

/**
 * The converted snapshot. Options replace host-owned fields the way a native host
 * publishes them: `provider` (synthetic labels only), `window` chrome, and `assetImport`.
 */
export function unionSnapshot(options = {}) {
  const snapshotFromEngine = loadSnapshotFromEngine();
  const { read, diagnostics } = unionRead();
  const converted = snapshotFromEngine(read);
  return {
    ...converted,
    diagnostics: [...converted.diagnostics, ...diagnostics],
    viewport: { status: 'not-attached', reason: 'Evidence test double: a browser page has no engine surface.' },
    ...(options.provider ? { provider: options.provider } : {}),
    ...(options.window ? { window: options.window } : {}),
    ...(options.assetImport ? { assetImport: options.assetImport } : {}),
  };
}

/** Init script installing the double as window.__INCANT_BRIDGE__. */
export function unionInitScript(options = {}) {
  const snapshot = unionSnapshot(options);
  const capabilities = options.capabilities ?? [];
  return `
    (() => {
      const snapshot = ${JSON.stringify(snapshot)};
      const refused = Promise.resolve({ ok: false, error: { code: 'test-double', message: 'Evidence test double: read-only.' } });
      window.__INCANT_BRIDGE__ = { protocolVersion: 1, capabilities: ${JSON.stringify(capabilities)}, label: 'Evidence test double (not engine)', isFixture: false,
        getSnapshot: () => snapshot, subscribe: () => () => {}, dispatch: () => refused, request: () => refused };
    })();
  `;
}

/**
 * A LIVE variant for the typed-import revision: the same snapshot, but the page can
 * observe what the UI sends and the review can change host state.
 *   - `window.__incantReview.commands` / `.requests` record every dispatch and request
 *     exactly as the UI built it (the request shape is real UI logic);
 *   - `options.results` maps a command type to a MOCKED result ({ ok: true } stands in
 *     for a native host that cooked and committed; nothing is cooked here);
 *   - `window.__incantReview.publish(patch)` replaces host-owned snapshot fields and
 *     notifies subscribers, the way the native host publishes a new state;
 *   - provider requests are simulated like providerFixture.ts (switch, sign out, cancel,
 *     connect to the browser step) only when `options.capabilities` names them.
 */
export function unionLiveInitScript(options = {}) {
  const snapshot = unionSnapshot(options);
  const capabilities = options.capabilities ?? [];
  return `
    (() => {
      let snapshot = ${JSON.stringify(snapshot)};
      const listeners = new Set();
      const results = ${JSON.stringify(options.results ?? {})};
      const refused = { ok: false, error: { code: 'test-double', message: 'Evidence test double: read-only.' } };
      const review = window.__incantReview = { commands: [], requests: [] };
      review.publish = (patch) => { snapshot = { ...snapshot, ...patch }; listeners.forEach((listener) => listener()); };
      const settled = (provider, active) => {
        const account = (provider.accounts ?? []).find((a) => a.id === active);
        const { activeAccount, accountLabel, method, phase, error, ...rest } = provider;
        return account
          ? { ...rest, status: 'connected', method: 'oauth', accountLabel: account.label, activeAccount: account.id }
          : { ...rest, status: 'not-connected' };
      };
      window.__INCANT_BRIDGE__ = {
        protocolVersion: 1, capabilities: ${JSON.stringify(capabilities)}, label: 'Evidence test double (not engine)', isFixture: false,
        getSnapshot: () => snapshot,
        subscribe: (listener) => { listeners.add(listener); return () => listeners.delete(listener); },
        dispatch: (command) => { review.commands.push(command); return Promise.resolve(results[command.type] ?? refused); },
        request: (request) => {
          review.requests.push(request);
          const provider = snapshot.provider;
          if (request.type === 'provider.switch') review.publish({ provider: settled(provider, request.accountId) });
          else if (request.type === 'provider.disconnect') review.publish({ provider: settled(provider, undefined) });
          else if (request.type === 'provider.cancel') review.publish({ provider: settled(provider, provider.activeAccount) });
          else if (request.type === 'provider.connect') review.publish({ provider: { ...provider, status: 'connecting', method: 'oauth', phase: 'browser' } });
          else return Promise.resolve(refused);
          return Promise.resolve({ ok: true });
        },
      };
    })();
  `;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const snapshot = unionSnapshot();
  const nodes = Object.values(snapshot.hierarchy.value.nodes);
  console.log(nodes.map((n) => `${n.kind}\t${n.name}`).join('\n'));
  console.log(`schemas: ${Object.keys(snapshot.schemas).join(', ')}`);
  const sprite = snapshot.schemas.SpriteRenderer;
  console.log(`SpriteRenderer order: ${JSON.stringify(sprite.order ?? null)}`);
  console.log(`SpriteRenderer fields: ${Object.entries(sprite.properties).map(([k, f]) => `${k}:${f.type}`).join(', ')}`);
  const projection = snapshot.schemas.Camera.properties.projection;
  console.log(`Camera projections: ${Object.keys(projection.variants).join(', ')}`);
}
