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
    expect(NATIVE.order).toEqual(['fov_degrees', 'near', 'far']);
    const projection = NATIVE.properties.projection!;
    expect(projection).toMatchObject({ type: 'tagged-union', discriminator: 'kind', optional: true });
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
    expect(description(projection)).toContain('Not authored. Cameras without a projection use perspective.');
    expect(rows(camera)).toEqual([
      'Field of view: 60 °',
      'Near clip: 0.1 m',
      'Far clip: 100 m',
      'Projection: Perspective Default',
    ]);
    // Field of view is live for perspective: no "Unused" tag, and no empty variant group.
    expect(description(camera.getByRole('textbox', { name: 'Field of view' }))).not.toContain('Not used');
    expect(camera.queryByRole('group', { name: 'Projection' })).toBeNull();
  });

  it('shows explicit perspective without the default tag', async () => {
    const camera = await inspect({ ...LEGACY, projection: { kind: 'perspective' } });
    expect(rows(camera).at(-1)).toBe('Projection: Perspective');
    expect(description(camera.getByRole('textbox', { name: 'Projection' }))).not.toContain('Not authored');
  });

  it('shows orthographic projection with its vertical size in metres and marks field of view unused', async () => {
    const camera = await inspect({ ...LEGACY, projection: { kind: 'orthographic', vertical_size: 8 } });
    expect(rows(camera)).toEqual([
      'Field of view: 60 °Unused',
      'Near clip: 0.1 m',
      'Far clip: 100 m',
      'Projection: Orthographic',
      'Vertical size: 8 m',
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
  });

  it('flags a field that is not part of the orthographic projection', async () => {
    const camera = await inspect({ ...LEGACY, projection: { kind: 'orthographic', vertical_size: 8, zoom: 2 } });
    expect(camera.getByRole('group', { name: 'Projection' }).textContent).toContain(
      'Field zoom is not part of the “orthographic” projection: 2',
    );
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
  });

  it('is read-only, reachable by Tab in reading order, and offers no projection controls', async () => {
    const camera = await inspect({ ...LEGACY, projection: { kind: 'orthographic', vertical_size: 8 } });
    const region = regionOf(camera);
    expect(region.querySelectorAll('button:not(.component__toggle), select, [role="combobox"], [role="radio"]')).toHaveLength(0);
    const inputs = Array.from(region.querySelectorAll<HTMLInputElement>('input'));
    expect(inputs.every((input) => input.readOnly)).toBe(true);
    expect(inputs.map((input) => input.getAttribute('aria-labelledby') && document.getElementById(input.getAttribute('aria-labelledby')!)?.textContent))
      .toEqual(['Field of view', 'Near clip', 'Far clip', 'Projection', 'Vertical size']);
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
