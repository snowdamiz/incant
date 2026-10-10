/**
 * Inspector presentation profiles: section captions, and labels or tooltips the
 * engine schema does not carry. Units, field order and the collision-mask widget
 * come from the schema (x-incant-unit, order, x-incant-widget), not from here.
 *
 * Presentation only. Nothing here validates, converts, reorders or hides a
 * value, and the engine schema wins: a schema `title` or `description`
 * overrides the matching profile entry, and a schema `order` is kept exactly.
 */

export interface FieldHint {
  readonly title?: string;
  readonly description?: string;
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
