import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { FieldSchema, Ulid } from '../bridge/contract';
import { FieldView } from './inspector/FieldView';

/**
 * Native review (0033, build f78d763): after End in a 64-part compound list, the
 * native app showed the pinned last part with a blank list above it, and its
 * accessibility tree still listed parts 1 to 13. The mounted window had not followed
 * the programmatic scroll. WebKit pauses or throttles animation frames for views that
 * are hidden, occluded or not frontmost, and the window was measured on a frame.
 *
 * These tests run with animation frames that never fire. jsdom has no layout, so the
 * list box reports a 177 px client height (6.5 rows of 26 px plus padding).
 */

const ENTITY = '01JA33RV000000000000000099' as Ulid;
const VECTOR = (n: number): FieldSchema => ({ type: 'array', items: { type: 'number' }, minItems: n, maxItems: n, optional: false });
const SHAPE: FieldSchema = {
  type: 'tagged-union',
  discriminator: 'type',
  optional: false,
  variants: {
    compound: {
      type: 'object',
      properties: {
        type: { type: 'string', enum: ['compound'], optional: false },
        parts: {
          type: 'array',
          optional: false,
          items: {
            type: 'object',
            properties: {
              id: { type: 'string', optional: false },
              translation: { ...VECTOR(3), 'x-incant-unit': 'm' } as FieldSchema,
              rotation: VECTOR(4),
              shape: {
                type: 'tagged-union',
                discriminator: 'type',
                optional: false,
                variants: {
                  box: { type: 'object', properties: { type: { type: 'string', enum: ['box'], optional: false }, half_extents: VECTOR(3) } },
                },
              },
            },
          },
        },
      },
    },
  },
};
const PARTS = Array.from({ length: 64 }, (_, i) => ({
  id: `01JA33RV0000000000000${String(1000 + i).padStart(5, '0')}`,
  translation: [i, 0, 0],
  rotation: [0, 0, 0, 1],
  shape: { type: 'box', half_extents: [0.25, 0.3, 0.4] },
}));

let restore: (() => void)[] = [];
beforeEach(() => {
  vi.stubGlobal('requestAnimationFrame', () => 1);
  vi.stubGlobal('cancelAnimationFrame', () => undefined);
  const client = vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockImplementation(function (this: HTMLElement) {
    return this.classList.contains('object-list__items') ? 177 : 0;
  });
  restore = [() => client.mockRestore()];
});
afterEach(() => {
  for (const undo of restore) undo();
  vi.unstubAllGlobals();
});

const show = () =>
  render(
    <FieldView
      name="shape"
      schema={SHAPE}
      value={{ type: 'compound', parts: PARTS }}
      path={['shape']}
      ctx={{ entity: ENTITY, component: 'Collider', diagnostics: [] }}
    />,
  );
const listbox = () => screen.getByRole('listbox', { name: 'Parts' });
const mounted = () => within(listbox()).getAllByRole('option').map((row) => Number(row.dataset.index));

describe('windowed lists without animation frames', () => {
  it('mounts the rows around the last part after End, not only the pinned row, and focuses it', () => {
    show();
    const first = within(listbox()).getAllByRole('option')[0]!;
    act(() => first.focus());
    fireEvent.keyDown(listbox(), { key: 'End' });
    // The list scrolled to its end: rows 57 to 63 are in view, so they are mounted.
    expect(listbox().scrollTop).toBeGreaterThan(1400);
    for (const index of [57, 58, 59, 60, 61, 62, 63]) expect(mounted()).toContain(index);
    expect(mounted()).not.toContain(0);
    expect(mounted().length).toBeLessThan(40);
    expect((document.activeElement as HTMLElement).dataset.index).toBe('63');
    expect(document.querySelector('.object-list__caption')?.textContent).toMatch(/^Part 64\s*of 64/);
  });

  it('follows a scroll made by the user once the frame fallback runs', async () => {
    show();
    act(() => {
      listbox().scrollTop = 26 * 30;
      fireEvent.scroll(listbox());
    });
    // No frame ever comes; the timer measures instead.
    await waitFor(() => expect(mounted()).toContain(32), { timeout: 1000 });
    // Part 1 stays mounted as the selection; its neighbours have left the window.
    expect(mounted()).not.toContain(1);
  });
});
