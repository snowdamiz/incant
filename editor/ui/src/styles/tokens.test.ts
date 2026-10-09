import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

// Vitest runs with the editor/ui package as its root.
const tokensCss = readFileSync(join(process.cwd(), 'src/styles/tokens.css'), 'utf8');

/** Parses `--name: #rrggbb;` declarations from tokens.css. */
function readTokens(css: string): Map<string, string> {
  const tokens = new Map<string, string>();
  for (const match of css.matchAll(/--([a-z0-9-]+):\s*(#[0-9a-fA-F]{6})\s*;/g)) {
    tokens.set(match[1]!, match[2]!.toLowerCase());
  }
  return tokens;
}

function luminance(hex: string): number {
  const channels = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
  const [r, g, b] = channels.map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4)) as [
    number,
    number,
    number,
  ];
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number];
  return (hi + 0.05) / (lo + 0.05);
}

const tokens = readTokens(tokensCss);
const color = (name: string) => {
  const value = tokens.get(name);
  if (!value) throw new Error(`Missing token --${name}`);
  return value;
};

// WCAG 2.2: 4.5:1 for text (SC 1.4.3), 3:1 for UI components and focus indicators (SC 1.4.11).
const TEXT_SURFACES = ['color-bg-app', 'color-bg-panel', 'color-bg-raised', 'color-bg-hover', 'color-bg-selected'];
const TEXT = [
  'color-text',
  'color-text-muted',
  'color-text-subtle',
  'color-accent',
  'color-danger',
  'color-warning',
  'color-success',
  'color-info',
  'color-origin-user',
  'color-origin-agent',
  'color-origin-script',
  'color-origin-import',
  'color-axis-x',
  'color-axis-y',
  'color-axis-z',
  'color-axis-w',
];

describe('design token contrast', () => {
  it('parses the palette', () => {
    expect(tokens.size).toBeGreaterThan(30);
  });

  for (const fg of TEXT) {
    for (const bg of TEXT_SURFACES) {
      it(`${fg} on ${bg} is at least 4.5:1`, () => {
        expect(contrast(color(fg), color(bg))).toBeGreaterThanOrEqual(4.5);
      });
    }
  }

  it.each([
    ['color-danger', 'color-danger-bg'],
    ['color-warning', 'color-warning-bg'],
    ['color-info', 'color-info-bg'],
    ['color-text', 'color-danger-bg'],
    ['color-text', 'color-warning-bg'],
    ['color-text', 'color-info-bg'],
    ['color-text-muted', 'color-info-bg'],
    ['color-fixture', 'color-fixture-bg'],
    ['color-text-on-accent', 'color-accent'],
    ['color-text', 'color-bg-selected-inactive'],
    ['color-text-subtle', 'color-bg-viewport'],
  ])('%s on %s is at least 4.5:1', (fg, bg) => {
    expect(contrast(color(fg), color(bg))).toBeGreaterThanOrEqual(4.5);
  });

  it.each(['color-bg-app', 'color-bg-panel', 'color-bg-raised', 'color-bg-hover', 'color-bg-selected'])(
    'focus ring is at least 3:1 against %s',
    (bg) => {
      expect(contrast(color('color-focus'), color(bg))).toBeGreaterThanOrEqual(3);
    },
  );

  it.each(['color-bg-app', 'color-bg-panel'])('control borders are at least 3:1 against %s', (bg) => {
    expect(contrast(color('color-border-control'), color(bg))).toBeGreaterThanOrEqual(3);
  });

  it('focus (amber) and selection (blue) differ in hue so focus never reads as selection', () => {
    expect(color('color-focus')).not.toBe(color('color-accent'));
    expect(contrast(color('color-focus'), color('color-bg-selected'))).toBeGreaterThanOrEqual(3);
  });
});
