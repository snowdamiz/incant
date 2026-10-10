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
  ProjectAsset,
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

/** Native PrimitiveColliderShape after bridge resolution: a compound part's shape. */
const PRIMITIVE_SHAPE = {
  type: 'tagged-union',
  discriminator: 'type',
  variants: {
    box: {
      type: 'object',
      properties: {
        half_extents: { type: 'array', items: { type: 'number' }, minItems: 3, maxItems: 3, 'x-incant-unit': 'm', optional: false },
        type: { type: 'string', enum: ['box'], optional: false },
      },
    },
    sphere: {
      type: 'object',
      properties: {
        radius: { type: 'number', 'x-incant-unit': 'm', optional: false },
        type: { type: 'string', enum: ['sphere'], optional: false },
      },
    },
    capsule: {
      type: 'object',
      description: 'Capsule along local Y; half_height excludes the hemispherical ends.',
      properties: {
        half_height: { type: 'number', 'x-incant-unit': 'm', optional: false },
        radius: { type: 'number', 'x-incant-unit': 'm', optional: false },
        type: { type: 'string', enum: ['capsule'], optional: false },
      },
    },
  },
} as const;

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
      // Mirrors the native DirectionalLight.shadows field after bridge resolution:
      // a nullable reference to { distance }, optional on the component.
      shadows: {
        type: 'object',
        title: 'Shadows',
        description: 'Cascaded shadow settings. Absent or null: this light casts no shadows.',
        optional: true,
        nullable: true,
        properties: {
          distance: {
            type: 'number',
            title: 'Distance',
            description: 'View distance covered by the shadow cascades.',
            minimum: 0.01,
            maximum: 10000,
            'x-incant-unit': 'm',
            optional: false,
          },
        },
      },
    },
  },
  // Mirror of the native Camera schema after bridge resolution: bare type name, units
  // and the projection default from Rust metadata, no titles (the Inspector's Camera
  // profile supplies labels). CameraFields.test.tsx checks it against
  // snapshotFromEngine over schemas/Camera.schema.json.
  Camera: {
    type: 'Camera',
    version: 1,
    title: 'Camera',
    order: ['projection', 'fov_degrees', 'near', 'far'],
    properties: {
      far: { type: 'number', format: 'double', 'x-incant-unit': 'm', optional: false },
      fov_degrees: {
        type: 'number',
        format: 'double',
        description: 'Vertical perspective field of view, retained when switching projection.',
        'x-incant-unit': '°',
        optional: false,
      },
      near: { type: 'number', format: 'double', 'x-incant-unit': 'm', optional: false },
      projection: {
        type: 'tagged-union',
        description: 'Perspective, or orthographic with parallel view rays and a fixed world-space height.',
        default: { kind: 'perspective' },
        discriminator: 'kind',
        variants: {
          perspective: {
            type: 'object',
            additionalProperties: false,
            properties: { kind: { type: 'string', const: 'perspective', enum: ['perspective'], optional: false } },
            required: ['kind'],
          },
          orthographic: {
            type: 'object',
            additionalProperties: false,
            properties: {
              kind: { type: 'string', const: 'orthographic', enum: ['orthographic'], optional: false },
              vertical_size: {
                type: 'number',
                format: 'double',
                description: 'Visible world-space height. Width follows the output aspect ratio.',
                'x-incant-unit': 'm',
                optional: false,
              },
            },
            required: ['kind', 'vertical_size'],
          },
        },
        optional: true,
      },
    },
  } as ComponentSchema,
  // Physics: mirrors of the native RigidBody / Collider / AngularVelocity schemas after
  // bridge resolution, including the registry's order, units and mask widget (bare
  // type names, no titles). PhysicsFields.test.tsx checks them against
  // snapshotFromEngine over schemas/*.schema.json.
  RigidBody: {
    type: 'RigidBody',
    version: 1,
    title: 'RigidBody',
    order: ['motion', 'gravity_scale', 'linear_damping', 'angular_damping', 'can_sleep', 'ccd'],
    properties: {
      motion: { type: 'string', enum: ['fixed', 'dynamic', 'kinematic'], optional: false },
      gravity_scale: { type: 'number', minimum: -100, maximum: 100, optional: false },
      linear_damping: {
        type: 'number',
        description: 'Linear velocity damping rate, in inverse seconds.',
        minimum: 0,
        maximum: 100,
        'x-incant-unit': '1/s',
        optional: false,
      },
      angular_damping: {
        type: 'number',
        description: 'Angular velocity damping rate, in inverse seconds.',
        minimum: 0,
        maximum: 100,
        'x-incant-unit': '1/s',
        optional: false,
      },
      can_sleep: { type: 'boolean', optional: false },
      ccd: { type: 'boolean', optional: false },
    },
  },
  Collider: {
    type: 'Collider',
    version: 1,
    title: 'Collider',
    order: ['shape', 'density', 'friction', 'restitution', 'sensor', 'memberships', 'filter'],
    properties: {
      shape: {
        type: 'tagged-union',
        discriminator: 'type',
        optional: false,
        variants: {
          box: {
            type: 'object',
            properties: {
              half_extents: {
                type: 'array',
                items: { type: 'number' },
                minItems: 3,
                maxItems: 3,
                'x-incant-unit': 'm',
                optional: false,
              },
              type: { type: 'string', enum: ['box'], optional: false },
            },
          },
          sphere: {
            type: 'object',
            properties: {
              radius: { type: 'number', 'x-incant-unit': 'm', optional: false },
              type: { type: 'string', enum: ['sphere'], optional: false },
            },
          },
          capsule: {
            type: 'object',
            description: 'Capsule along local Y; half_height excludes the hemispherical ends.',
            properties: {
              half_height: { type: 'number', 'x-incant-unit': 'm', optional: false },
              radius: { type: 'number', 'x-incant-unit': 'm', optional: false },
              type: { type: 'string', enum: ['capsule'], optional: false },
            },
          },
          compound: {
            type: 'object',
            description: 'One body and material shared by a bounded union of local primitive parts.',
            properties: {
              parts: {
                type: 'array',
                minItems: 1,
                maxItems: 64,
                optional: false,
                items: {
                  type: 'object',
                  properties: {
                    id: {
                      type: 'string',
                      description: 'Stable ULID, unique within this collider, retained when editing a part.',
                      optional: false,
                    },
                    rotation: {
                      type: 'array',
                      description: 'Unit quaternion (x, y, z, w), relative to the collider entity.',
                      items: { type: 'number' },
                      minItems: 4,
                      maxItems: 4,
                      optional: false,
                    },
                    shape: { ...PRIMITIVE_SHAPE, optional: false },
                    translation: {
                      type: 'array',
                      description: "Offset in the collider entity's local frame, in meters.",
                      items: { type: 'number' },
                      minItems: 3,
                      maxItems: 3,
                      'x-incant-unit': 'm',
                      optional: false,
                    },
                  },
                },
              },
              type: { type: 'string', enum: ['compound'], optional: false },
            },
          },
        },
      },
      density: { type: 'number', minimum: 0.001, maximum: 100000, 'x-incant-unit': 'kg/m³', optional: false },
      filter: { type: 'integer', minimum: 0, maximum: 4294967295, 'x-incant-widget': 'collision-mask', optional: false },
      friction: { type: 'number', minimum: 0, maximum: 10, optional: false },
      memberships: {
        type: 'integer',
        minimum: 0,
        maximum: 4294967295,
        'x-incant-widget': 'collision-mask',
        description: 'Collision requires both membership/filter intersections to be nonzero.',
        optional: false,
      },
      restitution: { type: 'number', minimum: 0, maximum: 1, optional: false },
      sensor: { type: 'boolean', optional: false },
    },
  },
  AngularVelocity: {
    type: 'AngularVelocity',
    version: 1,
    title: 'AngularVelocity',
    properties: {
      angular: {
        type: 'array',
        items: { type: 'number' },
        minItems: 3,
        maxItems: 3,
        'x-incant-unit': 'rad/s',
        optional: false,
      },
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
const ALL_GROUPS = 4294967295;
const collider = (shape: Record<string, unknown>, extra: Record<string, unknown> = {}): ComponentValue => ({
  type: 'Collider',
  schemaVersion: 1,
  value: { shape, density: 1000, friction: 0.5, restitution: 0, sensor: false, memberships: ALL_GROUPS, filter: ALL_GROUPS, ...extra },
});
const body = (motion: string, extra: Record<string, unknown> = {}): ComponentValue => ({
  type: 'RigidBody',
  schemaVersion: 1,
  value: { motion, gravity_scale: 1, linear_damping: 0, angular_damping: 0.05, can_sleep: true, ccd: true, ...extra },
});
/** Unit quaternion for a rotation of `degrees` about a unit axis. */
const turn = (axis: [number, number, number], degrees: number): number[] => {
  const half = (degrees * Math.PI) / 360;
  return [...axis.map((v) => Number((v * Math.sin(half)).toFixed(6))), Number(Math.cos(half).toFixed(6))];
};
let partSerial = 0;
const part = (translation: number[], shape: Record<string, unknown>, rotation: number[] = [0, 0, 0, 1]) => ({
  id: fixtureId(30000 + (partSerial += 1)),
  translation,
  rotation,
  shape,
});
const box = (x: number, y: number, z: number) => ({ type: 'box', half_extents: [x, y, z] });
const compound = (parts: unknown[], extra: Record<string, unknown> = {}) => collider({ type: 'compound', parts }, extra);

/** 64 parts (the maximum) in a deterministic heap: boxes, spheres and capsules. */
function rubbleParts() {
  return Array.from({ length: 64 }, (_, i) => {
    const ring = Math.floor(i / 16);
    const angle = (i % 16) * 22.5;
    const radius = 1.6 - ring * 0.35;
    const at = [
      Number((Math.cos((angle * Math.PI) / 180) * radius).toFixed(3)),
      Number((0.15 + ring * 0.28).toFixed(3)),
      Number((Math.sin((angle * Math.PI) / 180) * radius).toFixed(3)),
    ];
    const kind = i % 5;
    if (kind === 3) return part(at, { type: 'sphere', radius: 0.12 + (i % 3) * 0.04 });
    if (kind === 4) return part(at, { type: 'capsule', half_height: 0.18, radius: 0.08 }, turn([0, 0, 1], 90));
    return part(at, box(0.14 + (i % 4) * 0.03, 0.1, 0.12 + (i % 3) * 0.02), turn([0, 1, 0], angle));
  });
}

/** One plausible top rail followed by one malformed part of each kind. */
function railingParts() {
  const rail = part([0, 0.9, 0], box(1.2, 0.05, 0)); // z half extent 0: out of range
  return [
    rail,
    part([-1.1, 0.45, 0], box(0.05, 0.45, 0.05), [0, 0, 0.5, 0.5]), // not a unit quaternion
    part([1.1, 0.45, 0], { half_extents: [0.05, 0.45, 0.05] }), // shape without a type
    { ...part([0, 0.45, 0], { type: 'cylinder', half_height: 0.45, radius: 0.04 }), id: rail.id }, // unknown tag, duplicate ID
    { ...part([0, 0.2, 0], box(1.2, 0.04, 0.04)), material: 'oak' }, // field outside the schema
    'post-04', // not an object
  ];
}

const crate = (name: string, x: number, half = [0.5, 0.5, 0.5], more: ComponentValue[] = []): Spec => ({
  name,
  kind: 'mesh',
  components: [
    transform([x, 0.5, 2]),
    mesh('assets/dock_kit/crate.mesh', 'materials/weathered_wood.mat'),
    body('dynamic'),
    collider({ type: 'box', half_extents: half }, { density: 420, friction: 0.7 }),
    ...more,
  ],
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
              value: { kind: 'directional', color: '#FFE9C7', intensity: 98000, shadows: { distance: 120 } },
            },
          ],
        },
        {
          name: 'Sky Fill',
          kind: 'light',
          components: [
            transform([0, 30, 0]),
            {
              type: 'incant.Light',
              schemaVersion: 1,
              // Shadows omitted: an authored, valid "disabled" state.
              value: { kind: 'directional', color: '#BFD4FF', intensity: 12000 },
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
                // Legacy document: no projection field, so the engine uses perspective.
                { type: 'Camera', schemaVersion: 1, value: { fov_degrees: 70, near: 0.1, far: 800 } },
              ],
            },
            {
              name: 'Map Camera',
              kind: 'camera',
              components: [
                transform([0, 24, 0]),
                {
                  type: 'Camera',
                  schemaVersion: 1,
                  value: { fov_degrees: 60, near: 0.1, far: 100, projection: { kind: 'orthographic', vertical_size: 8 } },
                },
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
        {
          name: 'Pier Planks',
          kind: 'mesh',
          components: [
            transform([0, 0, 0]),
            mesh('assets/dock_kit/pier_planks.mesh', 'materials/weathered_wood.mat'),
            // Deliberately unknown shape tag: the Inspector must say so, not pick a default.
            collider({ type: 'cylinder', half_height: 0.1, radius: 4 }, { friction: 0.9 }),
          ],
        },
        crate('Crate 01', -2, [0.5, 0.5, 0.5], [
          { type: 'AngularVelocity', schemaVersion: 1, value: { angular: [0, 1.5708, 0] } },
        ]),
        crate('Crate 02', -1),
        crate('Crate 03', 0, [0.5, -0.5, 0.5]),
        crate('Crate 04', 1),
        {
          name: 'Mooring Post with an intentionally long name to test truncation',
          kind: 'mesh',
          components: [
            transform([3, 0, 0]),
            mesh('assets/dock_kit/post.mesh', 'materials/weathered_wood.mat'),
            collider({ type: 'capsule', half_height: 0.6, radius: 0.15 }, { memberships: 1, filter: 6 }),
          ],
        },
        {
          name: 'Stone Arch',
          kind: 'mesh',
          components: [
            transform([-4, 0, 1]),
            mesh('assets/dock_kit/arch.mesh', 'materials/harbor_stone.mat'),
            body('fixed'),
            compound([
              part([-1.1, 1.1, 0], box(0.3, 1.1, 0.4)),
              part([1.1, 1.1, 0], box(0.3, 1.1, 0.4)),
              part([0, 2.4, 0], box(1.4, 0.2, 0.4)),
              part([0, 2.62, 0], box(0.18, 0.18, 0.42), turn([0, 0, 1], 45)),
            ], { density: 2400, friction: 0.8 }),
          ],
        },
        {
          name: 'Handcart',
          kind: 'mesh',
          components: [
            transform([-1.5, 1.2, -1]),
            mesh('assets/dock_kit/handcart.mesh', 'materials/weathered_wood.mat'),
            body('dynamic'),
            compound([
              part([0, 0.35, 0], box(0.6, 0.06, 0.4)),
              part([-0.3, 0.18, -0.46], { type: 'sphere', radius: 0.18 }),
              part([-0.3, 0.18, 0.46], { type: 'sphere', radius: 0.18 }),
              part([0.82, 0.55, 0], { type: 'capsule', half_height: 0.32, radius: 0.04 }, turn([0, 0, 1], -60)),
            ], { density: 520, friction: 0.6, memberships: 2, filter: 0xfffffffd }),
          ],
        },
        {
          name: 'Broken Railing',
          kind: 'mesh',
          components: [
            transform([4.5, 0, -1]),
            mesh('assets/dock_kit/railing.mesh', 'materials/weathered_wood.mat'),
            body('fixed'),
            // Deliberately malformed parts: each one must be shown plainly, never repaired.
            compound(railingParts()),
          ],
        },
        {
          name: 'Rubble Pile',
          kind: 'mesh',
          components: [
            transform([6.5, 0, 3]),
            mesh('assets/dock_kit/rubble.mesh', 'materials/harbor_stone.mat'),
            body('fixed'),
            compound(rubbleParts(), { density: 2200, friction: 0.9 }),
          ],
        },
        {
          name: 'Wave Trigger',
          kind: 'group',
          components: [
            transform([0, 0.5, 1]),
            collider({ type: 'sphere', radius: 1.25 }, { sensor: true, memberships: 4, filter: 3, density: 1 }),
          ],
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
      component: 'Collider',
      path: '/shape/half_extents/1',
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
      id: 'd6',
      severity: 'error',
      message: 'Unknown collider shape `cylinder`; expected `box`, `sphere` or `capsule`.',
      entity: id('Pier Planks'),
      component: 'Collider',
      path: '/shape/type',
    },
    // Engine wording (incant_doc collider_shapes.rs), addressed to the exact part path.
    {
      id: 'd7',
      severity: 'error',
      message: 'compound part 0: collider dimensions must be 0.001..10000 meters',
      entity: id('Broken Railing'),
      component: 'Collider',
      path: '/shape/parts/0/shape/half_extents/2',
    },
    {
      id: 'd8',
      severity: 'error',
      message: 'compound part 1 rotation must be a unit quaternion',
      entity: id('Broken Railing'),
      component: 'Collider',
      path: '/shape/parts/1/rotation',
    },
    {
      id: 'd9',
      severity: 'error',
      message: 'compound part 3 requires a unique valid ULID',
      entity: id('Broken Railing'),
      component: 'Collider',
      path: '/shape/parts/3/id',
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

/** Sample asset rows. Fingerprints are invented; nothing was cooked. */
const SAMPLE_ASSETS: ProjectAsset[] = [
  { id: fixtureId(7001), name: 'dock_kit', path: 'models/dock_kit.glb', kind: 'model', fingerprint: 'sample-fingerprint-dock-kit' },
  { id: fixtureId(7002), name: 'crate', path: 'models/props/crate.gltf', kind: 'model', fingerprint: 'sample-fingerprint-crate' },
  { id: fixtureId(7003), name: 'Crate albedo', path: 'textures/crate_albedo.png', kind: 'texture', fingerprint: 'sample-fingerprint-albedo', textureUsage: 'color' },
  { id: fixtureId(7004), name: 'crate_normal', path: 'textures/crate_normal.png', kind: 'texture', fingerprint: 'sample-fingerprint-normal', textureUsage: 'normal' },
  { id: fixtureId(7005), name: 'pier_roughness', path: 'textures/pier_roughness.jpg', kind: 'texture', fingerprint: 'sample-fingerprint-rough', textureUsage: 'linear' },
  { id: fixtureId(7006), name: 'harbor_sky', path: 'textures/sky/harbor_sky_4k.exr', kind: 'texture', fingerprint: 'sample-fingerprint-sky', textureUsage: 'linear' },
  {
    id: fixtureId(7007),
    name: 'weathered_pier_planks_with_moss_and_salt_stains_variant_b',
    path: 'textures/environment/harbor/district_02/surfaces/wood/weathered_pier_planks_with_moss_and_salt_stains_variant_b_albedo.png',
    kind: 'texture',
    fingerprint: 'sample-fingerprint-long',
    textureUsage: 'color',
  },
];

const READ_ONLY_IMPORT = { available: false, reason: 'The sample fixture is read-only.' } as const;

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
        assets: { status: 'ready', value: SAMPLE_ASSETS },
        assetImport: READ_ONLY_IMPORT,
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
        assets: { status: 'ready', value: largeAssets(240) },
        assetImport: READ_ONLY_IMPORT,
      };
    }
    case 'empty':
      return {
        ...baseSnapshot(),
        connection: { status: 'ready', project: { id: fixtureId(0), name: 'Untitled Project' } },
        hierarchy: { status: 'ready', value: { roots: [], nodes: {} } },
        schemas: SCHEMAS,
        assets: { status: 'ready', value: [] },
        assetImport: READ_ONLY_IMPORT,
      };
    case 'loading':
      return {
        ...baseSnapshot(),
        connection: { status: 'ready', project: PROJECT },
        hierarchy: { status: 'loading' },
        assets: { status: 'loading' },
        assetImport: READ_ONLY_IMPORT,
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
        assets: { status: 'loading' },
        assetImport: { available: false, reason: 'Loading project.' },
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
    assets: { status: 'error', error },
    assetImport: { available: false, reason: error.message },
    diagnostics: [{ id: 'project-load', severity: 'error', message: error.message, entity: null, component: null, path: null }],
    agent: { status: 'unavailable', reason: error.message },
    viewport: { status: 'error', error },
  };
}

function largeAssets(count: number): ProjectAsset[] {
  const folders = ['models/props', 'models/architecture', 'textures/props', 'textures/terrain'];
  return Array.from({ length: count }, (_, index) => {
    const folder = folders[index % folders.length]!;
    const model = folder.startsWith('models');
    const name = `${model ? 'prop' : 'surface'}_${String(index).padStart(3, '0')}`;
    return {
      id: fixtureId(20000 + index),
      name,
      path: `${folder}/${name}.${model ? 'glb' : 'png'}`,
      kind: model ? 'model' : 'texture',
      fingerprint: `sample-fingerprint-${index}`,
      ...(model ? {} : { textureUsage: index % 3 === 0 ? ('normal' as const) : ('color' as const) }),
    };
  });
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
