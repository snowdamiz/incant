/**
 * Inspector presentation profiles: section captions, and labels or tooltips the
 * engine schema does not carry. Units, field order, defaults and the
 * collision-mask widget come from the schema (x-incant-unit, order, default,
 * x-incant-widget), not from here.
 *
 * Presentation only. Nothing here validates, converts, reorders or hides a
 * value, and the engine schema wins: a schema `title` or `description`
 * overrides the matching profile entry, and a schema `order` is kept exactly.
 */

/** A short visible tag beside a value, with a full sentence for assistive tech and hover. */
export interface FieldStatus {
  readonly short: string;
  readonly long: string;
}

export interface FieldHint {
  readonly title?: string;
  readonly description?: string;
  /** Display names for a tagged union's variants, keyed by tag. Unlisted tags show as authored. */
  readonly variants?: Readonly<Record<string, string>>;
  /**
   * Sentence announced with the "Default" tag when an omitted optional field is
   * shown as its schema `default`. The value itself always comes from the schema.
   */
  readonly omittedNote?: string;
  /**
   * A tag that depends on sibling values in the same component, such as a field
   * the current mode ignores. Returns null when there is nothing to say or the
   * siblings are not well-formed (it never guesses or repairs).
   */
  readonly status?: (component: Readonly<Record<string, unknown>>) => FieldStatus | null;
  /** Presents a plain string as a stable identifier (monospace, never a reference). */
  readonly identifier?: boolean;
  /** Abbreviation in one-line list summaries ("r" for radius); the full label is spoken. */
  readonly short?: string;
  /** Row order for an object without a schema order (list item detail). */
  readonly order?: readonly string[];
  /**
   * Names for the entries of a fixed-length number list that is not a spatial
   * vector, such as a width and height. `short` is the visible prefix and `name`
   * the accessible name. A list of any other length is shown as a mismatch.
   */
  readonly axes?: readonly AxisName[];
}

export interface AxisName {
  readonly short: string;
  readonly name: string;
}

export interface FieldSection {
  /** Caption above the rows; null for the leading, uncaptioned rows. */
  readonly title: string | null;
  readonly keys: readonly string[];
}

export interface ComponentPresentation {
  readonly sections: readonly FieldSection[];
  /**
   * Keyed by JSON pointer relative to the component, e.g. `/shape/radius`. An
   * array index may be written as a `*` segment (`/shape/parts/` + `*` + `/id`);
   * an exact pointer wins.
   */
  readonly fields: Readonly<Record<string, FieldHint>>;
}

const PROFILES: Readonly<Record<string, ComponentPresentation>> = {
  RigidBody: {
    sections: [
      { title: null, keys: ['motion'] },
      { title: 'Gravity and damping', keys: ['gravity_scale', 'linear_damping', 'angular_damping'] },
      { title: 'Solver', keys: ['can_sleep', 'ccd'] },
    ],
    fields: {
      '/motion': { description: 'fixed, dynamic, or kinematic (moved by its authored velocity).' },
      '/gravity_scale': { description: 'Multiplier on world gravity (dimensionless).' },
      '/can_sleep': { description: 'Lets the solver pause this body while it is at rest.' },
      '/ccd': { title: 'CCD', description: 'Continuous collision detection for fast-moving bodies.' },
    },
  },
  Collider: {
    sections: [
      { title: null, keys: ['shape'] },
      { title: 'Material', keys: ['density', 'friction', 'restitution'] },
      { title: 'Collision', keys: ['sensor', 'memberships', 'filter'] },
    ],
    fields: {
      '/friction': { description: 'Friction coefficient (dimensionless).' },
      '/restitution': { description: 'Bounciness, 0 to 1 (dimensionless).' },
      '/sensor': { description: 'Reports overlaps instead of producing contacts.' },
      '/filter': { description: 'Collision groups this collider can interact with.' },
      // Compound parts: what the part is, where it sits, then its primitive shape.
      '/shape/parts/*': { order: ['id', 'translation', 'rotation', 'shape'] },
      '/shape/parts/*/id': { title: 'ID', identifier: true },
      '/shape/parts/*/translation': { title: 'Offset' },
      '/shape/parts/*/shape': { title: 'Primitive', description: 'Box, sphere or capsule (local Y). Parts cannot nest.' },
      '/shape/parts/*/shape/half_extents': { short: '½' },
      '/shape/parts/*/shape/half_height': { short: '½h' },
      '/shape/parts/*/shape/radius': { short: 'r' },
    },
  },
  Camera: {
    // One run in schema order: the projection (with its variant fields beneath
    // it), then lens and clip planes.
    sections: [{ title: null, keys: ['projection', 'fov_degrees', 'near', 'far'] }],
    fields: {
      '/projection': {
        title: 'Projection',
        variants: { perspective: 'Perspective', orthographic: 'Orthographic', pixel_perfect: 'Pixel perfect' },
        omittedNote: 'Not authored. Cameras without a projection use the schema default.',
      },
      '/projection/vertical_size': { title: 'Vertical size' },
      '/projection/pixels_per_unit': { title: 'Pixel density' },
      '/projection/reference_resolution': {
        // Short enough that the "px" unit stays visible at the 280 px minimum
        // Inspector width; the logical sense is in the entry names and help.
        title: 'Resolution',
        axes: [
          { short: 'W', name: 'Logical width' },
          { short: 'H', name: 'Logical height' },
        ],
      },
      '/fov_degrees': {
        title: 'Field of view',
        status: (camera) => {
          const mode = isOrthographicCamera(camera)
            ? 'orthographic'
            : isPixelPerfectCamera(camera)
              ? 'pixel-perfect'
              : null;
          return mode
            ? { short: 'Unused', long: `Not used by ${mode} projection. Kept for switching back to perspective.` }
            : null;
        },
      },
      '/near': { title: 'Near clip', description: 'Distance from the camera to the near clipping plane.' },
      '/far': { title: 'Far clip', description: 'Distance from the camera to the far clipping plane.' },
    },
  },
};

/**
 * True only for a well-formed orthographic projection: exactly `kind` and a
 * finite, positive numeric `vertical_size`. A missing, mistyped or non-positive
 * size, or any extra field, is malformed; the Inspector reports it on the
 * Projection row and no sibling tag is derived from it.
 */
export function isOrthographicCamera(camera: Readonly<Record<string, unknown>>): boolean {
  const projection = camera.projection;
  if (typeof projection !== 'object' || projection === null || Array.isArray(projection)) return false;
  const record = projection as Record<string, unknown>;
  const keys = Object.keys(record).sort();
  const size = record.vertical_size;
  return (
    record.kind === 'orthographic' &&
    keys.length === 2 &&
    keys[0] === 'kind' &&
    keys[1] === 'vertical_size' &&
    typeof size === 'number' &&
    Number.isFinite(size) &&
    size > 0
  );
}

/**
 * The engine's pixel-perfect bounds (docs/spikes/pixel-perfect-cameras.md): each
 * logical dimension is a whole number from 16 to 4096, the product is at most
 * 4,194,304 pixels, and density is finite from 1 to 4096 pixels per metre.
 */
export const PIXEL_PERFECT_LIMITS = {
  minDimension: 16,
  maxDimension: 4096,
  maxPixels: 4_194_304,
  minDensity: 1,
  maxDensity: 4096,
} as const;

/**
 * True only for a well-formed pixel-perfect projection: exactly `kind`,
 * `pixels_per_unit` and `reference_resolution`, all within the engine's bounds.
 * Anything else is malformed, so no sibling tag is derived from it; the values
 * are still shown as authored and engine diagnostics explain them.
 */
export function isPixelPerfectCamera(camera: Readonly<Record<string, unknown>>): boolean {
  const projection = camera.projection;
  if (typeof projection !== 'object' || projection === null || Array.isArray(projection)) return false;
  const record = projection as Record<string, unknown>;
  const keys = Object.keys(record).sort();
  const density = record.pixels_per_unit;
  const size = record.reference_resolution;
  const limits = PIXEL_PERFECT_LIMITS;
  const dimension = (n: unknown): n is number =>
    typeof n === 'number' && Number.isInteger(n) && n >= limits.minDimension && n <= limits.maxDimension;
  return (
    record.kind === 'pixel_perfect' &&
    keys.length === 3 &&
    keys[0] === 'kind' &&
    keys[1] === 'pixels_per_unit' &&
    keys[2] === 'reference_resolution' &&
    typeof density === 'number' &&
    Number.isFinite(density) &&
    density >= limits.minDensity &&
    density <= limits.maxDensity &&
    Array.isArray(size) &&
    size.length === 2 &&
    dimension(size[0]) &&
    dimension(size[1]) &&
    size[0] * size[1] <= limits.maxPixels
  );
}

/** Native component types are bare ("Collider"); the fixture prefixes "incant.". */
export function componentPresentation(type: string): ComponentPresentation | undefined {
  return PROFILES[type.replace(/^incant\./, '')];
}

export function fieldHint(component: string, pointerPath: string): FieldHint {
  const fields = componentPresentation(component)?.fields;
  if (!fields) return {};
  return fields[pointerPath] ?? fields[pointerPath.replace(/\/\d+(?=\/|$)/g, '/*')] ?? {};
}

/**
 * Captions runs of keys without changing their order when the schema supplies
 * one. Consecutive keys from the same profile section share a caption; keys the
 * profile does not name are captioned "Other" (never dropped). If a schema order
 * would split a section into separate runs, grouping is incompatible with it and
 * the rows stay in schema order with no captions at all.
 *
 * Only when the schema has no order does the profile also supply the order.
 */
export function sectionKeys(type: string, keys: readonly string[], schemaOrdered: boolean): FieldSection[] {
  const profile = componentPresentation(type);
  if (!profile) return [{ title: null, keys }];
  const sectionOf = (key: string) => profile.sections.find((section) => section.keys.includes(key));
  const ordered = schemaOrdered
    ? keys
    : [
        ...profile.sections.flatMap((section) => section.keys.filter((key) => keys.includes(key))),
        ...keys.filter((key) => !sectionOf(key)),
      ];
  const runs: { section: FieldSection | undefined; keys: string[] }[] = [];
  for (const key of ordered) {
    const section = sectionOf(key);
    const last = runs.at(-1);
    if (last && last.section === section) last.keys.push(key);
    else runs.push({ section, keys: [key] });
  }
  const split = new Set(runs.map((run) => run.section)).size !== runs.length;
  // An uncaptioned profile section after a captioned one would read as part of it.
  const buried = runs.some((run, index) => index > 0 && run.section?.title === null);
  if (split || buried) return [{ title: null, keys: [...ordered] }];
  return runs.map((run, index) => ({
    title: run.section ? run.section.title : index === 0 ? null : 'Other',
    keys: run.keys,
  }));
}

export const MASK_BITS = 32;
export const MASK_ALL = 0xffffffff;

/** Collision groups set in a mask, 1-based (group 1 is bit 0). */
export function maskGroups(mask: number): number[] {
  const groups: number[] = [];
  for (let bit = 0; bit < MASK_BITS; bit += 1) {
    if (Math.floor(mask / 2 ** bit) % 2 === 1) groups.push(bit + 1);
  }
  return groups;
}

export function isMask(value: unknown): value is number {
  return typeof value === 'number' && Number.isInteger(value) && value >= 0 && value <= MASK_ALL;
}

/** Short visible summary of a mask. The raw value is always shown beside it. */
export function maskSummary(mask: number): string {
  if (mask === MASK_ALL) return 'All groups';
  if (mask === 0) return 'No groups';
  const groups = maskGroups(mask);
  return groups.length === 1 ? `Group ${groups[0]}` : `${groups.length} groups`;
}

/** Full spoken description, listing every group (and runs as ranges). */
export function maskDescription(mask: number): string {
  if (mask === MASK_ALL) return 'Every collision group, 1 to 32.';
  if (mask === 0) return 'No collision groups.';
  const groups = maskGroups(mask);
  const runs: string[] = [];
  for (let i = 0; i < groups.length; ) {
    let j = i;
    while (j + 1 < groups.length && groups[j + 1] === groups[j]! + 1) j += 1;
    runs.push(j - i >= 2 ? `${groups[i]} to ${groups[j]}` : groups.slice(i, j + 1).join(', '));
    i = j + 1;
  }
  return `${groups.length === 1 ? 'Collision group' : 'Collision groups'} ${runs.join(', ')} of 32.`;
}
