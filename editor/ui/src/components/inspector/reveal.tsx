import { createContext, useContext, useLayoutEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { flushSync } from 'react-dom';
import { fieldDomId } from './FieldView';
import { parsePointer } from './format';

type Reveal = (remainingPath: readonly string[]) => void;
type Registry = Map<string, Map<symbol, Reveal>>;
const RevealContext = createContext<Registry | null>(null);

/** Local to one Inspector: registrations disappear with their mounted lists. */
export function FieldRevealProvider({ children }: { children: ReactNode }) {
  const [registry] = useState<Registry>(() => new Map());
  return <RevealContext.Provider value={registry}>{children}</RevealContext.Provider>;
}

/** Registers presentation-only work needed before a descendant field can be focused. */
export function useFieldReveal(id: string, reveal: Reveal) {
  const registry = useContext(RevealContext);
  const latest = useRef(reveal);
  useLayoutEffect(() => { latest.current = reveal; });
  useLayoutEffect(() => {
    if (!registry) return;
    const token = Symbol();
    let handlers = registry.get(id);
    if (!handlers) registry.set(id, handlers = new Map());
    handlers.set(token, (path) => latest.current(path));
    return () => {
      handlers.delete(token);
      if (handlers.size === 0) registry.delete(id);
    };
  }, [id, registry]);
}

/** Only a real, in-bounds list index may select or scroll a virtualized item. */
export function revealIndex(path: readonly string[], count: number): number | null {
  const token = path[0];
  if (token === undefined || !/^(0|[1-9]\d*)$/.test(token)) return null;
  const index = Number(token);
  return Number.isSafeInteger(index) && index < count ? index : null;
}

/** Called from diagnostic activation, never rendering/effects. No deferred focus work. */
export function useRevealField() {
  const registry = useContext(RevealContext);
  return (entity: string, component: string, path: string | null) => {
    // Malformed bridge paths fall back to the component, never a guessed field.
    const parts = parsePointer(path) ?? [];
    // Commit each ancestor's selection/open/scroll before resolving its descendants.
    // Opening a numeric table can register another handler for the same field ID.
    for (let length = 0; length <= parts.length; length += 1) {
      const id = fieldDomId(entity, component, parts.slice(0, length));
      const visited = new Set<symbol>();
      for (;;) {
        const entry = [...(registry?.get(id) ?? [])].find(([token]) => !visited.has(token));
        if (!entry) break;
        visited.add(entry[0]);
        flushSync(() => entry[1](parts.slice(length)));
      }
    }
    for (let length = parts.length; length >= 0; length -= 1) {
      const target = document.getElementById(fieldDomId(entity, component, parts.slice(0, length)));
      if (!target) continue;
      target.scrollIntoView({ block: 'center', inline: 'nearest' });
      (target.matches('input,button,[tabindex]') ? target : target.querySelector<HTMLElement>('input,button,[tabindex]'))?.focus();
      break;
    }
  };
}
