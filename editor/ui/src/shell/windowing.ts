import { useEffect, useState } from 'react';
import type { RefObject } from 'react';

/**
 * Bounded rendering for long panels (PLAN.md revision 4, ADR 0009): a list mounts
 * only the rows near its scroll position, plus any row that must stay mounted
 * (the focused, selected or renaming one), however long the list is.
 *
 * Without layout (jsdom, or a panel that has not been measured yet) a scroller
 * counts as FALLBACK_HEIGHT tall, so short lists mount completely and long lists
 * stay bounded from the first render.
 */
export const FALLBACK_HEIGHT = 640;
export const FALLBACK_WIDTH = 640;

export interface Viewport {
  /** Scroll offset rounded down to a multiple of `step`, so small scrolls do not re-render. */
  readonly top: number;
  readonly height: number;
  /** Horizontal scroll offset, rounded down to a multiple of `step`. */
  readonly left: number;
  readonly width: number;
}

/**
 * Tracks a scroll container's viewport. Updates only when the scroll offset crosses
 * a `step` boundary or the container is resized; callers overscan by at least one
 * step, so the mounted rows always cover what is visible.
 */
export function useViewport(
  ref: RefObject<HTMLElement | null>,
  step: number,
  fallback: { readonly height: number; readonly width: number } = { height: FALLBACK_HEIGHT, width: FALLBACK_WIDTH },
): Viewport {
  const [viewport, setViewport] = useState<Viewport>({ top: 0, height: fallback.height, left: 0, width: fallback.width });
  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    let frame = 0;
    const measure = () => {
      frame = 0;
      const top = Math.max(0, Math.floor(element.scrollTop / step) * step);
      const height = element.clientHeight || fallback.height;
      const left = Math.max(0, Math.floor(element.scrollLeft / step) * step);
      const width = element.clientWidth || fallback.width;
      setViewport((current) =>
        current.top === top && current.height === height && current.left === left && current.width === width ? current : { top, height, left, width },
      );
    };
    const schedule = () => {
      if (frame === 0) frame = requestAnimationFrame(measure);
    };
    measure();
    element.addEventListener('scroll', schedule, { passive: true });
    const observer = new ResizeObserver(schedule);
    observer.observe(element);
    return () => {
      element.removeEventListener('scroll', schedule);
      observer.disconnect();
      if (frame !== 0) cancelAnimationFrame(frame);
    };
  }, [ref, step, fallback.height, fallback.width]);
  return viewport;
}

/** First and last (exclusive) index of fixed-height rows to mount for a viewport. */
export function rowRange(viewport: Viewport, count: number, rowHeight: number, overscan: number, offset = 0): [number, number] {
  const first = Math.max(0, Math.floor((viewport.top - offset) / rowHeight) - overscan);
  const last = Math.min(count, Math.ceil((viewport.top - offset + viewport.height) / rowHeight) + overscan);
  return [Math.min(first, count), Math.max(first, last)];
}

/** Indices first..last-1 plus pinned ones, ascending and unique. Pinned indices outside 0..count are ignored. */
export function mountedIndices(first: number, last: number, count: number, pinned: readonly number[]): number[] {
  const set = new Set<number>();
  for (let index = first; index < last; index += 1) set.add(index);
  for (const index of pinned) if (index >= 0 && index < count) set.add(index);
  return [...set].sort((a, b) => a - b);
}

/**
 * Scrolls `scroller` the least distance that shows the span [top, top + height),
 * like scrollIntoView({ block: 'nearest' }) for a row that may not be mounted yet.
 */
export function revealSpan(scroller: HTMLElement | null, top: number, height: number, margin = 0): void {
  if (!scroller) return;
  const view = scroller.clientHeight;
  if (view === 0) return;
  if (top - margin < scroller.scrollTop) scroller.scrollTop = Math.max(0, top - margin);
  else if (top + height + margin > scroller.scrollTop + view) scroller.scrollTop = top + height + margin - view;
}

/** Index of the last item whose start is at or before `y` in an ascending list of starts. */
export function indexAt(starts: readonly number[], y: number): number {
  let low = 0;
  let high = starts.length - 1;
  let found = 0;
  while (low <= high) {
    const mid = (low + high) >> 1;
    if (starts[mid]! <= y) {
      found = mid;
      low = mid + 1;
    } else high = mid - 1;
  }
  return found;
}
