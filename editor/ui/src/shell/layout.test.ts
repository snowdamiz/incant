import { describe, expect, it } from 'vitest';
import { CHROME_HEIGHT, clampLayout, defaultLayout, IDLE_AGENT_MIN, idleAgentHeight, MIN_VIEWPORT, SPLITTER } from './layout';

const viewportWidth = (width: number, l: { left: number; right: number }) => width - l.left - l.right - 2 * SPLITTER;

describe('layout', () => {
  it.each([
    [1280, 800],
    [1920, 1080],
  ])('default layout at %i×%i keeps readable side panels and a dominant viewport', (w, h) => {
    const layout = defaultLayout(w, h);
    expect(layout.left).toBeGreaterThanOrEqual(232);
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
    expect(600 - CHROME_HEIGHT - SPLITTER - layout.dock).toBeGreaterThanOrEqual(MIN_VIEWPORT.height);
  });

  it.each([
    [650, 200],
    [874, 210],
    [1080, 240],
    [1440, 240],
  ])('gives an unavailable agent a compact height at %ipx tall (%ipx), never above the regular default', (h, expected) => {
    expect(idleAgentHeight(h)).toBe(expected);
    expect(idleAgentHeight(h)).toBeGreaterThanOrEqual(IDLE_AGENT_MIN);
    expect(idleAgentHeight(h)).toBeLessThan(defaultLayout(1440, h).agent);
  });
});
