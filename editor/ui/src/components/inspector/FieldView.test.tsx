import { cleanup, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import type { Diagnostic, FieldSchema, Ulid } from '../../bridge/contract';
import { FieldView, unsetKind } from './FieldView';

afterEach(cleanup);

/**
 * Exactly the shape the native bridge emits for DirectionalLight.shadows after
 * resolving `anyOf: [{ $ref: '#/$defs/DirectionalShadows' }, { type: 'null' }]`
 * (see editor/bridge/native.test.ts), plus a title as the fixture schema has.
 */
const SHADOWS: FieldSchema = {
  type: 'object',
  title: 'Shadows',
  optional: true,
  nullable: true,
  properties: {
    distance: { type: 'number', minimum: 0.01, maximum: 10000, 'x-incant-unit': 'm', optional: false },
  },
};

const ENTITY = '01J9ZF1XTR0000000000000001' as Ulid;

function show(schema: FieldSchema, value: unknown, diagnostics: readonly Diagnostic[] = []) {
  return render(
    <FieldView
      name="shadows"
      schema={schema}
      value={value}
      path={['shadows']}
      ctx={{ entity: ENTITY, component: 'DirectionalLight', diagnostics }}
    />,
  );
}

function description(element: HTMLElement): string {
  return (element.getAttribute('aria-describedby') ?? '')
    .split(' ')
    .filter(Boolean)
    .map((id) => document.getElementById(id)?.textContent ?? '')
    .join(' ');
}

describe('FieldView optional and nullable values', () => {
  it('shows omitted optional shadows as a quiet read-only Off state, not a mismatch', () => {
    const { container } = show(SHADOWS, undefined);
    const control = screen.getByRole('textbox', { name: 'Shadows' });
    expect((control as HTMLInputElement).value).toBe('Off');
    expect((control as HTMLInputElement).readOnly).toBe(true);
    expect(description(control)).toContain('not present in the document');
    expect(container.querySelector('.control--unset')?.getAttribute('data-unset')).toBe('omitted');
    expect(container.querySelector('.control--notice')).toBeNull();
    expect(screen.queryByText(/Cannot show as/)).toBeNull();
    // Disabled shadows have no distance to present, and no edit affordance appears.
    expect(screen.queryByText('Distance')).toBeNull();
    expect(container.querySelector('fieldset')).toBeNull();
    expect(container.querySelector('button, [role="checkbox"], [role="switch"]')).toBeNull();
  });

  it('shows an explicit nullable null as Off and says it was authored as null', () => {
    const { container } = show(SHADOWS, null);
    const control = screen.getByRole('textbox', { name: 'Shadows' });
    expect((control as HTMLInputElement).value).toBe('Off');
    expect(description(control)).toContain('set to null');
    expect(container.querySelector('.control--unset')?.getAttribute('data-unset')).toBe('null');
    expect(screen.queryByText(/Cannot show as/)).toBeNull();
  });

  it('presents enabled shadows as a group with the distance in meters', () => {
    const { container } = show(SHADOWS, { distance: 120 });
    const group = screen.getByRole('group', { name: 'Shadows' });
    const distance = within(group).getByRole('textbox', { name: 'Distance' }) as HTMLInputElement;
    expect(distance.value).toBe('120');
    expect(distance.readOnly).toBe(true);
    expect(distance.closest('.control--number')?.querySelector('.control__unit')?.textContent).toBe('m');
    expect(container.querySelector('.control--unset')).toBeNull();
    expect(screen.queryByDisplayValue('Off')).toBeNull();
  });

  it('still reports a mismatch when a required child of an enabled group is missing', () => {
    show(SHADOWS, {});
    const group = screen.getByRole('group', { name: 'Shadows' });
    expect(within(group).getByText(/Cannot show as a number/)).toBeTruthy();
    expect(within(group).queryByDisplayValue('Not set')).toBeNull();
  });

  it('reports a mismatch for a missing or null required value', () => {
    const required: FieldSchema = { ...SHADOWS, optional: false, nullable: false };
    show(required, undefined);
    expect(screen.getByText(/Cannot show as object/)).toBeTruthy();
    expect(screen.queryByDisplayValue('Off')).toBeNull();
    cleanup();
    // No flags at all (older bridges) must behave as required, not as disabled.
    const { optional: _optional, nullable: _nullable, ...unflagged } = SHADOWS;
    show(unflagged as FieldSchema, null);
    expect(screen.getByText(/Cannot show as object/)).toBeTruthy();
  });

  it('keeps optional and nullable distinct: null needs nullable, omission needs optional', () => {
    show({ ...SHADOWS, nullable: false }, null);
    expect(screen.getByText(/Cannot show as object/)).toBeTruthy();
    cleanup();
    show({ ...SHADOWS, optional: false }, undefined);
    expect(screen.getByText(/Cannot show as object/)).toBeTruthy();
    expect(unsetKind({ ...SHADOWS, optional: false }, null)).toBe('null');
    expect(unsetKind({ ...SHADOWS, nullable: false }, undefined)).toBe('omitted');
    expect(unsetKind(SHADOWS, false)).toBeNull();
    expect(unsetKind(SHADOWS, 0)).toBeNull();
  });

  it('shows any optional scalar generically as Not set', () => {
    show({ type: 'number', title: 'Range', optional: true, 'x-incant-unit': 'm' }, undefined);
    const control = screen.getByRole('textbox', { name: 'Range' }) as HTMLInputElement;
    expect(control.value).toBe('Not set');
    expect(screen.queryByText('m')).toBeNull();
    cleanup();
    show({ type: 'string', title: 'Label', nullable: true }, null);
    expect((screen.getByRole('textbox', { name: 'Label' }) as HTMLInputElement).value).toBe('Not set');
  });

  it('keeps engine diagnostics attached to an unset field', () => {
    const diagnostic: Diagnostic = {
      id: 'd1',
      severity: 'warning',
      message: 'Shadows are off for the only sun.',
      entity: ENTITY,
      component: 'DirectionalLight',
      path: '/shadows',
    };
    const { container } = show(SHADOWS, undefined, [diagnostic]);
    const control = screen.getByRole('textbox', { name: 'Shadows' });
    expect(description(control)).toContain('Shadows are off for the only sun.');
    expect(container.querySelector('.field--warning')).not.toBeNull();
    expect(control.id).toBe(`field-${ENTITY}-DirectionalLight_shadows`);
  });
});
