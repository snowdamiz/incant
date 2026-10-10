import { createRef, StrictMode } from 'react';
import type { RefObject } from 'react';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useViewport } from './windowing';

afterEach(() => vi.unstubAllGlobals());

function Probe({ elementRef, visible = true, step = 100 }: { elementRef: RefObject<HTMLDivElement | null>; visible?: boolean; step?: number }) {
  const [viewport, attach] = useViewport(elementRef, step, { height: 100, width: 300 });
  return (
    <>
      <output aria-label="Viewport">{JSON.stringify(viewport)}</output>
      {visible ? <div ref={attach} role="region" aria-label="Scroller" /> : null}
    </>
  );
}

const measured = () => JSON.parse(screen.getByLabelText('Viewport').textContent!);

/** Deterministic browser notifications; no layout or performance claims. */
function notifications() {
  const frames = new Map<number, FrameRequestCallback>();
  let nextFrame = 1;
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
    const id = nextFrame++;
    frames.set(id, callback);
    return id;
  });
  vi.stubGlobal('cancelAnimationFrame', (id: number) => frames.delete(id));
  const observers: { target: Element | null; resize: () => void }[] = [];
  vi.stubGlobal('ResizeObserver', class {
    readonly observation: (typeof observers)[number];
    constructor(callback: () => void) {
      this.observation = { target: null, resize: callback };
      observers.push(this.observation);
    }
    observe(target: Element) { this.observation.target = target; }
    disconnect() { this.observation.target = null; }
  });
  const flush = () => act(() => {
    const pending = [...frames.values()];
    frames.clear();
    for (const callback of pending) callback(0);
  });
  return { frames, observers, flush };
}

describe('viewport DOM lifecycle', () => {
  it('keeps one subscription through rerenders, then observes the replacement node and cancels detached work', () => {
    const { frames, observers, flush } = notifications();
    const elementRef = createRef<HTMLDivElement>();
    const show = (visible = true) => <StrictMode><Probe elementRef={elementRef} visible={visible} /></StrictMode>;
    const { rerender, unmount } = render(show());
    const original = screen.getByRole('region', { name: 'Scroller' });
    expect(elementRef.current).toBe(original);
    expect(observers.filter((observer) => observer.target === original)).toHaveLength(1);
    const initialSubscriptions = observers.length;
    original.scrollTop = 350;
    original.scrollLeft = 215;
    fireEvent.scroll(original);
    fireEvent.scroll(original);
    expect(frames.size).toBe(1);
    flush();
    expect(measured()).toEqual({ top: 300, height: 100, left: 200, width: 300 });
    rerender(show());
    expect(observers).toHaveLength(initialSubscriptions);

    // A scroll queued just before an empty state must not measure its detached node.
    fireEvent.scroll(original);
    expect(frames.size).toBe(1);
    rerender(show(false));
    expect(elementRef.current).toBeNull();
    expect(frames.size).toBe(0);
    expect(observers.every((observer) => observer.target === null)).toBe(true);
    fireEvent.scroll(original);
    expect(frames.size).toBe(0);

    rerender(show());
    const replacement = screen.getByRole('region', { name: 'Scroller' });
    expect(replacement).not.toBe(original);
    expect(elementRef.current).toBe(replacement);
    expect(measured()).toEqual({ top: 0, height: 100, left: 0, width: 300 });
    expect(observers.filter((observer) => observer.target === replacement)).toHaveLength(1);
    Object.defineProperties(replacement, { clientHeight: { value: 250 }, clientWidth: { value: 450 } });
    observers.find((observer) => observer.target === replacement)!.resize();
    flush();
    expect(measured()).toEqual({ top: 0, height: 250, left: 0, width: 450 });
    replacement.scrollTop = 710;
    fireEvent.scroll(replacement);
    flush();
    expect(measured().top).toBe(700);
    fireEvent.scroll(replacement);
    unmount();
    expect(elementRef.current).toBeNull();
    expect(frames.size).toBe(0);
    expect(observers.every((observer) => observer.target === null)).toBe(true);
  });

  it('attaches after an initially empty render and remeasures when the scroll step changes', () => {
    const { observers, flush } = notifications();
    const elementRef = createRef<HTMLDivElement>();
    const { rerender } = render(<Probe elementRef={elementRef} visible={false} />);
    expect(observers).toHaveLength(0);
    rerender(<Probe elementRef={elementRef} />);
    const scroller = screen.getByRole('region', { name: 'Scroller' });
    scroller.scrollTop = 350;
    fireEvent.scroll(scroller);
    flush();
    expect(measured().top).toBe(300);
    rerender(<Probe elementRef={elementRef} step={200} />);
    expect(elementRef.current).toBe(scroller);
    expect(measured().top).toBe(200);
    expect(observers.filter((observer) => observer.target === scroller)).toHaveLength(1);
  });
});
