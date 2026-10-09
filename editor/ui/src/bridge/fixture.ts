/**
 * SAMPLE FIXTURE. NOT ENGINE DATA.
 *
 * A static, read-only stand-in for the engine bridge so the visual shell can be
 * designed, reviewed and screenshotted before editor/bridge exists. It is only
 * used when the page is opened with `?fixture=<variant>` and no bridge was
 * injected. It advertises no capabilities and rejects every command, so it is
 * not a second document model and cannot stand in for the command bus. The
 * shell shows a persistent "Sample fixture" label whenever it is active.
 *
 * Contains no credentials. Every id is a syntactically valid but invented ULID.
 */
import type {
  BridgeError,
  BridgeResult,
  BridgeSnapshot,
  ComponentSchema,
  ComponentValue,
  ConsoleEntry,
  Diagnostic,
  EditorBridge,
  EntityDetail,
  EntityKind,
  HierarchyNode,
  HistoryEntry,
  Ulid,
} from './contract';
import { UI_PROTOCOL_VERSION } from './contract';

export const FIXTURE_VARIANTS = [
  'sample',
  'empty',
  'loading',
  'hierarchy-error',
  'connection-error',
  'project-loading',
  'project-error',
  'large',
] as const;
export type FixtureVariant = (typeof FIXTURE_VARIANTS)[number];

export function isFixtureVariant(value: string): value is FixtureVariant {
  return (FIXTURE_VARIANTS as readonly string[]).includes(value);
}

const CROCKFORD = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';

/** Deterministic, well-formed ULID for fixture entity `n`. */
export function fixtureId(n: number): Ulid {
  let suffix = '';
  let rest = n;
  for (let i = 0; i < 16; i += 1) {
    suffix = CROCKFORD.charAt(rest % 32) + suffix;
    rest = Math.floor(rest / 32);
  }
  return `01J9ZF1XTR${suffix}` as Ulid;
}

const SCHEMAS: Record<string, ComponentSchema> = {
  'incant.Transform': {
    type: 'incant.Transform',
    version: 1,
    title: 'Transform',
    description: 'Position, rotation and scale relative to the parent.',
    order: ['translation', 'rotation', 'scale'],
    properties: {
      translation: {
        type: 'array',
        title: 'Translation',
        items: { type: 'number' },
        minItems: 3,
        maxItems: 3,
        'x-incant-widget': 'vec3',
      },
      rotation: {
        type: 'array',
        title: 'Rotation',
        description: 'Unit quaternion (x, y, z, w).',
        items: { type: 'number' },
        minItems: 4,
        maxItems: 4,
        'x-incant-widget': 'quat',
      },
      scale: {
        type: 'array',
        title: 'Scale',
        items: { type: 'number' },
        minItems: 3,
        maxItems: 3,
        'x-incant-widget': 'vec3',
      },
    },
  },
  'incant.MeshRenderer': {
    type: 'incant.MeshRenderer',
    version: 1,
    title: 'Mesh Renderer',
    order: ['mesh', 'material', 'castShadows', 'lodBias'],
    properties: {
      mesh: { type: 'string', title: 'Mesh', format: 'asset-ref' },
      material: { type: 'string', title: 'Material', format: 'asset-ref' },
      castShadows: { type: 'boolean', title: 'Cast shadows' },
      lodBias: { type: 'number', title: 'LOD bias', minimum: -2, maximum: 2 },
    },
  },
  'incant.Light': {
    type: 'incant.Light',
    version: 1,
    title: 'Light',
    order: ['kind', 'color', 'intensity', 'shadows'],
    properties: {
      kind: { type: 'string', title: 'Kind', enum: ['directional', 'point', 'spot'] },
      color: { type: 'string', title: 'Color', format: 'color' },
      intensity: { type: 'number', title: 'Intensity', minimum: 0, 'x-incant-unit': 'lx' },
      shadows: { type: 'boolean', title: 'Shadows' },
    },
  },
  'incant.Camera': {
    type: 'incant.Camera',
    version: 1,
    title: 'Camera',
    order: ['fov', 'near', 'far'],
    properties: {
      fov: { type: 'number', title: 'Field of view', minimum: 1, maximum: 179, 'x-incant-unit': '°' },
      near: { type: 'number', title: 'Near plane', minimum: 0, 'x-incant-unit': 'm' },
      far: { type: 'number', title: 'Far plane', minimum: 0, 'x-incant-unit': 'm' },
    },
  },
  'incant.Collider': {
    type: 'incant.Collider',
    version: 1,
    title: 'Collider',
    order: ['shape', 'halfExtents', 'isTrigger'],
    properties: {
      shape: { type: 'string', title: 'Shape', enum: ['box', 'sphere', 'capsule'] },
      halfExtents: {
        type: 'array',
        title: 'Half extents',
        items: { type: 'number' },
        minItems: 3,
        maxItems: 3,
        'x-incant-widget': 'vec3',
      },
      isTrigger: { type: 'boolean', title: 'Trigger' },
    },
  },
  'incant.Script': {
    type: 'incant.Script',
    version: 1,
    title: 'Script',
    order: ['source', 'enabled', 'params'],
    properties: {
      source: { type: 'string', title: 'Source', format: 'asset-ref' },
      enabled: { type: 'boolean', title: 'Enabled' },
      params: {
        type: 'object',
        title: 'Parameters',
        properties: {
          range: { type: 'number', title: 'Range', minimum: 0, 'x-incant-unit': 'm' },
          pullSpeed: { type: 'number', title: 'Pull speed', minimum: 0, 'x-incant-unit': 'm/s' },
          curve: { type: 'curve', title: 'Pull curve' },
        },
      },
    },
  },
  'incant.AudioSource': {
    type: 'incant.AudioSource',
    version: 1,
    title: 'Audio Source',
    order: ['clip', 'volume', 'loop', 'spatial'],
    properties: {
      clip: { type: 'string', title: 'Clip', format: 'asset-ref' },
      volume: { type: 'number', title: 'Volume', minimum: 0, maximum: 1 },
      loop: { type: 'boolean', title: 'Loop' },
      spatial: { type: 'boolean', title: 'Spatial' },
    },
  },
};

interface Spec {
  name: string;
  kind: EntityKind;
  components?: ComponentValue[];
  children?: Spec[];
}

const transform = (t: number[], s: number[] = [1, 1, 1]): ComponentValue => ({
  type: 'incant.Transform',
  schemaVersion: 1,
  value: { translation: t, rotation: [0, 0, 0, 1], scale: s },
});
const mesh = (meshRef: string, material: string): ComponentValue => ({
  type: 'incant.MeshRenderer',
  schemaVersion: 1,
  value: { mesh: meshRef, material, castShadows: true, lodBias: 0 },
});
const box = (half: number[]): ComponentValue => ({
  type: 'incant.Collider',
  schemaVersion: 1,
  value: { shape: 'box', halfExtents: half, isTrigger: false },
});
const crate = (name: string, x: number, half = [0.5, 0.5, 0.5]): Spec => ({
  name,
  kind: 'mesh',
  components: [transform([x, 0.5, 2]), mesh('assets/dock_kit/crate.mesh', 'materials/weathered_wood.mat'), box(half)],
});

const SAMPLE_SCENE: Spec = {
  name: 'Dock Prototype',
  kind: 'scene',
  components: [transform([0, 0, 0])],
  children: [
    {
      name: 'Environment',
      kind: 'group',
      components: [transform([0, 0, 0])],
      children: [
        {
          name: 'Sun',
          kind: 'light',
          components: [
            transform([0, 40, 0]),
            {
              type: 'incant.Light',
              schemaVersion: 1,
              value: { kind: 'directional', color: '#FFE9C7', intensity: 98000, shadows: true },
            },
          ],
        },
        { name: 'Sky Dome', kind: 'mesh', components: [transform([0, 0, 0], [500, 500, 500]), mesh('assets/sky/dome.mesh', 'materials/sky_overcast.mat')] },
        { name: 'Sea Surface', kind: 'mesh', components: [transform([0, -0.2, 0], [200, 1, 200]), mesh('assets/water/plane.mesh', 'materials/sea.mat')] },
      ],
    },
    {
      name: 'Player Spawn',
      kind: 'group',
      components: [transform([0, 0, -4])],
      children: [
        {
          name: 'Player',
          kind: 'prefab',
          components: [transform([0, 0, 0])],
          children: [
            {
              name: 'Camera Rig',
              kind: 'camera',
              components: [
                transform([0, 1.7, -3.5]),
                { type: 'incant.Camera', schemaVersion: 1, value: { fov: 70, near: 0.1, far: 800 } },
              ],
            },
            {
              name: 'Grapple Controller',
              kind: 'script',
              components: [
                {
                  type: 'incant.Script',
                  schemaVersion: 1,
                  value: {
                    source: 'scripts/grapple.ts',
                    enabled: true,
                    params: { range: 18, pullSpeed: '12', curve: { keys: 3 } },
                  },
                },
                { type: 'incant.ExperimentalRope', schemaVersion: 0, value: { segments: 24 } },
              ],
            },
          ],
        },
      ],
    },
    {
      name: 'Dock',
      kind: 'group',
      components: [transform([0, 0, 6])],
      children: [
        { name: 'Pier Planks', kind: 'mesh', components: [transform([0, 0, 0]), mesh('assets/dock_kit/pier_planks.mesh', 'materials/weathered_wood.mat')] },
        crate('Crate 01', -2),
        crate('Crate 02', -1),
        crate('Crate 03', 0, [0.5, -0.5, 0.5]),
        crate('Crate 04', 1),
        {
          name: 'Mooring Post with an intentionally long name to test truncation',
          kind: 'mesh',
          components: [transform([3, 0, 0]), mesh('assets/dock_kit/post.mesh', 'materials/weathered_wood.mat')],
        },
      ],
    },
    {
      name: 'Harbor Ambience',
      kind: 'audio',
      components: [
        transform([0, 2, 0]),
        {
          type: 'incant.AudioSource',
          schemaVersion: 1,
          value: { clip: 'audio/harbor_loop.ogg', volume: 0.6, loop: true, spatial: false },
        },
      ],
    },
  ],
};

function largeScene(groups: number, perGroup: number): Spec {
  return {
    name: 'Stress Scene',
    kind: 'scene',
    children: Array.from({ length: groups }, (_, g) => ({
      name: `Group ${String(g + 1).padStart(3, '0')}`,
      kind: 'group',
      children: Array.from({ length: perGroup }, (_, i) => ({
        name: `Entity ${String(g + 1).padStart(3, '0')}.${String(i + 1).padStart(3, '0')}`,
        kind: 'mesh',
        components: [transform([i, 0, g])],
      })),
    })),
  };
}

function flatten(root: Spec) {
  const nodes: Record<string, HierarchyNode> = {};
  const entities: Record<string, EntityDetail> = {};
  const byName = new Map<string, Ulid>();
  let next = 1;
  const visit = (spec: Spec, parent: Ulid | null): Ulid => {
    const id = fixtureId(next);
    next += 1;
    const children = (spec.children ?? []).map((child) => visit(child, id));
    nodes[id] = { id, name: spec.name, kind: spec.kind, parent, children };
    entities[id] = { id, name: spec.name, kind: spec.kind, components: spec.components ?? [] };
    byName.set(spec.name, id);
    return id;
  };
  const rootId = visit(root, null);
  return { roots: [rootId], nodes, entities, byName };
}

const AT = '2026-10-08T21:00:00Z';
const at = (minutes: number) => new Date(Date.parse(AT) + minutes * 60_000).toISOString();

function sampleDiagnostics(byName: Map<string, Ulid>): Diagnostic[] {
  const id = (name: string) => byName.get(name) ?? null;
  return [
    {
      id: 'd1',
      severity: 'error',
      message: 'Half extents must be positive; y is -0.5.',
      entity: id('Crate 03'),
      component: 'incant.Collider',
      path: '/halfExtents/1',
    },
    {
      id: 'd2',
      severity: 'error',
      message: 'Script asset not found: scripts/grapple.ts',
      entity: id('Grapple Controller'),
      component: 'incant.Script',
      path: '/source',
    },
    {
      id: 'd3',
      severity: 'warning',
      message: 'Expected a number for pullSpeed; got the string "12".',
      entity: id('Grapple Controller'),
      component: 'incant.Script',
      path: '/params/pullSpeed',
    },
    {
      id: 'd4',
      severity: 'warning',
      message: 'Mesh has no LOD 2; distant rendering will use LOD 1.',
      entity: id('Pier Planks'),
      component: 'incant.MeshRenderer',
      path: '/mesh',
    },
    {
      id: 'd5',
      severity: 'info',
      message: 'Project has 2 unused materials.',
      entity: null,
      component: null,
      path: null,
    },
  ];
}

const SAMPLE_HISTORY: HistoryEntry[] = [
  { transaction: fixtureId(9001), description: 'Import dock_kit.glb (6 meshes)', origin: { kind: 'import', source: 'dock_kit.glb' }, at: at(0) },
  { transaction: fixtureId(9002), description: 'Create Crate 01 to Crate 04', origin: { kind: 'user' }, at: at(3) },
  {
    transaction: fixtureId(9003),
    description: 'Add Collider to 4 crates',
    origin: { kind: 'agent', model: 'sample-model', conversation: 'sample-conversation' },
    at: at(5),
  },
  { transaction: fixtureId(9004), description: 'Set Grapple Controller range to 18 m', origin: { kind: 'script', script: 'tools/tune.ts' }, at: at(8) },
  { transaction: fixtureId(9005), description: "Rename 'Spawn' to 'Player Spawn'", origin: { kind: 'user' }, at: at(11) },
];

const SAMPLE_CONSOLE: ConsoleEntry[] = [
  { id: 'c1', level: 'info', source: 'project', message: 'Opened sample fixture project "Dock Prototype".', at: at(0) },
  { id: 'c2', level: 'debug', source: 'schema', message: 'Registered 7 component schemas.', at: at(0) },
  { id: 'c3', level: 'warn', source: 'assets', message: 'pier_planks.mesh: LOD 2 missing.', at: at(1) },
  { id: 'c4', level: 'error', source: 'script', message: 'Cannot resolve scripts/grapple.ts referenced by Grapple Controller.', at: at(2) },
  { id: 'c5', level: 'info', source: 'viewport', message: 'Native viewport not attached (Spike 1 pending).', at: at(2) },
];

const NO_VIEWPORT = {
  status: 'not-attached',
  reason: 'Sample data has no native surface. Open a project in the Incant editor app to attach the wgpu viewport.',
} as const;

function baseSnapshot(): Omit<BridgeSnapshot, 'connection' | 'hierarchy'> {
  return {
    schemas: {},
    entities: {},
    diagnostics: [],
    history: { entries: [], applied: 0 },
    console: [],
    provider: { status: 'not-connected', provider: 'openai' },
    agent: { status: 'unavailable', reason: 'Sign in with ChatGPT to use the agent.' },
    viewport: NO_VIEWPORT,
  };
}

const PROJECT = { id: fixtureId(0), name: 'Dock Prototype' };

export function fixtureSnapshot(variant: FixtureVariant): BridgeSnapshot {
  switch (variant) {
    case 'sample': {
      const { roots, nodes, entities, byName } = flatten(SAMPLE_SCENE);
      return {
        ...baseSnapshot(),
        connection: { status: 'ready', project: PROJECT },
        hierarchy: { status: 'ready', value: { roots, nodes } },
        schemas: SCHEMAS,
        entities,
        diagnostics: sampleDiagnostics(byName),
        history: { entries: SAMPLE_HISTORY, applied: 4 },
        console: SAMPLE_CONSOLE,
      };
    }
    case 'large': {
      const { roots, nodes, entities } = flatten(largeScene(40, 50));
      return {
        ...baseSnapshot(),
        connection: { status: 'ready', project: { id: fixtureId(0), name: 'Stress Scene' } },
        hierarchy: { status: 'ready', value: { roots, nodes } },
        schemas: SCHEMAS,
        entities,
      };
    }
    case 'empty':
      return {
        ...baseSnapshot(),
        connection: { status: 'ready', project: { id: fixtureId(0), name: 'Untitled Project' } },
        hierarchy: { status: 'ready', value: { roots: [], nodes: {} } },
        schemas: SCHEMAS,
      };
    case 'loading':
      return {
        ...baseSnapshot(),
        connection: { status: 'ready', project: PROJECT },
        hierarchy: { status: 'loading' },
      };
    case 'hierarchy-error':
      return {
        ...baseSnapshot(),
        connection: { status: 'ready', project: PROJECT },
        hierarchy: {
          status: 'error',
          error: {
            code: 'document.parse',
            message: 'scenes/dock.scene.ron line 214: expected "}" but found end of file.',
          },
        },
        console: [
          { id: 'e1', level: 'error', source: 'document', message: 'Failed to load scenes/dock.scene.ron: unexpected end of file at line 214.', at: at(0) },
        ],
      };
    case 'connection-error':
      return failedProject({ code: 'bridge.disconnected', message: 'The editor process stopped responding.' });
    // Mirrors editor/bridge/native.ts snapshotFromEngine() for an engine `loading` response.
    case 'project-loading':
      return {
        ...baseSnapshot(),
        connection: { status: 'connecting' },
        hierarchy: { status: 'loading' },
        agent: { status: 'unavailable', reason: 'Loading project.' },
        viewport: { status: 'not-attached', reason: 'Loading project.' },
      };
    case 'project-error':
      return failedProject({
        code: 'project.timeout',
        message: "Project loading timed out. Check the file's availability and folder permissions, then reopen the project.",
      });
  }
}

/** Mirrors snapshotFromEngine() for an engine `error` response: no entities, no history. */
function failedProject(error: BridgeError): BridgeSnapshot {
  return {
    ...baseSnapshot(),
    connection: { status: 'error', error },
    hierarchy: { status: 'error', error },
    diagnostics: [{ id: 'project-load', severity: 'error', message: error.message, entity: null, component: null, path: null }],
    agent: { status: 'unavailable', reason: error.message },
    viewport: { status: 'error', error },
  };
}

const READ_ONLY: BridgeResult = {
  ok: false,
  error: { code: 'fixture.read-only', message: 'The sample fixture is read-only and has no command bus.' },
};

export function createFixtureBridge(variant: FixtureVariant): EditorBridge {
  const snapshot = fixtureSnapshot(variant);
  return {
    protocolVersion: UI_PROTOCOL_VERSION,
    capabilities: [],
    label: `Sample fixture: ${variant}`,
    isFixture: true,
    getSnapshot: () => snapshot,
    subscribe: () => () => undefined,
    dispatch: () => Promise.resolve(READ_ONLY),
    request: () => Promise.resolve(READ_ONLY),
  };
}
