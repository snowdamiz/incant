import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import axe from 'axe-core';
import { afterEach, describe, expect, it } from 'vitest';
import { App } from '../../App';
import { snapshotFromEngine } from '../../../../bridge/native';
import type { EngineRead } from '../../../../bridge/native';
import type { BridgeSnapshot, ComponentSchema, Diagnostic, EditorBridge, FieldSchema, Ulid } from '../../bridge/contract';
import { createFixtureBridge, fixtureSnapshot } from '../../bridge/fixture';
import { FieldView, fieldDomId } from './FieldView';
import { maskDescription, maskGroups, maskSummary, sectionKeys } from './presentation';

afterEach(cleanup);

const ENTITY = '01J9ZF1XTR0000000000000042' as Ulid;
const PHYSICS = ['RigidBody', 'Collider', 'AngularVelocity'] as const;

/** The real bridge conversion of the Rust-generated schemas, as the native editor sees them. */
function nativeSchemas() {
  const schemas = Object.fromEntries(
    PHYSICS.map((name) => [
      name,
      JSON.parse(readFileSync(resolve(__dirname, `../../../../../schemas/${name}.schema.json`), 'utf8')),
    ]),
  );
  const read = {
    project: { id: '00000000000000000000000001', name: 'Physics', scenes: {} },
    revision: 1,
    can_redo: false,
    history: [],
    applied: 0,
    schemas,
    console: [],
    viewport_error: null,
  } as unknown as EngineRead;
  return snapshotFromEngine(read).schemas;
}

const NATIVE = nativeSchemas();
const SHAPE = NATIVE.Collider!.properties.shape!;

function show(
  name: string,
  schema: FieldSchema,
  value: unknown,
  { component = 'Collider', diagnostics = [] as readonly Diagnostic[] } = {},
) {
  return render(
    <FieldView name={name} schema={schema} value={value} path={[name]} ctx={{ entity: ENTITY, component, diagnostics }} />,
  );
}

function described(element: Element): string {
  return (element.getAttribute('aria-describedby') ?? '')
    .split(' ')
    .filter(Boolean)
    .map((id) => document.getElementById(id)?.textContent ?? '')
    .join(' ');
}

const diagnostic = (path: string, message = 'Engine says no.'): Diagnostic => ({
  id: path,
  severity: 'error',
  message,
  entity: ENTITY,
  component: 'Collider',
  path,
});

describe('native physics schemas', () => {
  it('reach the Inspector as a tagged union over box, sphere and capsule', () => {
    expect(SHAPE.type).toBe('tagged-union');
    expect(SHAPE).toMatchObject({ discriminator: 'type' });
    expect(Object.keys((SHAPE as { variants: object }).variants)).toEqual(['box', 'sphere', 'capsule']);
  });

  it('are mirrored exactly by the browser fixture (no drift in what the fixture shows)', () => {
    const fixture = fixtureSnapshot('sample').schemas;
    for (const name of PHYSICS) {
      expect(NATIVE[name]!.properties).toMatchObject(fixture[name]!.properties);
      expect(Object.keys(NATIVE[name]!.properties).sort()).toEqual(Object.keys(fixture[name]!.properties).sort());
      expect(fixture[name]!.order).toEqual(NATIVE[name]!.order);
    }
  });
});

describe('Collider shape union', () => {
  it('selects the box variant from its tag and shows half extents in meters', () => {
    const { container } = show('shape', SHAPE, { type: 'box', half_extents: [0.5, 1, 0.25] });
    const tag = screen.getByRole('textbox', { name: 'Shape' }) as HTMLInputElement;
    expect(tag.value).toBe('box');
    expect(tag.readOnly).toBe(true);
    const variant = screen.getByRole('group', { name: 'Shape' });
    const extents = within(variant).getByRole('group', { name: /Half extents/ });
    expect(extents.getAttribute('aria-labelledby')).toBeTruthy();
    expect(container.querySelector('.field__unit')?.textContent).toContain('m');
    expect(within(extents).getAllByRole('textbox').map((input) => (input as HTMLInputElement).value)).toEqual([
      '0.5',
      '1',
      '0.25',
    ]);
    // The tag is the row's value, not a second "Type" row; other variants' fields are absent.
    expect(screen.queryByText('Type')).toBeNull();
    expect(screen.queryByText('Radius')).toBeNull();
    expect(container.querySelector('.control--notice')).toBeNull();
  });

  it('shows sphere radius and capsule dimensions with the engine’s capsule note', () => {
    show('shape', SHAPE, { type: 'sphere', radius: 1.25 });
    const radius = screen.getByRole('textbox', { name: 'Radius' }) as HTMLInputElement;
    expect(radius.value).toBe('1.25');
    expect(radius.closest('.control--number')?.querySelector('.control__unit')?.textContent).toBe('m');
    cleanup();

    show('shape', SHAPE, { type: 'capsule', half_height: 0.6, radius: 0.15 });
    expect((screen.getByRole('textbox', { name: 'Shape' }) as HTMLInputElement).value).toBe('capsule');
    expect(described(screen.getByRole('textbox', { name: 'Shape' }))).toContain('Capsule along local Y');
    expect((screen.getByRole('textbox', { name: 'Half height' }) as HTMLInputElement).value).toBe('0.6');
    expect((screen.getByRole('textbox', { name: 'Radius' }) as HTMLInputElement).value).toBe('0.15');
    expect(screen.queryByText('Half extents')).toBeNull();
  });

  it('reports an unknown tag as a focusable mismatch and never picks a default variant', () => {
    const { container } = show('shape', SHAPE, { type: 'cylinder', half_height: 0.1, radius: 4 });
    const notice = screen.getByRole('group', { name: 'Shape' });
    expect(notice.textContent).toContain('Unknown shape type "cylinder"');
    expect(notice.textContent).toContain('Expected box, sphere or capsule');
    expect(notice.textContent).toContain('"radius":4');
    expect(notice.tabIndex).toBe(0);
    expect(notice.id).toBe(fieldDomId(ENTITY, 'Collider', ['shape']));
    expect(screen.queryByRole('textbox')).toBeNull();
    expect(container.querySelector('.field-group--variant')).toBeNull();
  });

  it('reports a missing tag, a non-string tag and a non-record value explicitly', () => {
    show('shape', SHAPE, { half_extents: [1, 1, 1] });
    expect(screen.getByRole('group', { name: 'Shape' }).textContent).toContain('No “type” is set');
    expect(screen.queryByText('Half extents')).toBeNull();
    cleanup();
    show('shape', SHAPE, { type: 3, radius: 1 });
    expect(screen.getByRole('group', { name: 'Shape' }).textContent).toContain('Unknown shape type 3');
    expect(screen.queryByText('Radius')).toBeNull();
    cleanup();
    show('shape', SHAPE, 'box');
    expect(screen.getByRole('group', { name: 'Shape' }).textContent).toContain('Cannot show as a shape');
    cleanup();
    // A prototype key is not a variant.
    show('shape', SHAPE, { type: 'constructor' });
    expect(screen.getByRole('group', { name: 'Shape' }).textContent).toContain('Unknown shape type "constructor"');
  });

  it('flags fields the selected variant does not define instead of dropping them', () => {
    show('shape', SHAPE, { type: 'sphere', radius: 1, half_extents: [1, 1, 1] });
    expect(screen.getByText(/is not part of the “sphere” shape/)).toBeTruthy();
    expect(screen.getByText('[1,1,1]')).toBeTruthy();
  });

  it('keeps full nested diagnostic paths on the axis, tag and union rows', () => {
    show('shape', SHAPE, { type: 'box', half_extents: [0.5, -0.5, 0.5] }, {
      diagnostics: [diagnostic('/shape/half_extents/1', 'Half extents must be positive.'), diagnostic('/shape/type', 'Tag note.')],
    });
    const extents = screen.getByRole('group', { name: /Half extents/ });
    const [x, y] = within(extents).getAllByRole('textbox');
    expect(y!.getAttribute('aria-invalid')).toBe('true');
    expect(x!.getAttribute('aria-invalid')).toBeNull();
    expect(y!.id).toBe(fieldDomId(ENTITY, 'Collider', ['shape', 'half_extents', '1']));
    expect(described(y!)).toContain('Half extents must be positive.');
    expect(described(y!)).toContain('/shape/half_extents/1');
    expect(described(screen.getByRole('textbox', { name: 'Shape' }))).toContain('Tag note./shape/type');
    cleanup();

    // With no variant selected, everything under /shape stays on the Shape row.
    show('shape', SHAPE, { type: 'cylinder', radius: 4 }, { diagnostics: [diagnostic('/shape/radius', 'Radius issue.')] });
    const notice = screen.getByRole('group', { name: 'Shape' });
    expect(described(notice)).toContain('Radius issue./shape/radius');
    expect(notice.closest('.field')?.classList.contains('field--error')).toBe(true);
  });
});

describe('collision masks', () => {
  const mask = NATIVE.Collider!.properties.memberships!;

  it('summarizes all, none and partial masks while keeping the raw value', () => {
    for (const [value, summary, spoken] of [
      [4294967295, 'All groups', 'Every collision group, 1 to 32.'],
      [0, 'No groups', 'No collision groups.'],
      [4, 'Group 3', 'Collision group 3 of 32.'],
      [6, '2 groups', 'Collision groups 2, 3 of 32.'],
      [0b1111_0001, '5 groups', 'Collision groups 1, 5 to 8 of 32.'],
    ] as const) {
      const { container } = show('memberships', mask, value);
      const input = screen.getByRole('textbox', { name: 'Memberships' }) as HTMLInputElement;
      expect(input.value).toBe(String(value));
      expect(container.querySelector('.control--mask__summary')?.textContent).toBe(summary);
      expect(described(input)).toContain(spoken);
      expect(container.querySelectorAll('.mask-strip__cell')).toHaveLength(32);
      expect(container.querySelectorAll('.mask-strip__cell.is-set')).toHaveLength(maskGroups(value).length);
      cleanup();
    }
    expect(maskSummary(2 ** 31)).toBe('Group 32');
    expect(maskDescription(2 ** 31 + 1)).toBe('Collision groups 1, 32 of 32.');
  });

  it('refuses values that are not a 32-bit mask', () => {
    for (const value of [-1, 1.5, 2 ** 32, '7']) {
      show('memberships', mask, value);
      expect(screen.getByText(/Cannot show as a 32-bit group mask/)).toBeTruthy();
      cleanup();
    }
  });
});

describe('physics component layout', () => {
  it('keeps a schema order exactly and captions only contiguous profile runs', () => {
    const native = ['shape', 'density', 'friction', 'restitution', 'sensor', 'memberships', 'filter'];
    expect(sectionKeys('Collider', native, true)).toEqual([
      { title: null, keys: ['shape'] },
      { title: 'Material', keys: ['density', 'friction', 'restitution'] },
      { title: 'Collision', keys: ['sensor', 'memberships', 'filter'] },
    ]);
    // Reordered within sections: the schema order wins, the grouping still fits.
    expect(sectionKeys('Collider', ['shape', 'restitution', 'density', 'friction', 'filter', 'sensor', 'memberships', 'new_field'], true)).toEqual([
      { title: null, keys: ['shape'] },
      { title: 'Material', keys: ['restitution', 'density', 'friction'] },
      { title: 'Collision', keys: ['filter', 'sensor', 'memberships'] },
      { title: 'Other', keys: ['new_field'] },
    ]);
    // An order that splits a section, or puts the uncaptioned lead row later, gets no captions.
    const split = ['shape', 'density', 'sensor', 'friction', 'restitution', 'memberships', 'filter'];
    expect(sectionKeys('Collider', split, true)).toEqual([{ title: null, keys: split }]);
    const late = ['density', 'friction', 'restitution', 'shape', 'sensor', 'memberships', 'filter'];
    expect(sectionKeys('Collider', late, true)).toEqual([{ title: null, keys: late }]);
  });

  it('uses the profile order only when the schema supplies none, and never drops unknown keys', () => {
    expect(sectionKeys('Collider', ['density', 'filter', 'friction', 'memberships', 'restitution', 'sensor', 'shape', 'new_field'], false)).toEqual([
      { title: null, keys: ['shape'] },
      { title: 'Material', keys: ['density', 'friction', 'restitution'] },
      { title: 'Collision', keys: ['sensor', 'memberships', 'filter'] },
      { title: 'Other', keys: ['new_field'] },
    ]);
    expect(sectionKeys('Transform', ['translation', 'rotation'], true)).toEqual([{ title: null, keys: ['translation', 'rotation'] }]);
  });

  it('renders a reordered Collider schema in its own order', async () => {
    const rowsFor = async (order: readonly string[]) => {
      const base = fixtureSnapshot('sample');
      const collider: ComponentSchema = { ...base.schemas.Collider!, order };
      const snapshot: BridgeSnapshot = { ...base, schemas: { ...base.schemas, Collider: collider } };
      const bridge: EditorBridge = {
        ...createFixtureBridge('sample'),
        getSnapshot: () => snapshot,
        subscribe: () => () => undefined,
      };
      render(<App resolution={{ kind: 'bridge', bridge }} />);
      fireEvent.click(screen.getByRole('treeitem', { name: /^Crate 01/ }));
      const region = await waitFor(() =>
        within(document.querySelector<HTMLElement>('[data-region="inspector"]')!).getByRole('region', { name: 'Collider' }),
      );
      const result = {
        labels: [...region.querySelectorAll(':scope .component__body > .field .field__label, :scope .component__section > .field .field__label')].map(
          (label) => label.firstChild?.textContent,
        ),
        captions: [...region.querySelectorAll('.component__section-title')].map((caption) => caption.textContent),
      };
      cleanup();
      return result;
    };
    expect(await rowsFor(['shape', 'restitution', 'friction', 'density', 'filter', 'memberships', 'sensor'])).toEqual({
      labels: ['Shape', 'Restitution', 'Friction', 'Density', 'Filter', 'Memberships', 'Sensor'],
      captions: ['Material', 'Collision'],
    });
    expect(await rowsFor(['shape', 'sensor', 'density', 'friction', 'restitution', 'memberships', 'filter'])).toEqual({
      labels: ['Shape', 'Sensor', 'Density', 'Friction', 'Restitution', 'Memberships', 'Filter'],
      captions: [],
    });
  });

  it('takes units and the mask widget from the schema, not the profile', () => {
    show('density', { type: 'number', optional: false }, 420);
    expect(screen.getByRole('textbox', { name: 'Density' }).closest('.control--number')?.querySelector('.control__unit')).toBeNull();
    cleanup();
    show('memberships', { type: 'integer', optional: false }, 4294967295);
    expect(document.querySelector('.control--mask')).toBeNull();
  });

  it('presents a crate’s RigidBody, Collider and AngularVelocity read-only with semantic units', async () => {
    const { container } = render(<App resolution={{ kind: 'bridge', bridge: createFixtureBridge('sample') }} />);
    fireEvent.click(screen.getByRole('treeitem', { name: /^Crate 01/ }));
    const inspector = await waitFor(() => {
      const element = document.querySelector<HTMLElement>('[data-region="inspector"]')!;
      expect(element.textContent).toContain('Crate 01');
      return element;
    });
    const body = within(inspector).getByRole('region', { name: 'RigidBody' });
    expect((within(body).getByRole('textbox', { name: 'Motion' }) as HTMLInputElement).value).toBe('dynamic');
    const damping = within(body).getByRole('group', { name: 'Gravity and damping' });
    expect((within(damping).getByRole('textbox', { name: 'Gravity scale' }) as HTMLInputElement).value).toBe('1');
    // Damping is a rate (Rapier applies v / (1 + dt·damping)); unit and description come from the schema.
    for (const name of ['Linear damping', 'Angular damping']) {
      const input = within(damping).getByRole('textbox', { name });
      expect(input.closest('.control--number')?.querySelector('.control__unit')?.textContent).toBe('1/s');
      expect(described(input)).toMatch(/damping rate, in inverse seconds/);
    }
    expect(within(damping).getByRole('textbox', { name: 'Gravity scale' }).closest('.control--number')?.querySelector('.control__unit')).toBeNull();
    const solver = within(body).getByRole('group', { name: 'Solver' });
    expect(within(solver).getByRole('checkbox', { name: 'Can sleep' }).getAttribute('aria-checked')).toBe('true');
    expect(within(solver).getByRole('checkbox', { name: 'CCD' }).getAttribute('aria-checked')).toBe('true');

    const collider = within(inspector).getByRole('region', { name: 'Collider' });
    const material = within(collider).getByRole('group', { name: 'Material' });
    const density = within(material).getByRole('textbox', { name: 'Density' });
    expect((density as HTMLInputElement).value).toBe('420');
    expect(density.closest('.control--number')?.querySelector('.control__unit')?.textContent).toBe('kg/m³');
    expect(within(material).getByRole('textbox', { name: 'Friction' }).closest('.control--number')?.querySelector('.control__unit')).toBeNull();
    const collision = within(collider).getByRole('group', { name: 'Collision' });
    expect(within(collision).getByRole('checkbox', { name: 'Sensor' }).getAttribute('aria-checked')).toBe('false');
    expect(within(collision).getAllByText('All groups')).toHaveLength(2);

    const angular = within(inspector).getByRole('region', { name: 'AngularVelocity' });
    const vector = within(angular).getByRole('group', { name: /Angular/ });
    expect(vector.closest('.field')?.querySelector('.field__unit')?.textContent).toContain('rad/s');
    expect(within(vector).getAllByRole('textbox').map((input) => (input as HTMLInputElement).value)).toEqual(['0', '1.5708', '0']);

    // Read-only: no buttons or editable inputs appear inside the physics components.
    for (const region of [body, collider, angular]) {
      const inputs = [...region.querySelectorAll('input')];
      expect(inputs.every((input) => input.readOnly)).toBe(true);
      expect([...region.querySelectorAll('button')].map((b) => b.className)).toEqual(['component__toggle']);
    }
    const results = await axe.run(inspector, { rules: { 'color-contrast': { enabled: false } } });
    expect(results.violations.map((v) => v.id)).toEqual([]);
    expect(container).toBeTruthy();
  });

  it('jumps from a nested problem to the exact axis and from an unknown tag to the Shape row', async () => {
    render(<App resolution={{ kind: 'bridge', bridge: createFixtureBridge('sample') }} />);
    fireEvent.click(screen.getByRole('button', { name: /Half extents must be positive/ }));
    await waitFor(() => expect(screen.getByRole('treeitem', { name: /^Crate 03/ }).getAttribute('aria-selected')).toBe('true'));
    const link = await waitFor(() => screen.getByRole('link', { name: /Collider\/shape\/half_extents\/1/ }));
    fireEvent.click(link);
    expect(document.activeElement?.id).toMatch(/_shape_half_extents_1$/);

    fireEvent.click(screen.getByRole('button', { name: /Unknown collider shape/ }));
    await waitFor(() => expect(screen.getByRole('treeitem', { name: /^Pier Planks/ }).getAttribute('aria-selected')).toBe('true'));
    fireEvent.click(await waitFor(() => screen.getByRole('link', { name: /Collider\/shape\/type/ })));
    const focused = document.activeElement as HTMLElement;
    expect(focused.classList.contains('control--notice')).toBe(true);
    expect(focused.textContent).toContain('Unknown shape type "cylinder"');
  });
});

describe('field names and descriptions', () => {
  it('keeps the visible label as the name and announces the description separately', () => {
    // Native WebKit named fields from the <label> title attribute, demoting the visible
    // label ("Friction") to a description (artifacts/physics-native-final/minimum-pill-ax.txt).
    const friction = NATIVE.Collider!.properties.friction!;
    const { container } = show('friction', friction, 0.4);
    const input = screen.getByRole('textbox', { name: 'Friction' });
    expect(described(input)).toBe('Friction coefficient (dimensionless).');
    expect(container.querySelector('label')?.hasAttribute('title')).toBe(false);
    expect(container.querySelector('.field')?.getAttribute('title')).toBe('Friction coefficient (dimensionless).');
    cleanup();
    // Schema descriptions win, also for switches and masks.
    show('memberships', NATIVE.Collider!.properties.memberships!, 4294967295);
    const mask = screen.getByRole('textbox', { name: 'Memberships' });
    expect(described(mask)).toContain('Collision requires both membership/filter intersections to be nonzero.');
    expect(described(mask)).toContain('Every collision group, 1 to 32.');
    cleanup();
    show('ccd', NATIVE.RigidBody!.properties.ccd!, true, { component: 'RigidBody' });
    const ccd = screen.getByRole('checkbox', { name: 'CCD' });
    expect(described(ccd)).toBe('Continuous collision detection for fast-moving bodies.');
    cleanup();
    // No description: no hidden text, the row tooltip is just the label.
    const plain = show('density', NATIVE.Collider!.properties.density!, 420);
    expect(screen.getByRole('textbox', { name: 'Density' }).getAttribute('aria-describedby')).toBeNull();
    expect(plain.container.querySelector('.field')?.getAttribute('title')).toBe('Density');
  });
});

describe('narrow Inspector layout', () => {
  it('keeps one level of nesting and the mask strip inside the value column', () => {
    // jsdom has no layout; the structural guarantees that keep narrow widths readable
    // are checked here and the pixels are reviewed in Chrome at 1000x650.
    const { container } = show('shape', SHAPE, { type: 'capsule', half_height: 0.6, radius: 0.15 });
    expect(container.querySelectorAll('.field-group .field-group')).toHaveLength(0);
    expect(container.querySelectorAll('fieldset')).toHaveLength(0);
    cleanup();
    const masked = show('filter', NATIVE.Collider!.properties.filter!, 3);
    const strip = masked.container.querySelector('.mask-strip')!;
    expect(strip.closest('.field__value')).not.toBeNull();
    expect(strip.getAttribute('aria-hidden')).toBe('true');
    expect(masked.container.querySelectorAll('.mask-strip__byte')).toHaveLength(4);
  });
});
