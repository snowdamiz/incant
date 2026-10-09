import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Revision 2 ("connected") presentation contract. jsdom applies no stylesheet, so these
 * read the CSS source. Real rendering is reviewed in Chrome and in the native app
 * (handoffs/0006-connected-editor).
 */
const css = readFileSync(join(process.cwd(), 'src/styles/app.css'), 'utf8');
const tokens = readFileSync(join(process.cwd(), 'src/styles/tokens.css'), 'utf8');

/** Declarations of the first rule whose selector list is exactly `selector`. */
function rule(selector: string): Record<string, string> {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = new RegExp(`(?:^|\\})\\s*${escaped}\\s*\\{([^}]*)\\}`, 'm').exec(css);
  if (!match) throw new Error(`No rule for ${selector}`);
  return Object.fromEntries(
    match[1]!
      .split(';')
      .map((line) => line.replace(/\/\*[\s\S]*?\*\//g, '').trim())
      .filter(Boolean)
      .map((line) => {
        const at = line.indexOf(':');
        return [line.slice(0, at).trim(), line.slice(at + 1).trim()];
      }),
  );
}
const token = (name: string) => new RegExp(`--${name}:\\s*([^;]+);`).exec(tokens)?.[1]?.trim();

describe('connected frame presentation', () => {
  it('has no gutters between panels', () => {
    expect(rule('.workspace').padding).toBeUndefined();
    expect(rule('.workspace').gap).toBeUndefined();
    expect(css).not.toMatch(/var\(--gutter\)/);
  });

  it('draws panels as flat regions of one frame, not rounded floating cards', () => {
    const panel = rule('.panel');
    expect(panel['border-radius']).toBeUndefined();
    expect(panel['box-shadow']).toBeUndefined();
    expect(panel.border).toBeUndefined();
  });

  it('separates panels with 1 px opaque separators that remain the resize handles', () => {
    expect(token('splitter-size')).toBe('1px');
    expect(token('color-border')).toMatch(/^#[0-9a-f]{6}$/i);
    expect(rule('.splitter').background).toBe('var(--color-border)');
    // The drag target is wider than the line and overlaps both neighbours.
    expect(rule('.splitter--vertical::before').left).toBe('calc(-1 * var(--splitter-hit))');
    expect(rule('.splitter--horizontal::before').top).toBe('calc(-1 * var(--splitter-hit))');
  });

  it('keeps the viewport host square so the native surface needs no corner mask', () => {
    expect(rule('.viewport-host')['border-radius']).toBe('0');
  });

  it('rules a single header line across the hierarchy, viewport and inspector', () => {
    const header = rule('.panel__header');
    expect(header.height).toBe('var(--header-height)');
    expect(header['border-bottom']).toBe('1px solid var(--color-border)');
    expect(rule('.tabs').height).toBe('var(--header-height)');
  });

  it('keeps the macOS logo clear of the traffic lights', () => {
    // 78 px inset + 12 px titlebar padding + (8 - 12) px = logo box at 86 pt.
    expect(rule('.titlebar__inset + .titlebar__start')['margin-left']).toBe('calc(var(--space-3) - var(--space-4))');
  });

  it('separates the titlebar and status line from the panels with hairlines', () => {
    // A shadow, not a border, so the bar's content centres on the full 40 px with the lights.
    expect(rule('.titlebar')['box-shadow']).toBe('inset 0 -1px 0 var(--color-border)');
    expect(rule('.titlebar')['border-bottom']).toBeUndefined();
    expect(rule('.statusbar')['border-top']).toBe('1px solid var(--color-border)');
    // The native traffic lights are centred for a 40 px bar (TITLEBAR.md); keep it.
    expect(token('titlebar-height')).toBe('40px');
  });
});
