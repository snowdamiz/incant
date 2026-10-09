/**
 * Inspector presentation profiles: grouping, labels and semantic units for
 * components whose engine schema carries no display annotations yet.
 *
 * Presentation only. Nothing here validates, converts or hides a value, and the
 * engine schema always wins: a schema `title`, `description`, `x-incant-unit`,
 * `x-incant-widget` or `order` overrides the matching profile entry. Fields the
 * profile does not mention still render, after the grouped ones.
 */

export interface FieldHint {
  readonly title?: string;
  readonly description?: string;
  /** Semantic unit, shown beside the value. Dimensionless values have none. */
  readonly unit?: string;
  /** A 32-bit collision group mask: summary plus a per-group strip. */
  readonly widget?: 'collision-mask';
}

export interface FieldSection {
  /** Caption above the rows; null for the leading, uncaptioned rows. */
  readonly title: string | null;
  readonly keys: readonly string[];
}

export interface ComponentPresentation {
  readonly sections: readonly FieldSection[];
  /** Keyed by JSON pointer relative to the component, e.g. `/shape/radius`. */
  readonly fields: Readonly<Record<string, FieldHint>>;
}

const METERS = 'm';

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
      '/linear_damping': { description: 'Linear velocity damping coefficient (dimensionless).' },
      '/angular_damping': { description: 'Angular velocity damping coefficient (dimensionless).' },
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
      '/shape/half_extents': { unit: METERS },
      '/shape/radius': { unit: METERS },
      '/shape/half_height': { unit: METERS },
      '/density': { unit: 'kg/m³' },
      '/friction': { description: 'Friction coefficient (dimensionless).' },
      '/restitution': { description: 'Bounciness, 0 to 1 (dimensionless).' },
      '/sensor': { description: 'Reports overlaps instead of producing contacts.' },
      '/memberships': { widget: 'collision-mask' },
      '/filter': { widget: 'collision-mask', description: 'Collision groups this collider can interact with.' },
    },
  },
  AngularVelocity: {
    sections: [{ title: null, keys: ['angular'] }],
    fields: {
      '/angular': { unit: 'rad/s', description: 'Angular velocity about each world axis.' },
    },
  },
};

/** Native component types are bare ("Collider"); the fixture prefixes "incant.". */
export function componentPresentation(type: string): ComponentPresentation | undefined {
  return PROFILES[type.replace(/^incant\./, '')];
}

export function fieldHint(component: string, pointerPath: string): FieldHint {
  return componentPresentation(component)?.fields[pointerPath] ?? {};
}

/**
 * Splits ordered keys into captioned sections. Keys the profile does not name
 * (new schema fields) are never dropped: they follow in a final, uncaptioned
 * section. Without a profile, everything is one section.
 */
export function sectionKeys(type: string, keys: readonly string[]): FieldSection[] {
  const profile = componentPresentation(type);
  if (!profile) return [{ title: null, keys }];
  const placed = new Set<string>();
  const sections: FieldSection[] = [];
  for (const section of profile.sections) {
    const present = section.keys.filter((key) => keys.includes(key));
    present.forEach((key) => placed.add(key));
    if (present.length > 0) sections.push({ title: section.title, keys: present });
  }
  const rest = keys.filter((key) => !placed.has(key));
  if (rest.length > 0) sections.push({ title: sections.length > 0 ? 'Other' : null, keys: rest });
  return sections;
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
