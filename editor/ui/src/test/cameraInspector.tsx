import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { expect } from 'vitest';
import { App } from '../App';
import { snapshotFromEngine } from '../../../bridge/native';
import type { EngineRead } from '../../../bridge/native';
import type { BridgeSnapshot, ComponentSchema, Diagnostic, EditorBridge } from '../bridge/contract';
import { createFixtureBridge, fixtureSnapshot } from '../bridge/fixture';

/**
 * Shared helpers for the Camera Inspector tests (CameraFields and PixelCameraFields).
 * Test-only: `src/test` is outside the app's TypeScript project.
 */

/** The real bridge conversion of main's Rust-generated Camera schema, as the native editor sees it. */
export function nativeCamera(): ComponentSchema {
  const schema = JSON.parse(readFileSync(resolve(__dirname, '../../../../schemas/Camera.schema.json'), 'utf8'));
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

export const LEGACY = { fov_degrees: 60, near: 0.1, far: 100 };
export const PIXEL = { kind: 'pixel_perfect', pixels_per_unit: 32, reference_resolution: [320, 180] };

/**
 * Mounts the real editor on the sample fixture with this value on the Camera Rig. A
 * `schema` replaces the fixture's Camera schema (only explicitly synthetic tests do).
 */
export async function inspect(value: unknown, diagnostics: readonly Diagnostic[] = [], schema?: ComponentSchema) {
  const base = fixtureSnapshot('sample');
  const rig = Object.values(base.entities).find((entity) => entity.name === 'Camera Rig')!;
  const snapshot: BridgeSnapshot = {
    ...base,
    schemas: schema ? { ...base.schemas, Camera: schema } : base.schemas,
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

/** The text of an element's accessible description. */
export const description = (element: Element) =>
  (element.getAttribute('aria-describedby') ?? '')
    .split(' ')
    .filter(Boolean)
    .map((id) => document.getElementById(id)?.textContent ?? '')
    .join(' ');

type Queries = { getAllByRole: (role: string) => HTMLElement[] };

/** The Camera component's section. */
export const regionOf = (camera: Queries) => camera.getAllByRole('textbox')[0]!.closest<HTMLElement>('section.component')!;

/** Visible row text: label, then the value box's text and input value. */
export const rows = (camera: Queries) =>
  Array.from(regionOf(camera).querySelectorAll<HTMLElement>('.field')).map((row) => {
    const label = row.querySelector('.field__label')?.textContent ?? '';
    const box = row.querySelector('.field__value')!;
    const input = box.querySelector('input');
    return `${label}: ${input ? input.value : ''} ${box.textContent ?? ''}`.replace(/\s+/g, ' ').trim();
  });

/** Field of view carries no "Unused" tag: it is live, or the projection is not well-formed. */
export function expectFieldOfViewLive(camera: { getByRole: (role: string, options: { name: string }) => HTMLElement }) {
  const fov = camera.getByRole('textbox', { name: 'Field of view' });
  expect(fov.closest('.field')?.querySelector('.control__tag')).toBeNull();
  expect(description(fov)).not.toContain('Not used');
}
