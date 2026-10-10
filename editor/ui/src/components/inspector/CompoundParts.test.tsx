import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import axe from 'axe-core';
import { afterEach, describe, expect, it } from 'vitest';
import { App } from '../../App';
import { snapshotFromEngine } from '../../../../bridge/native';
import type { EngineRead } from '../../../../bridge/native';
import type { Diagnostic, FieldSchema, Ulid } from '../../bridge/contract';
import { createFixtureBridge, fixtureId } from '../../bridge/fixture';
import { FieldView, fieldDomId } from './FieldView';

afterEach(cleanup);

const ENTITY = '01J9ZF1XTR0000000000000042' as Ulid;

/** The real schemas/Collider.schema.json through the native bridge conversion. */
const COLLIDER = (() => {
  const raw = JSON.parse(readFileSync(resolve(__dirname, '../../../../../schemas/Collider.schema.json'), 'utf8'));
  const read = {
    project: { id: '00000000000000000000000001', name: 'Compound', scenes: {} },
    revision: 1,
    can_redo: false,
    history: [],
    applied: 0,
    schemas: { Collider: raw },
    console: [],
    viewport_error: null,
  } as unknown as EngineRead;
  return snapshotFromEngine(read).schemas.Collider!;
})();
const SHAPE = COLLIDER.properties.shape!;

const id = (n: number) => fixtureId(50000 + n);
const MIXED = {
  type: 'compound',
  parts: [
    { id: id(1), translation: [0, 2.4, 0], rotation: [0, 0, 0, 1], shape: { type: 'box', half_extents: [1.4, 0.2, 0.4] } },
    { id: id(2), translation: [-0.3, 0.18, 0.46], rotation: [0, 0, 0, 1], shape: { type: 'sphere', radius: 0.18 } },
    {
      id: id(3),
      translation: [0.82, 0.55, 0],
      rotation: [0, 0, -0.5, 0.866025],
      shape: { type: 'capsule', half_height: 0.32, radius: 0.04 },
    },
  ],
};

function show(value: unknown, diagnostics: readonly Diagnostic[] = [], entity: string = ENTITY) {
  return render(
    <FieldView name="shape" schema={SHAPE} value={value} path={['shape']} ctx={{ entity, component: 'Collider', diagnostics }} />,
  );
}

const diagnostic = (path: string, message = 'Engine says no.', severity: Diagnostic['severity'] = 'error'): Diagnostic => ({
  id: `${severity}:${path}`,
  severity,
  message,
  entity: ENTITY,
  component: 'Collider',
  path,
});

const parts = () => screen.getByRole('listbox', { name: 'Parts' });
/** Each row's accessible name, computed from its visible text and hidden words. */
const expectNames = (names: readonly string[]) =>
  names.forEach((name, index) => expect(within(parts()).getByRole('option', { name })).toBe(rows()[index]));
const rows = () => within(parts()).getAllByRole('option');
const detail = () => screen.getByRole('group', { name: /^Part \d+ of \d+/ });
const value = (name: string | RegExp, scope: HTMLElement = detail()) =>
  (within(scope).getByRole('textbox', { name }) as HTMLInputElement).value;
const axes = (name: RegExp) =>
  within(within(detail()).getByRole('group', { name })).getAllByRole('textbox').map((input) => (input as HTMLInputElement).value);

describe('compound schema reaches the Inspector intact', () => {
  it('as a list of part records whose shape is a primitive-only union', () => {
    const compound = (SHAPE as { variants: Record<string, FieldSchema> }).variants.compound!;
    const list = (compound as { properties: Record<string, FieldSchema> }).properties.parts!;
    expect(list).toMatchObject({ type: 'array', minItems: 1, maxItems: 64, items: { type: 'object' } });
    const part = (list as { items: { properties: Record<string, FieldSchema> } }).items.properties;
    expect(Object.keys((part.shape as { variants: object }).variants)).toEqual(['box', 'sphere', 'capsule']);
  });
});

describe('compound parts', () => {
  it('lists mixed parts compactly and shows the selected part’s fields in authored order', () => {
    const { container } = show(MIXED);
    expect((screen.getByRole('textbox', { name: 'Shape' }) as HTMLInputElement).value).toBe('compound');
    expect(screen.getAllByRole('textbox', { name: 'Shape' })).toHaveLength(1); // the part's is "Primitive"
    expect(container.querySelector('.object-list__summary')?.textContent).toBe('3 parts · 1 box, 1 sphere, 1 capsule');
    expectNames([
      'Part 1: box, half extents 1.4 × 0.2 × 0.4 m',
      'Part 2: sphere, radius 0.18 m',
      'Part 3: capsule, half height 0.32, radius 0.04 m',
    ]);
    expect(rows().some((row) => row.hasAttribute('aria-label'))).toBe(false);
    expect([...container.querySelectorAll('.object-list__dims')].map((dims) => dims.textContent)).toEqual([
      '½ 1.4 × 0.2 × 0.4 m',
      'r 0.18 m',
      '½h 0.32 · r 0.04 m',
    ]);
    // No compound notice: every authored value has a presentation.
    expect(container.querySelector('.control--notice')).toBeNull();

    expect(detail().textContent).toContain('Part 1 of 3');
    expect(detail().querySelector('.object-list__path')?.textContent).toContain('/shape/parts/0');
    const labels = [...detail().querySelectorAll(':scope > .field > .field__label')].map((label) => label.firstChild?.textContent);
    expect(labels).toEqual(['ID', 'Offset', 'Rotation', 'Primitive']);
    const partId = within(detail()).getByRole('textbox', { name: 'ID' }) as HTMLInputElement;
    expect(partId.value).toBe(id(1));
    expect(partId.readOnly).toBe(true);
    expect(partId.classList.contains('mono')).toBe(true);
    expect(partId.closest('.control')?.querySelector('.icon')).toBeNull(); // its own ID, not a reference
    expect(axes(/Offset/)).toEqual(['0', '2.4', '0']);
    expect(axes(/Rotation/)).toEqual(['0', '0', '0', '1']);
    expect(value('Primitive')).toBe('box');
    expect(axes(/Half extents/)).toEqual(['1.4', '0.2', '0.4']);
    expect(partId.id).toBe(fieldDomId(ENTITY, 'Collider', ['shape', 'parts', '0', 'id']));
  });

  it('is one Tab stop; arrows, Page and Home/End choose a part and the detail follows', () => {
    show(MIXED);
    expect(rows().map((row) => row.tabIndex)).toEqual([0, -1, -1]);
    rows()[0]!.focus();
    fireEvent.keyDown(rows()[0]!, { key: 'ArrowDown' });
    expect(document.activeElement).toBe(rows()[1]);
    expect(rows()[1]!.getAttribute('aria-selected')).toBe('true');
    expect(rows().map((row) => row.tabIndex)).toEqual([-1, 0, -1]);
    expect(detail().textContent).toContain('Part 2 of 3');
    expect(value('Radius')).toBe('0.18');

    fireEvent.keyDown(rows()[1]!, { key: 'End' });
    expect(document.activeElement).toBe(rows()[2]);
    expect(value('Primitive')).toBe('capsule');
    expect(axes(/Rotation/)).toEqual(['0', '0', '-0.5', '0.866']);
    fireEvent.keyDown(rows()[2]!, { key: 'ArrowDown' }); // clamps at the end
    expect(document.activeElement).toBe(rows()[2]);
    fireEvent.keyDown(rows()[2]!, { key: 'Home' });
    expect(document.activeElement).toBe(rows()[0]);
    fireEvent.keyDown(rows()[0]!, { key: 'PageDown' });
    expect(document.activeElement).toBe(rows()[2]);
    fireEvent.click(rows()[1]!);
    expect(document.activeElement).toBe(rows()[1]);
    expect(detail().textContent).toContain('Part 2 of 3');
  });

  it('says plainly when a part’s shape tag is missing or unknown, keeping the raw value', () => {
    const value = {
      type: 'compound',
      parts: [
        { id: id(1), translation: [0, 0, 0], rotation: [0, 0, 0, 1], shape: { half_extents: [0.5, 0.5, 0.5] } },
        { id: id(2), translation: [0, 0, 0], rotation: [0, 0, 0, 1], shape: { type: 'cylinder', radius: 4 } },
        { id: id(3), translation: [0, 0, 0], rotation: [0, 0, 0, 1], shape: { type: 'compound', parts: [] } },
      ],
    };
    const { container } = show(value);
    expect(container.querySelector('.object-list__summary')?.textContent).toBe('3 parts · 3 unreadable');
    expectNames([
      'Part 1: primitive has no type',
      'Part 2: unknown primitive type "cylinder"',
      'Part 3: unknown primitive type "compound"', // nesting is not a primitive variant
    ]);
    const missing = within(detail()).getByRole('group', { name: 'Primitive' });
    expect(missing.textContent).toContain('No “type” is set');
    expect(missing.textContent).toContain('Expected box, sphere or capsule');
    expect(missing.textContent).toContain('"half_extents":[0.5,0.5,0.5]');
    expect(missing.id).toBe(fieldDomId(ENTITY, 'Collider', ['shape', 'parts', '0', 'shape']));
    fireEvent.click(rows()[1]!);
    const unknown = within(detail()).getByRole('group', { name: 'Primitive' });
    expect(unknown.textContent).toContain('Unknown primitive type "cylinder"');
    expect(unknown.textContent).toContain('"radius":4');
    expect(unknown.tabIndex).toBe(0);
  });

  it('shows malformed parts in place: non-objects, missing fields and fields outside the schema', () => {
    const value = {
      type: 'compound',
      parts: [
        'post-04',
        { id: id(2), translation: [0, 0, 0], shape: { type: 'sphere', radius: 1 } },
        { id: id(3), translation: [0, 0, 0], rotation: [0, 0, 0, 1], shape: { type: 'sphere', radius: 1 }, material: 'oak' },
      ],
    };
    show(value);
    expectNames(['Part 1: not an object: "post-04"']);
    expect(detail().textContent).toContain('Cannot show part 1 as an object; raw value:');
    expect(detail().textContent).toContain('"post-04"');
    fireEvent.click(rows()[1]!);
    const rotation = within(detail()).getByRole('group', { name: 'Rotation' });
    expect(rotation.textContent).toContain('Cannot show as a list of numbers');
    fireEvent.click(rows()[2]!);
    expect(detail().textContent).toContain('Field material is not in the part schema: "oak"');
  });

  it('refuses a non-list parts value instead of guessing', () => {
    show({ type: 'compound', parts: { id: id(1) } });
    expect(screen.queryByRole('listbox')).toBeNull();
    const notice = screen.getByRole('group', { name: 'Parts' });
    expect(notice.textContent).toContain('Cannot show as a list; raw value:');
    expect(notice.id).toBe(fieldDomId(ENTITY, 'Collider', ['shape', 'parts']));
  });

  it('places part, axis and list diagnostics at their exact paths and starts on the first problem', () => {
    show(MIXED, [
      diagnostic('/shape/parts/2/shape/radius', 'compound part 2: collider dimensions must be 0.001..10000 meters'),
      diagnostic('/shape/parts/2/rotation/3', 'Rotation w is not normalized.', 'warning'),
      diagnostic('/shape/parts/2', 'compound part 2 requires a unique valid ULID'),
      diagnostic('/shape/parts', 'compound collider requires 1..64 primitive parts'),
      diagnostic('/shape/parts/7/id', 'Addressed past the end of the list.'),
    ]);
    // List-level problems, including one addressed to a part that does not exist, stay on the list row.
    const listRow = parts().closest('.object-list')!.querySelector(':scope > .field')!;
    expect(listRow.textContent).toContain('requires 1..64 primitive parts');
    expect(listRow.textContent).toContain('/shape/parts/7/id');
    expect(parts().getAttribute('aria-invalid')).toBe('true');
    expect(within(parts()).getByRole('option', { name: 'Part 3: capsule, half height 0.32, radius 0.04 m, 2 errors, 1 warning' })).toBe(rows()[2]);
    expect(rows()[2]!.getAttribute('aria-selected')).toBe('true');
    expect(rows().map((row) => row.tabIndex)).toEqual([-1, -1, 0]);
    // Part-level message under the caption; axis and field messages on their rows.
    expect(detail().querySelector(':scope > .field__problems')?.textContent).toContain('requires a unique valid ULID');
    const radius = within(detail()).getByRole('textbox', { name: 'Radius' });
    expect(radius.getAttribute('aria-invalid')).toBe('true');
    expect(radius.closest('.field')?.textContent).toContain('collider dimensions must be');
    const rotation = within(detail()).getByRole('group', { name: 'Rotation' });
    expect(rotation.closest('.field')?.className).toContain('field--warning');
    expect(rotation.closest('.field')?.textContent).toContain('/shape/parts/2/rotation/3');
  });

  it('keeps 64 parts in one bounded list with a single Tab stop and one rendered part', async () => {
    const many = {
      type: 'compound',
      parts: Array.from({ length: 64 }, (_, i) => ({
        id: id(i),
        translation: [i * 0.1, 0, 0],
        rotation: [0, 0, 0, 1],
        shape: i % 2 ? { type: 'sphere', radius: 0.1 } : { type: 'box', half_extents: [0.1, 0.1, 0.1] },
      })),
    };
    const { container } = show(many, [diagnostic('/shape/parts/41/shape/radius')]);
    expect(rows()).toHaveLength(64);
    expect(container.querySelector('.object-list__summary')?.textContent).toBe('64 parts · 32 box, 32 sphere');
    expect(rows().filter((row) => row.tabIndex === 0).map((row) => row.dataset.index)).toEqual(['41']);
    expect(detail().textContent).toContain('Part 42 of 64');
    // Only the selected part's fields exist: ID, 3 offset, 4 rotation, shape tag and radius.
    expect(within(detail()).getAllByRole('textbox')).toHaveLength(10);
    const results = await axe.run(container, { rules: { 'color-contrast': { enabled: false } } });
    expect(results.violations.map((v) => v.id)).toEqual([]);
  });

  it('starts each entity at its own first problem rather than keeping another entity’s selection', () => {
    const { rerender } = show(MIXED);
    fireEvent.click(rows()[2]!);
    const other = '01J9ZF1XTR0000000000000043';
    rerender(<FieldView name="shape" schema={SHAPE} value={MIXED} path={['shape']} ctx={{ entity: other, component: 'Collider', diagnostics: [] }} />);
    expect(detail().textContent).toContain('Part 1 of 3');
  });

  it('keeps the selected stable part and keyboard focus through reordered snapshots', () => {
    const { rerender } = show(MIXED);
    fireEvent.click(rows()[1]!);
    const selectedRow = rows()[1]!;
    const reordered = { ...MIXED, parts: [MIXED.parts[1], MIXED.parts[2], MIXED.parts[0]] };
    const update = (next: unknown) => rerender(
      <FieldView name="shape" schema={SHAPE} value={next} path={['shape']} ctx={{ entity: ENTITY, component: 'Collider', diagnostics: [] }} />,
    );
    update(reordered);
    expect(value('ID')).toBe(id(2));
    expect(value('Primitive')).toBe('sphere');
    expect(detail().textContent).toContain('Part 1 of 3');
    expect(document.activeElement).toBe(selectedRow);
    expect(rows()[0]).toBe(selectedRow);
    expect(rows()[0]!.getAttribute('aria-selected')).toBe('true');
    expect(within(detail()).getByRole('textbox', { name: 'Radius' }).id).toBe(
      fieldDomId(ENTITY, 'Collider', ['shape', 'parts', '0', 'shape', 'radius']),
    );
    // Removing that part selects its replacement, without retaining stale fields.
    update({ ...MIXED, parts: [MIXED.parts[2], MIXED.parts[0]] });
    expect(value('ID')).toBe(id(3));
    expect(value('Primitive')).toBe('capsule');
    expect(rows().filter((row) => row.tabIndex === 0)).toHaveLength(1);
  });

  it('has no accessibility violations for mixed parts', async () => {
    const { container } = show(MIXED);
    const results = await axe.run(container, { rules: { 'color-contrast': { enabled: false } } });
    expect(results.violations.map((v) => v.id)).toEqual([]);
  });
});

describe('compound parts in the editor', () => {
  it('shows the sample arch, handcart and 64-part rubble read-only', async () => {
    render(<App resolution={{ kind: 'bridge', bridge: createFixtureBridge('sample') }} />);
    fireEvent.click(screen.getByRole('treeitem', { name: /^Rubble Pile/ }));
    const inspector = await waitFor(() => {
      const element = document.querySelector<HTMLElement>('[data-region="inspector"]')!;
      expect(element.textContent).toContain('Rubble Pile');
      return element;
    });
    const collider = within(inspector).getByRole('region', { name: 'Collider' });
    expect(within(collider).getAllByRole('option')).toHaveLength(64);
    expect([...collider.querySelectorAll('input')].every((input) => input.readOnly)).toBe(true);
    expect([...collider.querySelectorAll('button')].map((b) => b.className)).toEqual(['component__toggle']);
    // Material and collision rows are unchanged beneath the parts.
    expect(within(collider).getByRole('group', { name: 'Collision' })).toBeTruthy();

    fireEvent.click(screen.getByRole('treeitem', { name: /^Handcart/ }));
    await waitFor(() => expect(document.querySelector('[data-region="inspector"]')!.textContent).toContain('Handcart'));
    const cart = within(document.querySelector<HTMLElement>('[data-region="inspector"]')!).getByRole('region', { name: 'Collider' });
    expect(cart.querySelector('.object-list__summary')?.textContent).toBe('4 parts · 1 box, 2 sphere, 1 capsule');
    expect(within(cart).getByText('Group 2')).toBeTruthy();
  });

  it('follows a Problems link to the exact field of a part that was not selected', async () => {
    render(<App resolution={{ kind: 'bridge', bridge: createFixtureBridge('sample') }} />);
    fireEvent.click(screen.getByRole('button', { name: /compound part 3 requires a unique valid ULID/ }));
    await waitFor(() => expect(screen.getByRole('treeitem', { name: /^Broken Railing/ }).getAttribute('aria-selected')).toBe('true'));
    // The railing starts on its first problem, part 1.
    await waitFor(() => expect(detail().textContent).toContain('Part 1 of 6'));
    fireEvent.click(await waitFor(() => screen.getByRole('link', { name: /Collider\/shape\/parts\/3\/id/ })));
    await waitFor(() => expect(document.activeElement?.id).toMatch(/-Collider_shape_parts_3_id$/));
    expect(detail().textContent).toContain('Part 4 of 6');
    expect(rows()[3]!.getAttribute('aria-selected')).toBe('true');
  });
});
