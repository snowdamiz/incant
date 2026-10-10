/**
 * Inspector presentation profiles: section captions, and labels, tooltips or
 * units the engine schema does not carry. Field order and the collision-mask
 * widget come from the schema (order, x-incant-widget), not from here.
 *
 * Presentation only. Nothing here validates, converts, reorders or hides a
 * value, and the engine schema wins: a schema `title`, `description` or
 * `x-incant-unit` overrides the matching profile entry, and a schema `order` is
 * kept exactly.
 */

/** A short visible tag beside a value, with a full sentence for assistive tech and hover. */
export interface FieldStatus {
  readonly short: string;
  readonly long: string;
}

export interface FieldHint {
  readonly title?: string;
  readonly description?: string;
  /** Shown only when the schema has no `x-incant-unit` of its own. */
  readonly unit?: string;
  /** Display names for a tagged union's variants, keyed by tag. Unlisted tags show as authored. */
  readonly variants?: Readonly<Record<string, string>>;
  /**
   * The engine's documented meaning of an omitted optional union, presented as
   * that variant with a visible "Default" tag (never as an authored value).
   */
  readonly whenOmitted?: { readonly value: Readonly<Record<string, unknown>>; readonly note: string };
  /**
   * A tag that depends on sibling values in the same component, such as a field
   * the current mode ignores. Returns null when there is nothing to say or the
   * siblings are not recognizable (it never guesses).
   */
  readonly status?: (component: Readonly<Record<string, unknown>>) => FieldStatus | null;
  /** Presents a plain string as a stable identifier (monospace, never a reference). */
  readonly identifier?: boolean;
  /** Abbreviation in one-line list summaries ("r" for radius); the full label is spoken. */
  readonly short?: string;
  /** Row order for an object without a schema order (list item detail). */
  readonly order?: readonly string[];
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
    // One run in schema order: lens and clip planes, then the projection with
    // its variant fields beneath it.
    sections: [{ title: null, keys: ['fov_degrees', 'near', 'far', 'projection'] }],
    fields: {
      '/fov_degrees': {
        title: 'Field of view',
        unit: '°',
        description: 'Vertical field of view for perspective projection.',
        status: (camera) =>
          projectionKind(camera) === 'orthographic'
            ? {
                short: 'Unused',
                long: 'Not used by orthographic projection. Kept for switching back to perspective.',
              }
            : null,
      },
      '/near': { title: 'Near clip', unit: 'm', description: 'Distance from the camera to the near clipping plane.' },
      '/far': { title: 'Far clip', unit: 'm', description: 'Distance from the camera to the far clipping plane.' },
      '/projection': {
        title: 'Projection',
        description: 'Perspective, or orthographic with parallel view rays and a fixed world-space height.',
        variants: { perspective: 'Perspective', orthographic: 'Orthographic' },
        whenOmitted: {
          value: { kind: 'perspective' },
          note: 'Not authored. Cameras without a projection use perspective.',
        },
      },
      '/projection/vertical_size': {
        title: 'Vertical size',
        unit: 'm',
        description: 'Visible world-space height. Width follows the viewport aspect ratio.',
      },
    },
  },
};

/** Camera projection tag, or null when the value is malformed (the Inspector shows that itself). */
function projectionKind(camera: Readonly<Record<string, unknown>>): string | null {
  const projection = camera.projection;
  if (projection === undefined) return 'perspective';
  if (typeof projection !== 'object' || projection === null || Array.isArray(projection)) return null;
  const kind = (projection as Record<string, unknown>).kind;
  return typeof kind === 'string' ? kind : null;
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
