/** Panel regions in F6 order: left to right, top to bottom. */
export const REGIONS = ['hierarchy', 'viewport', 'dock', 'inspector', 'agent'] as const;
export type Region = (typeof REGIONS)[number];

export function regionOf(element: Element | null): Region | null {
  const name = element?.closest('[data-region]')?.getAttribute('data-region');
  return (REGIONS as readonly string[]).includes(name ?? '') ? (name as Region) : null;
}

const present = (region: Region) => document.querySelector(`[data-region="${region}"]`) !== null;

/** The next region in F6 order that is currently shown (hidden panels are skipped). */
export function nextRegion(current: Region | null, step: 1 | -1): Region {
  const count = REGIONS.length;
  let index = current === null ? (step === 1 ? -1 : count) : REGIONS.indexOf(current);
  for (let tries = 0; tries < count; tries += 1) {
    index = (index + step + count) % count;
    const candidate = REGIONS[index]!;
    if (present(candidate)) return candidate;
  }
  return 'viewport';
}

/** Focuses the region's roving item (data-focus-target) or, failing that, the region itself. */
export function focusRegion(region: Region): void {
  const root = document.querySelector<HTMLElement>(`[data-region="${region}"]`);
  if (!root) return;
  const target = root.querySelector<HTMLElement>('[data-focus-target]') ?? root;
  target.focus();
}
