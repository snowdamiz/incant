import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import axe from 'axe-core';
import { afterEach, describe, expect, it } from 'vitest';
import { App } from '../../App';
import { snapshotFromEngine } from '../../../../bridge/native';
import type { EngineRead } from '../../../../bridge/native';
import type { BridgeSnapshot, ComponentSchema, Diagnostic, EditorBridge } from '../../bridge/contract';
import { createFixtureBridge, fixtureSnapshot } from '../../bridge/fixture';
import { FieldView, formatNumber, presentableDefault } from './FieldView';
import { isOrthographicCamera } from './presentation';

afterEach(cleanup);

/** The real bridge conversion of the Rust-generated Camera schema, as the native editor sees it. */
function nativeCamera(): ComponentSchema {
  const schema = JSON.parse(readFileSync(resolve(__dirname, '../../../../../schemas/Camera.schema.json'), 'utf8'));
  const read = {
    project: { id: '00000000000000000000000001', name: 'Cameras', scenes: {} },
    revision: 1,
    can_redo: false,
    history: [],
    applied: 0,
    schemas: { Camera: schema },
    console: [],
    viewport_error: null,
  } as unknown as EngineRead;
  return snapshotFromEngine(read).schemas.Camera!;
}

const NATIVE = nativeCamera();
const LEGACY = { fov_degrees: 60, near: 0.1, far: 100 };

/** Field of view carries no "Unused" tag: it is live, or the projection is not well-formed. */
function expectFieldOfViewLive(camera: { getByRole: (role: string, options: { name: string }) => HTMLElement }) {
  const fov = camera.getByRole('textbox', { name: 'Field of view' });
  expect(fov.closest('.field')?.querySelector('.control__tag')).toBeNull();
  expect(description(fov)).not.toContain('Not used');
}

/** Mounts the real editor on the sample fixture with this value on the Camera Rig. */
async function inspect(value: unknown, diagnostics: readonly Diagnostic[] = []) {
  const base = fixtureSnapshot('sample');
  const rig = Object.values(base.entities).find((entity) => entity.name === 'Camera Rig')!;
  const snapshot: BridgeSnapshot = {
    ...base,
    entities: {
      ...base.entities,
      [rig.id]: {
        ...rig,
        components: rig.components.map((c) => (c.type === 'Camera' ? { ...c, value: value as Record<string, unknown> } : c)),
      },
    },
    diagnostics: [...base.diagnostics, ...diagnostics.map((d) => ({ ...d, entity: rig.id }))],
  };
  const bridge: EditorBridge = {
    ...createFixtureBridge('sample'),
    getSnapshot: () => snapshot,
    subscribe: () => () => undefined,
  };
  render(<App resolution={{ kind: 'bridge', bridge }} />);
  fireEvent.click(screen.getByRole('treeitem', { name: /^Camera Rig/ }));
  const inspector = document.querySelector<HTMLElement>('[data-region="inspector"]')!;
  return within(await waitFor(() => within(inspector).getByRole('region', { name: 'Camera' })));
}

const description = (element: Element) =>
  (element.getAttribute('aria-describedby') ?? '')
    .split(' ')
    .filter(Boolean)
    .map((id) => document.getElementById(id)?.textContent ?? '')
    .join(' ');

/** Visible row text: label, then the value box's text and input value. */
type Queries = { getAllByRole: (role: string) => HTMLElement[] };
const regionOf = (camera: Queries) => camera.getAllByRole('textbox')[0]!.closest<HTMLElement>('section.component')!;
const rows = (camera: Queries) =>
  Array.from(regionOf(camera).querySelectorAll<HTMLElement>('.field')).map((row) => {
    const label = row.querySelector('.field__label')?.textContent ?? '';
    const box = row.querySelector('.field__value')!;
    const input = box.querySelector('input');
    return `${label}: ${input ? input.value : ''} ${box.textContent ?? ''}`.replace(/\s+/g, ' ').trim();
  });

describe('native Camera schema', () => {
  it('reaches the Inspector with projection as an optional perspective/orthographic union', () => {
    expect(NATIVE.order).toEqual(['projection', 'fov_degrees', 'near', 'far']);
    const projection = NATIVE.properties.projection!;
    expect(projection).toMatchObject({
      type: 'tagged-union',
      discriminator: 'kind',
      optional: true,
      default: { kind: 'perspective' },
    });
    // Units come from Rust metadata, not from the Inspector profile.
    expect(['fov_degrees', 'near', 'far'].map((key) => (NATIVE.properties[key] as { 'x-incant-unit'?: string })['x-incant-unit']))
      .toEqual(['°', 'm', 'm']);
    expect(Object.keys((projection as { variants: object }).variants)).toEqual(['perspective', 'orthographic']);
  });

  it('is mirrored exactly by the browser fixture', () => {
    expect(fixtureSnapshot('sample').schemas.Camera).toEqual(NATIVE);
  });
});

describe('Camera Inspector', () => {
  it('shows a legacy camera as perspective, tagged as the default rather than authored', async () => {
    const camera = await inspect(LEGACY);
    const projection = camera.getByRole('textbox', { name: 'Projection' });
    expect(projection).toHaveProperty('value', 'Perspective');
    expect(description(projection)).toContain('Not authored. Cameras without a projection use the schema default.');
    expect(rows(camera)).toEqual([
      'Projection: Perspective Default',
      'Field of view: 60 °',
      'Near clip: 0.1 m',
      'Far clip: 100 m',
    ]);
    // Field of view is live for perspective: no "Unused" tag, and no empty variant group.
    expect(description(camera.getByRole('textbox', { name: 'Field of view' }))).not.toContain('Not used');
    expect(camera.queryByRole('group', { name: 'Projection' })).toBeNull();
  });

  it('shows explicit perspective without the default tag', async () => {
    const camera = await inspect({ ...LEGACY, projection: { kind: 'perspective' } });
    expect(rows(camera)[0]).toBe('Projection: Perspective');
    expect(description(camera.getByRole('textbox', { name: 'Projection' }))).not.toContain('Not authored');
  });

  it('shows orthographic projection with its vertical size in metres and marks field of view unused', async () => {
    const camera = await inspect({ ...LEGACY, projection: { kind: 'orthographic', vertical_size: 8 } });
    expect(rows(camera)).toEqual([
      'Projection: Orthographic',
      'Vertical size: 8 m',
      'Field of view: 60 °Unused',
      'Near clip: 0.1 m',
      'Far clip: 100 m',
    ]);
    const size = camera.getByRole('textbox', { name: 'Vertical size' });
    expect(size).toHaveProperty('readOnly', true);
    expect(description(size)).toContain('Visible world-space height. Width follows the output aspect ratio.');
    const group = camera.getByRole('group', { name: 'Projection' });
    expect(within(group).getByRole('textbox', { name: 'Vertical size' })).toBe(size);
    expect(description(camera.getByRole('textbox', { name: 'Field of view' }))).toContain(
      'Not used by orthographic projection. Kept for switching back to perspective.',
    );
  });

  it.each([
    ['an unknown kind', { kind: 'isometric', vertical_size: 8 }, 'Unknown projection kind "isometric". Expected perspective or orthographic; raw value:'],
    ['a missing kind', { vertical_size: 8 }, 'No “kind” is set, so the projection cannot be shown. Expected perspective or orthographic; raw value:'],
    ['null', null, 'Cannot show as a projection (perspective or orthographic); raw value:'],
    ['a string', 'orthographic', 'Cannot show as a projection (perspective or orthographic); raw value:'],
  ])('reports %s explicitly instead of assuming perspective', async (_name, projection, message) => {
    const camera = await inspect({ ...LEGACY, projection });
    const notice = camera.getByRole('group', { name: 'Projection' });
    expect(notice.textContent).toContain(message);
    expect(notice.querySelector('code')?.textContent).toBe(JSON.stringify(projection));
    expect(notice.tabIndex).toBe(0);
    expect(camera.queryByRole('textbox', { name: 'Projection' })).toBeNull();
    // Never guesses a mode for the field-of-view tag from a malformed projection.
    expect(description(camera.getByRole('textbox', { name: 'Field of view' }))).not.toContain('Not used');
  });

  it.each([
    ['missing', { kind: 'orthographic' }, 'undefined'],
    ['text', { kind: 'orthographic', vertical_size: '8' }, '"8"'],
  ])('reports a %s vertical size as a mismatch with its raw value', async (_name, projection, raw) => {
    const camera = await inspect({ ...LEGACY, projection });
    expect(camera.getByRole('textbox', { name: 'Projection' })).toHaveProperty('value', 'Orthographic');
    const size = camera.getByRole('group', { name: 'Vertical size' });
    expect(size.textContent).toContain('Cannot show as a number; raw value:');
    expect(size.querySelector('code')?.textContent).toBe(raw);
    expectFieldOfViewLive(camera);
  });

  it('flags a field that is not part of the orthographic projection', async () => {
    const camera = await inspect({ ...LEGACY, projection: { kind: 'orthographic', vertical_size: 8, zoom: 2 } });
    expect(camera.getByRole('group', { name: 'Projection' }).textContent).toContain(
      'Field zoom is not part of the “orthographic” projection: 2',
    );
    expectFieldOfViewLive(camera);
  });

  it('shows a non-positive size as authored and lets the engine diagnostic explain it', async () => {
    const problem: Diagnostic = {
      id: 'ortho-size',
      severity: 'error',
      message: 'orthographic vertical size must be finite and positive',
      entity: null,
      component: 'Camera',
      path: '/projection/vertical_size',
    };
    const camera = await inspect({ ...LEGACY, projection: { kind: 'orthographic', vertical_size: -2 } }, [problem]);
    const size = camera.getByRole('textbox', { name: 'Vertical size' });
    expect(size).toHaveProperty('value', '-2');
    expect(size.getAttribute('aria-invalid')).toBe('true');
    expect(description(size)).toContain('orthographic vertical size must be finite and positive');
    expectFieldOfViewLive(camera);
  });

  it('shows tiny clip distances and vertical sizes as nonzero values', async () => {
    const camera = await inspect({
      fov_degrees: 60,
      near: 0.000001,
      far: 0.00042,
      projection: { kind: 'orthographic', vertical_size: 0.0000025 },
    });
    expect(rows(camera)).toEqual([
      'Projection: Orthographic',
      'Vertical size: 2.5e-6 m',
      'Field of view: 60 °Unused',
      'Near clip: 1e-6 m',
      'Far clip: 4.2e-4 m',
    ]);
    // The exact authored number is on hover when the compact form differs.
    expect(camera.getByRole('textbox', { name: 'Near clip' }).getAttribute('title')).toBe('0.000001');
    expect(camera.getByRole('textbox', { name: 'Field of view' }).getAttribute('title')).toBeNull();
  });

  it('keeps the visible distinction between the schema default and authored data', async () => {
    const legacy = await inspect(LEGACY);
    expect(legacy.getAllByText('Default')).toHaveLength(1);
    cleanup();
    const authored = await inspect({ ...LEGACY, projection: { kind: 'perspective' } });
    expect(authored.queryByText('Default')).toBeNull();
  });

  it('is read-only, reachable by Tab in reading order, and offers no projection controls', async () => {
    const camera = await inspect({ ...LEGACY, projection: { kind: 'orthographic', vertical_size: 8 } });
    const region = regionOf(camera);
    expect(region.querySelectorAll('button:not(.component__toggle), select, [role="combobox"], [role="radio"]')).toHaveLength(0);
    const inputs = Array.from(region.querySelectorAll<HTMLInputElement>('input'));
    expect(inputs.every((input) => input.readOnly)).toBe(true);
    expect(inputs.map((input) => input.getAttribute('aria-labelledby') && document.getElementById(input.getAttribute('aria-labelledby')!)?.textContent))
      .toEqual(['Projection', 'Vertical size', 'Field of view', 'Near clip', 'Far clip']);
    expect(inputs.every((input) => input.tabIndex === 0)).toBe(true);
    expect(screen.getByText('Read-only')).toBeTruthy();
  });

  it('has no axe violations for legacy, orthographic and malformed cameras', async () => {
    for (const value of [
      LEGACY,
      { ...LEGACY, projection: { kind: 'orthographic', vertical_size: 8 } },
      { ...LEGACY, projection: { kind: 'isometric' } },
    ]) {
      await inspect(value);
      const panel = document.querySelector<HTMLElement>('[data-region="inspector"]')!;
      const results = await axe.run(panel, { rules: { 'color-contrast': { enabled: false } } });
      expect(results.violations.map((v) => v.id)).toEqual([]);
      cleanup();
    }
  });
});

describe('sample fixture cameras', () => {
  it('holds a legacy rig and an orthographic map camera', () => {
    const cameras = Object.values(fixtureSnapshot('sample').entities).flatMap((entity) =>
      entity.components.filter((c) => c.type === 'Camera').map((c) => [entity.name, c.value.projection ?? null]),
    );
    expect(cameras).toEqual([
      ['Camera Rig', null],
      ['Map Camera', { kind: 'orthographic', vertical_size: 8 }],
    ]);
  });
});

describe('orthographic detection for the field-of-view tag', () => {
  it.each([
    ['well-formed', { kind: 'orthographic', vertical_size: 8 }, true],
    ['tiny but positive', { kind: 'orthographic', vertical_size: 1e-6 }, true],
    ['legacy (omitted)', undefined, false],
    ['perspective', { kind: 'perspective' }, false],
    ['missing size', { kind: 'orthographic' }, false],
    ['text size', { kind: 'orthographic', vertical_size: '8' }, false],
    ['null size', { kind: 'orthographic', vertical_size: null }, false],
    ['zero size', { kind: 'orthographic', vertical_size: 0 }, false],
    ['negative size', { kind: 'orthographic', vertical_size: -2 }, false],
    ['extra field', { kind: 'orthographic', vertical_size: 8, zoom: 2 }, false],
    ['array', ['orthographic', 8], false],
  ])('%s -> %s', (_name, projection, expected) => {
    const camera: Record<string, unknown> = { ...LEGACY };
    if (projection !== undefined) camera.projection = projection;
    expect(isOrthographicCamera(camera)).toBe(expected);
  });

  it('does not change the authored value it inspects', () => {
    const projection = { kind: 'orthographic', vertical_size: '8', zoom: 2 };
    const camera = { ...LEGACY, projection };
    const before = JSON.stringify(camera);
    isOrthographicCamera(camera);
    expect(JSON.stringify(camera)).toBe(before);
  });
});

describe('number formatting', () => {
  it.each([
    [0, '0'],
    [-0, '0'],
    [42, '42'],
    [0.1, '0.1'],
    [1.5707963, '1.5708'],
    [-3.5, '-3.5'],
    [0.01, '0.01'],
    [0.000001, '1e-6'],
    [-0.0000025, '-2.5e-6'],
    [0.0012346, '1.235e-3'],
    [0.00999, '9.99e-3'],
    [1e-300, '1e-300'],
    [123456.789, '123456.789'],
    [2.5e12, '2.5e12'],
  ])('%s -> %s', (value, shown) => {
    expect(formatNumber(value)).toBe(shown);
  });

  it('never shows a nonzero value as zero', () => {
    for (const value of [1e-4, 4.9e-5, -1e-9, 5e-324, -5e-324]) {
      expect(Number(formatNumber(value))).not.toBe(0);
      expect(Math.sign(Number(formatNumber(value)))).toBe(Math.sign(value));
    }
  });

  it('keeps small and negative-small vector components visible', () => {
    const vector = { type: 'array', items: { type: 'number' }, minItems: 3, maxItems: 3, 'x-incant-unit': 'm' } as const;
    render(
      <FieldView
        name="translation"
        schema={vector}
        value={[0.000001, -0.0000025, 0]}
        path={['translation']}
        ctx={{ entity: '01J9ZF1XTR0000000000000042', component: 'Transform', diagnostics: [] }}
      />,
    );
    const axes = screen.getAllByRole('textbox') as HTMLInputElement[];
    expect(axes.map((input) => input.value)).toEqual(['1e-6', '-2.5e-6', '0']);
    expect(axes.map((input) => input.getAttribute('title'))).toEqual(['0.000001', '-0.0000025', null]);
  });
});

describe('schema defaults for omitted fields', () => {
  const projection = NATIVE.properties.projection!;
  it('uses the bridge-supplied Camera projection default', () => {
    expect(presentableDefault(projection)).toEqual({ value: { kind: 'perspective' } });
  });

  it.each([
    ['no default', undefined],
    ['an unknown variant', { kind: 'isometric' }],
    ['a non-record', 'perspective'],
    ['null', null],
  ])('ignores %s and leaves the field "Not set"', async (_name, value) => {
    const { default: _omit, ...rest } = projection as Record<string, unknown>;
    const schema = (value === undefined ? rest : { ...rest, default: value }) as unknown as typeof projection;
    expect(presentableDefault(schema)).toBeNull();
    render(
      <FieldView name="projection" schema={schema} value={undefined} path={['projection']}
        ctx={{ entity: '01J9ZF1XTR0000000000000042', component: 'Camera', diagnostics: [], value: LEGACY }} />,
    );
    expect(screen.getByRole('textbox', { name: 'Projection' })).toHaveProperty('value', 'Not set');
    expect(screen.queryByText('Default')).toBeNull();
  });

  it('presents a numeric default with the Default tag and leaves authored numbers untagged', () => {
    const number = { type: 'number', optional: true, default: 0.5, 'x-incant-unit': 'm' } as const;
    const ctx = { entity: '01J9ZF1XTR0000000000000042', component: 'Example', diagnostics: [] };
    const { unmount } = render(<FieldView name="radius" schema={number} value={undefined} path={['radius']} ctx={ctx} />);
    expect(screen.getByRole('textbox', { name: 'Radius' })).toHaveProperty('value', '0.5');
    expect(screen.getByText('Default')).toBeTruthy();
    unmount();
    render(<FieldView name="radius" schema={number} value={0.5} path={['radius']} ctx={ctx} />);
    expect(screen.queryByText('Default')).toBeNull();
  });
});
