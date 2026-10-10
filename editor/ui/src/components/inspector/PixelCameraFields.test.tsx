import { cleanup, within } from '@testing-library/react';
import axe from 'axe-core';
import { afterEach, describe, expect, it } from 'vitest';
import type { ComponentSchema, Diagnostic, FieldSchema } from '../../bridge/contract';
import { LEGACY, PIXEL, description, expectFieldOfViewLive, inspect, nativeCamera, regionOf, rows } from '../../test/cameraInspector';
import { isPixelPerfectCamera } from './presentation';

/*
 * SYNTHETIC SCHEMA — NOT ON MAIN.
 *
 * Main's schemas/Camera.schema.json declares perspective and orthographic projections
 * only. The pixel-perfect renderer, its Rust schema and its acceptance evidence live on
 * the preserved handoff/0033-pixel-integration branch. These tests keep the Inspector's
 * pixel-perfect presentation (density in px/m, a W × H logical resolution, the "Unused"
 * field of view) exercised by injecting that branch's converted variant into main's
 * native schema. Nothing here reaches the sample fixture or the native parity tests.
 *
 * When pixel-perfect lands on main: delete SYNTHETIC, test against nativeCamera(), and
 * let CameraFields.test.tsx's parity tests cover the variant. The first test below
 * fails at that point as a reminder.
 */

afterEach(cleanup);

/** The pixel-perfect variant as handoff/0033-pixel-integration's bridge conversion produces it. */
const PIXEL_PERFECT_VARIANT = {
  type: 'object',
  additionalProperties: false,
  properties: {
    kind: { type: 'string', const: 'pixel_perfect', enum: ['pixel_perfect'], optional: false },
    pixels_per_unit: {
      type: 'number',
      format: 'double',
      description: 'Logical pixels per world metre. Camera motion snaps to this grid.',
      minimum: 1,
      maximum: 4096,
      'x-incant-unit': 'px/m',
      optional: false,
    },
    reference_resolution: {
      type: 'array',
      description: 'Fixed logical width and height, scaled by whole pixels with letterboxing.',
      items: { type: 'integer', format: 'uint32', minimum: 0 },
      minItems: 2,
      maxItems: 2,
      'x-incant-unit': 'px',
      optional: false,
    },
  },
  required: ['kind', 'pixels_per_unit', 'reference_resolution'],
} as unknown as FieldSchema;

const NATIVE = nativeCamera();
type Union = Extract<FieldSchema, { type: 'tagged-union' }>;
const nativeProjection = NATIVE.properties.projection as Union;

/** Main's native Camera schema plus the synthetic pixel-perfect variant. */
const SYNTHETIC: ComponentSchema = {
  ...NATIVE,
  properties: {
    ...NATIVE.properties,
    projection: {
      ...nativeProjection,
      description: 'Perspective, orthographic, or pixel-perfect with a fixed logical resolution.',
      variants: { ...nativeProjection.variants, pixel_perfect: PIXEL_PERFECT_VARIANT },
    } as FieldSchema,
  },
};

describe('synthetic pixel-perfect schema', () => {
  it('extends main, which does not declare pixel-perfect yet', () => {
    // If this fails, main's schema now has the variant: switch these tests to nativeCamera().
    expect(Object.keys(nativeProjection.variants)).toEqual(['perspective', 'orthographic']);
    expect(Object.keys((SYNTHETIC.properties.projection as Union).variants)).toEqual(['perspective', 'orthographic', 'pixel_perfect']);
  });

  it('has no axe violations for well-formed and malformed pixel-perfect cameras', async () => {
    for (const value of [{ ...LEGACY, projection: PIXEL }, { ...LEGACY, projection: { ...PIXEL, reference_resolution: [320] } }]) {
      await inspect(value, [], SYNTHETIC);
      const panel = document.querySelector<HTMLElement>('[data-region="inspector"]')!;
      const results = await axe.run(panel, { rules: { 'color-contrast': { enabled: false } } });
      expect(results.violations.map((v) => v.id)).toEqual([]);
      cleanup();
    }
  });
});

describe('pixel-perfect Camera Inspector', () => {
  it('shows density in px/m and the logical resolution as a labelled width and height', async () => {
    const camera = await inspect({ ...LEGACY, projection: PIXEL }, [], SYNTHETIC);
    expect(rows(camera)).toEqual([
      'Projection: Pixel perfect',
      'Pixel density: 32 px/m',
      // The row helper reads the first entry's value; both entries are checked below.
      'Resolution, in px: 320 WH',
      'Field of view: 60 °Unused',
      'Near clip: 0.1 m',
      'Far clip: 100 m',
    ]);
    const group = camera.getByRole('group', { name: 'Projection' });
    const density = within(group).getByRole('textbox', { name: 'Pixel density' });
    expect(description(density)).toContain('Logical pixels per world metre. Camera motion snaps to this grid.');
    const resolution = within(group).getByRole('group', { name: /^Resolution/ });
    const [width, height] = within(resolution).getAllByRole('textbox');
    expect(width).toBe(within(resolution).getByRole('textbox', { name: 'Logical width' }));
    expect(height).toBe(within(resolution).getByRole('textbox', { name: 'Logical height' }));
    expect([width, height].map((input) => (input as HTMLInputElement).value)).toEqual(['320', '180']);
    expect(description(width!)).toContain('Fixed logical width and height, scaled by whole pixels with letterboxing.');
    // W/H are names, not spatial axes: they never take the X/Y axis tints.
    expect(Array.from(resolution.querySelectorAll('.vector__axis-name')).map((n) => n.className)).toEqual([
      'vector__axis-name axis--plain',
      'vector__axis-name axis--plain',
    ]);
    expect(description(camera.getByRole('textbox', { name: 'Field of view' }))).toContain(
      'Not used by pixel-perfect projection. Kept for switching back to perspective.',
    );
  });

  it('is read-only and reachable by Tab in reading order, with no projection controls', async () => {
    const camera = await inspect({ ...LEGACY, projection: PIXEL }, [], SYNTHETIC);
    const region = regionOf(camera);
    expect(region.querySelectorAll('button:not(.component__toggle), select, [role="combobox"], [role="radio"]')).toHaveLength(0);
    const inputs = Array.from(region.querySelectorAll<HTMLInputElement>('input'));
    expect(inputs.every((input) => input.readOnly && input.tabIndex === 0)).toBe(true);
    const names = inputs.map((input) =>
      input.getAttribute('aria-label') ?? document.getElementById(input.getAttribute('aria-labelledby')!)?.textContent,
    );
    expect(names).toEqual(['Projection', 'Pixel density', 'Logical width', 'Logical height', 'Field of view', 'Near clip', 'Far clip']);
  });

  it.each([
    ['one number', [320], '[320]'],
    ['three numbers', [320, 180, 1], '[320,180,1]'],
    ['an empty list', [], '[]'],
  ])('reports a resolution of %s as a mismatch instead of guessing a width or height', async (_name, size, raw) => {
    const camera = await inspect({ ...LEGACY, projection: { ...PIXEL, reference_resolution: size } }, [], SYNTHETIC);
    expect(camera.getByRole('textbox', { name: 'Projection' })).toHaveProperty('value', 'Pixel perfect');
    const notice = camera.getByRole('group', { name: /^Resolution/ });
    expect(notice.textContent).toContain('Cannot show as 2 numbers (logical width and logical height); raw value:');
    expect(notice.querySelector('code')?.textContent).toBe(raw);
    expect(notice.tabIndex).toBe(0);
    expect(camera.queryByRole('textbox', { name: 'Logical width' })).toBeNull();
    expectFieldOfViewLive(camera);
  });

  it.each([
    ['a missing resolution', { kind: 'pixel_perfect', pixels_per_unit: 32 }, 'Resolution', 'Cannot show as a list of numbers; raw value:', 'undefined'],
    ['a text resolution', { ...PIXEL, reference_resolution: ['320', '180'] }, 'Resolution', 'Cannot show as a list of numbers; raw value:', '["320","180"]'],
    ['a missing density', { kind: 'pixel_perfect', reference_resolution: [320, 180] }, 'Pixel density', 'Cannot show as a number; raw value:', 'undefined'],
    ['a text density', { ...PIXEL, pixels_per_unit: '32' }, 'Pixel density', 'Cannot show as a number; raw value:', '"32"'],
  ])('reports %s with its raw value', async (_name, projection, label, message, raw) => {
    const camera = await inspect({ ...LEGACY, projection }, [], SYNTHETIC);
    expect(camera.getByRole('textbox', { name: 'Projection' })).toHaveProperty('value', 'Pixel perfect');
    const notice = camera.getByRole('group', { name: new RegExp(`^${label}`) });
    expect(notice.textContent).toContain(message);
    expect(notice.querySelector('code')?.textContent).toBe(raw);
    expectFieldOfViewLive(camera);
  });

  it('flags a field that is not part of the pixel-perfect projection', async () => {
    const camera = await inspect({ ...LEGACY, projection: { ...PIXEL, vertical_size: 8 } }, [], SYNTHETIC);
    expect(camera.getByRole('group', { name: 'Projection' }).textContent).toContain(
      'Field vertical_size is not part of the “pixel_perfect” projection: 8',
    );
    expectFieldOfViewLive(camera);
  });

  it('shows an out-of-range resolution as authored and lets the engine diagnostic explain it', async () => {
    const problem: Diagnostic = {
      id: 'pixel-size',
      severity: 'error',
      message: 'pixel-perfect reference resolution must be 16..4096 per side',
      entity: null,
      component: 'Camera',
      path: '/projection/reference_resolution/1',
    };
    const camera = await inspect({ ...LEGACY, projection: { ...PIXEL, reference_resolution: [320, 8] } }, [problem], SYNTHETIC);
    const width = camera.getByRole('textbox', { name: 'Logical width' });
    const height = camera.getByRole('textbox', { name: 'Logical height' });
    expect([width, height].map((input) => (input as HTMLInputElement).value)).toEqual(['320', '8']);
    expect(height.getAttribute('aria-invalid')).toBe('true');
    expect(width.getAttribute('aria-invalid')).toBeNull();
    expect(description(height)).toContain('pixel-perfect reference resolution must be 16..4096 per side');
    expectFieldOfViewLive(camera);
  });

  it('shows a tiny or fractional density as a nonzero authored value', async () => {
    const camera = await inspect({ ...LEGACY, projection: { ...PIXEL, pixels_per_unit: 0.0000025 } }, [], SYNTHETIC);
    const density = camera.getByRole('textbox', { name: 'Pixel density' });
    expect(density).toHaveProperty('value', '2.5e-6');
    expect(density.getAttribute('title')).toBe('0.0000025');
    // Out of the engine's 1..4096 range, so field of view is not marked unused.
    expectFieldOfViewLive(camera);
    cleanup();
    const fractional = await inspect({ ...LEGACY, projection: { ...PIXEL, pixels_per_unit: 16.5 } }, [], SYNTHETIC);
    expect(fractional.getByRole('textbox', { name: 'Pixel density' })).toHaveProperty('value', '16.5');
    expect(rows(fractional)[3]).toBe('Field of view: 60 °Unused');
  });
});

describe('pixel-perfect detection for the field-of-view tag', () => {
  it.each([
    ['well-formed', PIXEL, true],
    ['the smallest bounds', { ...PIXEL, pixels_per_unit: 1, reference_resolution: [16, 16] }, true],
    ['the largest density and a 2048 square', { ...PIXEL, pixels_per_unit: 4096, reference_resolution: [2048, 2048] }, true],
    ['a fractional density', { ...PIXEL, pixels_per_unit: 16.5 }, true],
    ['too many pixels', { ...PIXEL, reference_resolution: [4096, 1025] }, false],
    ['a side below 16', { ...PIXEL, reference_resolution: [320, 15] }, false],
    ['a side above 4096', { ...PIXEL, reference_resolution: [4097, 16] }, false],
    ['a fractional side', { ...PIXEL, reference_resolution: [320.5, 180] }, false],
    ['one side', { ...PIXEL, reference_resolution: [320] }, false],
    ['text sides', { ...PIXEL, reference_resolution: ['320', '180'] }, false],
    ['density below 1', { ...PIXEL, pixels_per_unit: 0.5 }, false],
    ['density above 4096', { ...PIXEL, pixels_per_unit: 4097 }, false],
    ['infinite density', { ...PIXEL, pixels_per_unit: Infinity }, false],
    ['NaN density', { ...PIXEL, pixels_per_unit: NaN }, false],
    ['missing density', { kind: 'pixel_perfect', reference_resolution: [320, 180] }, false],
    ['missing resolution', { kind: 'pixel_perfect', pixels_per_unit: 32 }, false],
    ['extra field', { ...PIXEL, vertical_size: 8 }, false],
    ['orthographic', { kind: 'orthographic', vertical_size: 8 }, false],
    ['legacy (omitted)', undefined, false],
    ['null', null, false],
  ])('%s -> %s', (_name, projection, expected) => {
    const camera: Record<string, unknown> = { ...LEGACY };
    if (projection !== undefined) camera.projection = projection;
    expect(isPixelPerfectCamera(camera)).toBe(expected);
  });

  it('does not change the authored value it inspects', () => {
    const camera = { ...LEGACY, projection: { ...PIXEL, reference_resolution: [320, 180], extra: [1] } };
    const before = JSON.stringify(camera);
    isPixelPerfectCamera(camera);
    expect(JSON.stringify(camera)).toBe(before);
  });
});
