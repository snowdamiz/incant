import { describe, expect, it } from 'vitest';
import { clampLayout, defaultLayout, MIN_VIEWPORT } from './layout';

const viewportWidth = (width: number, l: { left: number; right: number }) => width - l.left - l.right - 12;

describe('layout', () => {
  it.each([
    [1280, 800],
    [1920, 1080],
  ])('default layout at %i×%i keeps readable side panels and a dominant viewport', (w, h) => {
    const layout = defaultLayout(w, h);
    expect(layout.left).toBeGreaterThanOrEqual(240);
    expect(layout.right).toBeGreaterThanOrEqual(320);
    expect(viewportWidth(w, layout)).toBeGreaterThanOrEqual(w / 2);
  });

  it('gives panels back to the viewport when the window shrinks', () => {
    const layout = clampLayout({ left: 480, right: 560, dock: 300, agent: 300 }, 1000, 700);
    expect(viewportWidth(1000, layout)).toBeGreaterThanOrEqual(MIN_VIEWPORT.width);
    expect(layout.right).toBeGreaterThanOrEqual(280);
  });

  it('never lets the dock squeeze the viewport below its minimum height', () => {
    const layout = clampLayout({ left: 240, right: 320, dock: 640, agent: 300 }, 1280, 600);
    expect(600 - 64 - 6 - layout.dock).toBeGreaterThanOrEqual(MIN_VIEWPORT.height);
  });
});
